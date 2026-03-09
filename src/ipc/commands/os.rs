use serde_json::Value;
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

#[cfg(target_os = "macos")]
use std::ffi::CString;

#[allow(unexpected_cfgs)]
pub fn set_badge_count(args: &Value, proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>) -> Result<Value, String> {
    let count = args.get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    #[cfg(target_os = "macos")]
    {
        use objc::{class, msg_send, sel, sel_impl};
        use objc::runtime::Object;
        
        unsafe {
            let ns_app_class = class!(NSApplication);
            let app: *mut Object = msg_send![ns_app_class, sharedApplication];
            let dock_tile: *mut Object = msg_send![app, dockTile];
            
            if count > 0 {
                let string = format!("{}", count);
                let label = CString::new(string).unwrap_or_default();
                let ns_string_class = class!(NSString);
                let ns_label: *mut Object = msg_send![ns_string_class, alloc];
                let ns_label: *mut Object = msg_send![ns_label, initWithUTF8String: label.as_ptr()];
                let _: () = msg_send![dock_tile, setBadgeLabel: ns_label];
            } else {
                let null: *mut Object = std::ptr::null_mut();
                let _: () = msg_send![dock_tile, setBadgeLabel: null];
            }
        }
    }

    if let Some(p) = proxy {
         if let Ok(proxy_arc) = p.lock() {
             let _ = proxy_arc.send_event(crate::AppEvent::SetTrayBadge(count));
         }
    }
    
    Ok(serde_json::json!({ "status": "updated", "count": count }))
}
