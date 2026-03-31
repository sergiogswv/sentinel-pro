# 🛡️ Plan de Upgrade: SENTINEL — Agente de Monitoreo con ADK

> **Objetivo:** Transformar a Sentinel de un monitor de archivos pasivo (Rust) a un **Agente Proactivo** que analice cada cambio con memoria histórica de calidad y reglas de negocio, usando el **Google Agent Development Kit (ADK)**.

---

## 🏗️ Nueva Arquitectura (ADK Monitoring Agent)

Sentinel evolucionará para ser un Agente con **Pensamiento Crítico** sobre el código.

- **Framework:** `google-adk` (Python Sidecar)
- **Motor de Acción:** `sentinel-core` (Rust - existente para eficiencia en I/O)
- **Rol:** Agente de Calidad y Monitoreo en Tiempo Real.

### 1. Sistema de Memoria Persistente (Experience Storage)
Sentinel dejará de analizar cada archivo como si fuera la primera vez.
- **Cache de Archivos (Short-term):** Seguimiento de hashes de archivos modificados para evitar análisis redundantes.
- **Historial de Calidad (Long-term):** 
    - **Vector DB de Deuda Técnica:** Recordar qué archivos tienen mayor complejidad ciclomática histórica.
    - **Registro de Falsos Positivos:** Si el usuario marcó un aviso de IA como "ignorar", Sentinel no lo reportará de nuevo.

### 2. Conversión a Agente ADK
```python
from google.adk.agents import LlmAgent
from google.adk.tools import FunctionTool

# Sentinel se define como el Agente de Monitoreo
sentinel_agent = LlmAgent(
    name="Skrymir-Sentinel",
    model="gemini-2.0-flash",
    instruction="""
        Eres el Agente Sentinel de Skrymir Suite. 
        Tu misión es monitorear cambios en el filesystem y el repo Git.
        Cuando detectas un cambio, consulta tu memoria de calidad histórica.
        Si el cambio introduce deuda técnica o rompe patrones previos, genera un reporte detallado.
        Tienes acceso a herramientas para ejecutar escaneos profundos (Ollama/Rust server).
    """,
    tools=[...], # Wrappers de los servicios de Rust (monitor, scan, pro/audit)
    memory=QualityMemoryService() # Persistencia de calidad y deuda
)
```

---

## 🛠️ Pasos de Implementación

### Fase 1: Creación del Sidecar de IA (P0)
1. Iniciar `sentinel/ai_agent/` como proyecto Python.
2. Definir herramientas en `agent_tools.py` que invoquen los endpoints HTTP del servidor Rust (`localhost:4001`).

### Fase 2: Implementación de la Memoria de Calidad (P0)
- Usar **SQLite** para persistir el historial de cambios analizados por Sentinel y las sugerencias aceptadas/rechazadas por el usuario.
- Esto permitirá a Sentinel decir: "Este archivo `auth.ts` ha sido modificado 5 veces hoy, y el riesgo de regresión está aumentando".

### Fase 3: Integración Bidireccional con Cerebro (P1)
- Sentinel reportará no solo "un archivo cambió", sino "un cambio importante ocurrió en un módulo crítico según mis registros históricos de seguridad".

### Fase 4: Auditoría Proactiva (P1)
- Sentinel puede decidir motu proprio iniciar un `pro/audit` si detecta cambios acumulados que superen un umbral de riesgo guardado en memoria.

---

## ✅ Beneficios del Upgrade
- **Monitoreo Inteligente:** Filtra cambios irrelevantes y se enfoca en lo que realmente impacta la arquitectura.
- **Memoria de "Good Coding Practices":** Aprende del estilo de codificación del usuario y adapta sus sugerencias.
- **Reducción de Fatiga de Alertas:** Al recordar decisiones pasadas, evita reportar los mismos problemas una y otra vez.
