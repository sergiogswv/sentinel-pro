# 🔄 Guía de Creación de Workflows (Sentinel Pro)

Los **Workflows** en Sentinel Pro son secuencias automatizadas de tareas en las que colaboran múltiples Agentes Especializados (Coder, Reviewer, Tester, Refactor) para completar procesos de ingeniería de software complejos desde una sola instrucción.

---

## 🏗️ Anatomía de un Workflow

En esta versión Beta, los workflows están compuestos por **Pasos** (`WorkflowStep`). Cada paso define:
1. **Un Nombre**: Descripción corta de lo que hará el paso.
2. **Un Agente**: El especialista IA que ejecutará el trabajo (ej. `CoderAgent`, `TesterAgent`).
3. **Una Tarea (`TaskTemplate`)**: Las instrucciones exactas y el tipo de acción a realizar (`Fix`, `Refactor`, `Generate`, `Test`, `Analyze`).

```rust
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}
```

---

## 🛠️ Workflows Predefinidos

Actualmente Sentinel Pro incluye pre-cargados ciertos workflows de alto valor:

- **`fix-and-verify`**: Analiza un archivo buscando bugs -> Sugiere y aplica la corrección limpia -> Genera tests unitarios para verificar el caso borde del bug y la regresión.
- **`review-security`**: Realiza una auditoría estática OWASP Top 10 -> Seguido por el CoderAgent sugiriendo e implementando código mitigador inmediato.

### ¿Cómo ejecutarlos?

Puedes lanzarlos directamente por consola usando:
```bash
sentinel pro workflow fix-and-verify --file src/auth/login.ts
```
O usando el comando interactivo guiado:
```bash
sentinel pro workflow
```

---

## 🎯 Variables de Contexto Mágico

Dentro de la especificación de una Tarea en el workflow, puedes usar las siguientes variables virtuales que el **Agent Orquestador** inyectará dinámicamente en tiempo de ejecución:

- `{file}`: Representa el nombre o la ruta del archivo que el usuario especificó en el comando por CLI.
- **Contexto de Pasos Previos (Automático)**: Si el Agente 1 (Coder) genera un refactor de un archivo en el Paso 1, el Agente 2 (Tester) en el Paso 2 obtiene en su propio contexto el *código modificado resultante* en memoria, no tu archivo viejo. ¡Todo ocurre en un Pipeline perfecto y luego se guarda a disco!

---

## ⚙️ Creación de Workflows Personalizados (Próximamente)

En futuras versiones de la serie v5.0.0, abriremos la compatibilidad de *File-based Workflows*, donde podrás definir en tu directorio de proyecto un archivo `.sentinel/workflows/ci_pipeline.yml`. 

*Sintaxis Esperada:*
```yaml
name: "Clean & Test"
description: "Aplica clean code y crea una batería de smoke tests"
steps:
  - name: "Aplicar Clean Code"
    agent: "RefactorAgent"
    taskType: "Refactor"
    description: "Toma el archivo {file} y aplica los principios SOLID, removiendo dead code."
  - name: "Pruebas"
    agent: "TesterAgent"
    taskType: "Test"
    description: "Genera tests automatizados sólo para los exports principales de {file}."
```
