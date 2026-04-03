use std::collections::HashSet;
use std::sync::{mpsc, Arc, Condvar, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MilestoneId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneDefinition {
    pub key: String,
    pub label: String,
    pub weight: u32,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilestoneState {
    Pending,
    Running { progress: u8 },
    Completed,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneSnapshot {
    pub id: MilestoneId,
    pub key: String,
    pub label: String,
    pub weight: u32,
    pub required: bool,
    pub state: MilestoneState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSnapshot {
    pub aggregate_progress: u8,
    pub status_text: String,
    pub milestones: Vec<MilestoneSnapshot>,
    pub is_complete: bool,
    pub has_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupEvent {
    SnapshotUpdated(StartupSnapshot),
    ReadyForCef,
}

#[derive(Debug, Clone)]
pub struct StartupCoordinator {
    inner: Arc<CoordinatorInner>,
}

#[derive(Debug, Clone)]
pub struct StartupGate {
    inner: Arc<CoordinatorInner>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartupCoordinatorError {
    DuplicateMilestoneKey(String),
    UnknownMilestone,
    GateAlreadyOpened,
    RequiredMilestonesIncomplete,
}

impl std::fmt::Display for StartupCoordinatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateMilestoneKey(key) => {
                write!(f, "duplicate milestone key registered: {key}")
            }
            Self::UnknownMilestone => write!(f, "unknown milestone"),
            Self::GateAlreadyOpened => write!(f, "startup gate already opened"),
            Self::RequiredMilestonesIncomplete => {
                write!(f, "required milestones are not yet completed")
            }
        }
    }
}

impl std::error::Error for StartupCoordinatorError {}

#[derive(Debug)]
struct CoordinatorInner {
    state: Mutex<CoordinatorState>,
    gate_cv: Condvar,
}

#[derive(Debug)]
struct CoordinatorState {
    milestones: Vec<MilestoneRecord>,
    subscribers: Vec<mpsc::Sender<StartupEvent>>,
    ready_for_cef: bool,
}

#[derive(Debug, Clone)]
struct MilestoneRecord {
    definition: MilestoneDefinition,
    state: MilestoneState,
}

impl StartupCoordinator {
    pub fn new(definitions: Vec<MilestoneDefinition>) -> Result<Self, StartupCoordinatorError> {
        let mut seen = HashSet::new();
        let mut milestones = Vec::with_capacity(definitions.len());

        for definition in definitions {
            if !seen.insert(definition.key.clone()) {
                return Err(StartupCoordinatorError::DuplicateMilestoneKey(
                    definition.key,
                ));
            }

            milestones.push(MilestoneRecord {
                definition,
                state: MilestoneState::Pending,
            });
        }

        Ok(Self {
            inner: Arc::new(CoordinatorInner {
                state: Mutex::new(CoordinatorState {
                    milestones,
                    subscribers: Vec::new(),
                    ready_for_cef: false,
                }),
                gate_cv: Condvar::new(),
            }),
        })
    }

    pub fn gate(&self) -> StartupGate {
        StartupGate {
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn milestone_id(&self, key: &str) -> Option<MilestoneId> {
        let state = self
            .inner
            .state
            .lock()
            .expect("startup coordinator poisoned");
        state
            .milestones
            .iter()
            .position(|record| record.definition.key == key)
            .map(MilestoneId)
    }

    pub fn subscribe(&self) -> mpsc::Receiver<StartupEvent> {
        let (tx, rx) = mpsc::channel();
        let (initial, ready_for_cef) = {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("startup coordinator poisoned");
            state.subscribers.push(tx.clone());
            (snapshot_from_state(&state), state.ready_for_cef)
        };
        let _ = tx.send(StartupEvent::SnapshotUpdated(initial));
        if ready_for_cef {
            let _ = tx.send(StartupEvent::ReadyForCef);
        }
        rx
    }

    pub fn snapshot(&self) -> StartupSnapshot {
        let state = self
            .inner
            .state
            .lock()
            .expect("startup coordinator poisoned");
        snapshot_from_state(&state)
    }

    pub fn start(&self, id: MilestoneId) -> Result<(), StartupCoordinatorError> {
        self.update_milestone(id, |record| {
            if matches!(record.state, MilestoneState::Pending) {
                record.state = MilestoneState::Running { progress: 0 };
            }
        })
    }

    pub fn set_progress(
        &self,
        id: MilestoneId,
        progress: f32,
    ) -> Result<(), StartupCoordinatorError> {
        let normalized = normalize_progress(progress);
        self.update_milestone(id, move |record| {
            if matches!(record.state, MilestoneState::Completed) {
                return;
            }

            if let MilestoneState::Failed { .. } = record.state {
                return;
            }

            record.state = MilestoneState::Running {
                progress: normalized,
            };
        })
    }

    pub fn complete(&self, id: MilestoneId) -> Result<(), StartupCoordinatorError> {
        self.update_milestone(id, |record| {
            record.state = MilestoneState::Completed;
        })
    }

    pub fn fail(
        &self,
        id: MilestoneId,
        message: impl Into<String>,
    ) -> Result<(), StartupCoordinatorError> {
        let message = message.into();
        self.update_milestone(id, move |record| {
            record.state = MilestoneState::Failed {
                message: message.clone(),
            };
        })
    }

    pub fn mark_ready_for_cef(&self) -> Result<(), StartupCoordinatorError> {
        let subscribers = {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("startup coordinator poisoned");
            if state.ready_for_cef {
                return Err(StartupCoordinatorError::GateAlreadyOpened);
            }
            if !required_milestones_complete(&state) {
                return Err(StartupCoordinatorError::RequiredMilestonesIncomplete);
            }
            state.ready_for_cef = true;
            state.subscribers.clone()
        };

        self.inner.gate_cv.notify_all();
        notify(subscribers, StartupEvent::ReadyForCef);
        Ok(())
    }

    pub fn wait_until_ready_for_cef(&self) {
        self.gate().wait_until_ready_for_cef();
    }

    fn update_milestone<F>(
        &self,
        id: MilestoneId,
        updater: F,
    ) -> Result<(), StartupCoordinatorError>
    where
        F: FnOnce(&mut MilestoneRecord),
    {
        let (subscribers, snapshot) = {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("startup coordinator poisoned");
            let record = state
                .milestones
                .get_mut(id.0)
                .ok_or(StartupCoordinatorError::UnknownMilestone)?;
            updater(record);
            let snapshot = snapshot_from_state(&state);
            (state.subscribers.clone(), snapshot)
        };

        notify(subscribers, StartupEvent::SnapshotUpdated(snapshot));
        Ok(())
    }
}

impl StartupGate {
    pub fn wait_until_ready_for_cef(&self) {
        let mut state = self.inner.state.lock().expect("startup gate poisoned");
        while !state.ready_for_cef {
            state = self
                .inner
                .gate_cv
                .wait(state)
                .expect("startup gate poisoned");
        }
    }
}

fn required_milestones_complete(state: &CoordinatorState) -> bool {
    state.milestones.iter().all(|record| {
        !record.definition.required || matches!(record.state, MilestoneState::Completed)
    })
}

fn snapshot_from_state(state: &CoordinatorState) -> StartupSnapshot {
    let mut required_weight_total = 0.0f32;
    let mut required_weight_complete = 0.0f32;
    let mut milestones = Vec::with_capacity(state.milestones.len());
    let mut current_label = None;
    let mut has_failed = false;

    for (index, record) in state.milestones.iter().enumerate() {
        if record.definition.required {
            required_weight_total += record.definition.weight as f32;
            required_weight_complete += match &record.state {
                MilestoneState::Pending => 0.0,
                MilestoneState::Running { progress } => {
                    record.definition.weight as f32 * (f32::from(*progress) / 100.0)
                }
                MilestoneState::Completed => record.definition.weight as f32,
                MilestoneState::Failed { .. } => 0.0,
            };
        }

        if current_label.is_none()
            && matches!(
                record.state,
                MilestoneState::Running { .. } | MilestoneState::Failed { .. }
            )
        {
            current_label = Some(record.definition.label.clone());
        }

        has_failed |= matches!(record.state, MilestoneState::Failed { .. });

        milestones.push(MilestoneSnapshot {
            id: MilestoneId(index),
            key: record.definition.key.clone(),
            label: record.definition.label.clone(),
            weight: record.definition.weight,
            required: record.definition.required,
            state: record.state.clone(),
        });
    }

    let aggregate_progress = if required_weight_total == 0.0 {
        100
    } else {
        ((required_weight_complete / required_weight_total) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u8
    };

    let is_complete = required_milestones_complete(state);
    let status_text = if let Some(label) = current_label {
        label
    } else if has_failed {
        "Startup failed".to_string()
    } else if is_complete {
        "Ready".to_string()
    } else {
        "Preparing startup".to_string()
    };

    StartupSnapshot {
        aggregate_progress,
        status_text,
        milestones,
        is_complete,
        has_failed,
    }
}

fn notify(subscribers: Vec<mpsc::Sender<StartupEvent>>, event: StartupEvent) {
    for subscriber in subscribers {
        let _ = subscriber.send(event.clone());
    }
}

fn normalize_progress(progress: f32) -> u8 {
    if progress.is_nan() {
        return 0;
    }

    (progress.clamp(0.0, 1.0) * 100.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs() -> Vec<MilestoneDefinition> {
        vec![
            MilestoneDefinition {
                key: "config".to_string(),
                label: "Load config".to_string(),
                weight: 2,
                required: true,
            },
            MilestoneDefinition {
                key: "services".to_string(),
                label: "Start services".to_string(),
                weight: 3,
                required: true,
            },
            MilestoneDefinition {
                key: "warm-cache".to_string(),
                label: "Warm cache".to_string(),
                weight: 5,
                required: false,
            },
        ]
    }

    #[test]
    fn computes_weighted_required_progress() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        coordinator.set_progress(MilestoneId(0), 0.5).unwrap();
        coordinator.set_progress(MilestoneId(1), 0.2).unwrap();

        let snapshot = coordinator.snapshot();
        assert_eq!(snapshot.aggregate_progress, 32);
    }

    #[test]
    fn optional_milestones_do_not_block_readiness() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        coordinator.complete(MilestoneId(0)).unwrap();
        coordinator.complete(MilestoneId(1)).unwrap();

        assert!(coordinator.mark_ready_for_cef().is_ok());
    }

    #[test]
    fn rejects_duplicate_milestone_keys() {
        let result = StartupCoordinator::new(vec![
            MilestoneDefinition {
                key: "dup".to_string(),
                label: "One".to_string(),
                weight: 1,
                required: true,
            },
            MilestoneDefinition {
                key: "dup".to_string(),
                label: "Two".to_string(),
                weight: 1,
                required: true,
            },
        ]);

        assert!(matches!(
            result,
            Err(StartupCoordinatorError::DuplicateMilestoneKey(_))
        ));
    }

    #[test]
    fn complete_and_fail_are_terminal_states_for_snapshot() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        coordinator.complete(MilestoneId(0)).unwrap();
        coordinator.fail(MilestoneId(1), "boom").unwrap();

        let snapshot = coordinator.snapshot();
        assert_eq!(snapshot.aggregate_progress, 40);
        assert!(snapshot.has_failed);
        assert_eq!(
            snapshot.milestones[1].state,
            MilestoneState::Failed {
                message: "boom".to_string()
            }
        );
    }

    #[test]
    fn preserves_registration_order() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        let snapshot = coordinator.snapshot();
        let keys: Vec<_> = snapshot
            .milestones
            .iter()
            .map(|milestone| milestone.key.as_str())
            .collect();

        assert_eq!(keys, vec!["config", "services", "warm-cache"]);
    }

    #[test]
    fn mark_ready_requires_required_milestones_completed() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        coordinator.complete(MilestoneId(0)).unwrap();

        let result = coordinator.mark_ready_for_cef();
        assert_eq!(
            result,
            Err(StartupCoordinatorError::RequiredMilestonesIncomplete)
        );
    }

    #[test]
    fn late_subscribers_receive_ready_event() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        coordinator.complete(MilestoneId(0)).unwrap();
        coordinator.complete(MilestoneId(1)).unwrap();
        coordinator.mark_ready_for_cef().unwrap();

        let rx = coordinator.subscribe();
        let first = rx.recv().unwrap();
        let second = rx.recv().unwrap();

        assert!(matches!(first, StartupEvent::SnapshotUpdated(_)));
        assert_eq!(second, StartupEvent::ReadyForCef);
    }

    #[test]
    fn milestone_id_resolves_registered_keys() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();

        assert_eq!(coordinator.milestone_id("config"), Some(MilestoneId(0)));
        assert_eq!(coordinator.milestone_id("services"), Some(MilestoneId(1)));
        assert_eq!(coordinator.milestone_id("missing"), None);
    }

    #[test]
    fn gate_waits_until_ready_signal() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        coordinator.complete(MilestoneId(0)).unwrap();
        coordinator.complete(MilestoneId(1)).unwrap();

        let gate = coordinator.gate();
        let worker = coordinator.clone();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            gate.wait_until_ready_for_cef();
            tx.send(()).unwrap();
        });

        assert!(rx.recv_timeout(std::time::Duration::from_millis(50)).is_err());

        worker.mark_ready_for_cef().unwrap();
        assert!(rx.recv_timeout(std::time::Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn mark_ready_rejects_second_open() {
        let coordinator = StartupCoordinator::new(defs()).unwrap();
        coordinator.complete(MilestoneId(0)).unwrap();
        coordinator.complete(MilestoneId(1)).unwrap();
        coordinator.mark_ready_for_cef().unwrap();

        let result = coordinator.mark_ready_for_cef();
        assert_eq!(result, Err(StartupCoordinatorError::GateAlreadyOpened));
    }
}
