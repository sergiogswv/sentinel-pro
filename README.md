# Sentinel Pro 🛡️✨

<p align="center">
  <img src="./public/sentinel.jpg" alt="Sentinel Logo" width="100%"/>
</p>

<p align="center">
  <strong>The Ultimate AI-Powered Code Monitor & Development Suite.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-5.0.0--pro.beta.3-purple.svg" alt="Version">
  <img src="https://img.shields.io/badge/rust-2024-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-blue.svg" alt="License">
</p>

---

## 🚀 What is Sentinel Pro?

Sentinel Pro is the **AI-Powered Quality Guardian** for modern software development. Unlike traditional generators, Sentinel focuses on **ensuring the integrity, security, and architectural consistency** of your codebase in real-time.

It acts as a silent partner that monitors every change, providing a two-layered defense:
1. **Layer 1: High-Speed Static Analysis** (Powered by Tree-sitter) - Instant detection of dead code, complexity, and pattern violations (<100ms).
2. **Layer 2: AI Semantic Review** (Powered by LLMs) - Deep understanding of business logic, security vulnerabilities, and architectural design.

### ✨ Key Features (Quality Guardian Edition)

- 🛡️ **Two-Layer Analysis** - Hybrid approach combining ultra-fast static rules with deep AI reasoning.
- 🤖 **AI Code Quality Guardian** - Specialized agents like **FixSuggesterAgent** and **ReviewerAgent**.
- 🏗️ **Framework Rule Engine** - Validates architectural patterns (NestJS, React, etc.) before they hit production.
- 🧠 **Standalone Knowledge Base** - Deep project context via structural indexing (SQLite) and local semantic search.
- 💾 **Smart Caching** - Drastically reduces API costs by remembering previous reviews.
- 📊 **Real-time ROI Metrics** - Visibility into bugs prevented and engineering time saved.
- 🧪 **Autonomous Testing** - AI-assisted validation of every code modification.
- 🔒 **Privacy First** - Support for local models (Ollama/LM Studio) for 100% offline analysis.

---

## 📦 Quick Start

### Requirements

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- API Key from [Claude](https://console.anthropic.com), [Gemini](https://makersuite.google.com/app/apikey), or a local LLM.
- Project with TypeScript or JavaScript for full static analysis support

### Supported Languages

| Feature | TypeScript / JavaScript | Go, Python, Java, Rust, others |
|---------|------------------------|-------------------------------|
| Static analysis (`check`) | ✅ Complete | 🔜 Next major version |
| File monitor (`monitor`) | ✅ | ✅ |
| Audit & Review (LLM) | ✅ | ✅ |

> **Note:** `audit` and `review` use LLM directly and work with any language. Only static analysis rules (dead code, unused imports, complexity) are TypeScript/JavaScript-only.

### Installation

#### Linux / macOS / Windows (WSL)
```bash
git clone https://github.com/your-username/sentinel-rust.git
cd sentinel-rust
cargo build --release
```

---

## 🎮 Available Commands

### Pro CLI Commands

Access the Quality Guardian suite using the `pro` sub-command:

```bash
# Core Quality Commands
sentinel pro analyze <file>   # Hybrid Analysis (Static L1 + AI L2 hallazgos)
sentinel pro fix <file>       # Propose precise fixes for detected issues (FixSuggester)
sentinel pro refactor <file>  # Suggested improvements for maintainability
sentinel pro test-all         # Generate and verify missing tests (Tester)
sentinel pro audit <path>     # Recursive project-wide quality & security audit
sentinel pro review           # Full architectural consistency check
sentinel pro explain <file>   # Didactic breakdown of complex logic
sentinel pro optimize <file>  # Performance and resource usage suggestions
sentinel pro workflow <name>  # Multi-step automation (e.g., fix-and-verify)
```

💡 **Tip:** On startup, Sentinel automatically displays the command list.

---

## 📖 Complete Documentation

### 📚 User Guides

- **[Installation and Setup](docs/installation.md)** - Complete installation guide
- **[Advanced Configuration](docs/configuration.md)** - `.sentinelrc.toml` in detail
- **[Commands and Usage](docs/commands.md)** - Complete guide to all commands
- **[AI Providers](docs/ai-providers.md)** - Claude, Gemini, and more
- **[Advanced Workflows](docs/workflows.md)** - Guide for executing and creating workflows
- **[Usage Examples](docs/examples.md)** - Real-world use cases

### 🔧 Technical References

- **[Architecture](docs/architecture.md)** - System components and flow
- **[Security](docs/security.md)** - API key protection and best practices
- **[Troubleshooting](docs/troubleshooting.md)** - Common problems and solutions

### 📋 Project

- **[Roadmap](docs/roadmap.md)** - Planned features
- **[Changelog](CHANGELOG.md)** - Change history ([English version](CHANGELOG_EN.md))
- **[Structure](ESTRUCTURA.md)** - Project structure ([English version](STRUCTURE.md))

---

## 🎯 Quick Example

```bash
# Sentinel detects a change in a child file
🔔 CAMBIO EN: call-inbound.ts
   ℹ️  Archivo hijo detectado, usando tests del módulo: call

✨ CONSEJO DE CLAUDE:
SEGURO - El código sigue correctamente el patrón Repository.

   ✅ Arquitectura aprobada.
🧪 Ejecutando tests: test/calls/calls.spec.ts

 PASS  test/calls/calls.spec.ts
  ✓ should create user (12 ms)
  ✓ should find user by id (8 ms)

   ✅ Tests pasados con éxito

📚 Actualizando manual de bolsillo...
   ✅ Documento generado: src/calls/call.service.md

🚀 Mensaje: feat: add user validation in create method
📝 ¿Commit? (s/n): s
   ✅ Commit exitoso!
```

---

## 🔒 Security

Sentinel automatically protects your API keys:
- ✅ Adds sensitive files to `.gitignore`
- ✅ Per-project configuration (no global variables)
- ✅ Local cache without sharing credentials

[Read more about security →](docs/security.md)

---

## 🌟 Highlighted Features

### Parent File Detection
When you modify a child file (e.g., `call-inbound.ts`), Sentinel automatically detects the parent module (`call.service.ts`) and runs the complete module tests for better coverage.

### Multi-Model System
Use Claude for deep analysis and Gemini as fast fallback. Switch providers without restarting.

### Smart Cache
Reduce costs up to 70% by reusing responses for similar code.

### Productivity Metrics
Automatic tracking of bugs prevented, time saved, and API costs.

[See all features →](docs/configuration.md)

---

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the project
2. Create a branch (`git checkout -b feature/new-feature`)
3. Commit your changes (`git commit -am 'feat: add new feature'`)
4. Push to the branch (`git push origin feature/new-feature`)
5. Open a Pull Request

---

## 📊 Project Status

```
Phase 1: ✅ Completed - Monitoring and basic analysis
Phase 2: ✅ Completed - Productivity and documentation
Phase 3: ✅ Completed - Optimization and stability
Phase 4: ✅ Completed - Multi-model AI & Parent file detection (v4.5.0)
Phase 5: ✅ Completed - CLI Dispatcher, Local LLMs & Rules Engine (v5.0.0-pro)
Phase 6: ✅ Completed - Standalone Knowledge Base & Structural Indexing (v5.0.0-pro.alpha.4)
Phase 7: ✅ Completed - AI Multi-Agent System (Architect, QA, Dev) (v5.0.0-pro.beta.1)
Phase 8: ✅ Completed - Project Audit & ROI System (v5.0.0-pro.beta.2)
Phase 9: ✅ Completed - Refocus: Quality Guardian, Static Analysis & SQLite KB (v5.0.0-pro.beta.3)
```

[View complete roadmap →](docs/roadmap.md)

---

## License
Sentinel Pro is licensed under [AGPL-3.0](LICENSE).
For commercial licensing, contact: sergio.gs8925@gmail.com

---

## 👤 Author

**Sergio Guadarrama**

---

<p align="center">
  <a href="docs/installation.md">Installation</a> •
  <a href="docs/configuration.md">Configuration</a> •
  <a href="docs/commands.md">Commands</a> •
  <a href="docs/troubleshooting.md">Troubleshooting</a>
</p>
