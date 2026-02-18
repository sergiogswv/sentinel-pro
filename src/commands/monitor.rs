use crate::config::SentinelConfig;
use crate::rules::engine::RuleEngine;
use crate::stats::SentinelStats;
use crate::{ai, config, docs, files, git, kb, tests, ui};
use colored::*;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

pub fn start_monitor() {
    // Mostrar banner al inicio
    ui::mostrar_banner();

    let project_path = ui::seleccionar_proyecto();
    if !project_path.exists() {
        std::process::exit(1);
    }

    let config = Arc::new(ui::inicializar_sentinel(&project_path));
    let stats = Arc::new(Mutex::new(SentinelStats::cargar(&project_path)));

    // Motor de Reglas Pro
    let mut rule_engine = RuleEngine::new();
    let rules_path = project_path.join(".sentinel/rules.yaml");
    if rules_path.exists() {
        if let Err(e) = rule_engine.load_from_yaml(&rules_path) {
            println!("   ⚠️  Error al cargar rules.yaml: {}", e);
        } else {
            println!("   ✅ Reglas de arquitectura Pro cargadas.");
        }
    }
    let rule_engine = Arc::new(rule_engine);

    // --- Knowledge Base (v5.0.0 Pro) ---
    let mut kb_tx_opt = None;
    if let Some(features) = &config.features {
        if features.enable_knowledge_base {
            if let Some(kb_config) = &config.knowledge_base {
                if let Ok(db) = kb::VectorDB::new(&kb_config.vector_db_url) {
                    let db = Arc::new(db);
                    let manager =
                        Arc::new(kb::KBManager::new(Arc::clone(&db), &config, &project_path));

                    let (tx_kb, rx_kb) = tokio::sync::mpsc::channel(100);
                    kb_tx_opt = Some(tx_kb);

                    let manager_init = Arc::clone(&manager);
                    let kb_config_init = kb_config.clone();

                    thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            if let Err(e) = db.initialize_collection().await {
                                println!("   ⚠️  KB: Error al conectar con Qdrant: {}", e);
                                return;
                            }

                            if kb_config_init.index_on_start {
                                println!("   🧠 KB: Iniciando indexación del codebase...");
                                if let Err(e) = manager_init.initial_index().await {
                                    println!("   ⚠️  KB: Error en indexación inicial: {}", e);
                                } else {
                                    println!("   ✅ KB: Indexación inicial completada.");
                                }
                            }

                            manager_init.start_background_task(rx_kb).await;
                        });
                    });
                }
            }
        }
    }
    let kb_tx = kb_tx_opt;

    let esta_pausado = Arc::new(Mutex::new(false));
    let pausa_loop = Arc::clone(&esta_pausado);
    let (tx, rx) = mpsc::channel::<PathBuf>();
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>();
    let stdin_rx = Arc::new(Mutex::new(stdin_rx));
    let esperando_input = Arc::new(Mutex::new(false));

    // Hilo teclado
    let project_path_hilo = project_path.clone();
    let config_hilo = Arc::clone(&config);
    let stats_hilo = Arc::clone(&stats);
    let pausa_hilo = Arc::clone(&esta_pausado);
    let esperando_input_hilo = Arc::clone(&esperando_input);

    thread::spawn(move || {
        loop {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let cmd = input.trim().to_lowercase();
                if *esperando_input_hilo.lock().unwrap() {
                    let _ = stdin_tx.send(cmd);
                } else if cmd == "p" {
                    let mut p = pausa_hilo.lock().unwrap();
                    *p = !*p;
                    println!(
                        " ⌨️ SENTINEL: {}",
                        if *p {
                            "PAUSADO".yellow()
                        } else {
                            "ACTIVO".green()
                        }
                    );
                } else if cmd == "r" {
                    git::generar_reporte_diario(
                        &project_path_hilo,
                        &config_hilo,
                        Arc::clone(&stats_hilo),
                    );
                } else if cmd == "m" {
                    let s = stats_hilo.lock().unwrap();
                    println!(
                        "\n{}",
                        "📊 DASHBOARD DE RENDIMIENTO SENTINEL".bright_green().bold()
                    );
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!(
                        "🚫 Bugs Evitados:  {}",
                        s.bugs_criticos_evitados.to_string().red()
                    );
                    println!("💰 Costo Acumulado: ${:.4}", s.total_cost_usd);
                    println!("🎟️ Tokens Usados:   {}", s.total_tokens_used);
                    println!(
                        "⏳ Tiempo Ahorrado: {}h",
                        (s.tiempo_estimado_ahorrado_mins as f32 / 60.0)
                    );
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                } else if cmd == "l" {
                    print!(
                        "⚠️  ¿Limpiar todo el caché? Esto eliminará las respuestas guardadas (s/n): "
                    );
                    io::stdout().flush().unwrap();
                    let mut confirm = String::new();
                    if io::stdin().read_line(&mut confirm).is_ok()
                        && confirm.trim().to_lowercase() == "s"
                    {
                        if let Err(e) = ai::limpiar_cache(&project_path_hilo) {
                            println!("   ❌ Error al limpiar caché: {}", e);
                        }
                    } else {
                        println!("   ⏭️  Limpieza de caché cancelada.");
                    }
                } else if cmd == "h" || cmd == "help" {
                    ui::mostrar_ayuda(Some(&config_hilo));
                } else if cmd == "x" {
                    print!("⚠️  ¿Reiniciar configuración? (s/n): ");
                    io::stdout().flush().unwrap();
                    let mut confirm = String::new();
                    if io::stdin().read_line(&mut confirm).is_ok()
                        && confirm.trim().to_lowercase() == "s"
                    {
                        let _ = SentinelConfig::eliminar(&project_path_hilo);
                        std::process::exit(0);
                    }
                }
            }
        }
    });

    // Watcher
    let config_watcher = Arc::clone(&config);
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            if let EventKind::Modify(_) = event.kind {
                for path in event.paths {
                    if !config_watcher.debe_ignorar(&path) {
                        let _ = tx.send(path);
                    }
                }
            }
        }
    })
    .unwrap();
    watcher
        .watch(&project_path.join("src"), RecursiveMode::Recursive)
        .unwrap();

    let leer_respuesta = move || -> Option<String> {
        *esperando_input.lock().unwrap() = true;
        let res = stdin_rx
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(30))
            .ok();
        *esperando_input.lock().unwrap() = false;
        res
    };

    println!(
        "\n{} {}",
        format!("🛡️ Sentinel v{} activo en:", config::SENTINEL_VERSION)
            .green()
            .bold(),
        project_path.display()
    );

    // Mostrar ayuda de comandos al inicio
    ui::mostrar_ayuda(Some(&config));

    let mut ultimo_cambio: HashMap<PathBuf, Instant> = HashMap::new();
    while let Ok(changed_path) = rx.recv() {
        thread::sleep(std::time::Duration::from_millis(500));
        while rx.try_recv().is_ok() {}

        if *pausa_loop.lock().unwrap() {
            continue;
        }

        let ahora = Instant::now();
        if let Some(ultimo) = ultimo_cambio.get(&changed_path) {
            if ahora.duration_since(*ultimo) < std::time::Duration::from_secs(10) {
                continue;
            }
        }
        ultimo_cambio.insert(changed_path.clone(), ahora);

        let file_name = changed_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // --- Actualizar Knowledge Base (Incremental) ---
        if let Some(ref tx_kb) = kb_tx {
            let _ = tx_kb.try_send(kb::KBUpdate {
                file_path: changed_path.to_string_lossy().to_string(),
            });
        }

        let base_name = match files::detectar_archivo_padre(
            &changed_path,
            &project_path,
            &config.parent_patterns,
        ) {
            Some(padre) => {
                println!(
                    "   ℹ️  Archivo hijo detectado, usando tests del módulo: {}",
                    padre.yellow()
                );
                padre
            }
            None => file_name.split('.').next().unwrap().to_string(),
        };

        let test_rel_path =
            files::buscar_archivo_test(&base_name, &project_path, &config.test_patterns);

        if test_rel_path.is_none() {
            println!("\n🔔 CAMBIO EN: {}", file_name.cyan().bold());
            println!(
                "{}",
                "⚠️  No se encontraron tests para este archivo.".yellow()
            );
            print!("🔍 ¿Deseas que revise el código de todas formas? (s/n) [30s timeout]: ");
            io::stdout().flush().unwrap();

            match leer_respuesta() {
                Some(respuesta) if respuesta == "s" => {
                    if let Ok(codigo) = std::fs::read_to_string(&changed_path) {
                        // Validar Reglas Pro (Estáticas)
                        let spinner = ui::crear_progreso("   🔍 Validando reglas estáticas...");
                        let violaciones = rule_engine.validate_file(&changed_path, &codigo);
                        spinner.finish_and_clear();

                        if !violaciones.is_empty() {
                            println!(
                                "\n🚩 {}",
                                "VIOLACIONES DE ARQUITECTURA DETECTADAS:".bold().red()
                            );
                            for v in violaciones {
                                println!("   • [{}]: {}", v.rule_name.yellow(), v.message);
                            }
                        }

                        let spinner_ai = ui::crear_progreso("   🤖 Analizando arquitectura con IA...");
                        let resultado_analisis = ai::analizar_arquitectura(
                            &codigo,
                            &file_name,
                            Arc::clone(&stats),
                            &config,
                            &project_path,
                            &changed_path,
                        );
                        spinner_ai.finish_and_clear();

                        match resultado_analisis {
                            Ok(true) => {
                                println!(
                                    "   ✅ Código revisado. Sin tests, no se realizará commit automático."
                                );
                            }
                            Ok(false) => {
                                println!(
                                    "   ⚠️  Se encontraron problemas. Revisa las sugerencias."
                                );
                            }
                            Err(e) => {
                                println!("   ❌ Error al analizar: {}", e);
                            }
                        }
                    }
                }
                _ => {
                    println!("   ⏭️  Revisión omitida. Continuando monitoreo...");
                }
            }
            continue;
        }

        if let Some(test_path) = test_rel_path {
            println!("\n🔔 CAMBIO EN: {}", file_name.cyan().bold());

            if let Ok(codigo) = std::fs::read_to_string(&changed_path) {
                // Validar Reglas Pro (Estáticas)
                let spinner = ui::crear_progreso("   🔍 Validando reglas estáticas...");
                let violaciones = rule_engine.validate_file(&changed_path, &codigo);
                spinner.finish_and_clear();

                if !violaciones.is_empty() {
                    println!(
                        "\n🚩 {}",
                        "VIOLACIONES DE ARQUITECTURA DETECTADAS:".bold().red()
                    );
                    for v in violaciones {
                        let label = match v.level {
                            crate::rules::RuleLevel::Error => "ERROR".red().bold(),
                            crate::rules::RuleLevel::Warning => "WARN".yellow().bold(),
                            crate::rules::RuleLevel::Info => "INFO".blue().bold(),
                        };
                        println!("   • [{}][{}]: {}", label, v.rule_name.yellow(), v.message);
                    }
                }

                let spinner_ai = ui::crear_progreso("   🤖 Analizando arquitectura con IA...");
                let resultado_analisis = ai::analizar_arquitectura(
                    &codigo,
                    &file_name,
                    Arc::clone(&stats),
                    &config,
                    &project_path,
                    &changed_path,
                );
                spinner_ai.finish_and_clear();

                match resultado_analisis {
                    Ok(true) => {
                        if tests::ejecutar_tests(&test_path, &project_path).is_ok() {
                            let _ = docs::actualizar_documentacion(
                                &codigo,
                                &changed_path,
                                &config,
                                Arc::clone(&stats),
                                &project_path,
                            );
                            let msg = git::generar_mensaje_commit(
                                &codigo,
                                &file_name,
                                &config,
                                Arc::clone(&stats),
                                &project_path,
                            );
                            println!("\n🚀 Mensaje: {}", msg.bright_cyan().bold());
                            print!("📝 ¿Commit? (s/n): ");
                            io::stdout().flush().unwrap();
                            if let Some(r) = leer_respuesta() {
                                git::preguntar_commit(&project_path, &msg, &r);
                            }
                        } else {
                            print!("\n🔍 ¿Ayuda con test? (s/n): ");
                            io::stdout().flush().unwrap();
                            if leer_respuesta().as_deref() == Some("s") {
                                let _ = tests::pedir_ayuda_test(
                                    &codigo,
                                    &test_path,
                                    &config,
                                    Arc::clone(&stats),
                                    &project_path,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
