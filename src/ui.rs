//! Módulo de interfaz de usuario
//!
//! Funciones relacionadas con la interacción con el usuario en la terminal.

use crate::ai;
use crate::config::SentinelConfig;
use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Muestra el banner ASCII art de Sentinel al inicio del programa
pub fn mostrar_banner() {
    println!();
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        r"
   ███████╗███████╗███╗   ██╗████████╗██╗███╗   ██╗███████╗██╗     
   ██╔════╝██╔════╝████╗  ██║╚══██╔══╝██║████╗  ██║██╔════╝██║     
   ███████╗█████╗  ██╔██╗ ██║   ██║   ██║██╔██╗ ██║█████╗  ██║     
   ╚════██║██╔══╝  ██║╚██╗██║   ██║   ██║██║╚██╗██║██╔══╝  ██║     
   ███████║███████╗██║ ╚████║   ██║   ██║██║ ╚████║███████╗███████╗
   ╚══════╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝
"
        .bright_cyan()
        .bold()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════╝".bright_cyan()
    );
    println!();
    println!(
        "{}",
        "           🛡️  Sentinel Pro: AI-Powered Code Suite  🛡️"
            .bright_white()
            .bold()
    );
    println!(
        "{}",
        "           ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
}

use dialoguer::{theme::ColorfulTheme, Select};

/// Presenta un menú interactivo para seleccionar un proyecto del directorio padre.
///
/// Escanea el directorio padre (`../`) y muestra todos los subdirectorios como
/// opciones de proyectos. El usuario selecciona navegando con flechas.
///
/// # Retorna
///
/// PathBuf del proyecto seleccionado.
pub fn seleccionar_proyecto() -> PathBuf {
    println!("{}", "\n📂 Proyectos detectados:".bright_cyan().bold());

    let entries = match fs::read_dir("../") {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{}", "❌ Error al leer el directorio padre.".red().bold());
            eprintln!("   Error: {}", e);
            std::process::exit(1);
        }
    };

    let proyectos: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    if proyectos.is_empty() {
        eprintln!(
            "{}",
            "❌ No se encontraron proyectos en el directorio padre."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    let nombres: Vec<String> = proyectos
        .iter()
        .map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<nombre inválido>")
                .to_string()
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Selecciona un proyecto usando las flechas (↑/↓) y Enter")
        .default(0)
        .items(&nombres)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("{}", "❌ Selección cancelada.".red());
            std::process::exit(1);
        });

    proyectos[selection].clone()
}

/// Muestra la ayuda de comandos disponibles
pub fn mostrar_ayuda(config: Option<&SentinelConfig>) {
    println!(
        "\n{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
    println!("{}", "⌨️  COMANDOS DISPONIBLES".bright_cyan().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
    println!("{}", "  p       Pausar/Reanudar monitoreo".dimmed());
    println!(
        "{}",
        "  r       Generar reporte diario de productividad".dimmed()
    );
    println!(
        "{}",
        "  m       Ver dashboard de métricas (bugs, costos, tokens)".dimmed()
    );
    println!("{}", "  l       Limpiar caché de respuestas de IA".dimmed());

    // Mostrar comando T solo si hay testing configurado
    if let Some(cfg) = config {
        if cfg.testing_framework.is_some()
            && cfg.testing_status.as_ref().map_or(false, |s| s == "valid")
        {
            println!(
                "{}",
                "  t       Ver sugerencias de testing complementarias".dimmed()
            );
        }
    }

    println!(
        "{}",
        "  x       Reiniciar configuración desde cero".dimmed()
    );
    println!("{}", "  h/help  Mostrar esta ayuda".dimmed());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
    println!("{}", "🚀 COMANDOS PRO (Ejecutar en terminal)".bright_magenta().bold());
    println!("  sentinel pro analyze <file>   {}", "Análisis arquitectónico (Reviewer)".dimmed());
    println!("  sentinel pro generate <file>  {}", "Generación de código (Coder)".dimmed());
    println!("  sentinel pro refactor <file>  {}", "Refactorización (Refactor)".dimmed());
    println!("  sentinel pro fix <file>       {}", "Corrección de bugs".dimmed());
    println!("  sentinel pro test-all         {}", "Generación de tests (Tester)".dimmed());
    println!("  sentinel pro chat             {}", "Chat con el codebase".dimmed());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n".bright_cyan()
    );
}

pub fn inicializar_sentinel(project_path: &Path) -> SentinelConfig {
    let gestor = SentinelConfig::detectar_gestor(project_path);
    let nombre = project_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Intentar cargar configuración existente
    let mut config = if let Some(cfg) = SentinelConfig::load(project_path) {
        println!("{}", "🔄 Configuración existente encontrada".yellow());
        println!("   💾 Preservando API keys y configuraciones personalizadas...");
        cfg
    } else {
        // Nueva configuración - pedir API keys
        println!(
            "{}",
            "🚀 Configurando nuevo proyecto en Sentinel...".bright_cyan()
        );

        let mut config = SentinelConfig::default(
            nombre.clone(),
            gestor.clone(),
            "Detectando...".to_string(),
            vec!["Analizando proyecto...".to_string()],
            vec!["js".to_string(), "ts".to_string()],
            "typescript".to_string(),
            vec![],
            vec![],
        );

        println!(
            "\n{}",
            "🤖 Configuración de Modelos AI".bright_magenta().bold()
        );

        // 1. Seleccionar Proveedor
        println!("\n--- SELECCIÓN DE PROVEEDOR ---");
        println!("1. Anthropic (Claude)");
        println!("2. Google Gemini");
        println!("3. Ollama (IA Local)");
        println!("4. LM Studio / OpenAI Compat (IA Local)");
        print!("👉 Selecciona número [1]: ");
        io::stdout().flush().unwrap();
        let mut provider_sel = String::new();
        io::stdin().read_line(&mut provider_sel).unwrap();

        match provider_sel.trim() {
            "2" => {
                config.primary_model.provider = "gemini".to_string();
                config.primary_model.url = "https://generativelanguage.googleapis.com".to_string();
                println!("\n--- CONFIGURACIÓN GEMINI ---");
                print!("👉 API Key: ");
                io::stdout().flush().unwrap();
                let mut ak = String::new();
                io::stdin().read_line(&mut ak).unwrap();
                config.primary_model.api_key = ak.trim().to_string();

                if let Ok(modelos) = ai::listar_modelos_gemini(&config.primary_model.api_key) {
                    println!("{}", "📂 Modelos disponibles:".cyan());
                    for (i, m) in modelos.iter().enumerate() {
                        println!("{}. {}", i + 1, m);
                    }
                    print!("👉 Selecciona número: ");
                    io::stdout().flush().unwrap();
                    let mut sel = String::new();
                    io::stdin().read_line(&mut sel).unwrap();
                    if let Ok(idx) = sel.trim().parse::<usize>() {
                        if idx > 0 && idx <= modelos.len() {
                            config.primary_model.name = modelos[idx - 1].clone();
                        }
                    }
                }
            }
            "3" => {
                config.primary_model.provider = "ollama".to_string();
                config.primary_model.url = "http://localhost:11434".to_string();
                println!("\n--- CONFIGURACIÓN OLLAMA ---");
                print!("👉 URL [http://localhost:11434]: ");
                io::stdout().flush().unwrap();
                let mut u = String::new();
                io::stdin().read_line(&mut u).unwrap();
                if !u.trim().is_empty() {
                    config.primary_model.url = u.trim().to_string();
                }

                print!("👉 Nombre del modelo (ej: llama3, codestral): ");
                io::stdout().flush().unwrap();
                let mut nm = String::new();
                io::stdin().read_line(&mut nm).unwrap();
                config.primary_model.name = nm.trim().to_string();
            }
            "4" => {
                config.primary_model.provider = "openai".to_string();
                println!("\n--- CONFIGURACIÓN OPENAI / LM STUDIO ---");
                print!("👉 URL Base: ");
                io::stdout().flush().unwrap();
                let mut u = String::new();
                io::stdin().read_line(&mut u).unwrap();
                config.primary_model.url = u.trim().to_string();

                print!("👉 API Key [Opcional para local]: ");
                io::stdout().flush().unwrap();
                let mut ak = String::new();
                io::stdin().read_line(&mut ak).unwrap();
                config.primary_model.api_key = ak.trim().to_string();

                print!("👉 Nombre del modelo: ");
                io::stdout().flush().unwrap();
                let mut nm = String::new();
                io::stdin().read_line(&mut nm).unwrap();
                config.primary_model.name = nm.trim().to_string();
            }
            _ => {
                config.primary_model.provider = "anthropic".to_string();
                config.primary_model.url = "https://api.anthropic.com".to_string();
                println!("\n--- CONFIGURACIÓN ANTHROPIC ---");
                print!("👉 API Key: ");
                io::stdout().flush().unwrap();
                let mut ak = String::new();
                io::stdin().read_line(&mut ak).unwrap();
                config.primary_model.api_key = ak.trim().to_string();
                config.primary_model.name = "claude-3-5-sonnet-20241022".to_string();
            }
        }

        // 2. Configurar Modelo de Fallback (Opcional)
        println!("\n--- MODELO DE FALLBACK (Opcional) ---");
        print!("👉 ¿Configurar un modelo de respaldo por si falla el principal? (s/n): ");
        io::stdout().flush().unwrap();
        let mut use_fallback = String::new();
        io::stdin().read_line(&mut use_fallback).unwrap();

        if use_fallback.trim().to_lowercase() == "s" {
            let mut fb = crate::config::ModelConfig::default();
            print!("👉 Proveedor (anthropic, gemini, ollama, openai): ");
            io::stdout().flush().unwrap();
            let mut prov = String::new();
            io::stdin().read_line(&mut prov).unwrap();
            fb.provider = prov.trim().to_string();

            print!("👉 API Key: ");
            io::stdout().flush().unwrap();
            let mut ak = String::new();
            io::stdin().read_line(&mut ak).unwrap();
            fb.api_key = ak.trim().to_string();

            print!("👉 URL: ");
            io::stdout().flush().unwrap();
            let mut u = String::new();
            io::stdin().read_line(&mut u).unwrap();
            fb.url = u.trim().to_string();

            print!("👉 Nombre del modelo: ");
            io::stdout().flush().unwrap();
            let mut nm = String::new();
            io::stdin().read_line(&mut nm).unwrap();
            fb.name = nm.trim().to_string();

            config.fallback_model = Some(fb);
        }

        // 3. Configurar Características Pro
        println!("\n--- CARACTERÍSTICAS PRO ---");
        print!("👉 ¿Habilitar Machine Learning y Knowledge Base Local? (s/n) [s]: ");
        io::stdout().flush().unwrap();
        let mut enable_pro = String::new();
        io::stdin().read_line(&mut enable_pro).unwrap();

        if enable_pro.trim().to_lowercase() != "n" {
            config.features = Some(crate::config::FeaturesConfig {
                enable_ml: true,
                enable_agents: true,
                enable_knowledge_base: true,
            });

            config.ml = Some(crate::config::MlConfig {
                models_path: ".sentinel/models".to_string(),
                embeddings_model: "codebert".to_string(),
                bug_predictor_model: "bug-predictor-v1".to_string(),
            });

            config.knowledge_base = Some(crate::config::KnowledgeBaseConfig {
                vector_db_url: "http://localhost:6333".to_string(),
                index_on_start: true,
            });

            println!(
                "{}",
                "   ✨ Características Pro habilitadas por defecto.".green()
            );
        }

        config
    };

    // Guardar framework actual para comparar
    let framework_actual = config.framework.clone();
    let tiene_config_existente = SentinelConfig::load(project_path).is_some();

    // Detectar framework con IA (silenciosamente)
    let deteccion = match ai::detectar_framework_con_ia(project_path, &config) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "   ⚠️  Error al detectar framework: {}",
                e.to_string().yellow()
            );
            if tiene_config_existente {
                println!("   ℹ️  Manteniendo configuración actual");
                return config;
            }
            crate::config::FrameworkDetection {
                framework: "Generic".to_string(),
                rules: vec![
                    "Clean Code principles".to_string(),
                    "SOLID design patterns".to_string(),
                    "Code maintainability".to_string(),
                    "Comprehensive testing".to_string(),
                ],
                extensions: vec!["js".to_string(), "ts".to_string()],
                code_language: "typescript".to_string(),
                parent_patterns: vec![],
                test_patterns: vec!["{name}.test.ts".to_string(), "{name}.spec.ts".to_string()],
            }
        }
    };

    // Comparar con framework actual
    if tiene_config_existente && deteccion.framework == framework_actual {
        println!(
            "   ✓ Framework: {} (sin cambios)",
            deteccion.framework.green()
        );

        // Detectar frameworks de testing si no está ya configurado
        if config.testing_framework.is_none() || config.testing_status.is_none() {
            match ai::detectar_testing_framework(project_path, &config) {
                Ok(testing_info) => {
                    config.testing_framework = testing_info.testing_framework;
                    config.testing_status = Some(match testing_info.status {
                        ai::TestingStatus::Valid => "valid".to_string(),
                        ai::TestingStatus::Incomplete => "incomplete".to_string(),
                        ai::TestingStatus::Missing => "missing".to_string(),
                    });
                    let _ = config.save(project_path);
                }
                Err(e) => {
                    println!(
                        "   ⚠️  Error al detectar testing framework: {}",
                        e.to_string().yellow()
                    );
                    println!("   ℹ️  Continuando sin detección de testing");
                }
            }
        } else {
            let default_fw = "N/A".to_string();
            let default_status = "unknown".to_string();
            let testing_fw = config.testing_framework.as_ref().unwrap_or(&default_fw);
            let testing_status = config.testing_status.as_ref().unwrap_or(&default_status);

            println!(
                "   ✓ Testing: {} ({})",
                testing_fw.green(),
                testing_status.cyan()
            );
        }

        return config;
    }

    // Hay cambios o es primera vez - mostrar y confirmar
    println!("\n{}", "📋 Framework Detectado:".bright_yellow().bold());
    println!("   Framework: {}", deteccion.framework.bright_green());
    println!("   Lenguaje: {}", deteccion.code_language.bright_green());
    println!(
        "   Extensiones: {}",
        deteccion.extensions.join(", ").bright_green()
    );

    if tiene_config_existente {
        println!(
            "\n   ⚠️  Cambio detectado: {} → {}",
            framework_actual.yellow(),
            deteccion.framework.green()
        );
    }

    print!("\n👉 ¿Es correcto? (s/n): ");
    io::stdout().flush().unwrap();
    let mut confirmacion = String::new();
    io::stdin().read_line(&mut confirmacion).unwrap();

    if confirmacion.trim().to_lowercase() != "s" {
        println!("   ℹ️  Manteniendo configuración actual");
        return config;
    }

    // Actualizar config con framework, reglas, extensiones, lenguaje y patrones detectados
    config.framework = deteccion.framework;
    config.architecture_rules = deteccion.rules;
    config.file_extensions = deteccion.extensions;
    config.code_language = deteccion.code_language;
    config.parent_patterns = deteccion.parent_patterns;
    config.test_patterns = deteccion.test_patterns;

    // Detectar frameworks de testing
    match ai::detectar_testing_framework(project_path, &config) {
        Ok(testing_info) => {
            config.testing_framework = testing_info.testing_framework.clone();
            config.testing_status = Some(match testing_info.status {
                ai::TestingStatus::Valid => "valid".to_string(),
                ai::TestingStatus::Incomplete => "incomplete".to_string(),
                ai::TestingStatus::Missing => "missing".to_string(),
            });
        }
        Err(e) => {
            println!(
                "   ⚠️  Error al detectar testing framework: {}",
                e.to_string().yellow()
            );
            println!("   ℹ️  Continuando sin detección de testing");
        }
    }

    let _ = config.save(project_path);
    println!("{}", "✅ Configuración actualizada.".green());
    config
}

/// Helper para mostrar una barra de progreso genérica
pub fn crear_progreso(mensaje: &str) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(mensaje.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}
