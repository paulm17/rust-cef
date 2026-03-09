use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{WindowBuilder, WindowLevel},
};

#[cfg(target_os = "macos")]
use objc2::{
    class, msg_send, sel,
    runtime::{AnyClass, AnyObject, Bool, Sel},
};
#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;

use std::os::unix::process::CommandExt;

use cef::{self, ImplBrowser, ImplBrowserHost, ImplFrame};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::Arc;
pub mod client;
pub mod app;
pub mod ipc;
pub mod platform;
pub mod backend;
pub mod state;
pub mod assets;
pub mod tray;
pub mod menus;
pub mod debug_logger;
pub mod window_manager;

use client::{IcyClient, client::ClientBuilder};
use app::AppBuilder;
use ipc::bridge::CommandRouter;
use platform::scheme_handler::AssetResolver;
use debug_logger::{log_debug, print_debug, print_info, set_debug_mode};
use window_manager::WindowManager;

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub url: String,
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub resizable: bool,
    pub start_hidden: bool,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ContentLoaded,
    CreateWindow(WindowConfig),
}

/// Configuration for the Development Environment
#[derive(Clone)]
pub struct DevConfig {
    pub command: String,
    pub url: String,
    pub cwd: Option<String>,
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
        if let Some(browser) = self.browser {
            if let Some(host) = browser.host() {
                // If devtools are open, close them? CEF API doesn't have "is_devtools_open" easily.
                // We'll just show them for now as "toggle" usually implies separate control or toggle.
                // host.close_dev_tools(); // If we knew they were open.
                // For now, always show. User can close via UI.
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
        self.router.register(command, move |args, _proxy| handler(args));
        self
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure assets are provided
        let asset_resolver = self.asset_resolver
            .ok_or("Asset resolver must be provided via .assets()")?;
        
        // Wrap router in Arc for sharing
        let router = Arc::new(self.router);

        print_debug("╔════════════════════════════════════════╗");
        print_debug("║      APPLICATION STARTING              ║");
        print_debug(&format!("║      PID: {}                        ║", std::process::id()));
        print_debug("╚════════════════════════════════════════╝");
        print_debug(&format!("DEBUG: main() started, PID: {}", std::process::id()));

        let _ = tracing_subscriber::fmt::try_init();
        log_debug(&format!("DEBUG: Main Process Started PID: {}", std::process::id()));

        // Check for --debug flag
        if std::env::args().any(|a| a == "--debug") {
            set_debug_mode(true);
        }

        // Check for --dev flag
        let dev_flag = std::env::args().any(|a| a == "--dev");
        let is_bundle = std::env::current_exe()
             .map_or(false, |p| p.to_string_lossy().contains(".app/Contents/MacOS"));
        let is_subprocess = std::env::args().any(|a| a.starts_with("--type="));
        let log_prefix = if is_subprocess { "[HELPER]" } else { "[MAIN]" };
        
        print_debug(&format!("{} PID: {}", log_prefix, std::process::id()));
        print_debug(&format!("{} Current Dir: {:?}", log_prefix, std::env::current_dir()));

        let mut dev_process = None;
        let mut dev_target_url = None;

        let start_url = if dev_flag && !is_bundle && !is_subprocess {
             if let Some(config) = &self.dev_config {
                 print_debug(&format!("{} DEBUG: Dev mode detected. Starting dev server: {}", log_prefix, config.command));
                 
                 // Split command into program and args
                 let mut parts = config.command.split_whitespace();
                 if let Some(program) = parts.next() {
                     let mut cmd = std::process::Command::new(program);
                     cmd.args(parts);
                     
                     if let Some(cwd) = &config.cwd {
                         let absolute_cwd = std::fs::canonicalize(cwd);
                         print_debug(&format!("{} DEBUG: Resolved CWD for dev server: {:?}", log_prefix, absolute_cwd));
                         cmd.current_dir(cwd);
                     }
                     
                     // Explicitly inherit stdout/stderr so we can see bun output
                     // Use piped output to avoid FD conflicts with CEF and to prefix logs
                     cmd.stdout(std::process::Stdio::piped()); 
                     cmd.stderr(std::process::Stdio::piped());
                     
                     // Set process group to 0 to create a new PGID (same as PID)
                     // This allows us to kill the whole tree (bun -> node -> vite) later
                     cmd.process_group(0);

                     print_debug(&format!("{} DEBUG: Spawning command: '{}' (PGID: New)", log_prefix, config.command));

                     match cmd.spawn() {
                         Ok(mut child) => {
                             print_debug(&format!("{} DEBUG: Dev server spawned successfully with PID: {}", log_prefix, child.id()));
                             
                             // Spawn threads to pipe output
                             if let Some(stdout) = child.stdout.take() {
                                 std::thread::spawn(move || {
                                     use std::io::{BufRead, BufReader};
                                     let reader = BufReader::new(stdout);
                                     for line in reader.lines() {
                                         if let Ok(l) = line {
                                             print_debug(&format!("[BUN] {}", l));
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
                                             print_debug(&format!("[BUN ERROR] {}", l));
                                         }
                                     }
                                 });
                             }
                             
                             dev_process = Some(child);
                         }
                         Err(e) => {
                             eprintln!("{} ERROR: FAILED TO SPAWN DEV SERVER: {}", log_prefix, e);
                             eprintln!("{} HINT: Ensure '{}' is in your PATH and the directory 'frontend' exists.", log_prefix, program);
                         }
                     }
                 }
                
                 // Store the target URL to load once ready
                 dev_target_url = Some(config.url.clone());

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
             crate::app::get_start_url()
        };

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
                std::ptr::null_mut()
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
        let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build().unwrap();

        let proxy = event_loop.create_proxy();
        router.set_proxy(proxy.clone());
        
        let mut window_manager = WindowManager::new();

        print_debug("DEBUG: Creating main window");
        let main_window = WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
            .with_visible(!self.start_hidden)
            .with_resizable(self.resizable)
            .build(&event_loop)
            .unwrap();

        let main_window_id = window_manager.insert(main_window);

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
        settings.no_sandbox = 1;
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
                let framework_path = parent.join("../Frameworks/Chromium Embedded Framework.framework");
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
                    print_debug("DEBUG: Detected App Bundle environment. Using explicit Helper path.");
                    let helper_path = parent.join("../Frameworks/Rust CEF Helper.app/Contents/MacOS/Rust CEF Helper");
                    if helper_path.exists() {
                         print_debug(&format!("DEBUG: Found helper at {:?}", helper_path));
                         settings.browser_subprocess_path = cef::CefString::from(helper_path.to_str().unwrap());
                    } else {
                         eprintln!("WARNING: Helper not found at {:?}, falling back to auto-discovery", helper_path);
                    }
                } else {
                     print_debug("DEBUG: Detected Development environment. Using Self as subprocess.");
                     settings.browser_subprocess_path =
                        cef::CefString::from(exe_path.to_str().unwrap());
                }

                // Use a safe cache directory outside the bundle
                if let Some(mut cache_dir) = std::env::temp_dir().canonicalize().ok() {
                    cache_dir.push("rust-cef-cache");
                    print_debug(&format!("DEBUG: Cache path: {:?}", cache_dir));
                    settings.root_cache_path =
                        cef::CefString::from(cache_dir.to_str().unwrap());
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
            panic!("CEF initialization failed!");
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
                print_debug("DEBUG: Spawning background thread to wait for dev server...");
                std::thread::spawn(move || {
                    if let Ok(url) = url::Url::parse(&target_url) {
                        if let Some(port) = url.port() {
                           print_debug(&format!("DEBUG: Polling port {}...", port));
                           let start = std::time::Instant::now();
                           let timeout = std::time::Duration::from_secs(60); 
                           
                           loop {
                               if std::net::TcpStream::connect(("localhost", port)).is_ok() {
                                   print_info(&format!("Port {} ready! Loading URL: {}", port, target_url));
                                   std::thread::sleep(std::time::Duration::from_millis(200));
                                   if let Some(frame) = browser_clone.main_frame() {
                                       print_debug("DEBUG: Frame found, loading URL...");
                                       frame.load_url(Some(&cef::CefString::from(target_url.as_str())));
                                   } else {
                                       eprintln!("ERROR: Could not get main frame to load URL!");
                                   }
                                   break;
                               }
                               if start.elapsed() > timeout {
                                   eprintln!("WARNING: Timeout waiting for dev server.");
                                   break;
                               }
                               std::thread::sleep(std::time::Duration::from_millis(250));
                           }
                        }
                    }
                });
            }
        }

        // Extract on_ready callback to move into loop
        let on_ready_callback = self.on_ready;
        let on_exit_callback = self.on_exit;

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
        let _ = event_loop.run(move |event, window_target| {
            // KEEP HANDLES ALIVE: Move them into the closure
            let _ = &app_menu_handles; 
            let _ = &tray_menu;
            let _ = &_tray_icon;


            window_target.set_control_flow(ControlFlow::Poll);

            match event {
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
                                  callback(&handle);
                             }
                         }
                     }
                }
                Event::UserEvent(AppEvent::CreateWindow(config)) => {
                    print_info(&format!("Received CreateWindow request: {}", config.url));
                    let new_window = WindowBuilder::new()
                        .with_title(&config.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height))
                        .with_visible(!config.start_hidden)
                        .with_resizable(config.resizable)
                        .build(window_target)
                        .unwrap();

                    let new_id = window_manager.insert(new_window);

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
                    let (_new_client, new_client_handlers) = IcyClient::new(router.clone(), Some(proxy.clone()));
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
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    // Match window_id against our managed windows
                    for managed in window_manager.values() {
                         if managed.window.id() == window_id {
                              if let Some(browser) = &managed.browser {
                                  if let Some(host) = browser.host() {
                                      host.was_resized();
                                  }
                              }
                              break;
                         }
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
                         print_debug(&format!("DEBUG: Killing dev server process group (PGID: {})", pid));
                         
                         // Try to kill the process group (-PID) using libc::kill
                         unsafe {
                             let pgid = -(pid as i32);
                             print_debug(&format!("DEBUG: Sending SIGTERM to PGID {}", pgid));
                             let ret = libc::kill(pgid, libc::SIGTERM);
                             if ret != 0 {
                                 print_debug(&format!("DEBUG: Failed to kill process group: {}", std::io::Error::last_os_error()));
                             }
                         }
                             
                         // Also try normal kill as fallback
                         let _ = child.kill();
                         let _ = child.wait();
                     }

                     if let Some(callback) = &on_exit_callback {
                         print_debug("DEBUG: Executing on_exit callback");
                         callback();
                     }

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
                            if let Some(browser) = &browser {
                                browser.reload();
                            }
                        } else if id == muda::MenuId::new(menus::MENU_VIEW_DEVTOOLS) {
                            if let Some(browser) = &browser {
                                if let Some(host) = browser.host() {
                                    let window_info = cef::WindowInfo::default();
                                    let settings = cef::BrowserSettings::default();
                                    host.show_dev_tools(Some(&window_info), None, Some(&settings), None);
                                }
                            }
                        } 
                        // Dynamic Counter
                        else if id == muda::MenuId::new(menus::MENU_VIEW_COUNTER) {
                             counter += 1;
                             eprintln!("DEBUG: Counter incremented to {}", counter);
                             app_menu_handles.view_counter_item.set_text(format!("Counter: {}", counter));
                        }
                        // Always on Top
                        else if id == muda::MenuId::new(menus::MENU_WINDOW_ALWAYS_ON_TOP) {
                             if let Some(managed) = window_manager.get(main_window_id) {
                                 let current = app_menu_handles.window_always_on_top_item.is_checked();
                                 let new_state = !current;
                                 managed.window.set_window_level(if new_state { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal });
                                 app_menu_handles.window_always_on_top_item.set_checked(new_state);
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
                        }
                        else if id == muda::MenuId::new(menus::MENU_DIALOG_WARNING) {
                            rfd::MessageDialog::new()
                                .set_title("Warning")
                                .set_description("This is a warning dialog.")
                                .set_level(rfd::MessageLevel::Warning)
                                .show();
                        }
                        else if id == muda::MenuId::new(menus::MENU_DIALOG_ERROR) {
                            rfd::MessageDialog::new()
                                .set_title("Error")
                                .set_description("This is an error dialog.")
                                .set_level(rfd::MessageLevel::Error)
                                .show();
                        }
                        else if id == muda::MenuId::new(menus::MENU_DIALOG_CONFIRM) {
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

                    // log_debug("DEBUG: Loop tick"); // Too noisy
                    cef::do_message_loop_work();
                }
                _ => (),
            }
        });

        // NOTE: On macOS this code is unreachable because run() never returns (except maybe on error)
        // Cleanup is handled in Event::LoopExiting above.
        Ok(())
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
            print_debug(&format!("DEBUG: Class {} missing isHandlingSendEvent - patching...", cls_name));
            
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
            print_debug(&format!("DEBUG: Class {} missing setHandlingSendEvent: - patching...", cls_name));

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
