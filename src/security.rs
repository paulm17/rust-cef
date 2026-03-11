use serde_json::Value;
use std::sync::OnceLock;

static RUNTIME_DEV_MODE: OnceLock<bool> = OnceLock::new();

pub fn set_runtime_dev_mode(dev_mode: bool) {
    let _ = RUNTIME_DEV_MODE.set(dev_mode);
}

pub fn runtime_dev_mode() -> bool {
    *RUNTIME_DEV_MODE.get_or_init(|| false)
}

pub fn enforce_url_policy(url: &str, dev_mode: bool) -> Result<(), String> {
    if url.starts_with("app://") || url == "about:blank" {
        return Ok(());
    }

    if dev_mode
        && (is_loopback_http(url)
            || url.starts_with("https://")
            || url.starts_with("file://")
            || url.starts_with("data:"))
    {
        return Ok(());
    }

    Err(format!(
        "Blocked insecure or unsupported URL by policy: {url}. Production windows must use app:// URLs."
    ))
}

pub fn make_deep_link_start_url(base_url: &str, deep_link: &str) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    let encoded: String = url::form_urlencoded::byte_serialize(deep_link.as_bytes()).collect();
    format!("{base_url}{separator}deep_link={encoded}")
}

pub fn extract_deep_link_arg(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with("--") && arg.contains("://"))
        .cloned()
}

pub fn extract_file_args(args: &[String]) -> Vec<String> {
    args.iter()
        .skip(1)
        .filter(|arg| !arg.starts_with("--") && !arg.contains("://"))
        .filter_map(|arg| {
            std::path::Path::new(arg).canonicalize().ok().or_else(|| {
                let path = std::path::PathBuf::from(arg);
                if path.exists() {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

pub fn request_bool(args: &Value, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(default)
}

fn is_loopback_http(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };

    if parsed.scheme() != "http" {
        return false;
    }

    matches!(
        parsed.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    )
}
