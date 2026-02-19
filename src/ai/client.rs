//! Cliente para comunicación con APIs de IA
//!
//! Soporta múltiples proveedores:
//! - Anthropic (Claude)
//! - Google Gemini (Content API e Interactions API)
//!
//! Incluye sistema de fallback automático entre modelos.

use crate::ai::cache::{guardar_en_cache, intentar_leer_cache};
use crate::config::{ModelConfig, SentinelConfig};
use crate::stats::SentinelStats;
use colored::*;
use reqwest::blocking::Client;
use serde_json::json;
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

    // 4. Guardar en Caché si tuvo éxito
    if let Ok(ref res) = resultado {
        if config.use_cache {
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
    let resultado = if !model.provider.is_empty() {
        match model.provider.as_str() {
            "gemini" => {
                consultar_gemini_content(&client, prompt, &model.api_key, &model.url, &model.name)
            }
            "interactions" => consultar_gemini_interactions(
                &client,
                prompt,
                &model.api_key,
                &model.url,
                &model.name,
            ),
            "ollama" => consultar_ollama(&client, prompt, &model.url, &model.name),
            "lm-studio" | "openai" => {
                consultar_openai_compat(&client, prompt, &model.api_key, &model.url, &model.name)
            }
            _ => consultar_anthropic(&client, prompt, &model.api_key, &model.url, &model.name),
        }
    } else if model.url.contains("interactions") {
        consultar_gemini_interactions(&client, prompt, &model.api_key, &model.url, &model.name)
    } else if model.url.contains("googleapis.com") {
        consultar_gemini_content(&client, prompt, &model.api_key, &model.url, &model.name)
    } else {
        consultar_anthropic(&client, prompt, &model.api_key, &model.url, &model.name)
    };

    if let Ok(ref res) = resultado {
        // Track stats (Estimación simple: 1 token ≈ 4 caracteres)
        let tokens = (res.len() as u64 / 4) + (prompt_len as u64 / 4);
        let mut s = stats.lock().unwrap();
        s.total_tokens_used += tokens;

        // Estimación de costo: 0.01$ por cada 1K tokens (promedio)
        s.total_cost_usd += (tokens as f64 / 1000.0) * 0.01;
    }

    resultado
}

fn consultar_gemini_interactions(
    client: &Client,
    prompt: String,
    api_key: &str,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<String> {
    let response = client
        .post(base_url)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&json!({
            "model": model_name,
            "input": prompt
        }))
        .send()?;

    let status = response.status();
    let body_text = response.text()?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Error de API Gemini Interactions (Status {}): {}",
            status,
            body_text
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    body["output"]
        .as_str()
        .or_else(|| {
            body["outputs"].as_array().and_then(|outputs| {
                outputs
                    .iter()
                    .find(|o| o["type"] == "text")
                    .and_then(|o| o["text"].as_str())
            })
        })
        .or_else(|| body["candidates"][0]["content"]["parts"][0]["text"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No se pudo encontrar el texto en la respuesta de Gemini Interactions. Body: {}",
                body_text
            )
        })
}

fn consultar_gemini_content(
    client: &Client,
    prompt: String,
    api_key: &str,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<String> {
    let url = if base_url.contains("generateContent") {
        base_url.to_string()
    } else {
        format!(
            "{}/v1beta/models/{}:generateContent",
            base_url.trim_end_matches('/'),
            model_name
        )
    };

    let response = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }]
        }))
        .send()?;

    let status = response.status();
    let body_text = response.text()?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Error de API Gemini (Status {}): {}",
            status,
            body_text
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    body["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Estructura de Gemini inesperada. Body: {}", body_text))
}

fn consultar_anthropic(
    client: &Client,
    prompt: String,
    api_key: &str,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<String> {
    let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("content-type", "application/json")
        .json(&json!({
            "model": model_name,
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()?;

    let status = response.status();
    let body_text = response.text()?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Error de API Anthropic (Status {}): {}",
            status,
            body_text
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    body["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Estructura de Anthropic inesperada. Body: {}", body_text))
}

fn consultar_ollama(
    client: &Client,
    prompt: String,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<String> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .json(&json!({
            "model": model_name,
            "prompt": prompt,
            "stream": false
        }))
        .send()?;

    let status = response.status();
    let body_text = response.text()?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Error de API Ollama (Status {}): {}",
            status,
            body_text
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    body["response"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Estructura de Ollama inesperada. Body: {}", body_text))
}

fn consultar_openai_compat(
    client: &Client,
    prompt: String,
    api_key: &str,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": model_name,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()?;

    let status = response.status();
    let body_text = response.text()?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Error de API OpenAI-Compat/LM-Studio (Status {}): {}",
            status,
            body_text
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    body["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Estructura de OpenAI-Compat inesperada. Body: {}",
                body_text
            )
        })
}

pub fn obtener_embeddings(
    textos: Vec<String>,
    model: &ModelConfig,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let client = Client::new();

    match model.provider.as_str() {
        "gemini" => {
            obtener_embeddings_gemini(&client, textos, &model.api_key, &model.url, &model.name)
        }
        "ollama" => obtener_embeddings_ollama(&client, textos, &model.url, &model.name),
        "openai" | "lm-studio" => {
            obtener_embeddings_openai(&client, textos, &model.api_key, &model.url, &model.name)
        }
        "local" => {
            // Nota: Esto carga el modelo cada vez. Para indexación masiva deberíamos
            // instanciar EmbeddingModel una vez y reutilizarlo en capas superiores.
            // Por ahora, funciona para consultas esporádicas.
            let embedding_model = crate::ml::embeddings::EmbeddingModel::new()?;
            embedding_model.embed(&textos)
        }
        _ => Err(anyhow::anyhow!(
            "El proveedor {} no soporta embeddings actualmente",
            model.provider
        )),
    }
}

fn obtener_embeddings_gemini(
    client: &Client,
    textos: Vec<String>,
    api_key: &str,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let url = format!(
        "{}/v1beta/models/{}:batchEmbedContents",
        base_url.trim_end_matches('/'),
        model_name
    );

    let requests: Vec<serde_json::Value> = textos.into_iter().map(|t| {
        json!({ "model": format!("models/{}", model_name), "content": { "parts": [{ "text": t }] } })
    }).collect();

    let response = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .json(&json!({ "requests": requests }))
        .send()?;

    let body: serde_json::Value = response.json()?;
    let embeddings = body["embeddings"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Respuesta de Gemini Embeddings inesperada: {}", body))?
        .iter()
        .map(|e| {
            e["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap() as f32)
                .collect()
        })
        .collect();

    Ok(embeddings)
}

fn obtener_embeddings_ollama(
    client: &Client,
    textos: Vec<String>,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let url = format!("{}/api/embed", base_url.trim_end_matches('/'));
    let mut results = Vec::new();

    for texto in textos {
        let response = client
            .post(&url)
            .json(&json!({ "model": model_name, "input": texto }))
            .send()?;

        let body: serde_json::Value = response.json()?;
        let embedding = body["embeddings"][0]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Respuesta de Ollama Embeddings inesperada"))?
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        results.push(embedding);
    }

    Ok(results)
}

fn obtener_embeddings_openai(
    client: &Client,
    textos: Vec<String>,
    api_key: &str,
    base_url: &str,
    model_name: &str,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let url = format!("{}/v1/embeddings", base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({ "model": model_name, "input": textos }))
        .send()?;

    let body: serde_json::Value = response.json()?;
    let embeddings = body["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Respuesta de OpenAI Embeddings inesperada"))?
        .iter()
        .map(|d| {
            d["embedding"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap() as f32)
                .collect()
        })
        .collect();

    Ok(embeddings)
}
