# Plan de Implementación: Sentinel Pro CLI

Este documento detalla la estrategia de desarrollo para transformar Sentinel CLI en la versión Pro, basada en el documento de diseño técnico.

---

## 📅 Resumen del Cronograma
- **Fase Inicial (MVP):** Mes 1-2
- **Fase Intermedia (Beta):** Mes 3-6
- **Lanzamiento (v1.0):** Mes 7-9

---

## 🏗️ Etapas de Implementación

### Etapa 1: Fundamentos Pro e Infraestructura Core (4-6 semanas)
*Establecer la base técnica y la estructura de comandos extendida.*

- **1.1 CLI Dispatcher y Comandos Pro**
  - Implementar la estructura `sentinel pro <comando>` usando Clap 4.4.
  - Crear los "stubs" para todos los nuevos comandos (`analyze`, `generate`, `refactor`, etc.).
  - Implementar el sistema de logs estructurados y UI básica con `indicatif` y `colored`.
- **1.2 Expansión de Configuración**
  - Soporte para `.sentinelrc-pro.toml` con perfiles de proyecto.
  - Configuración de proveedores LLM locales (Ollama/LM Studio).
  - Sistema de gestión de secretos y paths para modelos ONNX.
- **1.3 Framework Engine Base**
  - Definición del esquema YAML para reglas de frameworks.
  - Implementación del `Loader` de reglas y registro de frameworks inicial (NestJS, Rust).
  - Motor de detección automática de framework en el proyecto.

### Etapa 2: Knowledge Base y Vector Store (4-5 semanas)
*Creación del "cerebro" local que entiende el contexto del código.*

- **2.1 Indexación con Tree-sitter**
  - Integración de `tree-sitter` para múltiples lenguajes.
  - Extracción de metadata: funciones, clases, imports y relaciones.
  - Sistema de "watching" para actualización incremental del índice.
- **2.2 Almacenamiento Vectorial (Qdrant)**
  - Setup de instancia local de Qdrant (vía binario o Docker).
  - Implementación del cliente `qdrant-client` en Rust.
  - Definición de esquemas de colecciones para funciones, clases y documentación.
- **2.3 Context Builder**
  - Algoritmo de recuperación de contexto semántico.
  - Generación de prompts dinámicos inyectando contexto del codebase.

### Etapa 3: Sistema Multi-Agent (6-8 semanas)
*Implementación de la inteligencia autónoma especializada.*

- **3.1 Arquitectura de Agentes**
  - Implementación de `Agent Trait` y clases base en Rust.
  - Desarrollo del `AgentOrchestrator` para manejo de turnos y estados.
  - Implementación del `WorkflowEngine` para tareas secuenciales y paralelas.
- **3.2 Implementación de Agentes Core**
  - **CoderAgent:** Generación y edición de archivos.
  - **ReviewerAgent:** Análisis estático y detección de "code smells".
  - **TesterAgent:** Generación de tests y validación de cobertura.
  - **RefactorAgent:** Transformación de código segura con validación AST.

### Etapa 4: Machine Learning Local e Inteligencia On-Device (4-6 semanas)
*Optimización de privacidad y velocidad sin depender de la nube.*

- **4.1 Sistema de Embeddings**
  - Integración de `candle-transformers` para modelos locales (CodeBERT).
  - Pipeline de generación de embeddings en background.
- **4.2 Modelos ONNX (Inferencia Local)**
  - Integración de `ort` (ONNX Runtime).
  - Implementación de **Bug Predictor** y **Complexity Scoring**.
  - Optimización para CPU (AVX2) y detección opcional de GPU.
- **4.3 Detección de Patrones**
  - Analizador de estilos de código y convenciones del proyecto.
  - Generación del `Code Style Profile` automático.

### Etapa 5: Interfaz Pro y Comandos Interactivos (3-4 semanas)
*Refinamiento de la experiencia de usuario y utilidades avanzadas.*

- **5.1 Comandos de Análisis y Refactor**
  - Implementación final de `sentinel pro analyze` con reportes visuales.
  - Implementación de `sentinel pro refactor` con sistema de backups automáticos.
- **5.2 Chat Interactivo (`sentinel pro chat`)**
  - Terminal REPL para chatear directamente con el codebase.
  - Soporte para comandos rápidos dentro del chat (`/explain`, `/fix`).
- **5.3 Sistema de Documentación y Reportes**
  - Generación de MD/PDF para revisiones de seguridad y performance.
  - Comando `sentinel pro docs` para auto-documentación técnica.

### Etapa 6: Integración y Workflows Avanzados (3-4 semanas)
*Conectar todas las piezas para escenarios complejos.*

- **6.1 Workflows Multi-Step**
  - Pull Request Review automático (Reviewer + Tester).
  - Migración de frameworks (Migrate command) usando mapeo de patrones.
  - Flow de "Fix & Verify" (Coder soluciona, Tester valida).
- **6.2 Optimizador de Performance**
  - Análisis de hot-paths y sugerencias de optimización automática.

### Etapa 7: Calidad, Testing y Lanzamiento (2-3 semanas)
*Asegurar que la herramienta sea "Production Ready".*

- **7.1 Hardening y Seguridad**
  - Auditoría de manejo de archivos (evitar "path traversal" por IA).
  - Verificación de sandboxing para ejecución de tests.
- **7.2 Beta Testing y Documentación**
  - Manual de usuario Pro y guía de creación de reglas custom.
  - Programa de Early Access para feedback de performance.
- **7.3 Release v1.0**
  - Empaquetado de binarios para Windows/Linux/macOS.
  - Pipeline de CI/CD para distribución de modelos y reglas.

---

## 📈 Hitos de Control (Milestones)

1. **M1: Fundamentos (Semana 6):** CLI base y motor de reglas funcionando.
2. **M2: Cerebro Local (Semana 11):** Indexación vectorial y búsqueda semántica activa.
3. **M3: Agentes (Semana 19):** Capacidad autónoma de codificación y review.
4. **M4: ML Local (Semana 25):** Predicción de bugs y embeddings offline.
5. **M5: Beta Release (Semana 32):** Chat interactivo y workflows completos.
6. **M6: v1.0 (Semana 36):** Lanzamiento oficial con soporte multi-plataforma.
