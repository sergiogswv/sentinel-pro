# Design: Sentinel Power — SARIF/diff, Rule Config, Python, Monitor, Go Richer Rules

**Date:** 2026-02-23
**Status:** Approved
**Context:** Five features to make Sentinel a credible CI/CD tool across multiple stacks.

---

## Feature 1A: SARIF Output + GitHub Actions

### Approach: --format sarif in check/audit + workflow example

Add `sarif` as a third format option to `sentinel pro check` and `sentinel pro audit`.

**SARIF 2.1.0 output structure:**
```json
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [{
    "tool": { "driver": { "name": "sentinel", "version": "0.1.0", "rules": [...] } },
    "results": [
      {
        "ruleId": "DEAD_CODE",
        "level": "warning",
        "message": { "text": "userId no se usa en ninguna parte" },
        "locations": [{
          "physicalLocation": {
            "artifactLocation": { "uri": "src/user.service.ts" },
            "region": { "startLine": 23 }
          }
        }]
      }
    ]
  }]
}
```

**Rule severity mapping:**
- `RuleLevel::Error` → SARIF `"error"`
- `RuleLevel::Warning` → SARIF `"warning"`
- `RuleLevel::Info` → SARIF `"note"`

**GitHub Actions workflow example** (placed in `docs/github-actions-example.yml`):
```yaml
- name: Sentinel check
  run: sentinel pro check src/ --format sarif > sentinel.sarif
- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: sentinel.sarif
```

**Implementation:**
- `src/commands/pro.rs` — `render_sarif(violations)` function + branch in check/audit format match
- `docs/github-actions-example.yml` — workflow reference

---

## Feature 1B: git diff in Review

### Approach: Changed files get priority slots in review context

**When running `sentinel pro review`:**

1. Run `git diff --name-only HEAD` (or `git diff --name-only main...HEAD` for branch).
2. Collect changed file paths that exist in the project.
3. In review context building:
   - Changed files occupy **first N slots** (up to half of the budget).
   - Remaining slots filled by existing priority logic (NestJS patterns, centrality).
4. Add a note line in the coverage output:
   ```
   📎 Contexto: 8 archivos · 1.240 líneas · 3 archivos del diff reciente
   ```
5. If `git` is not available or not a git repo, silently fall back to current behavior.

**Implementation:**
- `src/commands/pro.rs` — `get_changed_files() -> Vec<String>` + inject at start of file list

---

## Feature 2: Rule Thresholds + `sentinel rules list`

### Approach: [rules] section in .sentinelrc.toml + new subcommand

**`.sentinelrc.toml` format:**
```toml
[project]
name = "my-api"
root = "src/"

[rules]
complexity_threshold = 15      # default: 10
function_length_threshold = 80 # default: 50
dead_code_enabled = true
unused_imports_enabled = true
```

**Loading order:** defaults → `.sentinelrc.toml` (if present) → CLI flags (override).

**`sentinel rules list` output:**
```
📋 Reglas activas:
  DEAD_CODE            [ERROR]   Funciones/variables no referenciadas
  UNUSED_IMPORT        [WARNING] Imports sin uso en el archivo
  HIGH_COMPLEXITY      [WARNING] Complejidad ciclomática > 15  (threshold: 15)
  FUNCTION_TOO_LONG    [INFO]    Funciones > 80 líneas          (threshold: 80)
  UNCHECKED_ERROR      [WARNING] Error de Go sin verificar
  NAMING_CONVENTION_GO [INFO]    Constante Go en formato ALL_CAPS
  DEFER_IN_LOOP        [WARNING] defer dentro de bucle for
```

**Struct:**
```rust
pub struct RuleConfig {
    pub complexity_threshold: usize,     // default: 10
    pub function_length_threshold: usize, // default: 50
    pub dead_code_enabled: bool,         // default: true
    pub unused_imports_enabled: bool,    // default: true
}

impl Default for RuleConfig { ... }
```

**Implementation:**
- `src/config.rs` — `RuleConfig` struct + `load_rule_config(project_root) -> RuleConfig`
- `src/commands/mod.rs` — add `Rules` subcommand with `list` flag
- `src/commands/rules.rs` — `handle_rules_command`
- `src/rules/engine.rs` — accept `RuleConfig`, pass thresholds to analyzers
- `src/rules/static_analysis.rs` — analyzers accept threshold via constructor
- `src/commands/pro.rs` — load config before dispatching check/audit/review

---

## Feature 3: Python Basic Static Analysis

### Approach: tree-sitter-python + 3 analyzers in languages registry

**New crate:** `tree-sitter-python = "0.23"` in `Cargo.toml`.

**`src/rules/languages/python.rs`:**
- `PythonDeadCodeAnalyzer` — top-level `function_definition` and `class_definition` names not appearing elsewhere in source (word-boundary)
- `PythonUnusedImportsAnalyzer` — `import_statement` and `import_from_statement` identifiers not referenced later in source
- `PythonComplexityAnalyzer` — cyclomatic complexity via `if_statement`, `elif_clause`, `for_statement`, `while_statement`, `except_clause`, `with_statement`, `conditional_expression`, `boolean_operator`

**Registry entry in `src/rules/languages/mod.rs`:**
```rust
"py" => Some((tree_sitter_python::LANGUAGE.into(), python::analyzers())),
```

**`src/index/builder.rs`:**
```rust
"py" => Some(tree_sitter_python::LANGUAGE.into()),
```

**Implementation:**
- `Cargo.toml` — add tree-sitter-python
- `src/rules/languages/python.rs` — 3 analyzers + unit tests
- `src/rules/languages/mod.rs` — register "py"
- `src/index/builder.rs` — add "py" language parsing

---

## Feature 4: `sentinel monitor --daemon`

### Approach: Self-spawn with PID file at .sentinel/monitor.pid

**New flags on `sentinel monitor`:**
```bash
sentinel monitor           # current behavior (foreground)
sentinel monitor --daemon  # fork to background, write PID file
sentinel monitor --stop    # read PID file, send SIGTERM, remove PID file
sentinel monitor --status  # check if daemon is running (PID alive?)
```

**`--daemon` behavior:**
```rust
// In monitor handler:
if daemon {
    let exe = std::env::current_exe()?;
    let child = Command::new(exe)
        .args(["monitor"])  // re-run without --daemon
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let pid = child.id();
    std::fs::write(".sentinel/monitor.pid", pid.to_string())?;
    println!("✅ sentinel monitor iniciado (PID {})", pid);
    return Ok(());
}
```

**PID file:** `.sentinel/monitor.pid` (plain text, single integer).

**`--stop` behavior:**
1. Read PID from `.sentinel/monitor.pid`.
2. Send SIGTERM via `nix::sys::signal::kill(Pid::from_raw(pid), Signal::SIGTERM)`.
3. Remove `.sentinel/monitor.pid`.
4. Print `"✅ sentinel monitor detenido (PID {pid})"`.

**`--status` behavior:**
1. Read PID from `.sentinel/monitor.pid`. If missing: `"ℹ️  sentinel monitor no está corriendo"`.
2. Check `/proc/{pid}/status` (Linux) or `kill(pid, 0)` — if alive: `"✅ Corriendo (PID {pid})"`.

**Dependencies:** `nix = { version = "0.29", features = ["signal", "process"] }` in `Cargo.toml`.

**Implementation:**
- `Cargo.toml` — add nix
- `src/commands/mod.rs` — add `daemon`, `stop`, `status` flags to `Commands::Monitor`
- `src/commands/monitor.rs` (new or extend existing) — implement daemon/stop/status logic
- `src/main.rs` — wire flags through

---

## Feature 5: Go Richer Rules

### Approach: 3 new analyzers added to go.rs

**`GoUncheckedErrorAnalyzer`** — detects `_, _ :=` or `_ =` patterns where the blank identifier discards an error return:
- Query: `short_var_declaration` where right side is a `call_expression` and left side contains a blank identifier `_`.
- Rule: `UNCHECKED_ERROR` / WARNING.
- Message: `"Resultado de error descartado en llamada a {callee}"`.

**`GoNamingConventionAnalyzer`** — detects Go constants in ALL_CAPS format (violates Go naming conventions):
- Query: `const_spec` identifier nodes.
- Check: name matches `^[A-Z][A-Z0-9_]+$` (more than one char, all uppercase with underscores).
- Rule: `NAMING_CONVENTION_GO` / INFO.
- Message: `"Constante Go en formato ALL_CAPS: {name}. Usar PascalCase."`.

**`GoDeferInLoopAnalyzer`** — detects `defer` statements inside `for` loops:
- Query: `for_statement` containing `defer_statement` as descendant.
- Rule: `DEFER_IN_LOOP` / WARNING.
- Message: `"defer dentro de un bucle: el recurso no se libera hasta que la función retorna"`.

**Implementation:**
- `src/rules/languages/go.rs` — add 3 new analyzer structs + include in `analyzers()` + unit tests

---

## Success Criteria

- `sentinel pro check src/ --format sarif` outputs valid SARIF 2.1.0 parseable by GitHub Security tab.
- `sentinel pro review` on a project with recent `git diff` shows those files first in context.
- `.sentinelrc.toml` with `complexity_threshold = 15` → check reports HIGH_COMPLEXITY only above 15.
- `sentinel rules list` shows all active rules with current thresholds.
- `sentinel pro check src/` on a Python project reports DEAD_CODE, UNUSED_IMPORT, HIGH_COMPLEXITY.
- `sentinel monitor --daemon` exits immediately with PID message; process continues in background.
- `sentinel monitor --stop` terminates the background process.
- `sentinel pro check` on Go code with `_, err :=` discarded → UNCHECKED_ERROR reported.
- `ALL_CAPS` Go constants → NAMING_CONVENTION_GO reported.
- `defer` inside `for` → DEFER_IN_LOOP reported.
- All existing 53 tests continue to pass.

---

## Out of Scope

- SARIF suppression baselines
- git blame integration in review
- Per-file rule overrides (only global thresholds)
- Monitor on Windows (daemon uses POSIX signals)
- Go generics-aware analysis
- Python type annotation analysis
