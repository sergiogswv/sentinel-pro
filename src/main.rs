//! # Sentinel Pro - AI-Powered Code Monitor & Development Suite
//!
//! Herramienta de monitoreo en tiempo real que vigila cambios en archivos TypeScript,
//! analiza el código con IA, ejecuta tests y gestiona commits automáticamente.
//! Ahora con capacidades extendidas en su versión Pro.

use clap::Parser;
use commands::{Cli, Commands};

// Módulos
mod ai;
mod commands;
mod config;
mod docs;
mod files;
mod git;
mod kb;
mod rules;
mod stats;
mod tests;
mod ui;

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
