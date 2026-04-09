use std::path::Path;
use crate::config::SentinelConfig;
use crate::rules::engine::RuleEngine;
use crate::stats::SentinelStats;
use crate::{ai, config, docs, files, git, index, ui, business_logic_guard};
use colored::*;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::Local;

pub static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);

fn report_sync(event_type: &str, severity: &str, mut payload: HashMap<String, serde_json::Value>) {
    let agent_config = crate::agent_config::AgentConfig::from_env();
    if !agent_config.report_enabled { return; }
    
    // Inyectar timestamp si no existe
    if !payload.contains_key("timestamp") {
        payload.insert("timestamp".to_string(), serde_json::Value::Number(serde_json::Number::from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())));
    }

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().ok();
    if let Some(r) = rt {
        let _ = r.block_on(crate::agent_reporter::report_event(&agent_config, event_type, severity, payload));
    }
}

pub(crate) fn write_pid_file(pid_path: &Path, pid: u32) -> anyhow::Result<()> {
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(pid_path, pid.to_string())?;
    Ok(())
}

pub(crate) fn read_pid_file(pid_path: &Path) -> Option<u32> {
    std::fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
}

pub(crate) fn is_process_alive(_pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal;
        use nix::unistd::Pid;
        // PID 0 means the whole process group; PIDs > i32::MAX wrap to negative
        // values (e.g. -1) — both have special semantics in kill(2). Reject them.
        if pid == 0 || pid > i32::MAX as u32 {
            return false;
        }
        // kill(pid, 0) checks process existence without sending a signal
        signal::kill(Pid::from_raw(pid as i32), None).is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn handle_daemon(project_root: &Path) -> anyhow::Result<()> {
    let pid_path = project_root.join(".sentinel/monitor.pid");
    if pid_path.exists() {
        if let Some(pid) = read_pid_file(&pid_path) {
            if is_process_alive(pid) {
                println!("⚠️  sentinel monitor ya está corriendo (PID {}). Usa --stop para detenerlo.", pid);
                return Ok(());
            }
        }
        // Stale PID file (process dead): write_pid_file below will overwrite it.
    }

    // Crear archivos de log
    let sentinel_dir = project_root.join(".sentinel");
    std::fs::create_dir_all(&sentinel_dir)?;
    let stdout_log = std::fs::File::create(sentinel_dir.join("monitor.stdout.log"))?;
    let stderr_log = std::fs::File::create(sentinel_dir.join("monitor.stderr.log"))?;

    let exe = std::env::current_exe()?;
    let mut command = std::process::Command::new(exe);
    command
        .arg("monitor")
        .stdin(std::process::Stdio::null())
        .stdout(stdout_log)
        .stderr(stderr_log);

    // Detach from the controlling terminal on Unix: create a new session so
    // the daemon does not receive SIGHUP when the parent terminal closes.
    // SAFETY: setsid(2) is async-signal-safe and has no preconditions.
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            nix::unistd::setsid()
                .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
            Ok(())
        });
    }

    let child = command.spawn()?;
    let pid = child.id();
    // Forget the Child handle so it is not waited on drop — the daemon
    // runs independently after this process exits.
    std::mem::forget(child);

    write_pid_file(&pid_path, pid)?;
    println!("✅ sentinel monitor iniciado en background (PID {})", pid);
    println!("   Detener: sentinel monitor --stop");
    Ok(())
}

pub fn handle_stop(project_root: &Path) -> anyhow::Result<()> {
    let pid_path = project_root.join(".sentinel/monitor.pid");
    match read_pid_file(&pid_path) {
        None => {
            println!("ℹ️  No hay PID file. sentinel monitor no está corriendo como daemon.");
        }
        Some(pid) => {
            // Guard: PIDs outside i32 range cannot be valid process IDs.
            if pid > i32::MAX as u32 {
                eprintln!("⚠️  PID {} no es válido. Limpiando PID file.", pid);
                let _ = std::fs::remove_file(&pid_path);
                return Ok(());
            }
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                match signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                    Ok(_) => {
                        if let Err(e) = std::fs::remove_file(&pid_path) {
                            eprintln!("⚠️  No se pudo eliminar PID file: {}", e);
                        }
                        println!("✅ sentinel monitor detenido (PID {})", pid);
                    }
                    Err(e) => {
                        eprintln!("⚠️  No se pudo enviar SIGTERM a PID {}: {}. Limpiando PID file.", pid, e);
                        let _ = std::fs::remove_file(&pid_path);
                    }
                }
            }
            #[cfg(not(unix))]
            {
                println!("⚠️  --stop solo está soportado en sistemas Unix.");
            }
        }
    }
    Ok(())
}

pub fn handle_status(project_root: &Path) -> anyhow::Result<()> {
    let pid_path = project_root.join(".sentinel/monitor.pid");
    match read_pid_file(&pid_path) {
        None => println!("ℹ️  sentinel monitor no está corriendo como daemon."),
        Some(pid) => {
            if is_process_alive(pid) {
                println!("✅ sentinel monitor corriendo (PID {})", pid);
            } else {
                eprintln!("⚠️  PID {} encontrado pero el proceso ya no existe. Limpiando PID file.", pid);
                let _ = std::fs::remove_file(&pid_path);
            }
        }
    }
    Ok(())
}

pub fn start_monitor_with_options(auto: bool, project: Option<String>) {
    if auto {
        println!("{}", "🤖 Modo Autómata/Proactivo activado en el monitor.".green().bold());
    }
    start_monitor(project, auto);
}

/// Agregar un archivo a la lista de exclusión temporal (para evitar re-procesamiento)
pub fn exclude_file_from_monitor(project_root: &Path, file_path: &Path) -> anyhow::Result<()> {
    let sentinel_dir = project_root.join(".sentinel");
    std::fs::create_dir_all(&sentinel_dir)?;

    let excluded_file = sentinel_dir.join("excluded_files.txt");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let relative_path = file_path
        .strip_prefix(project_root)
        .unwrap_or(file_path)
        .to_string_lossy();

    let entry = format!("{}:{}\n", relative_path, timestamp);
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&excluded_file)?
        .write_all(entry.as_bytes())?;

    Ok(())
}

/// Verificar si un archivo está excluido temporalmente (últimos 60 segundos)
pub fn is_file_excluded(project_root: &Path, file_path: &Path) -> bool {
    let sentinel_dir = project_root.join(".sentinel");
    let excluded_file = sentinel_dir.join("excluded_files.txt");

    if !excluded_file.exists() {
        return false;
    }

    let relative_path = file_path
        .strip_prefix(project_root)
        .unwrap_or(file_path)
        .to_string_lossy();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Ok(content) = std::fs::read_to_string(&excluded_file) {
        for line in content.lines() {
            if let Some((path, timestamp_str)) = line.rsplit_once(':') {
                if path == relative_path.as_ref() {
                    if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                        if now - timestamp < 60 {
                            return true;
                        }
                    }
                }
            }
        }

        // Limpiar entradas antiguas
        let valid_lines: Vec<&str> = content
            .lines()
            .filter(|line| {
                if let Some((_, ts_str)) = line.rsplit_once(':') {
                    ts_str.parse::<u64>().ok().map_or(false, |ts| now - ts < 60)
                } else {
                    false
                }
            })
            .collect();

        let _ = std::fs::write(&excluded_file, valid_lines.join("\n"));
    }

    false
}

pub fn start_monitor(project: Option<String>, auto_mode: bool) {
    eprintln!("[DEBUG] start_monitor llamado con project: {:?}, auto_mode: {}", project, auto_mode);

    // Mostrar banner al inicio
    ui::mostrar_banner();

    let project_path = if let Some(project_name) = project {
        // Si se proporciona proyecto vía --project, usarlo directamente
        let mut path = std::path::PathBuf::from(&project_name);

        // Si es ".", canonicalizarlo al directorio actual
        if path.as_os_str() == "." {
            path = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(e) => {
                    eprintln!("❌ Error obteniendo directorio actual: {}", e);
                    std::process::exit(1);
                }
            };
        }

        if !path.exists() {
            eprintln!("❌ El proyecto no existe: {}", project_name);
            std::process::exit(1);
        }
        path
    } else {
        // Si no, pedir interactivamente
        let path = ui::seleccionar_proyecto();
        if !path.exists() {
            std::process::exit(1);
        }
        path
    };

    // Guardar como proyecto activo
    let _ = SentinelConfig::save_active_project(&project_path);

    let config = Arc::new(ui::inicializar_sentinel(&project_path));
    let stats = Arc::new(Mutex::new(SentinelStats::cargar(&project_path)));

    // --- Knowledge Base (v5.0.0 Pro) con SQLite ---
    let db_path = project_path.join(".sentinel/index.db");
    let index_db = Arc::new(index::IndexDb::open(db_path).expect("No se pudo abrir la base de datos de índice"));
    let index_builder = Arc::new(index::ProjectIndexBuilder::new(Arc::clone(&index_db)));

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
    let rule_engine = Arc::new(rule_engine.with_index_db(Arc::clone(&index_db)));

    // Indexación inicial (Capa 1)
    let spinner_index = ui::crear_progreso("   🧠 Indexando proyecto (Capa 1)...");
    let _ = index_builder.index_project(&project_path, &config.file_extensions);
    spinner_index.finish_and_clear();
    println!("   ✅ Proyecto indexado en SQLite.");

    // Reportar evento ready al Cerebro cuando Sentinel está completamente inicializado
    let agent_config = crate::agent_config::AgentConfig::from_env();
    if agent_config.report_enabled {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok();
        if let Some(r) = rt {
            let _ = r.block_on(crate::agent_reporter::report_event(
                &agent_config,
                "sentinel_ready",
                "info",
                HashMap::from([
                    ("message".to_string(), serde_json::Value::String(format!("Sentinel v{} listo para monitorear", config::SENTINEL_VERSION))),
                    ("version".to_string(), serde_json::Value::String(config::SENTINEL_VERSION.to_string())),
                    ("project".to_string(), serde_json::Value::String(project_path.to_string_lossy().to_string())),
                ])
            ));
        }
    }

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
    let index_builder_hilo = Arc::clone(&index_builder);

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
                } else if cmd == "a" {
                    print!("🔍 Ingrese la ruta a auditar (ej. src/, .): ");
                    io::stdout().flush().unwrap();
                    let mut input_path = String::new();
                    if io::stdin().read_line(&mut input_path).is_ok() {
                        let path = input_path.trim();
                        let final_path = if path.is_empty() { "." } else { path };
                        println!("🚀 Lanzando auditoría interactiva en: {}", final_path);
                        crate::commands::pro::handle_pro_command(
                            crate::commands::ProCommands::Audit {
                                target: final_path.to_string(),
                                no_fix: false,
                                format: "text".to_string(),
                                max_files: 20,
                                concurrency: 3,
                            },
                            false,
                            false,
                        );
                        println!("✅ Auditoría terminada. Volviendo a monitorear...\n");
                    }
                } else if cmd == "k" {
                    println!("   🧠 Re-indexando proyecto...");
                    let _ = index_builder_hilo.index_project(&project_path_hilo, &config_hilo.file_extensions);
                    println!("   ✅ Re-indexación completada.");
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
    let project_path_for_watcher = project_path.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any) {
                for path in event.paths {
                    if path.is_file() && !config_watcher.debe_ignorar(&path) && !is_file_excluded(&project_path_for_watcher, &path) {
                        let _ = tx.send(path);
                    }
                }
            }
        }
    })
    .unwrap();
    let project_path_watcher = project_path.clone();
    watcher
        .watch(&project_path_watcher, RecursiveMode::Recursive)
        .unwrap();

    let pausa_leer = Arc::clone(&pausa_loop);
    let _leer_respuesta = move || -> Option<String> {
        *esperando_input.lock().unwrap() = true;

        // Pausar el monitor mientras se espera input del usuario
        // para evitar que se acumulen eventos del watcher
        *pausa_leer.lock().unwrap() = true;

        let res = stdin_rx
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(30))
            .ok();

        *esperando_input.lock().unwrap() = false;
        *pausa_leer.lock().unwrap() = false;

        res
    };

    let remote_prompt = |prompt_text: &str| -> Option<String> {
        let prompt_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        crate::agent_interaction::MANAGER.register(prompt_id.clone(), tx);

        // Reportar el prompt al Cerebro
        let agent_config = crate::agent_config::AgentConfig::from_env();
        let mut payload = std::collections::HashMap::new();
        payload.insert("message".to_string(), serde_json::Value::String(prompt_text.to_string()));
        payload.insert("prompt_id".to_string(), serde_json::Value::String(prompt_id.clone()));
        payload.insert("interaction_type".to_string(), serde_json::Value::String("boolean".to_string())); // s/n

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let _ = rt.block_on(crate::agent_reporter::report_event(
            &agent_config,
            "interaction_required",
            "info",
            payload
        ));

        println!("⏳ Esperando respuesta remota (ID: {})...", prompt_id);

        // Esperar la respuesta (con timeout de 60 segundos)
        match rt.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(60), rx).await
        }) {
            Ok(Ok(answer)) => Some(answer),
            _ => {
                println!("⚠️ Timeout o error esperando respuesta remota.");
                None
            }
        }
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

    // Reiniciar señal de stop por si venimos de un reinicio
    STOP_SIGNAL.store(false, Ordering::SeqCst);

    let ultimo_cambio: HashMap<PathBuf, Instant> = HashMap::new();
    let mut was_paused = false;

    while !STOP_SIGNAL.load(Ordering::SeqCst) {
        // Usar recv_timeout para poder revisar el STOP_SIGNAL periódicamente
        let changed_path = match rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(p) => p,
            Err(_) => continue,
        };

        println!("👀 CAMBIO DETECTADO: {}", changed_path.display());
        // Normalizar la path (resolver /../ y ./.)
        let changed_path = match std::fs::canonicalize(&changed_path) {
            Ok(canonical) => canonical,
            Err(_) => changed_path, // Si no se puede canonicalizar, usar la original
        };

        thread::sleep(std::time::Duration::from_millis(500));

        // Si estamos pausados, descartar este evento y todos acumulados
        if *pausa_loop.lock().unwrap() {
            // Descartar todos los eventos acumulados
            let mut dropped = 0;
            while let Ok(_) = rx.try_recv() {
                dropped += 1;
            }
            if std::env::var("VERBOSE").is_ok() {
                println!("   [DEBUG] Pausa activa: descartado evento de {} + {} acumulados", changed_path.display(), dropped);
            }
            was_paused = true;
            continue;
        }

        // Si se acaba de reanudar después de pausa, descartar TODO incluyendo este evento
        if was_paused {
            if std::env::var("VERBOSE").is_ok() {
                println!("   [DEBUG] Monitor reanudado: descartando evento rezagado de {}", changed_path.display());
            }
            // Limpiar toda la cola
            while let Ok(_) = rx.try_recv() {}
            was_paused = false;
            continue;
        }

        // Limpiar eventos acumulados normales (debouncing)
        let mut dropped_events = 0;
        while let Ok(_) = rx.try_recv() {
            dropped_events += 1;
        }
        if dropped_events > 0 && std::env::var("VERBOSE").is_ok() {
            println!("   [DEBUG] {} eventos descartados de la cola (debounce)", dropped_events);
        }

        let ahora = Instant::now();
        if let Some(ultimo) = ultimo_cambio.get(&changed_path) {
            if ahora.duration_since(*ultimo) < std::time::Duration::from_secs(10) {
                if std::env::var("VERBOSE").is_ok() {
                    println!("   [DEBUG] {} ignorado (debounce activo)", changed_path.display());
                }
                continue;
            }
        }

        let file_name = changed_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        println!("📝 Procesando cambio en: {}", file_name);

        // --- Reportar al Cerebro (Modo Agente) ---
        let agent_config = crate::agent_config::AgentConfig::from_env();

        // Log a archivo para debuggear en modo daemon
        let log_file = project_path.join(".sentinel/monitor.log");
        let log_msg = format!("[{}] Cambio detectado: {} | report_enabled={} | cerebro_url={}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            changed_path.display(),
            agent_config.report_enabled,
            agent_config.cerebro_url
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .and_then(|mut f| std::io::Write::write_all(&mut f, log_msg.as_bytes()));

        if agent_config.report_enabled {
            println!("📡 Reportando evento a Cerebro...");
            let mut payload = std::collections::HashMap::new();
            payload.insert("file".to_string(), serde_json::Value::String(changed_path.to_string_lossy().to_string()));
            payload.insert("message".to_string(), serde_json::Value::String(format!("Archivo modificado: {}", file_name)));

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            match rt.block_on(crate::agent_reporter::report_event(
                &agent_config,
                "file_change",
                "warning",
                payload
            )) {
                Ok(_) => {
                    println!("   ✅ Evento reportado con éxito.");
                    // Log exito
                    let success_msg = format!("[{}] ✅ Evento reportado exitosamente a Cerebro\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S")
                    );
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_file)
                        .and_then(|mut f| std::io::Write::write_all(&mut f, success_msg.as_bytes()));
                }
                Err(e) => {
                    eprintln!("   ❌ Error al reportar evento: {}", e);
                    // Log error
                    let error_msg = format!("[{}] ❌ Error reportando a Cerebro: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"), e
                    );
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_file)
                        .and_then(|mut f| std::io::Write::write_all(&mut f, error_msg.as_bytes()));
                }
            }
        } else {
            // Log que report está deshabilitado
            let disabled_msg = format!("[{}] ⚠️ Reporte deshabilitado - evento no enviado\n",
                Local::now().format("%Y-%m-%d %H:%M:%S")
            );
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)
                .and_then(|mut f| std::io::Write::write_all(&mut f, disabled_msg.as_bytes()));
        }

        // --- Actualizar Índice de Símbolos (SQLite) ---
        let _ = index_builder.index_file(&changed_path, &project_path);

        // --- BusinessLogicGuard: detectar regresiones vs último commit ---
        let regression_context = {
            let prev = business_logic_guard::get_git_previous_content(&changed_path, &project_path);
            if let Some(prev_content) = prev {
                if let Ok(new_content) = std::fs::read_to_string(&changed_path) {
                    business_logic_guard::build_regression_context(&prev_content, &new_content)
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(ref diff_ctx) = regression_context {
            println!("\n🔍 {} Analizando regresiones vs último commit...", "BusinessLogicGuard:".bold().yellow());
            let regression_prompt = business_logic_guard::build_regression_prompt(diff_ctx, &file_name);
            let config_bg = Arc::clone(&config);
            let stats_bg = Arc::clone(&stats);
            let project_bg = project_path.clone();
            if let Ok(result) = ai::client::consultar_ia_dinamico(regression_prompt, ai::client::TaskType::Light, &config_bg, stats_bg, &project_bg) {
                if result.contains("REGRESION_DETECTADA") {
                    println!("   {} {}", "⚠️  REGRESIÓN:".red().bold(), result.lines().find(|l| l.contains("REGRESION_DETECTADA")).unwrap_or(""));
                } else if result.contains("REVISAR") {
                    println!("   {} {}", "🔎 REVISAR:".yellow(), result.lines().find(|l| l.contains("REVISAR")).unwrap_or(""));
                } else {
                    println!("   {} Sin regresiones de lógica de negocio detectadas.", "✅".green());
                }
            }
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

        let auto_mode_active = auto_mode || config.auto_mode;
        let test_rel_path =
            files::buscar_archivo_test(&base_name, &project_path, &config.test_patterns);

        if test_rel_path.is_none() {
            let auto_label = if auto_mode_active { "[AUTONOMOUS]".green().bold() } else { "[INTERACTIVE]".yellow() };
            println!("\n{} 🔔 CAMBIO EN: {}", auto_label, file_name.cyan().bold());
            println!(
                "{}",
                "⚠️  No se encontraron tests para este archivo.".yellow()
            );
            
            let run_full_analysis = if auto_mode_active {
                println!("   🤖 Modo autónomo: Procediendo con análisis sin tests automáticamente.");
                true
            } else {
                let query_text = format!("Sentinel: No hay tests para {}. ¿Deseas que revise el código de todas formas? (s/n)", file_name);
                match remote_prompt(&query_text) {
                    Some(respuesta) if respuesta == "s" => true,
                    _ => false,
                }
            };

            if run_full_analysis {
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
                        Ok((aprobado, consejo)) => {
                            let mut payload = std::collections::HashMap::new();
                            payload.insert("file".to_string(), serde_json::Value::String(changed_path.to_string_lossy().to_string()));
                            payload.insert("status".to_string(), serde_json::Value::String(if aprobado { "approved".to_string() } else { "rejected".to_string() }));
                            payload.insert("findings".to_string(), serde_json::Value::String(consejo.clone()));
                            payload.insert("message".to_string(), serde_json::Value::String(format!("Análisis IA: {}", if aprobado { "SEGURO" } else { "HALLAZGOS DETECTADOS" })));
                            
                            report_sync("analysis_completed", if aprobado { "info" } else { "warning" }, payload);

                            if aprobado {
                                println!("   ✅ Código revisado. Sin tests, no se realizará commit automático.");
                            } else {
                                println!("   ⚠️  Se encontraron problemas. Revisa las sugerencias.");
                            }
                        },
                        Err(e) => {
                            report_sync("analysis_failed", "error", std::collections::HashMap::from([
                                ("message".to_string(), serde_json::Value::String(format!("Error en análisis IA: {}", e))),
                                ("error".to_string(), serde_json::Value::String(e.to_string())),
                                ("file".to_string(), serde_json::Value::String(changed_path.to_string_lossy().to_string()))
                            ]));
                            println!("   ❌ Error al analizar: {}", e);
                        }
                    }
                }
            } else {
                println!("   ⏭️  Revisión omitida. Continuando monitoreo...");
            }
            continue;
        }

        if let Some(test_path) = test_rel_path {
            println!("\n🔔 CAMBIO EN: {}", file_name.cyan().bold());
            
            let query_text = format!("Sentinel: Se encontró el test '{}' para {}. ¿Procedo con validación y ejecución? (s/n)", test_path, file_name);
            let procedo = auto_mode_active || match remote_prompt(&query_text) {
                Some(respuesta) if respuesta == "s" => true,
                _ => false,
            };
            
            if procedo {
                if let Ok(codigo) = std::fs::read_to_string(&changed_path) {
                    // Validar Reglas Pro (Estáticas)
                    let spinner = ui::crear_progreso("   🔍 Validando reglas estáticas...");
                    let violaciones = rule_engine.validate_file(&changed_path, &codigo);
                    spinner.finish_and_clear();

                    if !violaciones.is_empty() {
                        println!("\n🚩 {}", "VIOLACIONES DE ARQUITECTURA DETECTADAS:".bold().red());
                        let mut violations_msg = String::new();
                        for v in &violaciones {
                            let label = match v.level {
                                crate::rules::RuleLevel::Error => "ERROR",
                                crate::rules::RuleLevel::Warning => "WARN",
                                crate::rules::RuleLevel::Info => "INFO",
                            };
                            let line_info = format!("   • [{}][{}]: {}\n", label, v.rule_name.yellow(), v.message);
                            print!("{}", line_info);
                            violations_msg.push_str(&format!("[{}] {}: {}\n", label, v.rule_name, v.message));
                        }

                        // Reportar violaciones estáticas al Cerebro
                        let agent_config = crate::agent_config::AgentConfig::from_env();
                        let mut payload = std::collections::HashMap::new();
                        payload.insert("file".to_string(), serde_json::Value::String(changed_path.to_string_lossy().to_string()));
                        payload.insert("violations".to_string(), serde_json::Value::String(violations_msg));
                        payload.insert("message".to_string(), serde_json::Value::String(format!("⚠️ Se detectaron {} violaciones de reglas en {}", violaciones.len(), file_name)));
                        
                        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                        let _ = rt.block_on(crate::agent_reporter::report_event(
                            &agent_config,
                            "static_analysis_violations",
                            "warning",
                            payload
                        ));
                    }

                    let spinner_ai = ui::crear_progreso("   🤖 Analizando arquitectura con IA...");
                    let resultado_analisis = ai::analizar_arquitectura(&codigo, &file_name, Arc::clone(&stats), &config, &project_path, &changed_path);
                    spinner_ai.finish_and_clear();

                    match resultado_analisis {
                        Ok((aprobado, consejo)) => {
                            // Reportar Análisis al Cerebro
                            let mut ai_payload = std::collections::HashMap::new();
                            ai_payload.insert("file".to_string(), serde_json::Value::String(changed_path.to_string_lossy().to_string()));
                            ai_payload.insert("status".to_string(), serde_json::Value::String(if aprobado { "approved".to_string() } else { "rejected".to_string() }));
                            ai_payload.insert("findings".to_string(), serde_json::Value::String(consejo.clone()));
                            report_sync("analysis_completed", if aprobado { "info" } else { "warning" }, ai_payload);

                            if aprobado {
                                // Reportar que inician tests
                                report_sync("tests_starting", "info", std::collections::HashMap::from([
                                    ("test_path".to_string(), serde_json::Value::String(test_path.clone())),
                                    ("message".to_string(), serde_json::Value::String(format!("🧪 Ejecutando tests para {}", file_name)))
                                ]));

                                match crate::tests::ejecutar_tests(&test_path, &project_path, &config) {
                                    Ok(_) => {
                                        report_sync("test_result", "info", std::collections::HashMap::from([
                                            ("status".to_string(), serde_json::Value::String("passed".to_string())),
                                            ("message".to_string(), serde_json::Value::String(format!("✅ Tests pasados con éxito para {}", file_name)))
                                        ]));

                                        let _ = docs::actualizar_documentacion(&codigo, &changed_path, &config, Arc::clone(&stats), &project_path);
                                        let msg = git::generar_mensaje_commit(&codigo, &file_name, &config, Arc::clone(&stats), &project_path);
                                        println!("\n🚀 Mensaje: {}", msg.bright_cyan().bold());
                                        
                                        // Reportar sugerencia de commit al Cerebro antes de proceder
                                        report_sync("commit_suggestion", "info", std::collections::HashMap::from([
                                            ("message".to_string(), serde_json::Value::String(msg.clone())),
                                            ("file".to_string(), serde_json::Value::String(file_name.clone()))
                                        ]));

                                        if auto_mode_active {
                                            println!("   📦 Modo Autónomo: Realizando commit automático...");
                                            git::preguntar_commit(&project_path, &msg, "s");
                                        } else {
                                            let query_commit = format!("Sentinel: Tests pasaron para {}. ¿Hago el commit con el mensaje '{}'? (s/n)", file_name, msg);
                                            if let Some(r) = remote_prompt(&query_commit) {
                                                git::preguntar_commit(&project_path, &msg, &r);
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        report_sync("test_result", "error", std::collections::HashMap::from([
                                            ("status".to_string(), serde_json::Value::String("failed".to_string())),
                                            ("error".to_string(), serde_json::Value::String(e.clone())),
                                            ("message".to_string(), serde_json::Value::String(format!("❌ Tests fallidos para {}: {}", file_name, e)))
                                        ]));

                                        let query_help = format!("Sentinel: Tests fallaron para {}. ¿Deseas ayuda de la IA para corregirlo? (s/n)", file_name);
                                        let pedir_ayuda = if auto_mode_active { false } else { remote_prompt(&query_help).as_deref() == Some("s") };
                                        
                                        if pedir_ayuda {
                                            let _ = crate::tests::pedir_ayuda_test(&codigo, &test_path, &config, Arc::clone(&stats), &project_path);
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            report_sync("analysis_failed", "error", std::collections::HashMap::from([
                                ("message".to_string(), serde_json::Value::String(format!("Error en análisis IA: {}", e))),
                                ("error".to_string(), serde_json::Value::String(e.to_string())),
                                ("file".to_string(), serde_json::Value::String(changed_path.to_string_lossy().to_string()))
                            ]));
                            println!("   ❌ Error al analizar: {}", e);
                        }
                    }
                }
            } else {
                println!("   ⏭️  Revisión omitida por el usuario.");
            }
        }

        // Guardar stats después de procesar cada archivo (para registrar historial diario)
        {
            let stats_snapshot = stats.lock().unwrap().clone();
            stats_snapshot.guardar(&project_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pid_file_write_and_read() {
        let tmp = TempDir::new().unwrap();
        let sentinel_dir = tmp.path().join(".sentinel");
        std::fs::create_dir_all(&sentinel_dir).unwrap();
        let pid_path = sentinel_dir.join("monitor.pid");

        write_pid_file(&pid_path, 12345).unwrap();
        let pid = read_pid_file(&pid_path).unwrap();
        assert_eq!(pid, 12345);
    }

    #[test]
    fn test_read_pid_file_returns_none_if_missing() {
        let tmp = TempDir::new().unwrap();
        let pid_path = tmp.path().join(".sentinel/monitor.pid");
        assert!(read_pid_file(&pid_path).is_none());
    }

    #[test]
    fn test_write_pid_file_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let pid_path = tmp.path().join(".sentinel/nested/monitor.pid");
        // Parent does not exist yet
        write_pid_file(&pid_path, 99).unwrap();
        assert!(pid_path.exists());
        assert_eq!(read_pid_file(&pid_path).unwrap(), 99);
    }

    #[test]
    fn test_read_pid_file_with_corrupt_content() {
        let tmp = TempDir::new().unwrap();
        let pid_path = tmp.path().join("monitor.pid");
        std::fs::write(&pid_path, "not_a_number").unwrap();
        // Corrupt content must return None, not panic
        assert!(read_pid_file(&pid_path).is_none());
    }

    #[test]
    fn test_read_pid_file_with_whitespace() {
        let tmp = TempDir::new().unwrap();
        let pid_path = tmp.path().join("monitor.pid");
        std::fs::write(&pid_path, "  42  \n").unwrap();
        // Whitespace around PID must be trimmed correctly
        assert_eq!(read_pid_file(&pid_path), Some(42));
    }

    #[cfg(unix)]
    #[test]
    fn test_is_process_alive_self() {
        // The current process must always be alive
        let my_pid = std::process::id();
        assert!(is_process_alive(my_pid), "own PID should be alive");
    }

    #[cfg(unix)]
    #[test]
    fn test_is_process_alive_impossible_pid() {
        // PID u32::MAX is guaranteed not to exist on any real system
        // (max Linux PID is 4194304). Must return false, not panic.
        assert!(!is_process_alive(u32::MAX));
    }

    #[cfg(unix)]
    #[test]
    fn test_handle_status_removes_stale_pid_file() {
        let tmp = TempDir::new().unwrap();
        let sentinel_dir = tmp.path().join(".sentinel");
        std::fs::create_dir_all(&sentinel_dir).unwrap();
        let pid_path = sentinel_dir.join("monitor.pid");

        // Write a PID that is guaranteed not to exist
        write_pid_file(&pid_path, u32::MAX).unwrap();
        assert!(pid_path.exists(), "pid file should exist before handle_status");

        handle_status(tmp.path()).unwrap();

        // handle_status must clean up stale PID file (is_process_alive(u32::MAX) = false)
        assert!(!pid_path.exists(), "stale pid file should be removed by handle_status");
    }
}
