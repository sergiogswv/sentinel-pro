//! Módulo de ejecución de tests
//!
//! Se encarga de correr los tests con Jest y reportar resultados.

use crate::ai;
use colored::*;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::config::SentinelConfig;
use crate::stats::SentinelStats;

/// Ejecuta los tests de un archivo específico usando el comando configurado.
pub fn ejecutar_tests(test_path: &str, project_path: &Path, config: &SentinelConfig) -> Result<(), String> {
    println!("🧪 Ejecutando tests: {}", test_path.cyan());
    
    let cmd_str = if config.test_command.contains("{path}") {
        config.test_command.replace("{path}", test_path)
    } else {
        format!("{} {}", config.test_command, test_path)
    };

    println!("   Comando: {}", cmd_str.dimmed());
    println!(); 

    // Ejecutar vía shell para soportar argumentos en el comando
    #[cfg(windows)]
    let mut command = Command::new("pwsh");
    #[cfg(windows)]
    command.arg("-Command");

    #[cfg(not(windows))]
    let mut command = Command::new("sh");
    #[cfg(not(windows))]
    command.arg("-c");

    let status = command
        .arg(&cmd_str)
        .current_dir(project_path)
        .env("NODE_ENV", "test")
        .status()
        .map_err(|e| format!("Error al ejecutar comando de test: {}", e))?;

    println!();

    if status.success() {
        println!("{}", "   ✅ Tests pasados con éxito".green());
        Ok(())
    } else {
        println!("{}", "   ❌ Tests fallaron".red());
        Err("Tests fallidos. Revisa la salida anterior.".to_string())
    }
}

/// Captura el error de un test específico ejecutando Jest nuevamente.
pub fn capturar_error_test(test_path: &str, project_path: &Path) -> String {
    let output = Command::new("npx")
        .args(["jest", test_path, "--passWithNoTests", "--no-colors"])
        .current_dir(project_path)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("NODE_ENV", "test")
        .env("USER", std::env::var("USER").unwrap_or_default())
        .env("HOME", std::env::var("HOME").unwrap_or_default())
        .output();

    match output {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();

            // Combinar stdout y stderr para obtener todo el contexto del error
            if !stderr.is_empty() {
                format!("{}\n{}", stdout, stderr)
            } else {
                stdout
            }
        }
        Err(e) => format!("Error al capturar salida de Jest: {}", e),
    }
}

/// Pide ayuda a la IA cuando un test falla.
pub fn pedir_ayuda_test(
    codigo: &str,
    test_path: &str,
    config: &SentinelConfig,
    stats: Arc<Mutex<SentinelStats>>,
    project_path: &Path,
) -> anyhow::Result<()> {
    println!(
        "{}",
        "🔍 Analizando el error con IA...".magenta()
    );

    // Capturar el error ejecutando Jest nuevamente
    let error_jest = capturar_error_test(test_path, project_path);

    let prompt = format!(
        "Eres un experto en NestJS que da soluciones directas y accionables.\n\n\
        ERROR DEL TEST:\n{}\n\n\
        CÓDIGO:\n{}\n\n\
        INSTRUCCIONES:\n\
        1. Identifica el problema en UNA oración\n\
        2. Da la solución en formato de pasos numerados (máximo 3 pasos)\n\
        3. Incluye SOLO el código que debe cambiar (no repitas todo el archivo)\n\
        4. Sé ultra-conciso: máximo 150 palabras\n\n\
        Formato esperado:\n\
        🔴 PROBLEMA: [una línea]\n\
        ✅ SOLUCIÓN:\n\
        1. [paso específico]\n\
        2. [paso específico]\n\
        ```typescript\n[código a cambiar]\n```",
        error_jest, codigo
    );

    let respuesta =
        ai::consultar_ia_dinamico(prompt, ai::TaskType::Deep, config, stats, project_path)?;

    println!("\n💡 SOLUCIÓN SUGERIDA:\n{}", respuesta.yellow());
    Ok(())
}
