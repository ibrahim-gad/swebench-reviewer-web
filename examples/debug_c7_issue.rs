// Debug script to reproduce the C7 issue

// Import the actual test detection module
use swe_reviewer_web::api::test_detection;

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
    
    // Test with a more complex diff that matches what C7 actually processes
    let complex_diff = r#"
diff --git a/src/file.rs b/src/file.rs
index 1234567..abcdefg 100644
--- a/src/file.rs
+++ b/src/file.rs
@@ -10,6 +10,12 @@ impl FileProcessor {
     }
 }
 
+#[test]
+fn test_file_removal() {
+    let processor = FileProcessor::new();
+    assert!(processor.remove_file("test.txt"));
+}
+
 impl Drop for FileProcessor {
     fn drop(&mut self) {
         self.cleanup();
"#;
    
    println!("\n--- Testing with complex diff ---");
    let result_complex = test_detection::contains_exact_test_name(complex_diff, test_name, language);
    println!("Complex diff result: {}", result_complex);
    
    // Test both the old pattern (what C7 used to do) and new pattern
    println!("\n--- Testing how C7 actually calls the function ---");
    
    // This is how C7 currently calls it (the problematic call)
    let f2p_test = "tests::test_file_removal";  // This is what F2P test names look like
    let test_name_to_search = if f2p_test.contains("::") {
        f2p_test.split("::").last().unwrap_or(f2p_test)
    } else {
        f2p_test
    };
    
    println!("F2P test name: '{}'", f2p_test);
    println!("Extracted for search: '{}'", test_name_to_search);
    
    // This is the CURRENT call in C7 (which is wrong)
    let current_c7_result = test_detection::contains_exact_test_name(complex_diff, f2p_test, language);
    println!("Current C7 call result (passing full f2p_test): {}", current_c7_result);
    
    // This is what C7 SHOULD call  
    let correct_c7_result = test_detection::contains_exact_test_name(complex_diff, test_name_to_search, language);
    println!("Correct C7 call result (passing extracted name): {}", correct_c7_result);
}
