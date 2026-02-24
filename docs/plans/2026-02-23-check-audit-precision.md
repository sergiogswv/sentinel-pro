# Check/Audit Precision Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate false positives in `check` (cross-file dead code via SQLite, decorator imports) and make `audit` usable on real projects via `--max-files` and module batching.

**Architecture:** Three independent changes. (1) Add `symbol: Option<String>` to `RuleViolation`, populate it in static analyzers, then filter DEAD_CODE violations in `engine.rs` using a new `CallGraph::is_called_from_other_file()`. (2) In `UnusedImportsAnalyzer`, skip imports that appear as decorators (`@Name`). (3) In the audit handler, sort files by mtime, take `--max-files N`, group by parent directory, send each group as one batched LLM call.

**Tech Stack:** Rust, tree-sitter, regex crate (already in Cargo.toml), rusqlite, SQLite call_graph table.

**Key files:**
- `src/rules/mod.rs` — `RuleViolation` struct
- `src/rules/static_analysis.rs` — `DeadCodeAnalyzer`, `UnusedImportsAnalyzer`
- `src/index/call_graph.rs` — new method `is_called_from_other_file`
- `src/rules/engine.rs` — cross-file post-filter
- `src/commands/mod.rs` — new `--max-files` flag on Audit
- `src/commands/pro.rs` — audit handler batching logic

---

### Task 1: Add `symbol` field to `RuleViolation`

**Files:**
- Modify: `src/rules/mod.rs`

**Step 1: Add the field**

Open `src/rules/mod.rs`. The current struct is:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    pub rule_name: String,
    pub message: String,
    pub level: RuleLevel,
    pub line: Option<usize>,
}
```

Change it to:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    pub rule_name: String,
    pub message: String,
    pub level: RuleLevel,
    pub line: Option<usize>,
    pub symbol: Option<String>,
}
```

**Step 2: Fix all constructor call sites**

Run:
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo build 2>&1 | grep "^error"
```
Expected: errors about missing field `symbol` in struct literals.

For EVERY `RuleViolation { ... }` constructor in `src/rules/engine.rs` that already has `line: None`, add `symbol: None`. There are 2 of them (DEAD_CODE_GLOBAL and framework pattern rules).

In `src/rules/static_analysis.rs`, all constructors for NON-dead-code/unused-import rules (HIGH_COMPLEXITY, FUNCTION_TOO_LONG, NAMING_CONVENTION) add `symbol: None`.

For `DeadCodeAnalyzer` (DEAD_CODE violations), set `symbol: Some(name.to_string())`.
For `UnusedImportsAnalyzer` (UNUSED_IMPORT violations), set `symbol: Some(name.to_string())`.

**Step 3: Build to confirm no errors**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo build 2>&1 | grep "^error"
```
Expected: empty output (no errors).

**Step 4: Run all tests**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test 2>&1 | tail -5
```
Expected: `28 passed; 0 failed`

---

### Task 2: Add `CallGraph::is_called_from_other_file()`

**Files:**
- Modify: `src/index/call_graph.rs`

**Step 1: Write the failing test**

Add this test inside the existing `mod tests` block at the bottom of `src/index/call_graph.rs`:

```rust
    #[test]
    fn test_is_called_from_other_file_false_when_no_callers() {
        let (_f, db) = make_db();
        let cg = CallGraph::new(&db);
        // No data in call_graph table → should return false
        assert!(!cg.is_called_from_other_file("myFunction", "src/app.service.ts"));
    }
```

**Step 2: Run to verify FAIL (method doesn't exist yet)**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test test_is_called_from_other_file -- --nocapture 2>&1
```
Expected: compile error — method not found.

**Step 3: Implement the method**

Add this method to `impl<'a> CallGraph<'a>` in `src/index/call_graph.rs`, after `get_dead_code`:

```rust
    /// Returns true if `symbol` is called from any file OTHER than `file_path`.
    /// Used to skip DEAD_CODE violations for cross-file symbols.
    pub fn is_called_from_other_file(&self, symbol: &str, file_path: &str) -> bool {
        let conn = self.db.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM call_graph \
             WHERE callee_symbol = ? AND caller_file != ?",
            rusqlite::params![symbol, file_path],
            |row| row.get(0),
        ).unwrap_or(0);
        count > 0
    }
```

**Step 4: Run test to verify PASS**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test test_is_called_from_other_file -- --nocapture 2>&1
```
Expected: `test ... ok`

**Step 5: Run all tests**
```bash
cargo test 2>&1 | tail -5
```
Expected: `29 passed; 0 failed`

---

### Task 3: Filter DEAD_CODE in `engine.rs` using cross-file check

**Files:**
- Modify: `src/rules/engine.rs`

**Step 1: Write the failing test**

Add to `src/rules/static_analysis.rs` tests block (or a new test in engine if you prefer — easier in static_analysis since we can test through engine):

Actually, add this test in `src/rules/static_analysis.rs` in the `mod tests` block:

```rust
    #[test]
    fn test_dead_code_symbol_field_populated() {
        let lang = ts_lang();
        let analyzer = DeadCodeAnalyzer::new();
        let code = "function unusedFn() { return 42; }";
        let violations = analyzer.analyze(&lang, code);
        let v = violations.iter().find(|v| v.rule_name == "DEAD_CODE")
            .expect("Should detect DEAD_CODE");
        assert_eq!(v.symbol, Some("unusedFn".to_string()),
            "symbol field must be populated for DEAD_CODE violations");
    }
```

**Step 2: Run to verify test passes (symbol was set in Task 1)**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test test_dead_code_symbol_field_populated -- --nocapture 2>&1
```
Expected: `test ... ok` (confirms Task 1 set the symbol correctly)

**Step 3: Add cross-file post-filter to `engine.rs`**

In `src/rules/engine.rs`, in `validate_file()`, find the section that starts with:
```rust
        // --- Análisis de Proyecto Cruzado (SI hay DB disponible) ---
        if let Some(ref db) = self.index_db {
            let rel_path = _file_path.to_string_lossy();
            // 1. Dead Code de Proyecto
            let call_graph = crate::index::call_graph::CallGraph::new(db);
```

Add a cross-file filter BEFORE the `DEAD_CODE_GLOBAL` block:

```rust
        // --- Análisis de Proyecto Cruzado (SI hay DB disponible) ---
        if let Some(ref db) = self.index_db {
            let rel_path = _file_path.to_string_lossy();
            let call_graph = crate::index::call_graph::CallGraph::new(db);

            // Post-filter: remove DEAD_CODE violations for symbols called from other files
            violations.retain(|v| {
                if v.rule_name != "DEAD_CODE" {
                    return true;
                }
                if let Some(ref sym) = v.symbol {
                    // Keep violation only if NOT called from another file
                    !call_graph.is_called_from_other_file(sym, &rel_path)
                } else {
                    true // no symbol info → keep to avoid hiding real issues
                }
            });

            // 1. Dead Code de Proyecto (DEAD_CODE_GLOBAL from call graph)
            if let Ok(dead_symbols) = call_graph.get_dead_code(Some(&rel_path)) {
```

**Step 4: Build and run all tests**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo build 2>&1 | grep "^error" && cargo test 2>&1 | tail -5
```
Expected: no errors, `30 passed; 0 failed`

---

### Task 4: Decorator recognition in `UnusedImportsAnalyzer`

**Files:**
- Modify: `src/rules/static_analysis.rs`

**Step 1: Write the failing test**

Add to the `mod tests` block:

```rust
    #[test]
    fn test_unused_import_not_flagged_when_decorator() {
        let lang = ts_lang();
        let analyzer = UnusedImportsAnalyzer::new();
        // ApiProperty aparece SOLO como decorador @ApiProperty() — antes era falso positivo
        let code = "import { ApiProperty } from '@nestjs/swagger';\n\nexport class UserDto {\n  @ApiProperty()\n  name: string;\n}";
        let violations = analyzer.analyze(&lang, code);
        let flagged = violations.iter().any(|v| v.rule_name == "UNUSED_IMPORT");
        assert!(!flagged, "@ApiProperty() está en uso como decorador — no debe ser UNUSED_IMPORT");
    }
```

**Step 2: Run to verify FAIL**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test test_unused_import_not_flagged_when_decorator -- --nocapture 2>&1
```
Expected: FAIL — `@ApiProperty` currently gets flagged.

**Step 3: Add decorator check in `UnusedImportsAnalyzer::analyze()`**

In `src/rules/static_analysis.rs`, find the `UnusedImportsAnalyzer` block. The current check is:
```rust
                if count_word_occurrences(source_code, name) == 1 {
                    violations.push(RuleViolation {
                        rule_name: "UNUSED_IMPORT".to_string(),
                        message: format!("El import '{}' no se está utilizando en este archivo.", name),
                        level: RuleLevel::Warning,
                        line: find_line_of(source_code, name),
                        symbol: Some(name.to_string()),
                    });
                }
```

Replace with:
```rust
                if count_word_occurrences(source_code, name) == 1 {
                    // Skip if used as a decorator (@Name or @Name())
                    let decorator_pattern = format!(r"@{}\b", regex::escape(name));
                    let used_as_decorator = regex::Regex::new(&decorator_pattern)
                        .map(|re| re.is_match(source_code))
                        .unwrap_or(false);
                    if used_as_decorator {
                        continue;
                    }
                    violations.push(RuleViolation {
                        rule_name: "UNUSED_IMPORT".to_string(),
                        message: format!("El import '{}' no se está utilizando en este archivo.", name),
                        level: RuleLevel::Warning,
                        line: find_line_of(source_code, name),
                        symbol: Some(name.to_string()),
                    });
                }
```

**Step 4: Run the new test**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test test_unused_import_not_flagged_when_decorator -- --nocapture 2>&1
```
Expected: `test ... ok`

**Step 5: Run ALL tests**
```bash
cargo test 2>&1 | tail -5
```
Expected: `31 passed; 0 failed`

---

### Task 5: Add `--max-files` flag to Audit command

**Files:**
- Modify: `src/commands/mod.rs`

**Step 1: Add the flag**

In `src/commands/mod.rs`, find the `Audit` variant:
```rust
    Audit {
        target: String,
        #[arg(long)]
        no_fix: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
```

Add the new flag:
```rust
    Audit {
        target: String,
        #[arg(long)]
        no_fix: bool,
        #[arg(long, default_value = "text")]
        format: String,
        /// Máximo de archivos a auditar (default: 20). Usa un número mayor para proyectos grandes.
        #[arg(long, default_value = "20")]
        max_files: usize,
    },
```

**Step 2: Fix the call site in `monitor.rs`**

In `src/commands/monitor.rs`, find:
```rust
                            crate::commands::ProCommands::Audit {
                                target: final_path.to_string(),
                                no_fix: false,
                                format: "text".to_string(),
                            },
```
Add `max_files: 20`:
```rust
                            crate::commands::ProCommands::Audit {
                                target: final_path.to_string(),
                                no_fix: false,
                                format: "text".to_string(),
                                max_files: 20,
                            },
```

**Step 3: Build to confirm no errors**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo build 2>&1 | grep "^error"
```
Expected: errors about unmatched `max_files` in `pro.rs`. Fix by adding `max_files` to the match arm in `src/commands/pro.rs`:

Find `ProCommands::Audit { target, no_fix, format }` and change to:
`ProCommands::Audit { target, no_fix, format, max_files }`

Then build again:
```bash
cargo build 2>&1 | grep "^error"
```
Expected: empty (no errors).

---

### Task 6: Implement `--max-files` + module batching in the audit handler

**Files:**
- Modify: `src/commands/pro.rs`

**Context:** The audit handler is at `ProCommands::Audit { ... }` around line 1700. After collecting `files_to_audit`, the current code loops one file at a time. We need to:
1. Sort by mtime, take `max_files`
2. Group by parent directory (module batches)
3. Send each batch as a single LLM call

**Step 1: Add `--max-files` selection after file collection**

After the block that populates `files_to_audit` and the `files_to_audit.is_empty()` check, add:

```rust
            // Seleccionar los archivos más recientes hasta max_files
            let total_found = files_to_audit.len();
            if total_found > max_files {
                files_to_audit.sort_by_key(|p| {
                    std::fs::metadata(p)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                });
                files_to_audit.reverse(); // newest first
                files_to_audit.truncate(max_files);
                if !json_mode {
                    println!(
                        "   ℹ️  Auditando {} de {} archivos (usa --max-files {} para todos)",
                        max_files, total_found, total_found
                    );
                }
            }
```

**Step 2: Replace single-file loop with module batching**

Find and REPLACE the entire `for (i, file_path) in files_to_audit.iter().enumerate() { ... }` loop (from `for (i, file_path)` to the closing `pb.finish_and_clear();` after the match on orchestrator result) with the batching implementation:

```rust
            // Agrupar archivos por directorio-módulo para batching
            use std::collections::HashMap;
            let mut module_batches: HashMap<std::path::PathBuf, Vec<std::path::PathBuf>> = HashMap::new();
            for f in &files_to_audit {
                let parent = f.parent().unwrap_or(f.as_path()).to_path_buf();
                module_batches.entry(parent).or_default().push(f.clone());
            }

            // Dividir batches grandes (>8 archivos o >800 líneas) en sub-batches
            const MAX_FILES_PER_BATCH: usize = 8;
            const MAX_LINES_PER_BATCH: usize = 800;

            let mut final_batches: Vec<Vec<std::path::PathBuf>> = Vec::new();
            for (_, files) in module_batches {
                let mut current_batch: Vec<std::path::PathBuf> = Vec::new();
                let mut current_lines = 0usize;
                for f in files {
                    let file_lines = std::fs::read_to_string(&f)
                        .map(|c| c.lines().count())
                        .unwrap_or(0);
                    if !current_batch.is_empty()
                        && (current_batch.len() >= MAX_FILES_PER_BATCH
                            || current_lines + file_lines > MAX_LINES_PER_BATCH)
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

            let total_batches = final_batches.len();

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
                    .and_then(|f| f.parent())
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "módulo".to_string());

                let pb = if !json_mode {
                    ui::crear_progreso(&format!(
                        "[{}/{}] Auditando módulo '{}'  ({} archivo(s))...",
                        batch_idx + 1,
                        total_batches,
                        module_name,
                        batch_files.len()
                    ))
                } else {
                    indicatif::ProgressBar::hidden()
                };

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
                        module_name,
                        batch_rel_paths.join(", ")
                    ),
                    task_type: TaskType::Analyze,
                    file_path: batch_files.first().cloned(),
                    context: Some(batch_context),
                };

                match rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context)) {
                    Ok(res) => {
                        let json_str = crate::ai::utils::extraer_json(&res.output);
                        match serde_json::from_str::<Vec<AuditIssue>>(&json_str) {
                            Ok(mut issues) => {
                                for issue in &mut issues {
                                    // Normalizar file_path: buscar en batch_files el que coincida con issue.file_path
                                    let matched_path = batch_files.iter()
                                        .find(|f| f.to_string_lossy().contains(&issue.file_path)
                                            || issue.file_path.contains(&f.file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_default()))
                                        .map(|f| f.to_string_lossy().to_string())
                                        .unwrap_or_else(|| {
                                            batch_files.first()
                                                .map(|f| f.to_string_lossy().to_string())
                                                .unwrap_or_default()
                                        });
                                    issue.file_path = matched_path;
                                    all_issues.push(issue.clone());
                                }
                            }
                            Err(_) => {
                                parse_failures += 1;
                                pb.finish_and_clear();
                                if !json_mode {
                                    println!(
                                        "   ⚠️  Módulo '{}': el AI no devolvió JSON válido — saltado.",
                                        module_name.yellow()
                                    );
                                }
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        parse_failures += 1;
                        pb.finish_and_clear();
                        if !json_mode {
                            println!("   ❌ Error auditando módulo '{}': {}", module_name, e);
                        }
                        continue;
                    }
                }
                pb.finish_and_clear();
            }
```

**Step 3: Build**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo build 2>&1 | grep "^error"
```
Expected: empty. Fix any errors that appear.

**Step 4: Smoke test — audit with `--max-files`**
```bash
cd /home/protec/Documentos/dev/backend-proleads && sentinel pro audit src/ --no-fix --max-files 5 2>&1 | head -20
```
Expected: runs 5 files grouped into batches, prints module names, finishes in <60s.

**Step 5: Run all unit tests**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test 2>&1 | tail -5
```
Expected: `31 passed; 0 failed`

---

### Task 7: Integration test — update `test-real-project.sh`

**Files:**
- Modify: `scripts/test-real-project.sh`

**Step 1: Add a `--max-files` check to the audit section**

Find the existing `[ 3/4 ] sentinel pro audit` section and add a new assertion after the existing ones:

```bash
# Assert: --max-files flag is accepted (no "unknown flag" error)
AUDIT_MAXFILES=$(timeout 60 sentinel pro audit src/ --no-fix --max-files 3 2>&1 || true)
if echo "$AUDIT_MAXFILES" | grep -qvE "error: unexpected argument|unrecognized"; then
  pass "audit acepta --max-files sin error"
else
  fail "audit rechaza --max-files" "$AUDIT_MAXFILES"
fi
```

**Step 2: Verify syntax**
```bash
bash -n /home/protec/Documentos/dev/sentinel-pro/scripts/test-real-project.sh && echo "Sintaxis OK"
```

---

### Task 8: Full verification

**Step 1: Run all unit tests**
```bash
cd /home/protec/Documentos/dev/sentinel-pro && cargo test 2>&1 | tail -10
```
Expected: `31 passed; 0 failed`

**Step 2: Run check against real project — expect fewer false positives**
```bash
cd /home/protec/Documentos/dev/backend-proleads && sentinel pro check src/ --format text 2>&1 | grep -c "DEAD_CODE\|UNUSED_IMPORT" || true
```
(Note the count — should be lower than before the fix once the index is populated.)

**Step 3: Run audit with --max-files**
```bash
cd /home/protec/Documentos/dev/backend-proleads && sentinel pro audit src/ --no-fix --max-files 10 2>&1 | tail -5
```
Expected: finishes in under 120s, shows batch-level progress.

**Step 4: Commit**
```bash
cd /home/protec/Documentos/dev/sentinel-pro
git add Cargo.toml Cargo.lock \
  src/rules/mod.rs \
  src/rules/static_analysis.rs \
  src/rules/engine.rs \
  src/index/call_graph.rs \
  src/commands/mod.rs \
  src/commands/monitor.rs \
  src/commands/pro.rs \
  scripts/test-real-project.sh \
  docs/plans/2026-02-23-check-audit-precision.md \
  docs/plans/2026-02-23-check-audit-precision-design.md
git commit -m "$(cat <<'EOF'
feat: check/audit precision — cross-file dead code, decorator imports, audit batching

check improvements:
- RuleViolation.symbol field: static analyzers now store the symbol name
- DEAD_CODE cross-file filter: CallGraph::is_called_from_other_file() removes
  false positives for symbols called from other modules (requires SQLite index)
- UNUSED_IMPORT decorator fix: @Injectable, @ApiProperty, @Column etc. no
  longer flagged — decorator usage (@Name) counts as a valid use

audit improvements:
- --max-files N (default 20): selects N most-recently-modified files, warns
  if project has more
- Module batching: files grouped by parent directory, 1 LLM call per batch
  (max 8 files / 800 lines per batch) — replaces 1 call per file

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```
