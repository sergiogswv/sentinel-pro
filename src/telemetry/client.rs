use super::event::TelemetryEvent;

const _TELEMETRY_ENDPOINT: &str = "https://telemetry.sentinel.dev/events";

pub struct TelemetryClient {
}

impl TelemetryClient {
    pub fn new() -> Self {
        Self {
        }
    }

    pub async fn send_event(&self, event: &TelemetryEvent) -> Result<(), String> {
        // Check if telemetry is enabled
        if !is_telemetry_enabled() {
            return Ok(());
        }

        // For now, we'll just log it locally
        // In production, this would send to the actual endpoint
        eprintln!("[Telemetry] Event logged: {}", event.command);
        Ok(())
    }
}

fn is_telemetry_enabled() -> bool {
    // Check environment variable first
    if let Ok(val) = std::env::var("SENTINEL_TELEMETRY") {
        return val != "false";
    }

    // Default: enabled
    true
}
