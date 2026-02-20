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
        Some(Commands::Pro { subcommand }) => {
            commands::pro::handle_pro_command(subcommand);
        }
        None => {
            // Comportamiento por defecto (legacy)
            commands::monitor::start_monitor();
        }
    }
}
