pub mod ignore;
pub mod index;
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
    /// Gestiona la lista de hallazgos ignorados (falsos positivos)
    Ignore {
        /// Regla a ignorar (ej: DEAD_CODE, UNUSED_IMPORT)
        rule: Option<String>,
        /// Archivo donde aplicar el ignore (relativo al proyecto)
        file: Option<String>,
        /// Símbolo específico a ignorar (opcional)
        #[arg(long)]
        symbol: Option<String>,
        /// Listar todos los ignores activos
        #[arg(long)]
        list: bool,
        /// Eliminar todos los ignores para un archivo
        #[arg(long)]
        clear: Option<String>,
    },
    /// Gestión del índice de símbolos y call graph
    Index {
        /// Reconstruir el índice desde cero
        #[arg(long)]
        rebuild: bool,
        /// Mostrar estado del índice sin modificar nada
        #[arg(long)]
        check: bool,
    },
    /// Comandos avanzados de la versión Pro
    Pro {
        #[command(subcommand)]
        subcommand: ProCommands,
    },
}

#[derive(Subcommand)]
pub enum ProCommands {
    /// Capa 1: Análisis estático rápido (Dead code, unused imports, complexity)
    Check {
        /// Archivo o carpeta a revisar
        target: String,
        /// Formato de salida: text (default) o json (para CI/CD)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Análisis profundo (Capa 1 + Capa 2) e interactivo de un archivo
    Analyze {
        /// Archivo a analizar
        file: String,
    },
    /// Genera un reporte de calidad completo del proyecto
    Report {
        /// Formato del reporte (json o html)
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Divide un archivo grande en múltiples archivos por dominio
    Split {
        /// Archivo a dividir
        file: String,
    },
    /// Corrección automática de bugs
    Fix {
        /// Archivo a corregir
        file: String,
    },
    /// Ejecución de tests con asistencia de IA
    TestAll,
    /// Review completo del proyecto (Arquitectura y Coherencia)
    Review,
    /// Ejecutar un workflow definido
    Workflow {
        /// Nombre del workflow (ej: fix-and-verify)
        name: String,
        /// Archivo objetivo (opcional)
        file: Option<String>,
    },
    /// Auditoría interactiva con correcciones automáticas
    Audit {
        /// Archivo o carpeta a auditar
        target: String,
        /// Solo mostrar findings sin aplicar fixes (compatible con CI/CD)
        #[arg(long)]
        no_fix: bool,
        /// Formato de salida: text (default) o json
        #[arg(long, default_value = "text")]
        format: String,
        /// Máximo de archivos a auditar (default: 20). Usa un número mayor para proyectos grandes.
        #[arg(long, default_value = "20")]
        max_files: usize,
        /// Llamadas LLM en paralelo (default: 3, rango 1-10)
        #[arg(long, default_value = "3")]
        concurrency: usize,
    },
    /// Gestión de modelos de ML Local
    Ml {
        #[command(subcommand)]
        subcommand: MlCommands,
    },
    /// Limpia la caché de IA para un archivo, directorio o todo el proyecto
    CleanCache {
        /// Archivo, directorio a limpiar (opcional, por defecto todo el proyecto)
        target: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MlCommands {
    /// Descarga y prepara los modelos locales
    Download,
    /// Prueba la generación de embeddings
    Test {
        /// Texto a procesar
        text: String,
    },
}
