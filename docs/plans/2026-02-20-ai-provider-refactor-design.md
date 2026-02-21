# Design: AI Provider Refactor — Trait-based Architecture

**Date:** 2026-02-20
**Status:** Approved

## Problem

`client.rs` and `framework.rs` contain duplicated `match model.provider.as_str()` blocks for dispatching to different AI providers. Adding a new provider requires touching multiple files. The URL-based provider detection is a maintenance burden.

## Goal

A single dispatch point for all AI providers. Adding a new provider = one new file + one line in the factory.

## Approach: Trait `AiProvider` + Factory

### File Structure

```
src/ai/
  providers/
    mod.rs              ← trait AiProvider + fn build_provider (the only match)
    anthropic.rs
    gemini.rs           ← handles "gemini" and "interactions" (two structs, same file)
    openai_compat.rs    ← openai, groq, kimi, deepseek, lm-studio
    ollama.rs
  client.rs             ← thinned: cache, fallback, stats only; delegates to provider
  framework.rs          ← thinned: uses build_provider for list_models
```

### The Trait

```rust
// src/ai/providers/mod.rs

pub trait AiProvider: Send + Sync {
    fn chat(
        &self,
        client: &reqwest::blocking::Client,
        prompt: &str,
        model_name: &str,
    ) -> anyhow::Result<String>;

    fn embed(
        &self,
        client: &reqwest::blocking::Client,
        texts: Vec<String>,
        model_name: &str,
    ) -> anyhow::Result<Vec<Vec<f32>>>;

    fn list_models(&self) -> anyhow::Result<Vec<String>>;
}
```

### Factory

```rust
pub fn build_provider(config: &ModelConfig) -> Box<dyn AiProvider> {
    match config.provider.as_str() {
        "gemini"       => Box::new(GeminiProvider::new(&config.api_key, &config.url, false)),
        "interactions" => Box::new(GeminiProvider::new(&config.api_key, &config.url, true)),
        "ollama"       => Box::new(OllamaProvider::new(&config.url)),
        "openai" | "lm-studio" | "groq" | "kimi" | "deepseek"
                       => Box::new(OpenAiCompatProvider::new(&config.api_key, &config.url)),
        _              => Box::new(AnthropicProvider::new(&config.api_key, &config.url)),
    }
}
```

URL-based provider detection (fallback when `provider` field is empty) moves into the factory as a pre-step, keeping it in one place.

### `client.rs` after refactor (~100 lines)

- `consultar_ia`: builds HTTP client, calls `build_provider`, updates stats
- `obtener_embeddings`: local/anthropic case stays as a separate branch (no HTTP), rest delegates to provider

### `framework.rs` after refactor

- `obtener_modelos_disponibles`: builds a temporary `ModelConfig`, calls `build_provider(...).list_models()`
- No more `match` blocks

## What Does NOT Change

- Public API: `consultar_ia_dinamico`, `obtener_embeddings`, `obtener_modelos_disponibles` signatures unchanged
- All callers: `git.rs`, `agents/`, `docs.rs`, `tests.rs` — zero changes needed
- Local embedding case: stays in `client.rs`, not inside the trait

## Out of Scope

- Async migration
- New providers
- Stats/cache changes
