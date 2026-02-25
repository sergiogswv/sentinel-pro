# Capa 4 - Producto: Diseño de Implementación

**Fecha**: 2026-02-25
**Estado**: Aprobado
**Enfoque**: Monolithic + CI/CD

## Visión General

Capa 4 productiza Sentinel para distribución global, documentación completa, telemetría anónima, y auto-actualización automática.

**Orden de implementación**:
1. Distribution System (Cargo, Homebrew, Chocolatey, Binaries)
2. Documentation Portal (Docusaurus)
3. Telemetry System (Privacy-first)
4. Update Command (Silent auto-update)
5. CI/CD Pipeline (GitHub Actions)

---

## 1. DISTRIBUTION ARCHITECTURE

### Cargo (Rust Package Registry)
- **Publicación**: Automática en crates.io al crear tag `v*`
- **Instalación**: `cargo install sentinel-pro`
- **Fuente de verdad**: Versión en `Cargo.toml`
- **Binarios precompilados**: Descargados de GitHub Releases por default

### Homebrew (macOS)
- **Tap**: `sentinel-team/homebrew-sentinel`
- **Fórmula**: `tools/homebrew/sentinel-pro.rb`
- **Arquitecturas**: `x86_64-apple-darwin`, `aarch64-apple-darwin`
- **Instalación**: `brew tap sentinel-team/homebrew-sentinel && brew install sentinel-pro`
- **Auto-update**: `brew upgrade sentinel-pro`

### Chocolatey (Windows)
- **Paquete**: Published to https://chocolatey.org/packages/sentinel-pro
- **Instalación**: `choco install sentinel-pro`
- **Auto-update**: `choco upgrade sentinel-pro`
- **Verificación**: SHA256 checksums en `tools/chocolatey/VERIFICATION.txt`

### GitHub Releases (All Platforms)
- **Binarios precompilados** para cada tag:
  - `sentinel-pro-v5.0.0-x86_64-unknown-linux-gnu.tar.gz`
  - `sentinel-pro-v5.0.0-x86_64-apple-darwin.zip`
  - `sentinel-pro-v5.0.0-aarch64-apple-darwin.zip`
  - `sentinel-pro-v5.0.0-x86_64-pc-windows-msvc.zip`
  - `sentinel-pro-v5.0.0-aarch64-unknown-linux-gnu.tar.gz`
- **Checksums**: SHA256 para cada binario
- **Notas de release**: Generadas desde CHANGELOG.md

### Installation Flow

```
User runs: cargo install sentinel-pro
  ↓
Cargo queries crates.io
  ↓
Crates.io redirects to GitHub Release binary
  ↓
Binary extracted to ~/.cargo/bin/
  ↓
User runs: sentinel --version
Output: sentinel 5.0.0
```

### Version Management
- **Única fuente**: Versión en `Cargo.toml`
- **Sync script**: Extrae versión y la usa en:
  - GitHub Release tag
  - Homebrew formula
  - Chocolatey package
  - Website version
- **Changelog**: Actualizado manualmente antes de crear tag

---

## 2. DOCUMENTATION STRUCTURE

### Docusaurus Portal
**Location**: `website/docs/`
**Hosting**: GitHub Pages o Netlify
**Domain**: `docs.sentinel.dev` (future)

### Documentation Sections

#### Getting Started (5 min read)
- `getting-started.md`: Instalación multi-platform
- Quick start example
- Primeros pasos: `sentinel init`

#### Features & Guides
- `features/custom-rules.md` - Escribir reglas YAML/JSON
  - Pattern rules (regex)
  - AST rules (Tree-sitter queries)
  - Examples de reglas comunes

- `features/java-rust.md` - Soporte Java/Rust
  - Auto-detection
  - Naming conventions
  - Custom rules para Java/Rust

- `features/pre-commit.md` - Pre-commit hooks
  - Setup: `sentinel precommit init`
  - Configuration
  - Bypass options

- `features/github-actions.md` - GitHub Actions integration
  - Workflow templates
  - Configuration examples
  - PR comments y artifacts

#### API Reference
- `api/commands.md` - Todos los comandos CLI
  - `sentinel init`, `audit`, `check`, `fix`, etc.
  - Flags y opciones
  - Output formats (JSON, plain text)

- `api/config.md` - Configuration schema
  - `.sentinelrc.toml` reference
  - All options documented
  - Examples por framework

- `api/rules.md` - Custom rules format
  - YAML vs JSON
  - Pattern rules syntax
  - AST rules & Tree-sitter queries

#### Examples
- `examples/no-console-logs.yaml` - Regla simple
- `examples/no-public-fields.json` - AST rule complexa
- `examples/github-actions-setup.md` - CI/CD workflow real
- `examples/java-project-config.toml` - Java project setup

#### Troubleshooting
- `troubleshooting.md` - Problemas comunes
  - "Sentinel not found" → installation steps
  - "Rule not matching" → debugging tips
  - "Pre-commit hook failing" → common causes
  - "Performance issues" → optimization guide

### README.md Updates
- Link a website documentation
- Multi-platform installation (cargo, homebrew, chocolatey)
- Quick example
- Features overview

### Built-in Help
- `sentinel --help` - Lista todos los comandos
- `sentinel <command> --help` - Help específico del comando
- `sentinel docs` - Opens documentation portal (future)

---

## 3. TELEMETRY SYSTEM

### Data Collection (Standard Level)

**Event Format**:
```json
{
  "event_type": "command_executed",
  "timestamp": "2026-02-25T10:30:00Z",
  "session_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "sentinel_version": "5.0.0",
  "os": "linux",
  "os_version": "6.17.0-14-generic",
  "command": "audit",
  "command_args_count": 2,
  "project_language": "typescript",
  "duration_ms": 1234,
  "rules_executed": 5,
  "custom_rules_count": 2,
  "violations_found": 12,
  "success": true
}
```

### What's NOT Collected
- File names, paths, or content
- API keys or credentials
- Personal information
- Command arguments (only counts)
- Error messages/stack traces

### Telemetry Backend
- **Endpoint**: `https://telemetry.sentinel.dev/events` (HTTPS only)
- **Protocol**: POST JSON
- **Batching**: Envía cada 100 eventos o diariamente
- **Retry**: 3 intentos con exponential backoff
- **Timeout**: 5 segundos (non-blocking)

### Local Storage
- **Log file**: `~/.sentinel/telemetry.log` (JSON lines)
- **Max size**: 10MB rotation
- **Retention**: 30 days local storage

### Privacy & Consent

#### Default Behavior
- **Enabled by default**: Telemetry is active
- **First run**: Banner explains telemetry + how to disable
  ```
  📊 Sentinel collects anonymous telemetry to improve the product.
  No personal data is collected. Disable with: SENTINEL_TELEMETRY=false
  Learn more: https://sentinel.dev/telemetry
  ```

#### Disabling Telemetry
```bash
# Environment variable (takes precedence)
export SENTINEL_TELEMETRY=false

# Configuration file (.sentinelrc.toml)
[telemetry]
enabled = false

# Sentinel respects env var > config file hierarchy
```

### Error Handling
- **Network failure**: Log locally, continue execution
- **Malformed data**: Skip event, continue
- **Endpoint down**: Graceful degradation (no errors shown)
- **User never sees telemetry errors**: Non-blocking by design

### Data Retention
- **Server-side**: 30 days aggregate statistics only
- **No PII stored**: All events anonymized
- **Compliance**: GDPR/CCPA compliant (no personal data)

---

## 4. UPDATE COMMAND IMPLEMENTATION

### Auto-Update Strategy

#### Check Mechanism
- **On startup**: Background thread checks GitHub Release API
- **Cache**: `~/.sentinel/.update-check` (TTL 24h)
- **Non-blocking**: Doesn't delay command execution
- **Silent**: No output unless new version found

#### Update Flow
```
1. Background thread: Check latest release
2. Compare: current version < latest version?
3. If yes: Download binary in background
4. If download complete: Atomic replace at next startup
5. If download fails: Retry next day
6. If new version fails: Rollback to previous
```

#### User Commands
```bash
# Check for updates (immediate)
sentinel update check
Output: v5.0.0 (current) -> v5.0.1 (available)

# Update now (non-blocking)
sentinel update now
Output: Downloading sentinel v5.0.1...
        ✓ Downloaded (2.5MB)
        Will use new version on next command

# Disable auto-update
export SENTINEL_AUTO_UPDATE=false
```

#### Rollback Safety
- **Backup mechanism**: Keeps `~/.sentinel/sentinel.backup`
- **Atomic replacement**: Uses temp file + rename
- **Failure detection**: Runs health check on new binary
- **Auto-rollback**: If new binary fails, restores previous
- **User unaffected**: Always has working sentinel

### Implementation Details

**Module**: `src/update.rs`

**Functions**:
- `check_for_updates()` - Queries GitHub API
- `download_binary()` - Downloads from Release
- `replace_binary()` - Atomic replacement
- `verify_binary()` - Health check on new version
- `rollback_to_previous()` - Restore backup if needed

**Integration**:
- Called in `main.rs` during initialization
- Spawned in separate thread (non-blocking)
- Respects `SENTINEL_AUTO_UPDATE` env var
- Respects `[update] enabled` in config

### Error Handling
- **Network errors**: Log, skip update, retry tomorrow
- **Invalid release**: Skip malformed releases
- **Permission errors**: Log, continue with current version
- **Disk full**: Skip update, log warning
- **All errors are non-fatal**: Always continues with current version

---

## 5. CI/CD PIPELINE (GitHub Actions)

### Release Workflow

**Trigger**: Push tag matching `v*` (e.g., `v5.0.0`)

**Jobs** (Matrix + Sequential):

#### 1. Build Job (Parallel Matrix)
Matrix combinations:
- `os: [ubuntu-latest, macos-latest, windows-latest]`
- `arch: [x86_64, aarch64]` (selective per OS)

Actions:
- Setup Rust toolchain
- Cross-compile for target
- Run `cargo build --release`
- Generate checksums (SHA256)
- Upload artifacts to shared storage

#### 2. Publish Crates.io (Sequential)
Actions:
- Wait for Build job completion
- Run `cargo publish --token ${{ secrets.CARGO_TOKEN }}`
- Wait for crates.io propagation (5 min)
- Verify package is published

#### 3. Create GitHub Release (Sequential)
Actions:
- Download all build artifacts
- Create GitHub Release with tag
- Upload binaries to Release
- Generate checksums file
- Extract changelog from CHANGELOG.md
- Publish release notes

#### 4. Update Homebrew (Sequential)
Actions:
- Fork/update `sentinel-team/homebrew-sentinel` tap
- Update `Formula/sentinel-pro.rb`
- Update version and SHA256
- Commit and push
- Create pull request to tap (auto-merge enabled)

#### 5. Update Chocolatey (Sequential)
Actions:
- Update `tools/chocolatey/tools/chocolateyinstall.ps1`
- Update `tools/VERIFICATION.txt` with SHA256
- Push to chocolatey.org

#### 6. Deploy Website (Sequential)
Actions:
- Run `npm install` in `website/`
- Run `npm run build` (builds Docusaurus)
- Deploy to GitHub Pages
- Invalidate CDN cache (if applicable)

### Version Sync Mechanism

**Script**: `scripts/extract-version.sh`
```bash
#!/bin/bash
# Reads Cargo.toml and extracts version
VERSION=$(grep '^version' Cargo.toml | sed 's/version = "//' | sed 's/".*//')
echo $VERSION
```

**Usage in workflow**:
```yaml
env:
  VERSION: ${{ env.CARGO_VERSION }}

steps:
  - name: Extract version
    run: |
      VERSION=$(scripts/extract-version.sh)
      echo "CARGO_VERSION=$VERSION" >> $GITHUB_ENV
```

### Error Handling & Notifications

**On failure**:
- Job failure stops subsequent jobs
- Slack notification (if configured): mentions #releases channel
- Manual intervention: team reviews logs and decides next step

**On success**:
- Slack notification: "🎉 Sentinel v5.0.0 released!"
- Tweet notification (future)
- GitHub Discussion announcement (future)

### Manual Rollback

If something goes wrong:
```bash
# Delete GitHub release
gh release delete v5.0.0 --yes

# Unpublish from crates.io (manual, requires cargo owner)
cargo yank --vers 5.0.0

# Revert crates.io tag
git tag -d v5.0.0 && git push origin :refs/tags/v5.0.0
```

---

## Data Flow Integration

```
┌──────────────────────────────────────────────┐
│           User Installs Sentinel             │
└─────────────┬────────────────────────────────┘
             │
             ├─▶ Method 1: cargo install
             ├─▶ Method 2: brew install
             ├─▶ Method 3: choco install
             └─▶ Method 4: manual binary download
                │
                └─▶ Runs sentinel --version
                    ├─▶ Starts telemetry background thread
                    ├─▶ Checks for updates (non-blocking)
                    └─▶ Executes command
                         ├─▶ Sends telemetry event
                         ├─▶ If new version ready → use it next time
                         └─▶ Returns results
```

---

## Success Criteria

- ✅ Users can install via 4+ package managers
- ✅ Documentation covers all features
- ✅ Telemetry is privacy-first and optional
- ✅ Updates are automatic and safe
- ✅ CI/CD fully automates releases
- ✅ Version is source of truth (Cargo.toml)
- ✅ No manual steps in release process

---

## Milestones

1. **Week 1**: Distribution system (cargo, binaries, scripts)
2. **Week 2**: Homebrew & Chocolatey integration
3. **Week 3**: Documentation portal (Docusaurus)
4. **Week 4**: Telemetry system
5. **Week 5**: Update command
6. **Week 6**: CI/CD pipeline & release automation

---

## Decisions de Diseño

| Decisión | Rationale |
|----------|-----------|
| Monolithic approach | Simple maintenance, single version source |
| All package managers | Maximum accessibility for users |
| Silent auto-update | Best UX, safety with rollback |
| Standard telemetry | Balance between insights and privacy |
| GitHub Actions CI/CD | Native to repository, no external services |
| Docusaurus docs | Modern, searchable, SEO-friendly |

---

## Riesgos y Mitigaciones

| Riesgo | Mitigación |
|--------|-----------|
| Breaking change in update | Semantic versioning, changelog before release |
| Package manager delays | Parallel publishing, status dashboard |
| Telemetry privacy concerns | Clear opt-out, no PII, GDPR compliance |
| Binary corruption | SHA256 verification, rollback system |
| CI/CD timeout | Parallel jobs, caching, timeout buffers |

