use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub event_type: String,
    pub timestamp: String,
    pub session_id: String,
    pub sentinel_version: String,
    pub os: String,
    pub os_version: String,
    pub command: String,
    pub duration_ms: u64,
    pub success: bool,
}

impl TelemetryEvent {
    pub fn new(
        event_type: &str,
        command: &str,
        duration_ms: u64,
        success: bool,
    ) -> Self {
        Self {
            event_type: event_type.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            session_id: Uuid::new_v4().to_string(),
            sentinel_version: env!("CARGO_PKG_VERSION").to_string(),
            os: std::env::consts::OS.to_string(),
            os_version: std::env::consts::ARCH.to_string(),
            command: command.to_string(),
            duration_ms,
            success,
        }
    }
}
