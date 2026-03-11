use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use global_hotkey::hotkey::HotKey;
use global_hotkey::GlobalHotKeyManager;

#[derive(Clone, Debug)]
pub struct PendingDownload {
    pub path: Option<String>,
    pub show_dialog: bool,
}

static PENDING_DOWNLOADS: OnceLock<Mutex<HashMap<i32, PendingDownload>>> = OnceLock::new();
static LAUNCH_CONTEXT: OnceLock<Mutex<LaunchContext>> = OnceLock::new();
static FILE_STREAMS: OnceLock<Mutex<HashMap<String, FileStreamEntry>>> = OnceLock::new();
static GLOBAL_SHORTCUT_MANAGER: OnceLock<Mutex<GlobalHotKeyManager>> = OnceLock::new();
static GLOBAL_SHORTCUTS: OnceLock<Mutex<HashMap<String, RegisteredGlobalShortcut>>> =
    OnceLock::new();
static GLOBAL_SHORTCUTS_BY_HOTKEY_ID: OnceLock<Mutex<HashMap<u32, String>>> = OnceLock::new();
static GLOBAL_SHORTCUT_EVENTS: OnceLock<Mutex<Vec<GlobalShortcutEvent>>> = OnceLock::new();

fn pending_downloads() -> &'static Mutex<HashMap<i32, PendingDownload>> {
    PENDING_DOWNLOADS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn set_pending_download(browser_id: i32, download: PendingDownload) -> Result<(), String> {
    let mut state = pending_downloads()
        .lock()
        .map_err(|_| "Failed to lock pending downloads state".to_string())?;
    state.insert(browser_id, download);
    Ok(())
}

pub fn take_pending_download(browser_id: i32) -> Option<PendingDownload> {
    pending_downloads().lock().ok()?.remove(&browser_id)
}

#[derive(Clone, Debug, Default)]
pub struct LaunchContext {
    pub deep_link: Option<String>,
    pub files: Vec<String>,
}

fn launch_context() -> &'static Mutex<LaunchContext> {
    LAUNCH_CONTEXT.get_or_init(|| Mutex::new(LaunchContext::default()))
}

pub fn set_launch_context(context: LaunchContext) -> Result<(), String> {
    let mut state = launch_context()
        .lock()
        .map_err(|_| "Failed to lock launch context state".to_string())?;
    *state = context;
    Ok(())
}

pub fn get_launch_context() -> Result<LaunchContext, String> {
    launch_context()
        .lock()
        .map(|state| state.clone())
        .map_err(|_| "Failed to lock launch context state".to_string())
}

#[derive(Clone, Debug)]
pub struct FileStreamEntry {
    pub path: String,
    pub mime_type: String,
}

fn file_streams() -> &'static Mutex<HashMap<String, FileStreamEntry>> {
    FILE_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_file_stream(entry: FileStreamEntry) -> Result<String, String> {
    let token = format!(
        "stream_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| err.to_string())?
            .as_nanos()
    );
    let mut state = file_streams()
        .lock()
        .map_err(|_| "Failed to lock file streams state".to_string())?;
    state.insert(token.clone(), entry);
    Ok(token)
}

pub fn get_file_stream(token: &str) -> Option<FileStreamEntry> {
    file_streams().lock().ok()?.get(token).cloned()
}

#[derive(Clone, Debug)]
pub struct RegisteredGlobalShortcut {
    pub id: String,
    pub accelerator: String,
    pub hotkey: HotKey,
}

#[derive(Clone, Debug)]
pub struct GlobalShortcutEvent {
    pub id: String,
    pub accelerator: String,
    pub state: String,
}

fn global_shortcuts() -> &'static Mutex<HashMap<String, RegisteredGlobalShortcut>> {
    GLOBAL_SHORTCUTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn global_shortcuts_by_hotkey_id() -> &'static Mutex<HashMap<u32, String>> {
    GLOBAL_SHORTCUTS_BY_HOTKEY_ID.get_or_init(|| Mutex::new(HashMap::new()))
}

fn global_shortcut_events() -> &'static Mutex<Vec<GlobalShortcutEvent>> {
    GLOBAL_SHORTCUT_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn init_global_shortcut_manager() -> Result<(), String> {
    if GLOBAL_SHORTCUT_MANAGER.get().is_some() {
        return Ok(());
    }

    let manager = GlobalHotKeyManager::new().map_err(|err| err.to_string())?;
    let _ = GLOBAL_SHORTCUT_MANAGER.set(Mutex::new(manager));
    Ok(())
}

pub fn register_global_shortcut(
    id: String,
    accelerator: String,
    hotkey: HotKey,
) -> Result<(), String> {
    let manager = GLOBAL_SHORTCUT_MANAGER
        .get()
        .ok_or_else(|| "Global shortcut manager is not initialized".to_string())?;
    manager
        .lock()
        .map_err(|_| "Failed to lock global shortcut manager".to_string())?
        .register(hotkey)
        .map_err(|err| err.to_string())?;

    global_shortcuts()
        .lock()
        .map_err(|_| "Failed to lock global shortcuts state".to_string())?
        .insert(
            id.clone(),
            RegisteredGlobalShortcut {
                id: id.clone(),
                accelerator,
                hotkey,
            },
        );
    global_shortcuts_by_hotkey_id()
        .lock()
        .map_err(|_| "Failed to lock global shortcut ids state".to_string())?
        .insert(hotkey.id(), id);
    Ok(())
}

pub fn unregister_global_shortcut(id: &str) -> Result<Option<RegisteredGlobalShortcut>, String> {
    let shortcut = global_shortcuts()
        .lock()
        .map_err(|_| "Failed to lock global shortcuts state".to_string())?
        .remove(id);

    if let Some(shortcut) = shortcut.clone() {
        if let Some(manager) = GLOBAL_SHORTCUT_MANAGER.get() {
            manager
                .lock()
                .map_err(|_| "Failed to lock global shortcut manager".to_string())?
                .unregister(shortcut.hotkey)
                .map_err(|err| err.to_string())?;
        }

        global_shortcuts_by_hotkey_id()
            .lock()
            .map_err(|_| "Failed to lock global shortcut ids state".to_string())?
            .remove(&shortcut.hotkey.id());
    }

    Ok(shortcut)
}

pub fn list_global_shortcuts() -> Result<Vec<RegisteredGlobalShortcut>, String> {
    global_shortcuts()
        .lock()
        .map(|state| state.values().cloned().collect())
        .map_err(|_| "Failed to lock global shortcuts state".to_string())
}

pub fn push_global_shortcut_event(hotkey_id: u32, state: &str) -> Result<(), String> {
    let shortcut_id = global_shortcuts_by_hotkey_id()
        .lock()
        .map_err(|_| "Failed to lock global shortcut ids state".to_string())?
        .get(&hotkey_id)
        .cloned();

    let Some(shortcut_id) = shortcut_id else {
        return Ok(());
    };

    let shortcut = global_shortcuts()
        .lock()
        .map_err(|_| "Failed to lock global shortcuts state".to_string())?
        .get(&shortcut_id)
        .cloned();

    if let Some(shortcut) = shortcut {
        global_shortcut_events()
            .lock()
            .map_err(|_| "Failed to lock global shortcut events state".to_string())?
            .push(GlobalShortcutEvent {
                id: shortcut.id,
                accelerator: shortcut.accelerator,
                state: state.to_string(),
            });
    }

    Ok(())
}

pub fn take_global_shortcut_events() -> Result<Vec<GlobalShortcutEvent>, String> {
    let mut state = global_shortcut_events()
        .lock()
        .map_err(|_| "Failed to lock global shortcut events state".to_string())?;
    Ok(std::mem::take(&mut *state))
}
