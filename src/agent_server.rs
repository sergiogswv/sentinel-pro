use axum::{
    routing::post,
    Json, Router,
};
use crate::agent_config::AgentConfig;
use crate::agent_models::{OrchestratorCommand, CommandAck};
use std::net::SocketAddr;
use std::thread;

pub async fn start_server(config: AgentConfig) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/command", post(handle_command));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("🚀 Sentinel Agente escuchando en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_command(
    Json(cmd): Json<OrchestratorCommand>,
) -> Json<CommandAck> {
    println!("📨 Comando recibido: action={} target={:?}", cmd.action, cmd.target);

    match cmd.action.as_str() {
        "monitor" | "start" => {
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());
            
            println!("🔄 Reiniciando monitoreo sobre: {}", target);
            
            // Indicar a hilos previos que se detengan
            crate::commands::monitor::STOP_SIGNAL.store(true, std::sync::atomic::Ordering::SeqCst);
            
            // Lanzar el monitoreo en un hilo separado
            thread::spawn(move || {
                // Esperar un poco a que el hilo anterior se entere y libere recursos
                thread::sleep(std::time::Duration::from_millis(1000));
                crate::commands::monitor::start_monitor(Some(target));
            });

            Json(CommandAck {
                request_id: cmd.request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({
                    "action": cmd.action,
                    "target": cmd.target.unwrap_or_else(|| ".".to_string()),
                    "message": "Monitoreo iniciado en segundo plano"
                })),
                error: None,
            })
        }
        "status" => {
            Json(CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "agent": "sentinel",
                    "version": "5.0.0",
                    "ready": true
                })),
                error: None,
            })
        }
        "answer" => {
            // Recibe respuesta de un prompt interactivo
            let prompt_id = cmd.options.get("prompt_id").and_then(|v| v.as_str());
            let answer = cmd.options.get("answer").and_then(|v| v.as_str());

            if let (Some(pid), Some(ans)) = (prompt_id, answer) {
                let resolved = crate::agent_interaction::MANAGER.resolve(pid, ans.to_string());
                Json(CommandAck {
                    request_id: cmd.request_id,
                    status: if resolved { "completed".to_string() } else { "rejected".to_string() },
                    result: Some(serde_json::json!({ "resolved": resolved })),
                    error: if resolved { None } else { Some("Prompt ID no encontrado o expirado".to_string()) },
                })
            } else {
                Json(CommandAck {
                    request_id: cmd.request_id,
                    status: "rejected".to_string(),
                    result: None,
                    error: Some("Faltan prompt_id o answer en options".to_string()),
                })
            }
        }
        _ => {
            Json(CommandAck {
                request_id: cmd.request_id,
                status: "rejected".to_string(),
                result: None,
                error: Some(format!("Acción desconocida: {}", cmd.action)),
            })
        }
    }
}
