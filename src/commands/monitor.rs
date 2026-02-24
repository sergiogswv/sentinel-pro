use std::path::Path;
use crate::config::SentinelConfig;
use crate::rules::engine::RuleEngine;
use crate::stats::SentinelStats;
use crate::{ai, config, docs, files, git, index, tests as test_runner, ui, business_logic_guard};
use colored::*;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

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

pub(crate) fn is_process_alive(pid: u32) -> bool {
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

    let exe = std::env::current_exe()?;
    let mut command = std::process::Command::new(exe);
    command
        .arg("monitor")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

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

pub fn start_monitor() {
    // Mostrar banner al inicio
    ui::mostrar_banner();

    let project_path = ui::seleccionar_proyecto();
    if !project_path.exists() {
        std::process::exit(1);
    }

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
    let project_path_watcher = project_path.clone();
    watcher
        .watch(&project_path_watcher.join("src"), RecursiveMode::Recursive)
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

                        let spinner_ai =
                            ui::crear_progreso("   🤖 Analizando arquitectura con IA...");
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
                        if test_runner::ejecutar_tests(&test_path, &project_path).is_ok() {
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
                                let _ = test_runner::pedir_ayuda_test(
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
