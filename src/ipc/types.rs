use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub id: u64,
    pub command: String,
    pub args: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub id: u64,
    pub data: Option<Value>,
    pub error: Option<String>,
}
