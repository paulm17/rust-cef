use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

pub fn print_debug(msg: &str) {
    if is_debug_mode() {
        eprintln!("{}", msg);
    }
}

pub fn print_info(msg: &str) {
    eprintln!("{}", msg);
}

pub fn log_debug(msg: &str) {
    // Also print to stderr if debug mode is on
    print_debug(msg);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_cef_debug.log") 
    {
        let _ = writeln!(file, "{}", msg);
    }
}
