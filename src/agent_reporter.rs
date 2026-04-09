use crate::agent_config::AgentConfig;
use crate::agent_models::AgentEvent;
use std::collections::HashMap;
use reqwest::Client;
use std::time::Duration;

/// Número máximo de reintentos para reportar eventos
const MAX_RETRIES: u32 = 3;
/// Timeout para la conexión HTTP (segundos)
const HTTP_TIMEOUT_SECS: u64 = 20;
/// Backoff inicial entre reintentos (milisegundos)
const INITIAL_BACKOFF_MS: u64 = 1000;

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

    // Cliente con timeout aumentado
    let client = Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?;

    // Intentar con reintentos y backoff exponencial
    let mut last_error = None;
    for attempt in 0..MAX_RETRIES {
        match client.post(&url).json(&event).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    if attempt > 0 {
                        eprintln!("✅ Evento reportado exitosamente (intentó {}/{})", attempt + 1, MAX_RETRIES);
                    }
                    return Ok(());
                } else {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_else(|_| "sin cuerpo".to_string());
                    eprintln!("⚠️ Error reportando al Cerebro ({}): {}", status, body);
                    // Para errores 4xx no reintentamos (son errores del cliente)
                    if status.is_client_error() {
                        return Err(anyhow::anyhow!("HTTP {}: {}", status, body));
                    }
                    last_error = Some(format!("HTTP {}: {}", status, body));
                }
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                eprintln!("⏱️ Timeout/Error de conexión (intentó {}/{}): {}", attempt + 1, MAX_RETRIES, err_msg);
                last_error = Some(err_msg);
            }
        }

        // Backoff exponencial antes del siguiente intento
        if attempt < MAX_RETRIES - 1 {
            let backoff_ms = INITIAL_BACKOFF_MS * (2_u64.pow(attempt));
            eprintln!("🔄 Reintentando en {} ms...", backoff_ms);
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }
    }

    Err(anyhow::anyhow!(
        "Falló reporte después de {} intentos: {:?}",
        MAX_RETRIES,
        last_error
    ))
}
