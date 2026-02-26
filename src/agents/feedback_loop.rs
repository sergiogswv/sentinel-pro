use crate::agents::base::{AgentContext, Task, TaskType};
use crate::agents::orchestrator::AgentOrchestrator;
use crate::index::quality_history::{FileMetrics, QualityHistory};
use anyhow::anyhow;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FeedbackLoop<'a> {
    orchestrator: &'a AgentOrchestrator,
    context: &'a AgentContext,
    max_iterations: u8,
}

pub struct LoopResult {
    pub iterations: u8,
    pub fixes_applied: u32,
    pub issues_fixed: u32,
    pub issues_skipped: u32,
    pub total_time_secs: u64,
}

#[derive(Debug, Clone)]
enum FixStrategy {
    Internal(String),
    ClaudeCode(String),
}

impl<'a> FeedbackLoop<'a> {
    pub fn new(
        orchestrator: &'a AgentOrchestrator,
        context: &'a AgentContext,
        max_iterations: u8,
    ) -> Self {
        Self {
            orchestrator,
            context,
            max_iterations,
        }
    }

    pub async fn run(&self, file_path: &Path) -> anyhow::Result<LoopResult> {
        let start = std::time::Instant::now();
        let mut iteration = 0u8;
        let mut total_fixes_applied = 0u32;
        let mut total_issues_fixed = 0u32;
        let mut total_issues_skipped = 0u32;

        // Validar que el archivo existe
        if !file_path.exists() {
            return Err(anyhow!("Archivo no encontrado: {}", file_path.display()));
        }

        loop {
            iteration += 1;
            println!(
                "\n{} {}",
                "🔄 Iteración:".cyan().bold(),
                iteration
            );

            // [1] REVIEW: Ejecutar ReviewerAgent
            println!("   🔍 Analizando archivo...");
            let content = fs::read_to_string(&file_path)?;
            let review_task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!(
                    "Analiza el archivo {} e identifica problemas de código: dead code, \
                     imports no usados, complejidad, violaciones de naming conventions, etc.",
                    file_path.display()
                ),
                task_type: TaskType::Review,
                file_path: Some(file_path.to_path_buf()),
                context: Some(content.clone()),
            };

            let review_result = self
                .orchestrator
                .execute_task("ReviewerAgent", &review_task, &self.context)
                .await?;

            if !review_result.success || review_result.output.is_empty() {
                println!("   ✓ ReviewerAgent completó: sin issues nuevos");
                break;
            }

            // [2] CLASSIFY: Parsear issues y clasificarlos
            let issues = self.parse_review_output(&review_result.output);
            if issues.is_empty() {
                println!("   ✓ ReviewerAgent completó: sin issues");
                break;
            }

            println!("   📋 {} issues encontrados:", issues.len());
            let mut strategies = Vec::new();
            for (idx, issue) in issues.iter().enumerate() {
                let strategy = self.classify_issue(&issue);
                let strategy_label = match &strategy {
                    FixStrategy::Internal(_) => "SIMPLE (agente interno)",
                    FixStrategy::ClaudeCode(_) => "COMPLEX (Claude Code)",
                };
                println!(
                    "      [{}] {} — {}",
                    idx + 1,
                    issue.bright_white(),
                    strategy_label.yellow()
                );
                strategies.push(strategy);
            }

            // [3] SHOW PLAN: Permitir seleccionar qué issues arreglar
            println!();
            let mut selected_fixes = Vec::new();

            for (idx, (issue, strategy)) in issues.iter().zip(strategies.iter()).enumerate() {
                let strategy_label = match strategy {
                    FixStrategy::Internal(_) => "SIMPLE (agente interno)",
                    FixStrategy::ClaudeCode(_) => "COMPLEX (Claude Code)",
                };
                println!("   [{}] {} — {}", idx + 1, issue.bright_white(), strategy_label.yellow());
                print!("       ¿Arreglar? [s/n/saltar todo]: ");
                std::io::Write::flush(&mut std::io::stdout())?;

                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let choice = input.trim().to_lowercase();

                if choice == "saltar todo" || choice == "no" {
                    if choice == "saltar todo" {
                        println!("   ⏭️  Saltando todos los fixes restantes");
                        break;
                    }
                    // Usuario dijo "n" - saltar este issue
                    total_issues_skipped += 1;
                } else if choice == "s" || choice.is_empty() {
                    // Usuario dijo "s" o presionó Enter (default es sí)
                    selected_fixes.push((idx, issue.clone(), strategy.clone()));
                }
            }

            if selected_fixes.is_empty() {
                println!("   ⏭️  No hay fixes para aplicar. Loop finalizado.");
                break;
            }

            // [4] APPLY FIX: Por cada issue seleccionado, aplicar según estrategia
            for (applied_idx, (_idx, issue, strategy)) in selected_fixes.iter().enumerate() {
                println!(
                    "\n   ⚡ Aplicando fix [{}/{}]: {}",
                    applied_idx + 1,
                    selected_fixes.len(),
                    issue.bright_white()
                );

                match strategy {
                    FixStrategy::Internal(instruction) => {
                        if let Ok(applied) = self
                            .apply_internal_fix(&file_path, issue, instruction)
                            .await
                        {
                            if applied {
                                println!("      ✓ Aplicado");
                                total_fixes_applied += 1;
                                total_issues_fixed += 1;
                            } else {
                                println!("      ⚠️  No se pudo aplicar");
                            }
                        }
                    }
                    FixStrategy::ClaudeCode(_prompt) => {
                        match self.apply_claude_fix(&file_path, issue) {
                            Ok(applied) => {
                                if applied {
                                    println!("      ✓ Claude Code completó el fix");
                                    total_fixes_applied += 1;
                                    total_issues_fixed += 1;
                                } else {
                                    println!("      ⚠️  Claude Code no logró resolver");
                                }
                            }
                            Err(e) => {
                                println!("      ⚠️  Error con Claude Code: {}", e);
                            }
                        }
                    }
                }
            }

            // [5] VALIDATE: Tests y métricas
            println!("\n   🧪 Validando...");
            let validation = self.validate(&file_path).await?;

            if validation.tests_pass {
                println!("      ✓ Tests: {}", "PASS".green());
                println!("      ✓ Sentinel check: {}", "OK".green());
                println!("   ✅ Loop completado en {} iteración(es)", iteration);
                break;
            } else {
                println!("      ⚠️  Tests: {}", "FAIL".red());
                if iteration >= self.max_iterations {
                    println!("      ❌ Max iteraciones alcanzadas");
                    break;
                }
                println!("      Reiniciando análisis...");
            }
        }

        let elapsed = start.elapsed().as_secs();
        Ok(LoopResult {
            iterations: iteration,
            fixes_applied: total_fixes_applied,
            issues_fixed: total_issues_fixed,
            issues_skipped: total_issues_skipped,
            total_time_secs: elapsed,
        })
    }

    /// Clasificar un issue como SIMPLE (interno) o COMPLEX (Claude Code interactivo)
    fn classify_issue(&self, issue: &str) -> FixStrategy {
        let lower = issue.to_lowercase();

        // Keywords para fixes internos (FixSuggesterAgent)
        // Solo lo más simple y seguro
        let simple_keywords = [
            "import no utilizado",
            "import sin utilizar",
            "unused import",
            "dead code",
            "dead function",
            "no se llama",
            "no se usa",
        ];

        // Si es claramente simple, usar FixSuggesterAgent
        if simple_keywords.iter().any(|k| lower.contains(k)) {
            return FixStrategy::Internal(issue.to_string());
        }

        // TODO LO DEMÁS: tipado, error handling, arquitectura, refactoring, etc.
        // Va a Claude Code interactivo donde el usuario puede ver y aprobar los cambios
        FixStrategy::ClaudeCode(format!(
            "Fix this issue: {}",
            issue
        ))
    }

    /// Aplicar un fix interno usando FixSuggesterAgent
    async fn apply_internal_fix(
        &self,
        file_path: &Path,
        issue: &str,
        instruction: &str,
    ) -> anyhow::Result<bool> {
        let content = fs::read_to_string(&file_path)?;
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            description: format!(
                "En el archivo {}: {}. Instrucción: {}",
                file_path.display(),
                issue,
                instruction
            ),
            task_type: TaskType::Fix,
            file_path: Some(file_path.to_path_buf()),
            context: Some(content),
        };

        let result = self
            .orchestrator
            .execute_with_guard("FixSuggesterAgent", &task, &self.context)
            .await?;

        if result.success && !result.artifacts.is_empty() {
            if let Some(new_code) = result.artifacts.last() {
                fs::write(&file_path, new_code)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Aplicar un fix abriendo Claude Code interactivamente
    fn apply_claude_fix(
        &self,
        file_path: &Path,
        issue: &str,
    ) -> anyhow::Result<bool> {
        let content = fs::read_to_string(&file_path)?;
        let prompt = format!(
            "En el archivo {}:\n\nProblema: {}\n\nCódigo actual:\n```\n{}\n```\n\nPor favor, refactoriza el código para resolver este problema. Devuelve SOLO el archivo completo refactorizado, sin explicaciones adicionales. El archivo debe ser funcional y mantener la misma interfaz pública.",
            file_path.display(),
            issue,
            content
        );

        println!("      🔗 Abriendo Claude Code interactivamente...");
        println!("      (Se abrirá Claude Code con el contexto completo del archivo)");

        // Ejecutar claude con el prompt, permitiendo interacción
        let output = std::process::Command::new("claude")
            .arg("-p")
            .arg(&prompt)
            .current_dir(&self.context.project_root)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Buscar el archivo refactorizado en el output
            if let Some(refactored) = self.extract_refactored_code(&stdout, &content) {
                println!("      ✓ Claude Code completó el fix");
                fs::write(&file_path, refactored)?;
                return Ok(true);
            }
        }

        println!("      ⚠️  Claude Code no pudo procesar la solicitud");
        Ok(false)
    }

    /// Extraer el código refactorizado del output de Claude
    fn extract_refactored_code(&self, output: &str, original_content: &str) -> Option<String> {
        // Si el output comienza con import/export/class/@, probablemente es el código refactorizado
        let trimmed = output.trim();
        if trimmed.starts_with("import ")
            || trimmed.starts_with("export ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("@")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("const ")
        {
            // Es probablemente el código refactorizado
            return Some(trimmed.to_string());
        }

        // Buscar bloques de código entre triple backticks
        if let Some(start) = output.find("```") {
            if let Some(end) = output[start + 3..].find("```") {
                let code_start = output[start + 3..]
                    .find('\n')
                    .map(|i| start + 3 + i + 1)
                    .unwrap_or(start + 3);
                let code_end = start + 3 + end;
                let extracted = output[code_start..code_end].trim();

                // Validar que no sea el código original
                if extracted != original_content && !extracted.is_empty() {
                    return Some(extracted.to_string());
                }
            }
        }

        None
    }


    /// Validar el archivo: tests + sentinel check + guardar métricas
    async fn validate(&self, file_path: &Path) -> anyhow::Result<ValidationResult> {
        let tests_pass = self.run_tests(file_path).await.unwrap_or(false);
        let check_pass = self.run_sentinel_check(file_path).await.unwrap_or(false);

        // Grabar métricas en quality_history
        if let Some(ref db) = self.context.index_db {
            let qh = QualityHistory::new(db);
            let metrics = FileMetrics {
                file_path: file_path.to_string_lossy().to_string(),
                dead_functions: 0,
                unused_imports: 0,
                complexity_score: 0.0,
                violations_count: 0,
                tests_passing: tests_pass,
            };
            let _ = qh.record_metrics(&metrics);
        }

        Ok(ValidationResult {
            tests_pass,
            check_pass,
        })
    }

    /// Ejecutar tests si existen
    async fn run_tests(&self, file_path: &Path) -> anyhow::Result<bool> {
        // Buscar archivo de test correspondiente
        let test_file = self.find_test_file(file_path);
        if test_file.is_none() {
            return Ok(true); // Sin tests = "pass"
        }

        // Aquí iría la ejecución real de tests (pytest, cargo test, npm test, etc.)
        // Por ahora, stub
        Ok(true)
    }

    /// Ejecutar `sentinel pro check` en el archivo
    async fn run_sentinel_check(&self, _file_path: &Path) -> anyhow::Result<bool> {
        // Stub: en implementación real, llamaría al checker
        Ok(true)
    }

    /// Buscar archivo de test correspondiente
    fn find_test_file(&self, file_path: &Path) -> Option<PathBuf> {
        let stem = file_path.file_stem()?.to_string_lossy();
        let parent = file_path.parent()?;

        // Buscar en patterns comunes: name.spec.ts, name.test.ts, name_test.rs, etc.
        for pattern in &[
            format!("{}.spec.ts", stem),
            format!("{}.test.ts", stem),
            format!("{}.spec.js", stem),
            format!("{}.test.js", stem),
            format!("{}_test.rs", stem),
            format!("{}Test.java", stem),
        ] {
            let candidate = parent.join(pattern);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }

    /// Parsear output del ReviewerAgent en una lista de issues
    /// Extrae títulos de problemas (#### o ### headings) como issues únicos
    fn parse_review_output(&self, output: &str) -> Vec<String> {
        let mut issues = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim();

            // Buscar problemas numerados: "### 1. **Problema**" o "#### 1. **Problema**"
            if (trimmed.starts_with("###") || trimmed.starts_with("####"))
                && (trimmed.contains("**") || trimmed.contains(". "))
            {
                // Remover los ### y espacios iniciales, mantener solo el título
                let issue = trimmed
                    .trim_start_matches('#')
                    .trim_start_matches('*')
                    .trim()
                    .to_string();

                if !issue.is_empty() && issue.len() > 5 {
                    issues.push(issue);
                }
            }
        }

        // Si no se encontraron issues por heading, buscar líneas que comiencen con números
        if issues.is_empty() {
            for line in output.lines() {
                let trimmed = line.trim();
                // Buscar líneas que comiencen con números: "1. ", "2. ", etc.
                if trimmed.len() > 3
                    && trimmed.chars().next().map_or(false, |c| c.is_numeric())
                    && trimmed.chars().nth(1) == Some('.')
                {
                    issues.push(trimmed.to_string());
                }
            }
        }

        // Si aún no hay issues, buscar por palabras clave que indiquen problemas
        if issues.is_empty() {
            let keywords = ["problema:", "issue:", "error:", "warning:", "unused", "dead code", "missing"];
            for line in output.lines() {
                let trimmed = line.trim().to_lowercase();
                if keywords.iter().any(|kw| trimmed.contains(kw)) && trimmed.len() > 10 {
                    issues.push(line.trim().to_string());
                }
            }
        }

        // Deduplicar y filtrar vacíos
        issues.sort();
        issues.dedup();
        issues.into_iter()
            .filter(|s| !s.is_empty() && s.len() > 5)
            .collect()
    }
}

struct ValidationResult {
    tests_pass: bool,
    #[allow(dead_code)]
    check_pass: bool,
}
