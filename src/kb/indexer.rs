use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub file_path: String,
    pub metadata: SymbolMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Method,
    Interface,
    Import,
    Export,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SymbolMetadata {
    pub parent_class: Option<String>,
    pub decorators: Vec<String>,
    pub return_type: Option<String>,
    pub params: Vec<String>,
}

pub struct CodeIndex {
    parser: Parser,
    language: Language,
}

impl CodeIndex {
    pub fn new_typescript() -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        parser
            .set_language(&language)
            .expect("Error al cargar gramática TS");

        Self { parser, language }
    }

    pub fn parse_file(&mut self, path: &Path) -> anyhow::Result<Vec<CodeSymbol>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let source_code = std::fs::read_to_string(path)?;
        let tree = self
            .parser
            .parse(&source_code, None)
            .ok_or_else(|| anyhow::anyhow!("Error al parsear archivo {:?}", path))?;

        let mut symbols = Vec::new();
        let root_node = tree.root_node();

        let query_str = r#"
            (function_declaration name: (identifier) @func_name)
            (class_declaration name: (type_identifier) @class_name)
            (method_definition name: (property_identifier) @method_name)
            (interface_declaration name: (type_identifier) @interface_name)
            (import_statement) @import
        "#;

        let query = Query::new(&self.language, query_str)?;
        let mut cursor = QueryCursor::new();

        let mut captures = cursor.captures(&query, root_node, source_code.as_bytes());

        while let Some(m) = captures.next() {
            let (query_match, capture_index) = m;
            let capture = query_match.captures[*capture_index];
            let node = capture.node;

            let capture_name = &query.capture_names()[capture.index as usize];
            let kind = match &capture_name[..] {
                "func_name" => SymbolKind::Function,
                "class_name" => SymbolKind::Class,
                "method_name" => SymbolKind::Method,
                "interface_name" => SymbolKind::Interface,
                "import" => SymbolKind::Import,
                _ => continue,
            };

            let name = node.utf8_text(source_code.as_bytes())?.to_string();
            let parent = node.parent().unwrap_or(node);

            symbols.push(CodeSymbol {
                name,
                kind,
                start_line: parent.start_position().row + 1,
                end_line: parent.end_position().row + 1,
                content: parent.utf8_text(source_code.as_bytes())?.to_string(),
                file_path: path.to_string_lossy().to_string(),
                metadata: SymbolMetadata::default(),
            });
        }

        Ok(symbols)
    }

    pub fn index_all_project(&mut self, root: &Path) -> anyhow::Result<Vec<CodeSymbol>> {
        let mut all_symbols = Vec::new();
        self.explore_dir(root, &mut all_symbols)?;
        Ok(all_symbols)
    }

    fn explore_dir(&mut self, dir: &Path, symbols: &mut Vec<CodeSymbol>) -> anyhow::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name != "node_modules"
                        && name != "dist"
                        && name != ".git"
                        && name != "target"
                    {
                        self.explore_dir(&path, symbols)?;
                    }
                } else {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "ts" || ext == "js" {
                        if let Ok(mut file_symbols) = self.parse_file(&path) {
                            symbols.append(&mut file_symbols);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
