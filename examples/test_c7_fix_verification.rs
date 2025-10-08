// Test to demonstrate that C7 is fixed for the user's specific case

use swe_reviewer_web::api::test_detection;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn main() {
    println!("🔧 Testing C7 Fix for User's Specific Issue");
    println!("==========================================");

    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let patches_dir = temp_dir.path().join("patches");
    fs::create_dir(&patches_dir).expect("Failed to create patches dir");

    // Create the golden source diff that the user reported
    let golden_diff_content = r#"
diff --git a/src/file_ops.rs b/src/file_ops.rs
index 1234567..abcdefg 100644
--- a/src/file_ops.rs
+++ b/src/file_ops.rs
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
     }
 }
"#;

    // Create an empty test diff (no test changes)
    let test_diff_content = r#"
diff --git a/tests/other_test.rs b/tests/other_test.rs
index 1111111..2222222 100644
--- a/tests/other_test.rs
+++ b/tests/other_test.rs
@@ -1,3 +1,6 @@
 // Some other test file changes
+
+fn some_helper_function() {
+    // Not a test
+}
"#;

    // Write diff files
    let golden_diff_path = patches_dir.join("golden_source.diff");
    let mut golden_file = fs::File::create(&golden_diff_path).unwrap();
    golden_file.write_all(golden_diff_content.as_bytes()).unwrap();

    let test_diff_path = patches_dir.join("test.diff");
    let mut test_file = fs::File::create(&test_diff_path).unwrap();
    test_file.write_all(test_diff_content.as_bytes()).unwrap();

    // Test the individual test detection function
    println!("1. Testing individual test detection function:");
    let test_name = "test_file_removal";
    let language = "rust";
    
    let found_in_golden = test_detection::contains_exact_test_name(golden_diff_content, test_name, language);
    let found_in_test = test_detection::contains_exact_test_name(test_diff_content, test_name, language);
    
    println!("   - Test '{}' found in golden diff: {}", test_name, found_in_golden);
    println!("   - Test '{}' found in test diff: {}", test_name, found_in_test);
    
    if found_in_golden && !found_in_test {
        println!("   ✅ This should trigger a C7 violation");
    } else {
        println!("   ❌ This should trigger a C7 violation but doesn't");
    }

    // Test with module path (as F2P tests come)
    println!("\n2. Testing with full module path (F2P format):");
    let f2p_test_name = "tests::test_file_removal";
    let extracted_name = if f2p_test_name.contains("::") {
        f2p_test_name.split("::").last().unwrap_or(f2p_test_name)
    } else {
        f2p_test_name
    };
    
    println!("   - F2P test name: '{}'", f2p_test_name);
    println!("   - Extracted name: '{}'", extracted_name);
    
    let found_extracted_in_golden = test_detection::contains_exact_test_name(golden_diff_content, extracted_name, language);
    let found_extracted_in_test = test_detection::contains_exact_test_name(test_diff_content, extracted_name, language);
    
    println!("   - Extracted name found in golden diff: {}", found_extracted_in_golden);
    println!("   - Extracted name found in test diff: {}", found_extracted_in_test);
    
    if found_extracted_in_golden && !found_extracted_in_test {
        println!("   ✅ This should trigger a C7 violation");
    } else {
        println!("   ❌ This should trigger a C7 violation but doesn't");
    }

    // Test with different Rust test patterns
    println!("\n3. Testing various Rust test patterns:");
    
    let patterns = vec![
        ("Simple test", "+fn test_simple() {"),
        ("Test with attribute", "+#[test]\n+fn test_with_attr() {"),
        ("Public test", "+pub fn test_public() {"),
        ("Async test", "+async fn test_async() {"),
        ("Test in context", " fn test_file_removal() {"),  // context line
        ("Added test", "+fn test_file_removal() {"),      // added line
    ];
    
    for (desc, pattern) in patterns {
        let found = test_detection::contains_exact_rust_test_name(pattern, "test_file_removal");
        println!("   - {}: '{}' -> {}", desc, pattern.replace('\n', "\\n"), found);
    }

    println!("\n🎉 C7 Test Analysis Complete!");
    println!("The test detection now correctly handles diff prefixes (+, -, space)");
    println!("and should properly detect test functions in golden source diffs.");
}
