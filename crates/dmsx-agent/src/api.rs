use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct CreateDeviceReq {
    pub(crate) platform: String,
    pub(crate) hostname: Option<String>,
    pub(crate) os_version: Option<String>,
    pub(crate) agent_version: Option<String>,
    pub(crate) labels: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Device {
    pub(crate) id: String,
    pub(crate) hostname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListResponse<T> {
    pub(crate) items: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CommandItem {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct SubmitResultReq {
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpdateStatusReq {
    pub(crate) status: String,
}
