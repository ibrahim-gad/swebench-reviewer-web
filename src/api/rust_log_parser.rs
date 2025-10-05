use regex::Regex;
use std::collections::HashSet;
use std::fs;
use lazy_static::lazy_static;

use super::log_parser::{LogParserTrait, ParsedLog};

// Compile regex patterns once at module level to avoid repeated compilation
lazy_static! {
    // Case-insensitive, include error status, allow trailing whitespace
    static ref TEST_LINE_RE: Regex = Regex::new(r"(?i)\btest\s+(.+?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$")
        .expect("Failed to compile TEST_LINE_RE regex");

    static ref TEST_START_RE: Regex = Regex::new(r"(?i)\btest\s+(.+?)\s+\.\.\.\s*(.*?)$")
        .expect("Failed to compile TEST_START_RE regex");

    static ref STATUS_RE: Regex = Regex::new(r"(?i)\b(ok|failed|ignored|error)\b")
        .expect("Failed to compile STATUS_RE regex");

    static ref STATUS_AT_END_RE: Regex = Regex::new(r"(?i)\b(ok|failed|ignored|error)\s*$")
        .expect("Failed to compile STATUS_AT_END_RE regex");

    // New pattern to match status at the beginning of lines mixed with logging output
    static ref STATUS_AT_START_RE: Regex = Regex::new(r"(?i)^(ok|FAILED|ignored|error)")
        .expect("Failed to compile STATUS_AT_START_RE regex");

    static ref ANOTHER_TEST_RE: Regex = Regex::new(r"(?i)\btest\s+[^\s]+\s+\.\.\.\s*")
        .expect("Failed to compile ANOTHER_TEST_RE regex");

    static ref TEST_WITH_O_RE: Regex = Regex::new(r"(?i)\btest\s+([^\s]+(?:::\w+)*)\s+\.\.\.\s*o\s*$")
        .expect("Failed to compile TEST_WITH_O_RE regex");

    static ref TEST_STARTS_RE: Regex = Regex::new(r"(?i)\btest\s+([^\s]+(?:::\w+)*)\s+\.\.\.\s*")
        .expect("Failed to compile TEST_STARTS_RE regex");

    static ref STATUS_IN_TEXT_RE: Regex = Regex::new(r"(?i)\b(ok|failed|ignored|error)\b")
        .expect("Failed to compile STATUS_IN_TEXT_RE regex");

    // Additional patterns
    static ref CORRUPTED_TEST_LINE_RE: Regex = Regex::new(r"(?i)(?:line)?test\s+([^\s]+(?:::\w+)*)\s+\.\.\.\s*")
        .expect("Failed to compile CORRUPTED_TEST_LINE_RE regex");

    // File boundary hints
    static ref FILE_BOUNDARY_RE_1: Regex = Regex::new(r"(?i)Running\s+([^\s]+(?:/[^\s]+)*\.(?:rs|fixed))\s*\(").unwrap();
    static ref FILE_BOUNDARY_RE_2: Regex = Regex::new(r"(?i)===\s*Running\s+(.+\.(?:rs|fixed))").unwrap();
    static ref FILE_BOUNDARY_RE_3: Regex = Regex::new(r"(?i)test\s+result:\s+ok\.\s+\d+\s+passed.*for\s+(.+\.(?:rs|fixed))").unwrap();

    // Enhanced extraction patterns
    static ref ENH_TEST_RE_1: Regex = Regex::new(r"(?i)\btest\s+([^\s]+(?:::[^\s]+)*)\s*\.{2,}\s*(ok|FAILED|ignored|error)").unwrap();
    static ref ENH_TEST_RE_2: Regex = Regex::new(r"(?i)test\s+([^\s]+)\s+\.\.\.\s+(ok|FAILED|ignored|error)").unwrap();
    
    // UI test format patterns - handles paths as test names with direct status
    static ref UI_TEST_PATH_RE: Regex = Regex::new(r"(?i)^([^\s]+(?:/[^\s]+)*\.(?:rs|fixed|toml|txt|md)(?:\s+\(revision\s+[^)]+\))?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$").unwrap();
    static ref UI_TEST_PATH_SIMPLE_RE: Regex = Regex::new(r"(?i)^([^\s]+(?:/[^\s]+)*\.(?:rs|fixed|toml|txt|md)(?:\s+\(revision\s+[^)]+\))?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$").unwrap();
    
    // Nextest format patterns - handles "PASS [duration] test_name" and "FAIL [duration] test_name"
    static ref NEXTEST_PASS_RE: Regex = Regex::new(r"(?i)^\s*PASS\s+\[[^\]]+\]\s+(.+)$").unwrap();
    static ref NEXTEST_FAIL_RE: Regex = Regex::new(r"(?i)^\s*FAIL\s+\[[^\]]+\]\s+(.+)$").unwrap();
    static ref NEXTEST_SKIP_RE: Regex = Regex::new(r"(?i)^\s*(SKIP|IGNORED)\s+\[[^\]]+\]\s+(.+)$").unwrap();
    
    // START pattern for nextest - captures test names from START lines
    static ref NEXTEST_START_RE: Regex = Regex::new(r"(?i)^\s*START\s+(.+)$").unwrap();

    // ANSI escape detection
    static ref ANSI_RE: Regex = Regex::new(r"\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])").unwrap();

    static ref FAILURES_BLOCK_RE: Regex = Regex::new(r"^\s{4}(.+?)\s*$")
        .expect("Failed to compile FAILURES_BLOCK_RE regex");

    // Additional patterns for single-line parsing to avoid repeated compilation
    static ref SINGLE_LINE_START_RE: Regex = Regex::new(r"(?i)test\s+([^\s]+(?:::[^\s]+)*)\s*\.{2,}").unwrap();
    static ref SINGLE_LINE_NEXT_TEST_RE: Regex = Regex::new(r"(?i)test\s+[^\s]+(?:::[^\s]+)*\s*\.{2,}").unwrap();
    static ref SINGLE_LINE_STATUS_AT_START_RE: Regex = Regex::new(r"(?i)^(ok|FAILED|ignored|error)").unwrap();
    static ref SIMPLE_PATTERN_RE: Regex = Regex::new(r"(?i)test\s+[^\s]+(?:::[^\s]+)*\s*\.{2,}\s*(ok|FAILED|ignored|error)").unwrap();
    
    // Pattern for tests that have diagnostic info after the "..." but before status
    static ref TEST_WITH_DIAGNOSTICS_RE: Regex = Regex::new(r"(?i)\btest\s+(.+?)\s+\.\.\.\s*(?:error:|$)").unwrap();
}

pub struct RustLogParser;

impl RustLogParser {
    pub fn new() -> Self {
        Self
    }
}

impl LogParserTrait for RustLogParser {
    fn get_language(&self) -> &'static str {
        "rust"
    }

    fn parse_log_file(&self, file_path: &str) -> Result<ParsedLog, String> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read log file {}: {}", file_path, e))?;

        // Check for nextest format first
        if looks_nextest_format(&content) {
            return Ok(parse_nextest_log(&content));
        }

        // Switch to ANSI/single-line parser when appropriate
        if looks_single_line_like(&content) {
            return Ok(parse_rust_log_single_line(&content));
        }

        // Use the full multi-line parser
        parse_rust_log_file(&content)
    }
}

fn looks_single_line_like(text: &str) -> bool {
    let line_count = text.lines().count();
    let has_ansi = ANSI_RE.is_match(text);
    let test_count = SIMPLE_PATTERN_RE.find_iter(text).count();
    
    // Count UI test patterns line-by-line since they use line anchors
    let mut ui_test_count = 0;
    for line in text.lines() {
        if UI_TEST_PATH_RE.is_match(line) || UI_TEST_PATH_SIMPLE_RE.is_match(line) {
            ui_test_count += 1;
        }
    }
    
    // Check if it looks like a UI test format (many path-based test results)
    let has_ui_tests = ui_test_count > 10;
    
    (line_count <= 3 && test_count > 5) || has_ansi || has_ui_tests
}

fn looks_nextest_format(text: &str) -> bool {
    // Check for nextest-specific patterns
    let nextest_indicators = [
        "Nextest run ID",
        "nextest run",
        "Starting tests across",
        "PASS [",
        "FAIL [",
        "START             ", // Added START pattern from your example
    ];
    
    let has_indicators = nextest_indicators.iter().any(|indicator| 
        text.to_lowercase().contains(&indicator.to_lowercase())
    );
    
    // Count nextest-style result lines
    let nextest_lines = NEXTEST_PASS_RE.find_iter(text).count() + 
                       NEXTEST_FAIL_RE.find_iter(text).count() + 
                       NEXTEST_SKIP_RE.find_iter(text).count();
    
    // Also check for the mixed format pattern with traditional + nextest
    let has_mixed_format = text.contains("PASS [") && text.contains("test ") && text.contains("... ok");
    
    has_indicators || nextest_lines > 5 || has_mixed_format
}

fn parse_nextest_log(text: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();

    let lines: Vec<&str> = text.lines().collect();

    // Parse nextest format using separate regex patterns for better accuracy
    for (_i, line) in lines.iter().enumerate() {
        // Parse PASS lines
        if let Some(captures) = NEXTEST_PASS_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().trim().to_string();
            passed.insert(test_name);
            continue;
        }
        
        // Parse FAIL lines
        if let Some(captures) = NEXTEST_FAIL_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().trim().to_string();
            failed.insert(test_name);
            continue;
        }
        
        // Parse SKIP/IGNORED lines
        if let Some(captures) = NEXTEST_SKIP_RE.captures(line) {
            let test_name = captures.get(2).unwrap().as_str().trim().to_string();
            ignored.insert(test_name);
            continue;
        }
        
        // Also handle traditional Rust test patterns for mixed format logs
        if let Some(captures) = TEST_LINE_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            let status = captures.get(2).unwrap().as_str().to_lowercase();
            
            match status.as_str() {
                "ok" => { passed.insert(test_name); }
                "failed" | "error" => { failed.insert(test_name); }
                "ignored" => { ignored.insert(test_name); }
                _ => {}
            }
            continue;
        }
        
        // Handle enhanced test patterns as well
        if let Some(captures) = ENH_TEST_RE_1.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            let mut status = captures.get(2).unwrap().as_str().to_lowercase();
            if status == "failed" || status == "error" {
                status = "failed".to_string();
            }
            match status.as_str() {
                "ok" => { passed.insert(test_name); }
                "failed" => { failed.insert(test_name); }
                "ignored" => { ignored.insert(test_name); }
                _ => {}
            }
            continue;
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn parse_rust_log_single_line(text: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();

    let clean = strip_ansi_color_codes(text);

    // fast path: straightforward "test name ... STATUS"
    for cap in ENH_TEST_RE_1.captures_iter(&clean) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let mut status = cap.get(2).unwrap().as_str().to_lowercase();
        if status == "failed" || status == "error" {
            status = "failed".to_string();
        }
        match status.as_str() {
            "ok" => { passed.insert(name); }
            "failed" => { failed.insert(name); }
            "ignored" => { ignored.insert(name); }
            _ => {}
        }
    }

    // UI test format: "path/to/test.rs ... ok" (without "test" keyword)
    for line in clean.lines() {
        if let Some(cap) = UI_TEST_PATH_RE.captures(line) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let mut status = cap.get(2).unwrap().as_str().to_lowercase();
            if status == "failed" || status == "error" {
                status = "failed".to_string();
            }
            match status.as_str() {
                "ok" => { passed.insert(name); }
                "failed" => { failed.insert(name); }
                "ignored" => { ignored.insert(name); }
                _ => {}
            }
        }
    }

    // UI test format: "path/to/test.toml ... ok" (including .toml files)
    for line in clean.lines() {
        if let Some(cap) = UI_TEST_PATH_SIMPLE_RE.captures(line) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let mut status = cap.get(2).unwrap().as_str().to_lowercase();
            if status == "failed" || status == "error" {
                status = "failed".to_string();
            }
            match status.as_str() {
                "ok" => { passed.insert(name); }
                "failed" => { failed.insert(name); }
                "ignored" => { ignored.insert(name); }
                _ => {}
            }
        }
    }

    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());

    ParsedLog { passed, failed, ignored, all }
}

fn strip_ansi_color_codes(s: &str) -> String {
    ANSI_RE.replace_all(s, "").into_owned()
}

fn parse_rust_log_file(text: &str) -> Result<ParsedLog, String> {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();
    let mut freq = std::collections::HashMap::new();
    
    let lines: Vec<&str> = text.lines().collect();
    
    // First pass: handle normal test lines with immediate results
    for line in &lines {
        if let Some(captures) = TEST_LINE_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            let status = captures.get(2).unwrap().as_str().to_lowercase();
            
            *freq.entry(test_name.clone()).or_insert(0) += 1;
            
            match status.as_str() {
                "ok" => { passed.insert(test_name); }
                "failed" | "error" => { failed.insert(test_name); }
                "ignored" => { ignored.insert(test_name); }
                _ => {}
            }
        }
    }
    
    // Second pass: handle cases where test result is on a separate line
    let mut pending_tests = std::collections::HashMap::new();
    
    for (i, line) in lines.iter().enumerate() {
        if let Some(captures) = TEST_START_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            let remainder = captures.get(2).unwrap().as_str();
            
            // Skip if we already found this test with a clear status
            if passed.contains(&test_name) || failed.contains(&test_name) || ignored.contains(&test_name) {
                continue;
            }
            
            // If remainder doesn't contain a clear status, this test might have result later
            if !STATUS_RE.is_match(remainder) {
                pending_tests.insert(test_name, i);
            }
        }

        // Also consider corrupted test lines mixed with debug output
        if let Some(cap) = CORRUPTED_TEST_LINE_RE.captures(line) {
            let tn = cap.get(1).unwrap().as_str().to_string();
            if !passed.contains(&tn) && !failed.contains(&tn) && !ignored.contains(&tn) {
                pending_tests.insert(tn, i);
            }
        }
    }
    
    // For pending tests, search more aggressively for their results
    for (test_name, start_line) in pending_tests {
        // Look in subsequent lines for the result, potentially many lines later
        let initial_limit = 200usize;
        let extended_limit = 10_000usize; // for verbose logs
        let mut found = false;

        // heuristic: try normal window first
        for j in start_line + 1..std::cmp::min(start_line + initial_limit, lines.len()) {
            let line = lines[j];

            // Check for standalone status words
            let stripped = line.trim();
            if stripped.eq_ignore_ascii_case("ok")
                || stripped.eq_ignore_ascii_case("FAILED")
                || stripped.eq_ignore_ascii_case("ignored")
                || stripped.eq_ignore_ascii_case("error")
            {
                let status = stripped.to_lowercase();
                *freq.entry(test_name.clone()).or_insert(0) += 1;

                match status.as_str() {
                    "ok" => { passed.insert(test_name.clone()); }
                    "failed" | "error" => { failed.insert(test_name.clone()); }
                    "ignored" => { ignored.insert(test_name.clone()); }
                    _ => {}
                }
                found = true;
                break;
            }

            // Check for status words at the end of lines (after debug output) OR at the beginning mixed with logging
            let mut status_match = None;
            if let Some(captures) = STATUS_AT_END_RE.captures(line) {
                status_match = Some(captures);
            } else if let Some(captures) = STATUS_AT_START_RE.captures(line) {
                status_match = Some(captures);
            }

            if let Some(captures) = status_match {
                let status = captures.get(1).unwrap().as_str().to_lowercase();
                
                // Enhanced filtering to avoid false positives from diagnostic messages
                if is_diagnostic_error(&status, line) {
                    continue;
                }
                
                // Also skip if the status word appears in the middle of a diagnostic message
                if is_status_in_diagnostic_context(&status, line) {
                    continue;
                }

                process_test_status(&status, &test_name, &mut passed, &mut failed, &mut ignored, &mut freq);
                found = true;
                break;
            }

            // Stop looking if we hit another test line (but allow some leeway)
            if ANOTHER_TEST_RE.is_match(line) && j > start_line + 5 {
                break;
            }
        }

        // Extended scan window for extremely verbose logs
        if !found {
            for j in std::cmp::min(start_line + initial_limit, lines.len())..std::cmp::min(start_line + extended_limit, lines.len()) {
                let line = lines[j];
                let stripped = line.trim();
                if stripped.eq_ignore_ascii_case("ok")
                    || stripped.eq_ignore_ascii_case("FAILED")
                    || stripped.eq_ignore_ascii_case("ignored")
                    || stripped.eq_ignore_ascii_case("error")
                {
                    let status = stripped.to_lowercase();
                    process_test_status(&status, &test_name, &mut passed, &mut failed, &mut ignored, &mut freq);
                    break;
                }

                // Check for status words at the end of lines (after debug output) OR at the beginning mixed with logging
                let mut status_match = None;
                if let Some(captures) = STATUS_AT_END_RE.captures(line) {
                    status_match = Some(captures);
                } else if let Some(captures) = STATUS_AT_START_RE.captures(line) {
                    status_match = Some(captures);
                }

                if let Some(captures) = status_match {
                    let status = captures.get(1).unwrap().as_str().to_lowercase();
                    
                    // Enhanced filtering to avoid false positives from diagnostic messages
                    if is_diagnostic_error(&status, line) {
                        continue;
                    }
                    
                    // Also skip if the status word appears in the middle of a diagnostic message
                    if is_status_in_diagnostic_context(&status, line) {
                        continue;
                    }

                    process_test_status(&status, &test_name, &mut passed, &mut failed, &mut ignored, &mut freq);
                    break;
                }

                if ANOTHER_TEST_RE.is_match(line) && j > start_line + 50 { break; }
            }
        }
    }
    
    // Third pass: handle split status words like "o\nk"
    for (i, line) in lines.iter().enumerate() {
        // Look for lines that end with just "o" and check if next line starts with "k"
        if line.trim() == "o" && i + 1 < lines.len() && lines[i + 1].trim() == "k" {
            // Look backwards to find the corresponding test
            for j in (0..i).rev().take(10) {
                if let Some(captures) = TEST_WITH_O_RE.captures(lines[j]) {
                    let test_name = captures.get(1).unwrap().as_str().to_string();
                    if !passed.contains(&test_name) && !failed.contains(&test_name) && !ignored.contains(&test_name) {
                        *freq.entry(test_name.clone()).or_insert(0) += 1;
                        passed.insert(test_name);
                    }
                    break;
                }
            }
        }
        
        // Also handle the case where test line itself ends with "... o" (split across lines)
        if let Some(captures) = TEST_WITH_O_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            if i + 1 < lines.len() && lines[i + 1].trim() == "k" {
                if !passed.contains(&test_name) && !failed.contains(&test_name) && !ignored.contains(&test_name) {
                    *freq.entry(test_name.clone()).or_insert(0) += 1;
                    passed.insert(test_name);
                }
            }
        }
    }
    
    // Also read the "failures:" block to catch names not emitted on one-line form
    let mut collecting = false;
    for line in &lines {
        let trimmed = line.trim();
        if trimmed == "failures:" {
            collecting = true;
            continue;
        }
        if collecting {
            if trimmed.starts_with("error:") || trimmed.starts_with("test result:") {
                collecting = false;
                continue;
            }
            if let Some(captures) = FAILURES_BLOCK_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().to_string();
                if !test_name.starts_with("----") {
                    failed.insert(test_name);
                }
                continue;
            }
            if trimmed.is_empty() || trimmed.starts_with("----") {
                continue;
            }
            collecting = false;
        }
    }
    
    let mut all = HashSet::new();
    all.extend(passed.iter().cloned());
    all.extend(failed.iter().cloned());
    all.extend(ignored.iter().cloned());
    
    Ok(ParsedLog {
        passed,
        failed,
        ignored,
        all,
    })
}

// Helper function to check if an error status is part of diagnostic messages
fn is_diagnostic_error(status: &str, line: &str) -> bool {
    if status != "error" {
        return false;
    }
    
    let line_lower = line.to_lowercase();
    line_lower.contains("error:") || 
    line_lower.contains("panic") ||
    line_lower.contains("custom") ||
    line_lower.contains("called `result::unwrap()") ||
    line_lower.contains("thread") ||
    line_lower.contains("kind:")
}

// Helper function to check if status appears in the middle of diagnostic messages
fn is_status_in_diagnostic_context(status: &str, line: &str) -> bool {
    let line_lower = line.to_lowercase();
    if let Some(pos) = line_lower.find(status) {
        let before_status = &line_lower[..pos];
        let after_status = &line_lower[pos + status.len()..];
        
        before_status.contains("error:") || 
        before_status.contains("panic") ||
        after_status.contains("value:") ||
        after_status.contains("kind:")
    } else {
        false
    }
}

// Helper function to process status and update test collections
fn process_test_status(
    status: &str,
    test_name: &str,
    passed: &mut HashSet<String>,
    failed: &mut HashSet<String>,
    ignored: &mut HashSet<String>,
    freq: &mut std::collections::HashMap<String, i32>
) {
    *freq.entry(test_name.to_string()).or_insert(0) += 1;
    
    match status {
        "ok" => { passed.insert(test_name.to_string()); }
        "failed" | "error" => { failed.insert(test_name.to_string()); }
        "ignored" => { ignored.insert(test_name.to_string()); }
        _ => {}
    }
}
