use muda::{
    AboutMetadata, CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu,
};
use muda::accelerator::{Accelerator, Code, Modifiers};
use crate::debug_logger::print_debug;

// Menu IDs
pub const MENU_VIEW_RELOAD: &str = "view_reload";
pub const MENU_VIEW_DEVTOOLS: &str = "view_devtools";
pub const MENU_VIEW_COUNTER: &str = "view_counter";

pub const MENU_WINDOW_ALWAYS_ON_TOP: &str = "window_always_on_top";
pub const MENU_DIALOG_INFO: &str = "dialog_info";
pub const MENU_DIALOG_WARNING: &str = "dialog_warning";
pub const MENU_DIALOG_ERROR: &str = "dialog_error";
pub const MENU_DIALOG_CONFIRM: &str = "dialog_confirm";

pub struct AppMenuHandles {
    pub menu: Menu,
    pub submenus: Vec<Submenu>,
    pub view_counter_item: MenuItem,
    pub window_always_on_top_item: CheckMenuItem,
}

pub fn create_app_menu_bar() -> AppMenuHandles {
    print_debug("DEBUG: create_app_menu_bar starting");
    let menu = Menu::new();
    let mut submenus = Vec::new();

    // ----------------------------------------------------------------
    // 1. App Menu (macOS only)
    // ----------------------------------------------------------------
    #[cfg(target_os = "macos")]
    {
        print_debug("DEBUG: Creating App Menu");
        let app_menu = Submenu::new("App", true);
        app_menu.append(&PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("Rust + CEF Shell".to_string()),
                version: Some("0.1.0".to_string()),
                ..Default::default()
            }),
        )).expect("Failed to append About");
        app_menu.append(&PredefinedMenuItem::separator()).unwrap();
        app_menu.append(&PredefinedMenuItem::services(None)).unwrap();
        app_menu.append(&PredefinedMenuItem::separator()).unwrap();
        app_menu.append(&PredefinedMenuItem::hide(None)).unwrap();
        app_menu.append(&PredefinedMenuItem::hide_others(None)).unwrap();
        app_menu.append(&PredefinedMenuItem::show_all(None)).unwrap();
        app_menu.append(&PredefinedMenuItem::separator()).unwrap();
        app_menu.append(&PredefinedMenuItem::quit(None)).unwrap();
        
        menu.append(&app_menu).expect("Failed to append App Menu to Root");
        submenus.push(app_menu);
    }

    // ----------------------------------------------------------------
    // 2. File Menu
    // ----------------------------------------------------------------
    print_debug("DEBUG: Creating File Menu");
    let file_menu = Submenu::new("File", true);
    #[cfg(target_os = "macos")]
    file_menu.append(&PredefinedMenuItem::close_window(None)).unwrap();

    #[cfg(not(target_os = "macos"))]
    file_menu.append(&PredefinedMenuItem::quit(None)).unwrap();

    menu.append(&file_menu).expect("Failed to append File Menu");
    submenus.push(file_menu);

    // ----------------------------------------------------------------
    // 3. Edit Menu
    // ----------------------------------------------------------------
    print_debug("DEBUG: Creating Edit Menu");
    let edit_menu = Submenu::new("Edit", true);
    edit_menu.append(&PredefinedMenuItem::undo(None)).unwrap();
    edit_menu.append(&PredefinedMenuItem::redo(None)).unwrap();
    edit_menu.append(&PredefinedMenuItem::separator()).unwrap();
    edit_menu.append(&PredefinedMenuItem::cut(None)).unwrap();
    edit_menu.append(&PredefinedMenuItem::copy(None)).unwrap();
    edit_menu.append(&PredefinedMenuItem::paste(None)).unwrap();
    edit_menu.append(&PredefinedMenuItem::select_all(None)).unwrap();
    menu.append(&edit_menu).expect("Failed to append Edit Menu");
    submenus.push(edit_menu);

    // ----------------------------------------------------------------
    // 4. View Menu
    // ----------------------------------------------------------------
    print_debug("DEBUG: Creating View Menu");
    let view_menu = Submenu::new("View", true);
    
    // Reload (Cmd+R)
    view_menu.append(&MenuItem::with_id(
        MENU_VIEW_RELOAD,
        "Reload",
        true,
        Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyR)),
    )).unwrap();

    // Toggle DevTools (Cmd+Option+I)
    view_menu.append(&MenuItem::with_id(
        MENU_VIEW_DEVTOOLS,
        "Toggle Developer Tools",
        true,
        Some(Accelerator::new(Some(Modifiers::SUPER | Modifiers::ALT), Code::KeyI)),
    )).unwrap();

    // Dynamic Counter
    let view_counter_item = MenuItem::with_id(
        MENU_VIEW_COUNTER,
        "Counter: 0",
        true,
        None,
    );
    view_menu.append(&view_counter_item).expect("Failed to append Counter");

    menu.append(&view_menu).expect("Failed to append View Menu");
    submenus.push(view_menu);

    // ----------------------------------------------------------------
    // 5. Window Menu
    // ----------------------------------------------------------------
    print_debug("DEBUG: Creating Window Menu");
    let window_menu = Submenu::new("Window", true);
    window_menu.append(&PredefinedMenuItem::minimize(None)).unwrap();
    window_menu.append(&PredefinedMenuItem::separator()).unwrap();
    window_menu.append(&PredefinedMenuItem::bring_all_to_front(None)).unwrap();
    window_menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Always on Top (Checkable)
    let window_always_on_top_item = CheckMenuItem::with_id(
        MENU_WINDOW_ALWAYS_ON_TOP,
        "Always on Top",
        true,
        false, // checked
        None, // accelerator
    );
    window_menu.append(&window_always_on_top_item).expect("Failed to append Always on Top");

    menu.append(&window_menu).expect("Failed to append Window Menu");
    submenus.push(window_menu);

    // ----------------------------------------------------------------
    // 6. Help Menu
    // ----------------------------------------------------------------
    print_debug("DEBUG: Creating Help Menu");
    let help_menu = Submenu::new("Help", true);
    // User explicitly requested Help:About, so adding it here even for macOS
    help_menu.append(&PredefinedMenuItem::about(None, None)).unwrap();
    
    menu.append(&help_menu).expect("Failed to append Help Menu");
    submenus.push(help_menu);

    // ----------------------------------------------------------------
    // 7. Dialogs Menu (For testing)
    // ----------------------------------------------------------------
    print_debug("DEBUG: Creating Dialogs Menu");
    let dialog_menu = Submenu::new("Dialogs", true);
    dialog_menu.append(&MenuItem::with_id(MENU_DIALOG_INFO, "Show Info", true, None)).unwrap();
    dialog_menu.append(&MenuItem::with_id(MENU_DIALOG_WARNING, "Show Warning", true, None)).unwrap();
    dialog_menu.append(&MenuItem::with_id(MENU_DIALOG_ERROR, "Show Error", true, None)).unwrap();
    dialog_menu.append(&PredefinedMenuItem::separator()).unwrap();
    dialog_menu.append(&MenuItem::with_id(MENU_DIALOG_CONFIRM, "Show Confirmation", true, None)).unwrap();

    menu.append(&dialog_menu).expect("Failed to append Dialogs Menu");
    submenus.push(dialog_menu);

    print_debug("DEBUG: create_app_menu_bar completed");
    AppMenuHandles { 
        menu, 
        submenus,
        view_counter_item,
        window_always_on_top_item,
    }
}
