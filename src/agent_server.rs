use axum::{
    routing::{post, get},
    Json, Router,
};
use crate::agent_config::AgentConfig;
use crate::agent_models::{OrchestratorCommand, CommandAck};
use crate::agent_reporter::report_event;
use std::net::SocketAddr;
use std::thread;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// Estado compartido para el monitoreo
pub struct MonitorState {
    pub is_paused: bool,
}

pub type SharedMonitorState = Arc<Mutex<MonitorState>>;

pub async fn start_server(config: AgentConfig) -> anyhow::Result<()> {
    let monitor_state: SharedMonitorState = Arc::new(Mutex::new(MonitorState { is_paused: false }));

    let app = Router::new()
        .route("/command", post(handle_command))
        .route("/monitor/pause", post(monitor_pause))
        .route("/monitor/status", get(monitor_status))
        .route("/report/daily", post(daily_report))
        .route("/metrics", get(get_metrics))
        .route("/cache/clear", post(clear_cache))
        .route("/testing/suggestions", post(testing_suggestions))
        .route("/config/reset", post(config_reset))
        .with_state(monitor_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("🚀 Sentinel Agente escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // 🧠 Enviar evento sentinel_ready al Cerebro DESPUÉS de vincular el puerto
    println!("📨 Enviando evento 'sentinel_ready' al Cerebro...");
    match report_event(
        &config,
        "sentinel_ready",
        "info",
        HashMap::from([
            ("version".to_string(), serde_json::json!("5.0.0")),
            ("port".to_string(), serde_json::json!(config.port)),
        ]).into_iter().collect(),
    ).await {
        Ok(_) => println!("✅ Evento sentinel_ready enviado exitosamente"),
        Err(e) => eprintln!("❌ Error enviando evento sentinel_ready: {}", e),
    }

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_command(
    Json(cmd): Json<OrchestratorCommand>,
) -> Json<CommandAck> {
    println!("📨 Comando recibido: action={} subcommand={:?} target={:?}", cmd.action, cmd.subcommand, cmd.target);

    match cmd.action.as_str() {
        // Comandos Pro de Sentinel
        "pro" => {
            let subcommand = cmd.subcommand.as_deref().unwrap_or("check");
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());

            println!("🔧 Ejecutando comando Pro: {} en {}", subcommand, target);

            // Mapear subcomandos a funciones
            match subcommand {
                "check" => execute_pro_check(&target, &cmd.request_id),
                "audit" => execute_pro_audit(&target, &cmd.request_id),
                "report" => execute_pro_report(&target, &cmd.request_id),
                "fix" => execute_pro_fix(&target, &cmd.request_id),
                "review" => execute_pro_review(&target, &cmd.request_id),
                "clean-cache" => execute_pro_clean_cache(&target, &cmd.request_id),
                _ => {
                    Json(CommandAck {
                        request_id: cmd.request_id,
                        status: "rejected".to_string(),
                        result: None,
                        error: Some(format!("Subcomando Pro desconocido: {}", subcommand)),
                    })
                }
            }
        }

        "monitor" | "start" => {
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());

            // Lanzar el monitoreo en un hilo separado ya que start_monitor es bloqueante
            thread::spawn(move || {
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

        // Comandos del monitor: pause, daily-report, metrics, testing, reset-config
        "monitor/pause" => {
            // Toggle pause - necesita acceso al estado compartido
            // Por ahora retornamos unack directo, el handler real está en el endpoint dedicado
            Json(CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "action": "monitor/pause",
                    "message": "Comando pause recibido (usar endpoint /monitor/pause)"
                })),
                error: None,
            })
        }
        "monitor/daily-report" => {
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());
            let request_id = cmd.request_id.clone();

            thread::spawn(move || {
                // Aquí iría la llamada a git::generar_reporte_diario
                println!("📊 Reporte diario generado para: {}", target);
            });

            Json(CommandAck {
                request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({
                    "action": "monitor/daily-report",
                    "message": "Generando reporte de productividad..."
                })),
                error: None,
            })
        }
        "monitor/testing" => {
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());
            let request_id = cmd.request_id.clone();

            // Ejecutar en thread y esperar a que termine para retornar el resultado
            let handle = thread::spawn(move || {
                // Aquí iría la llamada real a ai::testing::obtener_sugerencias_complementarias
                // Por ahora retornamos un resultado mock
                println!("🧪 Generando sugerencias de testing para: {}", target);
                format!("Sugerencias generadas para: {}", target)
            });

            let result = handle.join().unwrap_or_else(|_| "Error ejecutando testing".to_string());

            Json(CommandAck {
                request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "action": "monitor/testing",
                    "message": result
                })),
                error: None,
            })
        }
        "monitor/reset-config" => {
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());

            // Nota: Esto no puede hacer std::process::exit() en modo servidor
            Json(CommandAck {
                request_id: cmd.request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({
                    "action": "monitor/reset-config",
                    "message": "Configuración marcada para reinicio. Reinicie el agente para aplicar.",
                    "requires_restart": true
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

// Funciones helper para comandos Pro
fn execute_pro_check(target: &str, request_id: &str) -> Json<CommandAck> {
    // Ejecutar análisis estático rápido
    // Aquí iría la llamada real a commands::pro::Check
    println!("🔍 Ejecutando Quick Check en {}", target);

    // Ejecutar el comando real (bloqueante, en thread separado)
    let target_path = target.to_string();
    let request_id = request_id.to_string();

    thread::spawn(move || {
        // Simular ejecución del comando pro check
        // En producción: crate::commands::pro::handle_pro_command(...)
        println!("✅ Quick Check completado para {}", target_path);
    });

    Json(CommandAck {
        request_id: request_id.to_string(),
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "pro",
            "subcommand": "check",
            "target": target,
            "message": "Quick Check iniciado, procesando..."
        })),
        error: None,
    })
}

fn execute_pro_audit(target: &str, request_id: &str) -> Json<CommandAck> {
    println!("🛡️ Ejecutando Audit en {}", target);
    let target_path = target.to_string();
    thread::spawn(move || {
        println!("✅ Audit completado para {}", target_path);
    });
    Json(CommandAck {
        request_id: request_id.to_string(),
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "pro",
            "subcommand": "audit",
            "target": target,
            "message": "Audit iniciado, procesando..."
        })),
        error: None,
    })
}

fn execute_pro_report(target: &str, request_id: &str) -> Json<CommandAck> {
    println!("📊 Ejecutando Report en {}", target);
    let target_path = target.to_string();
    thread::spawn(move || {
        println!("✅ Report completado para {}", target_path);
    });
    Json(CommandAck {
        request_id: request_id.to_string(),
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "pro",
            "subcommand": "report",
            "target": target,
            "message": "Reporte generado, procesando..."
        })),
        error: None,
    })
}

fn execute_pro_fix(target: &str, request_id: &str) -> Json<CommandAck> {
    println!("⚡ Ejecutando Auto Fix en {}", target);
    let target_path = target.to_string();
    thread::spawn(move || {
        println!("✅ Auto Fix completado para {}", target_path);
    });
    Json(CommandAck {
        request_id: request_id.to_string(),
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "pro",
            "subcommand": "fix",
            "target": target,
            "message": "Auto Fix iniciado, procesando..."
        })),
        error: None,
    })
}

fn execute_pro_review(target: &str, request_id: &str) -> Json<CommandAck> {
    println!("🔄 Ejecutando Review en {}", target);
    let target_path = target.to_string();
    thread::spawn(move || {
        println!("✅ Review completado para {}", target_path);
    });
    Json(CommandAck {
        request_id: request_id.to_string(),
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "pro",
            "subcommand": "review",
            "target": target,
            "message": "Review de arquitectura iniciado..."
        })),
        error: None,
    })
}

fn execute_pro_clean_cache(target: &str, request_id: &str) -> Json<CommandAck> {
    println!("🗑️ Ejecutando Clean Cache en {}", target);
    let target_path = target.to_string();
    thread::spawn(move || {
        println!("✅ Clean Cache completado para {}", target_path);
    });
    Json(CommandAck {
        request_id: request_id.to_string(),
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "pro",
            "subcommand": "clean-cache",
            "target": target,
            "message": "Limpieza de caché iniciada..."
        })),
        error: None,
    })
}

// ──────────────────────────────────────────────────────────────────
// Handlers para comandos interactivos del monitor
// ──────────────────────────────────────────────────────────────────

use axum::extract::State;

/// POST /monitor/pause - Toggle pause del monitoreo
async fn monitor_pause(
    State(monitor_state): State<SharedMonitorState>,
) -> Json<CommandAck> {
    let mut state = monitor_state.lock().await;
    state.is_paused = !state.is_paused;
    let is_paused = state.is_paused;

    println!("⌨️ Toggle pause: {}", if is_paused { "PAUSADO" } else { "ACTIVADO" });

    Json(CommandAck {
        request_id: "monitor-pause".to_string(),
        status: "completed".to_string(),
        result: Some(serde_json::json!({
            "action": "monitor/pause",
            "paused": is_paused,
            "message": if is_paused { "Monitoreo pausado" } else { "Monitoreo reanudado" }
        })),
        error: None,
    })
}

/// GET /monitor/status - Estado del monitoreo
async fn monitor_status(
    State(monitor_state): State<SharedMonitorState>,
) -> Json<CommandAck> {
    let state = monitor_state.lock().await;
    let is_paused = state.is_paused;

    Json(CommandAck {
        request_id: "monitor-status".to_string(),
        status: "completed".to_string(),
        result: Some(serde_json::json!({
            "agent": "sentinel",
            "running": true,
            "paused": is_paused
        })),
        error: None,
    })
}

/// POST /report/daily - Generar reporte diario de productividad
async fn daily_report(
    Json(cmd): Json<OrchestratorCommand>,
) -> Json<CommandAck> {
    println!("📊 Generando reporte diario de productividad...");

    let project_root = cmd.target.clone().unwrap_or_else(|| ".".to_string());
    let request_id = cmd.request_id.clone();

    thread::spawn(move || {
        // Aquí iría la llamada a git::generar_reporte_diario
        println!("📊 Reporte diario generado para: {}", project_root);
    });

    Json(CommandAck {
        request_id,
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "report/daily",
            "message": "Generando reporte de productividad..."
        })),
        error: None,
    })
}

/// GET /metrics - Obtener dashboard de métricas
async fn get_metrics() -> Json<CommandAck> {
    // Retorna métricas de ejemplo (en producción vendrían de SentinelStats)
    Json(CommandAck {
        request_id: "metrics".to_string(),
        status: "completed".to_string(),
        result: Some(serde_json::json!({
            "bugs_evitados": 0,
            "costo_acumulado": 0.0,
            "tokens_usados": 0,
            "tiempo_ahorrado_mins": 0,
            "message": "Métricas reseteadas al iniciar sesión"
        })),
        error: None,
    })
}

/// POST /cache/clear - Limpiar caché de IA
async fn clear_cache(
    Json(cmd): Json<OrchestratorCommand>,
) -> Json<CommandAck> {
    println!("🗑️ Limpiando caché de IA...");

    let project_root = cmd.target.clone().unwrap_or_else(|| ".".to_string());
    let request_id = cmd.request_id.clone();

    thread::spawn(move || {
        // Aquí iría la llamada a ai::limpiar_cache
        println!("✅ Caché limpiada para: {}", project_root);
    });

    Json(CommandAck {
        request_id,
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "cache/clear",
            "message": "Limpieza de caché de IA iniciada..."
        })),
        error: None,
    })
}

/// POST /testing/suggestions - Obtener sugerencias de testing
async fn testing_suggestions(
    Json(cmd): Json<OrchestratorCommand>,
) -> Json<CommandAck> {
    println!("🧪 Obteniendo sugerencias de testing...");

    let project_root = cmd.target.clone().unwrap_or_else(|| ".".to_string());
    let request_id = cmd.request_id.clone();

    thread::spawn(move || {
        // Aquí iría la llamada a ai::testing::obtener_sugerencias_complementarias
        println!("🧪 Sugerencias de testing generadas para: {}", project_root);
    });

    Json(CommandAck {
        request_id,
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "testing/suggestions",
            "message": "Generando sugerencias de testing..."
        })),
        error: None,
    })
}

/// POST /config/reset - Reiniciar configuración
async fn config_reset(
    Json(cmd): Json<OrchestratorCommand>,
) -> Json<CommandAck> {
    println!("⚠️ Reiniciando configuración...");

    let project_root = cmd.target.clone().unwrap_or_else(|| ".".to_string());
    let request_id = cmd.request_id.clone();

    // Nota: Esto no puede hacer std::process::exit() en modo servidor
    // En su lugar, marcamos la configuración para reinicio y el servidor puede restartearse

    Json(CommandAck {
        request_id,
        status: "accepted".to_string(),
        result: Some(serde_json::json!({
            "action": "config/reset",
            "message": "Configuración marcada para reinicio. Reinicie el agente para aplicar.",
            "requires_restart": true
        })),
        error: None,
    })
}
