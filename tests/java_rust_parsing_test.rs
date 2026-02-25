//! Integration tests for Java and Rust language parsing with Tree-sitter

use sentinel_pro::rules::language_support::{
    LanguageParser, SupportedLanguage, detect_language_from_extension, execute_query,
};

#[test]
fn test_parse_java_class() {
    let java_code = r#"
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Java)
        .expect("Failed to create Java parser");

    let tree = parser
        .parse(java_code)
        .expect("Failed to parse Java code");

    // Verify the tree was created successfully
    // Verify tree was created
    let _ = tree.root_node();
    assert_eq!(parser.language(), SupportedLanguage::Java);
}

#[test]
fn test_parse_rust_function() {
    let rust_code = r#"
fn factorial(n: u32) -> u32 {
    match n {
        0 | 1 => 1,
        _ => n * factorial(n - 1),
    }
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Rust)
        .expect("Failed to create Rust parser");

    let tree = parser
        .parse(rust_code)
        .expect("Failed to parse Rust code");

    // Verify the tree was created successfully
    // Verify tree was created
    let _ = tree.root_node();
    assert_eq!(parser.language(), SupportedLanguage::Rust);
}

#[test]
fn test_parse_complex_java_code() {
    let java_code = r#"
public class DataProcessor {
    private List<String> items;

    public DataProcessor() {
        this.items = new ArrayList<>();
    }

    public void processItem(String item) {
        items.add(item);
    }

    public int getSize() {
        return items.size();
    }
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Java)
        .expect("Failed to create Java parser");

    let tree = parser
        .parse(java_code)
        .expect("Failed to parse Java code");

    // Verify tree was created
    let _ = tree.root_node();
}

#[test]
fn test_parse_complex_rust_code() {
    let rust_code = r#"
struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Person {
        Person { name, age }
    }

    fn birthday(&mut self) {
        self.age += 1;
    }
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Rust)
        .expect("Failed to create Rust parser");

    let tree = parser
        .parse(rust_code)
        .expect("Failed to parse Rust code");

    // Verify tree was created
    let _ = tree.root_node();
}

#[test]
fn test_detect_java_file_extension() {
    assert_eq!(
        detect_language_from_extension("Example.java"),
        Some(SupportedLanguage::Java)
    );
    assert_eq!(
        detect_language_from_extension("src/main/java/App.java"),
        Some(SupportedLanguage::Java)
    );
}

#[test]
fn test_detect_rust_file_extension() {
    assert_eq!(
        detect_language_from_extension("main.rs"),
        Some(SupportedLanguage::Rust)
    );
    assert_eq!(
        detect_language_from_extension("src/lib.rs"),
        Some(SupportedLanguage::Rust)
    );
}

#[test]
fn test_detect_typescript_file_extension() {
    assert_eq!(
        detect_language_from_extension("app.ts"),
        Some(SupportedLanguage::TypeScript)
    );
    assert_eq!(
        detect_language_from_extension("App.tsx"),
        Some(SupportedLanguage::TypeScript)
    );
}

#[test]
fn test_detect_unknown_file_extension() {
    assert_eq!(detect_language_from_extension("config.toml"), None);
    assert_eq!(detect_language_from_extension("README.md"), None);
}

#[test]
fn test_java_method_query() {
    let java_code = r#"
public class Calculator {
    public int add(int a, int b) {
        return a + b;
    }
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Java)
        .expect("Failed to create Java parser");

    let tree = parser
        .parse(java_code)
        .expect("Failed to parse Java code");

    // Query for method declarations
    let query = "(method_declaration name: (identifier) @method)";
    let results = execute_query(SupportedLanguage::Java, &tree, java_code, query)
        .expect("Failed to execute query");

    // Should find the add method
    assert!(!results.is_empty(), "Should find method declarations");
    // Check if the method text contains 'add'
    assert!(
        results.iter().any(|m| m.text.contains("add")),
        "Should find the 'add' method"
    );
}

#[test]
fn test_rust_function_query() {
    let rust_code = r#"
fn greet(name: &str) -> String {
    format!("Hello, {}", name)
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Rust)
        .expect("Failed to create Rust parser");

    let tree = parser
        .parse(rust_code)
        .expect("Failed to parse Rust code");

    // Query for function declarations
    let query = "(function_item name: (identifier) @func)";
    let results = execute_query(SupportedLanguage::Rust, &tree, rust_code, query)
        .expect("Failed to execute query");

    // Should find the greet function
    assert!(!results.is_empty(), "Should find function declarations");
}

#[test]
fn test_java_class_declaration_query() {
    let java_code = r#"
public class MyApplication {
    private String version = "1.0";
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Java)
        .expect("Failed to create Java parser");

    let tree = parser
        .parse(java_code)
        .expect("Failed to parse Java code");

    // Query for class declarations
    let query = "(class_declaration name: (identifier) @class)";
    let results = execute_query(SupportedLanguage::Java, &tree, java_code, query)
        .expect("Failed to execute query");

    assert!(!results.is_empty(), "Should find class declarations");
}

#[test]
fn test_rust_struct_query() {
    let rust_code = r#"
struct Book {
    title: String,
    author: String,
    pages: usize,
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Rust)
        .expect("Failed to create Rust parser");

    let tree = parser
        .parse(rust_code)
        .expect("Failed to parse Rust code");

    // Query for struct declarations
    let query = "(struct_item name: (type_identifier) @struct)";
    let results = execute_query(SupportedLanguage::Rust, &tree, rust_code, query)
        .expect("Failed to execute query");

    assert!(!results.is_empty(), "Should find struct declarations");
}

#[test]
fn test_query_match_position_information() {
    let java_code = "public class Test { }";

    let mut parser = LanguageParser::new(SupportedLanguage::Java)
        .expect("Failed to create Java parser");

    let tree = parser
        .parse(java_code)
        .expect("Failed to parse Java code");

    // Query for class names
    let query = "(class_declaration name: (identifier) @class)";
    let results = execute_query(SupportedLanguage::Java, &tree, java_code, query)
        .expect("Failed to execute query");

    if !results.is_empty() {
        let result = &results[0];
        // Verify position information is present
        assert!(result.start_point.0 < 100); // reasonable row
        assert!(result.start_point.1 < 100); // reasonable column
        assert!(result.start_byte < java_code.len());
        assert!(result.end_byte <= java_code.len());
    }
}

#[test]
fn test_multiple_classes_java() {
    let java_code = r#"
public class First { }
public class Second { }
public class Third { }
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Java)
        .expect("Failed to create Java parser");

    let tree = parser
        .parse(java_code)
        .expect("Failed to parse Java code");

    // Query for all class declarations
    let query = "(class_declaration name: (identifier) @class)";
    let results = execute_query(SupportedLanguage::Java, &tree, java_code, query)
        .expect("Failed to execute query");

    assert!(!results.is_empty(), "Should find class declarations");
}

#[test]
fn test_rust_trait_query() {
    let rust_code = r#"
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
"#;

    let mut parser = LanguageParser::new(SupportedLanguage::Rust)
        .expect("Failed to create Rust parser");

    let tree = parser
        .parse(rust_code)
        .expect("Failed to parse Rust code");

    // Query for trait declarations
    let query = "(trait_item name: (type_identifier) @trait)";
    let results = execute_query(SupportedLanguage::Rust, &tree, rust_code, query)
        .expect("Failed to execute query");

    assert!(!results.is_empty(), "Should find trait declarations");
}
