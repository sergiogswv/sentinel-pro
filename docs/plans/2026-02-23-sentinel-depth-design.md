# Design: Sentinel Depth — Multi-language, Smart Review, Ignore UX, Audit TTY, Review History

**Date:** 2026-02-23
**Status:** Approved
**Context:** Five UX and coverage improvements to make Sentinel production-ready across stacks.

---

## Feature 1: Multi-language Framework + Go

### Approach: LanguageRules registry + Go analyzers

Add a `LanguageRules` registry in `src/rules/languages/mod.rs` that maps file extensions to sets of
analyzers. Single dispatch point: `get_analyzers_for_extension(ext: &str) -> Vec<Box<dyn StaticAnalyzer>>`.

**Go analyzers** in `src/rules/languages/go.rs`:
- `GoDeadCodeAnalyzer` — top-level `func` declarations not appearing as callee in call_graph
- `GoUnusedImportsAnalyzer` — import paths that appear only once (declaration only) in source
- `GoComplexityAnalyzer` — cyclomatic complexity via tree-sitter-go (if/for/switch/select/case nodes)
- `GoFunctionLengthAnalyzer` — functions exceeding 50 lines

**TS/JS existing analyzers** move to `src/rules/languages/typescript.rs` — no functional changes.

**`src/index/builder.rs`** — add `"go"` to the extension match:
```rust
"go" => Some(tree_sitter_go::LANGUAGE.into()),
```

**`Cargo.toml`** — add `tree-sitter-go = "0.23"`.

**Extension path:** Adding Python = `src/rules/languages/python.rs` + register `"py"`. No engine changes.

**Implementation:**
- `Cargo.toml` — add tree-sitter-go
- `src/rules/languages/mod.rs` — new module with `get_analyzers_for_extension()`
- `src/rules/languages/typescript.rs` — move existing TS/JS analyzers (no logic change)
- `src/rules/languages/go.rs` — implement 4 Go analyzers
- `src/rules/static_analysis.rs` — update engine to use registry dispatch
- `src/index/builder.rs` — add Go language parsing

---

## Feature 2: Review Híbrido por Tamaño de Proyecto

### Approach: Size-based dispatch with centrality selection and multi-pass aggregation

```
< 20 files  → Current behavior: priority sort + top 8 × 100 lines
20-80 files → Centrality mode: top 20 most-referenced files × 150 lines
80+ files   → Multi-pass: group by top-level dir, one ReviewerAgent call per group
              (max 6 groups, max 10 files/group × 80 lines)
              Aggregate all suggestions → dedup by (title.lowercase, file) → unified output
```

**Centrality query:**
```sql
SELECT s.file_path, COUNT(c.callee_symbol) as hits
FROM call_graph c
JOIN symbols s ON c.callee_symbol = s.name
GROUP BY s.file_path
ORDER BY hits DESC
LIMIT 20
```

**Coverage line in output:**
```
📎 Contexto: 23 archivos · 3.180 líneas · modo centralidad (proyecto mediano)
```

**Multi-pass:** Multiple ReviewerAgent calls, results concatenated and deduped. Output format identical
to current — user sees unified result, not N separate reviews.

**Implementation:**
- `src/commands/pro.rs` — Review handler: add size detection + `select_by_centrality()` helper +
  multi-pass loop with aggregation

---

## Feature 3: Fricción en `sentinel ignore`

### Approach: Copy-ready hints per violation + symbol normalization

**A. Per-violation hint in check output (text mode only):**
```
⚠  [DEAD_CODE:45] userId no se usa en ninguna parte
   👉 sentinel ignore DEAD_CODE src/user.service.ts userId
```
Generated automatically from `RuleViolation` fields. Replaces generic hint at end of output.

**B. Symbol normalization on store and compare:**
```rust
fn normalize_symbol(s: &str) -> String {
    let suffixes = ["service", "controller", "repository", "guard",
                    "module", "handler", "resolver", "provider"];
    let s = s.to_lowercase().replace('_', "");
    for suffix in suffixes {
        if let Some(base) = s.strip_suffix(suffix) { return base.to_string(); }
    }
    s
}
```

Applied when **saving** an ignore entry and when **filtering** violations in check.
Result: `AuthService` matches violations reported as `authservice`, `auth_service`, `AuthServiceImpl`, etc.

**Implementation:**
- `src/commands/ignore.rs` — add `normalize_symbol()`, apply in add + filter paths
- `src/commands/pro.rs` — per-violation hint in check output (replace generic hint)

---

## Feature 4: Auto-detect TTY + Improved Interactive Menu

### Approach: IsTerminal auto-detect + per-issue loop replacing MultiSelect

**TTY detection:**
```rust
use std::io::IsTerminal;
let is_tty = std::io::stdout().is_terminal();
let non_interactive = no_fix || json_mode || !is_tty;
```
No new flags. Automatic non-interactive in CI/CD, pipes, and redirections.

**Improved interactive menu (when `is_tty && !no_fix && !json_mode`):**

Replace `dialoguer::MultiSelect` with a per-issue loop:
```
─────────────────────────────────────────────
Issue 3/12 · HIGH · src/auth.service.ts
Función demasiado larga: validateToken (87 líneas)

Fix sugerido:
  Extraer la lógica de validación JWT a un método privado
  validateJwtPayload(payload) y la lógica de refresh a
  refreshTokenIfExpired(token). Objetivo: < 40 líneas.

[a]plicar  [s]altar  [S]altar todos  [q]salir
```

Keys: `a` = apply fix, `s` = skip, `S` = skip all remaining, `q` = quit immediately.
Shows full `suggested_fix` (no 90-char truncation).

**Implementation:**
- `src/commands/pro.rs` — replace dialoguer MultiSelect block with per-issue loop

---

## Feature 5: Historial de Review + Diff

### Approach: JSON files in .sentinel/reviews/ + --history + --diff flags

**Storage:** `.sentinel/reviews/YYYY-MM-DD-HH-MM.json` per run:
```json
{
  "timestamp": "2026-02-23T14:32:00",
  "project_root": "src/",
  "files_reviewed": 8,
  "suggestions": [
    { "title": "...", "description": "...", "impact": "high", "files_involved": [...] }
  ]
}
```

**New flags on `sentinel pro review`:**
```bash
sentinel pro review            # current behavior + auto-save to .sentinel/reviews/
sentinel pro review --history  # list last 5 reviews: date · count · first suggestion title
sentinel pro review --diff     # compare last 2 reviews: new / resolved / persistent
```

**`--history` output:**
```
📋 Historial de reviews:
  2026-02-23 14:32  ·  6 sugerencias  ·  "Extraer lógica de validación JWT..."
  2026-02-22 09:15  ·  8 sugerencias  ·  "AuthModule acoplado directamente a UserModule..."
```

**`--diff` output:**
```
🔍 Comparando reviews (2026-02-23 vs 2026-02-22):
  ✅ Resueltas (2):    "Extraer validateToken"  ·  "Mover DTOs a carpeta separada"
  🆕 Nuevas (1):       "AuthGuard sin manejo de errores tipado"
  ⏳ Persistentes (5): ...
```

Diff key: `title.to_lowercase()` — same criterion as audit dedup.

**Implementation:**
- `src/commands/mod.rs` — add `history: bool` and `diff: bool` flags to `ProCommands::Review`
- `src/commands/pro.rs` — save after successful review + handle `--history` + `--diff`

---

## Success Criteria

- `sentinel check src/` on a Go project reports DEAD_CODE, UNUSED_IMPORT, HIGH_COMPLEXITY, FUNCTION_TOO_LONG.
- Adding Python support requires only a new `src/rules/languages/python.rs` + one registry entry.
- `sentinel pro review` on a 50-file project uses centrality-based file selection; on a 100-file project
  runs multi-pass and produces a unified report.
- Check output shows per-violation `sentinel ignore` hint with exact command to copy.
- `sentinel ignore AuthService src/auth.ts` matches violations reported as `auth_service`, `authService`.
- `sentinel pro audit src/` in a CI pipeline (no TTY) runs non-interactively without flags.
- Interactive audit shows full suggested_fix, supports `[a/s/S/q]` keys.
- `sentinel pro review --history` lists last 5 reviews.
- `sentinel pro review --diff` shows resolved/new/persistent suggestions between last 2 runs.
- All existing 43 tests continue to pass.

---

## Out of Scope

- Python, Rust, Java analyzer implementations (framework supports them, not implemented here)
- Vector/semantic search for review context (future)
- `// @sentinel-ignore` inline comments in source files
- Review diff with line-level granularity
- Remote review history sync
