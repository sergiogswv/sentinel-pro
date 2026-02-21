# AI Provider Trait Refactor — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace duplicated `match model.provider.as_str()` blocks across `client.rs` and `framework.rs` with a single `AiProvider` trait + factory, so adding a new provider = one new file + one line.

**Architecture:** A new `src/ai/providers/` module defines the `AiProvider` trait and a `build_provider(config)` factory. Each provider is a struct in its own file implementing the trait. `client.rs` and `framework.rs` become thin orchestrators that delegate to the factory.

**Tech Stack:** Rust, reqwest::blocking::Client, serde_json, anyhow

---

### Task 1: Create `src/ai/providers/mod.rs` — trait + factory

**Files:**
- Create: `src/ai/providers/mod.rs`

**Step 1: Verify current state compiles**

```bash
cargo check 2>&1 | tail -5
```
Expected: no errors (baseline).

**Step 2: Create the file**

```rust
// src/ai/providers/mod.rs
//! Trait AiProvider y factory build_provider
//!
//! Para agregar un nuevo proveedor:
//! 1. Crear `src/ai/providers/mi_proveedor.rs` implementando `AiProvider`
//! 2. Agregar `pub mod mi_proveedor;` y re-export aquí
//! 3. Agregar un arm al match en `build_provider`

pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai_compat::OpenAiCompatProvider;

use crate::config::ModelConfig;
use reqwest::blocking::Client;

pub trait AiProvider: Send + Sync {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> anyhow::Result<String>;

    fn embed(
        &self,
        client: &Client,
        texts: Vec<String>,
        model_name: &str,
    ) -> anyhow::Result<Vec<Vec<f32>>>;

    fn list_models(&self) -> anyhow::Result<Vec<String>>;
}

/// Único punto de despacho de providers.
/// El campo `provider` en ModelConfig determina cuál se usa.
/// Si está vacío, se intenta detectar por URL.
pub fn build_provider(config: &ModelConfig) -> Box<dyn AiProvider> {
    let provider = if config.provider.is_empty() {
        let url = config.url.to_lowercase();
        if url.contains("interactions") {
            "interactions"
        } else if url.contains("googleapis") {
            "gemini"
        } else if url.contains("deepseek")
            || url.contains("groq")
            || url.contains("kimi")
            || url.contains("moonshot")
        {
            "openai"
        } else {
            "anthropic"
        }
    } else {
        config.provider.as_str()
    };

    match provider {
        "gemini" => Box::new(GeminiProvider::new(&config.api_key, &config.url, false)),
        "interactions" => Box::new(GeminiProvider::new(&config.api_key, &config.url, true)),
        "ollama" => Box::new(OllamaProvider::new(&config.url)),
        "openai" | "lm-studio" | "groq" | "kimi" | "deepseek" => {
            Box::new(OpenAiCompatProvider::new(&config.api_key, &config.url))
        }
        _ => Box::new(AnthropicProvider::new(&config.api_key, &config.url)),
    }
}
```

**Step 3: Register the module in `src/ai/mod.rs`**

Add after the existing `pub mod` lines:
```rust
pub mod providers;
```

**Step 4: Check (will fail until provider files exist)**

```bash
cargo check 2>&1 | grep "^error" | head -10
```
Expected: errors about missing modules `anthropic`, `gemini`, etc. — that's fine, we'll add them next.

**Step 5: Commit (even with compile errors — we commit the trait contract)**

```bash
git add src/ai/providers/mod.rs src/ai/mod.rs
git commit -m "feat(ai): add AiProvider trait and build_provider factory"
```

---

### Task 2: Implement `AnthropicProvider`

**Files:**
- Create: `src/ai/providers/anthropic.rs`

**Step 1: Create the file** — extracted verbatim from `client.rs`'s `consultar_anthropic` and the anthropic arm of `obtener_modelos_disponibles`:

```rust
// src/ai/providers/anthropic.rs
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::json;

pub struct AnthropicProvider {
    api_key: String,
    url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, url: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            url: url.to_string(),
        }
    }
}

impl super::AiProvider for AnthropicProvider {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> Result<String> {
        let base = self.url.trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{}/messages", base)
        } else {
            format!("{}/v1/messages", base)
        };

        let response = client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
            .ok_or_else(|| {
                anyhow::anyhow!("Estructura de Anthropic inesperada. Body: {}", body_text)
            })
    }

    fn embed(&self, _client: &Client, _texts: Vec<String>, _model_name: &str) -> Result<Vec<Vec<f32>>> {
        Err(anyhow::anyhow!(
            "Anthropic no soporta embeddings vía API HTTP. Usa provider 'local'."
        ))
    }

    fn list_models(&self) -> Result<Vec<String>> {
        let client = Client::new();
        let url = format!("{}/v1/models", self.url.trim_end_matches('/'));
        let response = client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()?;

        let json: serde_json::Value = response.json()?;
        let models = json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Respuesta de Claude inválida"))?
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        Ok(models)
    }
}
```

**Step 2: cargo check**

```bash
cargo check 2>&1 | grep "^error" | head -10
```
Expected: still errors for gemini, ollama, openai_compat — anthropic should be clean.

**Step 3: Commit**

```bash
git add src/ai/providers/anthropic.rs
git commit -m "feat(ai): add AnthropicProvider"
```

---

### Task 3: Implement `GeminiProvider`

**Files:**
- Create: `src/ai/providers/gemini.rs`

**Step 1: Create the file** — merges `consultar_gemini_content`, `consultar_gemini_interactions`, `obtener_embeddings_gemini`, and the gemini arm of `obtener_modelos_disponibles`:

```rust
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
```

**Step 2: cargo check**

```bash
cargo check 2>&1 | grep "^error" | head -10
```

**Step 3: Commit**

```bash
git add src/ai/providers/gemini.rs
git commit -m "feat(ai): add GeminiProvider (content + interactions)"
```

---

### Task 4: Implement `OllamaProvider`

**Files:**
- Create: `src/ai/providers/ollama.rs`

**Step 1: Create the file**

```rust
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
```

**Step 2: cargo check**

```bash
cargo check 2>&1 | grep "^error" | head -10
```

**Step 3: Commit**

```bash
git add src/ai/providers/ollama.rs
git commit -m "feat(ai): add OllamaProvider"
```

---

### Task 5: Implement `OpenAiCompatProvider`

**Files:**
- Create: `src/ai/providers/openai_compat.rs`

**Step 1: Create the file** — covers openai, groq, kimi, deepseek, lm-studio:

```rust
// src/ai/providers/openai_compat.rs
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::json;

pub struct OpenAiCompatProvider {
    api_key: String,
    url: String,
}

impl OpenAiCompatProvider {
    pub fn new(api_key: &str, url: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            url: url.to_string(),
        }
    }
}

impl super::AiProvider for OpenAiCompatProvider {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> Result<String> {
        let base = self.url.trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        };

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": model_name,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()?;

        let status = response.status();
        let body_text = response.text()?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Error de API OpenAI-Compat (Status {}): {}",
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

    fn embed(&self, client: &Client, texts: Vec<String>, model_name: &str) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/v1/embeddings", self.url.trim_end_matches('/'));

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({ "model": model_name, "input": texts }))
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

    fn list_models(&self) -> Result<Vec<String>> {
        let client = Client::new();
        let url_str = self.url.trim_end_matches('/');
        let target_url = if url_str.ends_with("/v1") {
            format!("{}/models", url_str)
        } else {
            format!("{}/v1/models", url_str)
        };

        let mut request = client.get(&target_url);
        if !self.api_key.is_empty() {
            request = request.header("authorization", format!("Bearer {}", self.api_key));
        }

        let response = request.send()?;
        let json: serde_json::Value = response.json()?;
        let models = json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Respuesta API compatible inválida"))?
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        Ok(models)
    }
}
```

**Step 2: cargo check** — should be clean now that all 4 providers exist

```bash
cargo check 2>&1 | grep "^error" | head -20
```
Expected: 0 errors (or only errors from client.rs / framework.rs, which we haven't touched yet).

**Step 3: Commit**

```bash
git add src/ai/providers/openai_compat.rs
git commit -m "feat(ai): add OpenAiCompatProvider (openai/groq/kimi/deepseek/lm-studio)"
```

---

### Task 6: Refactor `src/ai/client.rs`

**Files:**
- Modify: `src/ai/client.rs`

**Step 1: Replace the entire file** with the thinned version. The provider functions (`consultar_anthropic`, `consultar_gemini_content`, `consultar_gemini_interactions`, `consultar_ollama`, `consultar_openai_compat`, `obtener_embeddings_gemini`, `obtener_embeddings_ollama`, `obtener_embeddings_openai`) are all deleted. The `match` blocks are deleted.

New content of `src/ai/client.rs`:

```rust
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
```

**Step 2: cargo check**

```bash
cargo check 2>&1 | grep "^error" | head -20
```
Expected: 0 errors from client.rs. Fix any that appear before continuing.

**Step 3: Commit**

```bash
git add src/ai/client.rs
git commit -m "refactor(ai): thin client.rs — delegate to AiProvider trait"
```

---

### Task 7: Refactor `src/ai/framework.rs`

**Files:**
- Modify: `src/ai/framework.rs`

**Step 1: Replace only `obtener_modelos_disponibles`**

Find the function starting at around line 195. Replace the entire function body with:

```rust
/// Obtiene el listado de modelos disponibles para cualquier proveedor
pub fn obtener_modelos_disponibles(
    provider: &str,
    api_url: &str,
    api_key: &str,
) -> anyhow::Result<Vec<String>> {
    let config = crate::config::ModelConfig {
        provider: provider.to_string(),
        url: api_url.to_string(),
        api_key: api_key.to_string(),
        name: String::new(),
    };
    crate::ai::providers::build_provider(&config).list_models()
}
```

Also add the import at the top of the file if not already present (it should be fine since `crate::ai::providers` is in scope via `crate`).

**Step 2: cargo check**

```bash
cargo check 2>&1 | grep "^error" | head -20
```
Expected: 0 errors.

**Step 3: Full build**

```bash
cargo build 2>&1 | tail -10
```
Expected: `Finished` with no errors.

**Step 4: Commit**

```bash
git add src/ai/framework.rs
git commit -m "refactor(ai): thin framework.rs — use build_provider for list_models"
```

---

### Task 8: Final verification

**Step 1: Full build clean**

```bash
cargo build 2>&1 | tail -5
```
Expected: `Finished dev [unoptimized + debuginfo] target(s)`

**Step 2: Check for any remaining old match blocks in ai/**

```bash
grep -n "match model.provider\|match provider" src/ai/client.rs src/ai/framework.rs
```
Expected: 0 results (all match blocks now live only in `src/ai/providers/mod.rs`).

**Step 3: Verify providers/mod.rs is the single dispatch point**

```bash
grep -rn "match.*provider" src/ai/
```
Expected: only results in `src/ai/providers/mod.rs`.

**Step 4: Final commit**

```bash
git add -p  # review any remaining unstaged changes
git commit -m "refactor(ai): complete AiProvider trait migration — single dispatch point"
```
