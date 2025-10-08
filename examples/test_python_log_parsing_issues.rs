// Test to reproduce the Python log parsing issues

use swe_reviewer_web::api::python_log_parser::PythonLogParser;
use swe_reviewer_web::api::log_parser::LogParserTrait;
use std::fs;
use tempfile::NamedTempFile;
use std::io::Write;

fn main() {
    println!("🔍 Testing Python Log Parser Issues");
    println!("===================================");

    let parser = PythonLogParser::new();

    // Test case 1: XFAIL tests (currently not handled)
    let xfail_log = r#"
XFAIL tests/test_initial_setup_logic.py::test_valid_json_output_from_llm - generate_world_building_logic interface updated
XFAIL tests/test_initial_setup_logic.py::test_invalid_json_output_decode_error - generate_world_building_logic interface updated
XFAIL tests/test_initial_setup_logic.py::test_llm_provides_string_for_expected_list - generate_world_building_logic interface updated
PASSED tests/test_basic.py::test_simple
FAILED tests/test_other.py::test_broken
"#;

    // Test case 2: Complex parametrized tests with special characters
    let parametrized_log = r#"
tests/test_character_labeling.py::test_get_cypher_labels_various_types[  Item -:Item:Entity] PASSED [ 12%]
tests/test_character_labeling.py::test_get_cypher_labels_various_types[User with spaces] PASSED [ 24%]
tests/test_character_labeling.py::test_get_cypher_labels_various_types[Complex[nested]Test] FAILED [ 36%]
"#;

    println!("1. Testing XFAIL log parsing:");
    test_log_content(&parser, xfail_log, "xfail");

    println!("\n2. Testing parametrized log parsing:");
    test_log_content(&parser, parametrized_log, "parametrized");

    println!("\n3. Testing the exact examples from user:");
    let user_examples = r#"
XFAIL tests/test_initial_setup_logic.py::test_valid_json_output_from_llm - generate_world_building_logic interface updated
563XFAIL tests/test_initial_setup_logic.py::test_invalid_json_output_decode_error - generate_world_building_logic interface updated
564XFAIL tests/test_initial_setup_logic.py::test_llm_provides_string_for_expected_list - generate_world_building_logic interface updated

tests/test_character_labeling.py::test_get_cypher_labels_various_types[  Item -:Item:Entity] PASSED [ 12%] -marked as failed
"#;
    
    test_log_content(&parser, user_examples, "user_examples");
}

fn test_log_content(parser: &PythonLogParser, log_content: &str, test_name: &str) {
    // Create a temporary file with the log content
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(log_content.as_bytes()).expect("Failed to write to temp file");
    let temp_path = temp_file.path().to_str().unwrap();

    // Parse the log
    match parser.parse_log_file(temp_path) {
        Ok(parsed) => {
            println!("  ✅ {} log parsed successfully:", test_name);
            println!("    - Passed: {} tests", parsed.passed.len());
            for test in &parsed.passed {
                println!("      📗 {}", test);
            }
            println!("    - Failed: {} tests", parsed.failed.len());
            for test in &parsed.failed {
                println!("      📕 {}", test);
            }
            println!("    - Ignored: {} tests", parsed.ignored.len());
            for test in &parsed.ignored {
                println!("      📙 {}", test);
            }
            println!("    - Total: {} tests", parsed.all.len());
            
            // Check for missing XFAIL tests
            if test_name == "xfail" || test_name == "user_examples" {
                let expected_xfail_tests = [
                    "tests/test_initial_setup_logic.py::test_valid_json_output_from_llm",
                    "tests/test_initial_setup_logic.py::test_invalid_json_output_decode_error", 
                    "tests/test_initial_setup_logic.py::test_llm_provides_string_for_expected_list"
                ];
                
                for expected in &expected_xfail_tests {
                    let found_in_any = parsed.passed.contains(*expected) || 
                                     parsed.failed.contains(*expected) || 
                                     parsed.ignored.contains(*expected);
                    if !found_in_any {
                        println!("    ❌ MISSING: {} (XFAIL test not detected)", expected);
                    }
                }
            }
            
            // Check for complex parametrized tests
            if test_name == "parametrized" {
                let expected_param_tests = [
                    "tests/test_character_labeling.py::test_get_cypher_labels_various_types[  Item -:Item:Entity]",
                    "tests/test_character_labeling.py::test_get_cypher_labels_various_types[User with spaces]",
                    "tests/test_character_labeling.py::test_get_cypher_labels_various_types[Complex[nested]Test]"
                ];
                
                for expected in &expected_param_tests {
                    let found_in_any = parsed.passed.contains(*expected) || 
                                     parsed.failed.contains(*expected) || 
                                     parsed.ignored.contains(*expected);
                    if !found_in_any {
                        println!("    ❌ MISSING: {} (parametrized test not detected)", expected);
                    }
                }
            }
        },
        Err(e) => {
            println!("  ❌ {} log parsing failed: {}", test_name, e);
        }
    }
}
