use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLaunchPayload {
    pub args: Vec<String>,
}

fn socket_path() -> PathBuf {
    std::env::temp_dir().join("rust-cef-single-instance.sock")
}

#[cfg(unix)]
pub enum InstanceMode {
    Primary(UnixListener),
    Secondary,
}

#[cfg(not(unix))]
pub enum InstanceMode {
    Primary,
}

#[cfg(unix)]
pub fn acquire(args: &[String]) -> Result<InstanceMode, String> {
    let socket_path = socket_path();

    if socket_path.exists() {
        match UnixStream::connect(&socket_path) {
            Ok(mut stream) => {
                let payload = serde_json::to_vec(&ExternalLaunchPayload {
                    args: args.to_vec(),
                })
                .map_err(|err| err.to_string())?;
                stream.write_all(&payload).map_err(|err| err.to_string())?;
                return Ok(InstanceMode::Secondary);
            }
            Err(_) => {
                let _ = std::fs::remove_file(&socket_path);
            }
        }
    }

    let listener = UnixListener::bind(&socket_path).map_err(|err| err.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|err| err.to_string())?;
    Ok(InstanceMode::Primary(listener))
}

#[cfg(not(unix))]
pub fn acquire(_args: &[String]) -> Result<InstanceMode, String> {
    Ok(InstanceMode::Primary)
}

#[cfg(unix)]
pub fn start_listener(
    listener: UnixListener,
    on_launch: Box<dyn Fn(ExternalLaunchPayload) + Send + 'static>,
) {
    std::thread::spawn(move || loop {
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                let mut buffer = Vec::new();
                if stream.read_to_end(&mut buffer).is_ok() {
                    if let Ok(payload) = serde_json::from_slice::<ExternalLaunchPayload>(&buffer) {
                        on_launch(payload);
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(_) => break,
        }
    });
}

pub fn extract_launch_context(args: &[String]) -> crate::state::LaunchContext {
    crate::state::LaunchContext {
        deep_link: crate::security::extract_deep_link_arg(args),
        files: crate::security::extract_file_args(args),
    }
}
