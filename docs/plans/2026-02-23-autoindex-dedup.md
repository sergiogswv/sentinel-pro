# Auto-indexación Cold-Start + Deduplicación Audit

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the cold-start "ÍNDICE VACÍO" warning with automatic background indexing, and eliminate duplicate issues in audit output.

**Architecture:** Two independent changes in `src/commands/pro.rs`. (1) Before the `match subcommand` block in `handle_pro_command`, detect empty index → spawn background thread running `ProjectIndexBuilder::index_project`, join after the match. (2) After the audit batch loop, deduplicate `all_issues` in-place using `Vec::retain()` with a `HashSet` seen-set keyed on `(title.to_lowercase(), file_path)`.

**Tech Stack:** Rust std (`std::thread`, `std::collections::HashSet`), `crate::index::ProjectIndexBuilder` (already in codebase at `src/index/builder.rs`), `colored` (already imported)

---

## Task 1: Audit deduplication

**Files:**
- Modify: `src/commands/pro.rs` (after audit batch loop, ~line 2027)

**Context:** `AuditIssue` is defined at lines 19-27 of `pro.rs`:
```rust
struct AuditIssue {
    title: String,
    description: String,
    severity: String,
    suggested_fix: String,
    #[serde(default)]
    file_path: String,
}
```
`all_issues: Vec<AuditIssue>` is accumulated across batch loops. After `pb.finish_and_clear()` at ~line 2027 and before `if all_issues.is_empty()` at ~line 2029, there is currently a blank line. That's where the dedup goes.

**Step 1: Write the failing test**

At the bottom of `src/commands/pro.rs`, inside the existing `#[cfg(test)] mod batching_tests` block (add AFTER the last `test_batch_flat_project_prefix_grouping` test function, before the closing `}`), add:

```rust
    #[test]
    fn test_audit_dedup_removes_duplicates() {
        // Helper to build a minimal AuditIssue
        fn make_issue(title: &str, file_path: &str) -> super::AuditIssue {
            super::AuditIssue {
                title: title.to_string(),
                description: String::new(),
                severity: "high".to_string(),
                suggested_fix: String::new(),
                file_path: file_path.to_string(),
            }
        }

        let mut issues = vec![
            make_issue("Función muy larga", "src/user.service.ts"),  // first occurrence
            make_issue("Función muy larga", "src/user.service.ts"),  // duplicate → removed
            make_issue("función muy larga", "src/user.service.ts"),  // case variant → removed
            make_issue("Función muy larga", "src/auth.service.ts"),  // different file → kept
            make_issue("Import no usado", "src/user.service.ts"),    // different title → kept
        ];

        // --- dedup logic (same as production code) ---
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        issues.retain(|issue| seen.insert((issue.title.to_lowercase(), issue.file_path.clone())));
        // ---

        assert_eq!(issues.len(), 3, "must keep: first occurrence, different file, different title");
        assert_eq!(issues[0].title, "Función muy larga");
        assert_eq!(issues[0].file_path, "src/user.service.ts");
        assert_eq!(issues[1].file_path, "src/auth.service.ts");
        assert_eq!(issues[2].title, "Import no usado");
    }
```

**Step 2: Run test to verify it passes (it tests inline logic, no production code yet)**

```bash
cargo test test_audit_dedup 2>&1 | grep -E "ok|FAILED"
```

Expected: `test batching_tests::test_audit_dedup_removes_duplicates ... ok`

Note: this test validates the dedup algorithm in isolation. The next step wires it into the production path.

**Step 3: Add dedup logic in audit handler**

In `src/commands/pro.rs`, find the blank line between `pb.finish_and_clear();` (~line 2027) and `if all_issues.is_empty()` (~line 2029). Insert:

```rust
            // Deduplicar: misma combinación (título normalizado, archivo) → conservar solo primero
            {
                let mut seen: std::collections::HashSet<(String, String)> =
                    std::collections::HashSet::new();
                all_issues.retain(|issue| {
                    seen.insert((issue.title.to_lowercase(), issue.file_path.clone()))
                });
            }
```

**Step 4: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: `test result: ok. 37 passed` (36 existing + 1 new)

---

## Task 2: Auto-indexación en cold start

**Files:**
- Modify: `src/commands/pro.rs` (imports, setup block ~lines 141-152, per-handler cold-start warnings, after match ~line 2217)

**Context:**

Current `handle_pro_command` structure (simplified):
```
line ~8:    use crate::index::IndexDb;   ← add ProjectIndexBuilder here
line ~141:  let agent_context = AgentContext { ... };
line ~143:  // Inicializar Orquestador y Agentes
            let mut orchestrator = ...
line ~151:  let rt = tokio::runtime::Runtime::new().unwrap();
line ~153:  match subcommand {
              ProCommands::Check { ... } => { ... cold_start warning at ~205 ... }
              ProCommands::Audit { ... } => { ... cold_start warning at ~1818 ... }
              ProCommands::Review       => { ... cold_start warning at ~1415 ... }
              ...
            }   ← line ~2217
}             ← line ~2218
```

Cold-start warnings currently live inside each subcommand handler. They will be REMOVED and replaced by the single startup check.

**Step 1: Add import**

In `src/commands/pro.rs`, find the imports at the top:
```rust
use crate::index::IndexDb;
```

Add on the next line:
```rust
use crate::index::ProjectIndexBuilder;
```

**Step 2: Write the failing compile check**

```bash
cargo build 2>&1 | grep "^error" | head -3
```

Expected: no errors (import is valid, just unused for now — Rust may warn but won't error).

**Step 3: Add json_mode_global + index handle declaration + spawn logic**

In `src/commands/pro.rs`, find the block just before `match subcommand {` (around line 151-153):

```rust
    let rt = tokio::runtime::Runtime::new().unwrap();

    match subcommand {
```

Replace with:

```rust
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Detect JSON mode before dispatching (to suppress indexing messages in JSON output)
    let json_mode_global = match &subcommand {
        ProCommands::Check { format, .. } => format.to_lowercase() == "json",
        ProCommands::Audit { format, .. } => format.to_lowercase() == "json",
        _ => false,
    };

    // Auto-indexación: si el índice está vacío, indexar en background mientras corre el comando
    let mut index_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>> = None;
    if let Some(ref db) = agent_context.index_db {
        if !db.is_populated() {
            if !json_mode_global {
                println!(
                    "\n{} {}",
                    "🧠 Indexando proyecto por primera vez...".cyan(),
                    "(segundo plano)".dimmed()
                );
            }
            let db_clone = Arc::clone(db);
            let root_clone = agent_context.project_root.clone();
            let extensions_clone = agent_context.config.file_extensions.clone();
            index_handle = Some(std::thread::spawn(move || {
                let builder = ProjectIndexBuilder::new(db_clone);
                builder.index_project(&root_clone, &extensions_clone)
            }));
        }
    }

    match subcommand {
```

**Step 4: Add join logic after the match block**

In `src/commands/pro.rs`, find the very end of `handle_pro_command` (lines 2217-2218):

```rust
    }
}
```

Replace with:

```rust
    }

    // Esperar a que termine la indexación background (si fue iniciada)
    if let Some(handle) = index_handle {
        match handle.join() {
            Ok(Ok(_)) => {
                if !json_mode_global {
                    println!(
                        "\n{} {}",
                        "✅".green(),
                        "Índice listo. Próxima ejecución tendrá análisis cross-file completo.".green()
                    );
                }
            }
            _ => {
                if !json_mode_global {
                    println!(
                        "\n{} {}",
                        "⚠️".yellow(),
                        "Error en indexación background (análisis cross-file no disponible).".yellow()
                    );
                }
            }
        }
    }
}
```

**Step 5: Build to verify**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

**Step 6: Remove cold-start warnings from Check handler**

In `src/commands/pro.rs`, find the cold-start block inside the check handler's `if !json_mode` section (~line 205). It looks like:

```rust
                let cold_start = agent_context
                    .index_db
                    .as_ref()
                    .map(|db| !db.is_populated())
                    .unwrap_or(false);
                if cold_start {
                    println!(
                        "\n{} {}",
                        "⚠️  ÍNDICE VACÍO —".yellow().bold(),
                        "Ejecuta `sentinel monitor` primero para análisis cross-file completo.".yellow()
                    );
                    println!(
                        "   {}\n",
                        "Continuando con análisis de archivo único...".yellow()
                    );
                }

                // TS-first note: only shown when index is ready (cold-start takes priority)
                if !cold_start {
                    let has_ts_js = files_to_check.iter().any(|f| {
                        matches!(
                            f.extension().and_then(|e| e.to_str()),
                            Some("ts" | "js" | "tsx" | "jsx")
                        )
                    });
                    if !has_ts_js {
                        println!(
                            "ℹ️  Análisis estático optimizado para TypeScript/JavaScript."
                        );
                        println!(
                            "   Soporte para Go, Python, Rust, Java y otros lenguajes: próxima versión.\n"
                        );
                    }
                }
```

Replace with (remove cold_start warning, simplify TS note — always shown now when no TS/JS files):

```rust
                // TS-first note: shown when no TS/JS files in target
                let has_ts_js = files_to_check.iter().any(|f| {
                    matches!(
                        f.extension().and_then(|e| e.to_str()),
                        Some("ts" | "js" | "tsx" | "jsx")
                    )
                });
                if !has_ts_js {
                    println!(
                        "ℹ️  Análisis estático optimizado para TypeScript/JavaScript."
                    );
                    println!(
                        "   Soporte para Go, Python, Rust, Java y otros lenguajes: próxima versión.\n"
                    );
                }
```

**Step 7: Remove cold-start warning from Review handler**

In `src/commands/pro.rs`, find the Review handler's cold-start block (~line 1415). It looks like:

```rust
            // Review has no --format flag; always terminal output, no json_mode guard needed.
            // Cold-start warning: shown once if index has never been populated
            if let Some(ref db) = agent_context.index_db {
                if !db.is_populated() {
                    println!(
                        "\n{} {}",
                        "⚠️  ÍNDICE VACÍO —".yellow().bold(),
                        "Ejecuta `sentinel monitor` primero para análisis cross-file completo.".yellow()
                    );
                    println!(
                        "   {}\n",
                        "Continuando con análisis de archivo único...".yellow()
                    );
                }
            }
```

Delete the entire block (those ~11 lines).

**Step 8: Remove cold-start warning from Audit handler**

In `src/commands/pro.rs`, find the Audit handler's cold-start block (~line 1818). It looks like:

```rust
            if !json_mode {
                if let Some(ref db) = agent_context.index_db {
                    if !db.is_populated() {
                        println!(
                            "\n{} {}",
                            "⚠️  ÍNDICE VACÍO —".yellow().bold(),
                            "Ejecuta `sentinel monitor` primero para análisis cross-file completo.".yellow()
                        );
                        println!(
                            "   {}\n",
                            "Continuando con análisis de archivo único...".yellow()
                        );
                    }
                }
            }
```

Delete the entire block (those ~13 lines).

**Step 9: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: `test result: ok. 37 passed`

**Step 10: Smoke test**

```bash
# Build and install
cargo install --path . --force 2>&1 | tail -2

# Test on a project that has never had sentinel monitor run
# (create a temp project without .sentinel/)
mkdir -p /tmp/coldtest/src && echo 'export function hello() {}' > /tmp/coldtest/src/hello.ts
cd /tmp/coldtest && sentinel pro check src/ 2>&1 | grep -E "Indexando|Índice listo|cross-file" || echo "MESSAGES NOT SHOWN"
```

Expected output includes:
```
🧠 Indexando proyecto por primera vez... (segundo plano)
✅ Índice listo. Próxima ejecución tendrá análisis cross-file completo.
```

---

## Final verification

```bash
cargo test 2>&1 | tail -5
cargo build --release 2>&1 | grep "^error"
```

Expected: `37 passed`, no build errors.
