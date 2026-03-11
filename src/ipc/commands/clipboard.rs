use arboard::Clipboard;
use base64::{engine::general_purpose, Engine as _};
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};
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

pub fn clipboard_read_image(_args: &Value) -> Result<Value, String> {
    let mut clipboard = Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?;
    let image = clipboard
        .get_image()
        .map_err(|err| format!("Clipboard does not currently contain an image: {err}"))?;

    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(
            image.bytes.as_ref(),
            image.width as u32,
            image.height as u32,
            ColorType::Rgba8.into(),
        )
        .map_err(|err| format!("Failed to encode clipboard image: {err}"))?;

    Ok(serde_json::json!({
        "png_base64": general_purpose::STANDARD.encode(png),
        "width": image.width,
        "height": image.height,
    }))
}

pub fn clipboard_write_image(args: &Value) -> Result<Value, String> {
    let png_base64 = args
        .get("png_base64")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Missing required string field 'png_base64'".to_string())?;
    let bytes = general_purpose::STANDARD
        .decode(png_base64)
        .map_err(|err| format!("Invalid base64 image payload: {err}"))?;
    let rgba = image::load_from_memory(&bytes)
        .map_err(|err| format!("Invalid image payload: {err}"))?
        .to_rgba8();
    let (width, height) = rgba.dimensions();

    let mut clipboard = Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        })
        .map_err(|err| format!("Failed to write clipboard image: {err}"))?;

    Ok(serde_json::json!({
        "status": "written",
        "width": width,
        "height": height,
    }))
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
