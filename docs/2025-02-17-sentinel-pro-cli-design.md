# Sentinel Pro CLI - Diseño Técnico Completo

**Fecha:** 2025-02-17
**Versión:** 1.0
**Autor:** Sergio Guadarrama + Claude AI
**Status:** Aprobado

---

## 📋 Índice

1. [Resumen Ejecutivo](#resumen-ejecutivo)
2. [Arquitectura General](#1-arquitectura-general)
3. [Sistema Multi-Agent](#2-sistema-multi-agent)
4. [Comandos CLI Pro](#3-comandos-cli-pro)
5. [Machine Learning Local](#4-machine-learning-local)
6. [Framework Rules Engine](#5-framework-rules-engine)
7. [Knowledge Base y Vector Store](#6-knowledge-base-y-vector-store)
8. [Plan de Implementación](#7-plan-de-implementación)
9. [Requisitos y Recursos](#8-requisitos-y-recursos)
10. [Riesgos y Mitigaciones](#9-riesgos-y-mitigaciones)

---

## Resumen Ejecutivo

**Sentinel Pro** es una evolución de Sentinel CLI que transforma el actual file watcher en una **super herramienta de desarrollo** con capacidades de IA autónoma, machine learning local, y un sistema multi-agent.

### Visión

Una CLI local-first que:
- **Escribe código** automáticamente (no solo analiza)
- **Aprende** de tu proyecto con ML
- **Valida** contra reglas específicas del framework
- **Busca** código semánticamente
- **Refactoriza** con seguridad
- **Genera tests** automáticamente

### Enfoque

**Local-first:** Todo corre localmente, nada en la nube. Privacidad total, sin latencia, funciona offline.

### Arquitectura

- **Rust native** para performance y seguridad
- **Multi-agent system** (4 agentes especializados)
- **ML local** con ONNX y Candle
- **Vector database** (Qdrant) para búsqueda semántica
- **Framework engine** con reglas YAML extensibles

---

## 1. Arquitectura General

### Estructura de Directorios

```
sentinel-pro/
├── src/
│   ├── main.rs                    # Punto de entrada (CLI dispatcher)
│   │
│   ├── commands/                  # Módulo de comandos
│   │   ├── mod.rs
│   │   ├── monitor.rs            # Comando "sentinel" (actual)
│   │   ├── analyze.rs            # Comando "sentinel pro analyze"
│   │   ├── generate.rs           # Comando "sentinel pro generate"
│   │   ├── refactor.rs           # Comando "sentinel pro refactor"
│   │   ├── fix.rs                # Comando "sentinel pro fix"
│   │   ├── test_all.rs           # Comando "sentinel pro test-all"
│   │   ├── explain.rs            # Comando "sentinel pro explain"
│   │   ├── chat.rs               # Comando "sentinel pro chat"
│   │   ├── review.rs             # Comando "sentinel pro review"
│   │   ├── docs.rs               # Comando "sentinel pro docs"
│   │   ├── migrate.rs            # Comando "sentinel pro migrate"
│   │   └── optimize.rs           # Comando "sentinel pro optimize"
│   │
│   ├── agents/                    # Sistema Multi-Agent
│   │   ├── mod.rs
│   │   ├── base.rs               # Agent trait y base
│   │   ├── coder.rs              # Agente generador de código
│   │   ├── tester.rs             # Agente de testing
│   │   ├── refactor.rs           # Agente de refactorización
│   │   ├── reviewer.rs           # Agente de revisión
│   │   ├── orchestrator.rs       # Orquestador de agentes
│   │   └── workflow.rs           # Workflows multi-agent
│   │
│   ├── ai/                        # Módulo AI existente (expandido)
│   │   ├── mod.rs
│   │   ├── client.rs             # Cliente LLM (expandido)
│   │   ├── cache.rs
│   │   ├── analysis.rs
│   │   ├── framework.rs
│   │   ├── utils.rs
│   │   └── local_models.rs       # Modelos locales (Ollama, LM Studio)
│   │
│   ├── ml/                        # Machine Learning Local
│   │   ├── mod.rs
│   │   ├── embeddings.rs         # Embeddings locales
│   │   ├── similarity.rs         # Búsqueda semántica
│   │   ├── predictor.rs          # Predicción de bugs
│   │   ├── patterns.rs           # Detección de patrones
│   │   └── models.rs             # Modelos ONNX
│   │
│   ├── framework_engine/          # Framework Rules Engine
│   │   ├── mod.rs
│   │   ├── rules.rs              # Motor de reglas
│   │   ├── loader.rs             # Carga de YAML/JSON
│   │   ├── versions.rs           # Detección de versiones
│   │   └── registry.rs           # Registro de frameworks
│   │
│   ├── knowledge/                 # Knowledge Base
│   │   ├── mod.rs
│   │   ├── codebase.rs           # Indexación de código
│   │   ├── vector_store.rs       # Vector DB (Qdrant local)
│   │   ├── search.rs             # Búsqueda semántica
│   │   └── context.rs            # Contexto del proyecto
│   │
│   ├── config.rs                 # Configuración (expandida)
│   ├── files.rs
│   ├── git.rs
│   ├── stats.rs
│   ├── tests.rs
│   └── ui.rs
│
├── frameworks/                    # Reglas de frameworks
│   ├── nestjs/
│   │   ├── rules.yaml
│   │   ├── patterns.yaml
│   │   └── tests.yaml
│   ├── laravel/
│   │   ├── rules.yaml
│   │   ├── patterns.yaml
│   │   └── tests.yaml
│   ├── django/
│   │   └── ...
│   └── ...
│
├── agents/                        # Configuraciones de agentes
│   ├── coder.yaml
│   ├── tester.yaml
│   ├── refactor.yaml
│   └── reviewer.yaml
│
└── Cargo.toml
```

### Stack Tecnológico

**Core (Rust):**
- Runtime: Tokio (async)
- CLI: Clap 4.4
- Parsing: tree-sitter 0.20

**AI/ML:**
- Framework ML: Candle 0.3
- ONNX Runtime: ort 1.4
- Embeddings: candle-transformers
- Tokenizers: tokenizers 0.13

**Vector DB:**
- Qdrant client: qdrant-client 1.7
- Local Qdrant instance

**Utilidades:**
- Colors: colored 2.0
- Progress: indicatif 0.17
- File walking: walkdir 2.4
- Regex: regex 1.10

---

## 2. Sistema Multi-Agent

### Agentes Implementados

#### 1. **CoderAgent** - Generador de Código

**Propósito:** Generar código nuevo, completar funciones, crear archivos.

**Capabilities:**
- Generación de código desde cero
- Completado de funciones
- Creación de archivos boilerplate
- Generación de DTOs, entities, services
- Aplicación de estilo de código del proyecto

**Prompt Template:**
```
Generate {language} code for {task}.

Project Context:
- Framework: {framework}
- Style: {code_style_profile}
- Similar code: {related_functions}

Requirements:
{requirements}

Generate the code following:
1. Framework best practices
2. Project naming conventions
3. Project patterns
```

#### 2. **TesterAgent** - Agente de Testing

**Propósito:** Generar tests, validar cobertura, detectar edge cases.

**Capabilities:**
- Generación de tests unitarios
- Generación de tests de integración
- Detección de edge cases
- Análisis de cobertura
- Mocking automático

**Workflow:**
1. Analizar función a testear
2. Identificar casos normales
3. Identificar edge cases
4. Generar mocks si es necesario
5. Crear asserts apropiados
6. Validar cobertura target

#### 3. **RefactorAgent** - Refactorizador

**Propósito:** Refactorizar código manteniendo comportamiento.

**Capabilities:**
- Refactorización automática
- Eliminación de código muerto
- Renombrado inteligente
- Extracción de funciones
- Simplificación de lógica compleja

**Safety Checks:**
- Comparación AST antes/después
- Verificación de tipos
- Ejecución de tests post-refactor
- Validación de comportamiento preservado

#### 4. **ReviewerAgent** - Revisor

**Propósito:** Code review automático, detectar bugs, sugerencias de mejora.

**Capabilities:**
- Detección de vulnerabilidades de seguridad
- Verificación de mejores prácticas
- Análisis de performance
- Detección de bugs potenciales
- Sugerencias de optimización

**Checklist:**
- OWASP Top 10
- Framework-specific rules
- Performance anti-patterns
- Code smells
- DRY violations

### Workflows Predefinidos

#### Generate-and-Test
```
Coder (generate) → Tester (create tests) → Reviewer (validate)
```

#### Refactor-and-Validate
```
Refactor (improve) → Tester (verify) → Reviewer (check)
```

#### Fix-and-Verify
```
Coder (fix bug) → Tester (verify fix) → Reviewer (review)
```

#### Full-Review
```
Reviewer (analyze) → Tester (check coverage)
```

### Orquestador

```rust
pub struct AgentOrchestrator {
    agents: HashMap<String, Arc<dyn Agent>>,
    workflow_engine: Arc<WorkflowEngine>,
}

impl AgentOrchestrator {
    pub async fn execute_workflow(&self, workflow: Workflow, context: &Context) -> Result<WorkflowResult> {
        // Ejecuta steps secuenciales o en paralelo según workflow
    }
}
```

---

## 3. Comandos CLI Pro

### Interface General

```bash
# Modo clásico (sin cambios)
sentinel                           # File watcher mode
sentinel init                      # Inicializar proyecto

# Modo Pro (nuevos comandos)
sentinel pro <comando> [opciones] [args]
```

### Lista de Comandos

#### 1. `sentinel pro analyze <file>`

**Descripción:** Análisis profundo e interactivo de un archivo.

**Uso:**
```bash
sentinel pro analyze src/users/users.service.ts
sentinel pro analyze src/users/users.service.ts --deep
sentinel pro analyze src/users/users.service.ts --security
```

**Flags:**
- `--deep` - Análisis profundo con todos los agentes
- `--security` - Focus en seguridad
- `--performance` - Focus en performance
- `--json` - Output en JSON

**Output:**
```
🔍 Analyzing: src/users/users.service.ts
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 OVERVIEW
  • Lines: 245
  • Functions: 12
  • Complexity: Medium (8.2 avg)
  • Issues found: 3

⚠️  ISSUES DETECTED
  1. [Medium] Function createUser() is too long (45 lines)
  2. [Low] Duplicate code in updateUser() and createUser()
  3. [High] Missing error handling in deleteUser()
```

#### 2. `sentinel pro generate <file>`

**Descripción:** IA genera código nuevo.

**Uso:**
```bash
sentinel pro generate src/auth/auth.service.ts \
  --prompt "Create a JWT authentication service"

sentinel pro generate src/products/products.controller.ts \
  --spec products-spec.yaml

sentinel pro generate src/users/users.service.ts \
  --interactive
```

**Flags:**
- `--prompt <text>` - Descripción de lo que generar
- `--spec <file>` - Archivo de especificación YAML
- `--interactive` - Modo interactivo (chat-based)
- `--with-tests` - Auto-generar tests
- `--dry-run` - Mostrar sin aplicar cambios

#### 3. `sentinel pro refactor <file>`

**Descripción:** Refactoriza automáticamente.

**Uso:**
```bash
sentinel pro refactor src/users/users.service.ts
sentinel pro refactor src/orders/orders.controller.ts \
  --extract-functions --rename-variables
sentinel pro refactor src/payments/payments.service.ts \
  --safety-first --backup
```

**Flags:**
- `--extract-functions` - Extraer funciones largas
- `--rename-variables` - Renombrar variables semánticamente
- `--remove-dead` - Eliminar código muerto
- `--simplify` - Simplificar lógica compleja
- `--safety-first` - Máxima verificación de comportamiento
- `--backup` - Crear backup antes de refactorizar

#### 4. `sentinel pro fix <file>`

**Descripción:** IA fix bugs automáticamente.

**Uso:**
```bash
sentinel pro fix src/users/users.service.ts \
  --error "TypeError: Cannot read property 'id' of undefined"

sentinel pro fix src/users/users.service.ts \
  --failing-test test/users/users.spec.ts#testCreateUser
```

**Flags:**
- `--error <text>` - Mensaje de error
- `--failing-test <test>` - Test específico que falla
- `--interactive` - Modo interactivo
- `--verify` - Ejecutar tests después del fix

#### 5. `sentinel pro test-all`

**Descripción:** Ejecuta todos los tests con IA assistance.

**Uso:**
```bash
sentinel pro test-all
sentinel pro test-all --generate-missing
sentinel pro test-all --coverage --target 80
```

**Flags:**
- `--generate-missing` - Auto-generar tests faltantes
- `--fix-failing` - Auto-fix tests que fallan
- `--coverage` - Mostrar reporte de cobertura
- `--target <percent>` - Target de cobertura
- `--parallel` - Ejecutar tests en paralelo

#### 6. `sentinel pro explain <file>`

**Descripción:** Explica código línea por línea.

**Uso:**
```bash
sentinel pro explain src/auth/auth.service.ts
sentinel pro explain src/auth/auth.service.ts --function "login"
sentinel pro explain src/orders/orders.service.ts --detail high
```

**Flags:**
- `--function <name>` - Explicar función específica
- `--detail <level>` - Nivel: low, medium, high
- `--format <format>` - Formato: text, markdown, json
- `--include-security` - Incluir análisis de seguridad

#### 7. `sentinel pro chat`

**Descripción:** Chat interactivo con el código.

**Uso:**
```bash
sentinel pro chat
sentinel pro chat --context src/users
```

**Comandos de chat:**
```
/help           # Show commands
/analyze <file> # Analyze file
/generate       # Generate code
/refactor       # Refactor code
/fix            # Fix bug
/exit           # Exit chat
```

#### 8. `sentinel pro review`

**Descripción:** Review completo del proyecto.

**Uso:**
```bash
sentinel pro review
sentinel pro review src/users
sentinel pro review --security --performance
```

**Flags:**
- `--security` - Focus en seguridad
- `--performance` - Focus en performance
- `--only-critical` - Mostrar solo issues críticos
- `--output <file>` - Guardar reporte en archivo
- `--format <format>` - Formato: text, json, html, pdf

#### 9. `sentinel pro docs <dir>`

**Descripción:** Genera documentación completa.

**Uso:**
```bash
sentinel pro docs src/users
sentinel pro docs . --full
sentinel pro docs src --format markdown
```

#### 10. `sentinel pro migrate <src> <dst>`

**Descripción:** Migra código entre frameworks.

**Uso:**
```bash
sentinel pro migrate src/nest-users dst/laravel-users \
  --from nestjs --to laravel

sentinel pro migrate src/django-orders dst/nestjs-orders \
  --from django --to nestjs --preserve-tests
```

#### 11. `sentinel pro optimize`

**Descripción:** Optimiza performance del código.

**Uso:**
```bash
sentinel pro optimize src/orders/orders.service.ts
sentinel pro optimize src/products/products.service.ts --profile
```

---

## 4. Machine Learning Local

### Componentes ML

#### 1. Embeddings Generator

**Propósito:** Convertir código en vectores para búsqueda semántica.

**Modelo:** CodeBERT (250MB)

**Uso:**
```rust
let embedder = EmbeddingGenerator::new(model_path)?;
let embedding = embedder.embed_code("function login(user) { ... }")?;
```

**Output:** Vec<f32> de tamaño 768

#### 2. Semantic Search

**Propósito:** Encontrar código similar por significado.

**Tecnología:** Qdrant (vector database)

**Uso:**
```bash
sentinel pro find-similar "function to validate user email"
```

**Output:**
```
Similar code found:
  1. 92% similarity - src/auth/validation.ts
  2. 87% similarity - src/users/users.service.ts
  3. 81% similarity - src/shared/validators.ts
```

#### 3. Bug Predictor

**Propósito:** Predecir bugs potenciales basado en historial.

**Modelo:** bug-predictor-v1.onnx (15MB)

**Features extraídas:**
- Complejidad ciclomática
- Longitud de funciones
- Nivel de anidación
- Uso de tipos inseguros
- Manejo de errores

**Uso:**
```bash
sentinel pro predict-bugs src/orders/orders.service.ts
```

**Output:**
```
🔮 Predicting bugs in: src/orders/orders.service.ts

Function: processPayment()
Probability: 78% 🟠 High

Likely Issues:
  • Missing timeout for payment gateway (45%)
  • No retry logic on failure (33%)
```

#### 4. Pattern Detector

**Propósito:** Aprender patrones específicos del proyecto.

**Patrones detectables:**
- Patrones de error handling
- Convenciones de nombrado
- Estructura de módulos
- Patrones de inyección de dependencias

#### 5. Code Style Profile

**Propósito:** Aprender el estilo y preferencias del desarrollador.

**Atributos:**
- Naming conventions
- Indentation style
- Code organization
- Preferred patterns
- Anti-patterns

### Modelos ONNX

| Modelo | Tamaño | Uso | Precisión |
|--------|--------|-----|-----------|
| bug-predictor-v1.onnx | 15MB | Predicción de bugs | 82% |
| pattern-detector.onnx | 8MB | Detección de patrones | 89% |
| complexity-scoring.onnx | 5MB | Scoring de complejidad | 91% |

**Requisitos:**
- RAM: 500MB - 1GB
- CPU: AVX2 compatible
- Disco: 300MB para modelos
- GPU: Opcional

---

## 5. Framework Rules Engine

### Arquitectura

```
Loader (YAML/JSON) → Parser (Rules) → Validator (Code)
                                   ↓
                            Rule Registry
```

### Estructura de Reglas

**Ejemplo: `frameworks/nestjs/rules.yaml`**

```yaml
name: "NestJS"
version: "10.x"
language: "typescript"
extensions: [".ts", ".js"]

architecture_rules:
  - id: "nest-001"
    name: "Module Pattern"
    severity: "error"
    description: "Every feature should be organized in modules"
    check: "has_decorator('Module')"

  - id: "nest-002"
    name: "Dependency Injection"
    severity: "error"
    description: "Use constructor injection for dependencies"
    pattern: "constructor(private readonly service: Service)"
    anti_pattern: "new Service()"

security_rules:
  - id: "nest-sec-001"
    name: "ValidationPipe"
    check: "app.useGlobalPipes(new ValidationPipe())"
    severity: "error"
```

### Componentes

#### 1. Rule Loader

Carga reglas desde archivos YAML/JSON.

#### 2. Code Validator

Valida código contra reglas del framework.

**Uso:**
```bash
sentinel pro validate src/users --framework nestjs
```

**Output:**
```
✅ Validating against NestJS 10.x rules

📊 Validation Score: 82/100

⚠️  Issues Found: 5

  [nest-002] Dependency Injection
    File: users.service.ts:23
    Severity: error
    └─ Direct instantiation detected: 'new UserRepository()'
```

### Frameworks Soportados

| Framework | Versión | Reglas | Patrones | Testing |
|-----------|---------|--------|----------|---------|
| **NestJS** | 10.x, 9.x | ✅ | ✅ | ✅ |
| **Laravel** | 10.x, 11.x | ✅ | ✅ | ✅ |
| **Django** | 4.x, 5.x | ✅ | ✅ | ✅ |
| **FastAPI** | 0.100+ | ✅ | ✅ | ✅ |
| **Express** | 4.x | ✅ | ✅ | ✅ |
| **Next.js** | 14.x | ✅ | ✅ | ✅ |
| **React** | 18+ | ✅ | ✅ | ✅ |
| **Go** | 1.21+ | ✅ | ✅ | ✅ |
| **Rust** | 1.75+ | ✅ | ✅ | ✅ |

---

## 6. Knowledge Base y Vector Store

### Arquitectura

```
Code Indexer (AST Parse) → Vector Store (Qdrant) → Context Builder
```

### Componentes

#### 1. Codebase Indexer

Indexa código usando tree-sitter (AST parsing).

**Extrae:**
- Functions (nombre, signature, body, complexity)
- Classes (métodos, propiedades, herencia)
- Imports/exports
- Relations entre funciones

#### 2. Vector Store

Almacena embeddings en Qdrant local.

**Collections:**
- Functions
- Classes
- Patterns
- Documentation

#### 3. Context Builder

Construye contexto rico para operaciones.

**Tipos de contexto:**
- `FileContext` - Todo un archivo
- `FunctionContext` - Una función específica
- `ProjectContext` - Todo el proyecto

### Uso

```bash
# Buscar código relacionado
sentinel pro find-related "user authentication"

# Ver contexto de función
sentinel pro context src/auth/auth.service.ts authenticateUser
```

**Output:**
```
📋 Context for: authenticateUser()

🔹 FUNCTION SIGNATURE
  async authenticateUser(credentials: LoginDto): Promise<User>

🔹 CALLS (2 functions)
  • validateEmail() - src/auth/validation.ts:12
  • hashPassword() - src/auth/crypto.ts:45

🔹 CALLED BY (3 functions)
  • login() - src/auth/auth.controller.ts:23
  • refresh() - src/auth/auth.controller.ts:45
  • verifyToken() - src/middleware/auth.ts:67
```

---

## 7. Plan de Implementación

### Roadmap

```
Phase 1: Fundamentos Pro        (4-6 semanas)
Phase 2: Sistema Multi-Agent    (6-8 semanas)
Phase 3: ML Local               (4-6 semanas)
Phase 4: Framework Engine       (3-4 semanas)
Phase 5: Knowledge Base         (4-5 semanas)
Phase 6: Integración y Testing  (3-4 semanas)
Phase 7: Polishing y Docs       (2-3 semanas)

Total: 26-36 semanas (~6-9 meses)
```

### Milestones

1. **Milestone 1 (Semana 6):** Fundamentos Pro completados
2. **Milestone 2 (Semana 12):** Sistema Multi-Agent funcional
3. **Milestone 3 (Semana 18):** ML Local operativo
4. **Milestone 4 (Semana 24):** Framework Engine activo
5. **Milestone 5 (Semana 30):** Knowledge Base lista
6. **Milestone 6 (Semana 36):** Sentinel Pro v1.0 lanzado

### Cronograma Detallado

Ver cronograma completo en sección anterior del documento.

---

## 8. Requisitos y Recursos

### Requisitos de Desarrollo

**Rust:**
- Edition 2024
- Version 1.75+
- Toolchain stable

**Dependencias:**

```toml
[dependencies]
# CLI
clap = { version = "4.4", features = ["derive"] }

# Parsing
tree-sitter = "0.20"

# AI/ML
candle = "0.3"
candle-transformers = "0.3"
ort = "1.4"
tokenizers = "0.13"

# Vector DB
qdrant-client = "1.7"

# Utilidades
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
regex = "1.10"
walkdir = "2.4"
colored = "2.0"
indicatif = "0.17"
```

**Servicios:**
- Qdrant (Docker o binario)

### Recursos Humanos

**Equipo sugerido:**
- 1-2 Desarrolladores Rust senior
- 1 Desarrollador ML/Rust (part-time)
- 1 DevOps/Infra (part-time)

### Recursos de Infraestructura

**Desarrollo:**
- CPU: 4+ cores
- RAM: 16GB+
- Disco: 50GB+

**Producción (Qdrant):**
- CPU: 2 cores
- RAM: 4GB
- Disco: 100GB+

### Costos Estimados

**Desarrollo (6-9 meses):**
- Desarrollo: $80,000 - $120,000
- Infraestructura: $500 - $1,000
- Herramientas: $500
- **Total: ~$81,000 - $121,500**

---

## 9. Riesgos y Mitigaciones

### Riesgos Técnicos

| Riesgo | Impacto | Probabilidad | Mitigación |
|--------|---------|--------------|------------|
| Performance de ML local | Alto | Media | Usar modelos optimizados, caching |
| Complejidad de tree-sitter | Medio | Alta | Empezar con parsers simples |
| Qdrant reliability | Medio | Baja | Fallback a SQLite + índices |
| ONNX compatibility | Medio | Media | Testear en múltiples plataformas |

### Riesgos de Proyecto

| Riesgo | Impacto | Probabilidad | Mitigación |
|--------|---------|--------------|------------|
| Scope creep | Alto | Alta | Phases claras, MVP primero |
| Delay en ML | Medio | Media | Models pre-trained, ONNX |
| Adopción de usuarios | Alto | Media | Beta testing temprano |
| Competencia | Medio | Alta | Focus en local-first |

### Plan de Contingencia

**Si ML local es demasiado lento:**
- Opción A: Hybrid (local + cloud fallback)
- Opción B: Modelo más ligero
- Opción C: Remover ML, mantener búsqueda

**Si tree-sitter es problemático:**
- Opción A: Regex-based parsing simple
- Opción B: AST externo (librería por lenguaje)
- Opción C: Parsing básico sin AST

---

## 10. Próximos Pasos

### Inmediato

1. ✅ Diseño técnico completado
2. ⏳ Crear plan de implementación detallado (skill: writing-plans)
3. ⏳ Setup de infraestructura básica
4. ⏳ Phase 1: CLI Dispatcher

### Corto Plazo (1-3 meses)

1. Completar Phase 1 y 2
2. MVP con 3 comandos básicos
3. Alpha testing con usuarios selectos

### Mediano Plazo (3-6 meses)

1. Completar Phase 3, 4, 5
2. Beta público
3. Documentation completa

### Largo Plazo (6-9 meses)

1. Phase 6 y 7 completadas
2. Release v1.0
3. Comenzar trabajo en monetización

---

## Apéndices

### A. Comandos Rápidos

```bash
# Instalación
cargo build --release
cargo install --path .

# Uso básico
sentinel                           # File watcher
sentinel pro analyze <file>         # Análisis
sentinel pro generate <file>        # Generar código
sentinel pro refactor <file>        # Refactorizar

# Development
cargo test                          # Tests
cargo clippy                        # Linter
cargo fmt                           # Format
```

### B. Archivos de Configuración

**`.sentinelrc-pro.toml`:**

```toml
[general]
version = "1.0"
framework = "nestjs"

[features]
enable_ml = true
enable_agents = true
enable_knowledge_base = true

[local_llm]
provider = "ollama"
model_path = "~/.ollama/models"
api_port = 11434

[ml]
models_path = ".sentinel/models"
embeddings_model = "codebert"
bug_predictor_model = "bug-predictor-v1"

[knowledge_base]
vector_db_url = "http://localhost:6333"
index_on_start = true
```

### C. Referencias

- [Rust Book](https://doc.rust-lang.org/book/)
- [Candle ML](https://github.com/huggingface/candle)
- [Qdrant Docs](https://qdrant.tech/documentation/)
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [ONNX Runtime](https://onnxruntime.ai/docs/)

---

**Fin del Documento de Diseño**

**Status:** ✅ Aprobado para implementación
**Próximo paso:** Invocar skill `writing-plans` para crear plan de implementación detallado
