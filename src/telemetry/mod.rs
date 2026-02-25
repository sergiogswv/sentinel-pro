pub mod event;
pub mod client;
pub mod storage;

pub use event::TelemetryEvent;
pub use client::TelemetryClient;
pub use storage::TelemetryStorage;

pub async fn record_command(
    command: &str,
    duration_ms: u64,
    success: bool,
) {
    let event = TelemetryEvent::new("command_executed", command, duration_ms, success);

    // Save locally
    if let Err(e) = TelemetryStorage::save_event(&event) {
        eprintln!("Failed to save telemetry: {}", e);
    }

    // Send to server
    let client = TelemetryClient::new();
    if let Err(e) = client.send_event(&event).await {
        eprintln!("Failed to send telemetry: {}", e);
    }
}
