use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use winit::event_loop::EventLoopProxy;

pub fn start_download(
    args: &Value,
    proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let url = args
        .get("url")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Missing required string field 'url'".to_string())?
        .to_string();
    crate::security::enforce_url_policy(&url, crate::security::runtime_dev_mode())?;

    let response_rx = {
        let (response_tx, response_rx) = std::sync::mpsc::channel();
        let request = crate::StartDownloadRequest {
            url: url.clone(),
            path: args
                .get("path")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            show_dialog: crate::security::request_bool(args, "show_dialog", true),
            response_tx,
        };

        let proxy = proxy
            .as_ref()
            .ok_or_else(|| "EventLoopProxy is not configured for downloads".to_string())?;
        let proxy = proxy
            .lock()
            .map_err(|_| "Failed to lock EventLoopProxy mutex".to_string())?;
        proxy
            .send_event(crate::AppEvent::StartDownload(request))
            .map_err(|_| "Winit EventLoop is no longer active".to_string())?;

        response_rx
    };

    response_rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| format!("Timed out waiting to start download: {url}"))?
}

pub fn print_to_pdf(
    args: &Value,
    proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Missing required string field 'path'".to_string())?
        .to_string();

    let response_tx = {
        let (response_tx, response_rx) = std::sync::mpsc::channel();
        let request = crate::PrintToPdfRequest {
            path: path.clone(),
            landscape: crate::security::request_bool(args, "landscape", false),
            print_background: crate::security::request_bool(args, "print_background", true),
            display_header_footer: crate::security::request_bool(
                args,
                "display_header_footer",
                false,
            ),
            scale: args
                .get("scale")
                .and_then(|value| value.as_f64())
                .unwrap_or(100.0),
            response_tx,
        };

        let proxy = proxy
            .as_ref()
            .ok_or_else(|| "EventLoopProxy is not configured for PDF printing".to_string())?;
        let proxy = proxy
            .lock()
            .map_err(|_| "Failed to lock EventLoopProxy mutex".to_string())?;
        proxy
            .send_event(crate::AppEvent::PrintToPdf(request))
            .map_err(|_| "Winit EventLoop is no longer active".to_string())?;

        response_rx
    };

    response_tx
        .recv_timeout(Duration::from_secs(30))
        .map_err(|_| format!("Timed out waiting for PDF generation: {path}"))?
}
