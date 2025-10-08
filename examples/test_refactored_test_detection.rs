// Example demonstrating the refactored test detection system
// This example shows how language-specific test detection is now properly organized

use swe_reviewer_web::api::test_detection;

fn main() {
    println!("🔍 Testing the Refactored Test Detection System");
    println!("================================================");
    
    // Example Rust diff content
    let rust_diff = r#"
#[test]
fn test_rust_functionality() {
    assert_eq!(2 + 2, 4);
}

pub fn test_another_rust_function() {
    // Some implementation
}
"#;

    // Example Python diff content
    let python_diff = r#"
def test_python_functionality():
    assert 2 + 2 == 4

class TestPythonClass:
    def test_class_method(self):
        assert True
        
@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_function(value):
    assert value > 0
"#;

    println!("\n🦀 Testing Rust test detection:");
    println!("- test_rust_functionality: {}", 
        test_detection::contains_exact_test_name(rust_diff, "test_rust_functionality", "rust"));
    println!("- test_another_rust_function: {}", 
        test_detection::contains_exact_test_name(rust_diff, "test_another_rust_function", "rust"));
    println!("- test_nonexistent: {}", 
        test_detection::contains_exact_test_name(rust_diff, "test_nonexistent", "rust"));
    
    println!("\n🐍 Testing Python test detection:");
    println!("- test_python_functionality: {}", 
        test_detection::contains_exact_test_name(python_diff, "test_python_functionality", "python"));
    println!("- test_class_method: {}", 
        test_detection::contains_exact_test_name(python_diff, "test_class_method", "python"));
    println!("- TestPythonClass::test_class_method: {}", 
        test_detection::contains_exact_test_name(python_diff, "TestPythonClass::test_class_method", "python"));
    println!("- test_parametrized_function: {}", 
        test_detection::contains_exact_test_name(python_diff, "test_parametrized_function", "python"));
    println!("- test_parametrized_function[1]: {}", 
        test_detection::contains_exact_test_name(python_diff, "test_parametrized_function[1]", "python"));
    println!("- test_nonexistent: {}", 
        test_detection::contains_exact_test_name(python_diff, "test_nonexistent", "python"));
    
    println!("\n🔄 Testing language auto-detection:");
    println!("- Rust with 'rust' language: {}", 
        test_detection::contains_exact_test_name(rust_diff, "test_rust_functionality", "rust"));
    println!("- Python with 'python' language: {}", 
        test_detection::contains_exact_test_name(python_diff, "test_python_functionality", "python"));
    println!("- Unknown language (defaults to Rust): {}", 
        test_detection::contains_exact_test_name(rust_diff, "test_rust_functionality", "unknown"));
    
    println!("\n✅ Refactoring Benefits:");
    println!("1. 🗂️  Language-specific functions are now organized in dedicated test_detection module");
    println!("2. 🔄 Single entry point with automatic language dispatch");
    println!("3. 🧪 Comprehensive test coverage in the test_detection module");
    println!("4. 🚀 Easy to extend for new languages in the future");
    println!("5. 📝 Clean separation of concerns - log parsing vs test detection");
    
    println!("\n🎉 Test detection system is now properly organized and ready for future extensions!");
}
