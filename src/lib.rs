use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{WindowBuilder, WindowLevel},
};

#[cfg(target_os = "macos")]
use objc2::{
    class, msg_send,
    runtime::{AnyClass, AnyObject, Bool, Sel},
    sel,
};
#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;

use std::os::unix::process::CommandExt;
use std::sync::mpsc;

use cef::{self, CefString, ImplBrowser, ImplBrowserHost, ImplFrame};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::Arc;
pub mod app;
pub mod assets;
pub mod backend;
pub mod client;
pub mod config;
pub mod debug_logger;
pub mod error_reporting;
pub mod ipc;
pub mod menus;
pub mod platform;
pub mod security;
pub mod single_instance;
pub mod state;
pub mod tray;
pub mod updater;
pub mod window_manager;

use app::AppBuilder;
use client::{client::ClientBuilder, IcyClient};
use debug_logger::{init_logging, log_debug, print_debug, print_info, set_debug_mode};
use ipc::bridge::CommandRouter;
use platform::scheme_handler::AssetResolver;
use window_manager::WindowManager;

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub url: String,
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub persist_key: Option<String>,
    pub resizable: bool,
    pub start_hidden: bool,
    pub frameless: Option<bool>,
    pub transparent: Option<bool>,
    pub always_on_top: Option<bool>,
    pub kiosk: Option<bool>,
    pub icon: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct PrintToPdfRequest {
    pub path: String,
    pub landscape: bool,
    pub print_background: bool,
    pub display_header_footer: bool,
    pub scale: f64,
    pub response_tx: mpsc::Sender<Result<serde_json::Value, String>>,
}

#[derive(Debug, Clone)]
pub struct StartDownloadRequest {
    pub url: String,
    pub path: Option<String>,
    pub show_dialog: bool,
    pub response_tx: mpsc::Sender<Result<serde_json::Value, String>>,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ContentLoaded,
    CreateWindow(WindowConfig),
    ExternalLaunch(crate::state::LaunchContext),
    PrintToPdf(PrintToPdfRequest),
    StartDownload(StartDownloadRequest),
    ScheduleMessagePumpWork(i64),
    SetDecorations(Option<usize>, bool),
    SetAlwaysOnTop(Option<usize>, bool),
    SetWindowIcon(Option<usize>, Option<winit::window::Icon>),
    SetKiosk(Option<usize>, bool),
    SetTrayBadge(u32),
}

use std::sync::OnceLock;
pub static EVENT_LOOP_PROXY: OnceLock<
    std::sync::Mutex<winit::event_loop::EventLoopProxy<AppEvent>>,
> = OnceLock::new();

struct PdfPrintCallbackBridge {
    object: *mut cef::rc::RcImpl<cef::sys::_cef_pdf_print_callback_t, Self>,
    response_tx: mpsc::Sender<Result<serde_json::Value, String>>,
}

impl PdfPrintCallbackBridge {
    fn new(response_tx: mpsc::Sender<Result<serde_json::Value, String>>) -> cef::PdfPrintCallback {
        cef::PdfPrintCallback::new(Self {
            object: std::ptr::null_mut(),
            response_tx,
        })
    }
}

impl cef::WrapPdfPrintCallback for PdfPrintCallbackBridge {
    fn wrap_rc(&mut self, object: *mut cef::rc::RcImpl<cef::sys::_cef_pdf_print_callback_t, Self>) {
        self.object = object;
    }
}

impl cef::rc::Rc for PdfPrintCallbackBridge {
    fn as_base(&self) -> &cef::sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl Clone for PdfPrintCallbackBridge {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            cef::rc::Rc::add_ref(&rc_impl.interface);
            rc_impl
        };

        Self {
            object,
            response_tx: self.response_tx.clone(),
        }
    }
}

impl cef::ImplPdfPrintCallback for PdfPrintCallbackBridge {
    fn get_raw(&self) -> *mut cef::sys::_cef_pdf_print_callback_t {
        self.object.cast()
    }

    fn on_pdf_print_finished(&self, path: Option<&CefString>, ok: std::os::raw::c_int) {
        let path = path
            .map(cef::CefStringUtf8::from)
            .and_then(|value| value.as_str().map(|value| value.to_string()))
            .unwrap_or_default();
        let result = if ok != 0 {
            Ok(serde_json::json!({
                "status": "printed",
                "path": path,
            }))
        } else {
            Err(format!("CEF failed to print PDF: {path}"))
        };

        let _ = self.response_tx.send(result);
    }
}

fn logical_window_bounds(window: &winit::window::Window) -> Option<config::WindowBounds> {
    let position = window.outer_position().ok()?;
    let logical_position = position.to_logical::<i32>(window.scale_factor());
    let logical_size = window.inner_size().to_logical::<u32>(window.scale_factor());

    Some(config::WindowBounds {
        x: logical_position.x,
        y: logical_position.y,
        width: logical_size.width,
        height: logical_size.height,
    })
}

fn emit_browser_event(
    window_manager: &WindowManager,
    window_id: usize,
    event: &str,
    payload: serde_json::Value,
) {
    let script = format!(
        "window.dispatchEvent(new CustomEvent('rust-cef-event', {{ detail: {{ event: {}, payload: {} }} }}));",
        serde_json::to_string(event).unwrap_or_else(|_| "\"unknown\"".to_string()),
        payload
    );

    if let Some(managed) = window_manager.get(window_id) {
        if let Some(browser) = &managed.browser {
            if let Some(frame) = browser.main_frame() {
                frame.execute_java_script(
                    Some(&cef::CefString::from(script.as_str())),
                    Some(&cef::CefString::from("app://localhost/__rust_event__.js")),
                    1,
                );
            }
        }
    }
}

/// Configuration for the Development Environment
#[derive(Clone)]
pub struct DevConfig {
    pub command: String,
    pub url: String,
    pub cwd: Option<String>,
    pub open_devtools: bool,
}

/// A handle to control the application (window, devtools, etc.)
pub struct AppHandle<'a> {
    window: &'a winit::window::Window,
    browser: Option<&'a cef::Browser>,
}

impl<'a> AppHandle<'a> {
    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.focus_window();
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub fn toggle_tools(&self) {
        self.open_devtools();
    }

    pub fn open_devtools(&self) {
        if let Some(browser) = self.browser {
            if let Some(host) = browser.host() {
                let window_info = cef::WindowInfo::default();
                let settings = cef::BrowserSettings::default();
                host.show_dev_tools(Some(&window_info), None, Some(&settings), None);
            }
        }
    }
}

/// The main entry point for the library.
pub struct App {
    title: String,
    width: f64,
    height: f64,
    resizable: bool,
    start_hidden: bool,
    asset_resolver: Option<Arc<AssetResolver>>,
    router: CommandRouter,
    dev_config: Option<DevConfig>,
    on_ready: Option<Box<dyn Fn(&AppHandle) + Send + Sync>>,
    on_exit: Option<Box<dyn Fn() + Send + Sync>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            title: "Rust CEF App".to_string(),
            width: 1280.0,
            height: 800.0,
            resizable: true,
            start_hidden: false,
            asset_resolver: None,
            router: CommandRouter::new(),
            dev_config: None,
            on_ready: None,
            on_exit: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.start_hidden = !visible;
        self
    }

    pub fn dev_config(mut self, config: DevConfig) -> Self {
        self.dev_config = Some(config);
        self
    }

    pub fn on_ready<F>(mut self, callback: F) -> Self
    where
        F: Fn(&AppHandle) + Send + Sync + 'static,
    {
        self.on_ready = Some(Box::new(callback));
        self
    }

    pub fn on_exit<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_exit = Some(Box::new(callback));
        self
    }

    pub fn assets<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&str) -> Option<rust_embed::EmbeddedFile> + Send + Sync + 'static,
    {
        self.asset_resolver = Some(Arc::new(resolver));
        self
    }

    pub fn register_ipc<F>(mut self, command: &str, handler: F) -> Self
    where
        F: Fn(&serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
    {
        self.router
            .register(command, move |args, _proxy| handler(args));
        self
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        crate::error_reporting::install_panic_hook();

        // Ensure assets are provided
        let asset_resolver = self
            .asset_resolver
            .ok_or("Asset resolver must be provided via .assets()")?;

        // Wrap router in Arc for sharing
        let router = Arc::new(self.router);

        let args: Vec<String> = std::env::args().collect();
        let debug_flag = args.iter().any(|a| a == "--debug");
        set_debug_mode(debug_flag);

        let dev_flag = args.iter().any(|a| a == "--dev");
        crate::security::set_runtime_dev_mode(dev_flag);
        let is_subprocess = args.iter().any(|a| a.starts_with("--type="));
        let single_instance_mode = if !is_subprocess {
            Some(crate::single_instance::acquire(&args)?)
        } else {
            None
        };
        #[cfg(unix)]
        if matches!(
            single_instance_mode,
            Some(crate::single_instance::InstanceMode::Secondary)
        ) {
            return Ok(());
        }
        init_logging(dev_flag, debug_flag);
        tracing::info!(
            pid = std::process::id(),
            dev_mode = dev_flag,
            debug_mode = debug_flag,
            "application starting"
        );
        log_debug(&format!(
            "DEBUG: Main Process Started PID: {}",
            std::process::id()
        ));

        let is_bundle = std::env::current_exe().map_or(false, |p| {
            p.to_string_lossy().contains(".app/Contents/MacOS")
        });
        let log_prefix = if is_subprocess { "[HELPER]" } else { "[MAIN]" };

        print_debug(&format!("{} PID: {}", log_prefix, std::process::id()));
        print_debug(&format!(
            "{} Current Dir: {:?}",
            log_prefix,
            std::env::current_dir()
        ));

        let mut dev_process = None;
        let mut dev_target_url = None;
        let mut dev_target_port = None;
        let deep_link_arg = crate::security::extract_deep_link_arg(&args);
        let launch_context = crate::single_instance::extract_launch_context(&args);
        let _ = crate::state::set_launch_context(launch_context.clone());

        let start_url = if is_subprocess {
            "about:blank".to_string()
        } else if dev_flag && !is_bundle {
            if let Some(config) = &self.dev_config {
                dev_target_port = parse_port_from_url(&config.url);
                if let Some(port) = dev_target_port {
                    kill_processes_on_port(port);
                }

                print_debug(&format!(
                    "{} DEBUG: Dev mode detected. Starting dev server: {}",
                    log_prefix, config.command
                ));

                // Split command into program and args
                let mut parts = config.command.split_whitespace();
                if let Some(program) = parts.next() {
                    let mut cmd = std::process::Command::new(program);
                    cmd.args(parts);

                    if let Some(cwd) = &config.cwd {
                        let absolute_cwd = std::fs::canonicalize(cwd);
                        print_debug(&format!(
                            "{} DEBUG: Resolved CWD for dev server: {:?}",
                            log_prefix, absolute_cwd
                        ));
                        cmd.current_dir(cwd);
                    }
                    // Disable Vite/Bun opening the default system browser
                    cmd.env("BROWSER", "none");

                    // Explicitly inherit stdout/stderr so we can see bun output
                    // Use piped output to avoid FD conflicts with CEF and to prefix logs
                    cmd.stdout(std::process::Stdio::piped());
                    cmd.stderr(std::process::Stdio::piped());

                    // Set process group to 0 to create a new PGID (same as PID)
                    // This allows us to kill the whole tree (bun -> node -> vite) later
                    cmd.process_group(0);

                    print_debug(&format!(
                        "{} DEBUG: Spawning command: '{}' (PGID: New)",
                        log_prefix, config.command
                    ));

                    match cmd.spawn() {
                        Ok(mut child) => {
                            tracing::info!(pid = child.id(), "{} dev server spawned", log_prefix);

                            // Spawn threads to pipe output
                            if let Some(stdout) = child.stdout.take() {
                                std::thread::spawn(move || {
                                    use std::io::{BufRead, BufReader};
                                    let reader = BufReader::new(stdout);
                                    for line in reader.lines() {
                                        if let Ok(l) = line {
                                            tracing::info!("[dev-server] {}", l);
                                        }
                                    }
                                });
                            }

                            if let Some(stderr) = child.stderr.take() {
                                std::thread::spawn(move || {
                                    use std::io::{BufRead, BufReader};
                                    let reader = BufReader::new(stderr);
                                    for line in reader.lines() {
                                        if let Ok(l) = line {
                                            tracing::warn!("[dev-server] {}", l);
                                        }
                                    }
                                });
                            }

                            dev_process = Some(child);
                        }
                        Err(e) => {
                            tracing::error!("{} failed to spawn dev server: {}", log_prefix, e);
                            tracing::error!(
                                "{} ensure '{}' is in PATH and 'frontend' exists",
                                log_prefix,
                                program
                            );
                        }
                    }
                }

                // Store the target URL to load once ready
                dev_target_url = Some(
                    deep_link_arg
                        .as_deref()
                        .map(|deep_link| {
                            crate::security::make_deep_link_start_url(&config.url, deep_link)
                        })
                        .unwrap_or_else(|| config.url.clone()),
                );

                // Create a temporary loading file
                let loading_html = format!("<html><body style='background:#111;color:#eee;font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh'><h1>Starting Dev Server...</h1><p>Waiting for {}</p></body></html>", config.url);

                let temp_dir = std::env::temp_dir();
                let loading_path = temp_dir.join("rust_cef_loading.html");
                if let Err(e) = std::fs::write(&loading_path, loading_html) {
                    eprintln!("ERROR: Failed to write loading file: {}", e);
                    "about:blank".to_string()
                } else {
                    format!("file://{}", loading_path.to_string_lossy())
                }
            } else {
                "http://localhost:5173".to_string()
            }
        } else {
            // Production / Fallback
            let base_url = crate::app::get_start_url();
            deep_link_arg
                .as_deref()
                .map(|deep_link| crate::security::make_deep_link_start_url(&base_url, deep_link))
                .unwrap_or(base_url)
        };

        if !is_subprocess {
            crate::security::enforce_url_policy(&start_url, dev_flag)
                .map_err(|err| format!("Invalid startup URL: {err}"))?;
        }

        // 0. LOAD LIBRARY (MacOS specific)
        #[cfg(target_os = "macos")]
        let _loader = {
            // ... [Logic remains same, but omitted for brevity in replace block if possible? No, must replace contiguous]
            // I will duplicate the library loader logic to ensure correctness as I am editing a large block
            if !is_subprocess {
                print_info("Loading CEF Library (macOS)");
            }

            // Re-check for internal flags
            if is_subprocess {
                print_debug("DEBUG: Helper process detected during load");
            }
            if is_bundle {
                print_debug("DEBUG: Running in App Bundle");
            } else {
                print_debug("DEBUG: Running in Dev/Command-line");
            }

            // If we are not in a bundle, we must lie to LibraryLoader and say we are NOT a helper
            // because in dev mode, the helper IS the main executable, and it should look for the framework
            // in the same place as the main process (relative to target/debug).
            let loader_is_helper = is_subprocess && is_bundle;

            let loader = cef::library_loader::LibraryLoader::new(
                &std::env::current_exe().expect("cannot get current exe"),
                loader_is_helper,
            );

            if !loader.load() {
                // If it fails, report specifically
                if is_subprocess {
                    eprintln!("CRITICAL: Helper failed to load CEF library! is_bundle={} loader_is_helper={}", is_bundle, loader_is_helper);
                } else {
                    eprintln!("CRITICAL: Main process failed to load CEF library!");
                }
                panic!("cannot load cef library");
            }

            print_debug("DEBUG: CEF library loaded successfully");
            loader
        };

        print_debug("DEBUG: Checking API hash");

        let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);

        // 1. PARSE ARGS & HANDLE SUBPROCESSES
        print_debug("DEBUG: Parsing command line arguments");
        let args = cef::args::Args::new();

        // Create App instance early to handle subprocesses
        print_debug("DEBUG: Creating App instance");
        let mut app = AppBuilder::build(asset_resolver.clone());
        if !is_subprocess {
            print_info("App instance created");
        }

        if is_subprocess {
            print_debug("========================================");
            print_debug("DEBUG: SUBPROCESS DETECTED");
            log_debug("DEBUG: SUBPROCESS DETECTED");

            print_debug("DEBUG: Executing subprocess with cef::execute_process");
            log_debug("DEBUG: Executing subprocess with cef::execute_process");
            let code = cef::execute_process(
                Some(args.as_main_args()),
                Some(&mut app),
                std::ptr::null_mut(),
            );
            print_debug(&format!("DEBUG: Subprocess exiting with code: {}", code));
            log_debug(&format!("DEBUG: Subprocess exiting with code: {}", code));
            std::process::exit(code as i32);
        }

        print_debug("========================================");
        print_debug("DEBUG: MAIN PROCESS INITIALIZATION");
        print_debug("========================================");

        // 2. INITIALIZE WINIT EVENT LOOP
        print_debug("DEBUG: Creating event loop");

        // Create event loop with custom configuration for macOS
        #[cfg(target_os = "macos")]
        let event_loop = {
            let mut builder = EventLoopBuilder::<AppEvent>::with_user_event();
            // CRITICAL: Disable winit's default menu creation on macOS
            builder.with_default_menu(false);
            builder.build().unwrap()
        };

        #[cfg(not(target_os = "macos"))]
        let event_loop = EventLoopBuilder::<AppEvent>::with_user_event()
            .build()
            .unwrap();

        let proxy = event_loop.create_proxy();
        router.set_proxy(proxy.clone());
        let _ = EVENT_LOOP_PROXY.set(std::sync::Mutex::new(proxy.clone()));
        crate::state::init_global_shortcut_manager()
            .map_err(|err| format!("Failed to initialize global shortcut manager: {err}"))?;
        #[cfg(unix)]
        if let Some(crate::single_instance::InstanceMode::Primary(listener)) = single_instance_mode
        {
            let proxy = proxy.clone();
            crate::single_instance::start_listener(
                listener,
                Box::new(move |payload| {
                    let context = crate::single_instance::extract_launch_context(&payload.args);
                    let _ = proxy.send_event(AppEvent::ExternalLaunch(context));
                }),
            );
        }

        let mut window_manager = WindowManager::new();
        let mut config_manager = config::ConfigManager::new();
        let workspace_config = config::WorkspaceConfig::load();

        if let Some(badges) = &workspace_config.badges {
            if let Some(taskbar_path) = &badges.taskbar {
                crate::tray::set_tray_icon_path(taskbar_path.clone());
            }
        }

        print_debug("DEBUG: Creating main window");

        let mut main_window_builder = WindowBuilder::new()
            .with_title(&self.title)
            .with_visible(!self.start_hidden)
            .with_resizable(self.resizable);

        if let Some(bounds) = &config_manager.current.main_window {
            main_window_builder = main_window_builder
                .with_inner_size(winit::dpi::LogicalSize::new(bounds.width, bounds.height))
                .with_position(winit::dpi::LogicalPosition::new(bounds.x, bounds.y));
        } else {
            main_window_builder = main_window_builder
                .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height));
        }

        let main_window = main_window_builder.build(&event_loop).unwrap();

        let main_window_id = window_manager.insert(main_window, None);

        let _runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        // Start backend in background (Optional? Maybe should be user controlled?)
        // runtime.spawn(async move {
        //     start_server().await;
        // });

        // 3. INITIALIZE CEF
        // app is already created above
        print_debug("DEBUG: Creating CEF settings");
        let mut settings = cef::Settings::default();
        let sandbox_enabled = std::env::var("RUST_CEF_ENABLE_SANDBOX")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        settings.no_sandbox = if sandbox_enabled { 0 } else { 1 };
        settings.log_severity = cef::LogSeverity::INFO;

        #[cfg(target_os = "macos")]
        {
            print_debug("DEBUG: Setting external_message_pump for macOS");
            settings.external_message_pump = 1;
        }

        // Set paths
        print_debug("DEBUG: Setting CEF paths");
        if let Ok(exe_path) = std::env::current_exe() {
            print_debug(&format!("DEBUG: Exe path: {:?}", exe_path));
            if let Some(parent) = exe_path.parent() {
                let framework_path =
                    parent.join("../Frameworks/Chromium Embedded Framework.framework");
                let resources_path = parent;

                print_debug(&format!("DEBUG: Framework path: {:?}", framework_path));
                print_debug(&format!("DEBUG: Resources path: {:?}", resources_path));

                settings.framework_dir_path =
                    cef::CefString::from(framework_path.to_str().unwrap());
                settings.resources_dir_path =
                    cef::CefString::from(resources_path.to_str().unwrap());
                settings.locales_dir_path =
                    cef::CefString::from(resources_path.join("locales").to_str().unwrap());

                // On macOS, we need to handle two cases:
                // 1. Packaged (.app)
                // 2. Dev (cargo run)

                let is_bundle = exe_path.to_string_lossy().contains(".app/Contents/MacOS");
                if is_bundle {
                    print_debug(
                        "DEBUG: Detected App Bundle environment. Using explicit Helper path.",
                    );
                    let helper_path = parent
                        .join("../Frameworks/Rust CEF Helper.app/Contents/MacOS/Rust CEF Helper");
                    if helper_path.exists() {
                        print_debug(&format!("DEBUG: Found helper at {:?}", helper_path));
                        settings.browser_subprocess_path =
                            cef::CefString::from(helper_path.to_str().unwrap());
                    } else {
                        eprintln!(
                            "WARNING: Helper not found at {:?}, falling back to auto-discovery",
                            helper_path
                        );
                    }
                } else {
                    print_debug(
                        "DEBUG: Detected Development environment. Using Self as subprocess.",
                    );
                    settings.browser_subprocess_path =
                        cef::CefString::from(exe_path.to_str().unwrap());
                }

                // Use a safe cache directory outside the bundle
                if let Some(mut cache_dir) = std::env::temp_dir().canonicalize().ok() {
                    cache_dir.push("rust-cef-cache");
                    print_debug(&format!("DEBUG: Cache path: {:?}", cache_dir));
                    settings.root_cache_path = cef::CefString::from(cache_dir.to_str().unwrap());
                } else {
                    settings.root_cache_path =
                        cef::CefString::from(parent.join("cef_cache").to_str().unwrap());
                }
            }
        }

        let init_result = cef::initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            std::ptr::null_mut(),
        );

        if init_result != 1 {
            panic!("CEF initialization failed! This usually means another instance of the app is already running and holding a lock on the CEF cache directory. Please close all other instances and try again.");
        }
        print_info("CEF Library Started");
        tracing::info!("CEF initialized");

        // Initialize System Tray
        print_debug("DEBUG: Initializing System Tray");
        let tray_menu = tray::create_app_menu(); // Use tray menu logic
        let _tray_icon = tray::create_tray_icon(&tray_menu);
        print_debug("DEBUG: System Tray initialized");

        // Initialize Application Menu (macOS)
        print_debug("DEBUG: Initializing Application Menu Bar");
        let app_menu_handles = menus::create_app_menu_bar();

        #[cfg(target_os = "macos")]
        {
            print_debug("DEBUG: Calling init_for_nsapp()");
            app_menu_handles.menu.init_for_nsapp();
            print_debug("DEBUG: Menu initialized for NSApp");

            if let Some(badges) = &workspace_config.badges {
                if let Some(dock_path) = &badges.dock {
                    use objc::runtime::Object;
                    use objc::{class, msg_send, sel, sel_impl};
                    use std::ffi::CString;

                    print_debug(&format!(
                        "DEBUG: Setting macOS Dock icon from: {}",
                        dock_path
                    ));

                    let c_path = CString::new(dock_path.clone()).unwrap_or_default();
                    #[allow(unexpected_cfgs)]
                    unsafe {
                        let ns_string_class = class!(NSString);
                        let ns_path: *mut Object = msg_send![ns_string_class, alloc];
                        let ns_path: *mut Object =
                            msg_send![ns_path, initWithUTF8String: c_path.as_ptr()];

                        let ns_image_class = class!(NSImage);
                        let ns_image: *mut Object = msg_send![ns_image_class, alloc];
                        let ns_image: *mut Object =
                            msg_send![ns_image, initWithContentsOfFile: ns_path];

                        if !ns_image.is_null() {
                            let ns_app_class = class!(NSApplication);
                            let app: *mut Object = msg_send![ns_app_class, sharedApplication];
                            let _: () = msg_send![app, setActivationPolicy:0isize];
                            let _: () = msg_send![app, setApplicationIconImage: ns_image];
                            print_debug("DEBUG: Successfully set macOS Dock icon");
                        } else {
                            print_debug("DEBUG: Failed to load macOS Dock icon, NSImage was null");
                        }
                    }
                }
            }
        }

        let mut browser_settings = cef::BrowserSettings::default();
        browser_settings.local_storage = cef::State::DISABLED;

        print_debug(&format!("DEBUG: Start URL: {}", start_url));

        print_debug("DEBUG: Creating IcyClient");
        // Pass the proxy to the client
        let (_client, client_handlers) = IcyClient::new(router.clone(), Some(proxy.clone()));

        print_debug("DEBUG: Building Client from handlers");
        let mut client = ClientBuilder::build(client_handlers);
        print_debug("DEBUG: Client built");

        print_debug("DEBUG: Creating WindowInfo");
        let window_info = {
            let mut info = cef::WindowInfo::default();

            #[cfg(target_os = "macos")]
            {
                print_debug("DEBUG: Configuring window for macOS");
                if let Some(managed) = window_manager.get(main_window_id) {
                    if let Ok(handle) = managed.window.window_handle() {
                        if let RawWindowHandle::AppKit(appkit_handle) = handle.as_raw() {
                            let view = appkit_handle.ns_view.as_ptr() as *mut std::ffi::c_void;
                            print_debug(&format!("DEBUG: Got AppKit view: {:?}", view));

                            let bounds = cef::Rect {
                                x: 0,
                                y: 0,
                                width: managed.window.inner_size().width as i32,
                                height: managed.window.inner_size().height as i32,
                            };
                            print_debug(&format!("DEBUG: Window bounds: {:?}", bounds));

                            info = info.set_as_child(view as _, &bounds);
                        }
                    }
                }
            }

            if let Some(managed) = window_manager.get(main_window_id) {
                let size = managed.window.inner_size();
                info.bounds.width = size.width as i32;
                info.bounds.height = size.height as i32;
                info.bounds.x = 0;
                info.bounds.y = 0;

                #[cfg(target_os = "windows")]
                {
                    if let Ok(handle) = managed.window.window_handle() {
                        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                            info.parent_window = win32_handle.hwnd.get() as _;
                        }
                    }
                }
            }
            info
        };

        print_info("DEBUG: Creating browser with cef::browser_host_create_browser_sync");
        let browser = cef::browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&cef::CefString::from(start_url.as_str())),
            Some(&browser_settings),
            None,
            None,
        );

        if browser.is_none() {
            panic!("Browser creation failed!");
        }

        if let Some(b) = &browser {
            window_manager.attach_browser(main_window_id, b.clone());
        }

        print_debug("DEBUG: ✓ Browser created successfully");
        tracing::info!("Browser created");
        log_debug("DEBUG: Browser created successfully");

        // Force initial resize
        print_debug("DEBUG: Forcing initial resize");
        if let Some(managed) = window_manager.get(main_window_id) {
            if let Some(browser) = &managed.browser {
                if let Some(host) = browser.host() {
                    host.was_resized();
                    print_debug("DEBUG: Initial resize triggered");
                }
            }
        }

        // If in dev mode with a target URL, spawn a background poller
        if let Some(target_url) = dev_target_url {
            if let Some(browser) = &browser {
                let browser_clone = browser.clone();
                tracing::info!(url = %target_url, "waiting for dev server before navigating");
                std::thread::spawn(move || {
                    if let Ok(url) = url::Url::parse(&target_url) {
                        if let Some(port) = url.port() {
                            let start = std::time::Instant::now();
                            let timeout = std::time::Duration::from_secs(60);

                            loop {
                                if std::net::TcpStream::connect(("localhost", port)).is_ok() {
                                    tracing::info!(port, url = %target_url, "dev server is ready; loading target URL");
                                    std::thread::sleep(std::time::Duration::from_millis(200));
                                    if let Some(frame) = browser_clone.main_frame() {
                                        tracing::debug!("loading target URL in main frame");
                                        frame.load_url(Some(&cef::CefString::from(
                                            target_url.as_str(),
                                        )));
                                    } else {
                                        tracing::error!("could not get main frame to load dev URL");
                                    }
                                    break;
                                }
                                if start.elapsed() > timeout {
                                    tracing::warn!(url = %target_url, "timeout waiting for dev server");
                                    break;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(250));
                            }
                        } else if let Some(frame) = browser_clone.main_frame() {
                            tracing::info!(url = %target_url, "dev URL has no port; loading directly");
                            frame.load_url(Some(&cef::CefString::from(target_url.as_str())));
                        }
                    }
                });
            }
        }

        // Extract on_ready callback to move into loop
        let on_ready_callback = self.on_ready;
        let on_exit_callback = self.on_exit;
        let open_devtools_on_ready = self
            .dev_config
            .as_ref()
            .map(|config| config.open_devtools)
            .unwrap_or(false);

        // 5. RUN THE EVENT LOOP
        print_debug("========================================");

        // Fix winit crash on macOS due to missing selector
        #[cfg(target_os = "macos")]
        unsafe {
            fix_winit_crash();
        }

        print_debug("DEBUG: ENTERING EVENT LOOP");
        print_debug("========================================");
        let mut counter = 0;
        let mut next_cef_pump_time: Option<std::time::Instant> = Some(std::time::Instant::now());
        let mut focused_window_id = Some(main_window_id);

        // We will mutate the tray icon's inner state if needed, but tray_icon::TrayIcon typically takes &self for set_icon.
        // We still need to own it or keep it alive. We'll shadow it.
        let tray_icon = _tray_icon;

        let _ = event_loop.run(move |event, window_target| {
            // KEEP HANDLES ALIVE: Move them into the closure
            let _ = &app_menu_handles;
            let _ = &tray_menu;
            let _ = &tray_icon;

            while let Ok(shortcut_event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
                let state = match shortcut_event.state {
                    global_hotkey::HotKeyState::Pressed => "pressed",
                    global_hotkey::HotKeyState::Released => "released",
                };
                if let Err(err) = crate::state::push_global_shortcut_event(shortcut_event.id, state)
                {
                    tracing::warn!("failed to enqueue global shortcut event: {}", err);
                } else if let Ok(events) = crate::state::list_global_shortcuts() {
                    if let Some(shortcut) = events
                        .into_iter()
                        .find(|shortcut| shortcut.hotkey.id() == shortcut_event.id)
                    {
                        emit_browser_event(
                            &window_manager,
                            main_window_id,
                            "global-shortcut",
                            serde_json::json!({
                                "id": shortcut.id,
                                "accelerator": shortcut.accelerator,
                                "state": state,
                            }),
                        );
                    }
                }
            }

            match event {
                Event::UserEvent(AppEvent::ScheduleMessagePumpWork(delay_ms)) => {
                    if delay_ms <= 0 {
                        next_cef_pump_time = Some(std::time::Instant::now());
                    } else {
                        next_cef_pump_time = Some(
                            std::time::Instant::now()
                                + std::time::Duration::from_millis(delay_ms as u64),
                        );
                    }
                }
                // Handle ContentLoaded event: Show window only when content is ready
                Event::UserEvent(AppEvent::ContentLoaded) => {
                    print_info("ContentLoaded event received. Showing window.");
                    // Now trigger the on_ready callback (which shows window)
                    if let Some(callback) = &on_ready_callback {
                        if let Some(managed) = window_manager.get(main_window_id) {
                            if !managed.window.is_visible().unwrap_or(false) {
                                let handle = AppHandle {
                                    window: &managed.window,
                                    browser: managed.browser.as_ref(),
                                };
                                if open_devtools_on_ready {
                                    handle.open_devtools();
                                }
                                callback(&handle);
                            }
                        }
                    }
                }
                Event::UserEvent(AppEvent::ExternalLaunch(context)) => {
                    let _ = crate::state::set_launch_context(context.clone());
                    let payload = serde_json::json!({
                        "deep_link": context.deep_link,
                        "files": context.files,
                    });
                    let _ = crate::state::push_app_event(crate::state::AppBridgeEvent {
                        event: "external-launch".to_string(),
                        payload: payload.clone(),
                    });
                    emit_browser_event(&window_manager, main_window_id, "external-launch", payload);
                    if let Some(managed) = window_manager.get(main_window_id) {
                        managed.window.set_visible(true);
                        managed.window.focus_window();
                    }
                    print_info("External launch forwarded to primary instance");
                }
                Event::UserEvent(AppEvent::CreateWindow(config)) => {
                    print_info(&format!("Received CreateWindow request: {}", config.url));
                    let restored_bounds = config
                        .persist_key
                        .as_deref()
                        .and_then(|key| config_manager.get_child_window_bounds(key))
                        .cloned();
                    let width = restored_bounds
                        .as_ref()
                        .map(|bounds| bounds.width as f64)
                        .unwrap_or(config.width);
                    let height = restored_bounds
                        .as_ref()
                        .map(|bounds| bounds.height as f64)
                        .unwrap_or(config.height);
                    let mut builder = WindowBuilder::new()
                        .with_title(&config.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(width, height))
                        .with_visible(!config.start_hidden)
                        .with_resizable(config.resizable);

                    if let Some(bounds) = &restored_bounds {
                        builder = builder
                            .with_position(winit::dpi::LogicalPosition::new(bounds.x, bounds.y));
                    } else if let (Some(x), Some(y)) = (config.x, config.y) {
                        builder = builder.with_position(winit::dpi::LogicalPosition::new(x, y));
                    }

                    if let Some(frameless) = config.frameless {
                        builder = builder.with_decorations(!frameless);
                    }
                    if let Some(transparent) = config.transparent {
                        builder = builder.with_transparent(transparent);
                    }
                    if let Some(always_on_top) = config.always_on_top {
                        builder = builder.with_window_level(if always_on_top {
                            WindowLevel::AlwaysOnTop
                        } else {
                            WindowLevel::Normal
                        });
                    }
                    if let Some(kiosk) = config.kiosk {
                        if kiosk {
                            builder = builder
                                .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                        }
                    }
                    if let Some(icon_bytes) = &config.icon {
                        if let Ok(img) = image::load_from_memory(icon_bytes) {
                            let rgba = img.into_rgba8();
                            let (width, height) = rgba.dimensions();
                            if let Ok(icon) =
                                winit::window::Icon::from_rgba(rgba.into_raw(), width, height)
                            {
                                builder = builder.with_window_icon(Some(icon));
                            }
                        }
                    }

                    let new_window = builder.build(window_target).unwrap();

                    let new_id = window_manager.insert(new_window, config.persist_key.clone());

                    let mut info = cef::WindowInfo::default();

                    if let Some(managed) = window_manager.get(new_id) {
                        #[cfg(target_os = "macos")]
                        if let Ok(handle) = managed.window.window_handle() {
                            if let RawWindowHandle::AppKit(appkit_handle) = handle.as_raw() {
                                let view = appkit_handle.ns_view.as_ptr() as *mut std::ffi::c_void;
                                let bounds = cef::Rect {
                                    x: 0,
                                    y: 0,
                                    width: managed.window.inner_size().width as i32,
                                    height: managed.window.inner_size().height as i32,
                                };
                                info = info.set_as_child(view as _, &bounds);
                            }
                        }

                        let size = managed.window.inner_size();
                        info.bounds.width = size.width as i32;
                        info.bounds.height = size.height as i32;

                        #[cfg(target_os = "windows")]
                        if let Ok(handle) = managed.window.window_handle() {
                            if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                                info.parent_window = win32_handle.hwnd.get() as _;
                            }
                        }
                    }

                    // Create the backend client handler for this new browser instance
                    // Note: Ideally we'd reuse the same IcyClient proxy setup
                    let (_new_client, new_client_handlers) =
                        IcyClient::new(router.clone(), Some(proxy.clone()));
                    let mut new_client_handler = ClientBuilder::build(new_client_handlers);

                    let new_browser = cef::browser_host_create_browser_sync(
                        Some(&info),
                        Some(&mut new_client_handler),
                        Some(&cef::CefString::from(config.url.as_str())),
                        Some(&browser_settings),
                        None,
                        None,
                    );

                    if let Some(b) = new_browser {
                        window_manager.attach_browser(new_id, b.clone());
                        if let Some(host) = b.host() {
                            host.was_resized();
                        }
                    }
                }
                Event::UserEvent(AppEvent::PrintToPdf(request)) => {
                    let target_browser = focused_window_id
                        .and_then(|window_id| window_manager.get(window_id))
                        .and_then(|managed| managed.browser.as_ref())
                        .or_else(|| {
                            window_manager
                                .get(main_window_id)
                                .and_then(|managed| managed.browser.as_ref())
                        });

                    if let Some(browser) = target_browser {
                        if let Some(host) = browser.host() {
                            let mut settings = cef::PdfPrintSettings::default();
                            settings.landscape = if request.landscape { 1 } else { 0 };
                            settings.print_background =
                                if request.print_background { 1 } else { 0 };
                            settings.display_header_footer =
                                if request.display_header_footer { 1 } else { 0 };
                            settings.scale = request.scale;

                            let mut callback =
                                PdfPrintCallbackBridge::new(request.response_tx.clone());
                            host.print_to_pdf(
                                Some(&cef::CefString::from(request.path.as_str())),
                                Some(&settings),
                                Some(&mut callback),
                            );
                        } else {
                            let _ = request
                                .response_tx
                                .send(Err("Browser host unavailable for PDF printing".to_string()));
                        }
                    } else {
                        let _ = request.response_tx.send(Err(
                            "No active browser available for PDF printing".to_string(),
                        ));
                    }
                }
                Event::UserEvent(AppEvent::StartDownload(request)) => {
                    let target_browser = focused_window_id
                        .and_then(|window_id| window_manager.get(window_id))
                        .and_then(|managed| managed.browser.as_ref())
                        .or_else(|| {
                            window_manager
                                .get(main_window_id)
                                .and_then(|managed| managed.browser.as_ref())
                        });

                    if let Some(browser) = target_browser {
                        let browser_id = browser.identifier();
                        let pending_download = crate::state::PendingDownload {
                            path: request.path.clone(),
                            show_dialog: request.show_dialog,
                        };

                        if let Err(error) =
                            crate::state::set_pending_download(browser_id, pending_download)
                        {
                            let _ = request.response_tx.send(Err(error));
                        } else if let Some(host) = browser.host() {
                            host.start_download(Some(&cef::CefString::from(request.url.as_str())));
                            let _ = request.response_tx.send(Ok(serde_json::json!({
                                "status": "started",
                                "url": request.url,
                            })));
                        } else {
                            let _ = request
                                .response_tx
                                .send(Err("Browser host unavailable for download".to_string()));
                        }
                    } else {
                        let _ = request
                            .response_tx
                            .send(Err("No active browser available for download".to_string()));
                    }
                }
                Event::UserEvent(AppEvent::SetDecorations(id_opt, show)) => {
                    let target_id = id_opt.unwrap_or(main_window_id);
                    if let Some(managed) = window_manager.get(target_id) {
                        managed.window.set_decorations(show);
                    }
                }
                Event::UserEvent(AppEvent::SetAlwaysOnTop(id_opt, always_on_top)) => {
                    let target_id = id_opt.unwrap_or(main_window_id);
                    if let Some(managed) = window_manager.get(target_id) {
                        managed.window.set_window_level(if always_on_top {
                            WindowLevel::AlwaysOnTop
                        } else {
                            WindowLevel::Normal
                        });
                    }
                }
                Event::UserEvent(AppEvent::SetWindowIcon(id_opt, icon_opt)) => {
                    let target_id = id_opt.unwrap_or(main_window_id);
                    if let Some(managed) = window_manager.get(target_id) {
                        managed.window.set_window_icon(icon_opt);
                    }
                }
                Event::UserEvent(AppEvent::SetKiosk(id_opt, is_kiosk)) => {
                    let target_id = id_opt.unwrap_or(main_window_id);
                    if let Some(managed) = window_manager.get(target_id) {
                        if is_kiosk {
                            managed
                                .window
                                .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                        } else {
                            managed.window.set_fullscreen(None);
                        }
                    }
                }
                Event::UserEvent(AppEvent::SetTrayBadge(count)) => {
                    let icon = crate::tray::generate_tray_icon_with_badge(if count > 0 {
                        Some(count)
                    } else {
                        None
                    });
                    let _ = tray_icon.set_icon(Some(icon));
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Focused(is_focused),
                    ..
                } => {
                    if is_focused {
                        focused_window_id = window_manager.find_id_by_window_id(window_id);
                    } else if focused_window_id == window_manager.find_id_by_window_id(window_id) {
                        focused_window_id = None;
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Moved(position),
                    ..
                } => {
                    let mut main_bounds = None;
                    let mut child_bounds = None;
                    for (id, managed) in window_manager.windows_iter() {
                        if managed.window.id() == window_id {
                            if *id == main_window_id {
                                let log_pos =
                                    position.to_logical::<i32>(managed.window.scale_factor());
                                let log_size = managed
                                    .window
                                    .inner_size()
                                    .to_logical::<u32>(managed.window.scale_factor());
                                main_bounds = Some(config::WindowBounds {
                                    x: log_pos.x,
                                    y: log_pos.y,
                                    width: log_size.width,
                                    height: log_size.height,
                                });
                            } else if let Some(persist_key) = &managed.persist_key {
                                let log_pos =
                                    position.to_logical::<i32>(managed.window.scale_factor());
                                let log_size = managed
                                    .window
                                    .inner_size()
                                    .to_logical::<u32>(managed.window.scale_factor());
                                child_bounds = Some((
                                    persist_key.clone(),
                                    config::WindowBounds {
                                        x: log_pos.x,
                                        y: log_pos.y,
                                        width: log_size.width,
                                        height: log_size.height,
                                    },
                                ));
                            }
                            break;
                        }
                    }
                    if let Some(bounds) = main_bounds {
                        config_manager.update_main_window_bounds(
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        );
                        config_manager.save();
                    }
                    if let Some((persist_key, bounds)) = child_bounds {
                        config_manager.update_child_window_bounds(
                            persist_key,
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        );
                        config_manager.save();
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let mut main_bounds = None;
                    let mut child_bounds = None;
                    // Match window_id against our managed windows
                    for (id, managed) in window_manager.windows_iter() {
                        if managed.window.id() == window_id {
                            if *id == main_window_id {
                                if let Ok(pos) = managed.window.outer_position() {
                                    let log_pos =
                                        pos.to_logical::<i32>(managed.window.scale_factor());
                                    let log_size =
                                        size.to_logical::<u32>(managed.window.scale_factor());
                                    main_bounds = Some(config::WindowBounds {
                                        x: log_pos.x,
                                        y: log_pos.y,
                                        width: log_size.width,
                                        height: log_size.height,
                                    });
                                }
                            } else if let Some(persist_key) = &managed.persist_key {
                                if let Some(bounds) = logical_window_bounds(&managed.window) {
                                    child_bounds = Some((persist_key.clone(), bounds));
                                }
                            }

                            if let Some(browser) = &managed.browser {
                                if let Some(host) = browser.host() {
                                    host.was_resized();
                                }
                            }
                            break;
                        }
                    }

                    if let Some(bounds) = main_bounds {
                        config_manager.update_main_window_bounds(
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        );
                        config_manager.save();
                    }
                    if let Some((persist_key, bounds)) = child_bounds {
                        config_manager.update_child_window_bounds(
                            persist_key,
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        );
                        config_manager.save();
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    // Check if this is the main window
                    let mut is_main = false;

                    let mut found_id = None;
                    for (id, managed) in window_manager.windows_iter() {
                        if managed.window.id() == window_id {
                            found_id = Some(*id);
                            if *id == main_window_id {
                                is_main = true;
                            }
                            break;
                        }
                    }

                    if is_main {
                        if dev_flag {
                            window_target.exit();
                        } else {
                            if let Some(managed) = window_manager.get(main_window_id) {
                                managed.window.set_visible(false);
                            }
                        }
                    } else if let Some(id) = found_id {
                        if focused_window_id == Some(id) {
                            focused_window_id = Some(main_window_id);
                        }
                        // Close secondary window
                        if let Some(managed) = window_manager.remove(id) {
                            if let Some(browser) = managed.browser {
                                if let Some(host) = browser.host() {
                                    host.close_browser(1);
                                }
                            }
                        }
                    }
                }
                Event::LoopExiting => {
                    // Kill the dev process and its children (process group) FIRST
                    // This ensures that even if CEF shutdown crashes, we don't leave zombie processes
                    if let Some(mut child) = dev_process.take() {
                        let pid = child.id();
                        print_debug(&format!(
                            "DEBUG: Killing dev server process group (PGID: {})",
                            pid
                        ));

                        // Try to kill the process group (-PID) using libc::kill
                        unsafe {
                            let pgid = -(pid as i32);
                            print_debug(&format!("DEBUG: Sending SIGTERM to PGID {}", pgid));
                            let ret = libc::kill(pgid, libc::SIGTERM);
                            if ret != 0 {
                                print_debug(&format!(
                                    "DEBUG: Failed to send SIGTERM: {}",
                                    std::io::Error::last_os_error()
                                ));
                            } else {
                                // Give the process group a brief moment to shut down gracefully
                                std::thread::sleep(std::time::Duration::from_millis(50));

                                // Escalate to SIGKILL to ensure the process group is dead
                                print_debug(&format!(
                                    "DEBUG: Escalating to SIGKILL to PGID {}",
                                    pgid
                                ));
                                let _ = libc::kill(pgid, libc::SIGKILL);
                            }
                        }

                        // Also try normal kill as fallback
                        let _ = child.kill();
                        let _ = child.wait();
                    }

                    if let Some(port) = dev_target_port {
                        kill_processes_on_port(port);
                    }

                    if let Some(callback) = &on_exit_callback {
                        print_debug("DEBUG: Executing on_exit callback");
                        callback();
                    }

                    config_manager.save();

                    print_debug("DEBUG: Event loop exiting, shutting down CEF");
                    cef::shutdown();
                    print_debug("DEBUG: CEF shutdown complete");
                }
                Event::AboutToWait => {
                    // Handle Menu Events
                    if let Ok(event) = muda::MenuEvent::receiver().try_recv() {
                        let id = event.id.clone();
                        if id == muda::MenuId::new(tray::MENU_ITEM_QUIT_ID) {
                            window_target.exit();
                        } else if id == muda::MenuId::new(tray::MENU_ITEM_SHOW_HIDE_ID) {
                            if let Some(managed) = window_manager.get(main_window_id) {
                                if managed.window.is_visible().unwrap_or(false) {
                                    managed.window.set_visible(false);
                                } else {
                                    managed.window.set_visible(true);
                                    managed.window.focus_window();
                                }
                            }
                        }
                        // View Menu Actions
                        else if id == muda::MenuId::new(menus::MENU_VIEW_RELOAD) {
                            let target_browser = focused_window_id
                                .and_then(|window_id| window_manager.get(window_id))
                                .and_then(|managed| managed.browser.as_ref())
                                .or(browser.as_ref());
                            if let Some(browser) = target_browser {
                                browser.reload();
                            }
                        } else if id == muda::MenuId::new(menus::MENU_VIEW_DEVTOOLS) {
                            let target_browser = focused_window_id
                                .and_then(|window_id| window_manager.get(window_id))
                                .and_then(|managed| managed.browser.as_ref())
                                .or(browser.as_ref());
                            if let Some(browser) = target_browser {
                                if let Some(host) = browser.host() {
                                    let window_info = cef::WindowInfo::default();
                                    let settings = cef::BrowserSettings::default();
                                    host.show_dev_tools(
                                        Some(&window_info),
                                        None,
                                        Some(&settings),
                                        None,
                                    );
                                }
                            }
                        }
                        // Dynamic Counter
                        else if id == muda::MenuId::new(menus::MENU_VIEW_COUNTER) {
                            counter += 1;
                            eprintln!("DEBUG: Counter incremented to {}", counter);
                            app_menu_handles
                                .view_counter_item
                                .set_text(format!("Counter: {}", counter));
                        }
                        // Always on Top
                        else if id == muda::MenuId::new(menus::MENU_WINDOW_ALWAYS_ON_TOP) {
                            if let Some(managed) = window_manager.get(main_window_id) {
                                let current =
                                    app_menu_handles.window_always_on_top_item.is_checked();
                                let new_state = !current;
                                managed.window.set_window_level(if new_state {
                                    WindowLevel::AlwaysOnTop
                                } else {
                                    WindowLevel::Normal
                                });
                                app_menu_handles
                                    .window_always_on_top_item
                                    .set_checked(new_state);
                                eprintln!("DEBUG: Always on Top toggled to {}", new_state);
                            }
                        }
                        // Dialogs
                        else if id == muda::MenuId::new(menus::MENU_DIALOG_INFO) {
                            rfd::MessageDialog::new()
                                .set_title("Info")
                                .set_description("This is an info dialog.")
                                .set_level(rfd::MessageLevel::Info)
                                .show();
                        } else if id == muda::MenuId::new(menus::MENU_DIALOG_WARNING) {
                            rfd::MessageDialog::new()
                                .set_title("Warning")
                                .set_description("This is a warning dialog.")
                                .set_level(rfd::MessageLevel::Warning)
                                .show();
                        } else if id == muda::MenuId::new(menus::MENU_DIALOG_ERROR) {
                            rfd::MessageDialog::new()
                                .set_title("Error")
                                .set_description("This is an error dialog.")
                                .set_level(rfd::MessageLevel::Error)
                                .show();
                        } else if id == muda::MenuId::new(menus::MENU_DIALOG_CONFIRM) {
                            let result = rfd::MessageDialog::new()
                                .set_title("Confirmation")
                                .set_description("Do you want to proceed?")
                                .set_buttons(rfd::MessageButtons::OkCancel)
                                .show();
                            eprintln!("DEBUG: Confirmation result: {}", result);
                        }
                    }

                    // Handle Tray Icon Events
                    if let Ok(_event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                        // eprintln!("DEBUG: Tray event: {:?}", event);
                    }

                    // Pump CEF message loop if it's time
                    let now = std::time::Instant::now();
                    let mut pumped = false;
                    if let Some(target) = next_cef_pump_time {
                        if now >= target {
                            cef::do_message_loop_work();
                            pumped = true;
                        }
                    }

                    // Schedule next wake up for winit
                    if pumped {
                        // We pumped, wait for CEF to schedule the next pump via on_schedule_message_pump_work
                        // But also make sure we don't completely freeze if CEF misses a cycle, max 50ms wait.
                        window_target.set_control_flow(ControlFlow::WaitUntil(
                            std::time::Instant::now() + std::time::Duration::from_millis(50),
                        ));
                    } else if let Some(target) = next_cef_pump_time {
                        window_target.set_control_flow(ControlFlow::WaitUntil(target));
                    } else {
                        // CEF hasn't scheduled anything (Wait), but realistically we always set a safety net.
                        window_target.set_control_flow(ControlFlow::WaitUntil(
                            std::time::Instant::now() + std::time::Duration::from_millis(50),
                        ));
                    }
                }
                _ => (),
            }
        });

        // NOTE: On macOS this code is unreachable because run() never returns (except maybe on error)
        // Cleanup is handled in Event::LoopExiting above.
        Ok(())
    }
}

fn parse_port_from_url(url: &str) -> Option<u16> {
    url::Url::parse(url).ok().and_then(|parsed| parsed.port())
}

fn kill_processes_on_port(port: u16) {
    let port_selector = format!(":{port}");
    let output = match std::process::Command::new("lsof")
        .args(["-ti", port_selector.as_str()])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            tracing::warn!(port, "failed to run lsof for dev-port cleanup: {}", error);
            return;
        }
    };

    if !output.status.success() && output.stdout.is_empty() {
        return;
    }

    let pids: Vec<i32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<i32>().ok())
        .collect();

    if pids.is_empty() {
        return;
    }

    tracing::warn!(port, pids = ?pids, "killing processes bound to dev port");

    for pid in pids {
        unsafe {
            if libc::kill(pid, libc::SIGKILL) != 0 {
                tracing::warn!(
                    port,
                    pid,
                    "failed to SIGKILL dev-port owner: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}

#[cfg(target_os = "macos")]
unsafe fn fix_winit_crash() {
    print_debug("DEBUG: Checking for winit crash condition...");

    // Get shared application
    let app: Option<&AnyObject> = msg_send![class!(NSApplication), sharedApplication];

    // Declare class_addMethod once for the whole function
    #[link(name = "objc", kind = "dylib")]
    extern "C" {
        fn class_addMethod(
            cls: &AnyClass,
            name: Sel,
            imp: *const std::ffi::c_void,
            types: *const std::ffi::c_char,
        ) -> Bool;
    }

    if let Some(app) = app {
        let cls: &AnyClass = msg_send![app, class];
        let cls_name = cls.name();
        print_debug(&format!("DEBUG: Current NSApp class: {:?}", cls_name));

        // 1. Patch isHandlingSendEvent
        let selector = sel!(isHandlingSendEvent);
        if !cls.responds_to(selector) {
            print_debug(&format!(
                "DEBUG: Class {} missing isHandlingSendEvent - patching...",
                cls_name
            ));

            // Define implementation: returns NO (false)
            extern "C" fn is_handling_send_event_impl(_this: &AnyObject, _cmd: Sel) -> Bool {
                // print_debug("DEBUG: Shim isHandlingSendEvent called!"); // Too noisy
                Bool::NO
            }

            let types = std::ffi::CString::new("B@:").unwrap();
            let success = class_addMethod(
                cls,
                selector,
                is_handling_send_event_impl as *const _,
                types.as_ptr(),
            );

            if success.as_bool() {
                print_debug("DEBUG: Patched isHandlingSendEvent successfully!");
            } else {
                print_debug("DEBUG: Failed to patch isHandlingSendEvent!");
            }
        } else {
            // eprintln!("DEBUG: Class {} already has isHandlingSendEvent", cls_name);
        }

        // 2. Patch setHandlingSendEvent:
        let set_selector = sel!(setHandlingSendEvent:);
        if !cls.responds_to(set_selector) {
            print_debug(&format!(
                "DEBUG: Class {} missing setHandlingSendEvent: - patching...",
                cls_name
            ));

            // Define implementation: accepts BOOL, returns void
            extern "C" fn set_handling_send_event_impl(_this: &AnyObject, _cmd: Sel, _val: Bool) {
                // print_debug("DEBUG: Shim setHandlingSendEvent: called!");
            }

            let types = std::ffi::CString::new("v@:B").unwrap();
            let success = class_addMethod(
                cls,
                set_selector,
                set_handling_send_event_impl as *const _,
                types.as_ptr(),
            );

            if success.as_bool() {
                print_debug("DEBUG: Patched setHandlingSendEvent: successfully!");
            } else {
                print_debug("DEBUG: Failed to patch setHandlingSendEvent:!");
            }
        }
    }
}
