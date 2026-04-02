use axum::{
    routing::post,
    Json, Router,
};
use crate::agent_config::AgentConfig;
use crate::agent_models::{OrchestratorCommand, CommandAck};
use crate::agents::base::AgentContext;
use std::net::SocketAddr;
use std::thread;
use std::sync::Arc;
use std::collections::HashMap;

pub async fn start_server(config: AgentConfig) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/command", post(http_handle_command));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("🚀 Sentinel Agente escuchando en http://{}", addr);

    // Reportar evento ready al Cerebro cuando Sentinel Core está listo
    if config.report_enabled {
        let _ = crate::agent_reporter::report_event(
            &config,
            "sentinel_ready",
            "info",
            HashMap::from([
                ("message".to_string(), serde_json::Value::String(format!("Sentinel Core v{} listo", crate::config::SENTINEL_VERSION))),
                ("version".to_string(), serde_json::Value::String(crate::config::SENTINEL_VERSION.to_string())),
                ("port".to_string(), serde_json::Value::Number(config.port.into())),
            ])
        ).await;
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn http_handle_command(
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
        // Comandos de Análisis (usando el bridge)
        "analyze" | "check" | "audit" | "review" => {
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());

            // Construir AgentContext
            let project_root = if target == "." {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            } else {
                std::path::PathBuf::from(&target)
            };

            let config_sentinel = crate::config::SentinelConfig::load(&project_root)
                .unwrap_or_default();

            let stats = Arc::new(std::sync::Mutex::new(
                crate::stats::SentinelStats::cargar(&project_root)
            ));

            let db_path = project_root.join(".sentinel/index.db");
            let index_db = crate::index::IndexDb::open(&db_path)
                .ok()
                .map(Arc::new);

            let agent_context = AgentContext {
                config: Arc::new(config_sentinel),
                stats,
                project_root: project_root.clone(),
                index_db,
            };

            // Ejecutar comando usando el bridge
            let cerebro_config = crate::agent_config::AgentConfig::from_env();
            let ack = crate::sentinel_cerebro_bridge::handle_command(cmd, &cerebro_config, &agent_context).await;
            Json(ack)
        }

        // Comandos del Monitor remotos
        "monitor/pause" => {
            // Alternar estado de pausa (no hay estado global, solo señal)
            println!("⏸️ Pausa/Reanudación del monitoreo solicitada");
            Json(CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "message": "Comando de pausa enviado (requiere reinicio de monitor para aplicar)"
                })),
                error: None,
            })
        }
        "monitor/daily-report" => {
            // Generar reporte diario desde Git
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());
            let project_path = std::path::PathBuf::from(&target);

            if !project_path.exists() {
                return Json(CommandAck {
                    request_id: cmd.request_id,
                    status: "error".to_string(),
                    result: None,
                    error: Some(format!("Proyecto no encontrado: {}", target)),
                });
            }

            let config = crate::config::SentinelConfig::load(&project_path).unwrap_or_default();
            let stats = crate::stats::SentinelStats::cargar(&project_path);

            match crate::git::generar_reporte_diario_inner(&project_path, &config, &stats) {
                Ok(report) => Json(CommandAck {
                    request_id: cmd.request_id,
                    status: "completed".to_string(),
                    result: Some(serde_json::json!({
                        "report": report,
                        "message": "Reporte diario generado"
                    })),
                    error: None,
                }),
                Err(e) => Json(CommandAck {
                    request_id: cmd.request_id,
                    status: "error".to_string(),
                    result: None,
                    error: Some(format!("Error generando reporte: {}", e)),
                }),
            }
        }
        "monitor/testing" => {
            // Generar sugerencias de testing
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());
            let project_path = std::path::PathBuf::from(&target);

            if !project_path.exists() {
                return Json(CommandAck {
                    request_id: cmd.request_id,
                    status: "error".to_string(),
                    result: None,
                    error: Some(format!("Proyecto no encontrado: {}", target)),
                });
            }

            // Obtener lista de archivos del proyecto
            let mut test_suggestions = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&project_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        if ["rs", "ts", "js", "py", "go", "java", "cs"].contains(&ext) {
                            if !path.to_string_lossy().contains(".test.") && !path.to_string_lossy().contains("_test.") {
                                test_suggestions.push(format!("Agregar test para: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                            }
                        }
                    }
                }
            }

            Json(CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "suggestions": test_suggestions,
                    "message": "Sugerencias de testing generadas"
                })),
                error: None,
            })
        }
        "monitor/metrics" => {
            // Retornar métricas del proyecto
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());
            let project_path = std::path::PathBuf::from(&target);

            let stats = crate::stats::SentinelStats::cargar(&project_path);

            Json(CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "bugs_evitados": stats.bugs_criticos_evitados,
                    "costo_acumulado": stats.total_cost_usd,
                    "tokens_usados": stats.total_tokens_used,
                    "tiempo_ahorrado_mins": stats.tiempo_estimado_ahorrado_mins
                })),
                error: None,
            })
        }
        "monitor/reset-config" => {
            // Reiniciar configuración
            let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());
            let project_path = std::path::PathBuf::from(&target);

            match crate::config::SentinelConfig::eliminar(&project_path) {
                Ok(_) => Json(CommandAck {
                    request_id: cmd.request_id,
                    status: "completed".to_string(),
                    result: Some(serde_json::json!({
                        "message": "Configuración reiniciada exitosamente"
                    })),
                    error: None,
                }),
                Err(e) => Json(CommandAck {
                    request_id: cmd.request_id,
                    status: "error".to_string(),
                    result: None,
                    error: Some(format!("Error al reiniciar config: {}", e)),
                }),
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
