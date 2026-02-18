# Rust + CEF Desktop Application

A high-performance desktop application framework using **Rust** for the backend and **CEF (Chromium Embedded Framework)** for the frontend. This project aims to provide an Electron-like experience but with the performance and safety of Rust, bypassing the need for Node.js in the main process.

> **Status**: V1 Milestone (Basic Desktop App) - Functional

## 🚀 Features

Based on the [V1 Roadmap](./roadmap.md), the following features are implemented:

- **Window Management**: Native window creation, resizing, and events via `winit`.
- **Modern Frontend**: Use React, Vue, Svelte, or any web framework.
- **IPC Bridge**: Simple `window.rust.invoke()` pattern to communicate between JS and Rust.
- **Native File Dialogs**: Open/Save files and pick folders using native OS dialogs (`rfd`).
- **File System Operations**: Read/Write files securely from Rust.
- **System Tray**: Custom tray icon with menus and event handling.
- **Application Menus**: Native macOS/Windows menus via `muda`.
- **Assets**: Embed HTML/CSS/JS features into the binary for single-file distribution.

## 🛠 Prerequisites

- **Rust**: Latest stable version.
- **Bun**: For building the frontend (faster than Node/NPM).
- **macOS/Linux/Windows**: Currently optimized and tested primarily on **macOS**.

## 📦 Installation & Quick Start (Running the Example)

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/your-username/rust-cef.git
    cd rust-cef
    ```

2.  **Install Frontend Dependencies:**
    ```bash
    cd frontend
    bun install
    cd ..
    ```

3.  **Build and Run (Dev Mode):**
    
    The first run requires setting up the CEF frameworks.
    
    ```bash
    # 1. Build Rust binary (this extracts CEF frameworks)
    cargo build
    
    # 2. Setup Frameworks structure (Required for macOS)
    chmod +x bundle_app.sh
    ./bundle_app.sh
    
    # 3. Run in Dev Mode (Starts Bun dev server + Rust app)
    cargo run -- --dev
    ```

    In dev mode (`--dev`), the app connects to `http://localhost:5173` (Vite) for hot-reloading.

4.  **Build for Release:**
    ```bash
    cargo build --release
    ./bundle_app.sh # Bundles into rust-cef.app in target/release
    ```
    
    The release app will use the embedded assets from `frontend/dist`.

## 📖 Usage Guide: Creating a New Project

This section explains how to use `rust-cef` as a library to build your own desktop applications.

### 1. Project Structure

A typical project structure should look like this:

```
my-app/
├── Cargo.toml          # Rust dependencies
├── build.rs            # Build script for frontend assets
├── bundle_app.sh       # Bundling script (copy from this repo)
├── src/
│   └── main.rs         # Application entry point
└── frontend/           # Your React/Vue/Svelte app
    ├── package.json
    ├── vite.config.ts
    └── src/
```

### 2. Dependencies

Add `rust-cef` and other required crates to your `Cargo.toml`:

```toml
[dependencies]
rust-cef = { git = "https://github.com/your-username/rust-cef" } # Or local path
rust-embed = "8.5"
serde_json = "1.0"
```

### 3. Application Entry Point (`src/main.rs`)

```rust
use rust_cef::App;
use rust_embed::RustEmbed;
use serde_json::json;

// 1. Embed your frontend assets
#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

fn main() {
    // 2. Configure the Application
    let app = App::new()
        .title("My Awesome App")
        .size(1024.0, 768.0)
        .resizable(true)
        // 3. Provide the asset resolver
        .assets(|path| Assets::get(path))
        // 4. Register IPC commands (Frontend calls window.rust.invoke('greet', { name: 'World' }))
        .register_ipc("greet", |args| {
            let name = args["name"].as_str().unwrap_or("Stranger");
            Ok(json!({ "message": format!("Hello, {}!", name) }))
        })
        // 5. Configure Dev Mode (Hot Reloading)
        // Ensure you have a valid command and URL for your frontend dev server
        .dev_config(rust_cef::DevConfig {
            command: "bun dev".to_string(),
            url: "http://localhost:5173".to_string(), // Vite default
            cwd: Some("frontend".to_string()),
        });

    // 6. Run the application
    if let Err(e) = app.run() {
        eprintln!("Error: {}", e);
    }
}
```

### 4. IPC Bridge (Frontend Implementation)

In your frontend code (e.g., `App.tsx`), add the type definition and invoke function:

```typescript
// Type definition
declare global {
  interface Window {
    rust: {
      invoke(command: string, args?: any): Promise<any>;
    };
  }
}

// Usage
const handleGreet = async () => {
  try {
    const response = await window.rust.invoke('greet', { name: 'Rust' });
    console.log(response.message); // Output: "Hello, Rust!"
  } catch (error) {
    console.error(error);
  }
};
```

### 5. Building and Bundling

CEF requires a specific bundle structure on macOS. You cannot simply run `cargo run` without setting up the Frameworks first. Ensure you copy the `bundle_app.sh` script from this repository to your project root and run it after `cargo build`.

```bash
cargo build
chmod +x bundle_app.sh
./bundle_app.sh
cargo run -- --dev
```

## 🏗 Roadmap

See [roadmap.md](./roadmap.md) for the complete development plan, including V2 features like advanced IPC and Electron parity.

## 📄 License

MIT
