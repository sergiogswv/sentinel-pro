pub mod typescript;
pub mod go;

use tree_sitter::Language;
use crate::rules::static_analysis::StaticAnalyzer;

/// Returns the tree-sitter Language and the set of analyzers for the given file extension.
/// Returns None for unsupported extensions.
pub fn get_language_and_analyzers(
    ext: &str,
) -> Option<(Language, Vec<Box<dyn StaticAnalyzer + Send + Sync>>)> {
    match ext {
        "ts" | "tsx" => Some((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            typescript::analyzers(),
        )),
        "js" | "jsx" => Some((
            tree_sitter_javascript::LANGUAGE.into(),
            typescript::analyzers(),
        )),
        "go" => Some((
            tree_sitter_go::LANGUAGE.into(),
            go::analyzers(),
        )),
        _ => None,
    }
}
