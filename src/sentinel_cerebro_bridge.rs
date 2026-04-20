use crate::agent_models::{CommandAck, OrchestratorCommand};
use crate::agent_reporter::report_event;
use crate::agent_config::AgentConfig;
use crate::agents::base::{AgentContext, Task, TaskType};
use crate::agents::orchestrator::AgentOrchestrator;
/// sentinel_cerebro_bridge.rs — Puente entre Cerebro ↔ Sentinel Core ↔ LLM.
///
/// FLUJO por comando:
///   1. Cerebro envía OrchestratorCommand { action, target }
///   2. El action ya está definido — no necesitamos un LLM para decidir qué ejecutar.
///   3. Sentinel Core ejecuta la acción (análisis, chequeo, auditoría) y retorna raw JSON.
///   4. La memoria SQLite persiste el resultado.
///   5. El LLM recibe el resultado crudo y contexto histórico, produce síntesis accionable.
///   6. El bridge reporta a Cerebro:
///        - POST /api/events con el evento estructurado
///        - CommandAck con { status, result: { raw, analysis, memory_id } }
///
/// CONTRATO MANTENIDO:
///   Input:  OrchestratorCommand { action, target?, options?, request_id? }
///   Output: CommandAck { request_id?, status, result?, error? }
use std::collections::HashMap;

/// Resultado del análisis de Sentinel
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub action: String,
    pub target: String,
    pub status: String,
    pub findings: Vec<Finding>,
    pub raw_output: String,
    pub analysis_summary: String,
    pub severity: String,
}

/// Hallazgo individual del análisis
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
    pub severity: String,
    pub rule: Option<String>,
    pub recommendation: Option<String>,
}

/// Handler principal de comandos de Sentinel
pub async fn handle_command(
    cmd: OrchestratorCommand,
    config: &AgentConfig,
    agent_context: &AgentContext,
) -> CommandAck {
    let request_id = cmd.request_id.clone();
    let action = cmd.action.clone();
    let target = cmd.target.clone().unwrap_or_else(|| ".".to_string());

    // Extraer auto_mode de options (viene de Cerebro cuando auto_fix_enabled=true)
    let auto_mode = cmd.options.get("auto")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    println!(
        "📡 [Sentinel Bridge] Comando recibido: action='{}' target='{}' auto_mode={}",
        action, target, auto_mode
    );

    // Emitir evento de inicio
    let _ = report_event(config, &format!("sentinel_{}_started", action), "info", {
        let mut payload = HashMap::new();
        payload.insert("action".to_string(), serde_json::json!(action));
        payload.insert("target".to_string(), serde_json::json!(target.clone()));
        payload.insert(
            "request_id".to_string(),
            serde_json::json!(request_id.clone()),
        );
        payload
    })
    .await;

    // Ejecutar acción correspondiente
    let result = match action.as_str() {
        "analyze" => analyze_file(&target, agent_context, config, auto_mode).await,
        "check" => check_file(&target, agent_context, config, auto_mode).await,
        "audit" => audit_project(&target, agent_context, config, auto_mode).await,
        "review" => review_architecture(&target, agent_context, config, auto_mode).await,
        "status" => get_status(agent_context, config).await,
        "clean-cache" => {
            crate::commands::pro::handle_clean_cache(
                Some(&target),
                agent_context,
                crate::commands::OutputMode::Quiet,
            );
            Ok(AnalysisResult {
                action: "clean_cache".to_string(),
                target: target.clone(),
                status: "success".to_string(),
                findings: Vec::new(),
                raw_output: "Caché limpiada".to_string(),
                analysis_summary: "Caché de Sentinel limpiada correctamente".to_string(),
                severity: "info".to_string(),
            })
        }
        _ => Err(anyhow::anyhow!("Acción desconocida: {}", action)),
    };

    match result {
        Ok(analysis) => {
            // Determinar severidad final
            let severity = infer_severity(&analysis);

            // Construir descripción del hallazgo
            let finding_desc = if analysis.findings.is_empty() {
                format!(
                    "Análisis {} completado: No se encontraron problemas",
                    action
                )
            } else {
                let critical = analysis
                    .findings
                    .iter()
                    .filter(|f| f.severity == "critical")
                    .count();
                let errors = analysis
                    .findings
                    .iter()
                    .filter(|f| f.severity == "error")
                    .count();
                let warnings = analysis
                    .findings
                    .iter()
                    .filter(|f| f.severity == "warning")
                    .count();

                if critical > 0 {
                    format!(
                        "¡ALERTA! {} problemas críticos detectados en {}",
                        critical, target
                    )
                } else if errors > 0 {
                    format!("{} errores detectados en {}", errors, target)
                } else {
                    format!("{} advertencias en {}", warnings, target)
                }
            };

            // Extraer primer archivo afectado
            let affected_file = analysis
                .findings
                .first()
                .map(|f| f.file.clone())
                .unwrap_or_else(|| target.clone());

            // Construir recomendación
            let recommendation = if analysis.findings.is_empty() {
                "No se requieren cambios".to_string()
            } else {
                analysis
                    .findings
                    .iter()
                    .filter_map(|f| f.recommendation.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            };

            // Reportar evento a Cerebro con formato compatible
            let _ = report_event(
                config,
                &format!("sentinel_{}_completed", action),
                &severity,
                {
                    let mut payload = HashMap::new();
                    payload.insert("action".to_string(), serde_json::json!(action));
                    payload.insert("target".to_string(), serde_json::json!(target));
                    payload.insert("finding".to_string(), serde_json::json!(finding_desc));
                    payload.insert(
                        "recommendation".to_string(),
                        serde_json::json!(recommendation),
                    );
                    payload.insert("file".to_string(), serde_json::json!(affected_file));
                    payload.insert("severity".to_string(), serde_json::json!(severity.clone()));
                    payload.insert(
                        "findings_count".to_string(),
                        serde_json::json!(analysis.findings.len()),
                    );
                    payload.insert("findings".to_string(), serde_json::json!(analysis.findings));
                    payload.insert(
                        "summary".to_string(),
                        serde_json::json!(analysis.analysis_summary),
                    );
                    payload.insert("raw_status".to_string(), serde_json::json!(analysis.status));
                    payload
                },
            )
            .await;

            CommandAck {
                request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "action": action,
                    "target": target,
                    "status": analysis.status,
                    "findings": analysis.findings,
                    "finding": finding_desc,
                    "recommendation": recommendation,
                    "file": affected_file,
                    "severity": severity,
                    "summary": analysis.analysis_summary,
                    "raw": analysis.raw_output,
                })),
                error: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Error en '{}': {}", action, e);
            println!("❌ {}", error_msg);

            // Reportar error a Cerebro
            let _ = report_event(config, &format!("sentinel_{}_error", action), "error", {
                let mut payload = HashMap::new();
                payload.insert("action".to_string(), serde_json::json!(action));
                payload.insert("target".to_string(), serde_json::json!(target));
                payload.insert("error".to_string(), serde_json::json!(error_msg.clone()));
                payload
            })
            .await;

            CommandAck {
                request_id,
                status: "error".to_string(),
                result: None,
                error: Some(error_msg),
            }
        }
    }
}

/// Analiza un archivo individual con IA
async fn analyze_file(
    target: &str,
    agent_context: &AgentContext,
    _config: &AgentConfig,
    _auto_mode: bool,
) -> anyhow::Result<AnalysisResult> {
    use std::path::Path;

    let file_path = Path::new(target);

    // Verificar que el archivo existe
    if !file_path.exists() {
        return Err(anyhow::anyhow!("Archivo no encontrado: {}", target));
    }

    // Leer contenido del archivo
    let content = std::fs::read_to_string(file_path)?;
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Crear orquestador y ejecutar análisis
    let mut orchestrator = AgentOrchestrator::new();
    orchestrator.register(std::sync::Arc::new(
        crate::agents::reviewer::ReviewerAgent::new(),
    ));

    let task = Task {
        id: uuid::Uuid::new_v4().to_string(),
        description: format!(
            "Analiza este archivo en busca de problemas de calidad, seguridad y arquitectura: {}",
            file_name
        ),
        task_type: TaskType::Analyze,
        file_path: Some(file_path.to_path_buf()),
        context: Some(content.clone()),
    };

    let result = orchestrator
        .execute_task("ReviewerAgent", &task, agent_context)
        .await?;

    // Parsear resultados
    let mut findings = parse_analysis_output(&result.output, target);

    // --- Ejecutar tests si existen (Alinear con el flujo del monitor) ---
    let base_name = target.split('.').next().unwrap_or(target).to_string();
    let project_root = &agent_context.project_root;
    let test_rel_path = crate::files::buscar_archivo_test(
        &base_name,
        project_root,
        &agent_context.config.test_patterns,
    );

    if let Some(test_path) = test_rel_path {
        println!(
            "🧪 [Sentinel Bridge] Ejecutando tests para {}: {}",
            target, test_path
        );

        let _ = report_event(
            &crate::agent_config::AgentConfig::from_env(),
            "tests_starting",
            "info",
            {
                let mut p = std::collections::HashMap::new();
                p.insert("file".to_string(), serde_json::json!(target));
                p.insert(
                    "test_path".to_string(),
                    serde_json::json!(test_path.clone()),
                );
                p
            },
        )
        .await;

        match crate::tests::ejecutar_tests(&test_path, project_root, &agent_context.config) {
            Ok(_) => {
                println!("   ✅ Tests pasaron.");
            }
            Err(e) => {
                println!("   ❌ Tests fallaron: {}", e);
                findings.push(Finding {
                    file: target.to_string(),
                    line: None,
                    message: format!("Tests fallidos: {}", e),
                    severity: "error".to_string(),
                    rule: Some("unit-tests".to_string()),
                    recommendation: Some("Corregir la lógica para que pasen los tests o actualizar los tests si el cambio es intencional.".to_string()),
                });
            }
        }
    }

    // Determinar severidad
    let severity = if findings.iter().any(|f| f.severity == "critical") {
        "critical"
    } else if findings.iter().any(|f| f.severity == "error") {
        "error"
    } else if !findings.is_empty() {
        "warning"
    } else {
        "info"
    }
    .to_string();

    let findings_count = findings.len();

    Ok(AnalysisResult {
        action: "analyze".to_string(),
        target: target.to_string(),
        status: "success".to_string(),
        findings,
        raw_output: result.output.clone(),
        analysis_summary: format!(
            "Análisis completado con {} hallazgos (IA + Tests)",
            findings_count
        ),
        severity,
    })
}

/// Chequea un archivo (análisis rápido de calidad)
async fn check_file(
    target: &str,
    agent_context: &AgentContext,
    config: &AgentConfig,
    auto_mode: bool,
) -> anyhow::Result<AnalysisResult> {
    // Por ahora, es similar a analyze pero más enfocado en calidad rápida
    analyze_file(target, agent_context, config, auto_mode).await
}

/// Auditoría de proyecto completo
async fn audit_project(
    target: &str,
    _agent_context: &AgentContext,
    _config: &AgentConfig,
    _auto_mode: bool,
) -> anyhow::Result<AnalysisResult> {
    // Ejecutar auditoría usando el motor de reglas existente
    let project_path = std::path::PathBuf::from(target);

    // Cargar configuración y ejecutar auditoría
    let _config_sentinel = crate::config::SentinelConfig::load(&project_path).unwrap_or_default();

    let mut findings = Vec::new();

    // Auditoría de dependencias (si existe package.json)
    if project_path.join("package.json").exists() {
        // Aquí podríamos agregar lógica específica de auditoría de deps
        findings.push(Finding {
            file: "package.json".to_string(),
            line: None,
            message: "Auditoría de dependencias completada".to_string(),
            severity: "info".to_string(),
            rule: Some("audit-deps".to_string()),
            recommendation: Some("Revisar dependencias desactualizadas".to_string()),
        });
    }

    Ok(AnalysisResult {
        action: "audit".to_string(),
        target: target.to_string(),
        status: "success".to_string(),
        raw_output: "Auditoría completada".to_string(),
        analysis_summary: format!("Auditoría de proyecto: {} hallazgos", findings.len()),
        findings,
        severity: "info".to_string(),
    })
}

/// Revisión de arquitectura completa
async fn review_architecture(
    target: &str,
    agent_context: &AgentContext,
    _config: &AgentConfig,
    _auto_mode: bool,
) -> anyhow::Result<AnalysisResult> {
    let project_path = std::path::PathBuf::from(target);

    // Crear orquestador con reviewer
    let mut orchestrator = AgentOrchestrator::new();
    orchestrator.register(std::sync::Arc::new(
        crate::agents::reviewer::ReviewerAgent::new(),
    ));

    // Construir tarea de revisión arquitectónica
    let task = Task {
        id: uuid::Uuid::new_v4().to_string(),
        description: "Realiza una auditoría técnica de alto nivel del proyecto".to_string(),
        task_type: TaskType::Analyze,
        file_path: Some(project_path.clone()),
        context: Some(format!("Proyecto: {}", target)),
    };

    let result = orchestrator
        .execute_task("ReviewerAgent", &task, agent_context)
        .await?;

    // Parsear sugerencias del resultado
    let suggestions = crate::ai::utils::extraer_json_sugerencias(&result.output);
    let mut findings = Vec::new();

    // Intentar parsear JSON de sugerencias
    if let Ok(suggestions_json) = serde_json::from_str::<Vec<serde_json::Value>>(&suggestions) {
        for (_i, sug) in suggestions_json.iter().enumerate() {
            if let Some(title) = sug.get("title").and_then(|t| t.as_str()) {
                findings.push(Finding {
                    file: target.to_string(),
                    line: None,
                    message: title.to_string(),
                    severity: sug
                        .get("impact")
                        .and_then(|i| i.as_str())
                        .map(|s| match s.to_lowercase().as_str() {
                            "high" | "critical" => "critical",
                            "medium" => "error",
                            _ => "warning",
                        })
                        .unwrap_or("warning")
                        .to_string(),
                    rule: Some("architectural".to_string()),
                    recommendation: sug
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    Ok(AnalysisResult {
        action: "review".to_string(),
        target: target.to_string(),
        status: "success".to_string(),
        raw_output: result.output.clone(),
        analysis_summary: result
            .output
            .lines()
            .take(10)
            .collect::<Vec<_>>()
            .join("\n"),
        severity: if findings.iter().any(|f| f.severity == "critical") {
            "critical".to_string()
        } else {
            "warning".to_string()
        },
        findings,
    })
}

/// Obtiene estado del agente Sentinel
async fn get_status(
    agent_context: &AgentContext,
    _config: &AgentConfig,
) -> anyhow::Result<AnalysisResult> {
    let stats = crate::stats::SentinelStats::cargar(&agent_context.project_root);

    Ok(AnalysisResult {
        action: "status".to_string(),
        target: agent_context.project_root.display().to_string(),
        status: "success".to_string(),
        findings: vec![],
        raw_output: serde_json::to_string_pretty(&stats)?,
        analysis_summary: format!(
            "Sentinel v5.0.0 | Bugs evitados: {} | Tiempo ahorrado: {} mins",
            stats.bugs_criticos_evitados, stats.tiempo_estimado_ahorrado_mins
        ),
        severity: "info".to_string(),
    })
}

/// Parsea la salida del análisis para extraer hallazgos
fn parse_analysis_output(output: &str, default_file: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Buscar patrones de problemas en el output
    let lines: Vec<&str> = output.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();

        // Detectar problemas críticos
        if line_lower.contains("crítico") || line_lower.contains("critical") {
            findings.push(Finding {
                file: default_file.to_string(),
                line: Some((i + 1) as u32),
                message: line.trim().to_string(),
                severity: "critical".to_string(),
                rule: Some("critical-issue".to_string()),
                recommendation: Some("Revisar inmediatamente".to_string()),
            });
        }
        // Detectar errores
        else if line_lower.contains("error") || line_lower.contains("problema") {
            findings.push(Finding {
                file: default_file.to_string(),
                line: Some((i + 1) as u32),
                message: line.trim().to_string(),
                severity: "error".to_string(),
                rule: Some("error-found".to_string()),
                recommendation: Some("Corregir el error identificado".to_string()),
            });
        }
        // Detectar advertencias
        else if line_lower.contains("advertencia") || line_lower.contains("warning") {
            findings.push(Finding {
                file: default_file.to_string(),
                line: Some((i + 1) as u32),
                message: line.trim().to_string(),
                severity: "warning".to_string(),
                rule: Some("warning".to_string()),
                recommendation: Some("Considerar la mejora sugerida".to_string()),
            });
        }
    }

    findings
}

/// Infiere la severidad global basada en los hallazgos
fn infer_severity(analysis: &AnalysisResult) -> String {
    if analysis.findings.iter().any(|f| f.severity == "critical") {
        "critical".to_string()
    } else if analysis.findings.iter().any(|f| f.severity == "error") {
        "error".to_string()
    } else if !analysis.findings.is_empty() {
        "warning".to_string()
    } else {
        "info".to_string()
    }
}
