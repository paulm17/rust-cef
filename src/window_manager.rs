use cef;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use winit::window::Window;
use winit::window::WindowId;

pub struct ManagedWindow {
    pub window: Window,
    pub browser: Option<cef::Browser>,
    pub persist_key: Option<String>,
}

pub struct WindowManager {
    windows: HashMap<usize, ManagedWindow>,
    next_id: AtomicUsize,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            next_id: AtomicUsize::new(1),
        }
    }

    pub fn insert(&mut self, window: Window, persist_key: Option<String>) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.windows.insert(
            id,
            ManagedWindow {
                window,
                browser: None,
                persist_key,
            },
        );
        id
    }

    pub fn attach_browser(&mut self, id: usize, browser: cef::Browser) {
        if let Some(managed) = self.windows.get_mut(&id) {
            managed.browser = Some(browser);
        }
    }

    pub fn remove(&mut self, id: usize) -> Option<ManagedWindow> {
        self.windows.remove(&id)
    }

    pub fn get(&self, id: usize) -> Option<&ManagedWindow> {
        self.windows.get(&id)
    }

    pub fn find_id_by_window_id(&self, window_id: WindowId) -> Option<usize> {
        self.windows
            .iter()
            .find_map(|(id, managed)| (managed.window.id() == window_id).then_some(*id))
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &ManagedWindow> {
        self.windows.values()
    }

    pub fn windows_iter(&self) -> impl Iterator<Item = (&usize, &ManagedWindow)> {
        self.windows.iter()
    }
}
