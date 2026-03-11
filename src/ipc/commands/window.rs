use base64::{engine::general_purpose::STANDARD as b64, Engine as _};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

pub fn create_window(
    args: &Value,
    proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("app://localhost/index.html")
        .to_string();
    crate::security::enforce_url_policy(&url, cfg!(debug_assertions))?;
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("New Window")
        .to_string();
    let width = args.get("width").and_then(|v| v.as_f64()).unwrap_or(800.0);
    let height = args.get("height").and_then(|v| v.as_f64()).unwrap_or(600.0);
    let x = args.get("x").and_then(|v| v.as_f64());
    let y = args.get("y").and_then(|v| v.as_f64());
    let persist_key = args
        .get("persist_key")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let resizable = args
        .get("resizable")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let frameless = args.get("frameless").and_then(|v| v.as_bool());
    let transparent = args.get("transparent").and_then(|v| v.as_bool());
    let always_on_top = args.get("always_on_top").and_then(|v| v.as_bool());
    let kiosk = args.get("kiosk").and_then(|v| v.as_bool());
    let icon = args
        .get("icon")
        .and_then(|v| v.as_str())
        .and_then(|b64_str| b64.decode(b64_str).ok());

    let config = crate::WindowConfig {
        url,
        title,
        width,
        height,
        x,
        y,
        persist_key,
        resizable,
        start_hidden: false,
        frameless,
        transparent,
        always_on_top,
        kiosk,
        icon,
    };

    if let Some(p) = proxy {
        if let Ok(proxy_arc) = p.lock() {
            if proxy_arc
                .send_event(crate::AppEvent::CreateWindow(config.clone()))
                .is_err()
            {
                return Err("Winit EventLoop is no longer active".to_string());
            }
        } else {
            return Err("Failed to lock EventLoopProxy mutex".to_string());
        }
    } else {
        return Err("EventLoopProxy is not configured for window creation".to_string());
    }

    Ok(serde_json::json!({ "status": "requested", "url": config.url }))
}

pub fn set_window_config(
    args: &Value,
    proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let p = match proxy {
        Some(p) => p,
        None => return Err("EventLoopProxy is not configured".to_string()),
    };

    let proxy_arc = p
        .lock()
        .map_err(|_| "Failed to lock EventLoopProxy mutex")?;

    // By absent window_id, we default to Main Window inside lib.rs logic (None)
    // Actually we don't have a way to serialize WindowId easily unless we track it manually, doing None covers the 90% case
    let target = None;

    if let Some(frameless) = args.get("frameless").and_then(|v| v.as_bool()) {
        let _ = proxy_arc.send_event(crate::AppEvent::SetDecorations(target, !frameless));
    }

    if let Some(always_on_top) = args.get("always_on_top").and_then(|v| v.as_bool()) {
        let _ = proxy_arc.send_event(crate::AppEvent::SetAlwaysOnTop(target, always_on_top));
    }

    if let Some(kiosk) = args.get("kiosk").and_then(|v| v.as_bool()) {
        let _ = proxy_arc.send_event(crate::AppEvent::SetKiosk(target, kiosk));
    }

    if let Some(icon_b64) = args.get("icon").and_then(|v| v.as_str()) {
        if let Ok(icon_bytes) = b64.decode(icon_b64) {
            if let Ok(img) = image::load_from_memory(&icon_bytes) {
                let rgba = img.into_rgba8();
                let (width, height) = rgba.dimensions();
                if let Ok(icon) = winit::window::Icon::from_rgba(rgba.into_raw(), width, height) {
                    let _ =
                        proxy_arc.send_event(crate::AppEvent::SetWindowIcon(target, Some(icon)));
                }
            }
        }
    }

    Ok(serde_json::json!({ "status": "updated" }))
}
