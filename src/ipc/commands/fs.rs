use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::SystemTime;

#[derive(Serialize)]
pub struct FileMetadata {
    is_file: bool,
    is_dir: bool,
    size: u64,
    modified: Option<u64>,
}

#[derive(Serialize)]
pub struct DirEntry {
    name: String,
    metadata: FileMetadata,
}

pub fn read_file(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    match fs::read_to_string(path) {
        Ok(content) => Ok(serde_json::json!(content)),
        Err(e) => Err(e.to_string()),
    }
}

pub fn read_file_binary(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let encoded = general_purpose::STANDARD.encode(&buffer);
    Ok(serde_json::json!(encoded))
}

pub fn write_file(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing content")?;
    match fs::write(path, content) {
        Ok(_) => Ok(serde_json::json!(true)),
        Err(e) => Err(e.to_string()),
    }
}

pub fn write_file_binary(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing content")?;
    let decoded = general_purpose::STANDARD
        .decode(content)
        .map_err(|e| e.to_string())?;
    let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
    file.write_all(&decoded).map_err(|e| e.to_string())?;
    Ok(serde_json::json!(true))
}

pub fn exists(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    Ok(serde_json::json!(Path::new(path).exists()))
}

pub fn read_dir(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);

        result.push(DirEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            metadata: FileMetadata {
                is_file: metadata.is_file(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified,
            },
        });
    }

    Ok(serde_json::json!(result))
}

pub fn get_metadata(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);

    Ok(serde_json::json!(FileMetadata {
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified,
    }))
}

pub fn create_file_stream_url(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;
    let canonical_path = fs::canonicalize(path).map_err(|e| e.to_string())?;
    let metadata = fs::metadata(&canonical_path).map_err(|e| e.to_string())?;

    if !metadata.is_file() {
        return Err("Stream path must point to a file".to_string());
    }

    let path_string = canonical_path.to_string_lossy().into_owned();
    let mime_type = mime_guess::from_path(&path_string)
        .first_or_octet_stream()
        .to_string();
    let token = crate::state::register_file_stream(crate::state::FileStreamEntry {
        path: path_string.clone(),
        mime_type: mime_type.clone(),
    })?;

    Ok(serde_json::json!({
        "url": format!("app://localhost/__stream__/{}", token),
        "path": path_string,
        "mime_type": mime_type,
        "size": metadata.len(),
    }))
}
