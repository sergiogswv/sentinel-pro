// src/ai/providers/gemini.rs
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::json;

/// Soporta dos APIs de Google:
/// - Content API (use_interactions = false): generateContent
/// - Interactions API (use_interactions = true): endpoint diferente
pub struct GeminiProvider {
    api_key: String,
    url: String,
    use_interactions: bool,
}

impl GeminiProvider {
    pub fn new(api_key: &str, url: &str, use_interactions: bool) -> Self {
        Self {
            api_key: api_key.to_string(),
            url: url.to_string(),
            use_interactions,
        }
    }
}

impl super::AiProvider for GeminiProvider {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> Result<String> {
        if self.use_interactions {
            let response = client
                .post(&self.url)
                .header("x-goog-api-key", &self.api_key)
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
                        "No se pudo encontrar texto en respuesta de Gemini Interactions. Body: {}",
                        body_text
                    )
                })
        } else {
            let url = if self.url.contains("generateContent") {
                self.url.clone()
            } else {
                format!(
                    "{}/v1beta/models/{}:generateContent",
                    self.url.trim_end_matches('/'),
                    model_name
                )
            };

            let response = client
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .header("content-type", "application/json")
                .json(&json!({
                    "contents": [{"parts": [{"text": prompt}]}]
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
                .ok_or_else(|| {
                    anyhow::anyhow!("Estructura de Gemini inesperada. Body: {}", body_text)
                })
        }
    }

    fn embed(&self, client: &Client, texts: Vec<String>, model_name: &str) -> Result<Vec<Vec<f32>>> {
        let url = format!(
            "{}/v1beta/models/{}:batchEmbedContents",
            self.url.trim_end_matches('/'),
            model_name
        );

        let requests: Vec<serde_json::Value> = texts
            .into_iter()
            .map(|t| {
                json!({
                    "model": format!("models/{}", model_name),
                    "content": { "parts": [{ "text": t }] }
                })
            })
            .collect();

        let response = client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&json!({ "requests": requests }))
            .send()?;

        let body: serde_json::Value = response.json()?;
        let embeddings = body["embeddings"]
            .as_array()
            .ok_or_else(|| {
                anyhow::anyhow!("Respuesta de Gemini Embeddings inesperada: {}", body)
            })?
            .iter()
            .map(|e| -> anyhow::Result<Vec<f32>> {
                let values = e["values"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Gemini embedding: 'values' faltante o no es array"))?;
                values
                    .iter()
                    .map(|v| {
                        v.as_f64()
                            .ok_or_else(|| anyhow::anyhow!("Gemini embedding: valor no numérico"))
                            .map(|f| f as f32)
                    })
                    .collect()
            })
            .collect::<anyhow::Result<Vec<Vec<f32>>>>()?;

        Ok(embeddings)
    }

    fn list_models(&self) -> Result<Vec<String>> {
        let client = Client::new();
        let response = client
            .get(format!(
                "{}/v1beta/models?key={}",
                self.url.trim_end_matches('/'),
                self.api_key
            ))
            .send()?;

        let json: serde_json::Value = response.json()?;
        let models = json["models"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Respuesta de Gemini inválida"))?
            .iter()
            .filter_map(|m| {
                m["name"]
                    .as_str()
                    .map(|s| s.trim_start_matches("models/").to_string())
            })
            .collect();
        Ok(models)
    }
}
