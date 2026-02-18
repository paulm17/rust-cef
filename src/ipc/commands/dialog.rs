use serde::Deserialize;
use serde_json::Value;
use rfd::FileDialog;

#[derive(Deserialize)]
struct OpenDialogOptions {
    title: Option<String>,
    directory: Option<String>,
    filters: Option<Vec<String>>, // Simple list of extensions
    multiple: Option<bool>,
}

#[derive(Deserialize)]
struct SaveDialogOptions {
    title: Option<String>,
    directory: Option<String>,
    filename: Option<String>,
    filters: Option<Vec<String>>,
}

pub fn show_open_dialog(args: &Value) -> Result<Value, String> {
    let options: OpenDialogOptions = serde_json::from_value(args.clone())
        .map_err(|e| format!("Invalid options: {}", e))?;

    let mut dialog = FileDialog::new();
    
    if let Some(title) = options.title {
        dialog = dialog.set_title(&title);
    }
    if let Some(directory) = options.directory {
        dialog = dialog.set_directory(&directory);
    }
    if let Some(filters) = options.filters {
        dialog = dialog.add_filter("Files", &filters);
    }

    if options.multiple.unwrap_or(false) {
        let files = dialog.pick_files();
        Ok(serde_json::json!(files))
    } else {
        let file = dialog.pick_file();
        Ok(serde_json::json!(file))
    }
}

pub fn show_save_dialog(args: &Value) -> Result<Value, String> {
    let options: SaveDialogOptions = serde_json::from_value(args.clone())
        .map_err(|e| format!("Invalid options: {}", e))?;

    let mut dialog = FileDialog::new();

    if let Some(title) = options.title {
        dialog = dialog.set_title(&title);
    }
    if let Some(directory) = options.directory {
        dialog = dialog.set_directory(&directory);
    }
    if let Some(filename) = options.filename {
        dialog = dialog.set_file_name(&filename);
    }
    if let Some(filters) = options.filters {
        dialog = dialog.add_filter("Files", &filters);
    }

    let file = dialog.save_file();
    Ok(serde_json::json!(file))
}

pub fn show_pick_folder_dialog(args: &Value) -> Result<Value, String> {
    let options: OpenDialogOptions = serde_json::from_value(args.clone())
        .map_err(|e| format!("Invalid options: {}", e))?;

    let mut dialog = FileDialog::new();
    
    if let Some(title) = options.title {
        dialog = dialog.set_title(&title);
    }
    if let Some(directory) = options.directory {
        dialog = dialog.set_directory(&directory);
    }

    if options.multiple.unwrap_or(false) {
        let files = dialog.pick_folders();
        Ok(serde_json::json!(files))
    } else {
        let file = dialog.pick_folder();
        Ok(serde_json::json!(file))
    }
}
