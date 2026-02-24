use std::path::Path;
use std::fs;
use sha2::{Sha256, Digest};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use crate::index::db::IndexDb;
use rusqlite::params;

pub struct ProjectIndexBuilder {
    db: std::sync::Arc<IndexDb>,
}

impl ProjectIndexBuilder {
    pub fn new(db: std::sync::Arc<IndexDb>) -> Self {
        Self { db }
    }

    pub fn index_project(&self, root: &Path, extensions: &[String]) -> anyhow::Result<()> {
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for result in walker {
            if let Ok(entry) = result {
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if extensions.contains(&ext.to_string()) {
                        self.index_file(path, root)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn index_file(&self, path: &Path, root: &Path) -> anyhow::Result<bool> {
        let content = fs::read_to_string(path)?;
        let hash = self.calculate_hash(&content);
        let rel_path = path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string();

        // Fase 1: verificar hash y limpiar datos anteriores.
        // El bloque { } suelta el MutexGuard antes de llamar a parse_and_fill,
        // que necesita adquirir el mismo mutex (evita deadlock).
        {
            let conn = self.db.lock();
            let mut stmt = conn.prepare("SELECT content_hash FROM file_index WHERE file_path = ?")?;
            let existing_hash: Option<String> = stmt.query_row(params![rel_path], |row| row.get(0)).ok();

            if existing_hash == Some(hash.clone()) {
                return Ok(false); // Sin cambios, no reindexar
            }

            conn.execute("DELETE FROM symbols WHERE file_path = ?", params![rel_path])?;
            conn.execute("DELETE FROM call_graph WHERE caller_file = ?", params![rel_path])?;
            conn.execute("DELETE FROM import_usage WHERE file_path = ?", params![rel_path])?;
        } // MutexGuard suelto aquí

        // Fase 2: parsear con tree-sitter (adquiere el mutex internamente)
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let language = match ext {
            "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
            "go"         => Some(tree_sitter_go::LANGUAGE.into()),
            "py"         => Some(tree_sitter_python::LANGUAGE.into()),
            _ => None,
        };

        if let Some(lang) = language {
            self.parse_and_fill(&lang, &content, &rel_path)?;
        }

        // Fase 3: actualizar índice de archivos
        let conn = self.db.lock();
        conn.execute(
            "INSERT OR REPLACE INTO file_index (file_path, content_hash, last_indexed) VALUES (?, ?, CURRENT_TIMESTAMP)",
            params![rel_path, hash],
        )?;

        Ok(true)
    }

    fn calculate_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn parse_and_fill(&self, language: &tree_sitter::Language, content: &str, rel_path: &str) -> anyhow::Result<()> {
        let mut parser = Parser::new();
        parser.set_language(language)?;
        let tree = parser.parse(content, None).unwrap();
        let root_node = tree.root_node();

        // 1. Extraer Símbolos
        let symbol_query_str = r#"
            (function_declaration name: (identifier) @name) @func
            (method_definition name: (property_identifier) @name) @method
            (class_declaration name: (identifier) @name) @class
            (variable_declarator name: (identifier) @name) @var
        "#;
        let symbol_query = Query::new(language, symbol_query_str)?;
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&symbol_query, root_node, content.as_bytes());

        let conn = self.db.lock();

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(content.as_bytes()).unwrap_or("");
                let kind = match capture.index {
                    0 | 1 => "function",
                    2 | 3 => "method",
                    4 | 5 => "class",
                    6 | 7 => "variable",
                    _ => "unknown",
                };
                
                // Avoid duplicates by only taking the @name capture for storage
                if symbol_query.capture_names()[capture.index as usize] == "name" {
                    let range = capture.node.range();
                    conn.execute(
                        "INSERT INTO symbols (name, kind, file_path, line_start, line_end) VALUES (?, ?, ?, ?, ?)",
                        params![name, kind, rel_path, range.start_point.row as i32, range.end_point.row as i32],
                    )?;
                }
            }
        }

        // 2. Extraer Grafo de Llamadas (Simplificado)
        let call_query_str = r#"
            (call_expression
                function: [(identifier) @callee (member_expression property: (property_identifier) @callee)]
            ) @call
        "#;
        let call_query = Query::new(language, call_query_str)?;
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&call_query, root_node, content.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                if call_query.capture_names()[capture.index as usize] == "callee" {
                    let callee_name = capture.node.utf8_text(content.as_bytes()).unwrap_or("");
                    let range = capture.node.range();
                    
                    conn.execute(
                        "INSERT INTO call_graph (caller_file, caller_symbol, callee_symbol, line_number) VALUES (?, ?, ?, ?)",
                        params![rel_path, "unknown", callee_name, range.start_point.row as i32],
                    )?;
                }
            }
        }

        // 3. Extraer Imports
        let import_query_str = r#"
            (import_specifier name: (identifier) @name)
            (import_clause (identifier) @name)
        "#;
        let import_query = Query::new(language, import_query_str)?;
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&import_query, root_node, content.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let import_name = capture.node.utf8_text(content.as_bytes()).unwrap_or("");
                conn.execute(
                    "INSERT INTO import_usage (file_path, import_name, import_src) VALUES (?, ?, ?)",
                    params![rel_path, import_name, "unknown"], // src requires more complex parsing
                )?;
            }
        }

        Ok(())
    }
}
