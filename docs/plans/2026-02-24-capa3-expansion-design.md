# Capa 3 - Expansión: Diseño de Implementación

**Fecha**: 2026-02-24
**Estado**: Aprobado
**Enfoque**: Modular + Secuencial

## Visión General

Capa 3 expande Sentinel a soportar múltiples lenguajes, permitir reglas personalizadas, e integrar con herramientas de desarrollo estándar (git, GitHub, VS Code).

**Orden de implementación**:
1. Custom Rules System (fundación)
2. Java/Rust Support
3. Pre-commit Integration
4. GitHub Actions CI/CD
5. VS Code Extension

---

## 1. CUSTOM RULES SYSTEM

### Arquitectura
- Nuevo módulo: `src/rules/custom.rs`
- Directorio de configuración: `.sentinel/custom-rules/` (YAML + JSON)
- Dos tipos de rules:
  - **Pattern Rules**: regex + file patterns (simple)
  - **AST Rules**: Tree-sitter queries (avanzado)

### Estructura de un Custom Rule

**Pattern Rule (YAML)**:
```yaml
name: "No console.log in production"
type: "pattern"
pattern: "console\\.(log|warn|error)"
filePatterns: ["src/**/*.ts", "!src/**/*.test.ts"]
severity: "error"
message: "Remove console.log before committing"
```

**AST Rule (JSON)**:
```json
{
  "name": "No untyped function parameters",
  "type": "ast",
  "language": "typescript",
  "query": "(function_declaration parameters: (formal_parameters) @params (#not @params (type_annotation)))",
  "severity": "warning",
  "message": "Add type annotations to function parameters"
}
```

### Validación
- JSON Schema para validar estructura
- Parser unificado para YAML + JSON
- Comando: `sentinel rules --validate`
- Soporte para incluir archivos: `include: ["rules/*.yaml"]`

### Carga de rules
- Escanea `.sentinel/custom-rules/` automáticamente
- Combina con rules estáticas del motor existente
- Prioridad: custom rules > built-in rules
- Caché de rules compiladas (`.sentinel/.rules-cache`)

---

## 2. JAVA/RUST SUPPORT

### Análisis Estático
Módulo: `src/rules/language_support.rs`

**Java**:
- Dead code detection (variables declaradas no usadas)
- Unused imports
- Naming conventions (camelCase validations)
- Complexity analysis

**Rust**:
- Dead code detection
- Unused imports
- Naming conventions (snake_case validations)
- Borrow checker warnings análisis

### Custom Rules para Java/Rust
- Pattern Rules funcionan con regex directa
- AST Rules usan Tree-sitter queries especializadas por lenguaje
- Ejemplo Java AST:
```yaml
name: "No public fields in entities"
type: "ast"
language: "java"
query: "(class_declaration body: (class_body (field_declaration (modifiers) @mods)) @field (#contains @mods \"public\"))"
severity: "error"
```

### Detección automática
- Detecta en `.sentinelrc.toml` basado en archivos presentes:
  - `Cargo.toml` → Rust
  - `pom.xml` o `build.gradle` → Java
- Activa análisis de lenguaje automáticamente
- Preserva análisis de otros lenguajes (TypeScript, Python, etc.)

### Limitaciones (Fase actual)
- No incluye IA analysis (solo análisis estático)
- No tiene full feature parity con TypeScript
- Frameworks específicos no detectados (Spring, Tokio, etc.)

---

## 3. PRE-COMMIT INTEGRATION

### Instalación
Comando: `sentinel init-precommit`
- Genera `.git/hooks/pre-commit` ejecutable
- Instala automáticamente en `.git/hooks/`
- Verifica que git esté inicializado

### Configuración (`.sentinelrc.toml`)
```toml
[precommit]
enabled = true
checks = ["static-analysis", "custom-rules"]
# Opciones disponibles:
# - "static-analysis": Dead code, imports, complexity, naming
# - "custom-rules": Custom rules cargadas desde .sentinel/custom-rules/
# - "framework-detection": Detección de frameworks
# - "ai-analysis": Análisis con IA (si está configurada)

fail_on = "error"  # "error" bloquea, "warning" solo reporta
timeout = 30  # segundos
skip_on_commit_msg_pattern = "skip-sentinel|WIP"  # regex para bypass
```

### Flujo de ejecución
1. Usuario: `git commit`
2. Hook ejecuta: `sentinel check --mode precommit`
3. Valida solo archivos staged (`git diff --cached`)
4. Si `fail_on = "error"` y hay errores → bloquea commit (exit 1)
5. Si `fail_on = "warning"` → reporta pero permite commit
6. Usuario puede `git commit --no-verify` (con mensaje de advertencia)

### Output
- Colores: rojo (error), amarillo (warning), verde (ok)
- Sugiere: `sentinel fix <file>` para auto-fixes
- Tiempo esperado: < 5s en proyectos medianos
- No bloquea si timeout se agota (fallback seguro)

### Casos de uso
```bash
# Bypass para commits rápidos
git commit -m "WIP: work in progress"

# Force commit si es necesario
git commit --no-verify

# Chequear manualmente antes de commit
sentinel check --mode precommit
```

---

## 4. GITHUB ACTIONS CI/CD PIPELINE

### Templates provistos
Comando: `sentinel init-ci`
Genera `.github/workflows/`:

**a) `sentinel-analysis.yml`**
```yaml
name: Sentinel Analysis
on: [push, pull_request]
jobs:
  analysis:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3  # o setup-rust, etc.
      - run: cargo install sentinel-pro
      - run: sentinel audit --json --output report.json
      - uses: actions/upload-artifact@v3
        with:
          name: sentinel-report
          path: report.json
      - run: sentinel check  # Bloquea si hay errores críticos
```

**b) `sentinel-tests.yml`**
```yaml
name: Tests + Sentinel Check
on: [push, pull_request]
jobs:
  test-and-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: npm install  # o cargo, pip, etc.
      - run: npm test
      - run: cargo install sentinel-pro
      - run: sentinel check --mode ci
```

**c) `sentinel-security.yml`**
```yaml
name: Security Checks
on: [push, pull_request]
jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo install sentinel-pro
      - run: sentinel check --rule-type security
```

### Características
- Check runs integrados en UI de GitHub
- PR comments automáticos para issues encontrados
- Artifacts descargables con reportes JSON
- Bloquea merge si hay errores críticos (configurable)
- Compatible con workflows existentes

### Salida
- Formato JSON: `sentinel-report.json`
- Markdown summary en `$GITHUB_STEP_SUMMARY`
- Check run status visible en PR

---

## 5. VS CODE EXTENSION

### Identificación
- Nombre: `vscode-sentinel`
- Publisher: sentinel (en Marketplace)
- Requisitos: Sentinel instalado globalmente (`cargo install sentinel-pro`)

### Comandos disponibles
Accessible desde Command Palette (`Ctrl+Shift+P`):

```
Sentinel: Run Audit
  → Ejecuta sentinel audit en workspace
  → Muestra resultados en nuevo panel output
  → Permite seleccionar issues para fix

Sentinel: Fix Issues
  → Para archivo activo
  → Propone fixes disponibles
  → Aplica con confirmación del usuario

Sentinel: Check File
  → Valida archivo en editor
  → Muestra diagnostics inline
  → Respeta severity settings

Sentinel: Show Configuration
  → Abre .sentinelrc.toml en editor

Sentinel: Initialize
  → Corre sentinel init si no existe
  → Configura directorios necesarios

Sentinel: Open Custom Rules
  → Abre .sentinel/custom-rules/ en explorador
  → Sugiere templates para nuevas rules

Sentinel: View Metrics
  → Muestra stats de productividad (ROI)
```

### Diagnostics Integration
- Muestra errores/warnings inline en editor
- Integra con VS Code Problems panel
- Colores: rojo (error), amarillo (warning)
- Click para ver explanation
- Code actions para quick fixes

### Requisitos y setup
- Detecta `sentinel` en PATH automáticamente
- Si no lo encuentra: sugiere instalación
- Busca `.sentinelrc.toml` en workspace root
- Si no existe: ofrece crear con `sentinel init`

### Configuración
En `settings.json` (opcional):
```json
{
  "sentinel.enable": true,
  "sentinel.showProblems": true,
  "sentinel.autoFixOnSave": false,
  "sentinel.customRulesPath": ".sentinel/custom-rules"
}
```

---

## Data Flow Integrado

```
┌─────────────────────────────────────────────────┐
│         Developer Workflow (Local)              │
└────────┬────────────────────────────────────────┘
         │
         ├─▶ Edit code in VS Code
         │    ├─▶ VS Code Extension shows diagnostics (inline)
         │    └─▶ Command: "Sentinel: Fix Issues"
         │
         ├─▶ git add / git commit
         │    └─▶ Pre-commit hook
         │         └─▶ Runs: sentinel check --mode precommit
         │         └─▶ Validates staged files
         │         └─▶ Blocks if fail_on=error
         │
         └─▶ git push
              │
              └─▶ GitHub Actions triggered
                   ├─▶ sentinel-analysis.yml
                   ├─▶ sentinel-tests.yml
                   └─▶ sentinel-security.yml
                        └─▶ PR comments + artifacts
```

---

## Error Handling

### Custom Rules
- Rule validation errors → reporta en `sentinel rules --validate`
- Query syntax errors (AST) → fallback a pattern matching
- File pattern errors → logged, skip rule

### Java/Rust Support
- Parse errors → continue with next file
- Unsupported language → logged warning
- Missing parser → auto-download Tree-sitter grammar

### Pre-commit
- Hook not found → silent create
- Timeout → exit 0 (safe fail)
- Git not initialized → error with instructions

### GitHub Actions
- Artifact upload failure → continues
- Job timeout → cancels and reports
- Permissions error → logs suggestion

### VS Code Extension
- Sentinel not in PATH → shows notification + install link
- Parse error in settings.json → uses defaults
- Commands timeout → shows "Sentinel is taking longer..."

---

## Testing Strategy

### Custom Rules
- Unit tests para Pattern Rules (regex matching)
- Unit tests para AST Rules (Tree-sitter queries)
- Integration tests con archivos de ejemplo
- Validation schema tests

### Java/Rust Support
- Test files: `tests/java-analysis.rs`, `tests/rust-analysis.rs`
- Coverage: dead code, imports, naming
- Custom rule execution en Java/Rust

### Pre-commit
- Mock git environment
- Test hook generation
- Test config parsing
- Test file staging

### GitHub Actions
- Validate YAML syntax
- Test artifact generation
- Mock GitHub API calls

### VS Code Extension
- Extension activation tests
- Command execution tests
- Settings parsing tests
- Diagnostic reporting tests

---

## Milestones

1. **Semana 1**: Custom Rules System (completo + tests)
2. **Semana 2**: Java/Rust Support (análisis estático + custom rules)
3. **Semana 3**: Pre-commit Integration (hook generation + config)
4. **Semana 4**: GitHub Actions (3 templates + artifact handling)
5. **Semana 5**: VS Code Extension (6 commands + diagnostics)

**Criterio de éxito**: Cada componente funciona independientemente y se integra sin conflictos.

---

## Decisiones de Diseño

| Decisión | Rationale |
|----------|-----------|
| YAML + JSON para rules | Máxima flexibilidad, ya familiar con YAML |
| AST + Pattern rules | Accesible (pattern) + poderoso (AST) |
| Java/Rust sin IA (v1) | Evita complejidad innecesaria, puede agregarse luego |
| Pre-commit configurable | Cada equipo tiene diferentes necesidades |
| CI/CD pipeline completo | No solo linting, también tests y seguridad |
| VS Code sin UI custom | Pragmático, usa estándares VS Code |

---

## Riesgos y Mitigaciones

| Riesgo | Mitigación |
|--------|-----------|
| Complejidad de Tree-sitter queries | Documentación + ejemplos |
| Performance en proyectos grandes | Caché de rules, análisis incremental |
| Hook conflicts con otros pre-commit | Documentación de coexistencia |
| VS Code extension marketplace approval | Seguir guidelines, ícono/descripción claros |
| Java/Rust grammar updates | Pin Tree-sitter versions, test updates |

