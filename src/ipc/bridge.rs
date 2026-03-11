use crate::debug_logger::print_debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Incoming IPC request from the frontend.
/// Matches the JSON shape sent by `window.rust.invoke(cmd, args)`.
#[derive(Debug, Deserialize)]
pub struct IpcRequest {
    pub cmd: String,
    #[serde(default)]
    pub args: Value,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct ShowMessageDialogRequest {
    pub level: String,
    pub title: String,
    pub message: String,
}

/// Outgoing IPC response to the frontend.
#[derive(Debug, Serialize)]
pub struct IpcResponse {
    pub id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl IpcResponse {
    pub fn ok(id: String, data: Value) -> Self {
        Self {
            id,
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(id: String, error: String) -> Self {
        Self {
            id,
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

/// Handler function signature: takes JSON args and an optional Winit proxy, returns JSON result or error string.
pub type CommandHandler = Box<
    dyn Fn(&Value, &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>) -> Result<Value, String>
        + Send
        + Sync,
>;

/// Routes IPC commands to registered handler functions.
pub struct CommandRouter {
    handlers: HashMap<String, CommandHandler>,
    proxy: Mutex<Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>>,
}

impl CommandRouter {
    pub fn new() -> Self {
        let mut router = Self {
            handlers: HashMap::new(),
            proxy: Mutex::new(None),
        };
        router.register_builtins();
        router
    }

    pub fn set_proxy(&self, proxy: winit::event_loop::EventLoopProxy<crate::AppEvent>) {
        if let Ok(mut p) = self.proxy.lock() {
            *p = Some(Arc::new(Mutex::new(proxy)));
        }
    }

    /// Register a command handler.
    pub fn register<F>(&mut self, command: &str, handler: F)
    where
        F: Fn(
                &Value,
                &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
            ) -> Result<Value, String>
            + Send
            + Sync
            + 'static,
    {
        self.handlers.insert(command.to_string(), Box::new(handler));
    }

    /// Dispatch a raw JSON string from cefQuery.
    /// Returns a JSON string response (always succeeds — errors are encoded in the response).
    pub fn dispatch(&self, raw_request: &str) -> String {
        let trimmed = raw_request.trim();
        if !trimmed.starts_with('{') {
            // Plain string — fall back to old echo behavior behavior for legacy support or debug
            print_debug("DEBUG: bridge::dispatch - Plain string, echoing...");
            return format!("Rust received: {}", raw_request);
        }

        // Parse the incoming JSON
        let request: IpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                // Not a structured IPC request — return error
                let resp = IpcResponse::err(String::new(), format!("Invalid IPC request: {}", e));
                return serde_json::to_string(&resp).unwrap_or_default();
            }
        };

        let id = request.id.clone();

        // Look up the handler
        let response = match self.handlers.get(&request.cmd) {
            Some(handler) => {
                // Execute the handler
                let proxy_ref = if let Ok(proxy_guard) = self.proxy.lock() {
                    proxy_guard.clone()
                } else {
                    None
                };

                match handler(&request.args, &proxy_ref) {
                    Ok(data) => IpcResponse::ok(id, data),
                    Err(e) => IpcResponse::err(id, e),
                }
            }
            None => IpcResponse::err(id, format!("Unknown command: '{}'", request.cmd)),
        };

        serde_json::to_string(&response).unwrap_or_default()
    }

    /// Register built-in demo commands.
    fn register_builtins(&mut self) {
        // greet: { name: "Paul" } → "Hello, Paul!"
        self.register("greet", |args, _| {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("World");
            Ok(serde_json::json!({
                "message": format!("Hello, {}!", name)
            }))
        });

        // echo: returns whatever args were sent
        self.register("echo", |args, _| Ok(args.clone()));

        // get_app_info: returns application metadata
        self.register("get_app_info", |_args, _| {
            Ok(serde_json::json!({
                "name": "Rust + CEF Shell",
                "version": env!("CARGO_PKG_VERSION"),
                "rust_version": "1.x",
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            }))
        });

        // show_message_dialog: { level, title, message } -> bool
        self.register("show_message_dialog", |args, _| {
            let req: ShowMessageDialogRequest =
                serde_json::from_value(args.clone()).map_err(|e| format!("Invalid args: {}", e))?;

            let mut dialog = rfd::MessageDialog::new()
                .set_title(&req.title)
                .set_description(&req.message);

            // Map level string to MessageLevel/Buttons
            // rfd 0.14 MessageDialog parsing
            let result = match req.level.as_str() {
                "error" => {
                    dialog = dialog.set_level(rfd::MessageLevel::Error);
                    dialog.show();
                    true
                }
                "warning" => {
                    dialog = dialog.set_level(rfd::MessageLevel::Warning);
                    dialog.show();
                    true
                }
                "confirm" => {
                    // Confirm dialog usually has Ok/Cancel or Yes/No
                    dialog = dialog.set_buttons(rfd::MessageButtons::OkCancel);
                    let res = dialog.show();
                    matches!(
                        res,
                        rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes
                    )
                }
                _ => {
                    // Default to Info
                    dialog = dialog.set_level(rfd::MessageLevel::Info);
                    dialog.show();
                    true
                }
            };

            Ok(serde_json::json!(result))
        });

        // Window creation and configuration
        self.register("create_window", crate::ipc::commands::window::create_window);
        self.register(
            "set_window_config",
            crate::ipc::commands::window::set_window_config,
        );

        // OS Integration
        self.register("set_badge_count", crate::ipc::commands::os::set_badge_count);
        self.register(
            "get_launch_context",
            crate::ipc::commands::os::get_launch_context,
        );
        self.register(
            "show_notification",
            crate::ipc::commands::os::show_notification,
        );
        self.register(
            "register_global_shortcut",
            crate::ipc::commands::os::register_global_shortcut,
        );
        self.register(
            "unregister_global_shortcut",
            crate::ipc::commands::os::unregister_global_shortcut,
        );
        self.register(
            "list_global_shortcuts",
            crate::ipc::commands::os::list_global_shortcuts,
        );
        self.register(
            "poll_global_shortcut_events",
            crate::ipc::commands::os::poll_global_shortcut_events,
        );
        self.register("poll_app_events", crate::ipc::commands::os::poll_app_events);
        self.register("clipboard_read_text", |args, _| {
            crate::ipc::commands::clipboard::clipboard_read_text(args)
        });
        self.register("clipboard_write_text", |args, _| {
            crate::ipc::commands::clipboard::clipboard_write_text(args)
        });
        self.register("clipboard_read_image", |args, _| {
            crate::ipc::commands::clipboard::clipboard_read_image(args)
        });
        self.register("clipboard_write_image", |args, _| {
            crate::ipc::commands::clipboard::clipboard_write_image(args)
        });
        self.register("clipboard_clear", |args, _| {
            crate::ipc::commands::clipboard::clipboard_clear(args)
        });
        self.register(
            "start_download",
            crate::ipc::commands::browser::start_download,
        );
        self.register("print_to_pdf", crate::ipc::commands::browser::print_to_pdf);

        // File System Commands
        self.register("read_file", |args, _| {
            crate::ipc::commands::fs::read_file(args)
        });
        self.register("read_file_binary", |args, _| {
            crate::ipc::commands::fs::read_file_binary(args)
        });
        self.register("write_file", |args, _| {
            crate::ipc::commands::fs::write_file(args)
        });
        self.register("write_file_binary", |args, _| {
            crate::ipc::commands::fs::write_file_binary(args)
        });
        self.register("exists", |args, _| crate::ipc::commands::fs::exists(args));
        self.register("read_dir", |args, _| {
            crate::ipc::commands::fs::read_dir(args)
        });
        self.register("get_metadata", |args, _| {
            crate::ipc::commands::fs::get_metadata(args)
        });
        self.register("create_file_stream_url", |args, _| {
            crate::ipc::commands::fs::create_file_stream_url(args)
        });

        // Dialog Commands
        self.register("show_open_dialog", |args, _| {
            crate::ipc::commands::dialog::show_open_dialog(args)
        });
        self.register("show_save_dialog", |args, _| {
            crate::ipc::commands::dialog::show_save_dialog(args)
        });
        self.register("show_pick_folder_dialog", |args, _| {
            crate::ipc::commands::dialog::show_pick_folder_dialog(args)
        });
    }
}
