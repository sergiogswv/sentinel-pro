# Sentinel Roadmap — Diseño Completo

**Fecha:** 2026-02-24
**Objetivo:** Roadmap completo organizado por capas técnicas para convertir Sentinel en un producto confiable, distribuible y diferenciado como la herramienta todo-en-uno (linting estático + IA + monitor en tiempo real + git) para dev individual y equipos.

**Audiencia:** Dev individual + Tech Lead / equipo con CI/CD.
**Diferenciador central:** Integración todo-en-uno — un solo binario, sin configuración compleja.

---

## Capa 1: Confiabilidad

Arreglar los problemas que hacen que Sentinel sea ruidoso e ignorable hoy.

### 1.1 Word-boundary matching en análisis estático
Reemplazar `.matches(name).count()` en `static_analysis.rs` con regex `\bname\b` (usando la crate `regex` ya en Cargo.toml). Afecta `DEAD_CODE` y `UNUSED_IMPORT`. Reducción estimada de falsos positivos: 60-70% en proyectos TypeScript reales.

**Archivos:** `src/rules/static_analysis.rs`

### 1.2 Números de línea en todas las violaciones
El campo `line: Option<usize>` existe en `RuleViolation` pero no se propaga al display ni al JSON output. Asegurar que todos los analizadores (TypeScript, Go, Python) emitan líneas. El output pasa de `[DEAD_CODE]: función 'process'` a `[DEAD_CODE:47]: función 'process'`.

**Archivos:** `src/rules/static_analysis.rs`, `src/rules/languages/*.rs`, `src/commands/pro.rs`

### 1.3 Audit modo no-interactivo
Agregar `--no-fix` y `--format json` a `sentinel pro audit`. Sin estos flags, audit es incompatible con CI/CD porque lanza un menú interactivo (dialoguer falla sin TTY). Exit code 1 si hay errores. Quitar el cap de 10 issues en el MultiSelect (aumentar a 20 con aviso de truncación).

**Archivos:** `src/commands/mod.rs`, `src/commands/pro.rs`

### 1.4 RuleConfig thresholds efectivos
El `complexity_threshold` y `function_length_threshold` configurados en TOML son ignorados si están por debajo del floor hardcodeado (10 y 50 respectivamente) en los analizadores. Eliminar los floors hardcodeados y hacer que los analizadores lean el umbral del config como única fuente de verdad.

**Archivos:** `src/rules/languages/go.rs`, `src/rules/languages/python.rs`, `src/rules/static_analysis.rs`, `src/rules/engine.rs`

### 1.5 Cobertura de tests para paths de error
Agregar tests para: `is_process_alive` con PID propio (siempre vivo) y PID inexistente, `handle_status` con PID file stale, `read_pid_file` con contenido corrupto, y al menos 2 tests de integración básicos en `pro.rs` para `check --format json`.

**Archivos:** `src/commands/monitor.rs`, `src/commands/pro.rs`

---

## Capa 2: Experiencia de Desarrollador (DX/UX)

Hacer que Sentinel sea un placer usar, tanto para el dev individual como para el equipo.

### 2.1 `sentinel init` — onboarding guiado
Nuevo comando que detecta el lenguaje/framework del proyecto, genera `.sentinel/config.toml` con valores razonables, y muestra qué reglas están activas. Tiempo de first-value objetivo: < 30 segundos.

**Archivos:** `src/commands/mod.rs`, `src/commands/init.rs` (nuevo), `src/main.rs`

### 2.2 Refactorizar `pro.rs`
`src/commands/pro.rs` tiene ~1600+ líneas mezclando handlers, structs locales, lógica de display y llamadas a IA. Dividir en módulos: `src/commands/pro/check.rs`, `pro/audit.rs`, `pro/review.rs`, `pro/render.rs`, `pro/mod.rs`.

**Archivos:** `src/commands/pro.rs` → `src/commands/pro/`

### 2.3 Output agrupado por archivo en `check`
Agrupar violations por archivo y ordenar por nivel (error > warning > info). Formato:
```
src/services/user.service.ts
  ❌ :47  DEAD_CODE     — función 'processLegacy' nunca usada
  ⚠️  :89  HIGH_COMPLEX  — complejidad 14 (máx 10)
```

**Archivos:** `src/commands/pro/check.rs`

### 2.4 `sentinel doctor` — diagnóstico del entorno
Comando que verifica: config válida, API key configurada, índice SQLite actualizado, lenguajes activos, versión instalada vs última disponible.

**Archivos:** `src/commands/mod.rs`, `src/commands/doctor.rs` (nuevo), `src/main.rs`

### 2.5 Flags globales `--quiet` y `--verbose`
`--quiet`: solo errores, exit code. `--verbose`: debug info, queries tree-sitter, tiempos de ejecución. Críticos para CI y debugging respectivamente.

**Archivos:** `src/commands/mod.rs`, `src/commands/pro.rs`

### 2.6 `.sentinelignore` por directorio
Soporte para archivos `.sentinelignore` en subdirectorios (como `.gitignore`). El comando `sentinel ignore --show-file` muestra la ruta del archivo de config activo para el directorio actual.

**Archivos:** `src/commands/ignore.rs`, `src/config.rs`

---

## Capa 3: Expansión

Ampliar el alcance de Sentinel para cubrir más lenguajes, reglas e integraciones.

### 3.1 Soporte Java y Rust
Java: mercado enterprise que usa SonarQube — target de reemplazo directo. Rust: natural dado que Sentinel mismo es Rust. Ambos tienen grammars tree-sitter maduros. Siguiendo el patrón establecido, ~200 líneas + tests cada uno.

**Archivos:** `Cargo.toml`, `src/rules/languages/java.rs` (nuevo), `src/rules/languages/rust_lang.rs` (nuevo), `src/rules/languages/mod.rs`, `src/index/builder.rs`

### 3.2 Reglas custom en YAML expandidas
Expandir `.sentinel/rules.yaml` para soportar: queries tree-sitter custom, mensajes personalizados, niveles configurables. Un Tech Lead puede escribir `"ningún controller puede importar directamente de repositories"` sin tocar código Rust.

**Archivos:** `src/rules/engine.rs`, `src/rules/mod.rs`

### 3.3 GitHub Actions nativa
Publicar una GitHub Action oficial (`uses: sentinel-pro/action@v1`) que instale el binario, corra `check --format sarif`, y suba el reporte a GitHub Security tab. El SARIF ya está implementado — solo falta el `action.yml` y la distribución.

**Archivos:** `.github/actions/sentinel/action.yml` (nuevo), documentación

### 3.4 Pre-commit hook installer
`sentinel install-hook` agrega un hook en `.git/hooks/pre-commit` que corre `sentinel pro check`. Exit code 1 bloquea el commit. Flag `--warn-only` para equipos que quieren visibilidad sin bloquear.

**Archivos:** `src/commands/mod.rs`, `src/commands/hooks.rs` (nuevo)

### 3.5 VS Code extension básica
Extensión que corre `sentinel pro check --format json` en background y muestra violations como diagnósticos en el editor (subrayado rojo/amarillo en la línea correcta) vía `vscode.languages.createDiagnosticCollection`. No requiere LSP completo.

**Archivos:** Repositorio separado `sentinel-pro-vscode/`

### 3.6 Métricas de tendencia en audit
Tracking temporal en SQLite: comparar estado actual vs semana anterior. `"12 DEAD_CODE vs 18 la semana pasada — mejoraste 33%"`. Para Tech Leads, la tendencia es más valiosa que el snapshot.

**Archivos:** `src/stats.rs`, `src/commands/pro/audit.rs`

---

## Capa 4: Producto

Convertir Sentinel de herramienta técnica a producto distribuible y mantenible.

### 4.1 Distribución — binarios pre-compilados + Homebrew
GitHub Actions que compile y publique binarios para Linux x86_64, Linux aarch64, macOS arm64, macOS x86_64 en cada release. Homebrew tap (`brew install sentinel-pro/tap/sentinel`). Soporte `cargo-binstall`. Objetivo: instalación en < 10 segundos.

**Archivos:** `.github/workflows/release.yml` (nuevo)

### 4.2 Documentación — sitio estático
Sitio con mdBook: instalación, referencia de comandos, catálogo de reglas por lenguaje con ejemplos, guía de config TOML, guía CI/CD. Sin documentación navegable el onboarding de un equipo depende de leer el código.

**Archivos:** `docs/book/` (nuevo)

### 4.3 CHANGELOG y versionado semántico
Conventional commits + `git-cliff` para CHANGELOG automático. Los usuarios necesitan saber qué cambió entre versiones para confiar en actualizar. Actualmente en `5.0.0-pro.beta.3` sin CHANGELOG visible.

**Archivos:** `CHANGELOG.md`, `.github/workflows/release.yml`

### 4.4 Telemetría opt-in
Datos anónimos de uso: comandos más usados, lenguajes más comunes, tipos de violations más frecuentes. Completamente opt-in (`sentinel telemetry --enable`). Crítico para decisiones de producto basadas en datos.

**Archivos:** `src/telemetry.rs` (nuevo), `src/config.rs`

### 4.5 `sentinel update`
Verificación de versión nueva en GitHub Releases API. Silencioso al inicio del proceso: `⚡ Nueva versión disponible: v5.1.0. Corre 'sentinel update' para instalar.` El dev no tiene que recordar `cargo install` manualmente.

**Archivos:** `src/commands/mod.rs`, `src/commands/update.rs` (nuevo)

### 4.6 Feature flag para dependencias ML
Mover `candle-core`, `candle-nn`, `candle-transformers`, `tokenizers` a un feature flag: `cargo build` da binario ligero (< 10MB), `cargo build --features ml-local` incluye ML local. Reduce tiempo de build para contribuidores y tamaño del binario para usuarios que usan IA cloud.

**Archivos:** `Cargo.toml`

---

## Resumen de capas

| Capa | Tareas | Impacto | Urgencia |
|------|--------|---------|----------|
| 1. Confiabilidad | 5 | Crítico — falsos positivos destruyen credibilidad | Inmediata |
| 2. DX/UX | 6 | Alto — retención de usuarios | Corto plazo |
| 3. Expansión | 6 | Alto — alcance de mercado | Mediano plazo |
| 4. Producto | 6 | Crítico para distribución | Mediano plazo |

**Total: 23 tareas**

## Orden de implementación recomendado

Capa 1 completa → Capa 2 (2.1, 2.2, 2.3 primero) → Capa 4 (4.1, 4.6 — distribución y build) → Capa 3 → resto de Capa 2 y 4.

La distribución (4.1) se adelanta porque sin binarios pre-compilados es difícil obtener feedback real de usuarios externos.
