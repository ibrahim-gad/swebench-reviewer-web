//! Language-specific test detection functions for diff parsing
//! 
//! This module provides functions to detect test names in diff content across different
//! programming languages. Each language has its own test detection logic that understands
//! the specific syntax and patterns used in that language.

use regex::Regex;
use lazy_static::lazy_static;

/// Detect if a Rust test name exists in diff content
/// 
/// This function performs precise matching for Rust test patterns including:
/// - Function declarations with #[test] attributes
/// - Module paths (test_module::test_function)
/// - Various function signature formats (pub, async, unsafe, generics)
/// 
/// # Arguments
/// * `diff_content` - The diff content to search in
/// * `test_name` - The test name to search for (may include module paths)
/// 
/// # Returns
/// * `true` if the test name is found in the diff content, `false` otherwise
pub fn contains_exact_rust_test_name(diff_content: &str, test_name: &str) -> bool {
    lazy_static! {
        // Enhanced regex to match various function declaration formats in diff content
        // Accounts for diff prefixes: +, -, or space at line start
        static ref FUNCTION_DECLARATION_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:<[^>]*>)?\s*\("
        ).unwrap();
        
        // Regex to match test attribute annotations in diff content
        static ref TEST_ATTRIBUTE_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*#\[test\]\s*(?:\n[+\-\s]\s*)*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*"
        ).unwrap();
        
        // Regex to match mod declarations (for module path matching) in diff content
        static ref MOD_DECLARATION_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*[{;]"
        ).unwrap();
    }
    
    // Split the test name to handle module paths like "test_module::test_function"
    let test_parts: Vec<&str> = test_name.split("::").collect();
    let function_name = test_parts.last().unwrap_or(&test_name);
    
    // 1. Check for exact function name matches in function declarations
    for caps in FUNCTION_DECLARATION_RE.captures_iter(diff_content) {
        if let Some(found_fn_name) = caps.get(1) {
            if found_fn_name.as_str() == *function_name {
                return true;
            }
        }
    }
    
    // 2. Check for test attribute with matching function name
    for caps in TEST_ATTRIBUTE_RE.captures_iter(diff_content) {
        if let Some(found_fn_name) = caps.get(1) {
            if found_fn_name.as_str() == *function_name {
                return true;
            }
        }
    }
    
    // 3. For module paths, check if the full path exists
    if test_name.contains("::") {
        // Try to match the full module path in diff content
        let escaped_test_name = regex::escape(test_name);
        let full_path_regex = Regex::new(&format!(
            r"(?m)^[+\-\s]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+{}\s*(?:<[^>]*>)?\s*\(",
            escaped_test_name
        )).unwrap_or_else(|_| Regex::new(r"$^").unwrap()); // fallback to never-matching regex
        
        if full_path_regex.is_match(diff_content) {
            return true;
        }
        
        // Also check for the module path structure
        let module_parts = &test_parts[..test_parts.len().saturating_sub(1)];
        if !module_parts.is_empty() {
            let mut found_all_modules = true;
            for module in module_parts {
                let module_regex = Regex::new(&format!(
                    r"(?m)^[+\-\s]\s*(?:pub\s+)?mod\s+{}\s*[{{;]",
                    regex::escape(module)
                )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
                
                if !module_regex.is_match(diff_content) {
                    found_all_modules = false;
                    break;
                }
            }
            
            // If we found all modules and the function, it's a match
            if found_all_modules {
                let function_in_module_regex = Regex::new(&format!(
                    r"(?m)^[+\-\s]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+{}\s*(?:<[^>]*>)?\s*\(",
                    regex::escape(function_name)
                )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
                
                if function_in_module_regex.is_match(diff_content) {
                    return true;
                }
            }
        }
    }
    
    // 4. Additional check for word boundaries to ensure exact matches
    // This prevents partial matches like "test_foo" matching "test_foobar"
    let word_boundary_regex = Regex::new(&format!(
        r"\b{}\b",
        regex::escape(function_name)
    )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
    
    // Only consider it a match if it appears in a function context
    let lines: Vec<&str> = diff_content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if word_boundary_regex.is_match(line) {
            // Check if this line or nearby lines contain function declaration keywords
            let context_start = i.saturating_sub(2);
            let context_end = (i + 3).min(lines.len());
            let context = lines[context_start..context_end].join("\n");
            
            if context.contains("fn ") || context.contains("#[test]") || context.contains("mod ") {
                // Verify it's actually a function declaration, not just a reference
                let fn_declaration_regex = Regex::new(&format!(
                    r"(?m)^[+\-\s]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+{}\s*(?:<[^>]*>)?\s*\(",
                    regex::escape(function_name)
                )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
                
                if fn_declaration_regex.is_match(&context) {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Detect if a Python test name exists in diff content
/// 
/// This function performs precise matching for Python test patterns including:
/// - Function declarations (def test_name())
/// - Class-based tests (TestClass::test_method)
/// - pytest parametrized tests with brackets
/// - Module paths and file paths
/// 
/// # Arguments
/// * `diff_content` - The diff content to search in
/// * `test_name` - The test name to search for (may include class/file paths)
/// 
/// # Returns
/// * `true` if the test name is found in the diff content, `false` otherwise
pub fn contains_exact_python_test_name(diff_content: &str, test_name: &str) -> bool {
    lazy_static! {
        // Python function declaration patterns in diff content
        // Accounts for diff prefixes: +, -, or space at line start
        static ref PY_FUNCTION_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\("
        ).unwrap();
        
        // Python class declaration patterns in diff content
        static ref PY_CLASS_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\("
        ).unwrap();
        
        // Python class method patterns in diff content
        static ref PY_METHOD_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*def\s+(test_[a-zA-Z0-9_]*)\s*\(self"
        ).unwrap();
        
        // Python module-level test function in diff content
        static ref PY_TEST_FUNCTION_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*def\s+(test_[a-zA-Z0-9_]*)\s*\("
        ).unwrap();
        
        // Python parametrized test patterns (pytest)
        static ref PY_PARAMETRIZE_RE: Regex = Regex::new(
            r"@pytest\.mark\.parametrize"
        ).unwrap();
        
        // Python unittest patterns
        static ref PY_UNITTEST_RE: Regex = Regex::new(
            r"unittest\.TestCase"
        ).unwrap();
    }
    
    // Handle different Python test name formats
    let clean_test_name = if test_name.contains("::") {
        // pytest format: "test_file.py::TestClass::test_method" -> "test_method"
        test_name.split("::").last().unwrap_or(test_name)
    } else if test_name.contains(".py:") {
        // Format: "test_file.py:test_method" -> "test_method"
        test_name.split(':').last().unwrap_or(test_name)
    } else if test_name.contains("/") {
        // Path format: "tests/test_file.py::test_method" -> "test_method"
        if let Some(after_double_colon) = test_name.split("::").last() {
            after_double_colon
        } else {
            test_name.split('/').last().unwrap_or(test_name)
        }
    } else {
        test_name
    }.trim();
    
    // Remove any parametrization brackets: "test_name[param]" -> "test_name"
    let base_test_name = if let Some(bracket_pos) = clean_test_name.find('[') {
        &clean_test_name[..bracket_pos]
    } else {
        clean_test_name
    };
    
    // 1. Check for exact function name matches in Python function declarations
    for caps in PY_FUNCTION_RE.captures_iter(diff_content) {
        if let Some(found_fn_name) = caps.get(1) {
            if found_fn_name.as_str() == base_test_name {
                return true;
            }
        }
    }
    
    // 2. Check for test functions specifically (starting with "test_")
    for caps in PY_TEST_FUNCTION_RE.captures_iter(diff_content) {
        if let Some(found_fn_name) = caps.get(1) {
            if found_fn_name.as_str() == base_test_name {
                return true;
            }
        }
    }
    
    // 3. Check for class methods (test methods in test classes)
    for caps in PY_METHOD_RE.captures_iter(diff_content) {
        if let Some(found_method_name) = caps.get(1) {
            if found_method_name.as_str() == base_test_name {
                return true;
            }
        }
    }
    
    // 4. Handle pytest class-based tests: "TestClass::test_method"
    if test_name.contains("::") {
        let parts: Vec<&str> = test_name.split("::").collect();
        if parts.len() >= 2 {
            let class_name = parts[parts.len() - 2];
            let method_name = parts[parts.len() - 1];
            
            // Remove parametrization from method name
            let clean_method_name = if let Some(bracket_pos) = method_name.find('[') {
                &method_name[..bracket_pos]
            } else {
                method_name
            };
            
            // Look for class definition
            let class_regex = Regex::new(&format!(
                r"(?m)^\s*class\s+{}\s*\(",
                regex::escape(class_name)
            )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
            
            if class_regex.is_match(diff_content) {
                // Look for method within the class context
                let method_regex = Regex::new(&format!(
                    r"(?m)^\s*def\s+{}\s*\(self",
                    regex::escape(clean_method_name)
                )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
                
                if method_regex.is_match(diff_content) {
                    return true;
                }
            }
        }
    }
    
    // 5. Word boundary check for additional context
    let word_boundary_regex = Regex::new(&format!(
        r"\b{}\b",
        regex::escape(base_test_name)
    )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
    
    let lines: Vec<&str> = diff_content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if word_boundary_regex.is_match(line) {
            // Check if this line or nearby lines contain Python function/class declaration keywords
            let context_start = i.saturating_sub(2);
            let context_end = (i + 3).min(lines.len());
            let context = lines[context_start..context_end].join("\n");
            
            // Look for Python-specific patterns
            if context.contains("def ") || context.contains("class ") || 
               context.contains("@pytest.mark") || context.contains("unittest") {
                
                // Verify it's actually a function/method declaration
                let py_declaration_regex = Regex::new(&format!(
                    r"(?m)^\s*def\s+{}\s*\(",
                    regex::escape(base_test_name)
                )).unwrap_or_else(|_| Regex::new(r"$^").unwrap());
                
                if py_declaration_regex.is_match(&context) {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Detect if a JavaScript/TypeScript test name exists in diff content
/// 
/// This function performs precise matching for JS/TS test patterns including:
/// - Jest test functions (test(), it(), describe())
/// - Mocha test functions (it(), describe())
/// - Vitest test functions (test(), it(), describe())
/// - Test suite hierarchies (describe blocks)
/// - Various function declaration formats
/// 
/// # Arguments
/// * `diff_content` - The diff content to search in
/// * `test_name` - The test name to search for (may include suite paths)
/// 
/// # Returns
/// * `true` if the test name is found in the diff content, `false` otherwise
pub fn contains_exact_js_test_name(diff_content: &str, test_name: &str) -> bool {
    lazy_static! {
        // Regex to match test function calls like test(), it(), describe()
        static ref TEST_FUNCTION_RE: Regex = Regex::new(
            r#"(?m)^[+\-\s]\s*(?:test|it|describe)\s*\(\s*['"`]([^'"`]+)['"`]"#
        ).unwrap();
        
        // Regex to match arrow function test declarations
        static ref ARROW_TEST_RE: Regex = Regex::new(
            r#"(?m)^[+\-\s]\s*(?:const|let|var)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*\(\)\s*=>"#
        ).unwrap();
        
        // Regex to match function declarations for tests
        static ref FUNCTION_TEST_RE: Regex = Regex::new(
            r"(?m)^[+\-\s]\s*(?:function\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(|([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*\(\)\s*=>)"
        ).unwrap();
        
        // Regex to match exported test functions
        static ref EXPORT_TEST_RE: Regex = Regex::new(
            r#"(?m)^[+\-\s]\s*export\s+(?:const|let|function)\s+([a-zA-Z_][a-zA-Z0-9_]*)"#
        ).unwrap();
    }
    
    // Handle hierarchical test names like "Suite - Test" or "Suite.Test"
    let test_parts: Vec<&str> = test_name.split(&['-', '.'][..])
        .map(|s| s.trim())
        .collect();
    let final_test_name = test_parts.last().unwrap_or(&test_name);
    
    // 1. Check for test function calls (test(), it(), describe())
    for caps in TEST_FUNCTION_RE.captures_iter(diff_content) {
        if let Some(found_test_name) = caps.get(1) {
            let found_name = found_test_name.as_str();
            // Check if the found name matches any part of the hierarchical test name
            if test_parts.iter().any(|part| found_name == *part) || found_name == test_name {
                return true;
            }
        }
    }
    
    // 2. Check for arrow function declarations
    for caps in ARROW_TEST_RE.captures_iter(diff_content) {
        if let Some(found_fn_name) = caps.get(1) {
            if found_fn_name.as_str() == *final_test_name {
                return true;
            }
        }
    }
    
    // 3. Check for function declarations
    for caps in FUNCTION_TEST_RE.captures_iter(diff_content) {
        if let Some(found_fn_name) = caps.get(1).or_else(|| caps.get(2)) {
            if found_fn_name.as_str() == *final_test_name {
                return true;
            }
        }
    }
    
    // 4. Check for exported functions
    for caps in EXPORT_TEST_RE.captures_iter(diff_content) {
        if let Some(found_fn_name) = caps.get(1) {
            if found_fn_name.as_str() == *final_test_name {
                return true;
            }
        }
    }
    
    // 5. Simple string matching as fallback
    diff_content.contains(final_test_name)
}

/// Detect the JavaScript/TypeScript testing framework from project files
/// 
/// # Arguments
/// * `project_path` - Path to the project root
/// 
/// # Returns
/// * The detected framework name or "vitest" as default
pub fn detect_js_testing_framework(project_path: &str) -> String {
    use std::path::Path;
    
    let package_json_path = Path::new(project_path).join("package.json");
    
    if let Ok(content) = std::fs::read_to_string(&package_json_path) {
        // Parse package.json to detect testing frameworks
        if content.contains("\"cypress\"") || content.contains("cypress") {
            return "cypress".to_string();
        }
        if content.contains("\"playwright\"") || content.contains("@playwright/test") {
            return "playwright".to_string();
        }
        if content.contains("\"jest\"") || content.contains("@jest/") {
            return "jest".to_string();
        }
        if content.contains("\"jasmine\"") || content.contains("jasmine") {
            return "jasmine".to_string();
        }
        if content.contains("\"qunit\"") || content.contains("qunit") {
            return "qunit".to_string();
        }
        if content.contains("\"ava\"") || content.contains("ava") {
            return "ava".to_string();
        }
        if content.contains("\"mocha\"") || content.contains("mocha") {
            return "mocha".to_string();
        }
        if content.contains("\"vitest\"") || content.contains("vitest") {
            return "vitest".to_string();
        }
        if content.contains("\"karma\"") || content.contains("karma") {
            return "karma".to_string();
        }
        if content.contains("\"tap\"") || content.contains("node-tap") {
            return "tap".to_string();
        }
    }
    
    // Check for config files
    let config_files = [
        ("cypress.config.js", "cypress"),
        ("cypress.config.ts", "cypress"),
        ("cypress.json", "cypress"),
        ("playwright.config.js", "playwright"),
        ("playwright.config.ts", "playwright"),
        ("jest.config.js", "jest"),
        ("jest.config.ts", "jest"),
        ("jest.config.json", "jest"),
        ("jasmine.json", "jasmine"),
        ("spec/support/jasmine.json", "jasmine"),
        ("ava.config.js", "ava"),
        ("ava.config.cjs", "ava"),
        ("ava.config.mjs", "ava"),
        ("vitest.config.js", "vitest"),
        ("vitest.config.ts", "vitest"),
        ("mocha.opts", "mocha"),
        (".mocharc.json", "mocha"),
        ("karma.conf.js", "karma"),
        ("karma.conf.ts", "karma"),
    ];
    
    for (file_name, framework) in &config_files {
        let config_path = Path::new(project_path).join(file_name);
        if config_path.exists() {
            return framework.to_string();
        }
    }
    
    // Default fallback
    "vitest".to_string()
}

/// Main entry point for language-specific test detection
/// 
/// This function dispatches to the appropriate language-specific test detection
/// function based on the provided language.
/// 
/// # Arguments
/// * `diff_content` - The diff content to search in
/// * `test_name` - The test name to search for
/// * `language` - The programming language ("rust", "python", etc.)
/// 
/// # Returns
/// * `true` if the test name is found in the diff content, `false` otherwise
pub fn contains_exact_test_name(diff_content: &str, test_name: &str, language: &str) -> bool {
    match language.to_lowercase().as_str() {
        "python" => contains_exact_python_test_name(diff_content, test_name),
        "rust" => contains_exact_rust_test_name(diff_content, test_name),
        "javascript" | "typescript" => contains_exact_js_test_name(diff_content, test_name),
        _ => {
            // Default to Rust behavior for unknown languages
            contains_exact_rust_test_name(diff_content, test_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_test_detection() {
        let diff_content = r#"
#[test]
fn test_basic_functionality() {
    assert_eq!(2 + 2, 4);
}

pub fn test_advanced_feature() -> Result<(), Error> {
    // Some test code
    Ok(())
}

mod tests {
    #[test]
    fn test_module_function() {
        // Test implementation
    }
}
"#;

        assert!(contains_exact_rust_test_name(diff_content, "test_basic_functionality"));
        assert!(contains_exact_rust_test_name(diff_content, "test_advanced_feature"));
        assert!(contains_exact_rust_test_name(diff_content, "test_module_function"));
        assert!(!contains_exact_rust_test_name(diff_content, "test_nonexistent"));
    }

    #[test]
    fn test_python_test_detection() {
        let diff_content = r#"
def test_basic_functionality():
    assert 2 + 2 == 4

class TestUserModel:
    def test_user_creation(self):
        user = User()
        assert user is not None
    
    def test_user_validation(self):
        # Test implementation
        pass

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_function(value):
    assert value > 0
"#;

        assert!(contains_exact_python_test_name(diff_content, "test_basic_functionality"));
        assert!(contains_exact_python_test_name(diff_content, "test_user_creation"));
        assert!(contains_exact_python_test_name(diff_content, "test_user_validation"));
        assert!(contains_exact_python_test_name(diff_content, "test_parametrized_function"));
        assert!(!contains_exact_python_test_name(diff_content, "test_nonexistent"));
    }

    #[test]
    fn test_js_basic_test_detection() {
        let diff_content = r#"
test('basic functionality', () => {
    expect(2 + 2).toBe(4);
});

it('advanced feature', () => {
    // Some test code
});

describe('module tests', () => {
    test('module function', () => {
        // Test implementation
    });
});
"#;

        assert!(contains_exact_js_test_name(diff_content, "basic functionality"));
        assert!(contains_exact_js_test_name(diff_content, "advanced feature"));
        assert!(contains_exact_js_test_name(diff_content, "module function"));
        assert!(!contains_exact_js_test_name(diff_content, "nonexistent test"));
    }

    #[test]
    fn test_language_dispatcher() {
        let rust_diff = r#"
#[test]
fn test_rust_function() {
    assert!(true);
}
"#;

        let python_diff = r#"
def test_python_function():
    assert True
"#;

        let js_diff = r#"
test('test_js_function', () => {
    expect(true).toBe(true);
});
"#;

        // Test language-specific dispatch
        assert!(contains_exact_test_name(rust_diff, "test_rust_function", "rust"));
        assert!(contains_exact_test_name(python_diff, "test_python_function", "python"));
        assert!(contains_exact_test_name(js_diff, "test_js_function", "javascript"));
        
        // Test case insensitive language matching
        assert!(contains_exact_test_name(rust_diff, "test_rust_function", "RUST"));
        assert!(contains_exact_test_name(python_diff, "test_python_function", "Python"));
        assert!(contains_exact_test_name(js_diff, "test_js_function", "JavaScript"));
        
        // Test default behavior for unknown languages (falls back to Rust)
        assert!(contains_exact_test_name(rust_diff, "test_rust_function", "unknown"));
    }

    #[test]
    fn test_python_class_based_tests() {
        let diff_content = r#"
class TestUserModel:
    def test_user_creation(self):
        user = User()
        assert user is not None
    
    def test_user_validation(self):
        # Test implementation
        pass
"#;

        // Test pytest class::method format
        assert!(contains_exact_python_test_name(diff_content, "TestUserModel::test_user_creation"));
        assert!(contains_exact_python_test_name(diff_content, "TestUserModel::test_user_validation"));
        
        // Test just method names
        assert!(contains_exact_python_test_name(diff_content, "test_user_creation"));
        assert!(contains_exact_python_test_name(diff_content, "test_user_validation"));
        
        // Test non-existent methods
        assert!(!contains_exact_python_test_name(diff_content, "TestUserModel::test_nonexistent"));
    }

    #[test]
    fn test_js_framework_detection() {
        use std::fs;
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Test Jest detection
        let package_json_jest = r#"
        {
          "devDependencies": {
            "jest": "^29.0.0",
            "@types/jest": "^29.0.0"
          }
        }
        "#;
        fs::write(temp_path.join("package.json"), package_json_jest).unwrap();
        assert_eq!(detect_js_testing_framework(temp_path.to_str().unwrap()), "jest");
        
        // Clean up and test Vitest detection
        fs::remove_file(temp_path.join("package.json")).unwrap();
        fs::write(temp_path.join("vitest.config.ts"), "// vitest config").unwrap();
        assert_eq!(detect_js_testing_framework(temp_path.to_str().unwrap()), "vitest");
        
        // Test default fallback
        fs::remove_file(temp_path.join("vitest.config.ts")).unwrap();
        assert_eq!(detect_js_testing_framework(temp_path.to_str().unwrap()), "vitest");
    }
}
