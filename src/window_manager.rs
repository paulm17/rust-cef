use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use cef;
use winit::window::Window;

pub struct ManagedWindow {
    pub window: Window,
    pub browser: Option<cef::Browser>,
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

    pub fn insert(&mut self, window: Window) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.windows.insert(id, ManagedWindow {
            window,
            browser: None,
        });
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
