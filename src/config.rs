use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Versión actual de Sentinel (leída desde Cargo.toml en tiempo de compilación)
pub const SENTINEL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Resultado de la detección de framework por IA
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameworkDetection {
    pub framework: String,
    pub rules: Vec<String>,
    pub extensions: Vec<String>,
    pub code_language: String, // Lenguaje para bloques de código (ej: "typescript", "python", "go")
    pub parent_patterns: Vec<String>, // Patrones de archivos padre (ej: [".service.ts", ".controller.ts"])
    pub test_patterns: Vec<String>, // Patrones de ubicación de tests (ej: ["test/{name}/{name}.spec.ts"])
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub url: String,
    pub api_key: String,
    #[serde(default)]
    pub provider: String, // "anthropic", "gemini", "ollama", "lm-studio", "local"
}

impl ModelConfig {
    pub fn embedding_dimension(&self) -> u64 {
        match self.provider.as_str() {
            "local" | "anthropic" => 384,   // all-MiniLM-L6-v2 local model
            "openai" | "lm-studio" => 1536, // typical default for OpenAI embeddings
            "ollama" => {
                if self.name.contains("mxbai") {
                    1024
                } else if self.name.contains("all-minilm") {
                    384
                } else {
                    768
                } // fallback for llama2/nomic
            }
            _ => 768, // Default (Google Gemini, etc.)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeaturesConfig {
    #[serde(default = "default_true")]
    pub enable_ml: bool,
    #[serde(default = "default_true")]
    pub enable_agents: bool,
    #[serde(default = "default_true")]
    pub enable_knowledge_base: bool,
}

fn default_true() -> bool {
    true
}

fn default_complexity() -> usize { 10 }
fn default_function_length() -> usize { 50 }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleConfig {
    #[serde(default = "default_complexity")]
    pub complexity_threshold: usize,
    #[serde(default = "default_function_length")]
    pub function_length_threshold: usize,
    #[serde(default = "default_true")]
    pub dead_code_enabled: bool,
    #[serde(default = "default_true")]
    pub unused_imports_enabled: bool,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            complexity_threshold: 10,
            function_length_threshold: 50,
            dead_code_enabled: true,
            unused_imports_enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AiAnalysisConfig {
    #[serde(default)]
    pub skip_import_validation: bool,
    #[serde(default = "default_true")]
    pub focus_on_logic_quality: bool,
}

impl Default for AiAnalysisConfig {
    fn default() -> Self {
        Self {
            skip_import_validation: false,
            focus_on_logic_quality: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalLlmConfig {
    pub provider: String,
    pub model_path: String,
    pub api_port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MlConfig {
    pub models_path: String,
    pub embeddings_model: String,
    pub bug_predictor_model: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "claude-3-5-sonnet-20241022".to_string(),
            url: "https://api.anthropic.com".to_string(),
            api_key: "".to_string(),
            provider: "anthropic".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SentinelConfig {
    pub version: String,
    pub project_name: String,
    pub framework: String,
    pub manager: String,
    pub test_command: String,
    pub architecture_rules: Vec<String>,
    pub file_extensions: Vec<String>, // Extensiones de archivo a monitorear
    pub code_language: String,        // Lenguaje para bloques de código (detectado por IA)
    pub parent_patterns: Vec<String>, // Patrones de archivos padre específicos del framework
    pub test_patterns: Vec<String>, // Patrones de ubicación de tests (usa {name} como placeholder)
    pub ignore_patterns: Vec<String>,
    pub primary_model: ModelConfig,
    pub fallback_model: Option<ModelConfig>,
    pub use_cache: bool,
    // Testing framework detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub testing_framework: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub testing_status: Option<String>,

    // --- Pro Features ---
    #[serde(default)]
    pub features: Option<FeaturesConfig>,
    #[serde(default)]
    pub local_llm: Option<LocalLlmConfig>,
    #[serde(default)]
    pub ml: Option<MlConfig>,
    #[serde(default)]
    pub rule_config: RuleConfig,
    #[serde(default)]
    pub ai_analysis: AiAnalysisConfig,

    // --- Language Detection (Capa 3) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_languages: Option<Vec<String>>, // Auto-detected: ["java", "rust", "typescript", etc.]
}

impl SentinelConfig {
    pub fn create_default(
        name: String,
        manager: String,
        framework: String,
        rules: Vec<String>,
        extensions: Vec<String>,
        code_language: String,
        parent_patterns: Vec<String>,
        test_patterns: Vec<String>,
    ) -> Self {
        let default_model = ModelConfig {
            name: "claude-3-5-sonnet-20241022".to_string(),
            url: "https://api.anthropic.com".to_string(),
            api_key: "".to_string(),
            provider: "anthropic".to_string(),
        };

        Self {
            version: SENTINEL_VERSION.to_string(),
            project_name: name,
            framework,
            manager: manager.clone(),
            test_command: format!("{} run test", manager),
            architecture_rules: rules,
            file_extensions: extensions,
            code_language,
            parent_patterns,
            test_patterns,
            ignore_patterns: vec![
                "node_modules".to_string(),
                "dist".to_string(),
                ".git".to_string(),
                "build".to_string(),
                ".next".to_string(),
                "target".to_string(),
                "vendor".to_string(),
                "__pycache__".to_string(),
            ],
            primary_model: default_model,
            fallback_model: None,
            use_cache: true,
            testing_framework: None,
            testing_status: None,
            features: Some(FeaturesConfig {
                enable_ml: true,
                enable_agents: true,
                enable_knowledge_base: true,
            }),
            local_llm: Some(LocalLlmConfig {
                provider: "ollama".to_string(),
                model_path: "~/.ollama/models".to_string(),
                api_port: 11434,
            }),
            ml: Some(MlConfig {
                models_path: ".sentinel/models".to_string(),
                embeddings_model: "codebert".to_string(),
                bug_predictor_model: "bug-predictor-v1".to_string(),
            }),
            rule_config: RuleConfig::default(),
            ai_analysis: AiAnalysisConfig::default(),
            detected_languages: None,
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let toml = toml::to_string_pretty(self)?;
        fs::write(path.join(".sentinelrc.toml"), toml)?;

        // Agregar archivos sensibles al .gitignore automáticamente
        Self::actualizar_gitignore(path)?;

        Ok(())
    }

    /// Agrega archivos sensibles de Sentinel al .gitignore para proteger API keys
    pub fn actualizar_gitignore(path: &Path) -> anyhow::Result<()> {
        let gitignore_path = path.join(".gitignore");

        // Entradas que queremos agregar
        let sentinel_entries = vec![
            "# Sentinel - Archivos de configuración y caché (contienen API keys)",
            ".sentinelrc.toml",
            ".sentinel_stats.json",
            ".sentinel/",
        ];

        // Leer .gitignore existente o crear uno nuevo
        let mut content = if gitignore_path.exists() {
            fs::read_to_string(&gitignore_path)?
        } else {
            String::new()
        };

        // Verificar si ya existe la sección de Sentinel
        if !content.contains(".sentinelrc.toml") {
            // Agregar sección de Sentinel
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            for entry in sentinel_entries {
                content.push_str(entry);
                content.push('\n');
            }

            fs::write(&gitignore_path, content)?;
            println!(
                "{}",
                "   ✅ Archivos sensibles agregados a .gitignore".green()
            );
        }

        Ok(())
    }

    /// Carga la configuración desde el archivo .sentinelrc.toml
    ///
    /// Esta función implementa migración automática de configuraciones antiguas
    /// y es tolerante con campos faltantes, usando valores por defecto.
    pub fn load(path: &Path) -> Option<Self> {
        let config_path = path.join(".sentinelrc.toml");
        let content = fs::read_to_string(&config_path).ok()?;

        // Intentar deserializar directamente primero (configuración actual)
        if let Ok(mut config) = toml::from_str::<SentinelConfig>(&content) {
            // Validar y migrar si es necesario
            if config.version != SENTINEL_VERSION {
                println!(
                    "{}",
                    format!(
                        "   🔄 Migrando configuración de versión {} a {}...",
                        config.version, SENTINEL_VERSION
                    )
                    .yellow()
                );
                config = Self::migrar_config(config, path);
                // Guardar la configuración migrada
                let _ = config.save(path);
                println!("{}", "   ✅ Configuración migrada exitosamente".green());
            }
            return Some(config);
        }

        // Si falla, intentar cargar como configuración antigua (sin campo version)
        #[derive(Debug, Deserialize)]
        struct SentinelConfigV1 {
            project_name: Option<String>,
            framework: Option<String>,
            manager: Option<String>,
            test_command: Option<String>,
            architecture_rules: Option<Vec<String>>,
            file_extensions: Option<Vec<String>>,
            ignore_patterns: Option<Vec<String>>,
            primary_model: Option<ModelConfig>,
            fallback_model: Option<ModelConfig>,
            use_cache: Option<bool>,
        }

        if let Ok(old_config) = toml::from_str::<SentinelConfigV1>(&content) {
            println!(
                "{}",
                "   🔄 Detectada configuración antigua, migrando...".yellow()
            );

            // Crear nueva configuración con valores migrados o defaults
            let nombre = old_config.project_name.unwrap_or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

            let gestor = old_config
                .manager
                .unwrap_or_else(|| Self::detectar_gestor(path));

            let framework = old_config
                .framework
                .unwrap_or_else(|| Self::detectar_framework(path));

            let rules = old_config.architecture_rules.unwrap_or_else(|| {
                vec![
                    "Clean Code".to_string(),
                    "SOLID Principles".to_string(),
                    "Best Practices".to_string(),
                ]
            });

            let extensions = old_config
                .file_extensions
                .unwrap_or_else(|| vec!["js".to_string(), "ts".to_string()]);

            // Inferir code_language basado en extensiones (fallback)
            let code_language = if extensions.contains(&"ts".to_string()) {
                "typescript".to_string()
            } else if extensions.contains(&"js".to_string()) {
                "javascript".to_string()
            } else if extensions.contains(&"py".to_string()) {
                "python".to_string()
            } else if extensions.contains(&"go".to_string()) {
                "go".to_string()
            } else if extensions.contains(&"rs".to_string()) {
                "rust".to_string()
            } else {
                "code".to_string()
            };

            // Inferir parent_patterns basados en framework detectado (fallback)
            let parent_patterns = if framework.to_lowercase().contains("nest") {
                vec![
                    ".service.ts".to_string(),
                    ".controller.ts".to_string(),
                    ".repository.ts".to_string(),
                ]
            } else {
                vec![]
            };

            // Inferir test_patterns basados en framework detectado (fallback)
            let test_patterns = if framework.to_lowercase().contains("nest") {
                vec!["test/{name}/{name}.spec.ts".to_string()]
            } else {
                vec!["{name}.test.{ext}".to_string()]
            };

            let mut new_config = Self::create_default(
                nombre,
                gestor,
                framework,
                rules,
                extensions,
                code_language,
                parent_patterns,
                test_patterns,
            );

            // Preservar valores sensibles de la config antigua
            if let Some(model) = old_config.primary_model {
                new_config.primary_model = model;
            }
            if let Some(fallback) = old_config.fallback_model {
                new_config.fallback_model = Some(fallback);
            }
            if let Some(cache) = old_config.use_cache {
                new_config.use_cache = cache;
            }
            if let Some(test_cmd) = old_config.test_command {
                new_config.test_command = test_cmd;
            }
            if let Some(patterns) = old_config.ignore_patterns {
                new_config.ignore_patterns = patterns;
            }

            // Guardar la configuración migrada
            let _ = new_config.save(path);
            println!("{}", "   ✅ Configuración migrada exitosamente".green());

            return Some(new_config);
        }

        println!(
            "{}",
            "   ⚠️  No se pudo cargar la configuración. Se creará una nueva.".yellow()
        );
        None
    }

    /// Migra una configuración de una versión anterior a la versión actual
    fn migrar_config(mut config: SentinelConfig, _path: &Path) -> SentinelConfig {
        // Actualizar versión
        config.version = SENTINEL_VERSION.to_string();

        // Asegurar que todos los campos necesarios existan
        if config.test_command.is_empty() {
            config.test_command = format!("{} run test", config.manager);
        }

        if config.ignore_patterns.is_empty() {
            config.ignore_patterns = vec![
                "node_modules".to_string(),
                "dist".to_string(),
                ".git".to_string(),
                "build".to_string(),
                ".next".to_string(),
                "target".to_string(),
                "vendor".to_string(),
                "__pycache__".to_string(),
            ];
        }

        // Asegurar que haya extensiones configuradas
        if config.file_extensions.is_empty() {
            config.file_extensions = vec!["js".to_string(), "ts".to_string()];
        }

        // Asegurar que haya reglas de arquitectura
        if config.architecture_rules.is_empty() {
            config.architecture_rules = vec![
                "Clean Code".to_string(),
                "SOLID Principles".to_string(),
                "Best Practices".to_string(),
            ];
        }

        // Asegurar que exista code_language (fallback basado en extensiones)
        if config.code_language.is_empty() {
            config.code_language = if config.file_extensions.contains(&"ts".to_string()) {
                "typescript".to_string()
            } else if config.file_extensions.contains(&"js".to_string()) {
                "javascript".to_string()
            } else if config.file_extensions.contains(&"py".to_string()) {
                "python".to_string()
            } else if config.file_extensions.contains(&"go".to_string()) {
                "go".to_string()
            } else if config.file_extensions.contains(&"rs".to_string()) {
                "rust".to_string()
            } else {
                "code".to_string()
            };
        }

        // Asegurar que existan parent_patterns (fallback basado en framework/lenguaje)
        if config.parent_patterns.is_empty() {
            config.parent_patterns = if config.framework.to_lowercase().contains("nest") {
                vec![
                    ".service.ts".to_string(),
                    ".controller.ts".to_string(),
                    ".repository.ts".to_string(),
                    ".gateway.ts".to_string(),
                    ".module.ts".to_string(),
                ]
            } else if config.code_language == "python" {
                vec![
                    "_service.py".to_string(),
                    "_controller.py".to_string(),
                    "_repository.py".to_string(),
                ]
            } else if config.code_language == "go" {
                vec![
                    "_service.go".to_string(),
                    "_handler.go".to_string(),
                    "_repository.go".to_string(),
                ]
            } else {
                vec![] // Sin patrones específicos para otros frameworks
            };
        }

        // Asegurar que existan test_patterns (fallback basado en framework/lenguaje)
        if config.test_patterns.is_empty() {
            config.test_patterns = if config.framework.to_lowercase().contains("nest") {
                vec![
                    "test/{name}/{name}.spec.ts".to_string(),
                    "src/{name}/{name}.spec.ts".to_string(),
                ]
            } else if config.code_language == "python" {
                vec![
                    "tests/test_{name}.py".to_string(),
                    "{name}/tests.py".to_string(),
                ]
            } else if config.code_language == "go" {
                vec!["{name}_test.go".to_string()]
            } else if config.code_language == "php" {
                vec![
                    "tests/Unit/{Name}Test.php".to_string(),
                    "tests/Feature/{Name}Test.php".to_string(),
                ]
            } else {
                vec!["{name}.test.{ext}".to_string()] // Patrón genérico
            };
        }

        // Asegurar que existan secciones de Pro Features (v5.0.0+) con defaults sanos
        if config.features.is_none() {
            config.features = Some(FeaturesConfig {
                enable_ml: true,
                enable_agents: true,
                enable_knowledge_base: true,
            });
        }

        if config.ml.is_none() {
            config.ml = Some(MlConfig {
                models_path: ".sentinel/models".to_string(),
                embeddings_model: "all-MiniLM-L6-v2".to_string(),
                bug_predictor_model: "bug-predictor-v1".to_string(),
            });
        }

        // Limpieza de URLs: asegurar que tengan esquema
        if !config.primary_model.url.contains("://") && !config.primary_model.url.is_empty() {
            config.primary_model.url = format!("https://{}", config.primary_model.url);
        }

        // Migración/Inferencia de proveedor si está vacío o es incorrecto
        let inferir_proveedor = |model: &mut ModelConfig| {
            let url = model.url.to_lowercase();
            if model.provider.is_empty()
                || (model.provider == "anthropic" && !url.contains("anthropic"))
            {
                if url.contains("deepseek") {
                    model.provider = "deepseek".to_string();
                } else if url.contains("googleapis") {
                    model.provider = "gemini".to_string();
                } else if url.contains("groq") {
                    model.provider = "groq".to_string();
                } else if url.contains("moonshot") || url.contains("kimi") {
                    model.provider = "kimi".to_string();
                } else if url.contains("openai") {
                    model.provider = "openai".to_string();
                } else if url.contains("localhost") || url.contains("127.0.0.1") {
                    model.provider = "ollama".to_string();
                }
            }
        };

        inferir_proveedor(&mut config.primary_model);
        if let Some(ref mut fb) = config.fallback_model {
            inferir_proveedor(fb);
        }

        config
    }

    pub fn debe_ignorar(&self, path: &Path) -> bool {
        let path_str = path.to_str().unwrap_or("");

        // 1. Ignorar archivos de tests y sugerencias
        if path_str.contains(".spec.")
            || path_str.contains(".test.")
            || path_str.contains("_test.")
            || path_str.contains(".suggested")
        {
            return true;
        }

        // 2. Validar que tenga una extensión permitida
        let tiene_extension_valida = self
            .file_extensions
            .iter()
            .any(|ext| path_str.ends_with(&format!(".{}", ext)));

        if !tiene_extension_valida {
            return true;
        }

        // 3. Filtros personalizados del config (.sentinelrc)
        self.ignore_patterns
            .iter()
            .any(|pattern| path_str.contains(pattern))
    }

    pub fn detectar_gestor(path: &Path) -> String {
        if path.join("pnpm-lock.yaml").exists() {
            "pnpm".to_string()
        } else if path.join("yarn.lock").exists() {
            "yarn".to_string()
        } else {
            "npm".to_string()
        }
    }

    pub fn detectar_framework(project_root: &Path) -> String {
        // Django: manage.py + settings.py o manage.py
        if project_root.join("manage.py").exists() {
            return "django".to_string();
        }
        // Laravel: artisan
        if project_root.join("artisan").exists() {
            return "laravel".to_string();
        }
        // Spring: pom.xml con spring-boot o build.gradle con spring
        if project_root.join("pom.xml").exists() || project_root.join("build.gradle").exists() {
            if let Ok(content) = fs::read_to_string(project_root.join("pom.xml")) {
                if content.contains("spring-boot") {
                    return "spring".to_string();
                }
            }
        }
        // NestJS: nest-cli.json o package.json con @nestjs/core
        if project_root.join("nest-cli.json").exists() {
            return "nestjs".to_string();
        }
        // Default
        "typescript".to_string()
    }

    /// Lista los archivos en la raíz del proyecto (excluyendo node_modules, .git, etc.)
    pub fn listar_archivos_raiz(path: &Path) -> Vec<String> {
        let mut archivos = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    // Ignorar directorios comunes y archivos ocultos
                    if !file_name.starts_with('.')
                        && file_name != "node_modules"
                        && file_name != "dist"
                        && file_name != "build"
                        && file_name != "target"
                        && file_name != "vendor"
                    {
                        archivos.push(file_name);
                    }
                }
            }
        }

        archivos.sort();
        archivos
    }

    /// Devuelve la ruta al directorio home de Sentinel (~/.sentinel-pro)
    pub fn get_sentinel_home() -> std::path::PathBuf {
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE")
                .map(std::path::PathBuf::from)
                .ok()
        } else {
            std::env::var("HOME").map(std::path::PathBuf::from).ok()
        };

        home.unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".sentinel-pro")
    }

    /// Busca el archivo .sentinelrc.toml caminando hacia arriba por las carpetas
    pub fn find_project_root() -> Option<std::path::PathBuf> {
        let mut current_dir = std::env::current_dir().ok()?;

        loop {
            if current_dir.join(".sentinelrc.toml").exists() {
                return Some(current_dir);
            }

            if !current_dir.pop() {
                break;
            }
        }

        // Si no se encuentra subiendo, probar con el último proyecto activo guardado globalmente
        Self::get_active_project()
    }

    /// Guarda la ruta del proyecto actual como el proyecto "activo" globalmente
    pub fn save_active_project(path: &Path) -> anyhow::Result<()> {
        let sentinel_home = Self::get_sentinel_home();
        if !sentinel_home.exists() {
            fs::create_dir_all(&sentinel_home)?;
        }
        let active_path = sentinel_home.join("active_project");
        fs::write(active_path, path.to_string_lossy().to_string())?;
        Ok(())
    }

    /// Obtiene la ruta del último proyecto activo
    pub fn get_active_project() -> Option<std::path::PathBuf> {
        let active_path = Self::get_sentinel_home().join("active_project");
        if active_path.exists() {
            if let Ok(content) = fs::read_to_string(active_path) {
                let path = std::path::PathBuf::from(content.trim());
                if path.exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    pub fn eliminar(path: &Path) -> anyhow::Result<()> {
        let config_path = path.join(".sentinelrc.toml");
        if config_path.exists() {
            fs::remove_file(config_path)?;
            println!("{}", "🗑️  Configuración eliminada correctamente.".yellow());
        }
        Ok(())
    }

    /// Detect Java project configuration
    ///
    /// Detects Java projects by checking for:
    /// - pom.xml (Maven)
    /// - build.gradle or build.gradle.kts (Gradle)
    /// - .java files in src directory
    pub fn detect_java_project(path: &Path) -> bool {
        // Check for Maven
        if path.join("pom.xml").exists() {
            return true;
        }

        // Check for Gradle
        if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
            return true;
        }

        // Check for src/main/java directory structure
        if path.join("src").join("main").join("java").exists() {
            return true;
        }

        // Check for .java files in src or src subdirectories
        let src_path = path.join("src");
        if src_path.exists() {
            if let Ok(entries) = fs::read_dir(&src_path) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Check if it's a .java file directly in src
                        if name.ends_with(".java") {
                            return true;
                        }
                    }
                    // Also check in subdirectories
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                                for sub_entry in sub_entries.flatten() {
                                    if let Some(sub_name) = sub_entry.file_name().to_str() {
                                        if sub_name.ends_with(".java") {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Detect Rust project configuration
    ///
    /// Detects Rust projects by checking for:
    /// - Cargo.toml
    /// - Cargo.lock
    /// - .rs files in src directory
    pub fn detect_rust_project(path: &Path) -> bool {
        // Check for Cargo.toml
        if path.join("Cargo.toml").exists() {
            return true;
        }

        // Check for Cargo.lock
        if path.join("Cargo.lock").exists() {
            return true;
        }

        // Check for src/*.rs files
        if let Ok(entries) = fs::read_dir(path.join("src")) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".rs") {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Auto-detect supported languages in the project
    ///
    /// Scans the project directory for indicators of different language ecosystems
    /// Returns a vector of detected language names
    pub fn detect_project_languages(path: &Path) -> Vec<String> {
        let mut detected = Vec::new();

        // Detect Java
        if Self::detect_java_project(path) {
            detected.push("java".to_string());
        }

        // Detect Rust
        if Self::detect_rust_project(path) {
            detected.push("rust".to_string());
        }

        // Detect TypeScript/JavaScript (package.json)
        if path.join("package.json").exists() {
            detected.push("typescript".to_string());
        }

        // Detect Python (setup.py, pyproject.toml, requirements.txt)
        if path.join("setup.py").exists()
            || path.join("pyproject.toml").exists()
            || path.join("requirements.txt").exists()
        {
            detected.push("python".to_string());
        }

        // Detect Go (go.mod, go.sum)
        if path.join("go.mod").exists() || path.join("go.sum").exists() {
            detected.push("go".to_string());
        }

        detected
    }
}
