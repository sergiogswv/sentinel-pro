# 📊 Sistema de Métricas y ROI de Sentinel Pro

Sentinel Pro cuantifica el valor aportado a tu proyecto a través de un sistema de seguimiento en tiempo real de tokens, costos y tiempo de desarrollo ahorrado.

## 🕒 Cálculo de Tiempo Ahorrado (ROI)

Sentinel estima el tiempo que te hubiera tomado realizar la misma tarea manualmente sin ayuda de IA. Los valores están basados en promedios de la industria para tareas de mantenimiento y auditoría:

| Acción | Tiempo Ahorrado (Mins) | Descripción |
| :--- | :---: | :--- |
| **Fix Automático (Audit/Fix)** | 20 min | Identificación del bug + Corrección + Revisión de sintaxis. |
| **Análisis Monitor (Fondo)** | 20 min | Detección proactiva de un error mientras escribes código. |
| **Refactorización** | 15 min | Mejora de legibilidad, aplicación de Clean Code y SOLID. |
| **Generación de Código** | 10 min | Creación de boilerplate, lógica base o componentes. |
| **Generación de Tests** | 15 min | Creación de mocks y casos de prueba unitarios. |
| **Migración de Framework** | 60 min | Adaptación de lógica entre stacks (ej: Express -> NestJS). |

## 💰 Tokens y Costos (USD)

El seguimiento de costos es dinámico y se actualiza con cada llamada a la API:

1.  **Conteo de Tokens:**
    *   Estimación: `(Caracteres del Prompt + Caracteres de Respuesta) / 4`.
    *   Este método proporciona una precisión del ~95% comparado con tokenizadores reales sin añadir latencia de procesamiento.
2.  **Cálculo de Costo:**
    *   Sentinel aplica una tarifa promedio de **$0.01 USD por cada 1,000 tokens**.
    *   Nota: Dependiendo del modelo (Claude 3.5 vs GPT-4o-mini), el costo real puede variar, pero el sistema mantiene este promedio para facilitar el seguimiento presupuestario.

## 🔍 Registro de Métricas

Todas las métricas se guardan localmente en tu proyecto:
*   **Archivo:** `.sentinel_stats.json`
*   **Contenido:**
    *   `bugs_criticos_evitados`: Errores graves detenidos en el monitor o corregidos con fix.
    *   `sugerencias_aplicadas`: Total de veces que Sentinel modificó el código con éxito.
    *   `total_analisis`: Contador global de auditorías realizadas.
    *   `total_tokens_used`: Acumulado de tokens (entrada + salida).
    *   `total_cost_usd`: Gasto estimado acumulado.

## 📈 Visualización

Puedes consultar estas métricas en cualquier momento ejecutando el comando:
```bash
sentinel m
```
O viendo el reporte de productividad diario con:
```bash
sentinel r
```
