pub mod monitor;
pub mod pro;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sentinel")]
#[command(about = "AI-Powered Code Monitor & Development Suite", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Inicia el modo monitor (comportamiento clásico)
    Monitor,
    /// Comandos avanzados de la versión Pro
    Pro {
        #[command(subcommand)]
        subcommand: ProCommands,
    },
}

#[derive(Subcommand)]
pub enum ProCommands {
    /// Análisis profundo e interactivo de un archivo
    Analyze {
        /// Archivo a analizar
        file: String,
    },
    /// Generación de código nuevo mediante IA
    Generate {
        /// Archivo a generar o donde insertar
        file: String,
    },
    /// Refactorización automática de código
    Refactor {
        /// Archivo a refactorizar
        file: String,
    },
    /// Corrección automática de bugs
    Fix {
        /// Archivo a corregir
        file: String,
    },
    /// Ejecución de tests con asistencia de IA
    TestAll,
    /// Explicación detallada de código
    Explain {
        /// Archivo a explicar
        file: String,
    },
    /// Chat interactivo con el código
    Chat,
    /// Review completo del proyecto
    Review,
    /// Generación de documentación técnica
    Docs {
        /// Directorio a documentar
        dir: String,
    },
    /// Migración entre frameworks
    Migrate {
        /// Origen
        src: String,
        /// Destino
        dst: String,
    },
    /// Optimización de performance
    Optimize {
        /// Archivo a optimizar
        file: String,
    },
}
