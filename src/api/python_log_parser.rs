use regex::Regex;
use std::collections::HashSet;
use std::fs;
use lazy_static::lazy_static;

use super::log_parser::{LogParserTrait, ParsedLog};

// Test status enum matching Python test framework constants
#[derive(Debug, Clone, PartialEq)]
enum TestStatus {
    Passed,
    Failed,
    Error,
    Skipped,
}

impl TestStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Passed => "PASSED",
            TestStatus::Failed => "FAILED", 
            TestStatus::Error => "ERROR",
            TestStatus::Skipped => "SKIPPED",
        }
    }
}

// Compile regex patterns once at module level to avoid repeated compilation
lazy_static! {
    // PyTest patterns - now includes XFAIL support with better handling
    static ref PYTEST_STATUS_RE: Regex = Regex::new(r"^(PASSED|FAILED|ERROR|SKIPPED|XFAIL)\s+(.+?)(?:\s+-\s+.*)?$")
        .expect("Failed to compile PYTEST_STATUS_RE regex");
    
    // Enhanced pattern for pytest status lines with better parametrized test support and percentage handling
    static ref PYTEST_ENHANCED_STATUS_RE: Regex = Regex::new(r"^(\d*)(PASSED|FAILED|ERROR|SKIPPED|XFAIL)\s+(.+?)(?:\s+\[\s*\d+%\s*\])?(?:\s+-\s+.*)?$")
        .expect("Failed to compile PYTEST_ENHANCED_STATUS_RE regex");
    
    // Pattern for parametrized tests with complex parameters - more flexible
    static ref PYTEST_PARAMETRIZED_RE: Regex = Regex::new(r"(.+?)(?:\[([^\]]*)\])?\s+(PASSED|FAILED|ERROR|SKIPPED|XFAIL)")  
        .expect("Failed to compile PYTEST_PARAMETRIZED_RE regex");
    
    // New pattern for tests with status and percentage in between test name and status
    static ref PYTEST_STATUS_WITH_PERCENTAGE_RE: Regex = Regex::new(r"^(.+?)\s+(PASSED|FAILED|ERROR|SKIPPED|XFAIL)\s+\[\s*\d+%\s*\](?:\s+-\s+.*)?$")
        .expect("Failed to compile PYTEST_STATUS_WITH_PERCENTAGE_RE regex");
    
    // Pattern specifically for XFAIL tests with reason
    static ref PYTEST_XFAIL_RE: Regex = Regex::new(r"^(\d*)XFAIL\s+(.+?)(?:\s+-\s+(.*))?$")
        .expect("Failed to compile PYTEST_XFAIL_RE regex");
    
    static ref PYTEST_OPTIONS_RE: Regex = Regex::new(r"(.*?)\[(.*)\]")
        .expect("Failed to compile PYTEST_OPTIONS_RE regex");
    
    // Django patterns
    static ref DJANGO_OK_RE: Regex = Regex::new(r"^(.+?)\s+\.\.\.\s+(ok|OK)$")
        .expect("Failed to compile DJANGO_OK_RE regex");
    
    static ref DJANGO_SKIPPED_RE: Regex = Regex::new(r"^(.+?)\s+\.\.\.\s+skipped")
        .expect("Failed to compile DJANGO_SKIPPED_RE regex");
    
    static ref DJANGO_FAIL_RE: Regex = Regex::new(r"^(.+?)\s+\.\.\.\s+FAIL$")
        .expect("Failed to compile DJANGO_FAIL_RE regex");
    
    static ref DJANGO_ERROR_RE: Regex = Regex::new(r"^(.+?)\s+\.\.\.\s+ERROR$")
        .expect("Failed to compile DJANGO_ERROR_RE regex");
    
    static ref DJANGO_FAIL_PREFIX_RE: Regex = Regex::new(r"^FAIL:\s+(.+)")
        .expect("Failed to compile DJANGO_FAIL_PREFIX_RE regex");
    
    static ref DJANGO_ERROR_PREFIX_RE: Regex = Regex::new(r"^ERROR:\s+(.+)")
        .expect("Failed to compile DJANGO_ERROR_PREFIX_RE regex");
    
    // Django special patterns for multiline issues
    static ref DJANGO_MULTILINE_1_RE: Regex = Regex::new(r"^(.*?)\s\.\.\.\sTesting\ against\ Django\ installed\ in\ ((?s:.*?))\ silenced\)\.\nok$")
        .expect("Failed to compile DJANGO_MULTILINE_1_RE regex");
    
    static ref DJANGO_MULTILINE_2_RE: Regex = Regex::new(r"^(.*?)\s\.\.\.\sInternal\ Server\ Error:\ \/(.*)\/\nok$")
        .expect("Failed to compile DJANGO_MULTILINE_2_RE regex");
    
    static ref DJANGO_MULTILINE_3_RE: Regex = Regex::new(r"^(.*?)\s\.\.\.\sSystem check identified no issues \(0 silenced\)\nok$")
        .expect("Failed to compile DJANGO_MULTILINE_3_RE regex");
    
    // Seaborn patterns
    static ref SEABORN_FAILED_RE: Regex = Regex::new(r"^FAILED\s+(.+)")
        .expect("Failed to compile SEABORN_FAILED_RE regex");
    
    static ref SEABORN_PASSED_RE: Regex = Regex::new(r"^(.+)\s+PASSED$")
        .expect("Failed to compile SEABORN_PASSED_RE regex");
    
    static ref SEABORN_PASSED_PREFIX_RE: Regex = Regex::new(r"^PASSED\s+(.+)")
        .expect("Failed to compile SEABORN_PASSED_PREFIX_RE regex");
    
    // Sympy patterns
    static ref SYMPY_ERROR_RE: Regex = Regex::new(r"(_+)\s*([^/\s]*(?:/[^/\s]*)*)\.py:([^/\s]+)\s*(_+)")
        .expect("Failed to compile SYMPY_ERROR_RE regex");
    
    static ref SYMPY_TEST_STATUS_RE: Regex = Regex::new(r"^(test_\w+)\s+(E|F|ok)$")
        .expect("Failed to compile SYMPY_TEST_STATUS_RE regex");
    
    // PyTest v2 patterns (with ANSI escape codes)
    static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\[(\d+)m")
        .expect("Failed to compile ANSI_ESCAPE_RE regex");
    
    // Matplotlib patterns (similar to pytest but with mouse button replacements)
    static ref MATPLOTLIB_MOUSE_BUTTON_RE: Regex = Regex::new(r"MouseButton\.(LEFT|RIGHT)")
        .expect("Failed to compile MATPLOTLIB_MOUSE_BUTTON_RE regex");
}

pub struct PythonLogParser;

impl PythonLogParser {
    pub fn new() -> Self {
        Self
    }
    
    fn detect_framework(&self, content: &str) -> String {
        // Check for framework-specific indicators
        if content.contains("Django") || content.contains("django") {
            return "django".to_string();
        }
        if content.contains("seaborn") {
            return "seaborn".to_string();
        }
        if content.contains("sympy") {
            return "sympy".to_string();
        }
        if content.contains("matplotlib") {
            return "matplotlib".to_string();
        }
        
        // Check for pytest indicators
        if content.contains("pytest") || content.contains("PASSED") || content.contains("FAILED") || content.contains("XFAIL") {
            // Check if it has XFAIL or complex parametrized tests (enhanced format)
            if content.contains("XFAIL") || (content.contains("[") && content.contains("%]")) {
                return "pytest_enhanced".to_string();
            }
            // Check if it's pytest v2 format (with ANSI codes)
            if ANSI_ESCAPE_RE.is_match(content) {
                return "pytest_v2".to_string();
            }
            // Check if it has options format
            if content.contains("[") && content.contains("]") {
                return "pytest_options".to_string();
            }
            return "pytest".to_string();
        }
        
        // Default to pytest_v2 for best compatibility
        "pytest_v2".to_string()
    }
}

impl LogParserTrait for PythonLogParser {
    fn get_language(&self) -> &'static str {
        "python"
    }

    fn parse_log_file(&self, file_path: &str) -> Result<ParsedLog, String> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read log file {}: {}", file_path, e))?;

        let framework = self.detect_framework(&content);
        
        match framework.as_str() {
            "django" => Ok(parse_log_django(&content)),
            "seaborn" => Ok(parse_log_seaborn(&content)),
            "sympy" => Ok(parse_log_sympy(&content)),
            "matplotlib" => Ok(parse_log_matplotlib(&content)),
            "pytest_enhanced" => Ok(parse_log_pytest_enhanced(&content)),
            "pytest_options" => Ok(parse_log_pytest_options(&content)),
            "pytest_v2" => Ok(parse_log_pytest_v2(&content)),
            _ => Ok(parse_log_pytest(&content)),
        }
    }
}

fn parse_log_pytest(log: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();

    for line in log.lines() {
        let line = line.trim();
        
        // Check if line starts with any test status
        if line.starts_with("PASSED") || line.starts_with("FAILED") || 
           line.starts_with("ERROR") || line.starts_with("SKIPPED") || line.starts_with("XFAIL") {
            
            if let Some(captures) = PYTEST_STATUS_RE.captures(line) {
                let status = captures.get(1).unwrap().as_str();
                let mut test_case = captures.get(2).unwrap().as_str().to_string();
                
                // Additional parsing for FAILED status (remove error message)
                if status == "FAILED" && test_case.contains(" - ") {
                    if let Some(pos) = test_case.rfind(" - ") {
                        test_case = test_case[..pos].to_string();
                    }
                }
                
                match status {
                    "PASSED" => { passed.insert(test_case); }
                    "FAILED" | "ERROR" => { failed.insert(test_case); }
                    "SKIPPED" | "XFAIL" => { ignored.insert(test_case); }
                    _ => {}
                }
            }
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn parse_log_pytest_options(log: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();

    for line in log.lines() {
        let line = line.trim();
        
        if line.starts_with("PASSED") || line.starts_with("FAILED") || 
           line.starts_with("ERROR") || line.starts_with("SKIPPED") || line.starts_with("XFAIL") {
            
            if let Some(captures) = PYTEST_STATUS_RE.captures(line) {
                let status = captures.get(1).unwrap().as_str();
                let mut test_case = captures.get(2).unwrap().as_str().to_string();
                
                // Additional parsing for FAILED status
                if status == "FAILED" {
                    test_case = test_case.replace(" - ", " ");
                }
                
                // Handle options pattern
                let test_name = if let Some(option_match) = PYTEST_OPTIONS_RE.captures(&test_case) {
                    let main = option_match.get(1).unwrap().as_str();
                    let option = option_match.get(2).unwrap().as_str();
                    
                    // Special handling for path options
                    let processed_option = if option.starts_with('/') && !option.starts_with("//") && !option.contains('*') {
                        if let Some(last_part) = option.split('/').last() {
                            format!("/{}", last_part)
                        } else {
                            option.to_string()
                        }
                    } else {
                        option.to_string()
                    };
                    
                    format!("{}[{}]", main, processed_option)
                } else {
                    test_case
                };
                
                match status {
                    "PASSED" => { passed.insert(test_name); }
                    "FAILED" | "ERROR" => { failed.insert(test_name); }
                    "SKIPPED" | "XFAIL" => { ignored.insert(test_name); }
                    _ => {}
                }
            }
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn parse_log_django(log: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();
    let mut prev_test: Option<String> = None;

    let lines: Vec<&str> = log.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line = line.trim();

        // Special case for Django version check
        if line.contains("--version is equivalent to version") {
            passed.insert("--version is equivalent to version".to_string());
            continue;
        }

        // Track potential test for error case
        if line.contains(" ... ") {
            if let Some(test_name) = line.split(" ... ").next() {
                prev_test = Some(test_name.to_string());
            }
        }

        // Check for various pass patterns
        let pass_suffixes = [" ... ok", " ... OK", " ...  OK"];
        let mut found_pass = false;
        for suffix in &pass_suffixes {
            if line.ends_with(suffix) {
                // Special handling for django__django-7188 case
                let test_line = if line.trim().starts_with("Applying sites.0002_alter_domain_unique...test_no_migrations") {
                    if let Some(test_part) = line.split("...").nth(1) {
                        test_part.trim().strip_suffix(suffix).unwrap_or("").to_string()
                    } else {
                        continue;
                    }
                } else {
                    line.strip_suffix(suffix).unwrap_or("").to_string()
                };
                
                if !test_line.is_empty() {
                    passed.insert(test_line);
                    found_pass = true;
                    break;
                }
            }
        }
        
        if found_pass {
            continue;
        }

        // Check for skipped tests
        if line.contains(" ... skipped") {
            if let Some(test_name) = line.split(" ... skipped").next() {
                ignored.insert(test_name.to_string());
            }
            continue;
        }

        // Check for failed tests
        if line.ends_with(" ... FAIL") {
            if let Some(test_name) = line.split(" ... FAIL").next() {
                failed.insert(test_name.to_string());
            }
            continue;
        }

        // Check for error tests
        if line.ends_with(" ... ERROR") {
            if let Some(test_name) = line.split(" ... ERROR").next() {
                failed.insert(test_name.to_string()); // Treating ERROR as failed
            }
            continue;
        }

        // Check for FAIL: prefix
        if let Some(captures) = DJANGO_FAIL_PREFIX_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().trim().to_string();
            failed.insert(test_name);
            continue;
        }

        // Check for ERROR: prefix
        if let Some(captures) = DJANGO_ERROR_PREFIX_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().trim().to_string();
            failed.insert(test_name);
            continue;
        }

        // Handle case where test passed but with additional output
        if line.trim().starts_with("ok") && prev_test.is_some() {
            passed.insert(prev_test.clone().unwrap());
            prev_test = None;
        }
    }

    // Handle multiline Django patterns
    let multiline_patterns = [
        &*DJANGO_MULTILINE_1_RE,
        &*DJANGO_MULTILINE_2_RE, 
        &*DJANGO_MULTILINE_3_RE,
    ];
    
    for pattern in &multiline_patterns {
        for captures in pattern.captures_iter(log) {
            if let Some(test_name_match) = captures.get(1) {
                passed.insert(test_name_match.as_str().to_string());
            }
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn parse_log_seaborn(log: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let ignored = HashSet::new(); // Seaborn doesn't seem to have skipped tests

    for line in log.lines() {
        let line = line.trim();

        // Check for FAILED prefix
        if let Some(captures) = SEABORN_FAILED_RE.captures(line) {
            let test_case = captures.get(1).unwrap().as_str().to_string();
            failed.insert(test_case);
            continue;
        }

        // Check for "test_name PASSED" format
        if let Some(captures) = SEABORN_PASSED_RE.captures(line) {
            let test_case = captures.get(1).unwrap().as_str().to_string();
            passed.insert(test_case);
            continue;
        }

        // Check for PASSED prefix
        if let Some(captures) = SEABORN_PASSED_PREFIX_RE.captures(line) {
            let test_case = captures.get(1).unwrap().as_str().to_string();
            passed.insert(test_case);
            continue;
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn parse_log_sympy(log: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new(); // Sympy uses ERROR instead of ignored

    // Find failed tests using the pattern: "(_*) file.py:test_name (_*)"
    for captures in SYMPY_ERROR_RE.captures_iter(log) {
        if let (Some(file_match), Some(test_match)) = (captures.get(2), captures.get(3)) {
            let test_case = format!("{}.py:{}", file_match.as_str(), test_match.as_str());
            failed.insert(test_case);
        }
    }

    // Parse individual test results
    for line in log.lines() {
        let line = line.trim();
        
        if line.starts_with("test_") {
            if let Some(captures) = SYMPY_TEST_STATUS_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().to_string();
                let status = captures.get(2).unwrap().as_str();
                
                match status {
                    "E" => { ignored.insert(test_name); } // ERROR treated as ignored
                    "F" => { failed.insert(test_name); }
                    "ok" => { passed.insert(test_name); }
                    _ => {}
                }
            }
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn parse_log_matplotlib(log: &str) -> ParsedLog {
    // Matplotlib uses pytest format but with mouse button replacements
    
    // Replace mouse button constants first
    let processed_log = log
        .replace("MouseButton.LEFT", "1")
        .replace("MouseButton.RIGHT", "3");
    
    // Then use standard pytest parsing
    let mut result = parse_log_pytest(&processed_log);
    
    // Additional processing for matplotlib-specific patterns if needed
    for line in processed_log.lines() {
        let line = line.trim();
        
        if line.starts_with("PASSED") || line.starts_with("FAILED") || 
           line.starts_with("ERROR") || line.starts_with("SKIPPED") {
            
            if let Some(captures) = PYTEST_STATUS_RE.captures(line) {
                let status = captures.get(1).unwrap().as_str();
                let test_case = captures.get(2).unwrap().as_str().to_string();
                
                // Additional parsing for FAILED status
                let test_name = if status == "FAILED" {
                    test_case.replace(" - ", " ")
                } else {
                    test_case
                };
                
                match status {
                    "PASSED" => { result.passed.insert(test_name.clone()); result.all.insert(test_name); }
                    "FAILED" | "ERROR" => { result.failed.insert(test_name.clone()); result.all.insert(test_name); }
                    "SKIPPED" => { result.ignored.insert(test_name.clone()); result.all.insert(test_name); }
                    _ => {}
                }
            }
        }
    }
    
    result
}

fn parse_log_pytest_v2(log: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();

    // Remove ANSI escape codes and control characters
    let clean_log = clean_ansi_escapes(log);

    for line in clean_log.lines() {
        let line = line.trim();
        
        // Try the new pattern for tests with status and percentage first
        // Example: "tests/test_character_labeling.py::test_get_cypher_labels_various_types[  Item -:Item:Entity] PASSED [ 12%]"
        if let Some(captures) = PYTEST_STATUS_WITH_PERCENTAGE_RE.captures(line) {
            let test_case = captures.get(1).unwrap().as_str().to_string();
            let status = captures.get(2).unwrap().as_str();
            
            match status {
                "PASSED" => { passed.insert(test_case); }
                "FAILED" | "ERROR" => { failed.insert(test_case); }
                "SKIPPED" | "XFAIL" => { ignored.insert(test_case); }
                _ => {}
            }
            continue;
        }
        
        // Try XFAIL specific pattern
        if let Some(captures) = PYTEST_XFAIL_RE.captures(line) {
            let test_case = captures.get(2).unwrap().as_str().to_string();
            ignored.insert(test_case);
            continue;
        }
        
        // Check if line starts with test status
        if line.starts_with("PASSED") || line.starts_with("FAILED") || 
           line.starts_with("ERROR") || line.starts_with("SKIPPED") || line.starts_with("XFAIL") {
            
            if let Some(captures) = PYTEST_STATUS_RE.captures(line) {
                let status = captures.get(1).unwrap().as_str();
                let mut test_case = captures.get(2).unwrap().as_str().to_string();
                
                // Additional parsing for FAILED status
                if status == "FAILED" && test_case.contains(" - ") {
                    if let Some(pos) = test_case.rfind(" - ") {
                        test_case = test_case[..pos].to_string();
                    }
                }
                
                // For v2, we might have multiple words as test case name
                match status {
                    "PASSED" => { passed.insert(test_case); }
                    "FAILED" | "ERROR" => { failed.insert(test_case); }
                    "SKIPPED" | "XFAIL" => { ignored.insert(test_case); }
                    _ => {}
                }
            }
        }
        // Support older pytest versions by checking if the line ends with the test status
        else if line.ends_with("PASSED") || line.ends_with("FAILED") || 
                line.ends_with("ERROR") || line.ends_with("SKIPPED") || line.ends_with("XFAIL") {
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let test_name = parts[0].to_string();
                let status = parts[parts.len() - 1];
                
                match status {
                    "PASSED" => { passed.insert(test_name); }
                    "FAILED" | "ERROR" => { failed.insert(test_name); }
                    "SKIPPED" | "XFAIL" => { ignored.insert(test_name); }
                    _ => {}
                }
            }
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn parse_log_pytest_enhanced(log: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();

    // Remove ANSI escape codes and control characters
    let clean_log = clean_ansi_escapes(log);

    for line in clean_log.lines() {
        let line = line.trim();
        
        // Try enhanced pattern first (handles line numbers, percentages, etc.)
        if let Some(captures) = PYTEST_ENHANCED_STATUS_RE.captures(line) {
            let _line_number = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let status = captures.get(2).unwrap().as_str();
            let test_case = captures.get(3).unwrap().as_str().to_string();
            
            match status {
                "PASSED" => { passed.insert(test_case); }
                "FAILED" | "ERROR" => { failed.insert(test_case); }
                "SKIPPED" | "XFAIL" => { ignored.insert(test_case); }
                _ => {}
            }
            continue;
        }
        
        // Try parametrized pattern (handles complex parameters)
        if let Some(captures) = PYTEST_PARAMETRIZED_RE.captures(line) {
            let test_base = captures.get(1).unwrap().as_str();
            let params = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            let status = captures.get(3).unwrap().as_str();
            
            let test_case = if params.is_empty() {
                test_base.to_string()
            } else {
                format!("{}[{}]", test_base, params)
            };
            
            match status {
                "PASSED" => { passed.insert(test_case); }
                "FAILED" | "ERROR" => { failed.insert(test_case); }
                "SKIPPED" | "XFAIL" => { ignored.insert(test_case); }
                _ => {}
            }
            continue;
        }
        
        // Fallback to standard pytest pattern
        if line.starts_with("PASSED") || line.starts_with("FAILED") || 
           line.starts_with("ERROR") || line.starts_with("SKIPPED") || line.starts_with("XFAIL") {
            
            if let Some(captures) = PYTEST_STATUS_RE.captures(line) {
                let status = captures.get(1).unwrap().as_str();
                let mut test_case = captures.get(2).unwrap().as_str().to_string();
                
                // Additional parsing for FAILED status
                if status == "FAILED" && test_case.contains(" - ") {
                    if let Some(pos) = test_case.rfind(" - ") {
                        test_case = test_case[..pos].to_string();
                    }
                }
                
                match status {
                    "PASSED" => { passed.insert(test_case); }
                    "FAILED" | "ERROR" => { failed.insert(test_case); }
                    "SKIPPED" | "XFAIL" => { ignored.insert(test_case); }
                    _ => {}
                }
            }
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn clean_ansi_escapes(text: &str) -> String {
    // Remove ANSI escape codes
    let without_ansi = ANSI_ESCAPE_RE.replace_all(text, "");
    
    // Remove other control characters (ASCII 1-31 except newline and tab)
    let mut result = String::new();
    for ch in without_ansi.chars() {
        let code = ch as u32;
        if code >= 32 || ch == '\n' || ch == '\t' {
            result.push(ch);
        }
    }
    
    result
}

// Factory function to get parser by repository name (for compatibility with Python version)
pub fn get_py_parser_by_repo_name(repo_name: &str) -> fn(&str) -> ParsedLog {
    match repo_name {
        "astropy/astropy" => parse_log_pytest_v2,
        "django/django" => parse_log_django,
        "marshmallow-code/marshmallow" => parse_log_pytest,
        "matplotlib/matplotlib" => parse_log_matplotlib,
        "mwaskom/seaborn" => parse_log_seaborn,
        "pallets/flask" => parse_log_pytest,
        "psf/requests" => parse_log_pytest_options,
        "pvlib/pvlib-python" => parse_log_pytest,
        "pydata/xarray" => parse_log_pytest,
        "pydicom/pydicom" => parse_log_pytest_options,
        "pylint-dev/astroid" => parse_log_pytest,
        "pylint-dev/pylint" => parse_log_pytest_options,
        "pytest-dev/pytest" => parse_log_pytest,
        "pyvista/pyvista" => parse_log_pytest,
        "scikit-learn/scikit-learn" => parse_log_pytest_v2,
        "sqlfluff/sqlfluff" => parse_log_pytest,
        "sphinx-doc/sphinx" => parse_log_pytest_v2,
        "sympy/sympy" => parse_log_sympy,
        "redis/redis-py" => parse_log_pytest,
        "ipython/ipython" => parse_log_pytest,
        "Textualize/rich" => parse_log_pytest,
        "python-pillow/Pillow" => parse_log_pytest,
        "dask/dask" => parse_log_pytest_v2,
        "pandas-dev/pandas" => parse_log_pytest,
        "celery/celery" => parse_log_pytest,
        "reflex-dev/reflex" => parse_log_pytest,
        "getmoto/moto" => parse_log_pytest_v2,
        "saltstack/salt" => parse_log_pytest,
        "Pyomo/pyomo" => parse_log_pytest_v2,
        _ => parse_log_pytest_v2, // Default to pytest_v2 for best compatibility
    }
}

// Factory function to get parser by framework name
pub fn get_py_parser_by_name(name: &str) -> fn(&str) -> ParsedLog {
    match name {
        "pytest" => parse_log_pytest_v2,
        "django" => parse_log_django,
        "seaborn" => parse_log_seaborn,
        "sympy" => parse_log_sympy,
        "matplotlib" => parse_log_matplotlib,
        _ => parse_log_pytest_v2, // Default to pytest_v2
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_pytest() {
        let log_content = r#"
PASSED test_basic_functionality
FAILED test_advanced_feature - AssertionError: Expected 5 but got 3
SKIPPED test_slow_operation
ERROR test_network_connection
"#;

        let result = parse_log_pytest(log_content);
        
        assert!(result.passed.contains("test_basic_functionality"));
        assert!(result.failed.contains("test_advanced_feature"));
        assert!(result.ignored.contains("test_slow_operation"));
        assert!(result.failed.contains("test_network_connection"));
        assert_eq!(result.passed.len(), 1);
        assert_eq!(result.failed.len(), 2);
        assert_eq!(result.ignored.len(), 1);
        assert_eq!(result.all.len(), 4);
    }

    #[test]
    fn test_parse_log_pytest_options() {
        let log_content = r#"
PASSED test_module.py::test_basic[option1]
FAILED test_module.py::test_basic[/path/to/file.py] - Error message
PASSED test_other.py::test_case[//keep_this_path]
"#;

        let result = parse_log_pytest_options(log_content);
        
        assert!(result.passed.contains("test_module.py::test_basic[option1]"));
        assert!(result.failed.contains("test_module.py::test_basic[/file.py]"));
        assert!(result.passed.contains("test_other.py::test_case[//keep_this_path]"));
    }

    #[test]
    fn test_parse_log_django() {
        let log_content = r#"
test_basic_view ... ok
test_model_creation ... FAIL
test_database_migration ... ERROR
test_user_authentication ... skipped
test_form_validation ... OK
FAIL: test_model_creation
ERROR: test_database_migration
--version is equivalent to version
"#;

        let result = parse_log_django(log_content);
        
        assert!(result.passed.contains("test_basic_view"));
        assert!(result.passed.contains("test_form_validation"));
        assert!(result.passed.contains("--version is equivalent to version"));
        assert!(result.failed.contains("test_model_creation"));
        assert!(result.failed.contains("test_database_migration"));
        assert!(result.ignored.contains("test_user_authentication"));
    }

    #[test]
    fn test_parse_log_seaborn() {
        let log_content = r#"
FAILED test_plotting_function
test_data_processing PASSED
PASSED test_statistical_method
"#;

        let result = parse_log_seaborn(log_content);
        
        assert!(result.failed.contains("test_plotting_function"));
        assert!(result.passed.contains("test_data_processing"));
        assert!(result.passed.contains("test_statistical_method"));
    }

    #[test]
    fn test_parse_log_sympy() {
        let log_content = r#"
test_basic_algebra ok
test_calculus_derivative F
test_linear_algebra E
_______ sympy/tests/test_basic.py:test_advanced_case _______
"#;

        let result = parse_log_sympy(log_content);
        
        assert!(result.passed.contains("test_basic_algebra"));
        assert!(result.failed.contains("test_calculus_derivative"));
        assert!(result.ignored.contains("test_linear_algebra"));
        assert!(result.failed.contains("sympy/tests/test_basic.py:test_advanced_case"));
    }

    #[test]
    fn test_parse_log_matplotlib() {
        let log_content = r#"
PASSED test_plot_with_MouseButton.LEFT
FAILED test_interaction_MouseButton.RIGHT - Some error
"#;

        let result = parse_log_matplotlib(log_content);
        
        assert!(result.passed.contains("test_plot_with_1"));
        assert!(result.failed.contains("test_interaction_3"));
    }

    #[test]
    fn test_parse_log_pytest_v2_with_ansi() {
        let log_content = r#"
[32mPASSED[0m test_with_ansi_colors
[31mFAILED[0m test_with_ansi_fail - Error occurred
test_old_format PASSED
"#;

        let result = parse_log_pytest_v2(log_content);
        
        assert!(result.passed.contains("test_with_ansi_colors"));
        assert!(result.failed.contains("test_with_ansi_fail"));
        assert!(result.passed.contains("test_old_format"));
    }

    #[test]
    fn test_parse_log_pytest_v2_edge_cases() {
        let log_content = r#"
XFAIL tests/test_initial_setup_logic.py::test_valid_json_output_from_llm - generate_world_building_logic interface updated
tests/test_character_labeling.py::test_get_cypher_labels_various_types[  Item -:Item:Entity] PASSED [ 12%]
PASSED tests/test_simple.py::test_basic
"#;

        let result = parse_log_pytest_v2(log_content);
        
        // Check XFAIL test is properly categorized as ignored
        assert!(result.ignored.contains("tests/test_initial_setup_logic.py::test_valid_json_output_from_llm"));
        // Check complex parametrized test with percentage
        assert!(result.passed.contains("tests/test_character_labeling.py::test_get_cypher_labels_various_types[  Item -:Item:Entity]"));
        // Check simple passed test
        assert!(result.passed.contains("tests/test_simple.py::test_basic"));
    }

    #[test]
    fn test_clean_ansi_escapes() {
        let input = "[32mPASSED[0m test_name";
        let expected = "PASSED test_name";
        let result = clean_ansi_escapes(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_python_parser_framework_detection() {
        let parser = PythonLogParser::new();
        
        // Test Django detection
        let django_log = "Django test framework output";
        assert_eq!(parser.detect_framework(django_log), "django");
        
        // Test pytest detection
        let pytest_log = "PASSED test_something";
        assert_eq!(parser.detect_framework(pytest_log), "pytest");
        
        // Test pytest v2 detection (with ANSI)
        let pytest_v2_log = "[32mPASSED[0m test_something";
        assert_eq!(parser.detect_framework(pytest_v2_log), "pytest_v2");
        
        // Test pytest options detection
        let pytest_options_log = "PASSED test_something[option]";
        assert_eq!(parser.detect_framework(pytest_options_log), "pytest_options");
    }

    #[test]
    fn test_get_py_parser_by_repo_name() {
        // Test specific repo mappings
        let django_parser = get_py_parser_by_repo_name("django/django");
        let matplotlib_parser = get_py_parser_by_repo_name("matplotlib/matplotlib");
        let sympy_parser = get_py_parser_by_repo_name("sympy/sympy");
        
        // Test that they return different function pointers (addresses)
        assert_ne!(django_parser as *const (), matplotlib_parser as *const ());
        assert_ne!(matplotlib_parser as *const (), sympy_parser as *const ());
        
        // Test unknown repo defaults to pytest_v2
        let unknown_parser = get_py_parser_by_repo_name("unknown/repo");
        let pytest_v2_parser = get_py_parser_by_name("pytest");
        assert_eq!(unknown_parser as *const (), pytest_v2_parser as *const ());
    }

    #[test]
    fn test_get_py_parser_by_name() {
        let pytest_parser = get_py_parser_by_name("pytest");
        let django_parser = get_py_parser_by_name("django");
        let unknown_parser = get_py_parser_by_name("unknown");
        
        // Test that different names return different parsers
        assert_ne!(pytest_parser as *const (), django_parser as *const ());
        
        // Test that unknown defaults to pytest_v2
        assert_eq!(unknown_parser as *const (), pytest_parser as *const ());
    }

    #[test]
    fn test_django_multiline_patterns() {
        let log_content = r#"
test_basic ... Testing against Django installed in /path/to/django (git commit hash) silenced).
ok
test_server_error ... Internal Server Error: /admin/
ok
test_no_issues ... System check identified no issues (0 silenced)
ok
"#;

        let result = parse_log_django(log_content);
        
        // These should be parsed by the multiline regex patterns
        assert!(result.passed.len() >= 3);
    }

    #[test]
    fn test_complex_pytest_test_names() {
        let log_content = r#"
PASSED tests/test_module.py::TestClass::test_method
FAILED tests/test_deep/nested/test_file.py::test_complex_scenario[param1-param2]
SKIPPED tests/test_conditional.py::test_skip_on_condition
"#;

        let result = parse_log_pytest(log_content);
        
        assert!(result.passed.contains("tests/test_module.py::TestClass::test_method"));
        assert!(result.failed.contains("tests/test_deep/nested/test_file.py::test_complex_scenario[param1-param2]"));
        assert!(result.ignored.contains("tests/test_conditional.py::test_skip_on_condition"));
    }


}
