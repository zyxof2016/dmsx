use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct CreateDeviceReq {
    pub platform: String,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    pub labels: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct Device {
    pub id: String,
    pub hostname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct CommandItem {
    pub id: String,
    pub status: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SubmitResultReq {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateStatusReq {
    pub status: String,
}
