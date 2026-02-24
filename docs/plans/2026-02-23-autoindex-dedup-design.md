# Design: Auto-indexación Cold-Start + Deduplicación Audit

**Date:** 2026-02-23
**Status:** Approved
**Context:** Post precision-gaps pass. Two surgical improvements: (1) replace cold-start warning
with actual background indexing so cross-file analysis activates automatically on the next run,
(2) deduplicate audit issues across batches to eliminate duplicate reports.

---

## Problem Statement

1. **Cold-start indexing is manual** — users must know to run `sentinel monitor` before `check`.
   Even with the warning, many won't. Cross-file dead code detection stays broken indefinitely.

2. **Audit deduplication is missing** — module batching can send the same file to multiple LLM
   calls, producing duplicate issues with identical title + file_path. Clutters the report.

---

## Solution: Option A — Two Independent Surgical Changes

### Change 1 — Auto-indexación paralela en cold start

**File:** `src/commands/pro.rs` — `handle_pro_command()` setup block (~lines 141-152)

**New import:** `use crate::index::ProjectIndexBuilder;` in `pro.rs`

When `!is_populated()`, instead of printing a warning, spawn a background indexing thread:

```
if index_db.is_some() && !index_db.is_populated():
  // Only in text mode (JSON mode: suppress)
  if not json_mode:
    print "🧠 Indexando proyecto por primera vez... (segundo plano)"

  let db_clone = Arc::clone(&index_db)
  let root_clone = agent_context.project_root.clone()
  let extensions_clone = agent_context.config.file_extensions.clone()
  let index_handle = std::thread::spawn(move || {
    let builder = ProjectIndexBuilder::new(db_clone)
    builder.index_project(&root_clone, &extensions_clone)
  })
  // Store handle to join after match block
```

After the `match subcommand { ... }` block (at the end of `handle_pro_command`):

```
if let Some(handle) = index_handle {
  match handle.join() {
    Ok(Ok(_)) => if not json_mode: print "✅ Índice listo. Próxima ejecución tendrá análisis cross-file completo."
    _         => if not json_mode: print "⚠️  Error en indexación background (análisis cross-file no disponible)."
  }
}
```

**Key implementation detail:** `index_handle` is `Option<JoinHandle<anyhow::Result<()>>>`, initialized
to `None` before the match block. Set to `Some(...)` only when cold-start is detected.

**json_mode problem:** `json_mode` is determined per-subcommand inside the match, not at the
`handle_pro_command` level. Solution: peek at the subcommand before spawning to determine if
JSON mode is active:

```rust
let json_mode_global = match &subcommand {
    ProCommands::Check { format, .. } => format.to_lowercase() == "json",
    ProCommands::Audit { format, .. } => format.to_lowercase() == "json",
    _ => false,
};
```

This runs before the match block and gives us a flag to suppress messages.

**First-run behavior:**
- Check/audit/review run immediately (no blocking)
- Cross-file filtering: unavailable this run (race with background indexer)
- Index fully populated before command exits (join at end)
- **Next run:** `is_populated()` returns true → no indexing → cross-file filtering active

**The cold-start warning** ("ÍNDICE VACÍO") is removed from all three handlers — replaced entirely
by the startup message. No duplicate messaging.

### Change 2 — Deduplicación de issues en audit

**File:** `src/commands/pro.rs` — audit handler, after the batch loop ends (~line 2028)

After `pb.finish_and_clear()` and before `if all_issues.is_empty()`, insert:

```rust
// Deduplicar: misma combinación (título normalizado, archivo) → conservar solo primero
{
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    all_issues.retain(|issue| {
        seen.insert((issue.title.to_lowercase(), issue.file_path.clone()))
    });
}
```

Uses `Vec::retain()` in-place (no new allocation). `HashSet::insert()` returns `true` if the key
was new → the issue is kept.

Normalizes title to lowercase to catch LLM capitalization variations.
Two issues on different files with the same title → both kept (legitimately different).

**Unit test:** `test_audit_dedup_removes_duplicates` in a test module in `pro.rs`:
```rust
// Create 3 AuditIssue: two with same (title, file_path), one different → expect 2 after dedup
```

---

## Success Criteria

- `sentinel pro check src/` on a cold project shows "🧠 Indexando..." at start and
  "✅ Índice listo" at end. No "ÍNDICE VACÍO" warning.
- Second run: no indexing message (index is populated). Cross-file DEAD_CODE filtering active.
- `sentinel pro audit src/ --no-fix` with duplicate issues across batches → each
  (title, file_path) pair appears exactly once in the output.
- All 36 existing tests continue to pass.
- 1 new unit test for deduplication.

---

## Out of Scope

- Blocking indexation (index first, then check) — adds latency
- Incremental re-indexing on check (too expensive for the cold-start fix goal)
- Semantic deduplication across different titles — ruled out by user
