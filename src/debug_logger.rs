use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::EnvFilter;

static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

pub fn init_logging(dev_mode: bool, debug_mode: bool) {
    let default_level = if debug_mode {
        "debug"
    } else if dev_mode {
        "info"
    } else {
        "info"
    };

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_ids(debug_mode)
        .compact()
        .try_init();
}

pub fn print_debug(msg: &str) {
    if is_debug_mode() {
        tracing::debug!("{}", msg);
    }
}

pub fn print_info(msg: &str) {
    tracing::info!("{}", msg);
}

pub fn log_debug(msg: &str) {
    tracing::debug!("{}", msg);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_cef_debug.log")
    {
        let _ = writeln!(file, "{}", msg);
    }
}
