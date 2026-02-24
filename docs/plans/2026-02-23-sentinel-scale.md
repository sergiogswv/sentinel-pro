# Sentinel Scale — Review Context, Index Robustness, Ignore Loop, Audit Parallelism

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Four independent improvements: review sees real architecture via index, `sentinel index` manages the index explicitly, `sentinel ignore` persists false-positive suppression, and audit batches run in parallel with retries.

**Architecture:** All changes are surgical and additive. Tasks 1-5 are loosely coupled — Task 1 (DB methods) is a prerequisite for Tasks 2 and 3. Tasks 4 and 5 are fully independent. Each task has its own test coverage.

**Tech Stack:** Rust std, rusqlite (already in use), tokio::task::JoinSet (already in tokio::full), serde_json (already in use), colored (already in use), ignore crate (already in use).

---

## Task 1: IndexDb — admin + query methods

**Files:**
- Modify: `src/index/db.rs` (after `is_populated()` at line 132)

**Context:** `IndexDb` currently has `open()`, `lock()`, and `is_populated()`. This task adds 5 new methods needed by Tasks 2 and 3. All follow the same pattern: `self.lock()` → prepare stmt → return data.

**Step 1: Add the 5 methods**

In `src/index/db.rs`, insert after the closing `}` of `is_populated()` (after line 132, before the `}` that closes `impl IndexDb`):

```rust
    /// Top N symbols: (name, kind, file_path, line_start)
    pub fn get_symbols(&self, limit: usize) -> Vec<(String, String, String, i64)> {
        let conn = self.lock();
        let mut stmt = match conn.prepare(
            "SELECT name, kind, file_path, line_start FROM symbols ORDER BY file_path, kind LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3).unwrap_or(0),
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    /// Top N call relationships: (caller_file, caller_symbol, callee_symbol)
    pub fn get_call_graph(&self, limit: usize) -> Vec<(String, String, String)> {
        let conn = self.lock();
        let mut stmt = match conn.prepare(
            "SELECT caller_file, caller_symbol, callee_symbol FROM call_graph LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    /// Top N active imports: (file_path, import_name, import_src)
    pub fn get_import_usage(&self, limit: usize) -> Vec<(String, String, String)> {
        let conn = self.lock();
        let mut stmt = match conn.prepare(
            "SELECT file_path, import_name, import_src FROM import_usage WHERE is_used = 1 LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    /// Clears all index tables (for --rebuild). Does NOT drop the tables.
    pub fn clear_all(&self) -> rusqlite::Result<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM symbols", [])?;
        conn.execute("DELETE FROM call_graph", [])?;
        conn.execute("DELETE FROM import_usage", [])?;
        conn.execute("DELETE FROM file_index", [])?;
        Ok(())
    }

    /// Number of files currently in the index.
    pub fn indexed_file_count(&self) -> usize {
        let conn = self.lock();
        conn.query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get::<_, i64>(0))
            .map(|v| v as usize)
            .unwrap_or(0)
    }
```

**Step 2: Add unit tests**

In `src/index/db.rs`, inside `#[cfg(test)] mod tests { ... }`, add after the existing tests:

```rust
    #[test]
    fn test_clear_all_empties_index() {
        let (_f, db) = make_db();
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO file_index (file_path, content_hash) VALUES (?, ?)",
                rusqlite::params!["src/a.ts", "hash1"],
            )
            .unwrap();
        }
        assert!(db.is_populated());
        db.clear_all().unwrap();
        assert!(!db.is_populated());
    }

    #[test]
    fn test_indexed_file_count() {
        let (_f, db) = make_db();
        assert_eq!(db.indexed_file_count(), 0);
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO file_index (file_path, content_hash) VALUES (?, ?)",
                rusqlite::params!["src/a.ts", "h1"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO file_index (file_path, content_hash) VALUES (?, ?)",
                rusqlite::params!["src/b.ts", "h2"],
            )
            .unwrap();
        }
        assert_eq!(db.indexed_file_count(), 2);
    }
```

**Step 3: Run tests**

```bash
cargo test 2>&1 | tail -5
```

Expected: `test result: ok. 39 passed` (37 existing + 2 new)

---

## Task 2: Review — architectural context from index

**Files:**
- Modify: `src/agents/base.rs` (add `build_architectural_context` after `build_rag_context` at line 73)
- Modify: `src/commands/pro.rs` (inject context in the Review task, ~line 1603)

**Context:** `AgentContext::build_rag_context` (lines 45-73 in base.rs) builds file-specific context. The new `build_architectural_context` builds *project-wide* context — no file_path argument. It's called once in the Review handler and prepended to the task context string.

**Step 1: Add build_architectural_context to AgentContext**

In `src/agents/base.rs`, insert after the closing `}` of `build_rag_context` (after line 73):

```rust
    /// Builds a project-wide architectural context block from the SQLite index.
    /// Used by the Review handler to give the LLM structural insight beyond code samples.
    /// Returns empty string if the index is empty or unavailable.
    pub fn build_architectural_context(&self) -> String {
        let Some(ref db) = self.index_db else {
            return String::new();
        };

        let symbols = db.get_symbols(200);
        let calls = db.get_call_graph(100);
        let imports = db.get_import_usage(100);

        if symbols.is_empty() && calls.is_empty() && imports.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("\n=== CONTEXTO ARQUITECTURAL (del índice) ===\n");

        if !symbols.is_empty() {
            ctx.push_str(&format!("Símbolos exportados ({}):\n", symbols.len()));
            for (name, kind, file, line) in &symbols {
                ctx.push_str(&format!("  {} [{}] → {}:{}\n", name, kind, file, line + 1));
            }
        }

        if !calls.is_empty() {
            ctx.push_str(&format!("\nRelaciones de llamada ({}):\n", calls.len()));
            for (caller_file, caller_sym, callee_sym) in &calls {
                ctx.push_str(&format!(
                    "  {} → {} (via {})\n",
                    caller_file, callee_sym, caller_sym
                ));
            }
        }

        if !imports.is_empty() {
            ctx.push_str(&format!("\nImports activos ({}):\n", imports.len()));
            for (file, import_name, import_src) in &imports {
                ctx.push_str(&format!("  {} ← {} (from {})\n", file, import_name, import_src));
            }
        }

        ctx
    }
```

**Step 2: Inject arch context in the Review handler**

In `src/commands/pro.rs`, find the task context format string (around line 1603):

```rust
                context: Some(format!(
                    "ESTADÍSTICAS:\nArchivos escaneados: {}\n\nESTRUCTURA DE DIRECTORIOS:\n{}\n\nSTACK TECNOLÓGICO (Dependencias):\n{}\n\nMUESTRA DE CÓDIGO FUENTE (para análisis concreto):\n{}",
                    file_count, project_tree, deps_list, codigo_muestra
                )),
```

Replace with:

```rust
                context: Some({
                    let arch_ctx = agent_context.build_architectural_context();
                    format!(
                        "ESTADÍSTICAS:\nArchivos escaneados: {}\n\nESTRUCTURA DE DIRECTORIOS:\n{}\n\nSTACK TECNOLÓGICO (Dependencias):\n{}{}\n\nMUESTRA DE CÓDIGO FUENTE (para análisis concreto):\n{}",
                        file_count, project_tree, deps_list, arch_ctx, codigo_muestra
                    )
                }),
```

**Step 3: Build and test**

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | tail -3
```

Expected: no errors, `39 passed`.

---

## Task 3: `sentinel index --rebuild --check` + stale detection

**Files:**
- Modify: `src/commands/mod.rs` (add Commands::Index, add `pub mod index`)
- Create: `src/commands/index.rs`
- Modify: `src/main.rs` (wire Commands::Index)
- Modify: `src/commands/pro.rs` (stale check before `match subcommand`)

**Context:** This is a new top-level command `sentinel index`. It loads config independently (same 3 lines as pro.rs does). The stale check in pro.rs runs before the match block and warns once if the index is significantly behind disk file count.

**Step 1: Add Commands::Index to mod.rs**

In `src/commands/mod.rs`, add `pub mod index;` on line 1 (after existing pub mod declarations), and add the `Index` variant to `Commands`:

Find:
```rust
pub mod monitor;
pub mod pro;
```

Replace with:
```rust
pub mod index;
pub mod monitor;
pub mod pro;
```

Find the `Commands` enum:
```rust
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
```

Replace with:
```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Inicia el modo monitor (comportamiento clásico)
    Monitor,
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
```

**Step 2: Create src/commands/index.rs**

```rust
use crate::config::SentinelConfig;
use crate::index::{IndexDb, ProjectIndexBuilder};
use colored::Colorize;
use std::sync::Arc;

pub fn handle_index_command(rebuild: bool, check: bool) {
    let project_root = std::env::current_dir().unwrap();
    let config = SentinelConfig::load_or_default(&project_root);
    let index_path = project_root.join(".sentinel/index.db");
    let index_db = IndexDb::open(&index_path).ok().map(Arc::new);

    let Some(db) = index_db else {
        println!(
            "{} No se encontró el directorio .sentinel. Corre `sentinel pro check` primero.",
            "❌".red()
        );
        return;
    };

    if !rebuild && !check {
        println!("Uso: sentinel index --check | --rebuild");
        return;
    }

    if check {
        print_index_status(&db, &project_root, &config.file_extensions);
    }

    if rebuild {
        println!("\n{}", "🔄 Reconstruyendo índice desde cero...".bold());
        db.clear_all().expect("Error limpiando el índice");
        let builder = ProjectIndexBuilder::new(Arc::clone(&db));
        builder
            .index_project(&project_root, &config.file_extensions)
            .expect("Error indexando el proyecto");
        let count = db.indexed_file_count();
        println!(
            "{} Índice reconstruido. {} archivos indexados.",
            "✅".green(),
            count.to_string().cyan()
        );
    }
}

fn print_index_status(db: &IndexDb, project_root: &std::path::Path, extensions: &[String]) {
    let disk_count = count_project_files(project_root, extensions);
    let index_count = db.indexed_file_count();
    let diff = (disk_count as isize - index_count as isize).unsigned_abs();
    let stale_threshold = 5.max(disk_count / 10);
    let stale = diff > stale_threshold;

    let conn = db.lock();
    let last_indexed: Option<String> = conn
        .query_row("SELECT MAX(last_indexed) FROM file_index", [], |row| row.get(0))
        .ok()
        .flatten();
    drop(conn);

    println!("\n{}", "📊 Estado del índice:".bold());
    println!("   Archivos indexados:  {}", index_count.to_string().cyan());
    if disk_count > index_count {
        println!(
            "   Archivos en disco:   {}",
            format!("{}  (+{} no indexados)", disk_count, diff)
                .yellow()
                .to_string()
        );
    } else {
        println!("   Archivos en disco:   {}", disk_count.to_string().green());
    }
    println!(
        "   Último indexado:     {}",
        last_indexed.unwrap_or_else(|| "nunca".to_string())
    );
    println!(
        "   Estado:              {}",
        if stale {
            "⚠️  Desactualizado".yellow().to_string()
        } else {
            "✅ Al día".green().to_string()
        }
    );
    if stale {
        println!(
            "\n   Corre {} para actualizar.",
            "`sentinel index --rebuild`".cyan()
        );
    }
}

pub fn count_project_files(root: &std::path::Path, extensions: &[String]) -> usize {
    ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| extensions.contains(&x.to_string()))
                    .unwrap_or(false)
        })
        .count()
}
```

**Step 3: Wire Commands::Index in main.rs**

In `src/main.rs`, add the import and the match arm:

Find:
```rust
use commands::{Cli, Commands};
```

Replace with:
```rust
use commands::{Cli, Commands};
```
(no change needed to imports — Commands is already imported)

Find the match block:
```rust
    match cli.command {
        Some(Commands::Monitor) => {
            commands::monitor::start_monitor();
        }
        Some(Commands::Pro { subcommand }) => {
            commands::pro::handle_pro_command(subcommand);
        }
        None => {
            // Comportamiento por defecto (legacy)
            commands::monitor::start_monitor();
        }
    }
```

Replace with:
```rust
    match cli.command {
        Some(Commands::Monitor) => {
            commands::monitor::start_monitor();
        }
        Some(Commands::Index { rebuild, check }) => {
            commands::index::handle_index_command(rebuild, check);
        }
        Some(Commands::Pro { subcommand }) => {
            commands::pro::handle_pro_command(subcommand);
        }
        None => {
            // Comportamiento por defecto (legacy)
            commands::monitor::start_monitor();
        }
    }
```

**Step 4: Add stale check in pro.rs before match subcommand**

In `src/commands/pro.rs`, add this import at the top of the file (with the other `use crate::commands::` imports — search for a good spot near the top):

Find:
```rust
use crate::index::IndexDb;
use crate::index::ProjectIndexBuilder;
```

Replace with:
```rust
use crate::commands::index::count_project_files;
use crate::index::IndexDb;
use crate::index::ProjectIndexBuilder;
```

Then, in `handle_pro_command`, find the block just before `match subcommand {` (after the `index_handle` spawn block, around line 180):

```rust
    match subcommand {
```

Insert before it:

```rust
    // Stale-index warning: warn once if disk file count diverges significantly from index
    if !json_mode_global {
        if let Some(ref db) = agent_context.index_db {
            if db.is_populated() {
                let disk_count =
                    count_project_files(&agent_context.project_root, &agent_context.config.file_extensions);
                let index_count = db.indexed_file_count();
                let diff = (disk_count as isize - index_count as isize).unsigned_abs();
                let stale_threshold = 5.max(disk_count / 10);
                if diff > stale_threshold {
                    println!(
                        "\n{} {} ({} indexados, {} en disco).",
                        "⚠️".yellow(),
                        "Índice posiblemente desactualizado".yellow(),
                        index_count,
                        disk_count
                    );
                    println!(
                        "   Corre {} para actualizar.\n",
                        "`sentinel index --rebuild`".cyan()
                    );
                }
            }
        }
    }

    match subcommand {
```

**Step 5: Build and test**

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | tail -3
```

Expected: no errors, `39 passed`.

---

## Task 4: `sentinel ignore` — false-positive suppression

**Files:**
- Modify: `src/commands/mod.rs` (add Commands::Ignore + pub mod ignore)
- Create: `src/commands/ignore.rs`
- Modify: `src/commands/pro.rs` (load ignore list + filter violations in check handler)

**Context:** `.sentinel/ignore.json` is a JSON file in the project's `.sentinel/` directory. The check handler already has a `violations` vector of `RuleViolation` structs (which have `rule_name: String`, `file_path: String` via the engine, and `symbol: Option<String>` added in the precision pass). The filter runs after `violations` are collected, before display.

**Step 1: Add Commands::Ignore to mod.rs**

In `src/commands/mod.rs`, add `pub mod ignore;`:

Find:
```rust
pub mod index;
pub mod monitor;
pub mod pro;
```

Replace with:
```rust
pub mod ignore;
pub mod index;
pub mod monitor;
pub mod pro;
```

Add the `Ignore` variant to the `Commands` enum. Find:
```rust
    /// Gestión del índice de símbolos y call graph
    Index {
```

Insert before it:
```rust
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
```

**Step 2: Create src/commands/ignore.rs**

```rust
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreEntry {
    pub rule: String,
    pub file: String,
    pub symbol: Option<String>,
    pub added: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct IgnoreFile {
    version: u32,
    entries: Vec<IgnoreEntry>,
}

fn ignore_path(project_root: &Path) -> std::path::PathBuf {
    project_root.join(".sentinel/ignore.json")
}

pub fn load_ignore_entries(project_root: &Path) -> Vec<IgnoreEntry> {
    let path = ignore_path(project_root);
    if !path.exists() {
        return vec![];
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str::<IgnoreFile>(&content)
        .map(|f| f.entries)
        .unwrap_or_default()
}

fn save_ignore_entries(project_root: &Path, entries: Vec<IgnoreEntry>) {
    let path = ignore_path(project_root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = IgnoreFile {
        version: 1,
        entries,
    };
    let json = serde_json::to_string_pretty(&file).unwrap_or_default();
    let _ = std::fs::write(&path, json);
}

pub fn handle_ignore_command(
    rule: Option<String>,
    file: Option<String>,
    symbol: Option<String>,
    list: bool,
    clear: Option<String>,
) {
    let project_root = std::env::current_dir().unwrap();
    let mut entries = load_ignore_entries(&project_root);

    if list {
        if entries.is_empty() {
            println!("No hay ignores activos.");
        } else {
            println!("\n{}", "Ignores activos:".bold());
            for e in &entries {
                let sym = e.symbol.as_deref().unwrap_or("*");
                println!("  {} {} {}", e.rule.cyan(), e.file, sym.dimmed());
            }
        }
        return;
    }

    if let Some(ref clear_file) = clear {
        let before = entries.len();
        entries.retain(|e| &e.file != clear_file);
        let removed = before - entries.len();
        save_ignore_entries(&project_root, entries);
        println!("{} {} ignore(s) eliminados para '{}'.", "✅".green(), removed, clear_file);
        return;
    }

    let (Some(rule), Some(file)) = (rule, file) else {
        println!("Uso: sentinel ignore <REGLA> <ARCHIVO> [--symbol <SÍMBOLO>]");
        println!("     sentinel ignore --list");
        println!("     sentinel ignore --clear <ARCHIVO>");
        return;
    };

    // Check for duplicate
    let already = entries.iter().any(|e| {
        e.rule == rule && e.file == file && e.symbol == symbol
    });
    if already {
        println!("{} Ya existe ese ignore.", "ℹ️".cyan());
        return;
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    entries.push(IgnoreEntry {
        rule: rule.clone(),
        file: file.clone(),
        symbol: symbol.clone(),
        added: today,
    });
    save_ignore_entries(&project_root, entries);

    let sym_str = symbol.as_deref().map(|s| format!(" (símbolo: {})", s)).unwrap_or_default();
    println!(
        "{} Ignorando {} en {}{} en próximas ejecuciones.",
        "✅".green(),
        rule.cyan(),
        file,
        sym_str
    );
}
```

**Step 3: Wire Commands::Ignore in main.rs**

In `src/main.rs`, add the Ignore arm to the match:

Find:
```rust
        Some(Commands::Index { rebuild, check }) => {
            commands::index::handle_index_command(rebuild, check);
        }
```

Insert before it:
```rust
        Some(Commands::Ignore { rule, file, symbol, list, clear }) => {
            commands::ignore::handle_ignore_command(rule, file, symbol, list, clear);
        }
```

**Step 4: Add chrono import to ignore.rs**

`chrono` is already in Cargo.toml. The `use chrono::Utc;` is used in the handler. Add at the top of `src/commands/ignore.rs`:

The `chrono::Utc` is used inline in `handle_ignore_command` — it's already referenced as `chrono::Utc::now()` which works without a `use` statement since it's a fully-qualified path.

**Step 5: Filter violations in check handler**

In `src/commands/pro.rs`, add the import at the top:

Find:
```rust
use crate::commands::index::count_project_files;
```

Replace with:
```rust
use crate::commands::ignore::load_ignore_entries;
use crate::commands::index::count_project_files;
```

Then, in the Check handler, find the code that collects violations and displays them. The violations are collected per-file in a loop; after collecting all violations for all files, before displaying them, add the filter.

Search for a comment or line that marks "after collecting all violations". In the check handler, violations are stored in a `Vec<RuleViolation>` (likely called `violations` or accumulated). Find the pattern where violations are displayed (search for the icon/display loop):

Look for where violations iterate for display — something like:
```rust
for v in &violations {
```

Just before that loop, insert (replace the existing display loop start):

```rust
            // Load and apply ignore list
            let ignore_entries = load_ignore_entries(&agent_context.project_root);
            violations.retain(|v| {
                !ignore_entries.iter().any(|e| {
                    e.rule == v.rule_name
                        && (v.file_path.contains(&e.file) || e.file.contains(&v.file_path))
                        && e.symbol
                            .as_ref()
                            .map(|s| v.symbol.as_deref() == Some(s.as_str()))
                            .unwrap_or(true) // if no symbol in entry, match any symbol
                })
            });
```

**Important:** Read the check handler in `src/commands/pro.rs` carefully (around lines 197-430) to find the exact location of the violations accumulation and display. The `violations` variable is accumulated per-file in a for loop. The filter should be applied after the per-file loop ends and before the display block.

After the violations are displayed (in text mode), at the very end of the non-empty violations display, add the hint:

```rust
                    println!(
                        "\n💡 Para ignorar: {} {} {}",
                        "sentinel ignore".cyan(),
                        "<REGLA>".dimmed(),
                        "<ARCHIVO>".dimmed()
                    );
```

**Step 6: Add unit test for ignore filtering logic**

In `src/commands/pro.rs`, inside `#[cfg(test)] mod batching_tests`, add:

```rust
    #[test]
    fn test_ignore_entries_filter() {
        use crate::commands::ignore::IgnoreEntry;

        struct FakeViolation {
            rule_name: String,
            file_path: String,
            symbol: Option<String>,
        }

        let violations = vec![
            FakeViolation { rule_name: "DEAD_CODE".into(), file_path: "src/user.ts".into(), symbol: Some("userId".into()) },
            FakeViolation { rule_name: "DEAD_CODE".into(), file_path: "src/user.ts".into(), symbol: Some("getUser".into()) },
            FakeViolation { rule_name: "UNUSED_IMPORT".into(), file_path: "src/auth.ts".into(), symbol: None },
        ];

        let entries = vec![
            IgnoreEntry { rule: "DEAD_CODE".into(), file: "src/user.ts".into(), symbol: Some("userId".into()), added: "2026-02-23".into() },
        ];

        let kept: Vec<_> = violations.iter().filter(|v| {
            !entries.iter().any(|e| {
                e.rule == v.rule_name
                    && (v.file_path.contains(&e.file) || e.file.contains(&v.file_path))
                    && e.symbol
                        .as_ref()
                        .map(|s| v.symbol.as_deref() == Some(s.as_str()))
                        .unwrap_or(true)
            })
        }).collect();

        // userId should be filtered out; getUser and UNUSED_IMPORT should remain
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].symbol.as_deref(), Some("getUser"));
        assert_eq!(kept[1].rule_name, "UNUSED_IMPORT");
    }
```

**Step 7: Build and test**

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | tail -3
```

Expected: no errors, `40 passed` (39 + 1 new).

---

## Task 5: Audit `--concurrency` + retries

**Files:**
- Modify: `src/commands/mod.rs` (add `--concurrency` to `ProCommands::Audit`)
- Modify: `src/commands/pro.rs` (parallel batch loop with `JoinSet` + retry logic)

**Context:** The current audit batch loop (lines ~1900-2010 in pro.rs) calls `rt.block_on(orchestrator.execute_task(...))` sequentially. We replace it with a single `rt.block_on(async { ... })` block that uses `tokio::task::JoinSet` for bounded parallelism. Each task creates its own `AgentContext` from `Arc`-cloned fields and runs up to 3 attempts.

`ReviewerAgent` is a unit struct (`pub struct ReviewerAgent;`) — cheap to instantiate per task. It's `Send + Sync`. `AgentContext` fields are all `Arc`-wrapped or `Clone` → each spawned task owns its own `AgentContext`.

**Step 1: Add --concurrency flag to ProCommands::Audit**

In `src/commands/mod.rs`, find:

```rust
    Audit {
        /// Arquivo ou pasta a auditar
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
    },
```

Replace with:

```rust
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
```

**Step 2: Update the match arm in pro.rs**

In `src/commands/pro.rs`, find the match arm (line ~1822):

```rust
        ProCommands::Audit { target, no_fix, format, max_files } => {
```

Replace with:

```rust
        ProCommands::Audit { target, no_fix, format, max_files, concurrency } => {
```

**Step 3: Add necessary imports in pro.rs**

In `src/commands/pro.rs`, find the existing imports block (near the top, around lines 1-20). Add:

```rust
use crate::agents::reviewer::ReviewerAgent;
use crate::agents::base::{Agent, Task as AgentTask, TaskType, AgentContext as ProAgentContext};
```

Wait — check what's already imported. Search for existing `use` statements in pro.rs for Agent-related types. The pro.rs already uses `AgentContext`, `Task`, `TaskType`, `ReviewerAgent` etc. Read the top of pro.rs to see exact imports before adding duplicates.

**Step 4: Replace the sequential batch loop with parallel JoinSet loop**

In `src/commands/pro.rs`, find the sequential batch loop (starts around line 1900):

```rust
            for (batch_idx, batch_files) in final_batches.iter().enumerate() {
                // Construir contexto multi-archivo para el batch
                let mut batch_context = String::new();
                let mut batch_rel_paths: Vec<String> = Vec::new();

                for file_path in batch_files {
                    let rel_path = file_path
                        .strip_prefix(&agent_context.project_root)
                        .unwrap_or(file_path);
                    let content = std::fs::read_to_string(file_path).unwrap_or_default();
                    batch_context.push_str(&format!("\n\n=== {} ===\n{}", rel_path.display(), content));
                    batch_rel_paths.push(rel_path.display().to_string());
                }

                let module_name = batch_files.first()
                    ...

                let pb = if !json_mode { ... } else { ... };

                let task = Task { ... };

                match rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context)) {
                    Ok(res) => {
                        ...
                        all_issues.extend(issues);
                    }
                    Err(e) => {
                        parse_failures += 1;
                    }
                }
                pb.finish_and_clear();
            }
```

Replace the **entire** for loop (from `for (batch_idx, batch_files)` to the closing `}` of the for loop, including `pb.finish_and_clear()`) with:

```rust
            let concurrency = concurrency.clamp(1, 10);
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

            // Pre-build all batch data before entering async context
            struct BatchData {
                batch_idx: usize,
                batch_context: String,
                batch_rel_paths: Vec<String>,
                batch_files: Vec<std::path::PathBuf>,
                module_name: String,
            }

            let mut batch_data_list: Vec<BatchData> = Vec::new();
            for (batch_idx, batch_files) in final_batches.iter().enumerate() {
                let mut batch_context = String::new();
                let mut batch_rel_paths: Vec<String> = Vec::new();
                for file_path in batch_files {
                    let rel_path = file_path
                        .strip_prefix(&agent_context.project_root)
                        .unwrap_or(file_path);
                    let content = std::fs::read_to_string(file_path).unwrap_or_default();
                    batch_context.push_str(&format!(
                        "\n\n=== {} ===\n{}",
                        rel_path.display(),
                        content
                    ));
                    batch_rel_paths.push(rel_path.display().to_string());
                }
                let module_name = batch_files
                    .first()
                    .and_then(|f| f.parent())
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "módulo".to_string());
                batch_data_list.push(BatchData {
                    batch_idx,
                    batch_context,
                    batch_rel_paths,
                    batch_files: batch_files.clone(),
                    module_name,
                });
            }

            let batch_results: Vec<Result<(usize, String, Vec<std::path::PathBuf>), String>> =
                rt.block_on(async {
                    let mut set = tokio::task::JoinSet::new();

                    for bd in batch_data_list {
                        let permit = semaphore.clone().acquire_owned().await.unwrap();
                        let config = std::sync::Arc::clone(&agent_context.config);
                        let stats = std::sync::Arc::clone(&agent_context.stats);
                        let project_root = agent_context.project_root.clone();
                        let index_db = agent_context.index_db.clone();
                        let json_mode_inner = json_mode;

                        set.spawn(async move {
                            let _permit = permit;
                            let ctx = AgentContext {
                                config,
                                stats,
                                project_root,
                                index_db,
                            };
                            let reviewer = ReviewerAgent::new();
                            let task = Task {
                                id: uuid::Uuid::new_v4().to_string(),
                                description: format!(
                                    "Realiza una auditoría técnica de MÚLTIPLES archivos del módulo '{}'.\n\
                                    ARCHIVOS INCLUIDOS: {}\n\
                                    OBJETIVO: Identificar problemas de calidad, seguridad o bugs CORREGIBLES.\n\
                                    REGLAS:\n\
                                    1. Analiza TODOS los archivos y genera un array JSON con los problemas.\n\
                                    2. Cada objeto DEBE tener: title, description, severity (High/Medium/Low), suggested_fix, file_path (nombre del archivo al que pertenece el issue).\n\
                                    3. Responde ÚNICAMENTE con el bloque ```json — sin texto introductorio.\n\
                                    FORMATO JSON REQUERIDO:\n\
                                    ```json\n\
                                    [\n\
                                      {{\"title\": \"...\", \"description\": \"...\", \"severity\": \"High|Medium|Low\", \"suggested_fix\": \"...\", \"file_path\": \"nombre-del-archivo.ts\"}}\n\
                                    ]\n\
                                    ```",
                                    bd.module_name,
                                    bd.batch_rel_paths.join(", ")
                                ),
                                task_type: TaskType::Analyze,
                                file_path: bd.batch_files.first().cloned(),
                                context: Some(bd.batch_context),
                            };

                            // Up to 3 attempts with 2s delay between failures
                            let mut last_err = String::new();
                            for attempt in 0..3usize {
                                match reviewer.execute(&task, &ctx).await {
                                    Ok(res) => {
                                        return Ok((bd.batch_idx, res.output, bd.batch_files));
                                    }
                                    Err(e) => {
                                        last_err = e.to_string();
                                        if attempt < 2 {
                                            tokio::time::sleep(
                                                tokio::time::Duration::from_secs(2),
                                            )
                                            .await;
                                        }
                                    }
                                }
                            }
                            Err(last_err)
                        });

                        if !json_mode_inner {
                            // Progress is shown after each completion below
                        }
                    }

                    let mut results = Vec::new();
                    while let Some(join_result) = set.join_next().await {
                        results.push(join_result.unwrap_or_else(|e| Err(e.to_string())));
                    }
                    results
                });

            // Process results (same normalization logic as before)
            let pb_final = if !json_mode {
                ui::crear_progreso("Procesando resultados...")
            } else {
                indicatif::ProgressBar::hidden()
            };

            for result in batch_results {
                match result {
                    Ok((_batch_idx, output, batch_files)) => {
                        let json_str = crate::ai::utils::extraer_json(&output);
                        match serde_json::from_str::<Vec<AuditIssue>>(&json_str) {
                            Ok(mut issues) => {
                                for issue in &mut issues {
                                    let matched_path = batch_files
                                        .iter()
                                        .find(|f| {
                                            f.to_string_lossy().contains(&issue.file_path)
                                                || issue.file_path.contains(
                                                    &f.file_name()
                                                        .map(|n| n.to_string_lossy().to_string())
                                                        .unwrap_or_default(),
                                                )
                                        })
                                        .map(|f| f.to_string_lossy().to_string())
                                        .unwrap_or_else(|| {
                                            batch_files
                                                .first()
                                                .map(|f| f.to_string_lossy().to_string())
                                                .unwrap_or_default()
                                        });
                                    issue.file_path = matched_path;
                                }
                                all_issues.extend(issues);
                            }
                            Err(_) => {
                                parse_failures += 1;
                            }
                        }
                    }
                    Err(_) => {
                        parse_failures += 1;
                    }
                }
            }

            pb_final.finish_and_clear();
```

**Note:** The old `pb` (per-batch progress bar) is gone. The new code shows a single "Procesando resultados..." bar. This is intentional — with parallel execution, per-batch progress is misleading. If you want per-completion progress, you can use an `Arc<AtomicUsize>` counter, but YAGNI.

**Step 5: Build and fix compile errors**

```bash
cargo build 2>&1 | grep "^error" | head -20
```

There will likely be import errors (duplicate `use` for `ReviewerAgent`, `AgentContext`, `Task`). Read the error messages carefully:
- If `ReviewerAgent` is not imported at the top of pro.rs, add it: look for existing agent imports and add `ReviewerAgent` alongside them.
- The `AgentContext` in the spawn closure conflicts with the local `agent_context` variable name. If there's a name collision, rename the inner one to `task_ctx`.
- Verify `TaskType` is in scope.

Fix each error methodically. Do NOT add duplicate imports.

**Step 6: Run tests**

```bash
cargo test 2>&1 | tail -5
```

Expected: `40 passed` (same as after Task 4 — no new tests for this task).

**Step 7: Smoke test the concurrency flag**

```bash
sentinel pro audit src/ --no-fix --concurrency 2 2>&1 | tail -10
```

Expected: runs without panic, produces audit output.

---

## Final verification

```bash
cargo test 2>&1 | tail -5
cargo build --release 2>&1 | grep "^error"
```

Expected: `40 passed`, no build errors.

### Manual smoke tests

```bash
# Task 3: sentinel index
sentinel index --check
sentinel index --rebuild

# Task 4: sentinel ignore
sentinel ignore DEAD_CODE src/user.service.ts userId
sentinel ignore --list
sentinel ignore --clear src/user.service.ts

# Task 5: audit with concurrency
sentinel pro audit src/ --no-fix --concurrency 4 --max-files 10
```
