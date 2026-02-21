# Roadmap 🛡️✨

Sentinel Pro's development roadmap, from its foundations to its future vision as the ultimate AI-powered code suite.

## Fase 1: Fundamentos Pro e Infraestructura Core (Completada ✅)
**Enfoque:** Base técnica y estructura de comandos.

- [x] **CLI Dispatcher**: Implementación con `clap` para subcomandos anidados (`sentinel pro <cmd>`).
- [x] **Initial Commands**: Stubs para `analyze`, `generate`, `refactor`, `fix`, `chat`.
- [x] **Framework Engine**: Detección automática de tecnología y carga de reglas YAML.

**Release:** v5.0.0-pro (Stable)

---

## Fase 2: Smart Indexing y Motor de Símbolos (Completada ✅)
**Enfoque:** Cerebro local standalone y grafos de dependencia (Lite Refocus).

- [x] **Tree-sitter Indexing**: Extracción de metadata multilingüe (funciones, clases, imports).
- [x] **SQLite Integration**: Migración de Qdrant a `rusqlite` para mayor portabilidad y velocidad.
- [x] **Structural Context**: Integración de memoria basada en grafos de llamadas en los Agentes.
- [x] **Incremental Watching**: Actualización automática del índice al guardar archivos en tiempo real.

**Release:** v5.0.0-pro.alpha.2 (Refocused in beta.3)

---

## Fase 3: Sistema Multi-Agent (Completada ✅)
**Enfoque:** Inteligencia autónoma especializada.

- [x] **Agentes Core**: `CoderAgent`, `ReviewerAgent`, `TesterAgent` y `RefactorAgent`.
- [x] **AgentOrchestrator**: Sistema de gestión y comunicación entre agentes especializados.
- [x] **Comandos Interactivos**: Integración de agentes en el flujo diario de la terminal.

**Release:** v5.0.0-pro.alpha.3

---

## Fase 4: Machine Learning Local (On-Device) (Completada ✅)
**Enfoque:** Privacidad y velocidad sin nube.

- [x] **Embeddings Offline**: Integración de `candle-transformers` con modelo `all-MiniLM-L6-v2`.
- [x] **Bug Prediction**: Heurísticas asistidas por ML para predecir fallos basados en complejidad.
- [x] **Code Style Analysis**: Generación automática de perfiles de estilo basados en el código existente.

**Release:** v5.0.0-pro.alpha.4

---

## Fase 5: Interfaz Pro y Experiencia REPL (Completada ✅)
**Enfoque:** UX premium y herramientas de chat.

- [x] **Chat Interactivo**: Terminal REPL (`sentinel pro chat`) para consultar el codebase.
- [x] **Backups de Refactor**: Sistema de seguridad para revertir cambios automáticos.
- [x] **Auto-Doc System**: Generación dinámica de reportes del proyecto (`sentinel pro docs`).

**Release:** v5.0.0-pro (Stage 1 Stable)

---

## Fase 6: Workflows Avanzados e Integración (Completada ✅)
**Enfoque:** Escenarios complejos multi-paso y automatización iterativa.

- [x] **Workflow Engine**: Sistema para encadenar agentes de forma autónoma.
- [x] **Workflows Predefinidos**: `fix-and-verify` (Fix + Refactor + Test), `review-security`.
- [x] **Framework Migration**: Comando `migrate` para transiciones controladas de tecnología.

**Release:** v5.0.0-pro.beta.1

---

## Fase 7: Calidad, Testing y Lanzamiento (Completada ✅)
**Enfoque:** Robustez, seguridad y preparación para beta pública.

- [x] **Hardening & Security**: Prevención de Path Traversal y Sandboxing de Tests.
- [x] **CI/CD Multi-plataforma**: GitHub Actions para auto-releases en Windows, Linux y macOS.
- [x] **TesterAgent**: Integración con planes de prueba autónomos y generación de especificaciones.

**Release:** v5.0.0-pro.beta.2

---

## Fase 8: Auditoría y Sistema ROI (Completada ✅)
**Enfoque:** Escalabilidad de auditoría y medición de valor.

- [x] **Project Audit**: Comando `pro audit <path>` con selección múltiple de fixes.
- [x] **ROI Accounting**: Tracking de tiempo ahorrado y costos de tokens en tiempo real.
- [x] **METRICS_SYSTEM.md**: Documentación técnica del sistema de valor aportado.

**Release:** v5.0.0-pro.beta.2

---

## Fase 9: Refocus: Quality Guardian y Smart Discovery (Completada ✅)
**Enfoque:** Resiliencia de infraestructura y análisis protector.

- [x] **Static Analysis L1**: Analizadores de código muerto, complejidad y nombres (Tree-sitter).
- [x] **Smart Discovery**: Búsqueda recursiva de configuración en directorios padres.
- [x] **SQLite KB**: Sustitución de Qdrant por SQLite para una experiencia "zero-config".

**Release:** v5.0.0-pro.beta.3

---

## Fase 10: Monetización y Subscripciones (SaaS) (En Progreso 🚧)
**Enfoque:** Modelo de negocio comercial y licenciamiento.

- [ ] **Licenciamiento Core**: Validación criptográfica local de llaves RSA/Ed25519.
- [ ] **Trial System**: 7-14 días de prueba automática vía Hardware ID.
- [ ] **Pasarela de Pagos**: Integración con Stripe / Lemon Squeezy para suscripciones.
- [ ] **Grace Period Offline**: Validación periódica permitiendo trabajo offline controlado.

**Target Version:** v5.1.0-pro

---

## 🔮 Futuro y Visión (Roadmap Extendido)

### 🔒 SecOps Guardián (Fase 11)
- Escaneo de secretos basado en entropía.
- Auditoría automática de vulnerabilidades en dependencias.

### 🔍 PR Mode (Fase 12)
- Integración nativa con GitHub/GitLab Pull Requests.
- Comentarios automáticos de revisión línea por línea.

---

## Cronograma de Releases

| Versión | Fecha | Enfoque | Estado |
|---------|-------|---------|--------|
| v4.5.0 | Feb 2025 | Multi-model AI & Framework Detection | ✅ Stable |
| v5.0.0-pro | Feb 2025 | Sentinel Pro Infrastructure & Local AI | ✅ Stable |
| v5.0.0-pro.beta.1 | Feb 2025 | Workflow Engine & Multi-Agent System | ✅ Stable |
| v5.0.0-pro.beta.2 | Feb 2025 | Auditoría & Sistema ROI | ✅ Stable |
| **v5.0.0-pro.beta.3** | **Feb 2025** | **Quality Guardian & SQLite KB (Refocus)** | **✅ Actual** |
| v5.1.0-pro | Q2 2025 | Monetization & Licensing System | 📋 Planned |

---

**Current Release:** v5.0.0-pro.beta.3  
**Last Update:** Febrero 20, 2026
