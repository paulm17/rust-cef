# Building a Tauri-Like Desktop App with Rust + CEF
## Complete Implementation Roadmap

> **Your Goal:** Create a desktop application using Rust backend + CEF (Chromium) frontend, with feature parity to Electron/Tauri for your specific needs.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [V1 Milestone: Basic Desktop App](#v1-milestone-basic-desktop-app)
3. [V1 Implementation: Simple IPC Bridge](#v1-implementation-simple-ipc-bridge)
4. [V2 Milestone: Electron Feature Parity](#v2-milestone-electron-feature-parity)
5. [V2 Implementation: Advanced IPC Bridge](#v2-implementation-advanced-ipc-bridge)
6. [Complete Feature Matrix](#complete-feature-matrix)
7. [Timeline & Effort Estimates](#timeline--effort-estimates)

---

## Architecture Overview

### The Stack

```
┌─────────────────────────────────────────────────────┐
│  Frontend Layer (JavaScript/TypeScript)             │
│  ├─ Your Framework: React/Vue/Svelte/etc.          │
│  ├─ Runs in: CEF (Chromium Embedded Framework)     │
│  └─ UI Layer: HTML/CSS/JavaScript                   │
└──────────────────┬──────────────────────────────────┘
                   │
                   │ IPC Bridge (window.rust.invoke)
                   │
┌──────────────────▼──────────────────────────────────┐
│  Backend Layer (Rust)                               │
│  ├─ Native System Access                            │
│  ├─ File System Operations                          │
│  ├─ Database Connections                            │
│  ├─ Network Requests                                │
│  └─ OS Integration (Dialogs, Clipboard, Tray)      │
└─────────────────────────────────────────────────────┘
```

### How It Works: The Frontend-Backend Flow

```
User clicks button in React
    ↓
await window.rust.invoke('open-file-dialog', { filters: [...] })
    ↓
IPC Bridge serializes to JSON
    ↓
Rust receives command
    ↓
Rust opens NATIVE OS file dialog (rfd crate)
    ↓
Windows Explorer / macOS Finder / Linux GTK dialog appears
    ↓
User selects file
    ↓
Rust sends path back via IPC
    ↓
JavaScript Promise resolves
    ↓
React updates UI with file path
```

**Key Insight:** JavaScript frontend can only display UI and make requests. Rust backend does all the "desktop" stuff (files, clipboard, system tray, native dialogs, etc.). This is exactly how Electron works, but with Rust instead of Node.js.

---

## V1 Milestone: Basic Desktop App

### Goal
Create a functional desktop application with essential features needed for 90% of use cases.

### V1 Core Features (Must Have)

#### 1. Window Management ✅ (You Have This!)
- [x] Create main window
- [x] Basic window events (resize, close)
- [x] Cross-platform window (Windows, macOS, Linux)
- **Status:** Working in your current code

#### 2. Asset Loading (Week 1) ✅ (You Have This!)
- [x] Custom protocol handler (`app://localhost`)
- [x] Embed frontend assets with `rust-embed`
- [x] Load HTML/CSS/JS from binary (no HTTP server needed)
- [x] Proper MIME type detection
- **Crates:** `rust-embed`, `mime_guess`

#### 3. IPC Bridge - Simple Implementation (Week 1-2) ✅ (You Have This!)
- [x] JavaScript bridge: `window.rust.invoke(command, args)`
- [x] Rust command router with pattern matching
- [x] JSON serialization with `serde_json`
- [x] Error handling and Promise-based API
- [x] TypeScript type definitions
- **Crates:** `serde`, `serde_json`

#### 4. File System Operations (Week 2) ✅ (You Have This!)
- [x] Read files (text and binary)
- [x] Write files (text and binary)
- [x] Check file existence
- [x] List directory contents
- [x] File metadata (size, modified date)
- **Crates:** Built-in `std::fs`

#### 5. Native File Dialogs (Week 2) ✅ (You Have This!)
- [x] Open file dialog (single file)
- [x] Open file dialog (multiple files)
- [x] Save file dialog
- [x] Folder picker dialog
- [x] Custom file filters
- **Crates:** `rfd` (Rusty File Dialogs)

#### 6. System Tray (Week 2-3) ✅ (You Have This!)
- [x] Create system tray icon
- [x] Tray tooltip
- [x] Tray context menu
- [x] Tray click events
- [x] Show/hide window from tray
- **Crates:** `tray-icon`

#### 7. Application Menus (Week 3) ✅ (You Have This!)
- [x] Menu bar (File, Edit, Help, etc.)
- [x] Menu items with shortcuts
- [x] Checkable menu items
- [x] Dynamic menu updates
- [x] Separator items
- **Crates:** `muda`

#### 8. Clipboard Access (Week 3) - Low priority - ignore
- [ ] Read text from clipboard
- [ ] Write text to clipboard
- [ ] Read images from clipboard
- [ ] Write images to clipboard
- [ ] Clear clipboard
- **Crates:** `arboard`

#### 9. Message Dialogs (Week 3) ✅ (You Have This!)
- [x] Info dialog
- [x] Warning dialog
- [x] Error dialog
- [x] Confirmation dialog (OK/Cancel)
- **Crates:** `rfd::MessageDialog`

#### 10. Basic Packaging (Week 4)
- [ ] Configure `cargo-packager`
- [ ] Bundle CEF resources
- [ ] Create distributable for current OS
- [ ] Application icon
- **Tools:** `cargo-packager`

### V1 Success Criteria

✅ **You can build and distribute a desktop app that:**
- Loads a modern web frontend (React/Vue/Svelte)
- Opens native file dialogs
- Reads and writes files
- Has a system tray icon
- Has an application menu
- Uses clipboard
- Shows message boxes
- Can be packaged for distribution

**Time Estimate:** 4 weeks of focused work

---

## V1 Implementation: Simple IPC Bridge

### Why This Approach?

The simple IPC bridge uses:
- **Custom scheme handler** (`app://` protocol) OR fetch() to a special endpoint
- **ExecuteJavaScript** to inject the bridge code
- **All code runs in Browser Process** (no subprocess complexity)

**Advantages:**
- Works with your current codebase (no refactoring needed)
- Easier to debug (single process for Rust logic)
- Fast enough for 90% of desktop apps (~5-10ms latency)
- Simpler to understand and maintain

**When is this sufficient?**
- Productivity apps (note-taking, todo lists, document editors)
- Development tools (IDEs, git clients, API testers)
- Database clients
- Content creation tools
- Most CRUD applications

### Implementation Architecture

```
┌─────────────────────────────────────────────────────────┐
│ 1. Page Loads                                           │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 2. LoadHandler::on_load_end fires                       │
│    Inject window.rust.invoke via ExecuteJavaScript      │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 3. User Action in React/Vue                             │
│    Click button → call window.rust.invoke()             │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 4. Bridge makes fetch() to app://invoke                 │
│    Body: { command: "save-file", args: {...} }         │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 5. CEF routes to SchemeHandler (Browser Process)        │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 6. SchemeHandler parses JSON, routes command            │
│    match command {                                      │
│        "save-file" => handle_save_file(args),          │
│        "open-dialog" => handle_open_dialog(args),      │
│        _ => Err("Unknown command")                     │
│    }                                                    │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 7. Execute Rust function (file I/O, dialogs, etc.)     │
│    Serialize result to JSON                             │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 8. Return HTTP-like response with JSON body             │
└────────────────────┬────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────┐
│ 9. JavaScript fetch() resolves, Promise returns result  │
└─────────────────────────────────────────────────────────┘
```

### Code Structure for V1

```
src/
├── main.rs                     # Entry point, CEF init, event loop
├── app.rs                      # CEF App handler (command line args)
├── state.rs                    # NEW: Shared app state
│
├── client/
│   ├── mod.rs
│   ├── client.rs               # Client builder
│   └── handlers/
│       ├── mod.rs
│       ├── load_handler.rs     # MODIFIED: Inject IPC bridge
│       ├── lifespan_handler.rs
│       └── context_menu_handler.rs
│
├── ipc/                        # NEW: IPC system
│   ├── mod.rs
│   ├── bridge.rs               # Command router
│   ├── commands/               # Command implementations
│   │   ├── mod.rs
│   │   ├── filesystem.rs       # File operations
│   │   ├── dialogs.rs          # File/message dialogs
│   │   ├── clipboard.rs        # Clipboard operations
│   │   └── shell.rs            # Open URLs, show in folder
│   └── types.rs                # Shared types
│
├── platform/                   # NEW: Platform integrations
│   ├── mod.rs
│   ├── scheme_handler.rs       # Custom app:// protocol
│   ├── tray.rs                 # System tray
│   └── menu.rs                 # Application menu
│
└── backend/
    ├── mod.rs
    └── server.rs               # Optional: Axum backend
```

### Key Code Snippets for V1

#### 1. Inject Bridge JavaScript

```rust
// src/client/handlers/load_handler.rs

impl ImplLoadHandler for LoadHandlerBuilder {
    fn on_load_end(
        &self,
        browser: Option<&mut Browser>,
        frame: Option<&mut Frame>,
        _http_status_code: i32,
    ) {
        if let (Some(_browser), Some(frame)) = (browser, frame) {
            if frame.is_main() {
                let bridge_code = include_str!("../../../assets/bridge.js");
                frame.execute_java_script(bridge_code, "", 0);
                
                tracing::info!("IPC bridge injected");
            }
        }
    }
}
```

#### 2. Bridge JavaScript (assets/bridge.js)

```javascript
(function() {
    'use strict';
    
    const pendingRequests = new Map();
    let requestId = 0;
    
    window.rust = {
        invoke: async function(command, args = {}) {
            const id = requestId++;
            
            return new Promise(async (resolve, reject) => {
                try {
                    const response = await fetch('app://invoke', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                            id,
                            command,
                            args
                        })
                    });
                    
                    if (!response.ok) {
                        throw new Error(`IPC Error: ${response.status} ${response.statusText}`);
                    }
                    
                    const result = await response.json();
                    
                    if (result.error) {
                        reject(new Error(result.error));
                    } else {
                        resolve(result.data);
                    }
                } catch (error) {
                    console.error('[Rust IPC] Error:', error);
                    reject(error);
                }
            });
        }
    };
    
    console.log('[Rust IPC] Bridge initialized');
})();
```

#### 3. Custom Scheme Handler

```rust
// src/platform/scheme_handler.rs

use cef::{self, ResourceHandler, SchemeHandlerFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
struct IpcRequest {
    id: u32,
    command: String,
    args: Value,
}

#[derive(Serialize)]
struct IpcResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub struct IpcSchemeHandler {
    // State
}

impl ResourceHandler for IpcSchemeHandler {
    fn process_request(&mut self, request: &Request, callback: &Callback) -> bool {
        // 1. Parse POST body
        let post_data = request.get_post_data();
        // ... parse JSON ...
        
        // 2. Route command
        let result = match request.command.as_str() {
            "open-file-dialog" => handle_open_file_dialog(request.args),
            "save-file-dialog" => handle_save_file_dialog(request.args),
            "read-file" => handle_read_file(request.args),
            "write-file" => handle_write_file(request.args),
            "clipboard-read" => handle_clipboard_read(),
            "clipboard-write" => handle_clipboard_write(request.args),
            _ => Err(format!("Unknown command: {}", request.command)),
        };
        
        // 3. Serialize response
        let response = match result {
            Ok(data) => IpcResponse { data: Some(data), error: None },
            Err(e) => IpcResponse { data: None, error: Some(e) },
        };
        
        let json = serde_json::to_string(&response).unwrap();
        
        // 4. Return as HTTP response
        // Set headers: Content-Type: application/json
        // Set body: json
        callback.continue_();
        true
    }
}
```

#### 4. Command Implementations

```rust
// src/ipc/commands/dialogs.rs

use rfd::FileDialog;
use serde_json::{json, Value};

pub fn handle_open_file_dialog(args: Value) -> Result<Value, String> {
    let title = args["title"].as_str().unwrap_or("Open File");
    
    let mut dialog = FileDialog::new().set_title(title);
    
    // Add filters
    if let Some(filters) = args["filters"].as_array() {
        for filter in filters {
            let name = filter["name"].as_str().unwrap_or("Files");
            let exts: Vec<&str> = filter["extensions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .collect();
            
            dialog = dialog.add_filter(name, &exts);
        }
    }
    
    // Open native dialog (blocks until user selects)
    let result = dialog.pick_file();
    
    Ok(json!({
        "path": result.map(|p| p.to_string_lossy().to_string())
    }))
}

pub fn handle_save_file_dialog(args: Value) -> Result<Value, String> {
    let title = args["title"].as_str().unwrap_or("Save File");
    let default_name = args["defaultName"].as_str();
    
    let mut dialog = FileDialog::new().set_title(title);
    
    if let Some(name) = default_name {
        dialog = dialog.set_file_name(name);
    }
    
    let result = dialog.save_file();
    
    Ok(json!({
        "path": result.map(|p| p.to_string_lossy().to_string())
    }))
}
```

```rust
// src/ipc/commands/clipboard.rs

use arboard::Clipboard;
use serde_json::{json, Value};

pub fn handle_clipboard_read() -> Result<Value, String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    
    let text = clipboard.get_text()
        .map_err(|e| format!("Failed to read clipboard: {}", e))?;
    
    Ok(json!({ "text": text }))
}

pub fn handle_clipboard_write(args: Value) -> Result<Value, String> {
    let text = args["text"]
        .as_str()
        .ok_or("Missing 'text' parameter")?;
    
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    
    clipboard.set_text(text)
        .map_err(|e| format!("Failed to write clipboard: {}", e))?;
    
    Ok(json!({ "success": true }))
}
```

### TypeScript Definitions for V1

```typescript
// frontend/src/types/rust.d.ts

interface RustAPI {
  // File Dialogs
  invoke(command: 'open-file-dialog', args: {
    title?: string;
    filters?: Array<{ name: string; extensions: string[] }>;
    multiple?: boolean;
  }): Promise<{ path: string | null }>;
  
  invoke(command: 'save-file-dialog', args: {
    title?: string;
    defaultName?: string;
    filters?: Array<{ name: string; extensions: string[] }>;
  }): Promise<{ path: string | null }>;
  
  // File Operations
  invoke(command: 'read-file', args: {
    path: string;
    encoding?: 'utf8' | 'base64';
  }): Promise<{ content: string }>;
  
  invoke(command: 'write-file', args: {
    path: string;
    content: string;
    encoding?: 'utf8' | 'base64';
  }): Promise<{ success: boolean }>;
  
  // Clipboard
  invoke(command: 'clipboard-read'): Promise<{ text: string }>;
  invoke(command: 'clipboard-write', args: { text: string }): Promise<{ success: boolean }>;
  
  // Shell
  invoke(command: 'open-url', args: { url: string }): Promise<{ success: boolean }>;
  invoke(command: 'show-in-folder', args: { path: string }): Promise<{ success: boolean }>;
}

declare global {
  interface Window {
    rust: RustAPI;
  }
}

export {};
```

---

## V2 Milestone: Electron Feature Parity

### Goal
Match Electron's feature set for professional desktop applications. Add advanced features and optimize performance.

### V2 Enhancement Categories

#### Category A: Advanced Window Management

| Feature | Description | Rust Implementation | Priority |
|---------|-------------|---------------------|----------|
| Multiple Windows | Create/manage secondary browser windows natively | `WindowBuilder::new()` + `winit` event loop management | ✅ Done |
| Window State | Save/restore window position and size | Local JSON config (`dirs` crate) on window move/close | ✅ Done |
| Window Modes | Frameless, Transparent, Always on Top, Kiosk | `WindowBuilder` configuration flags (`winit`) | ✅ Done |
| OS Integration | macOS Dock badges, Windows taskbar progress | `objc` for macOS NSApp.dockTile, native tray updates | ✅ Done |
| Always On Top | Pin window above others | Winit `set_always_on_top()` | ✅ Done |
| Window Badges (macOS) | Dock icon badge count | Platform-specific API (`objc`) | ✅ Done |
| Progress Bar (Taskbar) | Windows taskbar progress | N/A (Omitted) | ⏩ Skipped |
| Kiosk Mode | Fullscreen lock | Winit `set_fullscreen()` | ✅ Done |

**Time Estimate:** 1-2 weeks

#### Category B: Advanced IPC & Performance

| Feature | Description | Rust Implementation | Priority |
|---------|-------------|---------------------|----------|
| Full CEF IPC | V8Handler + ProcessMessage | `CefRenderProcessHandler` | High |
| Streaming Data | Large file transfers | Custom streaming protocol | Medium |
| Binary IPC | Non-JSON data transfer | MessagePack or custom | Low |
| IPC Performance | Sub-millisecond latency | ProcessMessage optimization | Medium |
| Bidirectional Events | Rust → JS events | Event emitter pattern | High |

**Time Estimate:** 2-3 weeks

#### Category C: Enhanced System Integration

| Feature | Description | Rust Implementation | Priority |
|---------|-------------|---------------------|----------|
| Notifications | System notifications | `notify-rust` | High |
| Rich Notifications | Images, actions, sound | Platform-specific | Medium |
| Global Shortcuts | Hotkeys outside app | `global-hotkey` crate | Medium |
| Single Instance Lock | Prevent multiple launches | `single-instance` | High |
| Deep Linking | Handle `myapp://` URLs | OS protocol registration | Medium |
| Recent Documents | OS recent files list | Platform-specific | Low |
| File Associations | Open files with your app | OS registration | Medium |

**Time Estimate:** 1-2 weeks

#### Category D: Developer Experience

| Feature | Description | Rust Implementation | Priority |
|---------|-------------|---------------------|----------|
| DevTools Always Open | Auto-open inspector | CEF `ShowDevTools()` on start | High |
| Hot Reload | Auto-refresh on changes | File watcher + reload command | High |
| Debug Logging | Structured logs | `tracing` with filters | Medium |
| Error Reporting | Crash reports | `sentry` or custom | Medium |
| Auto-Update | Self-updating binary | `self_update` crate | High |

**Time Estimate:** 2-3 weeks

#### Category E: Advanced Web Features

| Feature | Description | Rust Implementation | Priority |
|---------|-------------|---------------------|----------|
| Print to PDF | Generate PDFs | CEF `PrintToPDF()` | Medium |
| Screenshot Capture | Capture page image | CEF off-screen rendering | Low |
| Download Manager | Handle file downloads | CEF `DownloadHandler` | Medium |
| Custom Context Menu | Override right-click | CEF `ContextMenuHandler` | Low |
| Find in Page | Text search | CEF `Find()` | Low |
| Zoom Control | Page zoom level | CEF `SetZoomLevel()` | Low |

**Time Estimate:** 1 week

#### Category F: Security & Sandboxing

| Feature | Description | Rust Implementation | Priority |
|---------|-------------|---------------------|----------|
| Content Security Policy | Restrict JS execution | CEF CSP headers | High |
| HTTPS Enforcement | Block insecure content | CEF request interceptor | Medium |
| Permission Management | Camera, mic, location | CEF permission handlers | Medium |
| Code Signing | Sign executables | Platform signing tools | High |

**Time Estimate:** 1 week

#### Category G: Professional Packaging

| Feature | Description | Rust Implementation | Priority |
|---------|-------------|---------------------|----------|
| Windows MSI | Windows installer | `cargo-packager` | High |
| macOS DMG | macOS disk image | `cargo-packager` | High |
| Linux Packages | .deb, .rpm, AppImage | `cargo-packager` | High |
| Auto-Updater | Background updates | `self_update` + backend | High |
| Code Signing | Trusted execution | Platform tools | High |
| Icon Sets | Multi-resolution icons | Image generation | Medium |

**Time Estimate:** 2-3 weeks

### V2 Total Time Estimate
**8-12 weeks** for complete Electron feature parity

---

## V2 Implementation: Advanced IPC Bridge

### Why Upgrade to Advanced IPC?

The advanced IPC bridge uses CEF's full multi-process architecture:
- **RenderProcessHandler** for direct V8 access
- **V8Handler** to inject native functions
- **ProcessMessage** for low-latency IPC
- **Separate renderer subprocess** with custom logic

**Advantages:**
- **10-100x faster** latency (<1ms vs 5-10ms)
- Direct V8 integration (can modify DOM, intercept APIs)
- More control over renderer process
- Closer to Electron's architecture
- Better for high-frequency IPC (real-time apps, games)

**When do you need this?**
- Real-time applications (chat, collaborative editing)
- Games or video editors
- High-frequency data streaming (stock tickers, live charts)
- Performance-critical applications
- Need to intercept browser APIs at V8 level

### Advanced Implementation Architecture

```
┌──────────────────────────────────────────────────────┐
│ Main Process (main.rs)                               │
│ ├─ Spawns Renderer subprocess                       │
│ ├─ Implements CefClient                             │
│ └─ Implements OnProcessMessageReceived               │
└──────────────────┬───────────────────────────────────┘
                   │
                   │ Spawns with custom App
                   │
┌──────────────────▼───────────────────────────────────┐
│ Renderer Subprocess (subprocess/mod.rs)              │
│ ├─ Implements CefRenderProcessHandler               │
│ ├─ OnContextCreated: Inject window.rust.invoke()    │
│ └─ Implements CefV8Handler                          │
└──────────────────┬───────────────────────────────────┘
                   │
                   │ Direct V8 injection
                   │
┌──────────────────▼───────────────────────────────────┐
│ V8 Context (JavaScript Environment)                  │
│ └─ window.rust.invoke() calls V8Handler directly     │
└──────────────────────────────────────────────────────┘

IPC Flow:
JS calls window.rust.invoke()
    ↓
V8Handler::execute() catches it (in Renderer Process)
    ↓
Create CefProcessMessage
    ↓
SendProcessMessage(PID_BROWSER, msg) → crosses process boundary
    ↓
OnProcessMessageReceived() in Main Process
    ↓
Execute Rust command handler
    ↓
Send result back via ProcessMessage OR ExecuteJavaScript
    ↓
Resolve Promise in JavaScript
```

### Code Structure for V2

```
src/
├── main.rs                     # Entry point with subprocess handling
├── app.rs                      # Browser process App handler
├── state.rs                    # Shared app state
│
├── subprocess/                 # NEW: Renderer process code
│   ├── mod.rs                  # Subprocess entry point
│   ├── app.rs                  # Subprocess App handler
│   ├── render_handler.rs       # RenderProcessHandler implementation
│   └── v8_handler.rs           # V8Handler implementation
│
├── client/
│   ├── mod.rs
│   ├── client.rs               # MODIFIED: Handle ProcessMessage
│   └── handlers/
│       ├── load_handler.rs     # No longer injects JS
│       ├── lifespan_handler.rs
│       └── context_menu_handler.rs
│
├── ipc/
│   ├── mod.rs
│   ├── bridge.rs               # Enhanced with ProcessMessage
│   ├── protocol.rs             # NEW: Message protocol definitions
│   ├── commands/
│   │   └── ...                 # Same command implementations
│   └── types.rs
│
├── platform/
│   ├── scheme_handler.rs       # Still needed for assets
│   ├── tray.rs
│   └── menu.rs
│
└── backend/
    └── server.rs
```

### Key Code Snippets for V2

#### 1. Subprocess Initialization

```rust
// src/main.rs

mod subprocess;

fn main() {
    // Parse args
    let args = cef::args::Args::new();
    
    // Check if this is a subprocess
    let process_type = std::env::args()
        .find(|arg| arg.starts_with("--type="))
        .and_then(|arg| arg.split('=').nth(1).map(String::from));
    
    if let Some(ptype) = process_type {
        // This is a subprocess - use custom handlers
        let subprocess_app = subprocess::SubprocessAppBuilder::build(ptype);
        
        let code = cef::execute_process(
            Some(args.as_main_args()),
            Some(&mut subprocess_app),
            std::ptr::null_mut()
        );
        
        std::process::exit(code as i32);
    }
    
    // Continue with main process initialization...
    // (Your existing code)
}
```

#### 2. Subprocess App Handler

```rust
// src/subprocess/app.rs

use cef::{App, ImplApp, RenderProcessHandler};

pub struct SubprocessApp {
    process_type: String,
}

impl SubprocessApp {
    pub fn new(process_type: String) -> Self {
        Self { process_type }
    }
}

pub struct SubprocessAppBuilder {
    app: SubprocessApp,
}

impl SubprocessAppBuilder {
    pub fn build(process_type: String) -> App {
        App::new(Self {
            app: SubprocessApp::new(process_type),
        })
    }
}

impl ImplApp for SubprocessAppBuilder {
    fn get_render_process_handler(&self) -> Option<RenderProcessHandler> {
        // Only for renderer process, not utility/gpu/etc
        if self.app.process_type == "renderer" {
            Some(crate::subprocess::render_handler::RenderHandlerBuilder::build())
        } else {
            None
        }
    }
}
```

#### 3. Render Process Handler

```rust
// src/subprocess/render_handler.rs

use cef::{Browser, Frame, ImplRenderProcessHandler, ProcessMessage, RenderProcessHandler, V8Context};

pub struct RenderHandler;

pub struct RenderHandlerBuilder;

impl RenderHandlerBuilder {
    pub fn build() -> RenderProcessHandler {
        RenderProcessHandler::new(Self)
    }
}

impl ImplRenderProcessHandler for RenderHandlerBuilder {
    fn on_context_created(
        &self,
        browser: Option<&mut Browser>,
        frame: Option<&mut Frame>,
        context: Option<&mut V8Context>,
    ) {
        if let (Some(_browser), Some(frame), Some(context)) = (browser, frame, context) {
            if frame.is_main() {
                // Get the global object (window)
                let global = context.get_global();
                
                // Create the rust object
                let rust_obj = V8Value::create_object();
                
                // Create the invoke function
                let v8_handler = crate::subprocess::v8_handler::V8HandlerBuilder::build();
                let invoke_fn = V8Value::create_function("invoke", v8_handler);
                
                // Attach: window.rust = { invoke: function() {...} }
                rust_obj.set_value("invoke", &invoke_fn);
                global.set_value("rust", &rust_obj);
                
                tracing::info!("V8 bridge injected via OnContextCreated");
            }
        }
    }
    
    fn on_process_message_received(
        &self,
        browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        _source_process: cef::ProcessId,
        message: Option<&mut ProcessMessage>,
    ) -> bool {
        // Handle responses from Browser Process
        if let Some(msg) = message {
            if msg.get_name() == "ipc_response" {
                // Get the request ID and result
                let args = msg.get_argument_list();
                let request_id = args.get_int(0);
                let result_json = args.get_string(1);
                
                // Execute JavaScript to resolve the Promise
                if let Some(browser) = browser {
                    if let Some(frame) = browser.get_main_frame() {
                        let js = format!(
                            "window.__rustResolve({}, {})",
                            request_id,
                            result_json
                        );
                        frame.execute_java_script(&js, "", 0);
                    }
                }
                
                return true;
            }
        }
        
        false
    }
}
```

#### 4. V8 Handler

```rust
// src/subprocess/v8_handler.rs

use cef::{Browser, ImplV8Handler, ProcessMessage, V8Handler, V8Value};

pub struct MyV8Handler {
    browser: Option<Browser>, // Store browser reference
}

pub struct V8HandlerBuilder;

impl V8HandlerBuilder {
    pub fn build() -> V8Handler {
        V8Handler::new(Self)
    }
}

impl ImplV8Handler for V8HandlerBuilder {
    fn execute(
        &self,
        name: Option<&cef::CefString>,
        _object: Option<&V8Value>,
        arguments: &[V8Value],
        _retval: Option<&mut V8Value>,
        _exception: Option<&mut cef::CefString>,
    ) -> bool {
        if name.and_then(|n| n.as_str()).map(|s| s == "invoke").unwrap_or(false) {
            // Arguments: (command: string, args: object, requestId: number)
            if arguments.len() >= 3 {
                let command = arguments[0].get_string_value();
                let args = arguments[1]; // V8 object - need to serialize
                let request_id = arguments[2].get_int_value();
                
                // Serialize args to JSON string
                let args_json = serialize_v8_to_json(&args);
                
                // Create ProcessMessage
                let msg = ProcessMessage::create("ipc_request");
                let msg_args = msg.get_argument_list();
                msg_args.set_int(0, request_id);
                msg_args.set_string(1, &command);
                msg_args.set_string(2, &args_json);
                
                // Send to Browser Process
                if let Some(browser) = &self.browser {
                    browser.send_process_message(cef::ProcessId::Browser, &msg);
                }
                
                return true;
            }
        }
        
        false
    }
}

fn serialize_v8_to_json(value: &V8Value) -> String {
    // Convert V8Value to JSON string
    // This is a simplified version - you'd want robust serialization
    if value.is_string() {
        format!("\"{}\"", value.get_string_value())
    } else if value.is_int() {
        value.get_int_value().to_string()
    } else if value.is_object() {
        // Recursively serialize object properties
        // For brevity, returning empty object
        "{}".to_string()
    } else {
        "null".to_string()
    }
}
```

#### 5. Browser Process Message Handler

```rust
// src/client/client.rs

impl ImplClient for ClientBuilder {
    fn on_process_message_received(
        &self,
        browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        source_process: cef::ProcessId,
        message: Option<&mut ProcessMessage>,
    ) -> bool {
        if source_process == cef::ProcessId::Renderer {
            if let Some(msg) = message {
                if msg.get_name() == "ipc_request" {
                    let args = msg.get_argument_list();
                    let request_id = args.get_int(0);
                    let command = args.get_string(1);
                    let args_json = args.get_string(2);
                    
                    // Parse args
                    let args_value: serde_json::Value = 
                        serde_json::from_str(&args_json).unwrap_or_default();
                    
                    // Route to command handler
                    let result = crate::ipc::bridge::handle_command(&command, args_value);
                    
                    // Serialize result
                    let result_json = serde_json::to_string(&result).unwrap();
                    
                    // Send response back to Renderer
                    let response = ProcessMessage::create("ipc_response");
                    let response_args = response.get_argument_list();
                    response_args.set_int(0, request_id);
                    response_args.set_string(1, &result_json);
                    
                    if let Some(browser) = browser {
                        browser.send_process_message(cef::ProcessId::Renderer, &response);
                    }
                    
                    return true;
                }
            }
        }
        
        false
    }
}
```

#### 6. JavaScript Promise Wrapper

```javascript
// Injected automatically by V8Handler, or in preload script

(function() {
    'use strict';
    
    const pendingRequests = new Map();
    let requestId = 0;
    
    // This function is called by Rust to resolve promises
    window.__rustResolve = function(id, resultJson) {
        const pending = pendingRequests.get(id);
        if (pending) {
            const result = JSON.parse(resultJson);
            if (result.error) {
                pending.reject(new Error(result.error));
            } else {
                pending.resolve(result.data);
            }
            pendingRequests.delete(id);
        }
    };
    
    // Override the invoke function to use promises
    const originalInvoke = window.rust.invoke;
    window.rust.invoke = function(command, args = {}) {
        return new Promise((resolve, reject) => {
            const id = requestId++;
            pendingRequests.set(id, { resolve, reject });
            
            // Call the native V8 function
            originalInvoke(command, args, id);
            
            // Timeout after 30 seconds
            setTimeout(() => {
                if (pendingRequests.has(id)) {
                    pendingRequests.delete(id);
                    reject(new Error('IPC timeout'));
                }
            }, 30000);
        });
    };
    
    console.log('[Rust IPC] Advanced bridge initialized (V8Handler)');
})();
```

### Performance Comparison

| Metric | Simple IPC (V1) | Advanced IPC (V2) |
|--------|-----------------|-------------------|
| **Latency** | 5-10ms | 0.1-1ms |
| **Throughput** | ~100 msg/sec | ~10,000 msg/sec |
| **Setup Complexity** | Low | High |
| **Debug Complexity** | Easy (single process) | Hard (multi-process) |
| **Memory Overhead** | Minimal | Moderate |
| **Use Case** | Most apps | Real-time apps |

### When to Migrate from V1 to V2

**Migrate if you experience:**
- Noticeable UI lag during IPC calls
- Need for real-time communication (< 5ms latency)
- High-frequency IPC (>1000 messages per second)
- Need to intercept browser APIs at V8 level

**Don't migrate if:**
- Your app feels responsive with V1
- IPC happens infrequently (on user actions only)
- Simplicity and maintainability are priorities
- You want easier debugging

---

## Complete Feature Matrix

### Core Features (V1)

| Feature | V1 Status | V2 Enhancement | Rust Crate | Complexity |
|---------|-----------|----------------|------------|------------|
| **Window Management** | ✅ Basic | Multiple windows | CEF + Winit | Low |
| **Asset Loading** | ✅ Custom protocol | Advanced caching | `rust-embed` | Low |
| **IPC Bridge** | ✅ Simple (fetch) | Advanced (V8) | Custom | Medium |
| **File System** | ✅ Full | Watching | `std::fs`, `notify` | Low |
| **File Dialogs** | ✅ Full | Remember locations | `rfd` | Low |
| **System Tray** | ✅ Full | Animations | `tray-icon` | Low |
| **App Menus** | ✅ Full | Recent items | `muda` | Low |
| **Clipboard** | ✅ Text + Images | Rich formats | `arboard` | Low |
| **Message Dialogs** | ✅ Basic | Custom dialogs | `rfd` | Low |
| **Packaging** | ✅ Basic | Auto-update | `cargo-packager` | Medium |

### Advanced Features (V2)

| Category | Feature | Priority | Effort | Rust Solution |
|----------|---------|----------|--------|---------------|
| **Windows** | Frameless | Medium | Low | Winit decorations |
| **Windows** | Transparent | Medium | Low | Winit transparency |
| **Windows** | Always On Top | Low | Low | Winit method |
| **Windows** | State Persistence | High | Low | Config file |
| **Windows** | Multi-Window | High | Medium | CEF instances |
| **Windows** | Progress Bar | Medium | Medium | Platform crates |
| **System** | Notifications | High | Low | `notify-rust` |
| **System** | Global Shortcuts | Medium | Low | `global-hotkey` |
| **System** | Single Instance | High | Low | `single-instance` |
| **System** | Deep Linking | Medium | Medium | OS registration |
| **System** | Power Management | Low | Low | `battery`, `keep-awake` |
| **Web** | DevTools | High | Low | CEF built-in |
| **Web** | Print to PDF | Medium | Low | CEF built-in |
| **Web** | Screenshots | Low | Medium | CEF off-screen |
| **Web** | Downloads | Medium | Medium | CEF handler |
| **Web** | Find in Page | Low | Low | CEF built-in |
| **Security** | CSP | High | Low | CEF headers |
| **Security** | Permissions | Medium | Medium | CEF handlers |
| **Security** | Code Signing | High | Medium | Platform tools |
| **DX** | Hot Reload | High | Medium | File watcher |
| **DX** | Auto-Update | High | High | `self_update` |
| **DX** | Error Reporting | Medium | Low | `sentry` |

---

## Timeline & Effort Estimates

### V1: Basic Desktop App (MVP)

| Week | Focus | Deliverables | Status |
|------|-------|--------------|--------|
| **Week 1** | Setup & Assets | Custom protocol, rust-embed integration | 📋 |
| **Week 2** | IPC Bridge | Simple bridge, command routing, TypeScript types | 📋 |
| **Week 3** | File Operations | Dialogs, read/write, clipboard, shell commands | 📋 |
| **Week 4** | System Integration | Tray, menus, message dialogs | 📋 |
| **Week 5** | Packaging | cargo-packager config, first distributable | 📋 |

**Total V1 Time:** 5 weeks (1 developer, full-time)

### V2: Feature Parity & Polish

| Phase | Focus | Time | Key Features |
|-------|-------|------|--------------|
| **Phase A** | Advanced Windows | 2 weeks | Multi-window, frameless, persistence |
| **Phase B** | Advanced IPC | 3 weeks | V8Handler, ProcessMessage, performance |
| **Phase C** | System Features | 2 weeks | Notifications, shortcuts, single instance |
| **Phase D** | Web Features | 1 week | Print, downloads, devtools |
| **Phase E** | Security | 1 week | CSP, permissions, signing |
| **Phase F** | DevEx | 2 weeks | Hot reload, auto-update, error reporting |
| **Phase G** | Packaging | 2 weeks | Multi-platform, signing, installers |

**Total V2 Time:** 13 weeks (1 developer, full-time)

**Combined Total:** 18 weeks (~4.5 months)

### Accelerated Timeline (If Needed)

**Focus on essentials only:**
- V1 Core: 3 weeks (skip packaging polish)
- V2 Essentials: 6 weeks (skip low-priority features)
- **Total: 9 weeks (~2 months)**

### Parallel Development

**If you have 2 developers:**
- Developer 1: IPC + Core Features (Weeks 1-5)
- Developer 2: Platform Integration (Weeks 3-7)
- **Total: 7-8 weeks**

---

## Getting Started: Your First Week

### Day 1-2: Project Organization
- [ ] Restructure code with new directory layout
- [ ] Add new dependencies to Cargo.toml
- [ ] Create placeholder files for new modules
- [ ] Set up basic logging with `tracing`

### Day 3-4: Custom Protocol Handler
- [ ] Implement `SchemeHandlerFactory` for `app://`
- [ ] Register scheme in CEF initialization
- [ ] Add `rust-embed` to bundle frontend assets
- [ ] Test loading assets via `app://localhost/index.html`

### Day 5: Simple IPC Bridge
- [ ] Create bridge.js with `window.rust.invoke()`
- [ ] Inject bridge in `LoadHandler::on_load_end`
- [ ] Implement basic command routing in Rust
- [ ] Test with simple "echo" command

### Weekend: First Real Feature
- [ ] Implement "open-file-dialog" command
- [ ] Test from React/Vue/JavaScript
- [ ] See native OS file dialog appear
- [ ] 🎉 Celebrate your first desktop feature!

---

## Recommended Tech Stack

### Frontend
```json
{
  "framework": "React 18+ / Vue 3+ / Svelte 4+",
  "language": "TypeScript",
  "bundler": "Vite",
  "styling": "Tailwind CSS / CSS Modules",
  "state": "Zustand / Pinia / Svelte stores"
}
```

### Backend (Rust)
```toml
[dependencies]
# CEF
cef = "0.x"  # Your CEF bindings

# IPC & Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async Runtime (if needed)
tokio = { version = "1", features = ["full"] }

# System Integration
rfd = "0.14"           # File dialogs
arboard = "3.4"        # Clipboard
tray-icon = "0.14"     # System tray
muda = "0.13"          # Menus
notify-rust = "4"      # Notifications

# Assets
rust-embed = "8"       # Embed frontend files
mime_guess = "2"       # MIME types

# Utilities
open = "5"             # Open URLs/files
trash = "4"            # Move to trash
directories = "5"      # Standard directories
single-instance = "0.3" # Prevent multi-launch

# Window management
winit = "0.29"         # Cross-platform windows
raw-window-handle = "0.6"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"
```

---

## Success Metrics

### V1 Success (You've reached MVP when...)
- ✅ User can install and launch your app
- ✅ App loads a modern web UI (React/Vue)
- ✅ User can open native file dialogs
- ✅ User can read and write files
- ✅ App has a system tray icon
- ✅ App has a working menu bar
- ✅ User can copy/paste with clipboard
- ✅ App shows native message boxes
- ✅ IPC latency < 10ms
- ✅ No crashes during normal use

### V2 Success (You've reached Production when...)
- ✅ All V1 criteria met
- ✅ Multiple windows work smoothly
- ✅ IPC latency < 1ms (if using advanced bridge)
- ✅ Notifications work on all platforms
- ✅ Single instance enforcement works
- ✅ Auto-update mechanism works
- ✅ Code signing for all platforms
- ✅ Professional installers (.msi, .dmg, .deb)
- ✅ Error reporting to backend
- ✅ Performance matches Electron/Tauri

---

## Common Pitfalls & Solutions

### Pitfall 1: CEF Subprocess Confusion
**Problem:** "My IPC doesn't work after adding RenderProcessHandler"
**Solution:** Make sure you're passing the subprocess App in `execute_process()`, not just exiting immediately.

### Pitfall 2: CORS Errors with Custom Protocol
**Problem:** "fetch() fails with CORS errors"
**Solution:** Register your scheme as "standard" and "cors_enabled" in CEF settings.

### Pitfall 3: File Dialog Blocks UI
**Problem:** "UI freezes when file dialog is open"
**Solution:** This is expected - native dialogs are modal. For non-blocking, use a separate thread or async runtime.

### Pitfall 4: Clipboard Timing Issues
**Problem:** "Clipboard read returns empty string"
**Solution:** Some platforms require a delay after write. Add a small sleep or retry logic.

### Pitfall 5: Packaging Path Issues
**Problem:** "App works in dev but crashes in production"
**Solution:** Use relative paths from `std::env::current_exe()`, not hard-coded paths.

---

## Next Steps

### Ready to Start V1?

1. **Reorganize your codebase** using the structure above
2. **Implement custom protocol handler** for asset loading
3. **Build the simple IPC bridge** with fetch()
4. **Add first real feature** (file dialog)
5. **Iterate on remaining features**

### Questions to Answer Before Starting

- [ ] What frontend framework will you use? (React/Vue/Svelte?)
- [ ] Do you need the Axum backend server, or is Rust IPC enough?
- [ ] What's your primary platform? (Windows/macOS/Linux)
- [ ] Do you need hot reload during development?
- [ ] What's your target file size for distributable?

---

## Resources & References

### Documentation
- **CEF Documentation:** https://bitbucket.org/chromiumembedded/cef/wiki/
- **Rust CEF Bindings:** (Check your specific crate docs)
- **Electron API Reference:** https://www.electronjs.org/docs/latest/api/

### Example Projects
- **rust-cef:** Look for examples in the crate repository
- **Tauri Source:** https://github.com/tauri-apps/tauri (for inspiration)
- **Electrobun:** https://github.com/blackboardsh/electrobun (architecture reference)

### Community
- Rust Discord: #desktop-dev channel
- CEF Forum: https://magpcss.org/ceforum/
- r/rust: Desktop app discussions

---

## Conclusion

You're building something unique: a desktop app with the power of Rust and the flexibility of modern web UI, without the overhead of Electron's Node.js runtime.

**Your advantages:**
- 🚀 **Performance:** Rust is faster than Node.js
- 🔒 **Security:** Compile-time safety, no eval()
- 📦 **Size:** Smaller binaries (no Node runtime)
- 🎯 **Control:** Direct system access, no FFI layer
- 🛠️ **Simplicity:** No framework abstraction, just Rust + CEF

**Start with V1, iterate quickly, and ship to users.** V2 features can be added based on real feedback.

**Good luck, and happy building! 🦀**
