use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::collections::BTreeMap;
use chrono::Local;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SentinelStats {
    pub bugs_criticos_evitados: u32,
    pub sugerencias_aplicadas: u32,
    pub tests_fallidos_corregidos: u32,
    pub total_analisis: u32,
    pub tiempo_estimado_ahorrado_mins: u32,
    pub total_cost_usd: f64,
    pub total_tokens_used: u64,
}

impl SentinelStats {
    pub fn cargar(path: &Path) -> Self {
        let stats_path = path.join(".sentinel_stats.json");
        if let Ok(content) = fs::read_to_string(stats_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn guardar(&self, path: &Path) {
        let stats_path = path.join(".sentinel_stats.json");
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(stats_path, content);
            // También guardar en el historial diario
            let _ = self.actualizar_historial_diario(path);
        }
    }

    /// Actualiza la entrada diaria en el historial
    fn actualizar_historial_diario(&self, path: &Path) -> anyhow::Result<()> {
        let history_path = path.join(".sentinel/stats_history.json");

        // Crear directorio si no existe
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Cargar historia existente o crear nueva
        let mut history: BTreeMap<String, HistoryEntry> = if let Ok(content) = fs::read_to_string(&history_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            BTreeMap::new()
        };

        // Obtener fecha de hoy
        let today = Local::now().format("%Y-%m-%d").to_string();

        // Actualizar entrada de hoy
        history.insert(today, HistoryEntry {
            timestamp: Local::now().to_rfc3339(),
            total_tokens: self.total_tokens_used,
            total_cost_usd: self.total_cost_usd,
            sugerencias_aplicadas: self.sugerencias_aplicadas,
            bugs_evitados: self.bugs_criticos_evitados,
        });

        // Guardar historial actualizado
        if let Ok(content) = serde_json::to_string_pretty(&history) {
            fs::write(history_path, content)?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub sugerencias_aplicadas: u32,
    pub bugs_evitados: u32,
}
