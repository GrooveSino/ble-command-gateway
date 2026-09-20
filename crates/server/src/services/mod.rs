use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod command_runner;
mod network;
mod system_commands;
mod wifi_profiles;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub struct SystemExecResult {
    pub ok: bool,
    pub code: String,
    pub text: String,
    pub data: Option<Map<String, Value>>,
}

impl SystemExecResult {
    pub fn ok(text: impl Into<String>, data: Option<Map<String, Value>>) -> Self {
        Self {
            ok: true,
            code: protocol::codes::CODE_OK.to_string(),
            text: text.into(),
            data,
        }
    }

    pub fn with_code(
        ok: bool,
        code: impl Into<String>,
        text: impl Into<String>,
        data: Option<Map<String, Value>>,
    ) -> Self {
        Self {
            ok,
            code: code.into(),
            text: text.into(),
            data,
        }
    }

    pub fn error(code: impl Into<String>, text: impl Into<String>) -> Self {
        Self::with_code(false, code, text, None)
    }
}

#[derive(Clone)]
pub struct ServiceContext {
    snapshot: Arc<Mutex<IdentitySnapshot>>,
    pub(super) alias_path: PathBuf,
    pub(super) prefix: String,
    pub(super) serial: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IdentitySnapshot {
    device_name: String,
    alias: Option<String>,
}

impl ServiceContext {
    pub fn new(device_name: impl Into<String>) -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(IdentitySnapshot {
                device_name: device_name.into(),
                alias: None,
            })),
            alias_path: crate::alias_store::alias_path(),
            prefix: String::new(),
            serial: String::new(),
        }
    }

    pub fn live(
        prefix: impl Into<String>,
        serial: impl Into<String>,
        device_name: impl Into<String>,
        alias: Option<String>,
    ) -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(IdentitySnapshot {
                device_name: device_name.into(),
                alias,
            })),
            alias_path: crate::alias_store::alias_path(),
            prefix: prefix.into(),
            serial: serial.into(),
        }
    }

    pub fn device_name(&self) -> String {
        self.snapshot
            .lock()
            .expect("identity mutex")
            .device_name
            .clone()
    }

    pub fn alias(&self) -> Option<String> {
        self.snapshot.lock().expect("identity mutex").alias.clone()
    }

    pub(super) fn pending_identity(&self) -> Option<(String, Option<String>)> {
        if self.serial.is_empty() {
            return None;
        }
        let pending_alias = crate::alias_store::load_alias(&self.alias_path, &self.serial);
        let pending_name = crate::device_name::compose_device_name(
            &self.prefix,
            pending_alias.as_deref(),
            &self.serial,
        );
        if pending_name == self.device_name() {
            None
        } else {
            Some((pending_name, pending_alias))
        }
    }
}

pub async fn run_payload_command(
    context: &ServiceContext,
    payload: &protocol::requests::CommandPayload,
    timeout_sec: f64,
) -> SystemExecResult {
    match payload {
        protocol::requests::CommandPayload::LinkAck(_) => SystemExecResult::error(
            protocol::codes::CODE_ACK_BAD_REQUEST,
            "link.ack is handled by the transport layer",
        ),
        protocol::requests::CommandPayload::SystemCapabilities => {
            system_commands::run_capabilities()
        }
        protocol::requests::CommandPayload::LinkHeartbeat => system_commands::run_heartbeat(),
        protocol::requests::CommandPayload::SystemStatus => {
            system_commands::run_status(context, timeout_sec).await
        }
        protocol::requests::CommandPayload::SystemSetAlias { alias } => {
            system_commands::run_set_alias(context, alias)
        }
        protocol::requests::CommandPayload::WifiScan { ifname } => {
            network::run_wifi_scan(ifname.as_deref()).await
        }
        protocol::requests::CommandPayload::WifiProvision { ssid, pwd } => {
            network::run_wifi_provision(ssid, pwd.as_deref()).await
        }
        protocol::requests::CommandPayload::WifiProfilesList => {
            wifi_profiles::run_wifi_profiles_list().await
        }
        protocol::requests::CommandPayload::WifiProfilesDelete { uuids, force } => {
            wifi_profiles::run_wifi_profiles_delete(uuids, *force).await
        }
    }
}

async fn run_system_command(cmd: Vec<&str>, timeout_sec: f64) -> SystemExecResult {
    map_run_output(
        command_runner::run_command_with_timeout(cmd, timeout_sec).await,
        timeout_sec,
    )
}

fn map_run_output(output: command_runner::CommandRunOutput, timeout_sec: f64) -> SystemExecResult {
    match output.status {
        command_runner::CommandRunStatus::Succeeded(_) => {
            SystemExecResult::ok(output.preferred_text(), None)
        }
        command_runner::CommandRunStatus::Failed(_)
        | command_runner::CommandRunStatus::Error(_) => SystemExecResult::error(
            protocol::codes::CODE_INTERNAL_ERROR,
            output.preferred_text(),
        ),
        command_runner::CommandRunStatus::TimedOut => SystemExecResult::error(
            protocol::codes::CODE_TIMEOUT,
            format!("system command timeout after {:.1}s", timeout_sec),
        ),
        command_runner::CommandRunStatus::InvalidInput(message) => {
            SystemExecResult::error(protocol::codes::CODE_BAD_REQUEST, message)
        }
    }
}
