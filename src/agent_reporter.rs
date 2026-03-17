use crate::agent_config::AgentConfig;
use crate::agent_models::AgentEvent;
use std::collections::HashMap;
use reqwest::Client;

pub async fn report_event(
    config: &AgentConfig,
    event_type: &str,
    severity: &str,
    payload: HashMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
    if !config.report_enabled {
        return Ok(());
    }

    let event = AgentEvent::new("sentinel", event_type, severity, payload);
    let url = format!("{}/api/events", config.cerebro_url);

    let client = Client::new();
    let res = client.post(&url)
        .json(&event)
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_else(|_| "sin cuerpo".to_string());
        eprintln!("⚠️ Error reportando al Cerebro ({}): {}", status, body);
    }

    Ok(())
}
