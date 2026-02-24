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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Monitor) => {
            commands::monitor::start_monitor();
        }
        Some(Commands::Ignore { rule, file, symbol, list, clear }) => {
            commands::ignore::handle_ignore_command(rule, file, symbol, list, clear);
        }
        Some(Commands::Index { rebuild, check }) => {
            commands::index::handle_index_command(rebuild, check);
        }
        Some(Commands::Pro { subcommand }) => {
            commands::pro::handle_pro_command(subcommand);
        }
        Some(Commands::Rules) => {
            let project_root = crate::config::SentinelConfig::find_project_root()
                .unwrap_or_else(|| std::env::current_dir().unwrap());
            commands::rules::handle_rules_command(&project_root);
        }
        None => {
            // Comportamiento por defecto (legacy)
            commands::monitor::start_monitor();
        }
    }
}
