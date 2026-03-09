use serde_json::Value;
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

pub fn create_window(args: &Value, proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>) -> Result<Value, String> {
    // 1. Parse arguments (fallback to sensible defaults)
    let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("app://localhost/index.html").to_string();
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("New Window").to_string();
    let width = args.get("width").and_then(|v| v.as_f64()).unwrap_or(800.0);
    let height = args.get("height").and_then(|v| v.as_f64()).unwrap_or(600.0);
    let resizable = args.get("resizable").and_then(|v| v.as_bool()).unwrap_or(true);

    let config = crate::WindowConfig {
         url,
         title,
         width,
         height,
         resizable,
         start_hidden: false,
    };

    if let Some(p) = proxy {
         if let Ok(proxy_arc) = p.lock() {
             if proxy_arc.send_event(crate::AppEvent::CreateWindow(config.clone())).is_err() {
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
