use serde_json::Value;
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

#[cfg(target_os = "macos")]
use std::ffi::CString;

use global_hotkey::hotkey::HotKey;

#[cfg(target_os = "macos")]
use mac_notification_sys::{MainButton, Notification, NotificationResponse, Sound};

#[allow(unexpected_cfgs)]
pub fn set_badge_count(
    args: &Value,
    proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    #[cfg(target_os = "macos")]
    {
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

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

pub fn get_launch_context(
    _args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let context = crate::state::get_launch_context()?;
    Ok(serde_json::json!({
        "deep_link": context.deep_link,
        "files": context.files,
    }))
}

pub fn show_notification(
    args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let title = args
        .get("title")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Missing required notification title".to_string())?;
    let body = args
        .get("body")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let subtitle = args.get("subtitle").and_then(|value| value.as_str());
    let sound = args.get("sound").and_then(|value| value.as_str());
    let app_icon = args.get("app_icon").and_then(|value| value.as_str());
    let content_image = args.get("content_image").and_then(|value| value.as_str());
    let action = args.get("action").and_then(|value| value.as_str());
    let close_button = args.get("close_button").and_then(|value| value.as_str());
    let wait_for_click = args
        .get("wait_for_click")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if wait_for_click {
        return Err(
            "wait_for_click is not supported from the synchronous IPC bridge because it blocks the app event loop"
                .to_string(),
        );
    }

    #[cfg(target_os = "macos")]
    {
        let bundle_identifier =
            std::env::var("RUST_CEF_BUNDLE_ID").unwrap_or_else(|_| "com.rustcef.app".to_string());
        let _ = mac_notification_sys::set_application(&bundle_identifier);

        let mut notification = Notification::new();
        notification.title(title).message(body);

        if let Some(subtitle) = subtitle.filter(|value| !value.trim().is_empty()) {
            notification.subtitle(subtitle);
        }

        if let Some(app_icon) = app_icon.filter(|value| !value.trim().is_empty()) {
            notification.app_icon(app_icon);
        }

        if let Some(content_image) = content_image.filter(|value| !value.trim().is_empty()) {
            notification.content_image(content_image);
        }

        if let Some(action) = action.filter(|value| !value.trim().is_empty()) {
            notification.main_button(MainButton::SingleAction(action));
        }

        if let Some(close_button) = close_button.filter(|value| !value.trim().is_empty()) {
            notification.close_button(close_button);
        }

        match sound.filter(|value| !value.trim().is_empty()) {
            Some("default") => {
                notification.default_sound();
            }
            Some(custom) => {
                notification.sound(Sound::Custom(custom.to_string()));
            }
            None => {}
        }

        notification.wait_for_click(false);
        notification.asynchronous(true);

        let response = notification.send().map_err(|err| err.to_string())?;

        return Ok(serde_json::json!({
            "status": "shown",
            "title": title,
            "body": body,
            "subtitle": subtitle,
            "sound": sound,
            "response": notification_response(response),
        }));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            title,
            body,
            subtitle,
            sound,
            app_icon,
            content_image,
            action,
            close_button,
            wait_for_click,
        );
        Err("Notifications are currently implemented on macOS only".to_string())
    }
}

pub fn register_global_shortcut(
    args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let accelerator = args
        .get("accelerator")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Missing required accelerator".to_string())?
        .trim()
        .to_string();
    let id = args
        .get("id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| accelerator.clone());

    let hotkey: HotKey = accelerator
        .parse()
        .map_err(|err| format!("Invalid accelerator '{accelerator}': {err}"))?;

    crate::state::register_global_shortcut(id.clone(), accelerator.clone(), hotkey)?;

    Ok(serde_json::json!({
        "status": "registered",
        "id": id,
        "accelerator": accelerator,
    }))
}

pub fn unregister_global_shortcut(
    args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let id = args
        .get("id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Missing required shortcut id".to_string())?;

    let removed = crate::state::unregister_global_shortcut(id)?;

    Ok(serde_json::json!({
        "status": if removed.is_some() { "unregistered" } else { "not_found" },
        "id": id,
    }))
}

pub fn list_global_shortcuts(
    _args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let shortcuts = crate::state::list_global_shortcuts()?;
    Ok(serde_json::json!(shortcuts
        .into_iter()
        .map(|shortcut| serde_json::json!({
            "id": shortcut.id,
            "accelerator": shortcut.accelerator,
        }))
        .collect::<Vec<_>>()))
}

pub fn poll_global_shortcut_events(
    _args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let events = crate::state::take_global_shortcut_events()?;
    Ok(serde_json::json!(events
        .into_iter()
        .map(|event| serde_json::json!({
            "id": event.id,
            "accelerator": event.accelerator,
            "state": event.state,
        }))
        .collect::<Vec<_>>()))
}

pub fn poll_app_events(
    _args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    let events = crate::state::take_app_events()?;
    Ok(serde_json::json!(events
        .into_iter()
        .map(|event| serde_json::json!({
            "event": event.event,
            "payload": event.payload,
        }))
        .collect::<Vec<_>>()))
}

#[cfg(target_os = "macos")]
fn notification_response(response: NotificationResponse) -> Value {
    match response {
        NotificationResponse::None => serde_json::json!({ "kind": "none" }),
        NotificationResponse::ActionButton(value) => {
            serde_json::json!({ "kind": "action", "value": value })
        }
        NotificationResponse::CloseButton(value) => {
            serde_json::json!({ "kind": "close", "value": value })
        }
        NotificationResponse::Click => serde_json::json!({ "kind": "click" }),
        NotificationResponse::Reply(value) => {
            serde_json::json!({ "kind": "reply", "value": value })
        }
    }
}
