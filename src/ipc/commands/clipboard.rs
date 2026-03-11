use arboard::Clipboard;
use serde_json::Value;

pub fn clipboard_read_text(_args: &Value) -> Result<Value, String> {
    let mut clipboard = Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?;
    let text = clipboard.get_text().map_err(map_clipboard_read_error)?;
    Ok(serde_json::json!({ "text": text }))
}

pub fn clipboard_write_text(args: &Value) -> Result<Value, String> {
    let text = args
        .get("text")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Missing required string field 'text'".to_string())?;

    let mut clipboard = Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|err| format!("Failed to write clipboard text: {err}"))?;

    Ok(serde_json::json!({ "status": "written" }))
}

pub fn clipboard_clear(_args: &Value) -> Result<Value, String> {
    let mut clipboard = Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?;
    clipboard
        .clear()
        .map_err(|err| format!("Failed to clear clipboard: {err}"))?;

    Ok(serde_json::json!({ "status": "cleared" }))
}

fn map_clipboard_read_error(error: arboard::Error) -> String {
    let error_text = error.to_string();
    if error_text.to_ascii_lowercase().contains("text") {
        format!("Clipboard does not currently contain text: {error_text}")
    } else {
        format!("Failed to read clipboard text: {error_text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_write_requires_text_field() {
        let error = clipboard_write_text(&serde_json::json!({}))
            .expect_err("expected missing text field to fail");
        assert!(error.contains("Missing required string field 'text'"));
    }
}
