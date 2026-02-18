// unused import
// use rust_cef::App;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist"] // Point this to your built UI assets
struct Assets;

fn main() {
    // Check if we are running in dev mode
    let dev_mode = std::env::args().any(|a| a == "--dev");

    let app = rust_cef::App::new()
        .title("Rust + CEF Shell (Lib)")
        .assets(|path| Assets::get(path));

    // Configure Dev Workflow
    let app = if dev_mode {
        app.dev_config(rust_cef::DevConfig {
            command: "bun dev".to_string(),
            url: "http://localhost:5173".to_string(),
            cwd: Some("frontend".to_string()),
        })
        .visible(false) // Start hidden, wait for on_ready
        .on_ready(|handle| {
             // In dev mode, show window when ready
             // handle.toggle_tools(); // Commented out to reduce startup flicker
             handle.show();
        })
    } else {
        // Prod mode
        app
    };

    if let Err(e) = app.run() {
        eprintln!("Application error: {}", e);
    }
}