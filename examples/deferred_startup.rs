use rust_embed::RustEmbed;
use std::thread;
use std::time::Duration;

#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

fn main() {
    let coordinator = rust_cef::StartupCoordinator::new(vec![
        rust_cef::MilestoneDefinition {
            key: "config".to_string(),
            label: "Loading configuration".to_string(),
            weight: 20,
            required: true,
        },
        rust_cef::MilestoneDefinition {
            key: "services".to_string(),
            label: "Starting background services".to_string(),
            weight: 35,
            required: true,
        },
        rust_cef::MilestoneDefinition {
            key: "assets".to_string(),
            label: "Warming bundled assets".to_string(),
            weight: 45,
            required: true,
        },
    ])
    .expect("failed to build startup coordinator");

    let worker = coordinator.clone();
    thread::spawn(move || {
        drive_milestone(&worker, "config", 4, 150);
        drive_milestone(&worker, "services", 6, 120);
        drive_milestone(&worker, "assets", 8, 90);
        worker
            .mark_ready_for_cef()
            .expect("startup milestones should be complete");
    });

    let app = rust_cef::App::new()
        .title("Rust + CEF Deferred Startup")
        .assets(|path| Assets::get(path))
        .deferred_startup(rust_cef::DeferredStartupConfig {
            coordinator,
            ui: rust_cef::StartupUiConfig {
                title: "Rust + CEF".to_string(),
                subtitle: Some("Host-driven startup coordinator".to_string()),
                ..Default::default()
            },
            transition_delay_ms: 350,
        });

    if let Err(err) = app.run() {
        eprintln!("Application error: {err}");
    }
}

fn drive_milestone(
    coordinator: &rust_cef::StartupCoordinator,
    key: &str,
    steps: u32,
    sleep_ms: u64,
) {
    let id = coordinator
        .milestone_id(key)
        .unwrap_or_else(|| panic!("missing milestone key: {key}"));
    coordinator.start(id).expect("failed to start milestone");

    for step in 1..=steps {
        thread::sleep(Duration::from_millis(sleep_ms));
        coordinator
            .set_progress(id, step as f32 / steps as f32)
            .expect("failed to update progress");
    }

    coordinator
        .complete(id)
        .expect("failed to complete milestone");
}
