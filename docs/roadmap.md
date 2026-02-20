# Roadmap 🛡️✨

Sentinel Pro's development roadmap, from its foundations to its future vision as the ultimate AI-powered code suite.

## Fase 1: Fundamentos Pro e Infraestructura Core (Completada ✅)
**Enfoque:** Base técnica y estructura de comandos.

- [x] **CLI Dispatcher**: Implementación con `clap` para subcomandos anidados (`sentinel pro <cmd>`).
- [x] **Initial Commands**: Stubs para `analyze`, `generate`, `refactor`, `fix`, `chat`.
- [x] **Configuración Pro**: Soporte para proveedores LLM locales (Ollama/LM Studio).
- [x] **Framework Engine**: Detección automática de tecnología y carga de reglas YAML.

**Release:** v5.0.0-pro (Stable)

---

## Fase 2: Knowledge Base y Vector Store (Completada ✅)
**Enfoque:** Cerebro local y búsqueda semántica avanzada.

- [x] **Tree-sitter Indexing**: Extracción de metadata multilingüe (funciones, clases, imports).
- [x] **Qdrant Integration**: Setup de base de datos vectorial local.
- [x] **Semantic RAG**: Integración de memoria semántica en los Agentes para mayor contexto.
- [x] **Incremental Watching**: Actualización automática del índice al guardar archivos.

**Release:** v5.0.0-pro.alpha.2

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

- [x] **Embeddings Offline**: Integración de `candle-transformers` para indexación local.
- [x] **Bug Prediction Stubs**: Preparación para modelos ONNX de análisis de complejidad.
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
- [x] **Architectural Audit**: Comando `review` para diagnósticos completos de salud.

**Release:** v5.0.0-pro.alpha.4

---

## Fase 7: Calidad, Testing y Lanzamiento (Completada ✅)
**Enfoque:** Robustez, seguridad y preparación para beta pública.

- [x] **Hardening & Security**: Prevención de Path Traversal y Sandboxing de Tests.
- [x] **CI/CD Multi-plataforma**: GitHub Actions para auto-releases en Windows, Linux y macOS.
- [x] **Testing Avanzado**: TesterAgent integrado con planes de prueba autónomos.
- [x] **Documentation Website**: Setup oficial de `website` con Docusaurus.

**Release:** v5.0.0-pro.beta.1 (Actual)

---

## Fase 8: Monetización y Subscripciones (SaaS) (En Progreso 🚧)
**Enfoque:** Modelo de negocio comercial y licenciamiento.

- [ ] **Licenciamiento Core**: Validación criptográfica local de llaves RSA/Ed25519.
- [ ] **Trial System**: 7-14 días de prueba automática vía Hardware ID.
- [ ] **Pasarela de Pagos**: Integración con Stripe / Lemon Squeezy para suscripciones.
- [ ] **Grace Period Offline**: Validación periódica permitiendo trabajo offline controlado.

**Target Version:** v5.1.0-pro

---

## 🔮 Futuro y Visión (Roadmap Extendido)

### 🔒 SecOps Guardián (Fase 9)
- Escaneo de secretos basado en entropía.
- Auditoría automática de vulnerabilidades en dependencias.
- Sanitización de DTOs y prevención de SQL Injection automática.

### 🔍 PR Mode (Fase 10)
- Integración nativa con GitHub/GitLab Pull Requests.
- Comentarios automáticos de revisión línea por línea.
- Bloqueo de merges si no se cumplen los estándares arquitectónicos.

### 🚀 Enterprise (Fase 11)
- Modo Daemon/Servicio con dashboard web centralizado.
- Soporte para equipos grandes con configuraciones compartidas en la nube.
- Integración con Jira / Linear para gestión de tickets automática.

---

## Cronograma de Releases

| Versión | Fecha | Enfoque | Estado |
|---------|-------|---------|--------|
| v4.5.0 | Feb 2025 | Multi-model AI & Framework Detection | ✅ Stable |
| v5.0.0-pro | Feb 2025 | Sentinel Pro Infrastructure & Local AI | ✅ Stable |
| v5.0.0-pro.alpha.4 | Feb 2025 | Workflow Engine & Multi-Agent System | ✅ Stable |
| **v5.0.0-pro.beta.1** | **Feb 2025** | **Quality Hardening & Multi-platform CI/CD** | **✅ Actual** |
| v5.1.0-pro | Q2 2025 | Monetization & Licensing System | 📋 Planned |
| v6.0.0-pro | Q3 2025 | SecOps & Automated Security Audits | 📋 Planned |

---

**Current Release:** v5.0.0-pro.beta.1  
**Last Update:** Febrero 2025
