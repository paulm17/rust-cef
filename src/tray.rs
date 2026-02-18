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

fn generate_icon() -> Icon {
    let width = 32u32;
    let height = 32u32;
    let mut rgba = Vec::new();
    
    // Generate a simple green square icon
    for _ in 0..height {
        for _ in 0..width {
            rgba.push(0);   // R
            rgba.push(255); // G
            rgba.push(0);   // B
            rgba.push(255); // A
        }
    }
    
    Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
}
