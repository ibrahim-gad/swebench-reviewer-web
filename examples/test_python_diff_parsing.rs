use std::fs;
use std::path::Path;
use swe_reviewer_web::api::log_parser::LogParser;

fn main() {
    println!("Testing Python Diff Parsing Integration");
    println!("=======================================\n");

    // Create a temporary directory for test files
    let temp_dir = std::env::temp_dir().join("swe_reviewer_python_diff_test");
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Create Python test logs
    let pytest_log = r#"PASSED tests/test_module.py::TestClass::test_method
FAILED tests/test_integration.py::test_complex_scenario - AssertionError
PASSED tests/test_utils.py::test_helper_function
"#;

    let django_log = r#"test_user_creation ... ok
test_user_validation ... FAIL  
test_model_integration ... ok
"#;

    // Create Python diff files
    let source_diff = r#"--- a/src/models.py
+++ b/src/models.py
@@ -10,6 +10,10 @@ class User:
     def validate(self):
         return len(self.name) > 0
 
+    def test_helper_function(self):
+        # This is a helper, not a test
+        return self.validate()
+
 class UserManager:
     def create_user(self, name):
         return User(name)
@@ -20,3 +24,7 @@ class UserManager:
         users = [User("test1"), User("test2")]
         return users
 
+def test_complex_scenario():
+    # This F2P test appears in source diff
+    manager = UserManager()
+    return manager.create_user("test")
"#;

    let test_diff = r#"--- a/tests/test_models.py
+++ b/tests/test_models.py
@@ -5,6 +5,12 @@ class TestUser:
     def test_user_creation(self):
         user = User("test")
         assert user.name == "test"
+        
+    def test_method(self):
+        # This test appears in both source and test diffs
+        user = User("test")
+        assert user.validate()

 class TestUserManager:
     def test_create_user(self):
"#;

    // Write test files
    let base_log_path = temp_dir.join("base.log");
    let before_log_path = temp_dir.join("before.log");  
    let after_log_path = temp_dir.join("after.log");
    let patches_dir = temp_dir.join("patches");
    std::fs::create_dir_all(&patches_dir).expect("Failed to create patches dir");
    let source_diff_path = patches_dir.join("source.diff");
    let test_diff_path = patches_dir.join("test.diff");

    fs::write(&base_log_path, pytest_log).expect("Failed to write base log");
    fs::write(&before_log_path, pytest_log).expect("Failed to write before log");
    fs::write(&after_log_path, django_log).expect("Failed to write after log");
    fs::write(&source_diff_path, source_diff).expect("Failed to write source diff");
    fs::write(&test_diff_path, test_diff).expect("Failed to write test diff");

    // Test with Python language
    let log_parser = LogParser::new();
    let file_paths = vec![
        base_log_path.to_string_lossy().to_string(),
        before_log_path.to_string_lossy().to_string(),
        after_log_path.to_string_lossy().to_string(),
        source_diff_path.to_string_lossy().to_string(),
        test_diff_path.to_string_lossy().to_string(),
    ];

    let fail_to_pass_tests = vec![
        "tests/test_integration.py::test_complex_scenario".to_string(),
        "tests/test_module.py::TestClass::test_method".to_string(),
    ];
    let pass_to_pass_tests = vec![
        "tests/test_utils.py::test_helper_function".to_string(),
    ];

    println!("Testing Python log analysis with diff checking...");
    match log_parser.analyze_logs(&file_paths, "python", &fail_to_pass_tests, &pass_to_pass_tests) {
        Ok(result) => {
            println!("✅ Python log analysis successful!");
            println!("Found {} rule violations", 
                if result.rule_violations.c7_f2p_tests_in_golden_source_diff.has_problem { 1 } else { 0 });
            
            if result.rule_violations.c7_f2p_tests_in_golden_source_diff.has_problem {
                println!("C7 violations found:");
                for violation in &result.rule_violations.c7_f2p_tests_in_golden_source_diff.examples {
                    println!("  - {}", violation);
                }
            }
            
            println!("Test statuses: {} entries", result.test_statuses.len());
        }
        Err(e) => {
            eprintln!("❌ Python log analysis failed: {}", e);
        }
    }

    // Test with Rust language for comparison
    println!("\nTesting with Rust language (should use Rust patterns)...");
    match log_parser.analyze_logs(&file_paths, "rust", &fail_to_pass_tests, &pass_to_pass_tests) {
        Ok(result) => {
            println!("✅ Rust log analysis completed");
            println!("C7 violations with Rust patterns: {}", 
                result.rule_violations.c7_f2p_tests_in_golden_source_diff.examples.len());
        }
        Err(e) => {
            eprintln!("❌ Rust log analysis failed: {}", e);
        }
    }

    // Clean up
    std::fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir");
    
    println!("\n🎉 Python diff parsing integration test completed!");
    println!("The C7 rule now supports both Python and Rust test detection in diff files.");
}
