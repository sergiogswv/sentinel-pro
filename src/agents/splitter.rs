use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{TaskType, consultar_ia_dinamico};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

pub struct SplitterAgent;

/// Métodos que nunca se extraen (son parte de la estructura del servicio).
const BLACKLIST: &[&str] = &[
    "constructor",
    "onModuleInit",
    "onModuleDestroy",
    "onApplicationBootstrap",
    "beforeApplicationShutdown",
    "onApplicationShutdown",
];

struct FnInfo {
    name: String,
    /// Línea de inicio (incluyendo decoradores @...)
    range_start: usize,
    /// Última línea (cierre `}`)
    range_end: usize,
}

impl SplitterAgent {
    pub fn new() -> Self {
        Self
    }

    // ─── Brace delta (template-literal-aware, regex-aware) ────────────────────

    fn brace_delta(code: &str) -> i32 {
        let mut depth: i32 = 0;
        let mut chars = code.chars().peekable();
        let mut last_non_space: char = ';';

        while let Some(ch) = chars.next() {
            match ch {
                '/' if chars.peek() == Some(&'/') => {
                    for c in chars.by_ref() {
                        if c == '\n' { break; }
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    let mut prev = ' ';
                    for c in chars.by_ref() {
                        if prev == '*' && c == '/' { break; }
                        prev = c;
                    }
                }
                '/' if Self::is_regex_context(last_non_space)
                    && chars.peek().map_or(false, |&c| c != '/' && c != '=') =>
                {
                    let mut esc = false;
                    let mut in_class = false;
                    for c in chars.by_ref() {
                        if esc { esc = false; }
                        else if c == '\\' { esc = true; }
                        else if c == '[' { in_class = true; }
                        else if c == ']' { in_class = false; }
                        else if c == '/' && !in_class {
                            while chars.peek().map_or(false, |c: &char| c.is_alphabetic()) {
                                chars.next();
                            }
                            break;
                        } else if c == '\n' { break; }
                    }
                    last_non_space = '/';
                    continue;
                }
                '"' => {
                    let mut esc = false;
                    for c in chars.by_ref() {
                        if esc { esc = false; }
                        else if c == '\\' { esc = true; }
                        else if c == '"' { break; }
                    }
                }
                '\'' => {
                    let mut esc = false;
                    for c in chars.by_ref() {
                        if esc { esc = false; }
                        else if c == '\\' { esc = true; }
                        else if c == '\'' { break; }
                    }
                }
                '`' => Self::skip_tpl(&mut chars),
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if !ch.is_whitespace() {
                last_non_space = ch;
            }
        }
        depth
    }

    fn is_regex_context(prev: char) -> bool {
        matches!(
            prev,
            '=' | '(' | ',' | '[' | '!' | '&' | '|' | '?' | ':' | '{' | '}' | ';' | '\n'
        )
    }

    fn skip_tpl(chars: &mut std::iter::Peekable<std::str::Chars>) {
        let mut esc = false;
        while let Some(c) = chars.next() {
            if esc { esc = false; continue; }
            match c {
                '\\' => esc = true,
                '`' => break,
                '$' if chars.peek() == Some(&'{') => {
                    chars.next();
                    let mut d = 1i32;
                    while let Some(inner) = chars.next() {
                        match inner {
                            '{' => d += 1,
                            '}' => { d -= 1; if d == 0 { break; } }
                            '`' => Self::skip_tpl(chars),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // ─── Detección de funciones ────────────────────────────────────────────────

    fn extract_fn_infos(content: &str) -> Vec<FnInfo> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();
            if let Some(name) = Self::detect_fn_name(trimmed) {
                if BLACKLIST.contains(&name.as_str()) {
                    i += 1;
                    continue;
                }
                let dec_start = Self::find_decorator_start(&lines, i);
                if let Some(end) = Self::find_fn_end(&lines, i) {
                    result.push(FnInfo { name, range_start: dec_start, range_end: end });
                    i = end + 1;
                    continue;
                }
            }
            i += 1;
        }
        result
    }

    fn detect_fn_name(line: &str) -> Option<String> {
        if line.starts_with('@')
            || line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
            || line.starts_with("import ")
            || line.starts_with("export type ")
            || line.starts_with("export interface ")
            || line.starts_with("interface ")
            || line.starts_with("type ")
            || line.starts_with("get ")
            || line.starts_with("set ")
            || line.is_empty()
        {
            return None;
        }

        let mut rest = line;
        let modifiers = [
            "private ", "protected ", "public ", "static ", "async ",
            "override ", "abstract ", "readonly ", "export ", "default ",
        ];
        let mut changed = true;
        while changed {
            changed = false;
            for m in &modifiers {
                if rest.starts_with(m) {
                    rest = rest[m.len()..].trim_start();
                    changed = true;
                }
            }
        }

        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
            .collect();

        if name.is_empty() { return None; }

        if matches!(
            name.as_str(),
            "function" | "return" | "if" | "for" | "while" | "switch"
                | "class" | "const" | "let" | "var" | "throw" | "new"
        ) {
            return None;
        }

        let after = rest[name.len()..].trim_start();
        if !after.starts_with('(') && !after.starts_with('<') { return None; }

        if line.contains('.') {
            if let Some(pos) = line.find(&*name) {
                let before = &line[..pos];
                if before.trim_end().ends_with('.') { return None; }
            }
        }

        if line.contains('=') && !line.contains(':') { return None; }

        Some(name)
    }

    fn find_decorator_start(lines: &[&str], sig_line: usize) -> usize {
        if sig_line == 0 { return 0; }
        let sig_indent = lines[sig_line].len() - lines[sig_line].trim_start().len();
        let mut start = sig_line;
        let mut j = sig_line;

        loop {
            if j == 0 { break; }
            j -= 1;
            let l = lines[j];
            if l.trim().is_empty() { continue; }
            let indent = l.len() - l.trim_start().len();
            if indent == sig_indent && l.trim().starts_with('@') {
                start = j;
            } else {
                break;
            }
        }
        start
    }

    fn find_fn_end(lines: &[&str], sig_line: usize) -> Option<usize> {
        let mut depth: i32 = 0;
        let mut entered = false;
        for (rel, line) in lines[sig_line..].iter().enumerate() {
            depth += Self::brace_delta(line);
            if !entered && depth > 0 { entered = true; }
            if entered && depth == 0 { return Some(sig_line + rel); }
        }
        None
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    fn filename_to_class(filename: &str) -> String {
        let stem = Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);
        stem.split(|c| c == '-' || c == '_' || c == '.')
            .filter(|p| !p.is_empty())
            .map(|part| {
                let mut c = part.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().to_string() + c.as_str(),
                }
            })
            .collect()
    }

    fn class_to_field(class_name: &str) -> String {
        let mut chars = class_name.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_lowercase().to_string() + chars.as_str(),
        }
    }

    // ─── Plan de división (AI Light) ─────────────────────────────────────────

    async fn plan_split(infos: &[FnInfo], context: &AgentContext) -> Vec<(String, Vec<String>)> {
        let language = &context.config.code_language;
        let framework = &context.config.framework;
        let ext = match language.to_lowercase().as_str() {
            "typescript" | "javascript" => "ts",
            "python" => "py",
            "go" => "go",
            _ => "rs",
        };

        let fn_list = infos
            .iter()
            .enumerate()
            .map(|(i, f)| format!("  [{}] {}", i + 1, f.name))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Eres un Arquitecto de Software experto en {framework} / {language}.\n\n\
            Este archivo tiene los siguientes métodos:\n{fn_list}\n\n\
            Propón cómo dividirlos en archivos separados por dominio o responsabilidad.\n\
            Criterios:\n\
            - Mismo dominio de negocio (ej: contacts, deals, webhooks, properties)\n\
            - Misma responsabilidad técnica (ej: mappers, validators, formatters)\n\
            - Mínimo 2-3 métodos por grupo para que tenga sentido crear un archivo\n\
            - El archivo original conserva los métodos más representativos del servicio\n\n\
            Responde SOLO con JSON válido (sin markdown, sin explicación extra):\n\
            [{{\"filename\": \"name.{ext}\", \"functions\": [\"fn1\", \"fn2\"]}}]\n\
            Si NO hay una división clara, responde solo: []"
        );

        let config = context.config.clone();
        let stats = Arc::clone(&context.stats);
        let root = context.project_root.clone();

        let response = match tokio::task::spawn_blocking(move || {
            consultar_ia_dinamico(prompt, TaskType::Light, &config, stats, &root)
        })
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => { println!("   ⚠️  Plan falló: {}", e); return Vec::new(); }
            Err(e) => { println!("   ⚠️  Plan (spawn) falló: {}", e); return Vec::new(); }
        };

        println!("   🗂️  Plan recibido: {}", response.trim());

        let json_str = crate::ai::utils::extraer_json(&response);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap_or_else(|e| {
            println!("   ⚠️  JSON inválido ({}): {}", e, json_str);
            Vec::new()
        });

        parsed
            .iter()
            .filter_map(|item| {
                let filename = item["filename"].as_str()?.to_string();
                let fns: Vec<String> = item["functions"]
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if fns.is_empty() { None } else { Some((filename, fns)) }
            })
            .collect()
    }

    // ─── Generación del nuevo archivo (AI Deep) ───────────────────────────────

    async fn generate_new_file(
        filename: &str,
        class_name: &str,
        extracted_code: &str,
        context: &AgentContext,
    ) -> String {
        let language = &context.config.code_language;
        let framework = &context.config.framework;

        let prompt = format!(
            "Eres un Arquitecto de Software experto en {framework} / {language}.\n\n\
            Crea el archivo '{filename}' con clase '{class_name}' que contiene estos métodos:\n\n\
            {extracted_code}\n\n\
            FORMATO DE RESPUESTA:\n\
            1. PRIMERO el BLOQUE DE CÓDIGO completo en triple comilla.\n\
               - Clase EXACTAMENTE '{class_name}'\n\
               - Analiza `this.xxx` para inferir dependencias del constructor\n\
               - Para NestJS/TypeScript: @Injectable() + imports correctos de @nestjs\n\
               - Preserva los métodos tal como están (mismo comportamiento)\n\
            2. Una línea indicando los tipos añadidos al constructor."
        );

        let config = context.config.clone();
        let stats = Arc::clone(&context.stats);
        let root = context.project_root.clone();

        let response = match tokio::task::spawn_blocking(move || {
            consultar_ia_dinamico(prompt, TaskType::Deep, &config, stats, &root)
        })
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                println!("   ⚠️  Generación de '{}' falló: {}", filename, e);
                return extracted_code.to_string();
            }
            Err(e) => {
                println!("   ⚠️  Generación de '{}' (spawn) falló: {}", filename, e);
                return extracted_code.to_string();
            }
        };

        let bloques = crate::ai::utils::extraer_todos_bloques(&response);
        if let Some((_, code)) = bloques.into_iter().next() {
            code
        } else {
            println!("   ⚠️  '{}': sin bloque de código, usando código extraído directamente.", filename);
            extracted_code.to_string()
        }
    }

    // ─── TODO comment para el archivo original ────────────────────────────────

    fn build_todo_comment(
        entries: &[(String, String, String, Vec<String>)],
        language: &str,
    ) -> String {
        let c = match language.to_lowercase().as_str() {
            "python" => "#",
            _ => "//",
        };

        let sep = format!("{} {}", c, "═".repeat(68));
        let mut lines = vec![
            sep.clone(),
            format!("{} [sentinel] SPLIT — Archivos generados automáticamente", c),
            format!("{} Completa la migración siguiendo los pasos de cada sección.", c),
            c.to_string(),
        ];

        for (filename, class_name, field_name, fn_names) in entries {
            let stem = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(filename);

            let import_line = match language.to_lowercase().as_str() {
                "typescript" | "javascript" => {
                    format!("import {{ {} }} from './{}';", class_name, stem)
                }
                "python" => format!("from .{} import {}", stem, class_name),
                _ => format!("use {};", stem.replace('-', "_")),
            };

            lines.push(format!("{}   📁 {}  →  {}", c, filename, class_name));
            lines.push(format!(
                "{}      Métodos extraídos : {}",
                c,
                fn_names.join(", ")
            ));
            lines.push(format!("{}      1. Añadir import  : {}", c, import_line));
            lines.push(format!(
                "{}      2. En constructor : private readonly {}: {}",
                c, field_name, class_name
            ));
            lines.push(format!(
                "{}      3. Delegar métodos o eliminarlos de este archivo",
                c
            ));
            if language.to_lowercase().contains("typescript")
                || language.to_lowercase().contains("javascript")
            {
                lines.push(format!(
                    "{}      4. Registrar {} en providers del módulo NestJS",
                    c, class_name
                ));
            }
            lines.push(c.to_string());
        }

        lines.push(sep);
        lines.join("\n") + "\n"
    }
}

// ─── Agent impl ───────────────────────────────────────────────────────────────

#[async_trait]
impl Agent for SplitterAgent {
    fn name(&self) -> &str {
        "SplitterAgent"
    }
    fn description(&self) -> &str {
        "Divide archivos grandes en módulos cohesivos por dominio"
    }

    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult> {
        println!("   ✂️  SplitterAgent: Analizando estructura del archivo...");

        let content = task.context.as_deref().unwrap_or("");
        let language = &context.config.code_language;

        // ── Fase 1: Detectar métodos ──────────────────────────────────────────
        let infos = Self::extract_fn_infos(content);
        if infos.is_empty() {
            println!("   ℹ️  No se encontraron métodos extraíbles.");
            return Ok(TaskResult {
                success: false,
                output: "No se encontraron métodos para dividir.".to_string(),
                files_modified: vec![],
                artifacts: vec![],
            });
        }
        println!("   🔍 {} método(s) detectado(s):", infos.len());
        for f in &infos {
            println!(
                "      {} (líneas {}-{})",
                f.name,
                f.range_start + 1,
                f.range_end + 1
            );
        }

        // ── Fase 2: Plan de división ──────────────────────────────────────────
        let plan = Self::plan_split(&infos, context).await;
        if plan.is_empty() {
            println!("   ℹ️  No se encontró una división clara — el archivo es coherente.");
            return Ok(TaskResult {
                success: false,
                output: "No se encontró una división clara para este archivo.".to_string(),
                files_modified: vec![],
                artifacts: vec![],
            });
        }
        println!("   📋 Plan: {} archivo(s) nuevo(s).", plan.len());

        let fn_map: std::collections::HashMap<&str, &FnInfo> =
            infos.iter().map(|f| (f.name.as_str(), f)).collect();

        let base_dir = task
            .file_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| context.project_root.clone());

        let content_lines: Vec<&str> = content.lines().collect();
        let mut new_files: Vec<std::path::PathBuf> = Vec::new();
        let mut todo_entries: Vec<(String, String, String, Vec<String>)> = Vec::new();
        let mut output_lines: Vec<String> = Vec::new();

        // ── Fase 3: Generar nuevos archivos ───────────────────────────────────
        for (filename, fn_names) in &plan {
            let assigned: Vec<&FnInfo> = fn_names
                .iter()
                .filter_map(|n| fn_map.get(n.as_str()).copied())
                .collect();

            if assigned.is_empty() {
                println!("   ⚠️  '{}': métodos no encontrados en el código.", filename);
                continue;
            }

            let class_name = Self::filename_to_class(filename);
            let field_name = Self::class_to_field(&class_name);

            // Extraer código de las funciones del original
            let extracted_code = assigned
                .iter()
                .map(|f| {
                    let end = f.range_end.min(content_lines.len().saturating_sub(1));
                    content_lines[f.range_start..=end].join("\n")
                })
                .collect::<Vec<_>>()
                .join("\n\n");

            println!("   🔧 Generando '{}' ({})...", filename, class_name);
            let file_content =
                Self::generate_new_file(filename, &class_name, &extracted_code, context).await;

            let file_path = base_dir.join(filename);
            match std::fs::write(&file_path, &file_content) {
                Ok(_) => {
                    println!("   📄 Creado: {}", file_path.display());
                    new_files.push(file_path);

                    let fn_list = assigned.iter().map(|f| f.name.clone()).collect::<Vec<_>>();
                    output_lines.push(format!(
                        "  📤 [{}] → {}",
                        fn_list.join(", "),
                        filename
                    ));
                    todo_entries.push((
                        filename.clone(),
                        class_name,
                        field_name,
                        fn_list,
                    ));
                }
                Err(e) => {
                    println!("   ❌ No se pudo escribir '{}': {}", filename, e);
                }
            }
        }

        if new_files.is_empty() {
            return Ok(TaskResult {
                success: false,
                output: "No se pudo crear ningún archivo nuevo.".to_string(),
                files_modified: vec![],
                artifacts: vec![],
            });
        }

        // ── Fase 4: Escribir TODO comment en el archivo original ──────────────
        let todo_comment = Self::build_todo_comment(&todo_entries, language);
        let original_path = task.file_path.as_ref();

        if let Some(path) = original_path {
            let updated = format!("{}\n{}", todo_comment, content);
            match std::fs::write(path, &updated) {
                Ok(_) => println!("   📝 TODO comment añadido al original."),
                Err(e) => println!("   ⚠️  No se pudo actualizar el original: {}", e),
            }
        }

        println!(
            "   ✅ {} archivo(s) generado(s). Revisa el TODO al inicio del original.",
            new_files.len()
        );

        let output = format!(
            "ARCHIVOS GENERADOS:\n{}\n\n\
             Se añadió un bloque TODO al inicio de tu archivo original con\n\
             las instrucciones exactas para completar la migración manualmente.\n\
             El archivo original NO fue modificado estructuralmente.",
            output_lines.join("\n")
        );

        Ok(TaskResult {
            success: true,
            output,
            files_modified: new_files,
            artifacts: vec![],
        })
    }
}
