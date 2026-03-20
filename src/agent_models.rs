use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug)]
pub struct AgentEvent {
    pub id: String,
    pub source: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub severity: String,
    pub timestamp: DateTime<Utc>,
    pub payload: HashMap<String, serde_json::Value>,
}

impl AgentEvent {
    pub fn new(source: &str, event_type: &str, severity: &str, payload: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source: source.to_string(),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            timestamp: Utc::now(),
            payload,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct OrchestratorCommand {
    pub action: String,
    pub target: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    pub request_id: String,
    pub subcommand: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct CommandAck {
    pub request_id: String,
    pub status: String, // accepted, completed, rejected
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
