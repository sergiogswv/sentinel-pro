use crate::rules::static_analysis::{StaticAnalyzer, DeadCodeAnalyzer, UnusedImportsAnalyzer, ComplexityAnalyzer};

/// Returns the set of static analyzers for TypeScript/JavaScript files.
pub fn analyzers() -> Vec<Box<dyn StaticAnalyzer + Send + Sync>> {
    vec![
        Box::new(DeadCodeAnalyzer::new()),
        Box::new(UnusedImportsAnalyzer::new()),
        Box::new(ComplexityAnalyzer::new()),
    ]
}
