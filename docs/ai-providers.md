# AI Providers

Sentinel works with multiple AI providers. The quality of AI-powered features (`pro review`, `pro analyze`, `pro fix`) depends directly on the model you choose.

## Model Capability Requirements

| Feature | Local (≤7B) | Local (70B+) | Cloud API |
|---------|:-----------:|:------------:|:---------:|
| `sentinel monitor` (Layer 1 static) | ✅ | ✅ | ✅ |
| Commit messages | ✅ | ✅ | ✅ |
| `pro fix` (small files) | ⚠️ Generic | ✅ | ✅ |
| `pro analyze <file>` | ⚠️ Generic | ✅ | ✅ |
| `pro review` (architecture audit) | ❌ Hallucination | ⚠️ Limited | ✅ |
| `pro refactor` | ❌ | ⚠️ Limited | ✅ |

> **Recommendation:** Use cloud APIs (Claude or Gemini) for `pro` commands.
> Ollama is a great option for `sentinel monitor` + commits in air-gapped environments.

---

## Supported AI Providers

### Anthropic Claude (Recommended)

**Available models:**
- `claude-opus-4-5-20251101` - Most powerful, deep analysis
- `claude-sonnet-4-20250514` - Balanced, good quality/cost ratio
- `claude-haiku-3-5-20241022` - Fast and economical

**Configuration:**
- URL: `https://api.anthropic.com`
- Get your API Key at: https://console.anthropic.com

**Example configuration:**
```toml
[primary_model]
name = "claude-opus-4-5-20251101"
url = "https://api.anthropic.com"
api_key = "sk-ant-api03-..."
```

**Best for:**
- Deep code analysis
- Complex architectural reviews
- Detailed debugging assistance
- High-quality documentation generation

---

### Google Gemini

**Available models:**
- `gemini-2.0-flash` - Fast and efficient
- `gemini-1.5-pro` - Deep analysis
- `gemini-1.5-flash` - Economical

**Configuration:**
- URL: `https://generativelanguage.googleapis.com`
- Get your API Key at: https://makersuite.google.com/app/apikey

**Example configuration:**
```toml
[primary_model]
name = "gemini-2.0-flash"
url = "https://generativelanguage.googleapis.com"
api_key = "AIza..."
```

**Best for:**
- Fast responses
- Cost-effective analysis
- Quick code reviews
- Lightweight tasks

**Note:** Sentinel can automatically list available Gemini models during configuration.

---

## Fallback System

You can configure a backup model that will activate automatically if the primary model fails:

```
Primary Model: Claude Opus (deep analysis)
      ↓ (if fails)
Fallback Model: Gemini Flash (fast response)
```

This ensures high availability and reduces workflow interruptions.

### Configuring Fallback

**During initial setup:**
```
👉 ¿Configurar un modelo de respaldo por si falla el principal? (s/n): s
👉 API Key: [tu-api-key]
👉 URL del modelo: [url-del-proveedor]
👉 Nombre del modelo: [nombre-del-modelo]
```

**In .sentinelrc.toml:**
```toml
[primary_model]
name = "claude-opus-4-5-20251101"
url = "https://api.anthropic.com"
api_key = "sk-ant-api03-..."

[fallback_model]
name = "gemini-2.0-flash"
url = "https://generativelanguage.googleapis.com"
api_key = "AIza..."
```

### Fallback in Action

```
🔔 CAMBIO EN: auth.service.ts

   ⚠️  Modelo principal falló: Connection timeout. Intentando fallback con gemini-2.0-flash...

✨ CONSEJO DE CLAUDE:
SEGURO - La implementación de autenticación JWT es correcta.
[... Código guardado en .suggested ...]

   ✅ Arquitectura aprobada.
```

---

## Model Selection Guidelines

### For Production/Critical Projects
- **Primary**: Claude Opus (highest quality)
- **Fallback**: Claude Sonnet or Gemini Pro

### For Development/Testing
- **Primary**: Claude Sonnet (balanced)
- **Fallback**: Gemini Flash (fast and cheap)

### For Cost Optimization
- **Primary**: Gemini Flash (economical)
- **Fallback**: Gemini Flash (same model, different region)

---

## Automatic Model Listing

Sentinel can automatically list available models during configuration (currently supported for Gemini):

```
Fetching available models from Gemini...

Available models:
  1. gemini-2.0-flash
  2. gemini-1.5-pro
  3. gemini-1.5-flash

Select model number: 1
```

---

## API Key Management

### Getting API Keys

**Anthropic Claude:**
1. Visit https://console.anthropic.com
2. Sign up or log in
3. Navigate to API Keys section
4. Create a new API key
5. Copy the key (starts with `sk-ant-api03-`)

**Google Gemini:**
1. Visit https://makersuite.google.com/app/apikey
2. Sign in with Google account
3. Create API key
4. Copy the key (starts with `AIza`)

### Security Best Practices

See the [Security Guide](security.md) for detailed information on:
- API key protection
- .gitignore configuration
- Safe project sharing
- Cache security

---

## Local Models (Ollama / LM Studio)

Run Sentinel completely offline — no API key required.

> **⚠️ Important:** Models smaller than 70B parameters typically produce **generic analysis** for `pro review` and `pro analyze`. They work well for `sentinel monitor`, commit messages, and simple fixes. For deep architectural reviews, use a cloud API.

### Ollama

**Prerequisites:** [Install Ollama](https://ollama.com) and pull a model.

```bash
# Recommended models for code analysis:
ollama pull qwen2.5-coder:7b      # Fast, good for commits/simple fixes
ollama pull codellama:34b          # Better quality for analysis
ollama pull llama3.1:70b           # Best local option for pro review
```

**Configuration:**
```toml
[primary_model]
provider = "ollama"
name = "qwen2.5-coder:7b"
url = "http://localhost:11434"
api_key = ""
```

**What works well with small models (≤7B):**
- Real-time monitoring with static analysis (Layer 1 — no AI)
- Automatic commit messages (`g` / `gc` commands)
- Simple single-file fixes (`pro fix`)
- Test generation for simple functions

**What requires a larger model (70B+) or cloud API:**
- `sentinel pro review` — full architectural audit
- `sentinel pro analyze <file>` — deep semantic analysis
- `sentinel pro refactor` — complex refactoring

> **Note:** Sentinel will warn you when a local model is detected and you run a `pro` command that benefits from cloud models.

### LM Studio

```toml
[primary_model]
provider = "lm-studio"
name = "your-model-name"
url = "http://localhost:1234"
api_key = "lm-studio"
```

---

## Future Providers (Roadmap)

- OpenAI (GPT-4o, o1)
- Mistral AI
- Dynamic model selection based on task type (light tasks → local, deep tasks → cloud)

---

**Navigation:**
- [← Previous: Commands](commands.md)
- [Next: Security →](security.md)
