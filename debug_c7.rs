// Debug script to reproduce the C7 issue

// Import the actual test detection module
use swebench_reviewer_web::api::test_detection;



fn main() {
    // Test case provided by user
    let diff_content = r#"
+fn test_file_removal() {
+    // Test implementation
+}
"#;
    
    let test_name = "test_file_removal";
    let language = "rust";
    
    println!("Testing with diff content:");
    println!("{}", diff_content);
    println!("Looking for test: '{}'", test_name);
    println!("Language: '{}'", language);
    
    // Test the new unified function
    let result_new = test_detection::contains_exact_test_name(diff_content, test_name, language);
    println!("New unified function result: {}", result_new);
    
    // Test the direct rust function
    let result_rust = test_detection::contains_exact_rust_test_name(diff_content, test_name);
    println!("Direct rust function result: {}", result_rust);
    
    // Let's also test what happens when we extract from module path
    let full_test_name = "tests::test_file_removal";
    let extracted_name = if full_test_name.contains("::") {
        full_test_name.split("::").last().unwrap_or(full_test_name)
    } else {
        full_test_name
    };
    
    println!("Full test name: '{}'", full_test_name);
    println!("Extracted name: '{}'", extracted_name);
    
    let result_extracted = test_detection::contains_exact_test_name(diff_content, extracted_name, language);
    println!("Result with extracted name: {}", result_extracted);
}
