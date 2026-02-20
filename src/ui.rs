//! Módulo de interfaz de usuario
//!
//! Funciones relacionadas con la interacción con el usuario en la terminal.

use crate::ai;
use crate::config::SentinelConfig;
use colored::*;
use std::fs;

use crate::stats::SentinelStats;
use std::sync::{Arc, Mutex};
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

use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

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
    println!("{}", "  a       Ejecutar auditoría interactiva (Pro Audit)".dimmed());

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
    println!("  sentinel-pro pro analyze <file>   {}", "Análisis arquitectónico (Reviewer)".dimmed());
    println!("  sentinel-pro pro generate <file>  {}", "Generación de código (Coder)".dimmed());
    println!("  sentinel-pro pro refactor <file>  {}", "Refactorización (Refactor)".dimmed());
    println!("  sentinel-pro pro fix <file>       {}", "Corrección de bugs".dimmed());
    println!("  sentinel-pro pro test-all         {}", "Generación de tests (Tester)".dimmed());
    println!("  sentinel-pro pro audit <path>     {}", "Auditoría interactiva + Fixes".dimmed());
    println!("  sentinel-pro pro chat             {}", "Chat con el codebase".dimmed());
    println!("  sentinel-pro pro docs <dir>       {}", "Generar documentación".dimmed());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
    println!("{}", "🔮 COMANDOS AVANZADOS".bright_magenta().bold());
    println!("  sentinel-pro pro workflow <name>  {}", "Ejecutar workflows:".dimmed());
    println!("  {}", "                                  - fix-and-verify (Fix + Refactor + Test)".dimmed());
    println!("  {}", "                                  - review-security (Audit + Mitigate)".dimmed());
    println!("  sentinel-pro pro migrate <s, d>   {}", "Migrar código entre frameworks".dimmed());
    println!("  sentinel-pro pro review           {}", "Auditoría completa de proyecto".dimmed());
    println!("  sentinel-pro pro explain <file>   {}", "Explicación didáctica de código".dimmed());
    println!("  sentinel-pro pro optimize <file>  {}", "Sugerencias de optimización".dimmed());
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

        config.primary_model.url = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(format!("URL de la API para {}", provider_str))
            .default(env_url.unwrap_or(default_url))
            .interact_text()
            .unwrap_or_default();

        let api_key_prompt = if provider_str == "ollama" {
            "API Key (opcional para Ollama)"
        } else {
            "API Key"
        };

        config.primary_model.api_key = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
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
                let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
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
                println!("   ⚠️  No se pudieron obtener los modelos automáticamente: {}", e);
                config.primary_model.name = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Ingresa el nombre del modelo manualmente")
                    .default(default_model)
                    .interact_text()
                    .unwrap_or_default();
            }
            _ => {
                config.primary_model.name = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Ingresa el nombre del modelo manualmente")
                    .default(default_model)
                    .interact_text()
                    .unwrap_or_default();
            }
        }

        // 2. Configurar Modelo de Fallback (Opcional)
        let use_fallback = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("¿Deseas configurar un modelo de respaldo (fallback)?")
            .default(false)
            .interact()
            .unwrap_or(false);

        if use_fallback {
            let mut fb = crate::config::ModelConfig::default();
            
            let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
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

            println!("🔍 Conectando con {} para obtener modelos de fallback...", fb_provider);
            match ai::obtener_modelos_disponibles(&fb.provider, &fb.url, &fb.api_key) {
                Ok(mut models) if !models.is_empty() => {
                    models.sort();
                    let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Selecciona el modelo de fallback")
                        .items(&models)
                        .default(0)
                        .interact()
                        .unwrap_or(0);
                    fb.name = models[selection].clone();
                }
                _ => {
                    fb.name = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Ingresa el nombre del modelo de fallback manualmente")
                        .interact_text()
                        .unwrap_or_default();
                }
            }
            config.fallback_model = Some(fb);
        }

        // 3. Configurar Características Pro
        let enable_pro = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
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
    let deteccion = match ai::detectar_framework_con_ia(project_path, &config, stats_for_detection) {
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

/// Ayuda al usuario a configurar un framework de testing si no se detectó uno válido
fn ayudar_configurar_testing(
    config: &mut SentinelConfig,
    testing_info: ai::TestingFrameworkInfo,
) {
    if testing_info.status == ai::TestingStatus::Valid {
        return;
    }

    println!(
        "\n{}",
        "🧪 Configuración de Testing".bright_magenta().bold()
    );

    if testing_info.suggestions.is_empty() {
        println!("   💡 Sentinel no detectó un framework de testing configurado.");
        println!("   {} Tener tests siempre ayudará a mantener tu código sano y prevenir regresiones.", "👉".yellow());
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
        println!("\n🚀 Para instalar {}, ejecuta:", suggestion.framework.green());
        println!("   {}", suggestion.install_command.cyan().bold());

        let confirmar = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("¿Deseas registrar '{}' como el framework oficial del proyecto?", suggestion.framework))
            .default(true)
            .interact()
            .unwrap_or(false);

        if confirmar {
            config.testing_framework = Some(suggestion.framework.clone());
            config.testing_status = Some("valid".to_string());
            println!("   ✅ Framework {} registrado. No olvides ejecutar el comando de instalación.", suggestion.framework.green());
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
    println!("\n🧠 {}", "Configuración de Knowledge Base".bright_magenta().bold());
    println!("   Sentinel utiliza Qdrant para dar 'memoria' a la IA sobre todo tu proyecto.");

    let has_docker = verificar_docker();
    let mut options = vec![];

    if has_docker {
        options.push("Ejecutar vía Docker (Recomendado)");
    } else {
        options.push("Descargar ejecutable nativo (GitHub)");
    }
    
    options.push("Ignorar por ahora (Modo Offline)");

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("¿Cómo deseas configurar el motor vectorial Qdrant?")
        .items(&options)
        .default(0)
        .interact()
        .unwrap_or(options.len() - 1);

    if selection == options.len() - 1 {
        println!("\n⚠️  {}", "Modo Offline seleccionado.".yellow());
        println!("   Al omitir esto, la IA perderá el contexto global de otros archivos.");
        println!("   Podrás habilitarlo después configurando Qdrant en localhost:6334.");
        config.knowledge_base = Some(crate::config::KnowledgeBaseConfig {
            vector_db_url: "http://localhost:6333".to_string(),
            index_on_start: false,
        });
        return;
    }

    if has_docker && selection == 0 {
        println!("\n🚀 Copia y ejecuta este comando en otra terminal para iniciar Qdrant:");
        println!("   {}", "docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant".cyan().bold());
    } else {
        println!("\n📦 Instalación manual (Sin Docker):");
        println!("   1. Descarga el binario de: {}", "https://github.com/qdrant/qdrant/releases".underline());
        println!("   2. Dale permisos de ejecución: {}", "chmod +x qdrant".cyan());
        println!("   3. Ejecútalo: {}", "./qdrant".cyan());
    }

    config.knowledge_base = Some(crate::config::KnowledgeBaseConfig {
        vector_db_url: "http://localhost:6333".to_string(),
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
