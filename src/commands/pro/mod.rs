pub mod audit;
pub mod check;
pub mod render;
pub mod review;

pub use render::{render_sarif, get_changed_files, SarifIssue};
pub use review::{ReviewRecord, save_review_record, load_review_records, diff_reviews};
pub use audit::AuditIssue;

use crate::agents::base::AgentContext;
use crate::commands::ProCommands;
use crate::config::SentinelConfig;
use crate::index::IndexDb;
use crate::index::ProjectIndexBuilder;
use crate::commands::index::count_project_files;
use colored::*;
use std::env;
use std::sync::Arc;

/// Convert a format string to (json_mode, sarif_mode) flags.
/// Case-insensitive.
pub fn format_to_mode(format: &str) -> (bool, bool) {
    let lower = format.to_lowercase();
    let json_mode = lower == "json";
    let sarif_mode = lower == "sarif";
    (json_mode, sarif_mode)
}

pub fn handle_pro_command(subcommand: ProCommands, quiet: bool, verbose: bool) {
    let output_mode = crate::commands::get_output_mode(quiet, verbose);

    // Buscar la raíz del proyecto inteligentemente
    let project_root = SentinelConfig::find_project_root()
        .unwrap_or_else(|| env::current_dir().expect("No se pudo obtener el directorio actual"));

    if output_mode != crate::commands::OutputMode::Quiet && project_root != env::current_dir().unwrap_or_default() {
        println!(
            "{} {}",
            "📂 Proyecto Activo:".cyan().bold(),
            project_root.display().to_string().bright_blue()
        );
    }

    let config = SentinelConfig::load(&project_root).unwrap_or_else(|| {
        if !project_root.join(".sentinelrc.toml").exists() {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!(
                    "{} {}",
                    "⚠️".yellow(),
                    "No se encontró configuración (.sentinelrc.toml) en este directorio ni en padres."
                        .yellow()
                );
                println!("   Ejecuta 'sentinel' primero para configurar un proyecto.");
            }
        }
        SentinelConfig::default()
    });

    let db_path = project_root.join(".sentinel/index.db");
    let index_db = match IndexDb::open(&db_path) {
        Ok(db) => Some(Arc::new(db)),
        Err(_) => {
            // Si falla abrirlo, intentamos crear el directorio si no existe
            let _ = std::fs::create_dir_all(project_root.join(".sentinel"));
            IndexDb::open(&db_path).ok().map(Arc::new)
        }
    };

    let stats = Arc::new(std::sync::Mutex::new(crate::stats::SentinelStats::cargar(&project_root)));

    let agent_context = AgentContext {
        config: Arc::new(config),
        stats,
        project_root,
        index_db,
    };

    // Inicializar Orquestador y Agentes
    let mut orchestrator = crate::agents::orchestrator::AgentOrchestrator::new();
    orchestrator.register(Arc::new(crate::agents::fix_suggester::FixSuggesterAgent::new()));
    orchestrator.register(Arc::new(crate::agents::reviewer::ReviewerAgent::new()));
    orchestrator.register(Arc::new(crate::agents::tester::TesterAgent::new()));
    orchestrator.register(Arc::new(crate::agents::splitter::SplitterAgent::new()));

    // Ejecutar en Runtime de Tokio
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Detect JSON/SARIF mode before dispatching (to suppress indexing messages in machine-readable output)
    let json_mode_global = match &subcommand {
        ProCommands::Check { format, .. } => {
            let fmt = format.to_lowercase();
            fmt == "json" || fmt == "sarif"
        }
        ProCommands::Audit { format, .. } => format.to_lowercase() == "json",
        _ => false,
    };

    // Auto-indexación: si el índice está vacío, indexar en background mientras corre el comando
    let mut index_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>> = None;
    if let Some(ref db) = agent_context.index_db {
        if !db.is_populated() {
            if !json_mode_global && output_mode != crate::commands::OutputMode::Quiet {
                println!(
                    "\n{} {}",
                    "🧠 Indexando proyecto por primera vez...".cyan(),
                    "(segundo plano)".dimmed()
                );
            }
            let db_clone = Arc::clone(db);
            let root_clone = agent_context.project_root.clone();
            let extensions_clone = agent_context.config.file_extensions.clone();
            index_handle = Some(std::thread::spawn(move || {
                let builder = ProjectIndexBuilder::new(db_clone);
                builder.index_project(&root_clone, &extensions_clone)
            }));
        }
    }

    // Stale-index warning: warn once if disk file count diverges significantly from index
    if !json_mode_global && output_mode != crate::commands::OutputMode::Quiet {
        if let Some(ref db) = agent_context.index_db {
            if db.is_populated() {
                let disk_count = count_project_files(
                    &agent_context.project_root,
                    &agent_context.config.file_extensions,
                );
                let index_count = db.indexed_file_count();
                let diff = (disk_count as isize - index_count as isize).unsigned_abs();
                let stale_threshold = 5.max(disk_count / 10);
                if diff > stale_threshold {
                    println!(
                        "\n{} {} ({} indexados, {} en disco).",
                        "⚠️".yellow(),
                        "Índice posiblemente desactualizado".yellow(),
                        index_count,
                        disk_count
                    );
                    println!(
                        "   Corre {} para actualizar.\n",
                        "`sentinel index --rebuild`".cyan()
                    );
                }
            }
        }
    }

    match subcommand {
        ProCommands::Check { target, format } => {
            check::handle_check(target, format, quiet, verbose, &agent_context, output_mode, index_handle);
        }
        ProCommands::Review { history, diff } => {
            review::handle_review(history, diff, quiet, verbose, &agent_context, output_mode, &rt);
        }
        ProCommands::Audit { target, no_fix, format, max_files, concurrency } => {
            audit::handle_audit(target, no_fix, format, max_files, concurrency, quiet, verbose, &agent_context, output_mode, index_handle, &rt);
        }
        ProCommands::Analyze { file } => {
            handle_analyze(&file, &agent_context, &orchestrator, output_mode, &rt);
        }
        ProCommands::Report { format } => {
            handle_report(&format, &agent_context, output_mode, &rt);
        }
        ProCommands::Split { file } => {
            handle_split(&file, &agent_context, &orchestrator, output_mode, &rt);
        }
        ProCommands::Fix { file } => {
            handle_fix(&file, &agent_context, &orchestrator, output_mode, &rt);
        }
        ProCommands::TestAll => {
            handle_test_all(&agent_context, &orchestrator, output_mode, &rt);
        }
        ProCommands::Ml { subcommand } => {
            handle_ml(subcommand, &agent_context, output_mode, &rt);
        }
        ProCommands::CleanCache { target } => {
            handle_clean_cache(target.as_deref(), &agent_context, output_mode);
        }
        ProCommands::Workflow { name, file } => {
            handle_workflow(&name, file.as_deref(), &agent_context, &orchestrator, output_mode, &rt);
        }
    }
}

// Handler functions for remaining commands
fn handle_analyze(
    file: &str,
    _agent_context: &AgentContext,
    _orchestrator: &crate::agents::orchestrator::AgentOrchestrator,
    output_mode: crate::commands::OutputMode,
    _rt: &tokio::runtime::Runtime,
) {
    // Placeholder - would be implemented from original pro.rs Analyze handler
    if output_mode != crate::commands::OutputMode::Quiet {
        println!("Analyze handler stub: {}", file);
    }
}

fn handle_report(
    format: &str,
    _agent_context: &AgentContext,
    output_mode: crate::commands::OutputMode,
    _rt: &tokio::runtime::Runtime,
) {
    // Placeholder
    if output_mode != crate::commands::OutputMode::Quiet {
        println!("Report handler stub: {}", format);
    }
}

fn handle_split(
    _file: &str,
    _agent_context: &AgentContext,
    _orchestrator: &crate::agents::orchestrator::AgentOrchestrator,
    output_mode: crate::commands::OutputMode,
    _rt: &tokio::runtime::Runtime,
) {
    // Placeholder
    if output_mode != crate::commands::OutputMode::Quiet {
        println!("Split handler stub");
    }
}

fn handle_fix(
    _file: &str,
    _agent_context: &AgentContext,
    _orchestrator: &crate::agents::orchestrator::AgentOrchestrator,
    output_mode: crate::commands::OutputMode,
    _rt: &tokio::runtime::Runtime,
) {
    // Placeholder
    if output_mode != crate::commands::OutputMode::Quiet {
        println!("Fix handler stub");
    }
}

fn handle_test_all(
    _agent_context: &AgentContext,
    _orchestrator: &crate::agents::orchestrator::AgentOrchestrator,
    output_mode: crate::commands::OutputMode,
    _rt: &tokio::runtime::Runtime,
) {
    // Placeholder
    if output_mode != crate::commands::OutputMode::Quiet {
        println!("TestAll handler stub");
    }
}

fn handle_ml(
    _subcommand: crate::commands::MlCommands,
    _agent_context: &AgentContext,
    output_mode: crate::commands::OutputMode,
    _rt: &tokio::runtime::Runtime,
) {
    // Placeholder
    if output_mode != crate::commands::OutputMode::Quiet {
        println!("ML handler stub");
    }
}

fn handle_clean_cache(
    target: Option<&str>,
    _agent_context: &AgentContext,
    output_mode: crate::commands::OutputMode,
) {
    // Placeholder
    if output_mode != crate::commands::OutputMode::Quiet {
        match target {
            Some(t) => println!("CleanCache handler stub: {}", t),
            None => println!("CleanCache handler stub: all"),
        }
    }
}

fn handle_workflow(
    _name: &str,
    _file: Option<&str>,
    _agent_context: &AgentContext,
    _orchestrator: &crate::agents::orchestrator::AgentOrchestrator,
    output_mode: crate::commands::OutputMode,
    _rt: &tokio::runtime::Runtime,
) {
    // Placeholder
    if output_mode != crate::commands::OutputMode::Quiet {
        println!("Workflow handler stub");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_to_mode_json() {
        let (json, sarif) = format_to_mode("json");
        assert!(json);
        assert!(!sarif);
    }

    #[test]
    fn test_format_to_mode_sarif() {
        let (json, sarif) = format_to_mode("sarif");
        assert!(!json);
        assert!(sarif);
    }

    #[test]
    fn test_format_to_mode_text() {
        let (json, sarif) = format_to_mode("text");
        assert!(!json);
        assert!(!sarif);
    }

    #[test]
    fn test_format_to_mode_case_insensitive() {
        let (json, _) = format_to_mode("JSON");
        assert!(json, "format detection must be case-insensitive");
    }
}
