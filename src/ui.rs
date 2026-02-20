//! Módulo de interfaz de usuario
//!
//! Funciones relacionadas con la interacción con el usuario en la terminal.

use crate::ai;
use crate::config::SentinelConfig;
use colored::*;
use std::fs;

use crate::stats::SentinelStats;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

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
    println!(
        "{}",
        "  a       Ejecutar auditoría interactiva (Pro Audit)".dimmed()
    );
    println!(
        "{}",
        "  k       Re-intentar conexión Knowledge Base (Qdrant)".dimmed()
    );

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
    println!(
        "{}",
        "🚀 COMANDOS PRO (Ejecutar en terminal)"
            .bright_magenta()
            .bold()
    );
    println!(
        "  sentinel pro analyze <file>   {}",
        "Análisis arquitectónico (Reviewer)".dimmed()
    );
    println!(
        "  sentinel pro generate <file>  {}",
        "Generación de código (Coder)".dimmed()
    );
    println!(
        "  sentinel pro refactor <file>  {}",
        "Refactorización (Refactor)".dimmed()
    );
    println!(
        "  sentinel pro fix <file>       {}",
        "Corrección de bugs".dimmed()
    );
    println!(
        "  sentinel pro test-all         {}",
        "Generación de tests (Tester)".dimmed()
    );
    println!(
        "  sentinel pro audit <path>     {}",
        "Auditoría interactiva + Fixes".dimmed()
    );
    println!(
        "  sentinel pro chat             {}",
        "Chat con el codebase".dimmed()
    );
    println!(
        "  sentinel pro docs <dir>       {}",
        "Generar documentación".dimmed()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
    println!("{}", "🔮 COMANDOS AVANZADOS".bright_magenta().bold());
    println!(
        "  sentinel pro workflow <name>  {}",
        "Ejecutar workflows:".dimmed()
    );
    println!(
        "  {}",
        "                                  - fix-and-verify (Fix + Refactor + Test)".dimmed()
    );
    println!(
        "  {}",
        "                                  - review-security (Audit + Mitigate)".dimmed()
    );
    println!(
        "  sentinel pro migrate <s, d>   {}",
        "Migrar código entre frameworks".dimmed()
    );
    println!(
        "  sentinel pro review           {}",
        "Auditoría completa de proyecto".dimmed()
    );
    println!(
        "  sentinel pro explain <file>   {}",
        "Explicación didáctica de código".dimmed()
    );
    println!(
        "  sentinel pro optimize <file>  {}",
        "Sugerencias de optimización".dimmed()
    );
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
        let _ = SentinelConfig::save_active_project(project_path);
        cfg
    } else {
        // Nueva configuración - pedir API keys
        println!(
            "{}",
            "🚀 Configurando nuevo proyecto en Sentinel...".bright_cyan()
        );

        let mut config = SentinelConfig::create_default(
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
        let providers = vec![
            "Claude (Anthropic)",
            "Gemini (Google)",
            "OpenAI",
            "Groq",
            "Ollama (Local)",
            "Kimi (Moonshot)",
            "DeepSeek",
        ];
        let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Selecciona un proveedor de IA principal")
            .items(&providers)
            .default(0)
            .interact()
            .unwrap_or(0);

        let provider_str = match selection {
            0 => "anthropic",
            1 => "gemini",
            2 => "openai",
            3 => "groq",
            4 => "ollama",
            5 => "kimi",
            6 => "deepseek",
            _ => "anthropic",
        };

        config.primary_model.provider = provider_str.to_string();

        let default_url = match provider_str {
            "anthropic" => "https://api.anthropic.com".to_string(),
            "gemini" => "https://generativelanguage.googleapis.com".to_string(),
            "openai" => "https://api.openai.com/v1".to_string(),
            "groq" => "https://api.groq.com/openai/v1".to_string(),
            "ollama" => "http://localhost:11434".to_string(),
            "kimi" => "https://api.moonshot.ai/v1".to_string(),
            "deepseek" => "https://api.deepseek.com".to_string(),
            _ => "".to_string(),
        };

        let env_url = std::env::var(format!("{}_BASE_URL", provider_str.to_uppercase())).ok();
        let env_key = std::env::var(format!("{}_API_KEY", provider_str.to_uppercase())).ok();

        config.primary_model.url =
            dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(format!("URL de la API para {}", provider_str))
                .default(env_url.unwrap_or(default_url))
                .interact_text()
                .unwrap_or_default();

        let api_key_prompt = if provider_str == "ollama" {
            "API Key (opcional para Ollama)"
        } else {
            "API Key"
        };

        config.primary_model.api_key =
            dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(format!("{} para {}", api_key_prompt, provider_str))
                .allow_empty(provider_str == "ollama")
                .default(env_key.unwrap_or_else(|| String::new()))
                .interact_text()
                .unwrap_or_default();

        let default_model = match provider_str {
            "anthropic" => "claude-3-5-sonnet-20241022".to_string(),
            "gemini" => "gemini-2.0-flash".to_string(),
            "openai" => "gpt-4o".to_string(),
            "groq" => "llama3-70b-8192".to_string(),
            "ollama" => "llama3".to_string(),
            "kimi" => "moonshot-v1-8k".to_string(),
            "deepseek" => "deepseek-coder".to_string(),
            _ => "".to_string(),
        };

        // 2. Intentar obtener modelos disponibles dinámicamente
        println!("🔍 Conectando con {} para obtener modelos...", provider_str);
        match ai::obtener_modelos_disponibles(
            &provider_str,
            &config.primary_model.url,
            &config.primary_model.api_key,
        ) {
            Ok(mut models) if !models.is_empty() => {
                models.sort();
                let selection =
                    dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt(format!("Selecciona el modelo para {}", provider_str))
                        .items(&models)
                        .default(0)
                        .interact()
                        .unwrap_or(0);

                if selection < models.len() {
                    config.primary_model.name = models[selection].clone();
                } else {
                    config.primary_model.name = default_model;
                }
            }
            Err(e) => {
                println!(
                    "   ⚠️  No se pudieron obtener los modelos automáticamente: {}",
                    e
                );
                config.primary_model.name =
                    dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Ingresa el nombre del modelo manualmente")
                        .default(default_model)
                        .interact_text()
                        .unwrap_or_default();
            }
            _ => {
                config.primary_model.name =
                    dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Ingresa el nombre del modelo manualmente")
                        .default(default_model)
                        .interact_text()
                        .unwrap_or_default();
            }
        }

        // 2. Configurar Modelo de Fallback (Opcional)
        let use_fallback =
            dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("¿Deseas configurar un modelo de respaldo (fallback)?")
                .default(false)
                .interact()
                .unwrap_or(false);

        if use_fallback {
            let mut fb = crate::config::ModelConfig::default();

            let selection =
                dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Selecciona un proveedor de IA para fallback")
                    .items(&providers)
                    .default(0)
                    .interact()
                    .unwrap_or(0);

            let fb_provider = match selection {
                0 => "anthropic",
                1 => "gemini",
                2 => "openai",
                3 => "groq",
                4 => "ollama",
                5 => "kimi",
                6 => "deepseek",
                _ => "anthropic",
            };
            fb.provider = fb_provider.to_string();

            let fb_default_url = match fb_provider {
                "anthropic" => "https://api.anthropic.com".to_string(),
                "gemini" => "https://generativelanguage.googleapis.com".to_string(),
                "openai" => "https://api.openai.com/v1".to_string(),
                "groq" => "https://api.groq.com/openai/v1".to_string(),
                "ollama" => "http://localhost:11434".to_string(),
                "kimi" => "https://api.moonshot.ai/v1".to_string(),
                "deepseek" => "https://api.deepseek.com".to_string(),
                _ => "".to_string(),
            };

            fb.url = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(format!("URL de la API para fallback ({})", fb_provider))
                .default(fb_default_url)
                .interact_text()
                .unwrap_or_default();

            fb.api_key = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(format!("API Key para fallback ({})", fb_provider))
                .allow_empty(fb_provider == "ollama")
                .interact_text()
                .unwrap_or_default();

            println!(
                "🔍 Conectando con {} para obtener modelos de fallback...",
                fb_provider
            );
            match ai::obtener_modelos_disponibles(&fb.provider, &fb.url, &fb.api_key) {
                Ok(mut models) if !models.is_empty() => {
                    models.sort();
                    let selection =
                        dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt("Selecciona el modelo de fallback")
                            .items(&models)
                            .default(0)
                            .interact()
                            .unwrap_or(0);
                    fb.name = models[selection].clone();
                }
                _ => {
                    fb.name =
                        dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt("Ingresa el nombre del modelo de fallback manualmente")
                            .interact_text()
                            .unwrap_or_default();
                }
            }
            config.fallback_model = Some(fb);
        }

        // 3. Configurar Características Pro
        let enable_pro =
            dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("¿Habilitar Machine Learning y Knowledge Base Local?")
                .default(true)
                .interact()
                .unwrap_or(true);

        if enable_pro {
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

            // Configurar KB con asesoramiento
            configurar_knowledge_base(&mut config);
        }

        config
    };

    // Guardar framework actual para comparar
    let framework_actual = config.framework.clone();
    let tiene_config_existente = SentinelConfig::load(project_path).is_some();

    // Detectar framework con IA (silenciosamente)
    let stats_for_detection = Arc::new(Mutex::new(SentinelStats::cargar(project_path)));
    let deteccion = match ai::detectar_framework_con_ia(project_path, &config, stats_for_detection)
    {
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

    // --- Verificación Pro-activa de Qdrant (v5.0 Pro) ---
    verificar_qdrant_proactivo(&mut config, project_path);

    // Si ya existe configuración y el framework no ha cambiado, no molestamos con re-detección
    // EXCEPTO para testing si no está configurado.
    if tiene_config_existente && deteccion.framework == framework_actual {
        println!(
            "   ✓ Framework: {} (sin cambios)",
            deteccion.framework.green()
        );

        // Detectar frameworks de testing si no está ya configurado
        if config.testing_framework.is_none() || config.testing_status.is_none() {
            if let Ok(testing_info) = ai::detectar_testing_framework(project_path, &config) {
                config.testing_framework = testing_info.testing_framework.clone();
                config.testing_status = Some(match testing_info.status {
                    ai::TestingStatus::Valid => "valid".to_string(),
                    ai::TestingStatus::Incomplete => "incomplete".to_string(),
                    ai::TestingStatus::Missing => "missing".to_string(),
                });

                ayudar_configurar_testing(&mut config, testing_info);
                let _ = config.save(project_path);
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

    let confirms = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("¿Es correcto el framework detectado?")
        .default(true)
        .interact()
        .unwrap_or(true);

    if !confirms {
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
    if let Ok(testing_info) = ai::detectar_testing_framework(project_path, &config) {
        config.testing_framework = testing_info.testing_framework.clone();
        config.testing_status = Some(match testing_info.status {
            ai::TestingStatus::Valid => "valid".to_string(),
            ai::TestingStatus::Incomplete => "incomplete".to_string(),
            ai::TestingStatus::Missing => "missing".to_string(),
        });
        ayudar_configurar_testing(&mut config, testing_info);
    } else {
        println!("   ℹ️  Continuando sin detección de testing");
    }

    let _ = config.save(project_path);
    println!("{}", "✅ Configuración actualizada.".green());

    config
}

/// Verifica si Qdrant está en ejecución y ofrece iniciarlo si se detecta localmente.
fn verificar_qdrant_proactivo(config: &mut SentinelConfig, project_path: &Path) {
    // --- Verificación Pro-activa de Qdrant (v5.0 Pro) ---
    // Se ejecuta siempre, incluso si el framework no cambió, para asegurar que el motor vectorial esté listo.
    if let Some(ref mut kb) = config.knowledge_base {
        if kb.index_on_start {
            // 1. Intentar conectar a la URL configurada
            let mut current_url_valid = false;
            let target = kb
                .vector_db_url
                .replace("http://", "")
                .replace("https://", "");
            if let Some((host, port_str)) = target.split_once(':') {
                let port = port_str.parse::<u16>().unwrap_or(6334);
                let actual_host = if host == "localhost" {
                    "127.0.0.1"
                } else {
                    host
                };

                if let Ok(addr) =
                    format!("{}:{}", actual_host, port).parse::<std::net::SocketAddr>()
                {
                    current_url_valid =
                        std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300))
                            .is_ok();
                }
            }

            let mut is_running = current_url_valid;

            // 2. Si falló, intentar con el "Heal" (127.0.0.1:6334)
            if !is_running
                && (kb.vector_db_url.contains("localhost") || kb.vector_db_url.contains("6333"))
            {
                let healed_addr: std::net::SocketAddr = "127.0.0.1:6334".parse().unwrap();
                if std::net::TcpStream::connect_timeout(&healed_addr, Duration::from_millis(300))
                    .is_ok()
                {
                    println!(
                        "   🔧 {} Detectado Qdrant en 127.0.0.1:6334. Actualizando configuración...",
                        "Auto-Fix:".cyan()
                    );
                    kb.vector_db_url = "http://127.0.0.1:6334".to_string();
                    is_running = true;
                    // Persistir el cambio inmediatamente
                    let _ = config.save(project_path);
                }
            }

            if !is_running {
                let sentinel_home = SentinelConfig::get_sentinel_home();
                let qdrant_bin = if cfg!(windows) {
                    sentinel_home.join("qdrant").join("qdrant.exe")
                } else {
                    sentinel_home.join("qdrant").join("qdrant")
                };

                if qdrant_bin.exists() {
                    println!(
                        "\n🧠 {}",
                        "Knowledge Base: Qdrant no está en ejecución.".yellow()
                    );
                    let iniciar = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Se detectó una instalación local de Qdrant. ¿Deseas iniciarla en segundo plano?")
                        .default(true)
                        .interact()
                        .unwrap_or(false);

                    if iniciar {
                        println!("   🚀 Iniciando Qdrant...");
                        let success = if cfg!(windows) {
                            // En Windows, a veces Start-Process es caprichoso.
                            // Intentamos simplemente ejecutarlo con spawn para que herede el entorno.
                            Command::new("powershell")
                                .args([
                                    "-NoProfile",
                                    "-Command",
                                    &format!(
                                        "Start-Job -ScriptBlock {{ & '{}' }}",
                                        qdrant_bin.display()
                                    ),
                                ])
                                .status()
                                .is_ok()
                        } else {
                            Command::new("sh")
                                .arg("-c")
                                .arg(format!("'{}' > /dev/null 2>&1 &", qdrant_bin.display()))
                                .status()
                                .is_ok()
                        };

                        if success {
                            println!("   ✅ Comando de inicio enviado.");
                            println!("   ℹ️  Si no conecta, puedes iniciarlo manualmente con:");
                            println!("      {}", format!("'{}'", qdrant_bin.display()).cyan());
                        }
                    }
                }
            }
        }
    }
}

/// Ayuda al usuario a configurar un framework de testing si no se detectó uno válido
fn ayudar_configurar_testing(config: &mut SentinelConfig, testing_info: ai::TestingFrameworkInfo) {
    if testing_info.status == ai::TestingStatus::Valid {
        return;
    }

    println!(
        "\n{}",
        "🧪 Configuración de Testing".bright_magenta().bold()
    );

    if testing_info.suggestions.is_empty() {
        println!("   💡 Sentinel no detectó un framework de testing configurado.");
        println!(
            "   {} Tener tests siempre ayudará a mantener tu código sano y prevenir regresiones.",
            "👉".yellow()
        );
        return;
    }

    let mut options: Vec<String> = testing_info
        .suggestions
        .iter()
        .map(|s| format!("{} - {}", s.framework.bold(), s.reason))
        .collect();

    options.push("Configurar manualmente".to_string());
    options.push("Omitir por ahora".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("¿Deseas configurar un framework de testing recomendado?")
        .items(&options)
        .default(0)
        .interact()
        .unwrap_or(options.len() - 1);

    if selection < testing_info.suggestions.len() {
        let suggestion = &testing_info.suggestions[selection];
        println!(
            "\n🚀 Para instalar {}, ejecuta:",
            suggestion.framework.green()
        );
        println!("   {}", suggestion.install_command.cyan().bold());

        let confirmar = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "¿Deseas registrar '{}' como el framework oficial del proyecto?",
                suggestion.framework
            ))
            .default(true)
            .interact()
            .unwrap_or(false);

        if confirmar {
            config.testing_framework = Some(suggestion.framework.clone());
            config.testing_status = Some("valid".to_string());
            println!(
                "   ✅ Framework {} registrado.",
                suggestion.framework.green()
            );

            let auto_install = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("¿Deseas intentar ejecutar el comando de instalación automáticamente?")
                .default(true)
                .interact()
                .unwrap_or(false);

            if auto_install {
                // Verificar si el comando de instalación parece ser un comando real o solo texto descriptivo
                let is_real_command = !suggestion
                    .install_command
                    .to_lowercase()
                    .contains("no requiere")
                    && !suggestion.install_command.to_lowercase().contains("manual")
                    && (suggestion.install_command.contains("npm")
                        || suggestion.install_command.contains("yarn")
                        || suggestion.install_command.contains("pnpm")
                        || suggestion.install_command.contains("pip")
                        || suggestion.install_command.contains("cargo")
                        || suggestion.install_command.contains("go"));

                if is_real_command {
                    println!("   🚀 Ejecutando: {}", suggestion.install_command.cyan());

                    let parts: Vec<&str> = suggestion.install_command.split_whitespace().collect();
                    if !parts.is_empty() {
                        let mut cmd = if cfg!(windows) {
                            let mut c = Command::new("powershell");
                            c.args(["-NoProfile", "-Command", &suggestion.install_command]);
                            c
                        } else {
                            let mut c = Command::new("sh");
                            c.args(["-c", &suggestion.install_command]);
                            c
                        };

                        match cmd.status() {
                            Ok(status) if status.success() => {
                                println!("   ✅ Instalación completada con éxito.");
                            }
                            _ => {
                                println!(
                                    "   ⚠️  La instalación falló o fue cancelada. Es posible que debas ejecutarla manualmente."
                                );
                            }
                        }
                    }
                } else {
                    println!("   💡 {}", "Este framework requiere configuración manual o no tiene un comando de instalación directo.".yellow());
                    println!("   📝 Instrucción: {}", suggestion.install_command.cyan());
                }
            } else {
                println!("   ℹ️  No olvides ejecutar el comando manualmente antes de empezar.");
            }
        }
    } else if selection == options.len() - 1 {
        // Omitir
        println!(
            "\n{}",
            "💡 Tener tests siempre ayudará a mantener tu código sano y prevenir regresiones."
                .bright_yellow()
        );
    } else if selection == options.len() - 2 {
        // Manual
        let manual_fw: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Nombre del framework de testing (ej: Jest, Pytest)")
            .interact_text()
            .unwrap_or_default();

        if !manual_fw.is_empty() {
            config.testing_framework = Some(manual_fw.clone());
            config.testing_status = Some("valid".to_string());
            println!("   ✅ Framework {} registrado.", manual_fw.green());
        }
    }
}

/// Verifica si Docker está instalado en el sistema
fn verificar_docker() -> bool {
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .is_ok()
}

/// Asesor de configuración para la Knowledge Base (Qdrant)
fn configurar_knowledge_base(config: &mut SentinelConfig) {
    println!(
        "\n🧠 {}",
        "Configuración de Knowledge Base".bright_magenta().bold()
    );
    println!("   Sentinel utiliza Qdrant para dar 'memoria' a la IA sobre todo tu proyecto.");

    // 1. Verificar si ya está corriendo
    if std::net::TcpStream::connect_timeout(
        &"127.0.0.1:6334".parse().unwrap(),
        Duration::from_millis(500),
    )
    .is_ok()
    {
        println!("   ✅ Qdrant ya está en ejecución y respondiendo.");
        config.knowledge_base = Some(crate::config::KnowledgeBaseConfig {
            vector_db_url: "http://127.0.0.1:6334".to_string(),
            index_on_start: true,
        });
        return;
    }

    // 2. Verificar si existe instalación en el home de Sentinel
    let sentinel_home = SentinelConfig::get_sentinel_home();
    let qdrant_bin = if cfg!(windows) {
        sentinel_home.join("qdrant").join("qdrant.exe")
    } else {
        sentinel_home.join("qdrant").join("qdrant")
    };

    let has_docker = verificar_docker();
    let bin_exists = qdrant_bin.exists();

    let mut options = vec![];
    if bin_exists {
        options.push("Iniciar Qdrant local (Detectado en .sentinel-pro)");
    }
    if has_docker {
        options.push("Ejecutar vía Docker");
    }
    options.push("Descargar/Reinstalar desde GitHub");
    options.push("Ignorar por ahora (Modo Offline)");

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("¿Cómo deseas configurar el motor vectorial Qdrant?")
        .items(&options)
        .default(0)
        .interact()
        .unwrap_or(options.len() - 1);

    let selected_text = &options[selection];

    if selected_text.contains("Ignorar") {
        println!("\n⚠️  {}", "Modo Offline seleccionado.".yellow());
        config.knowledge_base = Some(crate::config::KnowledgeBaseConfig {
            vector_db_url: "http://127.0.0.1:6334".to_string(),
            index_on_start: false,
        });
        return;
    }

    if selected_text.contains("Iniciar Qdrant local") {
        println!("   🚀 Iniciando Qdrant en segundo plano...");
        let success = if cfg!(windows) {
            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!("Start-Job -ScriptBlock {{ & '{}' }}", qdrant_bin.display()),
                ])
                .status()
                .is_ok()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(format!("'{}' > /dev/null 2>&1 &", qdrant_bin.display()))
                .status()
                .is_ok()
        };

        if success {
            println!("   ✅ Comando de inicio enviado.");
            println!("   ℹ️  Puede tardar unos segundos en estar listo.");
            println!(
                "   ℹ️  Comando manual: {}",
                format!("'{}'", qdrant_bin.display()).cyan()
            );
        } else {
            println!("   ❌ Error al enviar comando de inicio.");
        }
    } else if selected_text.contains("Docker") {
        println!("\n🚀 Ejecutando Qdrant vía Docker en segundo plano...");
        let status = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                "sentinel-qdrant",
                "-p",
                "6333:6333",
                "-p",
                "6334:6334",
                "qdrant/qdrant",
            ])
            .status();

        if let Ok(s) = status {
            if s.success() {
                println!("   ✅ Contenedor Docker iniciado.");
            } else {
                println!(
                    "   ⚠️  El comando Docker falló. Asegúrate de que el puerto 6333 esté libre."
                );
            }
        }
    } else if selected_text.contains("GitHub") {
        println!("\n📦 Instalación manual (Sin Docker):");
        println!(
            "   1. Descarga el binario de: {}",
            "https://github.com/qdrant/qdrant/releases".underline()
        );
        println!("   2. Ejecútalo antes de iniciar Sentinel.");
    }

    config.knowledge_base = Some(crate::config::KnowledgeBaseConfig {
        vector_db_url: "http://127.0.0.1:6334".to_string(),
        index_on_start: true,
    });

    println!("\n✅ Configuración de Knowledge Base guardada.");
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
