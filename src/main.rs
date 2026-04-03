// unused import
// use rust_cef::App;
use rust_embed::RustEmbed;
use std::thread;
use std::time::Duration;

#[derive(RustEmbed)]
#[folder = "frontend/dist"] // Point this to your built UI assets
struct Assets;

fn main() {
    // Check if we are running in dev mode
    let dev_mode = std::env::args().any(|a| a == "--dev");
    let open_devtools = std::env::args().any(|a| a == "--devtools")
        || std::env::var("RUST_CEF_OPEN_DEVTOOLS")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);

    let deferred_startup = build_default_deferred_startup(dev_mode);
    let app = rust_cef::App::new()
        .title("Rust + CEF Shell (Lib)")
        .assets(|path| Assets::get(path))
        .deferred_startup(deferred_startup);

    // Configure Dev Workflow
    let app = if dev_mode {
        app.dev_config(rust_cef::DevConfig {
            command: "bun dev".to_string(),
            url: "http://localhost:5173".to_string(),
            cwd: Some("frontend".to_string()),
            open_devtools,
        })
        .visible(false) // Start hidden, wait for on_ready
        .on_ready(|handle| {
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

fn build_default_deferred_startup(dev_mode: bool) -> rust_cef::DeferredStartupConfig {
    let mut milestones = vec![
        rust_cef::MilestoneDefinition {
            key: "bootstrap".to_string(),
            label: "Preparing application".to_string(),
            weight: 40,
            required: true,
        },
        rust_cef::MilestoneDefinition {
            key: "assets".to_string(),
            label: "Loading bundled assets".to_string(),
            weight: 35,
            required: true,
        },
    ];

    if dev_mode {
        milestones.push(rust_cef::MilestoneDefinition {
            key: "dev".to_string(),
            label: "Preparing development session".to_string(),
            weight: 25,
            required: true,
        });
    }

    let coordinator = rust_cef::StartupCoordinator::new(milestones)
        .expect("default startup coordinator should be valid");
    let worker = coordinator.clone();

    thread::spawn(move || {
        advance_milestone(&worker, "bootstrap", 3, 80);
        warm_assets(&worker);

        if dev_mode {
            advance_milestone(&worker, "dev", 4, 110);
        }

        worker
            .mark_ready_for_cef()
            .expect("default startup milestones should complete before CEF");
    });

    rust_cef::DeferredStartupConfig {
        coordinator,
        ui: rust_cef::StartupUiConfig {
            title: "Rust + CEF".to_string(),
            subtitle: Some("Starting application".to_string()),
            ..Default::default()
        },
        transition_delay_ms: 350,
    }
}

fn advance_milestone(
    coordinator: &rust_cef::StartupCoordinator,
    key: &str,
    steps: u32,
    sleep_ms: u64,
) {
    let id = coordinator
        .milestone_id(key)
        .unwrap_or_else(|| panic!("missing milestone key: {key}"));
    coordinator.start(id).expect("failed to start milestone");

    for step in 1..=steps {
        thread::sleep(Duration::from_millis(sleep_ms));
        coordinator
            .set_progress(id, step as f32 / steps as f32)
            .expect("failed to update milestone progress");
    }

    coordinator
        .complete(id)
        .expect("failed to complete milestone");
}

fn warm_assets(coordinator: &rust_cef::StartupCoordinator) {
    let id = coordinator
        .milestone_id("assets")
        .expect("assets milestone should exist");
    coordinator.start(id).expect("failed to start assets milestone");

    let asset_keys: Vec<_> = Assets::iter().collect();
    if asset_keys.is_empty() {
        coordinator
            .complete(id)
            .expect("failed to complete empty assets milestone");
        return;
    }

    let total = asset_keys.len();
    for (index, key) in asset_keys.into_iter().enumerate() {
        let _ = Assets::get(&key);
        coordinator
            .set_progress(id, (index + 1) as f32 / total as f32)
            .expect("failed to update assets milestone");
        thread::sleep(Duration::from_millis(35));
    }

    coordinator
        .complete(id)
        .expect("failed to complete assets milestone");
}
