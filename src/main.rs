//! # Sentinel Pro - AI-Powered Code Monitor & Development Suite
//!
//! Herramienta de monitoreo en tiempo real que vigila cambios en archivos TypeScript,
//! analiza el código con IA, ejecuta tests y gestiona commits automáticamente.
//! Ahora con capacidades extendidas en su versión Pro.

use clap::Parser;
use commands::{Cli, Commands};

// Módulos
pub mod agents;
pub mod ai;
pub mod commands;
pub mod config;
pub mod docs;
pub mod files;
pub mod git;
pub mod index;
pub mod business_logic_guard;
pub mod ml;
pub mod rules;
pub mod stats;
pub mod tests;
pub mod ui;
pub mod telemetry;
pub mod update;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Monitor { daemon, stop, status }) => {
            let project_root = crate::config::SentinelConfig::find_project_root()
                .unwrap_or_else(|| std::env::current_dir().unwrap());

            if stop {
                if let Err(e) = commands::monitor::handle_stop(&project_root) {
                    eprintln!("❌ Error al detener daemon: {}", e);
                    std::process::exit(1);
                }
            } else if status {
                if let Err(e) = commands::monitor::handle_status(&project_root) {
                    eprintln!("❌ Error al obtener estado: {}", e);
                    std::process::exit(1);
                }
            } else if daemon {
                if let Err(e) = commands::monitor::handle_daemon(&project_root) {
                    eprintln!("❌ Error iniciando daemon: {}", e);
                    std::process::exit(1);
                }
            } else {
                commands::monitor::start_monitor();
            }
        }
        Some(Commands::Init { force }) => {
            let project_root = crate::config::SentinelConfig::find_project_root()
                .unwrap_or_else(|| std::env::current_dir().unwrap());
            commands::init::handle_init_command(&project_root, force);
        }
        Some(Commands::Ignore { rule, file, symbol, list, clear, show_file }) => {
            commands::ignore::handle_ignore_command(rule, file, symbol, list, clear, show_file);
        }
        Some(Commands::Index { rebuild, check }) => {
            commands::index::handle_index_command(rebuild, check);
        }
        Some(Commands::Pro { subcommand }) => {
            commands::pro::handle_pro_command(subcommand, cli.quiet, cli.verbose);
        }
        Some(Commands::Doctor) => {
            let project_root = crate::config::SentinelConfig::find_project_root()
                .unwrap_or_else(|| std::env::current_dir().unwrap());
            commands::doctor::handle_doctor_command(&project_root);
        }
        Some(Commands::Rules) => {
            let project_root = crate::config::SentinelConfig::find_project_root()
                .unwrap_or_else(|| std::env::current_dir().unwrap());
            commands::rules::handle_rules_command(&project_root);
        }
        Some(Commands::PreCommit { install, uninstall, status }) => {
            let project_root = crate::config::SentinelConfig::find_project_root()
                .unwrap_or_else(|| std::env::current_dir().unwrap());

            let action = if install {
                commands::precommit::PreCommitAction::Install
            } else if uninstall {
                commands::precommit::PreCommitAction::Uninstall
            } else if status {
                commands::precommit::PreCommitAction::Status
            } else {
                // Default to status if no action specified
                commands::precommit::PreCommitAction::Status
            };

            commands::precommit::handle_precommit_command(&project_root, action);
        }
        Some(Commands::GitHubActions { workflow_type }) => {
            let project_root = crate::config::SentinelConfig::find_project_root()
                .unwrap_or_else(|| std::env::current_dir().unwrap());

            let action = match workflow_type.as_str() {
                "analysis" => commands::github_actions::WorkflowType::Analysis,
                "tests" => commands::github_actions::WorkflowType::Tests,
                "security" => commands::github_actions::WorkflowType::Security,
                _ => commands::github_actions::WorkflowType::All,
            };

            commands::github_actions::handle_github_actions_command(&project_root, action);
        }
        None => {
            // Comportamiento por defecto (legacy)
            commands::monitor::start_monitor();
        }
    }
}
