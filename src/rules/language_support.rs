//! Language support utilities for Tree-sitter based parsing and AST analysis
//!
//! This module provides language-specific parsing capabilities for Java, Rust, and other
//! languages using Tree-sitter. It enables AST-based custom rules evaluation.

use tree_sitter::{Language, Parser, Tree, QueryCursor, StreamingIterator};

/// Supported programming languages for AST analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    Java,
    Rust,
    TypeScript,
    JavaScript,
    Go,
    Python,
}

impl SupportedLanguage {
    /// Get the language identifier string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Java => "java",
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Go => "go",
            Self::Python => "python",
        }
    }

    /// Parse a language string into a SupportedLanguage enum
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "java" => Some(Self::Java),
            "rust" => Some(Self::Rust),
            "typescript" | "ts" => Some(Self::TypeScript),
            "javascript" | "js" => Some(Self::JavaScript),
            "go" => Some(Self::Go),
            "python" | "py" => Some(Self::Python),
            _ => None,
        }
    }

    /// Get the file extensions for this language
    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Java => &["java"],
            Self::Rust => &["rs"],
            Self::TypeScript => &["ts", "tsx"],
            Self::JavaScript => &["js", "jsx"],
            Self::Go => &["go"],
            Self::Python => &["py"],
        }
    }
}

/// Tree-sitter parser wrapper for a specific language
pub struct LanguageParser {
    language: SupportedLanguage,
    parser: Parser,
}

impl LanguageParser {
    /// Create a new parser for the specified language
    pub fn new(language: SupportedLanguage) -> Result<Self, String> {
        let mut parser = Parser::new();
        let tree_sitter_lang = Self::get_tree_sitter_language(language)?;
        parser
            .set_language(&tree_sitter_lang)
            .map_err(|e| format!("Failed to set language: {}", e))?;

        Ok(Self { language, parser })
    }

    /// Parse source code and return the syntax tree
    pub fn parse(&mut self, source_code: &str) -> Result<Tree, String> {
        self.parser
            .parse(source_code, None)
            .ok_or_else(|| "Failed to parse source code".to_string())
    }

    /// Get the supported language
    pub fn language(&self) -> SupportedLanguage {
        self.language
    }

    /// Get the Tree-sitter Language object for a supported language
    fn get_tree_sitter_language(lang: SupportedLanguage) -> Result<Language, String> {
        match lang {
            SupportedLanguage::Java => {
                Ok(tree_sitter_java::LANGUAGE.into())
            }
            SupportedLanguage::Rust => {
                Ok(tree_sitter_rust::language())
            }
            SupportedLanguage::TypeScript => {
                Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            }
            SupportedLanguage::JavaScript => {
                Ok(tree_sitter_javascript::LANGUAGE.into())
            }
            SupportedLanguage::Go => {
                Ok(tree_sitter_go::LANGUAGE.into())
            }
            SupportedLanguage::Python => {
                Ok(tree_sitter_python::LANGUAGE.into())
            }
        }
    }
}

/// Query results from Tree-sitter
#[derive(Debug, Clone)]
pub struct QueryMatch {
    pub node_type: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: (usize, usize), // (row, column)
    pub end_point: (usize, usize),   // (row, column)
    pub text: String,
}

/// Executes a Tree-sitter query on a parsed tree
pub fn execute_query(
    language: SupportedLanguage,
    tree: &Tree,
    source_code: &str,
    query_string: &str,
) -> Result<Vec<QueryMatch>, String> {
    let tree_sitter_lang = LanguageParser::get_tree_sitter_language(language)?;

    let query = tree_sitter::Query::new(&tree_sitter_lang, query_string)
        .map_err(|e| format!("Invalid query: {}", e))?;

    let mut cursor = QueryCursor::new();
    let mut results = Vec::new();

    let mut captures = cursor.captures(&query, tree.root_node(), source_code.as_bytes());

    while let Some((m, _)) = captures.next() {
        for capture in m.captures {
            let node = capture.node;
            let start_row = node.start_position().row;
            let start_col = node.start_position().column;
            let end_row = node.end_position().row;
            let end_col = node.end_position().column;

            // Extract the text for this capture
            let text = if let Some(source_slice) =
                source_code.get(node.start_byte()..node.end_byte())
            {
                source_slice.to_string()
            } else {
                String::new()
            };

            results.push(QueryMatch {
                node_type: node.kind().to_string(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_point: (start_row, start_col),
                end_point: (end_row, end_col),
                text,
            });
        }
    }

    Ok(results)
}

/// Detect language from file extension
pub fn detect_language_from_extension(file_path: &str) -> Option<SupportedLanguage> {
    let ext = std::path::Path::new(file_path)
        .extension()?
        .to_str()?
        .to_lowercase();

    match ext.as_str() {
        "java" => Some(SupportedLanguage::Java),
        "rs" => Some(SupportedLanguage::Rust),
        "ts" | "tsx" => Some(SupportedLanguage::TypeScript),
        "js" | "jsx" => Some(SupportedLanguage::JavaScript),
        "go" => Some(SupportedLanguage::Go),
        "py" => Some(SupportedLanguage::Python),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_language_from_str() {
        assert_eq!(SupportedLanguage::from_str("java"), Some(SupportedLanguage::Java));
        assert_eq!(SupportedLanguage::from_str("rust"), Some(SupportedLanguage::Rust));
        assert_eq!(SupportedLanguage::from_str("typescript"), Some(SupportedLanguage::TypeScript));
        assert_eq!(SupportedLanguage::from_str("ts"), Some(SupportedLanguage::TypeScript));
        assert_eq!(SupportedLanguage::from_str("javascript"), Some(SupportedLanguage::JavaScript));
        assert_eq!(SupportedLanguage::from_str("js"), Some(SupportedLanguage::JavaScript));
        assert_eq!(SupportedLanguage::from_str("python"), Some(SupportedLanguage::Python));
        assert_eq!(SupportedLanguage::from_str("py"), Some(SupportedLanguage::Python));
        assert_eq!(SupportedLanguage::from_str("unknown"), None);
    }

    #[test]
    fn test_supported_language_as_str() {
        assert_eq!(SupportedLanguage::Java.as_str(), "java");
        assert_eq!(SupportedLanguage::Rust.as_str(), "rust");
        assert_eq!(SupportedLanguage::TypeScript.as_str(), "typescript");
    }

    #[test]
    fn test_file_extensions() {
        assert_eq!(SupportedLanguage::Java.file_extensions(), &["java"]);
        assert_eq!(SupportedLanguage::Rust.file_extensions(), &["rs"]);
        assert_eq!(SupportedLanguage::TypeScript.file_extensions(), &["ts", "tsx"]);
    }

    #[test]
    fn test_detect_language_from_extension() {
        assert_eq!(detect_language_from_extension("Test.java"), Some(SupportedLanguage::Java));
        assert_eq!(detect_language_from_extension("main.rs"), Some(SupportedLanguage::Rust));
        assert_eq!(detect_language_from_extension("app.ts"), Some(SupportedLanguage::TypeScript));
        assert_eq!(detect_language_from_extension("script.js"), Some(SupportedLanguage::JavaScript));
        assert_eq!(detect_language_from_extension("config.toml"), None);
    }

    #[test]
    fn test_parse_java_simple_code() {
        let code = "public class HelloWorld { }";
        let mut parser = LanguageParser::new(SupportedLanguage::Java)
            .expect("Failed to create Java parser");

        let tree = parser.parse(code).expect("Failed to parse Java code");
        assert!(!tree.root_node().is_null());
    }

    #[test]
    fn test_parse_rust_simple_code() {
        let code = "fn main() { }";
        let mut parser = LanguageParser::new(SupportedLanguage::Rust)
            .expect("Failed to create Rust parser");

        let tree = parser.parse(code).expect("Failed to parse Rust code");
        assert!(!tree.root_node().is_null());
    }

    #[test]
    fn test_parse_typescript_simple_code() {
        let code = "function hello() { }";
        let mut parser = LanguageParser::new(SupportedLanguage::TypeScript)
            .expect("Failed to create TypeScript parser");

        let tree = parser.parse(code).expect("Failed to parse TypeScript code");
        assert!(!tree.root_node().is_null());
    }

    #[test]
    fn test_java_query_execution() {
        let code = "public class Test { public void method() { } }";
        let mut parser = LanguageParser::new(SupportedLanguage::Java)
            .expect("Failed to create Java parser");

        let tree = parser.parse(code).expect("Failed to parse Java code");

        // Query for method declarations
        let query = "(method_declaration name: (identifier) @method)";
        let results = execute_query(SupportedLanguage::Java, &tree, code, query)
            .expect("Failed to execute query");

        assert!(!results.is_empty(), "Should find method declarations");
    }

    #[test]
    fn test_rust_query_execution() {
        let code = "fn test_function() { }";
        let mut parser = LanguageParser::new(SupportedLanguage::Rust)
            .expect("Failed to create Rust parser");

        let tree = parser.parse(code).expect("Failed to parse Rust code");

        // Query for function declarations
        let query = "(function_item name: (identifier) @func)";
        let results = execute_query(SupportedLanguage::Rust, &tree, code, query)
            .expect("Failed to execute query");

        assert!(!results.is_empty(), "Should find function declarations");
    }

    #[test]
    fn test_invalid_query() {
        let code = "fn test() { }";
        let mut parser = LanguageParser::new(SupportedLanguage::Rust)
            .expect("Failed to create Rust parser");

        let tree = parser.parse(code).expect("Failed to parse Rust code");

        // Invalid query syntax should return an error
        let query = "(invalid [[[";
        let result = execute_query(SupportedLanguage::Rust, &tree, code, query);

        assert!(result.is_err(), "Invalid query should return error");
    }
}
