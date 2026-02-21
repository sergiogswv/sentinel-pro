// src/ai/providers/ollama.rs
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::json;

pub struct OllamaProvider {
    url: String,
}

impl OllamaProvider {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }
}

impl super::AiProvider for OllamaProvider {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.url.trim_end_matches('/'));

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
            .ok_or_else(|| {
                anyhow::anyhow!("Estructura de Ollama inesperada. Body: {}", body_text)
            })
    }

    fn embed(&self, client: &Client, texts: Vec<String>, model_name: &str) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/api/embed", self.url.trim_end_matches('/'));
        let mut results = Vec::new();

        for texto in texts {
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

    fn list_models(&self) -> Result<Vec<String>> {
        let client = Client::new();
        let url_str = self.url.trim_end_matches('/');
        let is_native = !url_str.ends_with("/v1");

        let target_url = if is_native {
            format!("{}/api/tags", url_str)
        } else {
            format!("{}/models", url_str)
        };

        let response = client.get(&target_url).send()?;
        let json: serde_json::Value = response.json()?;

        if is_native {
            let models = json["models"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Respuesta de Ollama inválida"))?
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect();
            Ok(models)
        } else {
            let models = json["data"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Respuesta API compatible inválida"))?
                .iter()
                .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                .collect();
            Ok(models)
        }
    }
}
