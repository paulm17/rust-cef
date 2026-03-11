use std::backtrace::Backtrace;
use std::fs;
use std::panic;
use std::path::PathBuf;
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};

static PANIC_HOOK: Once = Once::new();

pub fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            if let Err(error) = write_panic_report(panic_info) {
                eprintln!("failed to write panic report: {error}");
            }
            default_hook(panic_info);
        }));
    });
}

fn write_panic_report(info: &panic::PanicHookInfo<'_>) -> Result<PathBuf, String> {
    let reports_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("rust-cef")
        .join("crash-reports");
    fs::create_dir_all(&reports_dir).map_err(|err| format!("create crash report dir: {err}"))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("read system time: {err}"))?
        .as_secs();
    let report_path = reports_dir.join(format!("panic-{timestamp}.log"));

    let location = info
        .location()
        .map(|loc| format!("{}:{}", loc.file(), loc.line()))
        .unwrap_or_else(|| "unknown".to_string());

    let payload = if let Some(message) = info.payload().downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = info.payload().downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    };

    let report = format!(
        "panic at: {location}\nmessage: {payload}\n\nbacktrace:\n{}\n",
        Backtrace::force_capture()
    );

    fs::write(&report_path, report).map_err(|err| format!("write panic report: {err}"))?;
    tracing::error!(path = %report_path.display(), "panic report written");
    Ok(report_path)
}
