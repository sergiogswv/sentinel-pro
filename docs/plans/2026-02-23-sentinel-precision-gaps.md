# Sentinel Precision Gaps Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close four UX/reliability gaps: cold-start warning when index is empty, audit batching unit tests, flat-project prefix grouping, and TypeScript-first documentation.

**Architecture:** Four independent surgical changes: (1) new `IndexDb::is_populated()` method + warning printed once in `handle_pro_command`; (2) extract inline batching logic into a testable `build_audit_batches()` free function with prefix-based grouping; (3) TS-first note in check handler; (4) README language table. No new files needed — all changes in existing files.

**Tech Stack:** Rust, rusqlite, tempfile (already in dev-deps), colored

---

## Task 1: Add `IndexDb::is_populated()`

**Files:**
- Modify: `src/index/db.rs:9-121`

**Context:** `IndexDb` is in `src/index/db.rs`. It wraps a `Mutex<Connection>`. The `call_graph` table is populated by `sentinel monitor`. We need a method that returns `true` if that table has at least one row.

**Step 1: Write the failing test**

At the bottom of `src/index/db.rs`, add a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_db() -> (NamedTempFile, IndexDb) {
        let f = NamedTempFile::new().unwrap();
        let db = IndexDb::open(f.path()).unwrap();
        (f, db)
    }

    #[test]
    fn test_is_populated_false_when_empty() {
        let (_f, db) = make_db();
        assert!(!db.is_populated());
    }

    #[test]
    fn test_is_populated_true_after_call_graph_insert() {
        let (_f, db) = make_db();
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO call_graph (caller_file, caller_symbol, callee_symbol) VALUES (?, ?, ?)",
                rusqlite::params!["src/a.ts", "funcA", "funcB"],
            )
            .unwrap();
        }
        assert!(db.is_populated());
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test test_is_populated -p sentinel-pro 2>&1 | grep -E "FAILED|error\[|cannot find"
```

Expected: compile error "no method named `is_populated`".

**Step 3: Implement `is_populated`**

In `src/index/db.rs`, inside `impl IndexDb` block, after the `lock()` method (line ~116), add:

```rust
/// Returns true if the call_graph table has been populated (i.e., `sentinel monitor` has run).
pub fn is_populated(&self) -> bool {
    let conn = self.conn.lock().unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM call_graph",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
    .unwrap_or(false)
}
```

**Step 4: Run tests to verify they pass**

```bash
cargo test test_is_populated 2>&1 | grep -E "ok|FAILED"
```

Expected:
```
test tests::test_is_populated_false_when_empty ... ok
test tests::test_is_populated_true_after_call_graph_insert ... ok
```

**Step 5: Run full suite to check no regressions**

```bash
cargo test 2>&1 | tail -3
```

Expected: `test result: ok. 33 passed`

---

## Task 2: Cold-start warning + `index_populated` in JSON output

**Files:**
- Modify: `src/commands/pro.rs:82-84` (warning placement)
- Modify: `src/commands/pro.rs:210-224` (JSON output struct)

**Context:** `handle_pro_command` builds `agent_context` at lines 77-82, then sets up the orchestrator at lines 84-89. The cold-start warning goes between those two blocks. For JSON mode in `check`, the output struct `JsonOutput` is defined inline at line 210 — it needs a new `index_populated: bool` field.

**Step 1: Add cold-start warning in `handle_pro_command`**

In `src/commands/pro.rs`, find this block (around line 82-84):

```rust
    let agent_context = AgentContext {
        config: Arc::new(config),
        stats,
        project_root,
        index_db,
    };

    // Inicializar Orquestador y Agentes
```

Replace it with:

```rust
    let agent_context = AgentContext {
        config: Arc::new(config),
        stats,
        project_root,
        index_db,
    };

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

    // Inicializar Orquestador y Agentes
```

**Step 2: Add `index_populated` to check's JSON output**

In `src/commands/pro.rs`, find the `JsonOutput` struct (around line 210):

```rust
            if json_mode {
                #[derive(serde::Serialize)]
                struct JsonOutput {
                    checked: usize,
                    errors: usize,
                    warnings: usize,
                    infos: usize,
                    issues: Vec<JsonIssue>,
                }
                let out = JsonOutput {
                    checked: files_to_check.len(),
                    errors: n_errors,
                    warnings: n_warnings,
                    infos: n_infos,
                    issues: json_issues,
                };
```

Replace it with:

```rust
            if json_mode {
                #[derive(serde::Serialize)]
                struct JsonOutput {
                    checked: usize,
                    errors: usize,
                    warnings: usize,
                    infos: usize,
                    index_populated: bool,
                    issues: Vec<JsonIssue>,
                }
                let index_populated = agent_context
                    .index_db
                    .as_ref()
                    .map(|db| db.is_populated())
                    .unwrap_or(false);
                let out = JsonOutput {
                    checked: files_to_check.len(),
                    errors: n_errors,
                    warnings: n_warnings,
                    infos: n_infos,
                    index_populated,
                    issues: json_issues,
                };
```

**Step 3: Build to verify it compiles**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no output (clean build).

**Step 4: Smoke test — warning shows on cold start**

```bash
# Rename index.db temporarily to simulate cold start
mv /tmp/test_cold.db /tmp/test_cold.db.bak 2>/dev/null; true
sentinel pro check src/main.rs 2>&1 | grep "ÍNDICE VACÍO" || echo "WARNING NOT SHOWN"
```

This is a manual sanity check. If the project has an existing populated `.sentinel/index.db`, you can test with:
```bash
sentinel pro check src/ --format json 2>&1 | python3 -c "import sys,json; d=json.load(sys.stdin); print('index_populated:', d.get('index_populated'))"
```

Expected: `index_populated: True` (or False if monitor never ran).

**Step 5: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: `test result: ok. 33 passed`

---

## Task 3: Extract `build_audit_batches` + prefix grouping + unit tests

**Files:**
- Modify: `src/commands/pro.rs` (extract function, replace inline code, add tests)

**Context:** The batching logic is currently inline in the Audit handler at lines ~1772-1806. It groups by parent directory only, which means flat projects (all files in `src/`) get random batches. We extract it to `build_audit_batches()` and add prefix-based grouping: `user.service.ts` + `user.controller.ts` go in the same batch because prefix = `user`.

**Step 1: Write the failing tests first**

At the bottom of `src/commands/pro.rs`, before the final `}` of the file, add:

```rust
#[cfg(test)]
mod batching_tests {
    use super::build_audit_batches;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write_file(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, "x\n").unwrap();
        path
    }

    #[test]
    fn test_batch_groups_by_parent_dir() {
        let dir = TempDir::new().unwrap();
        let users_dir = dir.path().join("users");
        let auth_dir = dir.path().join("auth");
        std::fs::create_dir_all(&users_dir).unwrap();
        std::fs::create_dir_all(&auth_dir).unwrap();

        let f1 = { let p = users_dir.join("user.service.ts"); std::fs::write(&p, "x\n").unwrap(); p };
        let f2 = { let p = auth_dir.join("auth.service.ts"); std::fs::write(&p, "x\n").unwrap(); p };

        let batches = build_audit_batches(&[f1, f2], 8, 800);
        assert_eq!(batches.len(), 2, "files in different dirs must be in different batches");
    }

    #[test]
    fn test_batch_splits_large_group() {
        let dir = TempDir::new().unwrap();
        // 10 files with same prefix "module" → same group → splits at 8
        let files: Vec<PathBuf> = (0..10)
            .map(|i| write_file(&dir, &format!("module.part{}.ts", i)))
            .collect();

        let batches = build_audit_batches(&files, 8, 800);
        assert_eq!(batches.len(), 2, "10 files same prefix → 2 batches (8 + 2)");
        assert!(batches[0].len() <= 8);
        assert!(batches[1].len() <= 8);
    }

    #[test]
    fn test_batch_flat_project_prefix_grouping() {
        let dir = TempDir::new().unwrap();
        // All files in same directory but different module prefixes
        let f_user_svc  = write_file(&dir, "user.service.ts");
        let f_user_ctrl = write_file(&dir, "user.controller.ts");
        let f_auth_svc  = write_file(&dir, "auth.service.ts");

        let batches = build_audit_batches(&[f_user_svc, f_user_ctrl, f_auth_svc], 8, 800);
        assert_eq!(batches.len(), 2, "user.* and auth.* must be in separate batches");

        let user_batch = batches
            .iter()
            .find(|b| b.iter().any(|f| f.file_name().unwrap().to_str().unwrap().starts_with("user.")))
            .expect("user batch not found");
        assert_eq!(user_batch.len(), 2, "user batch must have both user.* files");
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test batching_tests 2>&1 | grep -E "FAILED|error\[|cannot find"
```

Expected: compile error "cannot find function `build_audit_batches`".

**Step 3: Add the `build_audit_batches` free function**

In `src/commands/pro.rs`, after the `use` imports at the top (after the last `use` statement, before `pub fn handle_pro_command`), add:

```rust
/// Groups files into batches for audit LLM calls.
///
/// Groups by `(parent_dir, module_prefix)` to keep semantically related files together.
/// `module_prefix` is the filename stem before the first dot: `user.service.ts` → `user`.
/// Splits groups exceeding `max_files_per_batch` or `max_lines_per_batch`.
pub fn build_audit_batches(
    files: &[std::path::PathBuf],
    max_files_per_batch: usize,
    max_lines_per_batch: usize,
) -> Vec<Vec<std::path::PathBuf>> {
    use std::collections::HashMap;

    fn module_prefix(path: &std::path::Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split('.').next())
            .unwrap_or("")
            .to_string()
    }

    // Group by (parent_dir, module_prefix) — keeps user.service.ts + user.controller.ts together
    let mut groups: HashMap<(std::path::PathBuf, String), Vec<std::path::PathBuf>> =
        HashMap::new();
    for f in files {
        let parent = f.parent().unwrap_or(f.as_path()).to_path_buf();
        let prefix = module_prefix(f);
        groups.entry((parent, prefix)).or_default().push(f.clone());
    }

    // Split each group by file count and line count caps
    let mut final_batches: Vec<Vec<std::path::PathBuf>> = Vec::new();
    for (_, group_files) in groups {
        let mut current_batch: Vec<std::path::PathBuf> = Vec::new();
        let mut current_lines = 0usize;
        for f in group_files {
            let file_lines = std::fs::read_to_string(&f)
                .map(|c| c.lines().count())
                .unwrap_or(0);
            if !current_batch.is_empty()
                && (current_batch.len() >= max_files_per_batch
                    || current_lines + file_lines > max_lines_per_batch)
            {
                final_batches.push(current_batch);
                current_batch = Vec::new();
                current_lines = 0;
            }
            current_batch.push(f);
            current_lines += file_lines;
        }
        if !current_batch.is_empty() {
            final_batches.push(current_batch);
        }
    }

    final_batches
}
```

**Step 4: Replace inline batching in Audit handler**

In `src/commands/pro.rs`, find the inline batching block in the Audit handler (starts with the comment `// Agrupar archivos por directorio-módulo para batching`, around line 1772). Replace from that comment through the closing `}` of the last `if !current_batch.is_empty()` block (around line 1806) with:

```rust
            // Agrupar archivos por módulo para batching
            const MAX_FILES_PER_BATCH: usize = 8;
            const MAX_LINES_PER_BATCH: usize = 800;
            let final_batches = build_audit_batches(&files_to_audit, MAX_FILES_PER_BATCH, MAX_LINES_PER_BATCH);
```

**Step 5: Run tests to verify they pass**

```bash
cargo test batching_tests 2>&1 | grep -E "ok|FAILED"
```

Expected:
```
test batching_tests::test_batch_groups_by_parent_dir ... ok
test batching_tests::test_batch_splits_large_group ... ok
test batching_tests::test_batch_flat_project_prefix_grouping ... ok
```

**Step 6: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: `test result: ok. 36 passed`

---

## Task 4: TypeScript-first message in `check` + README language table

**Files:**
- Modify: `src/commands/pro.rs` (check handler, after file collection)
- Modify: `README.md` (Requirements section, line ~46)

**Context:** `check` uses tree-sitter analyzers that only support TS/JS. If someone runs it on a Go or Python project they get zero violations with no explanation. We show a note once if none of the collected files are TS/JS.

**Step 1: Add TS-first note in check handler**

In `src/commands/pro.rs`, find this block in the check handler (around line 136-139):

```rust
            if files_to_check.is_empty() {
                ...
                return;
            }

            if !json_mode {
                println!("\n{} Capa 1 — Análisis Estático en {} archivo(s)...",
```

Between the end of `files_to_check.is_empty()` block and the `if !json_mode { println!("\n{} Capa 1` line, insert:

```rust
            // Note if no TS/JS files found — static analysis is TS/JS only
            if !json_mode {
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
                        "   Soporte para Go, Python, Java y otros lenguajes: próxima versión.\n"
                    );
                }
            }

```

**Step 2: Update README language table**

In `README.md`, find line 46:

```markdown
- Project with `tree-sitter` supported languages (TS, JS, etc.)
```

Replace with:

```markdown
- Project with TypeScript or JavaScript (full static analysis support)
  > Go, Python, Java, Rust and other languages: `monitor`, `audit`, and `review` work via LLM.
  > Static analysis rules (dead code, unused imports, complexity) — TypeScript/JavaScript only for now.

### Supported Languages

| Feature | TypeScript / JavaScript | Go, Python, Java, Rust, others |
|---------|------------------------|-------------------------------|
| Static analysis (`check`) | ✅ Complete | 🔜 Next major version |
| File monitor (`monitor`) | ✅ | ✅ |
| Audit & Review (LLM) | ✅ | ✅ |
```

**Step 3: Build and verify**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no output.

**Step 4: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: `test result: ok. 36 passed`

**Step 5: Smoke test — TS note shows for non-TS project**

```bash
# Create a temp Go file and run check on it
mkdir -p /tmp/gotest && echo 'func main() {}' > /tmp/gotest/main.go
sentinel pro check /tmp/gotest/main.go 2>&1 | grep "próxima versión" || echo "NOTE NOT SHOWN"
rm -rf /tmp/gotest
```

Expected: `Soporte para Go, Python, Java y otros lenguajes: próxima versión.`

---

## Final verification

```bash
cargo test 2>&1 | tail -5
```

Expected: `test result: ok. 36 passed; 0 failed`

```bash
cargo build --release 2>&1 | grep "^error"
```

Expected: no output.
