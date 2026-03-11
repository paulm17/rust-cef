use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub main_window: Option<WindowBounds>,
    pub child_windows: HashMap<String, WindowBounds>,
}

pub struct ConfigManager {
    config_path: PathBuf,
    pub current: AppConfig,
}

impl ConfigManager {
    pub fn new() -> Self {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("rust-cef");
        path.push("config.json");

        let current = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => AppConfig::default(),
            }
        } else {
            AppConfig::default()
        };

        Self {
            config_path: path,
            current,
        }
    }

    pub fn save(&self) {
        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.current) {
            let _ = fs::write(&self.config_path, json);
        }
    }

    pub fn update_main_window_bounds(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.current.main_window = Some(WindowBounds {
            x,
            y,
            width,
            height,
        });
    }

    pub fn get_child_window_bounds(&self, key: &str) -> Option<&WindowBounds> {
        self.current.child_windows.get(key)
    }

    pub fn update_child_window_bounds(
        &mut self,
        key: impl Into<String>,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) {
        self.current.child_windows.insert(
            key.into(),
            WindowBounds {
                x,
                y,
                width,
                height,
            },
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BadgesConfig {
    pub dock: Option<String>,
    pub taskbar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceConfig {
    pub badges: Option<BadgesConfig>,
}

impl WorkspaceConfig {
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string("config.toml") {
            let parsed: Result<WorkspaceConfig, _> = toml::from_str(&content);
            match parsed {
                Ok(config) => config,
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }
}
