use super::event::TelemetryEvent;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::Write;

pub struct TelemetryStorage;

impl TelemetryStorage {
    pub fn get_log_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home.join(".sentinel/telemetry.log")
    }

    pub fn save_event(event: &TelemetryEvent) -> Result<(), String> {
        let path = Self::get_log_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create telemetry dir: {}", e))?;
        }

        let json = serde_json::to_string(event)
            .map_err(|e| format!("Failed to serialize event: {}", e))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("Failed to open telemetry log: {}", e))?;

        writeln!(file, "{}", json)
            .map_err(|e| format!("Failed to write telemetry log: {}", e))?;

        Ok(())
    }
}
