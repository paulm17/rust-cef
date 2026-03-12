use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct UpdaterConfig {
    pub manifest_url: Option<String>,
    pub channel: String,
    pub current_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub url: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub signature: Option<String>,
}

pub fn current_config() -> UpdaterConfig {
    UpdaterConfig {
        manifest_url: std::env::var("RUST_CEF_UPDATE_MANIFEST_URL").ok(),
        channel: std::env::var("RUST_CEF_UPDATE_CHANNEL").unwrap_or_else(|_| "stable".to_string()),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn get_config(_args: &Value) -> Result<Value, String> {
    Ok(serde_json::to_value(current_config()).map_err(|err| err.to_string())?)
}

pub fn check_for_updates(args: &Value) -> Result<Value, String> {
    let config = current_config();
    let manifest_url = args
        .get("manifest_url")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or(config.manifest_url.clone())
        .ok_or_else(|| {
            "No updater manifest configured. Set RUST_CEF_UPDATE_MANIFEST_URL or pass manifest_url"
                .to_string()
        })?;

    let manifest = load_manifest(&manifest_url)?;
    let comparison = compare_versions(&manifest.version, &config.current_version)?;
    let update_available = comparison.is_gt();

    Ok(serde_json::json!({
        "status": if update_available { "update_available" } else { "up_to_date" },
        "current_version": config.current_version,
        "manifest_url": manifest_url,
        "channel": config.channel,
        "update_available": update_available,
        "manifest": manifest,
    }))
}

pub fn download_update(args: &Value) -> Result<Value, String> {
    let manifest_url = args
        .get("manifest_url")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let explicit_url = args
        .get("url")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    let manifest = if explicit_url.is_none() {
        let resolved_manifest_url = manifest_url
            .or_else(|| current_config().manifest_url)
            .ok_or_else(|| {
                "No updater manifest configured. Set RUST_CEF_UPDATE_MANIFEST_URL or pass manifest_url"
                    .to_string()
            })?;
        Some(load_manifest(&resolved_manifest_url)?)
    } else {
        None
    };

    let url = explicit_url.unwrap_or_else(|| manifest.as_ref().map(|item| item.url.clone()).unwrap());
    let output_path = args
        .get("output_path")
        .and_then(|value| value.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_download_path(&url));

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    download_to_path(&url, &output_path)?;

    Ok(serde_json::json!({
        "status": "downloaded",
        "url": url,
        "path": output_path,
        "manifest": manifest,
    }))
}

pub fn install_update(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Missing required update package path".to_string())?;
    let path = PathBuf::from(path);

    if !path.exists() {
        return Err(format!("Update package does not exist at {:?}", path));
    }

    let launcher = launch_installer(&path)?;

    Ok(serde_json::json!({
        "status": "launched",
        "path": path,
        "launcher": launcher,
    }))
}

fn load_manifest(manifest_url: &str) -> Result<UpdateManifest, String> {
    let manifest_text = read_text_from_url(manifest_url)?;
    serde_json::from_str::<UpdateManifest>(&manifest_text)
        .map_err(|err| format!("Invalid updater manifest JSON: {err}"))
}

fn read_text_from_url(url: &str) -> Result<String, String> {
    if let Some(path) = file_url_to_path(url) {
        return fs::read_to_string(path).map_err(|err| err.to_string());
    }

    if is_http_url(url) {
        let output = Command::new("curl")
            .arg("--fail")
            .arg("--silent")
            .arg("--show-error")
            .arg("--location")
            .arg(url)
            .output()
            .map_err(|err| format!("Failed to execute curl: {err}"))?;

        if !output.status.success() {
            return Err(format!(
                "curl failed for updater manifest with status {}",
                output.status
            ));
        }

        return String::from_utf8(output.stdout)
            .map_err(|err| format!("Manifest response was not valid UTF-8: {err}"));
    }

    fs::read_to_string(url).map_err(|err| err.to_string())
}

fn download_to_path(url: &str, output_path: &Path) -> Result<(), String> {
    if let Some(source_path) = file_url_to_path(url) {
        fs::copy(source_path, output_path)
            .map(|_| ())
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    if is_http_url(url) {
        let status = Command::new("curl")
            .arg("--fail")
            .arg("--silent")
            .arg("--show-error")
            .arg("--location")
            .arg("--output")
            .arg(output_path)
            .arg(url)
            .status()
            .map_err(|err| format!("Failed to execute curl: {err}"))?;

        if !status.success() {
            return Err(format!("curl failed to download update with status {status}"));
        }

        return Ok(());
    }

    fs::copy(url, output_path)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn default_download_path(url: &str) -> PathBuf {
    let file_name = url
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("rust-cef-update.bin");
    std::env::temp_dir().join("rust-cef-updates").join(file_name)
}

fn launch_installer(path: &Path) -> Result<&'static str, String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(path)
            .status()
            .map_err(|err| format!("Failed to execute open: {err}"))?;
        if !status.success() {
            return Err(format!("open failed to launch update package with status {status}"));
        }
        return Ok("open");
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", "start", "", path.to_string_lossy().as_ref()])
            .status()
            .map_err(|err| format!("Failed to execute start: {err}"))?;
        if !status.success() {
            return Err(format!("start failed to launch update package with status {status}"));
        }
        return Ok("start");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let status = Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|err| format!("Failed to execute xdg-open: {err}"))?;
        if !status.success() {
            return Err(format!(
                "xdg-open failed to launch update package with status {status}"
            ));
        }
        return Ok("xdg-open");
    }

    #[allow(unreachable_code)]
    Err("Installer handoff is not implemented for this platform".to_string())
}

fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn file_url_to_path(url: &str) -> Option<PathBuf> {
    url.strip_prefix("file://").map(PathBuf::from)
}

fn compare_versions(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    let left = parse_version(left)?;
    let right = parse_version(right)?;
    Ok(left.cmp(&right))
}

fn parse_version(value: &str) -> Result<Vec<u64>, String> {
    value
        .trim()
        .split('.')
        .map(|part| {
            part.parse::<u64>()
                .map_err(|_| format!("Unsupported version format '{value}'"))
        })
        .collect()
}
