use std::process::Command;
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/vite.config.ts");

    let profile = env::var("PROFILE").unwrap();
    let current_dir = env::current_dir().unwrap();
    let frontend_dir = current_dir.join("frontend");
    let dist_dir = frontend_dir.join("dist");

    if profile == "release" {
        println!("cargo:warning=Building frontend assets for release...");
        
        // Ensure bun is available
        let status = Command::new("bun")
            .arg("--version")
            .current_dir(&frontend_dir)
            .status();

        if status.is_err() {
            println!("cargo:warning=Bun not found. Skipping frontend build. Ensure 'bun' is in PATH.");
            return;
        }

        // Install dependencies
        let status = Command::new("bun")
            .arg("install")
            .current_dir(&frontend_dir)
            .status()
            .expect("Failed to run bun install");
        
        if !status.success() {
            panic!("bun install failed");
        }

        // Build
        let status = Command::new("bun")
            .arg("run")
            .arg("build")
            .current_dir(&frontend_dir)
            .status()
            .expect("Failed to run bun run build");

        if !status.success() {
             panic!("bun run build failed");
        }
    } else {
        // In debug mode, ensure dist folder exists so rust-embed doesn't fail
        if !dist_dir.exists() {
             std::fs::create_dir_all(&dist_dir).expect("failed to create frontend/dist");
             // Create a dummy file if needed? rust-embed might need at least one file?
             // Actually, empty folder is usually fine, but let's be safe.
             std::fs::write(dist_dir.join("index.html"), "<html><body>Dev Mode Placeholder</body></html>").ok();
        }
    }
}
