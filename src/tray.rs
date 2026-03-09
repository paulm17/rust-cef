use muda::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, Icon};

pub const MENU_ITEM_SHOW_HIDE_ID: &str = "show_hide";
pub const MENU_ITEM_QUIT_ID: &str = "quit";

pub fn create_app_menu() -> Menu {
    let menu = Menu::new();
    
    let show_hide_item = MenuItem::with_id(MENU_ITEM_SHOW_HIDE_ID, "Show/Hide Window", true, None);
    let quit_item = MenuItem::with_id(MENU_ITEM_QUIT_ID, "Quit", true, None);
    
    // Add items to menu
    // Note: append_items expects &[&dyn MenuItemExt] but in muda 0.15+ it might be slightly different.
    // Let's check docs or usage pattern. simplest is append.
    let _ = menu.append(&show_hide_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit_item);
    
    menu
}

pub fn create_tray_icon(menu: &Menu) -> TrayIcon {
    let icon = generate_icon();
    
    TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip("Rust + CEF Shell")
        .with_icon(icon)
        .build()
        .unwrap()
}

use std::sync::OnceLock;

static TRAY_ICON_PATH: OnceLock<String> = OnceLock::new();

pub fn set_tray_icon_path(path: String) {
    let _ = TRAY_ICON_PATH.set(path);
}

pub fn generate_tray_icon_with_badge(badge: Option<u32>) -> Icon {
    let mut rgba;
    let width;
    let height;

    if let Some(path) = TRAY_ICON_PATH.get() {
        match image::open(path) {
            Ok(img) => {
                let img_rgba = img.into_rgba8();
                let dims = img_rgba.dimensions();
                width = dims.0;
                height = dims.1;
                rgba = img_rgba.into_raw();
            },
            Err(_) => {
                return generate_fallback_icon(badge);
            }
        }
    } else {
        return generate_fallback_icon(badge);
    }
    
    // Draw badge logic over `rgba`
    if let Some(count) = badge {
        if count > 0 {
            let cx = width as i32 - 10;
            let cy = height as i32 - 10;
            for y in 0..height {
                for x in 0..width {
                    let dx = x as i32 - cx;
                    let dy = y as i32 - cy;
                    if dx*dx + dy*dy <= 64 { // radius 8
                        let idx = ((y * width + x) * 4) as usize;
                        if idx + 3 < rgba.len() {
                            rgba[idx] = 255;
                            rgba[idx+1] = 0;
                            rgba[idx+2] = 0;
                            rgba[idx+3] = 255;
                        }
                    }
                }
            }
        }
    }
    Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
}

fn generate_fallback_icon(badge: Option<u32>) -> Icon {
    let width = 32u32;
    let height = 32u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    
    // Generate a simple green square icon
    for y in 0..height {
        for x in 0..width {
            let mut is_badge = false;
            if let Some(count) = badge {
                if count > 0 {
                    let cx = width as i32 - 10;
                    let cy = height as i32 - 10;
                    let dx = x as i32 - cx;
                    let dy = y as i32 - cy;
                    if dx*dx + dy*dy <= 64 { // radius 8
                        is_badge = true;
                    }
                }
            }
            
            if is_badge {
                rgba.extend_from_slice(&[255, 0, 0, 255]); // Red badge
            } else {
                rgba.extend_from_slice(&[0, 255, 0, 255]); // Green base
            }
        }
    }
    
    Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
}

fn generate_icon() -> Icon {
    generate_tray_icon_with_badge(None)
}
