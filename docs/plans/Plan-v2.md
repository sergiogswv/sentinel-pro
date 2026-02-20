# Critical Gaps Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Cerrar los 5 gaps críticos que impiden que Sentinel sea una herramienta de calidad confiable.

**Architecture:** Cuatro mejoras incrementales sobre código existente + un feature nuevo (BusinessLogicGuard). No se crea ningún módulo nuevo — todo encaja en la estructura actual. El BusinessLogicGuard usa `git show HEAD:<file>` para obtener el estado anterior del archivo sin necesidad de snapshots propios.

**Tech Stack:** Rust 2024, tree-sitter 0.26, rusqlite 0.31, git (ya existe `src/git.rs`), std::process::Command para git diff

---

## Contexto para el implementador

Antes de tocar código, leer estos archivos:
- `src/rules/static_analysis.rs` — los 4 analizadores actuales
- `src/commands/monitor.rs:202-300` — el loop principal del watcher
- `src/index/call_graph.rs` — tiene un bug de SQL injection
- `src/agents/base.rs` — `build_rag_context()` ya está implementado y funcionando
- `src/agents/reviewer.rs` — ya usa RAG context correctamente

**Lo que ya funciona (NO tocar):**
- File watcher → IndexDb: `monitor.rs:226` ya llama `index_builder.index_file()`
- RAG context: `base.rs:45-73` ya lee de SQLite y se pasa al ReviewerAgent

---

## Task 1: Fix SQL Injection en CallGraph

**Prioridad: URGENTE** — bug de seguridad activo

**Files:**
- Modify: `src/index/call_graph.rs`

### Step 1: Escribir el test que expone el bug

En `src/index/call_graph.rs`, al final del archivo agregar:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::db::IndexDb;
    use tempfile::NamedTempFile;

    fn make_db() -> (NamedTempFile, std::sync::Arc<IndexDb>) {
        let f = NamedTempFile::new().unwrap();
        let db = std::sync::Arc::new(IndexDb::open(f.path()).unwrap());
        (f, db)
    }

    #[test]
    fn test_get_dead_code_with_special_chars_does_not_panic() {
        let (_f, db) = make_db();
        let cg = CallGraph::new(&db);
        // Un path con comilla simple causaría SQL injection / panic con la implementación actual
        let result = cg.get_dead_code(Some("src/user's-service.ts"));
        assert!(result.is_ok());
    }
}
```

### Step 2: Correr el test para verificar que falla

```bash
cd /home/protec/Documentos/dev/sentinel-pro
cargo test call_graph::tests::test_get_dead_code_with_special_chars_does_not_panic -- --nocapture 2>&1 | tail -20
```

Esperado: puede pasar o hacer panic dependiendo del sistema. El problema real es que la query usa `format!()` en lugar de parámetros.

### Step 3: Implementar la corrección

Reemplazar `src/index/call_graph.rs` completo con:

```rust
use crate::index::db::IndexDb;
use rusqlite::params;

pub struct CallGraph<'a> {
    db: &'a IndexDb,
}

impl<'a> CallGraph<'a> {
    pub fn new(db: &'a IndexDb) -> Self {
        Self { db }
    }

    pub fn get_dead_code(&self, file_path: Option<&str>) -> anyhow::Result<Vec<String>> {
        let conn = self.db.lock();
        let mut results = Vec::new();

        if let Some(path) = file_path {
            let mut stmt = conn.prepare(
                "SELECT name FROM symbols \
                 WHERE kind IN ('function', 'method') \
                 AND file_path = ? \
                 AND name NOT IN (SELECT callee_symbol FROM call_graph)",
            )?;
            let rows = stmt.query_map(params![path], |row| row.get(0))?;
            for row in rows {
                results.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT name FROM symbols \
                 WHERE kind IN ('function', 'method') \
                 AND name NOT IN (SELECT callee_symbol FROM call_graph)",
            )?;
            let rows = stmt.query_map([], |row| row.get(0))?;
            for row in rows {
                results.push(row?);
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::db::IndexDb;
    use tempfile::NamedTempFile;

    fn make_db() -> (NamedTempFile, std::sync::Arc<IndexDb>) {
        let f = NamedTempFile::new().unwrap();
        let db = std::sync::Arc::new(IndexDb::open(f.path()).unwrap());
        (f, db)
    }

    #[test]
    fn test_get_dead_code_with_special_chars_does_not_panic() {
        let (_f, db) = make_db();
        let cg = CallGraph::new(&db);
        let result = cg.get_dead_code(Some("src/user's-service.ts"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_dead_code_returns_empty_when_no_symbols() {
        let (_f, db) = make_db();
        let cg = CallGraph::new(&db);
        let result = cg.get_dead_code(None).unwrap();
        assert!(result.is_empty());
    }
}
```

**Nota:** Agregar `tempfile` a Cargo.toml si no existe:
```toml
[dev-dependencies]
tempfile = "3"
```

### Step 4: Correr tests

```bash
cargo test call_graph -- --nocapture 2>&1 | tail -20
```

Esperado: `test result: ok. 2 passed`

### Step 5: Commit

```bash
git add src/index/call_graph.rs Cargo.toml
git commit -m "fix(index): use parameterized queries in CallGraph to prevent SQL injection"
```

---

## Task 2: BusinessLogicGuard — Detección de Regresiones

**Prioridad: ALTA** — el feature más diferenciador del quality guardian

**Concepto:** Cuando el file watcher detecta un cambio, obtener el contenido anterior del archivo via `git show HEAD:<path>`. Si existe, enviarlo junto al contenido nuevo al ReviewerAgent con un prompt específico de regresión. Si no existe en git (archivo nuevo), saltar el guard.

**Files:**
- Create: `src/business_logic_guard.rs`
- Modify: `src/main.rs` (agregar `pub mod business_logic_guard;`)
- Modify: `src/commands/monitor.rs` (llamar al guard en el loop)

### Step 1: Escribir los tests

Crear `src/business_logic_guard.rs` con tests primero:

```rust
use std::path::Path;
use std::process::Command;

/// Obtiene el contenido del archivo en HEAD (último commit).
/// Retorna None si el archivo no está en git o si git no está disponible.
pub fn get_git_previous_content(file_path: &Path, project_root: &Path) -> Option<String> {
    let rel_path = file_path.strip_prefix(project_root).ok()?;
    let rel_str = rel_path.to_string_lossy();

    let output = Command::new("git")
        .args(["show", &format!("HEAD:{}", rel_str)])
        .current_dir(project_root)
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

/// Compara dos versiones del código y retorna un diff legible para el AI.
/// Retorna None si los archivos son idénticos o si prev_content es None.
pub fn build_regression_context(prev_content: &str, new_content: &str) -> Option<String> {
    if prev_content == new_content {
        return None;
    }

    let prev_lines: Vec<&str> = prev_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let removed: Vec<&str> = prev_lines.iter()
        .filter(|l| !new_lines.contains(l))
        .copied()
        .collect();

    let added: Vec<&str> = new_lines.iter()
        .filter(|l| !prev_lines.contains(l))
        .copied()
        .collect();

    if removed.is_empty() && added.is_empty() {
        return None;
    }

    let mut diff = String::from("CAMBIOS DETECTADOS (diff simplificado):\n");
    if !removed.is_empty() {
        diff.push_str("\nLÍNEAS ELIMINADAS:\n");
        for line in removed.iter().take(30) {
            diff.push_str(&format!("- {}\n", line));
        }
    }
    if !added.is_empty() {
        diff.push_str("\nLÍNEAS AGREGADAS:\n");
        for line in added.iter().take(30) {
            diff.push_str(&format!("+ {}\n", line));
        }
    }

    Some(diff)
}

/// Construye el prompt para detectar regresiones de lógica de negocio.
pub fn build_regression_prompt(diff_context: &str, file_name: &str) -> String {
    format!(
        "Actúa como un revisor de código experto en detección de regresiones.\n\
        Analiza los siguientes cambios en el archivo '{}' y determina:\n\
        1. ¿Se eliminó alguna validación, guard clause o lógica de negocio importante?\n\
        2. ¿Se cambió el comportamiento de alguna función de forma que pueda romper contratos existentes?\n\
        3. ¿Se eliminó manejo de errores o casos edge?\n\
        \n\
        Responde con:\n\
        - REGRESION_DETECTADA: [descripción concisa] si hay una regresión clara\n\
        - SIN_REGRESION si los cambios son seguros\n\
        - REVISAR: [motivo] si hay algo sospechoso pero no definitivo\n\
        \n\
        CAMBIOS A ANALIZAR:\n\
        {}\n",
        file_name, diff_context
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_regression_context_identical_files_returns_none() {
        let content = "function foo() { return 1; }";
        assert!(build_regression_context(content, content).is_none());
    }

    #[test]
    fn test_build_regression_context_detects_removed_lines() {
        let prev = "function validate(email) {\n  if (!email) throw new Error('required');\n  return true;\n}";
        let new = "function validate(email) {\n  return true;\n}";
        let result = build_regression_context(prev, new);
        assert!(result.is_some());
        let ctx = result.unwrap();
        assert!(ctx.contains("ELIMINADAS"));
        assert!(ctx.contains("throw new Error"));
    }

    #[test]
    fn test_build_regression_context_detects_added_lines() {
        let prev = "function foo() { return 1; }";
        let new = "function foo() { return 1; }\nfunction bar() { return 2; }";
        let result = build_regression_context(prev, new);
        assert!(result.is_some());
        assert!(result.unwrap().contains("AGREGADAS"));
    }

    #[test]
    fn test_build_regression_prompt_contains_file_name() {
        let prompt = build_regression_prompt("- old line\n+ new line", "user.service.ts");
        assert!(prompt.contains("user.service.ts"));
        assert!(prompt.contains("REGRESION_DETECTADA"));
        assert!(prompt.contains("SIN_REGRESION"));
    }
}
```

### Step 2: Correr tests para verificar que fallan

```bash
cargo test business_logic_guard -- --nocapture 2>&1 | tail -20
```

Esperado: error de compilación — módulo no existe todavía. Está bien, el archivo ya tiene el código.

### Step 3: Agregar el módulo a main.rs

En `src/main.rs`, agregar después de `pub mod index;`:

```rust
pub mod business_logic_guard;
```

### Step 4: Correr tests para verificar que pasan

```bash
cargo test business_logic_guard -- --nocapture 2>&1 | tail -20
```

Esperado: `test result: ok. 4 passed`

### Step 5: Integrar en monitor.rs

En `src/commands/monitor.rs`, al inicio del archivo agregar el import:
```rust
use crate::business_logic_guard;
```

En el loop principal, después de la línea 226 (`let _ = index_builder.index_file(...)`), agregar:

```rust
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
    if let Ok(result) = ai::consultar_ia_simple(&regression_prompt, &config_bg, stats_bg, &project_bg) {
        if result.contains("REGRESION_DETECTADA") {
            println!("   {} {}", "⚠️  REGRESIÓN:".red().bold(), result.lines().find(|l| l.contains("REGRESION_DETECTADA")).unwrap_or(""));
        } else if result.contains("REVISAR") {
            println!("   {} {}", "🔎 REVISAR:".yellow(), result.lines().find(|l| l.contains("REVISAR")).unwrap_or(""));
        } else {
            println!("   {} Sin regresiones de lógica de negocio detectadas.", "✅".green());
        }
    }
}
```

**Nota:** Si `ai::consultar_ia_simple` no existe, usar la función disponible más simple del módulo `ai`. Verificar con `grep -n "^pub fn" src/ai/*.rs` qué funciones están disponibles y usar la más apropiada.

### Step 6: Compilar para verificar

```bash
cargo build 2>&1 | grep -E "error|warning: unused" | head -30
```

Esperado: compila sin errores.

### Step 7: Commit

```bash
git add src/business_logic_guard.rs src/main.rs src/commands/monitor.rs
git commit -m "feat: add BusinessLogicGuard to detect logic regressions vs last git commit"
```

---

## Task 3: ComplexityAnalyzer — Longitud de Función y Profundidad de Anidamiento

**Files:**
- Modify: `src/rules/static_analysis.rs:104-182` (ComplexityAnalyzer)

### Step 1: Agregar tests al final de static_analysis.rs

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Language;

    fn ts_lang() -> Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    #[test]
    fn test_complexity_flags_long_function() {
        let lang = ts_lang();
        let analyzer = ComplexityAnalyzer::new();
        // Función de 55 líneas (sobre el límite de 50)
        let long_fn = format!(
            "function longFn() {{\n{}\n}}",
            "  const x = 1;\n".repeat(54)
        );
        let violations = analyzer.analyze(&lang, &long_fn);
        let has_length_violation = violations.iter().any(|v| v.rule_name == "FUNCTION_TOO_LONG");
        assert!(has_length_violation, "Debería detectar función de más de 50 líneas");
    }

    #[test]
    fn test_complexity_ok_for_short_function() {
        let lang = ts_lang();
        let analyzer = ComplexityAnalyzer::new();
        let short_fn = "function shortFn() {\n  return 1;\n}";
        let violations = analyzer.analyze(&lang, short_fn);
        let has_length_violation = violations.iter().any(|v| v.rule_name == "FUNCTION_TOO_LONG");
        assert!(!has_length_violation);
    }

    #[test]
    fn test_dead_code_detects_unused_function() {
        let lang = ts_lang();
        let analyzer = DeadCodeAnalyzer::new();
        let code = "function unusedFn() { return 42; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(!violations.is_empty(), "Debería detectar unusedFn como dead code");
    }

    #[test]
    fn test_unused_import_detected() {
        let lang = ts_lang();
        let analyzer = UnusedImportsAnalyzer::new();
        let code = "import { Injectable } from '@nestjs/common';\n\nfunction foo() { return 1; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(!violations.is_empty(), "Debería detectar Injectable como import no usado");
    }
}
```

### Step 2: Correr tests

```bash
cargo test static_analysis::tests -- --nocapture 2>&1 | tail -20
```

Esperado: `test_complexity_flags_long_function` falla porque no existe `FUNCTION_TOO_LONG`.

### Step 3: Agregar detección de longitud en ComplexityAnalyzer

En `src/rules/static_analysis.rs`, dentro del `impl StaticAnalyzer for ComplexityAnalyzer`, en el método `analyze`, agregar DESPUÉS del bloque de ciclomática (después de la línea que cierra el `while let Some((m, _)) = funcs.next()`):

```rust
        // Detectar funciones demasiado largas (> 50 líneas)
        let mut f_cursor2 = QueryCursor::new();
        let func_q2 = match Query::new(language, func_query) {
            Ok(q) => q,
            Err(_) => Query::new(language, "(function_declaration) @func").unwrap(),
        };
        let mut funcs2 = f_cursor2.captures(&func_q2, root_node, source_code.as_bytes());
        while let Some((m, _)) = funcs2.next() {
            for capture in m.captures {
                let node = capture.node;
                let start_line = node.range().start_point.row;
                let end_line = node.range().end_point.row;
                let line_count = end_line.saturating_sub(start_line);
                if line_count > 50 {
                    violations.push(RuleViolation {
                        rule_name: "FUNCTION_TOO_LONG".to_string(),
                        message: format!(
                            "Función de {} líneas (máximo recomendado: 50). Considera dividirla en funciones más pequeñas.",
                            line_count
                        ),
                        level: RuleLevel::Warning,
                    });
                }
            }
        }
```

### Step 4: Correr tests

```bash
cargo test static_analysis::tests -- --nocapture 2>&1 | tail -20
```

Esperado: `test result: ok. 4 passed`

### Step 5: Commit

```bash
git add src/rules/static_analysis.rs
git commit -m "feat(analyzer): add function length detection to ComplexityAnalyzer (max 50 lines)"
```

---

## Task 4: NamingAnalyzer Framework-Aware

**Files:**
- Modify: `src/rules/static_analysis.rs:184-227` (NamingAnalyzer)
- Modify: `src/rules/engine.rs` (pasar framework al NamingAnalyzer)

**Contexto:** Actualmente NamingAnalyzer siempre asume que snake_case es malo (TypeScript). Pero Python/Django usan snake_case correctamente.

### Step 1: Agregar tests

Al final del bloque `#[cfg(test)]` en `static_analysis.rs`:

```rust
    #[test]
    fn test_naming_snake_case_ok_in_python_context() {
        // En Python, snake_case es CORRECTO — no debe dar violación
        // El NamingAnalyzer necesita saber el framework para decidir
        // Por ahora validamos que la función acepta un parámetro de framework
        let analyzer = NamingAnalyzerWithFramework::new("django");
        let lang = ts_lang(); // usamos TS como proxy para el test
        let code = "function my_function() { return 1; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(violations.is_empty(), "En Django/Python, snake_case no debería ser violación");
    }

    #[test]
    fn test_naming_snake_case_bad_in_typescript() {
        let analyzer = NamingAnalyzerWithFramework::new("nestjs");
        let lang = ts_lang();
        let code = "function my_function() { return 1; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(!violations.is_empty(), "En NestJS/TS, snake_case sí es violación");
    }
```

### Step 2: Correr tests para verificar que fallan

```bash
cargo test static_analysis::tests::test_naming -- --nocapture 2>&1 | tail -10
```

Esperado: error de compilación — `NamingAnalyzerWithFramework` no existe.

### Step 3: Implementar NamingAnalyzerWithFramework

En `src/rules/static_analysis.rs`, reemplazar la struct `NamingAnalyzer` y su impl con:

```rust
/// Analizador de convenciones de nombres (framework-aware)
pub struct NamingAnalyzer;

impl NamingAnalyzer {
    pub fn new() -> Self { Self }
}

pub struct NamingAnalyzerWithFramework {
    /// "nestjs" | "django" | "laravel" | "spring" | "typescript" | "javascript"
    framework: String,
}

impl NamingAnalyzerWithFramework {
    pub fn new(framework: &str) -> Self {
        Self { framework: framework.to_lowercase() }
    }

    fn expects_snake_case(&self) -> bool {
        matches!(self.framework.as_str(), "django" | "python" | "laravel" | "php")
    }

    pub fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        parser.set_language(language).ok();

        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root_node = tree.root_node();

        let query_str = r#"
            (variable_declarator name: (identifier) @var_name)
            (function_declaration name: (identifier) @func_name)
        "#;
        let query = Query::new(language, query_str)
            .unwrap_or_else(|_| Query::new(language, "(function_declaration) @f").unwrap());
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root_node, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                let has_snake = name.contains('_') && !name.chars().next().unwrap_or(' ').is_uppercase();

                if self.expects_snake_case() {
                    // Python/PHP: camelCase ES la violación
                    let has_camel = name.chars().any(|c| c.is_uppercase())
                        && !name.chars().next().unwrap_or(' ').is_uppercase();
                    if has_camel {
                        violations.push(RuleViolation {
                            rule_name: "NAMING_CONVENTION".to_string(),
                            message: format!(
                                "'{}' usa camelCase. Se recomienda snake_case para {}.",
                                name, self.framework
                            ),
                            level: RuleLevel::Info,
                        });
                    }
                } else {
                    // TypeScript/JS: snake_case ES la violación
                    if has_snake {
                        violations.push(RuleViolation {
                            rule_name: "NAMING_CONVENTION".to_string(),
                            message: format!(
                                "'{}' usa snake_case. Se recomienda camelCase para {}.",
                                name, self.framework
                            ),
                            level: RuleLevel::Info,
                        });
                    }
                }
            }
        }
        violations
    }
}

impl StaticAnalyzer for NamingAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        // Default: TypeScript/camelCase
        NamingAnalyzerWithFramework::new("typescript").analyze(language, source_code)
    }
}
```

### Step 4: Actualizar RuleEngine para pasar el framework

En `src/rules/engine.rs`, en la función `validate_file` donde se llama al NamingAnalyzer, reemplazar:

```rust
// ANTES:
violations.extend(self.naming_analyzer.analyze(&lang, content));

// DESPUÉS:
let framework = self.framework_def.as_ref()
    .map(|f| f.framework.as_str())
    .unwrap_or("typescript");
let naming_violations = crate::rules::static_analysis::NamingAnalyzerWithFramework::new(framework)
    .analyze(&lang, content);
violations.extend(naming_violations);
```

Si la estructura de `engine.rs` es diferente, buscar dónde se instancia `NamingAnalyzer` y adaptarlo.

### Step 5: Correr tests

```bash
cargo test static_analysis::tests -- --nocapture 2>&1 | tail -20
```

Esperado: `test result: ok. 6 passed`

### Step 6: Compilar

```bash
cargo build 2>&1 | grep "^error" | head -10
```

### Step 7: Commit

```bash
git add src/rules/static_analysis.rs src/rules/engine.rs
git commit -m "feat(analyzer): make NamingAnalyzer framework-aware (snake_case for Python/PHP, camelCase for TS/JS)"
```

---

## Task 5: Framework Profiles YAML — Django, Laravel, Spring

**Files:**
- Create: `.sentinel/profiles/django.yaml`
- Create: `.sentinel/profiles/laravel.yaml`
- Create: `.sentinel/profiles/spring.yaml`
- Modify: `src/config.rs` (auto-detectar framework por archivos del proyecto)

### Step 1: Verificar la estructura YAML existente para NestJS

```bash
find /home/protec/Documentos/dev/sentinel-pro -name "*.yaml" -path "*/sentinel*" 2>/dev/null
cat /home/protec/Documentos/dev/sentinel-pro/docs/ -la 2>/dev/null || true
```

Leer también `src/rules/mod.rs` para entender la estructura de `FrameworkDefinition`.

### Step 2: Crear el profile de Django

Crear `src/config/profiles/django.yaml`:

```yaml
framework: django
language: python
rules:
  - name: "No SQL crudo sin parámetros"
    description: "Usar ORM de Django o queries parametrizadas. SQL crudo es vulnerable a injection."
    forbidden_patterns:
      - "execute(\""
      - 'execute('''
    required_imports: []
    level: error

  - name: "Modelos deben tener __str__"
    description: "Todo modelo Django debe implementar __str__ para debugging legible."
    forbidden_patterns: []
    required_imports: []
    level: warning

  - name: "No lógica de negocio en views"
    description: "Las views deben delegar a services o managers. Mantener views delgadas."
    forbidden_patterns:
      - "objects.filter("
      - "objects.create("
    required_imports: []
    level: warning

architecture_patterns:
  - name: "Models en models.py o models/"
    selector: "**/*model*.py"
    expected_layer: "models"
  - name: "Views en views.py o views/"
    selector: "**/*view*.py"
    expected_layer: "views"
  - name: "Tests en tests.py o tests/"
    selector: "**/test_*.py"
    expected_layer: "tests"
```

### Step 3: Crear el profile de Laravel

Crear `src/config/profiles/laravel.yaml`:

```yaml
framework: laravel
language: php
rules:
  - name: "No queries directas en Controllers"
    description: "Usar Eloquent ORM o Repository pattern. No DB::select() en controllers."
    forbidden_patterns:
      - "DB::select("
      - "DB::insert("
      - "DB::statement("
    required_imports: []
    level: error

  - name: "Validación en Form Requests"
    description: "La validación debe estar en Form Request classes, no en el controller."
    forbidden_patterns:
      - "$request->validate("
    required_imports: []
    level: warning

  - name: "No lógica en Migrations"
    description: "Las migrations solo deben modificar el schema, no insertar datos de negocio."
    forbidden_patterns:
      - "DB::table("
    required_imports: []
    level: info

architecture_patterns:
  - name: "Controllers en Http/Controllers"
    selector: "**/*Controller.php"
    expected_layer: "Http/Controllers"
  - name: "Models en Models/"
    selector: "**/*Model.php"
    expected_layer: "app/Models"
  - name: "Tests en tests/"
    selector: "**/Test*.php"
    expected_layer: "tests"
```

### Step 4: Crear el profile de Spring

Crear `src/config/profiles/spring.yaml`:

```yaml
framework: spring
language: java
rules:
  - name: "Services deben ser @Service"
    description: "Toda clase de servicio debe tener la anotación @Service para que Spring la gestione."
    forbidden_patterns: []
    required_imports:
      - "@Service"
    level: error

  - name: "No @Autowired en campos (usar constructor injection)"
    description: "Usar constructor injection en lugar de field injection para testabilidad."
    forbidden_patterns:
      - "@Autowired\n    private"
      - "@Autowired\r\n    private"
    required_imports: []
    level: warning

  - name: "Controllers deben ser @RestController o @Controller"
    description: "Todo endpoint REST debe tener @RestController."
    forbidden_patterns: []
    required_imports:
      - "@RestController"
    level: error

architecture_patterns:
  - name: "Controllers en controller/"
    selector: "**/*Controller.java"
    expected_layer: "controller"
  - name: "Services en service/"
    selector: "**/*Service.java"
    expected_layer: "service"
  - name: "Repositories en repository/"
    selector: "**/*Repository.java"
    expected_layer: "repository"
```

### Step 5: Mejorar auto-detección de framework en config.rs

Leer `src/config.rs` primero para entender la estructura actual. Luego en la función de detección de framework, agregar lógica para detectar por archivos del proyecto:

```rust
pub fn detect_framework(project_root: &Path) -> String {
    // Django: manage.py + settings.py
    if project_root.join("manage.py").exists() && project_root.join("settings.py").exists() {
        return "django".to_string();
    }
    // Django alternativo
    if project_root.join("manage.py").exists() {
        return "django".to_string();
    }
    // Laravel: artisan + composer.json con laravel/framework
    if project_root.join("artisan").exists() {
        return "laravel".to_string();
    }
    // Spring: pom.xml con spring-boot o build.gradle con spring
    if project_root.join("pom.xml").exists() || project_root.join("build.gradle").exists() {
        if let Ok(content) = std::fs::read_to_string(project_root.join("pom.xml")) {
            if content.contains("spring-boot") {
                return "spring".to_string();
            }
        }
    }
    // NestJS: nest-cli.json o package.json con @nestjs/core
    if project_root.join("nest-cli.json").exists() {
        return "nestjs".to_string();
    }
    // Default
    "typescript".to_string()
}
```

Adaptar según la función actual en config.rs. No reescribir lo que ya funciona.

### Step 6: Compilar

```bash
cargo build 2>&1 | grep "^error" | head -10
```

### Step 7: Commit

```bash
git add src/config/profiles/ src/config.rs
git commit -m "feat(frameworks): add Django, Laravel, Spring profiles with architecture rules and improved auto-detection"
```

---

## Verificación Final

Una vez completadas las 5 tareas:

```bash
# Correr todos los tests
cargo test -- --nocapture 2>&1 | tail -30

# Compilar release para verificar optimizaciones
cargo build --release 2>&1 | grep "^error"

# Test de integración manual
cargo run -- monitor
# Modificar un archivo TypeScript del proyecto de prueba
# Verificar que aparece output de BusinessLogicGuard
```

Esperado: todos los tests pasan, binario compila en release, el monitor muestra los 3 layers en orden.

---

## Resumen de Cambios

| Task | Archivos | Impacto |
|------|----------|---------|
| 1. SQL Injection fix | `call_graph.rs` | Seguridad |
| 2. BusinessLogicGuard | `business_logic_guard.rs` (nuevo), `monitor.rs`, `main.rs` | Feature clave |
| 3. Function length | `static_analysis.rs` | Completar Capa 1 |
| 4. Naming framework-aware | `static_analysis.rs`, `engine.rs` | Multi-framework |
| 5. Framework profiles | 3 YAMLs nuevos, `config.rs` | Phase 2 inicio |
