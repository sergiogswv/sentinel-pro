//! Cliente para comunicación con APIs de IA
//!
//! Soporta múltiples proveedores a través del trait AiProvider.
//! Para agregar un proveedor nuevo, ver `src/ai/providers/mod.rs`.
//!
//! Incluye sistema de fallback automático entre modelos.

use crate::ai::cache::{guardar_en_cache, intentar_leer_cache};
use crate::ai::providers::build_provider;
use crate::config::{ModelConfig, SentinelConfig};
use crate::stats::SentinelStats;
use colored::*;
use reqwest::blocking::Client;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy)]
pub enum TaskType {
    Light, // Commits, docs
    Deep,  // Arquitectura, debug tests
}

/// Punto de entrada inteligente con Fallback y Caché
pub fn consultar_ia_dinamico(
    prompt: String,
    task: TaskType,
    config: &SentinelConfig,
    stats: Arc<Mutex<SentinelStats>>,
    project_path: &Path,
) -> anyhow::Result<String> {
    // 1. Intentar Caché
    if config.use_cache {
        if let Some(res) = intentar_leer_cache(&prompt, project_path) {
            println!("{}", "   ♻️  Usando respuesta de caché...".dimmed());
            return Ok(res);
        }
    }

    // 2. Usar modelo primario
    let modelo_principal = &config.primary_model;

    // 3. Intentar ejecución con Fallback
    let resultado = ejecutar_con_fallback(
        prompt.clone(),
        modelo_principal,
        config.fallback_model.as_ref(),
        Arc::clone(&stats),
        task,
    );

    // 4. Guardar en Caché si tuvo éxito y parece una respuesta válida
    if let Ok(ref res) = resultado {
        if config.use_cache && res.trim().len() > 20 {
            let _ = guardar_en_cache(&prompt, res, project_path);
        }
    }

    resultado
}

fn ejecutar_con_fallback(
    prompt: String,
    principal: &ModelConfig,
    fallback: Option<&ModelConfig>,
    stats: Arc<Mutex<SentinelStats>>,
    task: TaskType,
) -> anyhow::Result<String> {
    match consultar_ia(prompt.clone(), principal, Arc::clone(&stats), task) {
        Ok(res) => Ok(res),
        Err(e) => {
            if let Some(fb) = fallback {
                println!(
                    "{}",
                    format!(
                        "   ⚠️  Modelo principal falló: {}. Intentando fallback con {}...",
                        e, fb.name
                    )
                    .yellow()
                );
                consultar_ia(prompt, fb, stats, task)
            } else {
                Err(e)
            }
        }
    }
}

pub fn consultar_ia(
    prompt: String,
    model: &ModelConfig,
    stats: Arc<Mutex<SentinelStats>>,
    task: TaskType,
) -> anyhow::Result<String> {
    let timeout = match task {
        TaskType::Light => std::time::Duration::from_secs(30),
        TaskType::Deep => std::time::Duration::from_secs(120),
    };

    let client = Client::builder()
        .timeout(timeout)
        .build()
        .unwrap_or_else(|_| Client::new());

    let prompt_len = prompt.len();
    let provider = build_provider(model);
    let resultado = provider.chat(&client, &prompt, &model.name);

    if let Ok(ref res) = resultado {
        let tokens = (res.len() as u64 / 4) + (prompt_len as u64 / 4);
        let mut s = stats.lock().unwrap();
        s.total_tokens_used += tokens;
        s.total_cost_usd += (tokens as f64 / 1000.0) * 0.01;
    }

    resultado
}

pub fn obtener_embeddings(
    textos: Vec<String>,
    model: &ModelConfig,
) -> anyhow::Result<Vec<Vec<f32>>> {
    if !model.url.contains("://") && !model.url.is_empty() && model.provider != "local" {
        return Err(anyhow::anyhow!(
            "URL del modelo inválida (falta esquema): {}",
            model.url
        ));
    }

    // El caso local no hace llamadas HTTP — se mantiene separado del trait
    if model.provider == "local" || model.provider == "anthropic" {
        let model_arc = crate::ml::embeddings::EmbeddingModel::get_or_init()?;
        return model_arc.embed(&textos);
    }

    let client = Client::new();
    let provider = build_provider(model);
    provider.embed(&client, textos, &model.name)
}
