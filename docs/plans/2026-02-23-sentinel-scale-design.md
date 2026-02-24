# Design: Sentinel Scale — Review Context, Audit Parallelism, Index Robustness, Feedback Loop

**Date:** 2026-02-23
**Status:** Approved
**Context:** Four independent improvements to make Sentinel usable on real large projects (NestJS 200+ files).

---

## Problem Statement

1. **Review is blind** — LLM sees 8 files × 100 lines ≈ 800 lines. A medium NestJS project has 5,000+. Output sounds generic because it is.

2. **Audit is slow** — Sequential batches. A 200-file project = 25+ sequential LLM calls (~8 min). No retries. If batch 18 fails, that work is lost.

3. **Index is fragile** — `sentinel monitor` indexes in real time but large refactors leave the index silently stale. No way to rebuild or check status.

4. **Zero feedback loop** — No way to mark a finding as a false positive. Users see the same `DEAD_CODE` on `userId` every run and start ignoring all findings.

---

## Feature 2: Review — Structural Index Summary

### Approach: Index-based context block

The existing SQLite index has 3 useful tables: `symbols`, `call_graph`, `import_usage`. Before building the LLM prompt, query all three and inject a structured text block into the `ReviewerAgent` prompt.

**Context block format (~400 tokens):**
```
=== CONTEXTO ARQUITECTURAL (del índice) ===
Símbolos exportados (top 200):
  UserService.createUser [method] → src/user.service.ts:45
  AuthGuard.canActivate [method]  → src/auth.guard.ts:12
  ...

Relaciones de llamada (top 100):
  user.service.ts → user.repository.ts (findById)
  auth.service.ts → user.service.ts (createUser)
  ...

Imports activos (top 100):
  user.service.ts ← UserRepository, JwtService
  auth.module.ts  ← UserModule, JwtModule
  ...
```

**Implementation:**
- `src/agents/reviewer.rs` — implement `build_rag_context()` (currently returns empty string)
- `ReviewerAgent` already receives `agent_context` which has `index_db: Option<Arc<IndexDb>>`
- `src/index/db.rs` — add 3 query methods: `get_symbols()`, `get_call_graph()`, `get_import_usage()`

**Fallback:** if index is empty, block is silently omitted.

**Limits:** symbols capped at 200, call_graph at 100, imports at 100 to control token budget.

---

## Feature 3: Audit — Parallel Batches with `--concurrency`

### Approach: tokio tasks bounded by Semaphore

Add `--concurrency N` flag to `sentinel pro audit` (default 3, range 1-10).

Replace sequential batch for-loop with bounded parallel tokio tasks:

```rust
let semaphore = Arc::new(Semaphore::new(concurrency));
let mut handles = vec![];

for batch in batches {
    let permit = Arc::clone(&semaphore).acquire_owned().await?;
    let agent = reviewer_agent.clone();  // ReviewerAgent must be Clone
    handles.push(rt.spawn(async move {
        let result = agent.audit_batch(batch).await;
        drop(permit);
        result
    }));
}
let results = join_all(handles).await;
```

**Retries:** each batch retries up to 2 times with 2s delay on failure. After 3 failures, records as `parse_failures` and continues — does not lose work done by other batches.

**Progress bar:** `indicatif` advances on completions (not order), so user sees real parallel progress.

**Performance:** 200-file project (~25 batches):
- Sequential (current): ~8 min
- `--concurrency 3` (default): ~3 min
- `--concurrency 5`: ~2 min

**Implementation:**
- `src/commands/mod.rs` — add `--concurrency` to `ProCommands::Audit`
- `src/commands/pro.rs` — replace sequential loop with `join_all`
- `src/agents/reviewer.rs` — derive `Clone` on `ReviewerAgent`

---

## Feature 4: `sentinel index` Subcommand + Stale Detection

### Approach: Explicit subcommand + auto-warning in check/audit

**New subcommand:**
```bash
sentinel index --check      # report status, no modifications
sentinel index --rebuild    # clear all tables + full reindex
```

**`--check` output:**
```
📊 Estado del índice:
   Archivos indexados:  142
   Archivos en disco:   158  (+16 nuevos no indexados)
   Último indexado:     2026-02-20 14:32
   Estado:              ⚠️  Desactualizado
```

**`--rebuild` behavior:**
- Clears tables: `file_index`, `symbols`, `call_graph`, `import_usage`
- Runs `ProjectIndexBuilder::index_project` with progress bar
- Reports completion summary

**Auto-stale detection in `check`/`audit`:**
Before dispatching analysis, count files on disk (project walk) vs in `file_index`.
If difference > 10% **or** > 5 files, print once (text mode only):
```
⚠️  Índice posiblemente desactualizado (142 indexados, 158 en disco).
   Corre `sentinel index --rebuild` para actualizar.
```
JSON mode: adds `"index_stale": true` field.

**Stale threshold:** `abs(disk_count - index_count) > max(5, disk_count / 10)`

**Implementation:**
- `src/commands/mod.rs` — new `Commands::Index { rebuild: bool, check: bool }`
- `src/commands/index.rs` — new module with `handle_index_command`
- `src/index/db.rs` — add `fn clear_all() -> Result<()>` + `fn indexed_file_count() -> usize`
- `src/commands/pro.rs` — stale check before `match subcommand`
- `src/main.rs` — wire `Commands::Index` to handler

---

## Feature 5: `sentinel ignore` + Feedback Loop

### Approach: CLI command + `.sentinel/ignore.json`

**New subcommand:**
```bash
sentinel ignore DEAD_CODE src/user.ts userId    # ignore rule+file+symbol
sentinel ignore UNUSED_IMPORT src/auth.ts        # ignore rule+file (all symbols)
sentinel ignore list                             # show all ignored entries
sentinel ignore clear src/user.ts               # remove all ignores for a file
```

**Storage — `.sentinel/ignore.json`** (committable to repo):
```json
{
  "version": 1,
  "entries": [
    { "rule": "DEAD_CODE", "file": "src/user.ts", "symbol": "userId", "added": "2026-02-23" },
    { "rule": "UNUSED_IMPORT", "file": "src/auth.ts", "symbol": null, "added": "2026-02-23" }
  ]
}
```

**Filtering in `check`:**
After running static analysis, before displaying results, filter violations where:
- `violation.rule_name == entry.rule`
- `violation.file_path == entry.file`
- If `entry.symbol` is Some: `violation.symbol == entry.symbol`

Filtered violations are suppressed in both text and JSON output.

**Hint in check output (text mode):**
```
💡 Para ignorar una violación: sentinel ignore DEAD_CODE src/user.ts userId
```

**Implementation:**
- `src/commands/mod.rs` — new `Commands::Ignore` with subcommands
- `src/commands/ignore.rs` — new module with `handle_ignore_command`
- `src/commands/pro.rs` check handler — load `.sentinel/ignore.json` + filter violations
- `src/rules/mod.rs` — `RuleViolation.symbol` field already exists (added in precision pass)

---

## Success Criteria

- `sentinel pro review` on a 100-file NestJS project includes symbol/call-graph context in the output.
- `sentinel pro audit src/ --concurrency 4` processes batches in parallel; progress bar advances concurrently.
- `sentinel index --check` reports correct file counts; `--rebuild` clears and reindexes.
- `check/audit` warns when >10% file count difference between disk and index.
- `sentinel ignore DEAD_CODE src/user.ts userId` → next `check` does not show that violation.
- All existing 37 tests continue to pass.

---

## Out of Scope

- Vector/semantic search (embeddings) for review context
- Incremental symbol cache invalidation on import changes
- `// @sentinel-ignore` inline comments in source files
- Distributed index (multi-machine)
- Semantic deduplication across different rule titles
