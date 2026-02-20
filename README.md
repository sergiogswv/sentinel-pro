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
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
</p>

---

## 🚀 What is Sentinel?

Real-time monitoring tool written in **Rust** that analyzes code changes using **multiple AI providers** (Claude, Gemini, etc.) and manages workflow with Git. Designed for **NestJS/TypeScript** projects as an intelligent development assistant.

### ✨ Key Features (Pro Edition)

- 🤖 **Advanced AI Orchestration** - Native support for **Ollama**, **LM Studio**, Claude, and Gemini
- 🏗️ **Framework Rule Engine** - YAML-based architecture validation (Pre-AI)
- 🧠 **Local Knowledge Base** - Code indexing and vector store for deep context
- ⚡ **Pro CLI Commands** - `analyze`, `generate`, `refactor`, `fix`, `chat`
- 👥 **Multi-Agent System** - Specialized agents (Coder, Reviewer) for complex tasks
- 💾 **Smart Caching** - Reduces API costs up to 70%
- 📊 **Real-time Metrics** - Tracking bugs, costs, tokens, and productivity
- 🧪 **Autonomous Testing** - AI-assisted test generation and execution
- 🔄 **Advanced Workflows** - Multi-step automation (Fix & Verify, Review & Mitigate)
- 🚀 **Framework Migration** - Intelligent code translation between frameworks
- 🔎 **Interactive Project Audit** - Recursive auditing with selective batch fixing (New!)
- 🎯 **Parent File Detection** - Automatically finds parent modules
- 📚 **Auto-documentation** - Generates technical manuals automatically
- 🔒 **Security Pro** - Local LLM support for 100% offline privacy
- 🚑 **Smart-Heal & Discovery** - Auto-fixes Qdrant connections and finds project root recursively (New!)

---

## 📦 Quick Start

### Requirements

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- API Key from [Claude](https://console.anthropic.com) or [Gemini](https://makersuite.google.com/app/apikey)
- NestJS project with Jest configured

### Installation

#### Windows (PowerShell)
```powershell
git clone https://github.com/your-username/sentinel-rust.git
cd sentinel-rust
./install.ps1
```
*(Note: You may need to restart your terminal after installation for the `sentinel` command to be recognized)*

#### Linux / macOS
```bash
git clone https://github.com/your-username/sentinel-rust.git
cd sentinel-rust
cargo build --release
```

On first use, Sentinel will start an **interactive configuration wizard**.

---

## 🎮 Available Commands

Once started, Sentinel responds to these commands:

| Command | Action |
|---------|--------|
| `p` | Pause/Resume monitoring |
| `r` | Generate daily report |
| `m` | View metrics (bugs, costs, tokens) |
| `l` | Clear cache |
| `t` | Ask AI for test suggestions |
| `k` | **Retry Knowledge Base connection** (Hot-reload) |
| `h` | Show help |
| `x` | Reset configuration |

### Pro CLI Commands

Access advanced features using the `pro` sub-command:

```bash
# Core Commands
sentinel pro analyze <file>   # Deep architectural analysis (Reviewer Agent)
sentinel pro generate <file>  # Generate code from local context (Coder Agent)
sentinel pro refactor <file>  # Proactive refactoring suggestions (Coder Agent)
sentinel pro fix <file>       # Intelligent bug fixing
sentinel pro chat             # Interactive codebase chat

# Advanced Workflow Commands (New!)
sentinel pro workflow <name>  # Execute multi-step workflows (e.g., fix-and-verify)
sentinel pro migrate <s, d>   # Migrate code between frameworks (Pro)
sentinel pro review           # Full project terminology & architectural audit
sentinel pro explain <file>   # Didactic code explanation
sentinel pro audit <path>     # Recursive project audit & interactive fixes (New!)
sentinel pro optimize <file>  # Performance optimization suggestions
sentinel pro docs <dir>       # Generates comprehensive Markdown documentation for a directory
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
Phase 6: ✅ Completed - Local Knowledge Base & Vector Search (v5.0.0-pro.alpha.4)
Phase 7: ✅ Completed - AI Multi-Agent System (Architect, QA, Dev) (v5.0.0-pro.beta.1)
Phase 8: ✅ Completed - Project Audit & ROI System (v5.0.0-pro.beta.2)
Phase 9: ✅ Completed - Smart Discovery, KB Auto-Healing & Installer Parity (v5.0.0-pro.beta.3)
Phase 10: 📅 Planned - Monetization & Licensing (SaaS)
```

[View complete roadmap →](docs/roadmap.md)

---

## 📝 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

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
