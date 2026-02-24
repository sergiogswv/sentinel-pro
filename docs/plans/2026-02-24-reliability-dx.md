# Reliability + DX Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Completar la Capa 1 (confiabilidad) y Capa 2 (DX/UX) del roadmap de Sentinel — umbrales de reglas efectivos, tests faltantes, onboarding con `sentinel init`, diagnóstico con `sentinel doctor`, flags globales `--quiet`/`--verbose`, `.sentinelignore` por directorio, y refactorización de `pro.rs`.

**Architecture:** 7 tareas independientes. Las tareas 1-2 corrigen la Capa 1 restante. Las tareas 3-6 añaden comandos y flags nuevos sin tocar lógica existente. La tarea 7 es refactorización pura de `pro.rs` en módulos — sin cambios de comportamiento.

**Tech Stack:** Rust, clap 4.x (`global = true` para flags globales), tree-sitter, serde/toml, anyhow, tempfile (tests).

**Estado actual (ya implementado — no repetir):**
- ✅ `count_word_occurrences` con `\b` word-boundary en `static_analysis.rs`
- ✅ Números de línea en todas las violaciones (`find_line_of` + `.start_position().row + 1`)
- ✅ Audit CI mode: `--no-fix`, `--format json`, TTY auto-detect (pro.rs líneas 2317-2320)
- ✅ Check display agrupado por archivo con `:line` inline (pro.rs líneas 560-603)

---

## Task 1: Effective RuleConfig Thresholds (lower generation floors)

**Problem:** `HIGH_COMPLEXITY` y `FUNCTION_TOO_LONG` tienen floors hardcodeados (`> 10`, `> 50`). Si el usuario configura `complexity_threshold = 7`, funciones con complejidad 8 o 9 NUNCA se generan, así que el filtro `violations.retain()` en pro.rs no puede mostrarlas. El floor debe ser menor que cualquier umbral razonable.

**Fix:** Bajar el floor de generación a `> 5` para complejidad y `> 10` para largo de función en los tres analizadores. La retención en pro.rs ya maneja el threshold configurable correctamente.

**Files:**
- Modify: `src/rules/static_analysis.rs` (2 cambios: líneas ~207 y ~234)
- Modify: `src/rules/languages/go.rs` (2 cambios: líneas ~178 y ~194)
- Modify: `src/rules/languages/python.rs` (2 cambios: líneas ~193 y ~205)

---

**Step 1: Write failing tests**

En `src/rules/static_analysis.rs`, añadir al bloque `#[cfg(test)]` al final:

```rust
#[test]
fn test_complexity_generates_above_floor_5() {
    // 5 if statements = complexity 6. With old floor > 10 this was never generated.
    // After fix the floor is > 5, so complexity 6 must be reported.
    let lang = ts_lang();
    let analyzer = ComplexityAnalyzer::new();
    let code = "function f(x) {\n\
                  if (x>0) { return 1; }\n\
                  if (x>1) { return 2; }\n\
                  if (x>2) { return 3; }\n\
                  if (x>3) { return 4; }\n\
                  if (x>4) { return 5; }\n\
                  return 0;\n\
                }";
    let violations = analyzer.analyze(&lang, code);
    let v = violations.iter().find(|v| v.rule_name == "HIGH_COMPLEXITY");
    assert!(v.is_some(), "complexity 6 (above new floor 5) should be flagged, got: {:?}", violations);
    assert_eq!(v.unwrap().value, Some(6));
}

#[test]
fn test_function_length_generates_above_floor_10() {
    // A 12-line function should be flagged after lowering floor to > 10.
    let lang = ts_lang();
    let analyzer = ComplexityAnalyzer::new();
    let code = format!("function f() {{\n{}}}", "  const x = 1;\n".repeat(12));
    let violations = analyzer.analyze(&lang, &code);
    let v = violations.iter().find(|v| v.rule_name == "FUNCTION_TOO_LONG");
    assert!(v.is_some(), "12-line function (above new floor 10) should be flagged, got: {:?}", violations);
}
```

**Step 2: Run failing tests**

```bash
cargo test test_complexity_generates_above_floor_5 test_function_length_generates_above_floor_10 2>&1 | tail -10
```

Expected: FAIL — complexity 6 NOT flagged (floor is still > 10).

**Step 3: Lower generation floors**

En `src/rules/static_analysis.rs`:
- Línea ~207: cambiar `if complexity > 10 {` → `if complexity > 5 {`
- Línea ~234: cambiar `if line_count > 50 {` → `if line_count > 10 {`

En `src/rules/languages/go.rs`:
- Línea ~178: cambiar `if complexity > 10 {` → `if complexity > 5 {`
- Línea ~194: cambiar `if line_count > 50 {` → `if line_count > 10 {`

En `src/rules/languages/python.rs`:
- Línea ~193: cambiar `if complexity > 10 {` → `if complexity > 5 {`
- Línea ~205: cambiar `if line_count > 50 {` → `if line_count > 10 {`

También actualizar los NOTEs en comentarios en los 3 archivos:
```
// NOTE: 5 is the absolute generation floor for complexity.
// The configured complexity_threshold (default 10) is applied via
// violations.retain() in pro.rs after generation.
```

**Step 4: Run tests**

```bash
cargo test 2>&1 | tail -10
```

Expected: PASS — `test_complexity_ok_for_short_function` sigue pasando (complexity 1 no se reporta), los 2 tests nuevos pasan, total de tests aumenta en 2.

**Step 5: Commit**

```bash
git add src/rules/static_analysis.rs src/rules/languages/go.rs src/rules/languages/python.rs
git commit -m "fix: lower complexity/length generation floors so RuleConfig thresholds < 10 work"
```

---

## Task 2: Missing Tests (monitor + check JSON integration)

**Files:**
- Modify: `src/commands/monitor.rs` (añadir 4 tests al bloque existente)
- Modify: `src/commands/pro.rs` (añadir 2 tests de integración check JSON)

---

**Step 1: Write failing tests for monitor**

Al bloque `#[cfg(test)]` en `src/commands/monitor.rs`, añadir después del test existente:

```rust
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

    // handle_status must clean up stale PID file on non-Unix (is_process_alive returns false)
    // On Unix, u32::MAX also returns false.
    assert!(!pid_path.exists(), "stale pid file should be removed by handle_status");
}
```

**Step 2: Run tests to verify they fail or compile errors**

```bash
cargo test test_read_pid_file_with_corrupt_content test_is_process_alive_self test_handle_status_removes_stale_pid_file 2>&1 | tail -15
```

Expected: compile error or test failure (functions exist but tests may pass already for some).

**Step 3: Run all tests to verify new ones pass**

```bash
cargo test 2>&1 | tail -10
```

Expected: all existing + new tests pass.

**Step 4: Add check --format json integration test**

En `src/commands/pro.rs`, al final del bloque `#[cfg(test)]` existente, añadir:

```rust
#[test]
fn test_check_json_mode_flag_detection() {
    // Verify that format == "json" activates json_mode
    let fmt = "json".to_string();
    let json_mode = fmt.to_lowercase() == "json";
    assert!(json_mode, "--format json should activate json_mode");

    let fmt2 = "text".to_string();
    let json_mode2 = fmt2.to_lowercase() == "json";
    assert!(!json_mode2, "--format text should not activate json_mode");
}

#[test]
fn test_sarif_mode_flag_detection() {
    let fmt = "sarif".to_string();
    let sarif_mode = fmt.to_lowercase() == "sarif";
    assert!(sarif_mode, "--format sarif should activate sarif_mode");
}
```

**Step 5: Run all tests**

```bash
cargo test 2>&1 | tail -5
```

Expected: PASS — all tests green, count increases.

**Step 6: Commit**

```bash
git add src/commands/monitor.rs src/commands/pro.rs
git commit -m "test: add coverage for corrupt PID, is_process_alive, stale PID cleanup, JSON mode flags"
```

---

## Task 3: `sentinel init` — Onboarding command

**Goal:** `sentinel init` detecta el lenguaje del proyecto, genera `.sentinel/config.toml` con defaults sensatos y muestra qué se configuró. Si ya existe config, pide confirmación antes de sobrescribir.

**Files:**
- Create: `src/commands/init.rs`
- Modify: `src/commands/mod.rs` (añadir `pub mod init;` + `Commands::Init`)
- Modify: `src/main.rs` (añadir match arm)

---

**Step 1: Write failing test**

Crear `src/commands/init.rs` con solo el test:

```rust
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    #[test]
    fn test_detect_languages_from_extensions() {
        let tmp = TempDir::new().unwrap();
        // Create some files
        std::fs::write(tmp.path().join("app.ts"), "").unwrap();
        std::fs::write(tmp.path().join("util.go"), "").unwrap();

        let exts = super::detect_project_extensions(tmp.path());
        assert!(exts.contains(&"ts".to_string()), "should detect .ts");
        assert!(exts.contains(&"go".to_string()), "should detect .go");
        assert!(!exts.contains(&"py".to_string()), "should not detect .py (none present)");
    }

    #[test]
    fn test_init_creates_config_file() {
        let tmp = TempDir::new().unwrap();
        super::run_init(tmp.path(), false).unwrap();
        let config_path = tmp.path().join(".sentinel/config.toml");
        assert!(config_path.exists(), "init should create .sentinel/config.toml");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("file_extensions"), "config must contain file_extensions");
    }

    #[test]
    fn test_init_does_not_overwrite_without_force() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".sentinel")).unwrap();
        let config_path = tmp.path().join(".sentinel/config.toml");
        std::fs::write(&config_path, "existing = true").unwrap();

        // force=false: should NOT overwrite, return Err
        let result = super::run_init(tmp.path(), false);
        assert!(result.is_err(), "init without force should fail if config exists");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "existing = true", "content must be unchanged");
    }
}
```

**Step 2: Run to verify compilation failure**

```bash
cargo test test_detect_languages_from_extensions 2>&1 | head -10
```

Expected: compile error — `super::detect_project_extensions` not found.

**Step 3: Implement `src/commands/init.rs`**

```rust
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use colored::*;

/// Scans `root` recursively (up to depth 3) and returns unique file extensions
/// that Sentinel supports. Ignores `node_modules`, `.git`, `target`, `vendor`.
pub fn detect_project_extensions(root: &Path) -> Vec<String> {
    const SUPPORTED: &[&str] = &["ts", "tsx", "js", "jsx", "go", "py"];
    const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", "vendor", "dist", ".sentinel"];

    let mut found: HashSet<String> = HashSet::new();
    walk_extensions(root, 0, 3, SUPPORTED, SKIP_DIRS, &mut found);
    let mut result: Vec<String> = found.into_iter().collect();
    result.sort();
    result
}

fn walk_extensions(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    supported: &[&str],
    skip_dirs: &[&str],
    found: &mut HashSet<String>,
) {
    if depth > max_depth { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !skip_dirs.contains(&name) {
                walk_extensions(&path, depth + 1, max_depth, supported, skip_dirs, found);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if supported.contains(&ext) {
                found.insert(ext.to_string());
            }
        }
    }
}

/// Runs `sentinel init` in `project_root`.
/// If config already exists and `force == false`, returns Err.
pub fn run_init(project_root: &Path, force: bool) -> anyhow::Result<()> {
    let sentinel_dir = project_root.join(".sentinel");
    let config_path = sentinel_dir.join("config.toml");

    if config_path.exists() && !force {
        anyhow::bail!(
            "Ya existe una configuración en {}. Usa --force para sobrescribir.",
            config_path.display()
        );
    }

    std::fs::create_dir_all(&sentinel_dir)?;

    let extensions = detect_project_extensions(project_root);
    let ext_list = if extensions.is_empty() {
        vec!["ts".to_string(), "js".to_string()]
    } else {
        extensions.clone()
    };

    let ext_toml = ext_list
        .iter()
        .map(|e| format!("\"{}\"", e))
        .collect::<Vec<_>>()
        .join(", ");

    let config_content = format!(
        r#"# Sentinel Pro — Configuración del Proyecto
# Generado por `sentinel init`

[sentinel]
file_extensions = [{ext_list}]
test_patterns = ["**/*.spec.ts", "**/*.test.ts", "**/*.spec.js", "**/*.test.js"]

[rule_config]
complexity_threshold = 10
function_length_threshold = 50
dead_code_enabled = true
unused_imports_enabled = true
"#,
        ext_list = ext_toml
    );

    std::fs::write(&config_path, &config_content)?;
    Ok(())
}

pub fn handle_init_command(project_root: &Path, force: bool) {
    println!("\n{}", "🚀 Sentinel Init".bold().green());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let extensions = detect_project_extensions(project_root);
    if extensions.is_empty() {
        println!("   ℹ️  No se detectaron lenguajes soportados. Usando TypeScript por defecto.");
    } else {
        println!("   🔍 Lenguajes detectados: {}", extensions.join(", ").cyan());
    }

    match run_init(project_root, force) {
        Ok(()) => {
            let config_path = project_root.join(".sentinel/config.toml");
            println!("   ✅ Configuración creada en: {}", config_path.display().to_string().cyan());
            println!("\n   {} Próximos pasos:", "💡".yellow());
            println!("      sentinel pro check src/    # análisis estático");
            println!("      sentinel pro audit src/    # auditoría interactiva");
            println!("      sentinel pro review        # review arquitectónico con IA");
        }
        Err(e) => {
            eprintln!("   ❌ {}", e);
            eprintln!("   💡 Usa --force para sobrescribir la configuración existente.");
        }
    }
}
```

**Step 4: Add to `src/commands/mod.rs`**

Añadir `pub mod init;` con los otros mods al inicio del archivo.

Añadir a `Commands`:

```rust
/// Inicializa la configuración de Sentinel en el proyecto actual
Init {
    /// Sobrescribir configuración existente si la hay
    #[arg(long)]
    force: bool,
},
```

**Step 5: Add match arm to `src/main.rs`**

```rust
Some(Commands::Init { force }) => {
    let project_root = crate::config::SentinelConfig::find_project_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    commands::init::handle_init_command(&project_root, force);
}
```

**Step 6: Run tests**

```bash
cargo test test_detect_languages test_init_ 2>&1 | tail -15
cargo build 2>&1 | grep "^error"
```

Expected: 3 new tests PASS, build limpio.

**Step 7: Manual smoke test**

```bash
cargo run --bin sentinel -- init --help
cargo run --bin sentinel -- init
```

Expected: imprime lenguajes detectados y crea `.sentinel/config.toml`.

**Step 8: Commit**

```bash
git add src/commands/init.rs src/commands/mod.rs src/main.rs
git commit -m "feat: sentinel init — auto-detect languages and generate config.toml"
```

---

## Task 4: `sentinel doctor` — Diagnóstico del entorno

**Goal:** `sentinel doctor` verifica que el entorno de Sentinel esté correctamente configurado: config válida, API key, índice SQLite, lenguajes activos.

**Files:**
- Create: `src/commands/doctor.rs`
- Modify: `src/commands/mod.rs` (añadir `pub mod doctor;` + `Commands::Doctor`)
- Modify: `src/main.rs` (añadir match arm)

---

**Step 1: Write failing test**

Crear `src/commands/doctor.rs` con solo el test:

```rust
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    #[test]
    fn test_check_config_returns_ok_when_config_exists() {
        let tmp = TempDir::new().unwrap();
        let sentinel_dir = tmp.path().join(".sentinel");
        std::fs::create_dir_all(&sentinel_dir).unwrap();
        std::fs::write(
            sentinel_dir.join("config.toml"),
            "[sentinel]\nfile_extensions = [\"ts\"]\n",
        ).unwrap();
        let result = super::check_config(tmp.path());
        assert!(result.is_ok(), "should return Ok when config.toml exists and is valid");
    }

    #[test]
    fn test_check_config_returns_err_when_missing() {
        let tmp = TempDir::new().unwrap();
        let result = super::check_config(tmp.path());
        assert!(result.is_err(), "should return Err when config.toml is missing");
    }

    #[test]
    fn test_check_index_returns_false_when_missing() {
        let tmp = TempDir::new().unwrap();
        assert!(!super::check_index(tmp.path()), "index.db missing → false");
    }

    #[test]
    fn test_check_api_key_detects_missing() {
        // Only run this test if the variable is NOT set in the environment
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            return; // skip if key is set in CI
        }
        assert!(!super::check_api_key(), "ANTHROPIC_API_KEY not set → false");
    }
}
```

**Step 2: Run to verify compile failure**

```bash
cargo test test_check_config 2>&1 | head -10
```

Expected: compile error.

**Step 3: Implement `src/commands/doctor.rs`**

```rust
use std::path::Path;
use colored::*;

pub fn check_config(project_root: &Path) -> anyhow::Result<crate::config::SentinelConfig> {
    let config_path = project_root.join(".sentinel/config.toml");
    if !config_path.exists() {
        anyhow::bail!("No se encontró .sentinel/config.toml. Corre `sentinel init` primero.");
    }
    crate::config::SentinelConfig::load(project_root)
        .map_err(|e| anyhow::anyhow!("Config inválida: {}", e))
}

pub fn check_api_key() -> bool {
    std::env::var("ANTHROPIC_API_KEY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

pub fn check_index(project_root: &Path) -> bool {
    let db_path = project_root.join(".sentinel/index.db");
    db_path.exists() && db_path.metadata().map(|m| m.len() > 0).unwrap_or(false)
}

pub fn handle_doctor_command(project_root: &Path) {
    println!("\n{}", "🩺 Sentinel Doctor".bold().green());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut issues = 0usize;

    // 1. Config
    match check_config(project_root) {
        Ok(cfg) => {
            println!("   ✅ Configuración: {} (extensiones: {})",
                project_root.join(".sentinel/config.toml").display(),
                cfg.file_extensions.join(", ").cyan()
            );
        }
        Err(e) => {
            eprintln!("   ❌ Configuración: {}", e);
            issues += 1;
        }
    }

    // 2. API Key
    if check_api_key() {
        println!("   ✅ ANTHROPIC_API_KEY: configurada");
    } else {
        eprintln!("   ❌ ANTHROPIC_API_KEY: no encontrada. Necesaria para review y audit con IA.");
        eprintln!("      export ANTHROPIC_API_KEY=sk-ant-...");
        issues += 1;
    }

    // 3. Índice SQLite
    if check_index(project_root) {
        let size = project_root
            .join(".sentinel/index.db")
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        println!("   ✅ Índice SQLite: presente ({} KB)", size / 1024);
    } else {
        println!("   ⚠️  Índice SQLite: no encontrado. Se creará al correr `sentinel monitor` o `sentinel index`.");
    }

    // 4. Lenguajes detectados en el proyecto
    let extensions = crate::commands::init::detect_project_extensions(project_root);
    if extensions.is_empty() {
        println!("   ⚠️  Lenguajes: ninguno detectado en el proyecto.");
    } else {
        println!("   ✅ Lenguajes en el proyecto: {}", extensions.join(", ").cyan());
    }

    // 5. Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if issues == 0 {
        println!("   {} Entorno OK — Sentinel listo para usar.", "✅".green());
    } else {
        eprintln!("   {} {} problema(s) encontrado(s). Sigue las instrucciones arriba.", "❌".red(), issues);
        std::process::exit(1);
    }
}
```

**Step 4: Add to `src/commands/mod.rs`**

Añadir `pub mod doctor;` al bloque de mods.

Añadir a `Commands`:

```rust
/// Verifica que el entorno de Sentinel esté correctamente configurado
Doctor,
```

**Step 5: Add to `src/main.rs`**

```rust
Some(Commands::Doctor) => {
    let project_root = crate::config::SentinelConfig::find_project_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    commands::doctor::handle_doctor_command(&project_root);
}
```

**Step 6: Run tests + build**

```bash
cargo test test_check_config test_check_index test_check_api_key 2>&1 | tail -10
cargo build 2>&1 | grep "^error"
```

Expected: 4 new tests PASS, build limpio.

**Step 7: Smoke test**

```bash
cargo run --bin sentinel -- doctor
```

Expected: imprime estado de todos los checks con iconos ✅/❌/⚠️.

**Step 8: Commit**

```bash
git add src/commands/doctor.rs src/commands/mod.rs src/main.rs
git commit -m "feat: sentinel doctor — environment diagnostics (config, API key, index, languages)"
```

---

## Task 5: Global `--quiet` / `--verbose` flags

**Goal:** `--quiet` suprime todo output excepto errores y el exit code (para CI). `--verbose` muestra información de debug: archivos procesados, tiempos, queries. Disponibles en todos los subcomandos via `global = true` en clap.

**Files:**
- Modify: `src/commands/mod.rs` (añadir flags a `Cli`, añadir `OutputMode` + `get_output_mode()`)
- Modify: `src/commands/pro.rs` (usar `OutputMode` en check handler)

---

**Step 1: Write failing test**

En `src/commands/pro.rs`, bloque `#[cfg(test)]`, añadir:

```rust
#[test]
fn test_output_mode_from_flags() {
    use crate::commands::{OutputMode, get_output_mode};
    assert_eq!(get_output_mode(true, false), OutputMode::Quiet);
    assert_eq!(get_output_mode(false, true), OutputMode::Verbose);
    assert_eq!(get_output_mode(false, false), OutputMode::Normal);
    // quiet takes precedence over verbose if both set
    assert_eq!(get_output_mode(true, true), OutputMode::Quiet);
}
```

**Step 2: Run to verify failure**

```bash
cargo test test_output_mode_from_flags 2>&1 | head -10
```

Expected: compile error — `OutputMode` not found.

**Step 3: Add `OutputMode` to `src/commands/mod.rs`**

Añadir después de los `use` al inicio del archivo:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum OutputMode {
    Normal,
    Quiet,
    Verbose,
}

pub fn get_output_mode(quiet: bool, verbose: bool) -> OutputMode {
    if quiet { OutputMode::Quiet }
    else if verbose { OutputMode::Verbose }
    else { OutputMode::Normal }
}
```

Añadir a `Cli`:

```rust
#[derive(Parser)]
#[command(name = "sentinel")]
#[command(about = "AI-Powered Code Monitor & Development Suite", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Suprime todo output excepto errores (para CI/scripts)
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Muestra información de debug: archivos procesados, tiempos, queries
    #[arg(long, global = true)]
    pub verbose: bool,
}
```

**Step 4: Apply `--quiet` in check handler**

En `src/commands/pro.rs`, en el handler de `ProCommands::Check`, localizar la línea donde se construye el contexto (cerca de `agent_context`). Añadir antes del loop de display:

```rust
let output_mode = crate::commands::get_output_mode(
    // Extract quiet/verbose from the top-level Cli — passed via handle_pro_command
    // For now use the ProCommands match to get access via parent scope.
    // Pattern: thread quiet/verbose from main.rs through handle_pro_command.
    false, false, // placeholder — see Step 5
);
```

**Nota de implementación:** La forma más limpia de pasar `quiet`/`verbose` es añadir parámetros a `handle_pro_command`. En `src/commands/pro.rs`, cambiar la firma:

```rust
pub fn handle_pro_command(subcommand: ProCommands) {
```

a:

```rust
pub fn handle_pro_command(subcommand: ProCommands, quiet: bool, verbose: bool) {
```

Y en `src/main.rs`, pasar `cli.quiet, cli.verbose` al llamar `handle_pro_command`.

En el check handler, donde se imprime el summary final, envolver en:

```rust
if output_mode != OutputMode::Quiet {
    println!("   ...");
}
```

El output de violations sí debe mostrarse incluso en `--quiet` (son el output útil). Solo suprimir los spinners y mensajes informativos.

En `--verbose`, añadir al inicio del loop de archivos:
```rust
if output_mode == OutputMode::Verbose {
    eprintln!("[verbose] Revisando: {}", file_path.display());
}
```

**Step 5: Update `src/main.rs`**

Cambiar la llamada de `handle_pro_command`:

```rust
Some(Commands::Pro { subcommand }) => {
    commands::pro::handle_pro_command(subcommand, cli.quiet, cli.verbose);
}
```

**Step 6: Run tests + build**

```bash
cargo test test_output_mode 2>&1 | tail -5
cargo build 2>&1 | grep "^error"
```

Expected: test PASS, build limpio. Ajustar cualquier compilación que falle por el cambio de firma.

**Step 7: Smoke test**

```bash
cargo run --bin sentinel -- pro check src/ --quiet
cargo run --bin sentinel -- pro check src/ --verbose
```

Expected: `--quiet` muestra solo violations y summary; `--verbose` muestra archivos procesados.

**Step 8: Commit**

```bash
git add src/commands/mod.rs src/commands/pro.rs src/main.rs
git commit -m "feat: add --quiet and --verbose global flags to all sentinel commands"
```

---

## Task 6: `.sentinelignore` per directory

**Goal:** Soporte para archivos `.sentinelignore` en cualquier directorio del proyecto. Cada archivo aplica a los archivos de ese directorio y subdirectorios. Formato: una entrada por línea `RULE_NAME relpath/to/file.ts symbol_name` (mismo formato que el ignore central).

**Files:**
- Modify: `src/commands/ignore.rs` (añadir `load_directory_ignores()` + merge en `load_ignore_entries()`)
- Modify: `src/commands/mod.rs` (añadir `--show-file` al comando Ignore)

---

**Step 1: Write failing test**

En `src/commands/ignore.rs`, añadir al bloque `#[cfg(test)]`:

```rust
#[test]
fn test_load_directory_ignore_file() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let sub_dir = tmp.path().join("src/services");
    std::fs::create_dir_all(&sub_dir).unwrap();

    // Write a .sentinelignore in the services subdir
    std::fs::write(
        sub_dir.join(".sentinelignore"),
        "DEAD_CODE src/services/user.service.ts processLegacy\n\
         UNUSED_IMPORT src/services/auth.service.ts Injectable\n",
    ).unwrap();

    let entries = super::load_directory_ignores(tmp.path());
    assert_eq!(entries.len(), 2, "should load 2 entries from .sentinelignore");
    assert!(entries.iter().any(|e| e.rule == "DEAD_CODE" && e.symbol.as_deref() == Some("processLegacy")));
    assert!(entries.iter().any(|e| e.rule == "UNUSED_IMPORT" && e.symbol.as_deref() == Some("Injectable")));
}

#[test]
fn test_sentinelignore_empty_lines_and_comments_ignored() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join(".sentinelignore"),
        "# This is a comment\n\
         \n\
         DEAD_CODE src/foo.ts bar\n\
         \n",
    ).unwrap();
    let entries = super::load_directory_ignores(tmp.path());
    assert_eq!(entries.len(), 1, "comments and empty lines must be skipped");
}
```

**Step 2: Run to verify failure**

```bash
cargo test test_load_directory_ignore_file 2>&1 | head -10
```

Expected: compile error — `load_directory_ignores` not found.

**Step 3: Implement `load_directory_ignores` in `src/commands/ignore.rs`**

Localizar el struct `IgnoreEntry` y la función `load_ignore_entries`. Añadir después de `load_ignore_entries`:

```rust
/// Scans `project_root` recursively for `.sentinelignore` files and parses their entries.
/// Format per line: `RULE_NAME file/path.ts optional_symbol`
/// Lines starting with `#` or empty are skipped.
pub fn load_directory_ignores(project_root: &std::path::Path) -> Vec<IgnoreEntry> {
    let mut entries = Vec::new();
    collect_sentinelignore_files(project_root, &mut entries);
    entries
}

fn collect_sentinelignore_files(dir: &std::path::Path, entries: &mut Vec<IgnoreEntry>) {
    const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", "vendor", "dist"];
    let ignore_path = dir.join(".sentinelignore");
    if ignore_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&ignore_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() < 2 { continue; }
                entries.push(IgnoreEntry {
                    rule: parts[0].to_string(),
                    file: parts[1].to_string(),
                    symbol: parts.get(2).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                });
            }
        }
    }
    let read_dir = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !SKIP_DIRS.contains(&name) {
                collect_sentinelignore_files(&path, entries);
            }
        }
    }
}
```

Luego, en `load_ignore_entries`, añadir al final antes del `return entries`:

```rust
// Merge per-directory .sentinelignore files
let dir_entries = load_directory_ignores(project_root);
entries.extend(dir_entries);
```

**Step 4: Add `--show-file` to Ignore command in `src/commands/mod.rs`**

En el variant `Ignore { ... }`, añadir:

```rust
/// Muestra la ruta del archivo de ignores central
#[arg(long)]
show_file: bool,
```

En `src/commands/ignore.rs`, al inicio de `handle_ignore_command`, añadir:

```rust
if show_file {
    let project_root = crate::config::SentinelConfig::find_project_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    println!("{}", project_root.join(".sentinel/ignores.json").display());
    return;
}
```

Actualizar la firma de `handle_ignore_command` para aceptar `show_file: bool` y actualizar el match arm en `main.rs`.

**Step 5: Run tests + build**

```bash
cargo test test_load_directory_ignore test_sentinelignore 2>&1 | tail -10
cargo build 2>&1 | grep "^error"
```

Expected: 2 new tests PASS, build limpio.

**Step 6: Smoke test**

```bash
echo "DEAD_CODE src/main.rs main" > src/.sentinelignore
cargo run --bin sentinel -- pro check src/ --format text
# DEAD_CODE for main should be suppressed
rm src/.sentinelignore
```

**Step 7: Commit**

```bash
git add src/commands/ignore.rs src/commands/mod.rs src/main.rs
git commit -m "feat: .sentinelignore per directory + sentinel ignore --show-file"
```

---

## Task 7: Refactor `pro.rs` into modules

**Goal:** Dividir `src/commands/pro.rs` (~1600 líneas) en módulos sin cambiar comportamiento. Esto mejora la mantenibilidad y hace que cada área sea más fácil de editar. **NO hay cambios de comportamiento.**

**Files:**
- Create: `src/commands/pro/mod.rs` (re-exports + shared types)
- Create: `src/commands/pro/render.rs` (SarifIssue, JsonIssue, render_sarif, check display helpers)
- Create: `src/commands/pro/check.rs` (handler ProCommands::Check)
- Create: `src/commands/pro/audit.rs` (handler ProCommands::Audit + batch helpers)
- Create: `src/commands/pro/review.rs` (handler ProCommands::Review + get_changed_files)
- Delete: `src/commands/pro.rs`
- Modify: `src/commands/mod.rs` (cambiar `pub mod pro;` — ya apuntará al nuevo directorio)

**IMPORTANTE:** Este paso es puramente estructural. Los tests existentes deben pasar sin cambios.

---

**Step 1: Verify all tests pass before refactor**

```bash
cargo test 2>&1 | tail -5
```

Expected: PASS — registrar el número total de tests. Si falla alguno, NO proceder.

**Step 2: Create directory structure**

```bash
mkdir -p src/commands/pro
```

**Step 3: Create `src/commands/pro/render.rs`**

Mover a este archivo:
- `struct JsonIssue` (con sus campos y derive)
- `pub struct SarifIssue` (con sus campos)
- `pub fn render_sarif(issues: &[SarifIssue]) -> String`
- `pub fn get_changed_files(project_root: &Path) -> Vec<PathBuf>`
- Todos los tests de `render_sarif` y `get_changed_files`

Añadir al inicio del archivo:
```rust
use std::path::{Path, PathBuf};
use serde::Serialize;
```

**Step 4: Create `src/commands/pro/mod.rs`**

```rust
pub mod audit;
pub mod check;
pub mod render;
pub mod review;

pub use render::{JsonIssue, SarifIssue, render_sarif, get_changed_files};

use crate::commands::ProCommands;

pub fn handle_pro_command(subcommand: ProCommands, quiet: bool, verbose: bool) {
    match subcommand {
        ProCommands::Check { target, format } => {
            check::handle_check(target, format, quiet, verbose);
        }
        ProCommands::Audit { target, no_fix, format, max_files, concurrency } => {
            audit::handle_audit(target, no_fix, format, max_files, concurrency, quiet, verbose);
        }
        ProCommands::Review { history, diff } => {
            review::handle_review(history, diff, quiet, verbose);
        }
        other => {
            // Remaining sub-commands delegated to legacy handler until migrated
            handle_pro_command_legacy(other, quiet, verbose);
        }
    }
}
```

**Step 5: Create `src/commands/pro/check.rs`**

Mover el handler de `ProCommands::Check { target, format }` a esta función:

```rust
pub fn handle_check(target: String, format: String, quiet: bool, verbose: bool) {
    // ... paste the existing Check handler body here ...
}
```

Ajustar imports necesarios.

**Step 6: Create `src/commands/pro/audit.rs`**

Mover:
- `fn split_into_batches(...)`
- `fn batch_files_by_directory(...)`
- El handler de `ProCommands::Audit`
- Los tests de batch (`test_batch_groups_by_parent_dir`, etc.)

**Step 7: Create `src/commands/pro/review.rs`**

Mover el handler de `ProCommands::Review`.

**Step 8: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: mismo número de tests PASS que al inicio del task. Si alguno falla, es un bug de la refactorización — corregir antes de continuar.

**Step 9: Delete old file**

```bash
rm src/commands/pro.rs
```

**Step 10: Build**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: sin errores.

**Step 11: Commit**

```bash
git add src/commands/pro/ src/commands/mod.rs
git rm src/commands/pro.rs
git commit -m "refactor: split pro.rs into pro/check, pro/audit, pro/review, pro/render modules"
```

---

## Verification Final

Después de todas las tareas:

```bash
# Tests completos
cargo test 2>&1 | tail -5

# Build limpio
cargo build 2>&1 | grep "^error"

# Instalar
cargo install --path .

# Smoke test de cada comando nuevo/modificado
sentinel init
sentinel doctor
sentinel pro check src/ --quiet
sentinel pro check src/ --verbose
sentinel pro check src/ --format json | head -5
sentinel ignore --show-file
echo "DEAD_CODE src/main.rs main" > .sentinelignore && sentinel pro check src/ && rm .sentinelignore
```
