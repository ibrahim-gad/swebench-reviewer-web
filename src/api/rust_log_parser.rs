use regex::Regex;
use std::collections::HashSet;
use std::fs;
use lazy_static::lazy_static;

use super::log_parser::{LogParserTrait, ParsedLog};

// Compile regex patterns once at module level to avoid repeated compilation
lazy_static! {
    // Case-insensitive, include error status, allow trailing whitespace, handle line numbers
    static ref TEST_LINE_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+(.+?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$")
        .expect("Failed to compile TEST_LINE_RE regex");

    // Pattern for mixed format: "test name ... status additional_content"
    static ref TEST_MIXED_FORMAT_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+(.+?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s+(.+)")
        .expect("Failed to compile TEST_MIXED_FORMAT_RE regex");

    static ref TEST_START_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+(.+?)\s+\.\.\.\s*(.*?)$")
        .expect("Failed to compile TEST_START_RE regex");

    static ref STATUS_RE: Regex = Regex::new(r"(?i)\b(ok|failed|ignored|error)\b")
        .expect("Failed to compile STATUS_RE regex");

    static ref STATUS_AT_END_RE: Regex = Regex::new(r"(?i)\b(ok|failed|ignored|error)\s*$")
        .expect("Failed to compile STATUS_AT_END_RE regex");

    // New pattern to match status at the beginning of lines mixed with logging output
    static ref STATUS_AT_START_RE: Regex = Regex::new(r"(?i)^(ok|FAILED|ignored|error)")
        .expect("Failed to compile STATUS_AT_START_RE regex");

    static ref ANOTHER_TEST_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+[^\s]+\s+\.\.\.\s*")
        .expect("Failed to compile ANOTHER_TEST_RE regex");

    static ref TEST_WITH_O_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+([^\s]+(?:::\w+)*)\s+\.\.\.\s*o\s*$")
        .expect("Failed to compile TEST_WITH_O_RE regex");

    static ref TEST_STARTS_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+([^\s]+(?:::\w+)*)\s+\.\.\.\s*")
        .expect("Failed to compile TEST_STARTS_RE regex");

    static ref STATUS_IN_TEXT_RE: Regex = Regex::new(r"(?i)\b(ok|failed|ignored|error)\b")
        .expect("Failed to compile STATUS_IN_TEXT_RE regex");

    // Additional patterns
    static ref CORRUPTED_TEST_LINE_RE: Regex = Regex::new(r"(?i)(?:line)?(?:\d+)?test\s+([^\s]+(?:::\w+)*)\s+\.\.\.\s*")
        .expect("Failed to compile CORRUPTED_TEST_LINE_RE regex");

    // File boundary hints
    static ref FILE_BOUNDARY_RE_1: Regex = Regex::new(r"(?i)Running\s+([^\s]+(?:/[^\s]+)*\.(?:rs|fixed))\s*\(").unwrap();
    static ref FILE_BOUNDARY_RE_2: Regex = Regex::new(r"(?i)===\s*Running\s+(.+\.(?:rs|fixed))").unwrap();
    static ref FILE_BOUNDARY_RE_3: Regex = Regex::new(r"(?i)test\s+result:\s+ok\.\s+\d+\s+passed.*for\s+(.+\.(?:rs|fixed))").unwrap();

    // Enhanced extraction patterns
    static ref ENH_TEST_RE_1: Regex = Regex::new(r"(?i)(?:\d+)?test\s+([^\s]+(?:::[^\s]+)*)\s*\.{2,}\s*(ok|FAILED|ignored|error)").unwrap();
    static ref ENH_TEST_RE_2: Regex = Regex::new(r"(?i)(?:\d+)?test\s+([^\s]+)\s+\.\.\.\s+(ok|FAILED|ignored|error)").unwrap();
    
    // UI test format patterns - handles paths as test names with direct status
    static ref UI_TEST_PATH_RE: Regex = Regex::new(r"(?i)^([^\s]+(?:/[^\s]+)*\.(?:rs|fixed|toml|txt|md)(?:\s+\(revision\s+[^)]+\))?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$").unwrap();
    static ref UI_TEST_PATH_SIMPLE_RE: Regex = Regex::new(r"(?i)^([^\s]+(?:/[^\s]+)*\.(?:rs|fixed|toml|txt|md)(?:\s+\(revision\s+[^)]+\))?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$").unwrap();
    
    // Nextest format patterns - handles "PASS [duration] test_name" and "FAIL [duration] test_name"
    static ref NEXTEST_PASS_RE: Regex = Regex::new(r"(?i)\s*PASS\s+\[[^\]]+\]\s+(.+?)\s*$").unwrap();
    static ref NEXTEST_FAIL_RE: Regex = Regex::new(r"(?i)\s*FAIL\s+\[[^\]]+\]\s+(.+?)\s*$").unwrap();
    static ref NEXTEST_SKIP_RE: Regex = Regex::new(r"(?i)\s*(SKIP|IGNORED)\s+\[[^\]]+\]\s+(.+?)\s*$").unwrap();
    
    // START pattern for nextest - captures test names from START lines
    static ref NEXTEST_START_RE: Regex = Regex::new(r"(?i)^\s*START\s+(.+)$").unwrap();

    // ANSI escape detection
    static ref ANSI_RE: Regex = Regex::new(r"\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])").unwrap();

    static ref FAILURES_BLOCK_RE: Regex = Regex::new(r"^\s{4}(.+?)\s*$")
        .expect("Failed to compile FAILURES_BLOCK_RE regex");

    // Additional patterns for single-line parsing to avoid repeated compilation
    static ref SINGLE_LINE_START_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+([^\s]+(?:::[^\s]+)*)\s*\.{2,}").unwrap();
    static ref SINGLE_LINE_NEXT_TEST_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+[^\s]+(?:::[^\s]+)*\s*\.{2,}").unwrap();
    static ref SINGLE_LINE_STATUS_AT_START_RE: Regex = Regex::new(r"(?i)^(ok|FAILED|ignored|error)").unwrap();
    static ref SIMPLE_PATTERN_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+[^\s]+(?:::[^\s]+)*\s*\.{2,}\s*(ok|FAILED|ignored|error)").unwrap();
    
    // Pattern for tests that have diagnostic info after the "..." but before status
    static ref TEST_WITH_DIAGNOSTICS_RE: Regex = Regex::new(r"(?i)(?:\d+)?test\s+(.+?)\s+\.\.\.\s*(?:error:|$)").unwrap();
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
    
    // Only treat as single-line if:
    // 1. Very short logs with many tests (condensed format), OR
    // 2. Many UI test patterns (path-based tests), OR  
    // 3. ANSI codes AND it's actually a short log (not just diagnostic output with ANSI)
    (line_count <= 3 && test_count > 5) || 
    has_ui_tests || 
    (has_ansi && line_count <= 10 && test_count >= line_count / 2)
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
    
    // Check for cargo nextest run command line
    let has_nextest_command = text.contains("cargo nextest run");
    
    has_indicators || nextest_lines > 5 || has_mixed_format || has_nextest_command
}

fn parse_nextest_log(text: &str) -> ParsedLog {
    let mut passed = HashSet::new();
    let mut failed = HashSet::new();
    let mut ignored = HashSet::new();

    let lines: Vec<&str> = text.lines().collect();

    // Parse nextest format using separate regex patterns for better accuracy
    for (i, line) in lines.iter().enumerate() {
        // Parse PASS lines
        if let Some(captures) = NEXTEST_PASS_RE.captures(line) {
            let full_match = captures.get(1).unwrap().as_str().trim();
            // Extract just the test name part (after the crate name)
            let test_name = extract_test_name_from_nextest_line(full_match);
            passed.insert(test_name);
            continue;
        }
        
        // Parse FAIL lines
        if let Some(captures) = NEXTEST_FAIL_RE.captures(line) {
            let full_match = captures.get(1).unwrap().as_str().trim();
            // Extract just the test name part (after the crate name)
            let test_name = extract_test_name_from_nextest_line(full_match);
            failed.insert(test_name);
            continue;
        }
        
        // Parse SKIP/IGNORED lines - note: using capture group 2 for SKIP/IGNORED
        if let Some(captures) = NEXTEST_SKIP_RE.captures(line) {
            // For SKIP/IGNORED pattern, the test name is in group 2
            if let Some(test_name_match) = captures.get(2) {
                let full_match = test_name_match.as_str().trim();
                let test_name = extract_test_name_from_nextest_line(full_match);
                ignored.insert(test_name);
            }
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
        
        // Handle mixed format: "test name ... status additional_content"
        if let Some(captures) = TEST_MIXED_FORMAT_RE.captures(line) {
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
        
        // Handle the diagnostic pattern: test starts with error/diagnostic but ends with status
        if let Some(captures) = TEST_START_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            let remainder = captures.get(2).unwrap().as_str().trim();
            
            // Skip if we already processed this test
            if passed.contains(&test_name) || failed.contains(&test_name) || ignored.contains(&test_name) {
                continue;
            }
            
            // Look for diagnostic pattern: test starts with diagnostic info or is empty after "..."
            if remainder.starts_with("error:") || remainder.is_empty() {
                // Search forward for the final status
                for j in (i + 1)..std::cmp::min(i + 50, lines.len()) {
                    let search_line = lines[j].trim();
                    
                    // Stop if we hit another test
                    if TEST_START_RE.is_match(lines[j]) {
                        break;
                    }
                    
                    // Look for standalone status words
                    if search_line.eq_ignore_ascii_case("ok") {
                        passed.insert(test_name.clone());
                        break;
                    } else if search_line.eq_ignore_ascii_case("failed") || 
                             search_line.eq_ignore_ascii_case("error") {
                        failed.insert(test_name.clone());
                        break;
                    } else if search_line.eq_ignore_ascii_case("ignored") {
                        ignored.insert(test_name.clone());
                        break;
                    }
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

    // Handle mixed format: "test name ... status additional_content"
    for cap in TEST_MIXED_FORMAT_RE.captures_iter(&clean) {
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

    // harder cases: "test name ... <debug> STATUS" before next test
    for cap in SINGLE_LINE_START_RE.captures_iter(&clean) {
        let name = cap.get(1).unwrap().as_str().to_string();
        if passed.contains(&name) || failed.contains(&name) || ignored.contains(&name) {
            continue;
        }
        let search_pos = cap.get(0).unwrap().end();
        let end_pos = if let Some(ncap) = SINGLE_LINE_NEXT_TEST_RE.find_at(&clean, search_pos) {
            ncap.start()
        } else {
            std::cmp::min(search_pos + 1000, clean.len())
        };
        let window = &clean[search_pos..end_pos];

        // Find all status matches including beginning-of-line patterns and pick the most appropriate one
        let mut status_matches = Vec::new();
        
        // Look for status at end of lines within window
        for m in STATUS_IN_TEXT_RE.find_iter(window) {
            let status = m.as_str().to_lowercase();
            let match_start = m.start();
            
            // Get context around the match (safely handle UTF-8 boundaries)
            let context_start = match_start.saturating_sub(50);
            let context_end = std::cmp::min(match_start + 50, window.len());
            
            // Optimized single-pass character boundary detection
            let mut safe_start = None;
            let mut safe_end = None;
            for (i, _) in window.char_indices() {
                if safe_start.is_none() && i >= context_start {
                    safe_start = Some(i);
                }
                if safe_end.is_none() && i >= context_end {
                    safe_end = Some(i);
                }
                if safe_start.is_some() && safe_end.is_some() {
                    break;
                }
            }
            let safe_start = safe_start.unwrap_or(context_start);
            let safe_end = safe_end.unwrap_or(context_end);
            let context = &window[safe_start..safe_end].to_lowercase();
            
            // Enhanced filtering to avoid false positives
            if status == "error" && (
                context.contains("error:") || 
                context.contains("panic") ||
                context.contains("custom") ||
                context.contains("called `result::unwrap()") ||
                context.contains("thread") ||
                context.contains("kind:")
            ) {
                continue;
            }
            
            status_matches.push((status, match_start));
        }
        
        // Also look for status at the beginning of lines mixed with logging
        for line in window.lines() {
            if let Some(cap) = SINGLE_LINE_STATUS_AT_START_RE.captures(line) {
                let status = cap.get(1).unwrap().as_str().to_lowercase();
                let line_lower = line.to_lowercase();
                
                // Special handling for status mixed with logging output
                if (status == "failed" || status == "error") && 
                   (line_lower.contains("logging at") || 
                    line_lower.contains("debug:") || 
                    line_lower.contains("trace:") || 
                    line_lower.contains("info:") || 
                    line_lower.contains("warn:")) {
                    
                    // Check for panic evidence for this test in the window
                    let panic_for_this_test = window.to_lowercase().contains(&format!("thread '{}'", name)) && 
                                            window.to_lowercase().contains("panicked at");
                    
                    if panic_for_this_test {
                        status_matches.push((status, 0)); // Use 0 as position indicator for start-of-line matches
                    }
                } else {
                    status_matches.push((status, 0));
                }
            }
        }
        
        // Use the last (most recent) valid status match
        if let Some((status, _)) = status_matches.last() {
            match status.as_str() {
                "ok" => { passed.insert(name); }
                "failed" | "error" => { failed.insert(name); }
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
        // Handle standard format: "test name ... status"
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
            continue;
        }
        
        // Handle mixed format: "test name ... status additional_content"
        if let Some(captures) = TEST_MIXED_FORMAT_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            let status = captures.get(2).unwrap().as_str().to_lowercase();
            
            *freq.entry(test_name.clone()).or_insert(0) += 1;
            
            match status.as_str() {
                "ok" => { passed.insert(test_name); }
                "failed" | "error" => { failed.insert(test_name); }
                "ignored" => { ignored.insert(test_name); }
                _ => {}
            }
            continue;
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

                // Special handling for status mixed with logging output
                // Skip if the status appears mixed with logging output UNLESS there's evidence of a panic for this test
                let line_lower = line.to_lowercase();
                if (status == "failed" || status == "error") && 
                   (line_lower.contains("logging at") || 
                    line_lower.contains("debug:") || 
                    line_lower.contains("trace:") || 
                    line_lower.contains("info:") || 
                    line_lower.contains("warn:")) {
                    
                    // Check if there's a panic message for this specific test in a broader range
                    let search_start = start_line.saturating_sub(100);
                    let search_end = std::cmp::min(j + 1, lines.len());
                    
                    if !has_panic_evidence(&test_name, &lines, search_start, search_end) {
                        // This status is mixed with logging output and no panic evidence, skip it
                        continue;
                    }
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

                    // Special handling for status mixed with logging output
                    // Skip if the status appears mixed with logging output UNLESS there's evidence of a panic for this test
                    let line_lower = line.to_lowercase();
                    if (status == "failed" || status == "error") && 
                       (line_lower.contains("logging at") || 
                        line_lower.contains("debug:") || 
                        line_lower.contains("trace:") || 
                        line_lower.contains("info:") || 
                        line_lower.contains("warn:")) {
                        
                        // Check if there's a panic message for this specific test in a broader range
                        let search_start = start_line.saturating_sub(100);
                        let search_end = std::cmp::min(j + 1, lines.len());
                        
                        if !has_panic_evidence(&test_name, &lines, search_start, search_end) {
                            // This status is mixed with logging output and no panic evidence, skip it
                            continue;
                        }
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
    
    // Fourth pass: handle tests with complex formatting
    let mut test_starts = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(captures) = TEST_STARTS_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            test_starts.push((i, test_name));
        }
    }
    
    // For each test start, look for the corresponding result within a reasonable range
    for (line_idx, test_name) in test_starts {
        if passed.contains(&test_name) || failed.contains(&test_name) || ignored.contains(&test_name) {
            continue;
        }
        
        // Search forward through lines for the result
        let mut search_text = String::new();
        for j in line_idx..std::cmp::min(line_idx + 100, lines.len()) {
            search_text.push_str(lines[j]);
            search_text.push('\n');
            
            // Stop if we hit another test (but give some leeway for interleaved output)
            if j > line_idx + 5 && TEST_STARTS_RE.is_match(lines[j]) {
                break;
            }
        }
        
        // Look for status in this accumulated text, but be more selective
        // Find all status matches and pick the most likely one
        let mut status_matches = Vec::new();
        for cap in STATUS_IN_TEXT_RE.captures_iter(&search_text) {
            let status = cap.get(1).unwrap().as_str().to_lowercase();
            let match_start = cap.get(0).unwrap().start();
            
            // Get some context around the match (safely handle UTF-8 boundaries)
            let context_start = match_start.saturating_sub(50);
            let context_end = std::cmp::min(match_start + 50, search_text.len());
            
            // Optimized single-pass character boundary detection
            let mut safe_start = None;
            let mut safe_end = None;
            for (i, _) in search_text.char_indices() {
                if safe_start.is_none() && i >= context_start {
                    safe_start = Some(i);
                }
                if safe_end.is_none() && i >= context_end {
                    safe_end = Some(i);
                }
                if safe_start.is_some() && safe_end.is_some() {
                    break;
                }
            }
            let safe_start = safe_start.unwrap_or(context_start);
            let safe_end = safe_end.unwrap_or(context_end);
            let context = &search_text[safe_start..safe_end].to_lowercase();
            
            // Enhanced filtering to avoid false positives
            if status == "error" && (
                context.contains("error:") || 
                context.contains("panic") ||
                context.contains("custom") ||
                context.contains("called `result::unwrap()") ||
                context.contains("thread") ||
                context.contains("kind:")
            ) {
                continue;
            }
            
            status_matches.push((status, match_start));
        }
        
        // Use the last (most recent) valid status match
        if let Some((status, _)) = status_matches.last() {
            process_test_status(&status, &test_name, &mut passed, &mut failed, &mut ignored, &mut freq);
        }
    }
    
    // Fifth pass: handle tests with diagnostic output followed by status on separate line
    // This handles patterns like:
    // test name ... error: some diagnostic
    // more diagnostic lines
    // ok
    for (i, line) in lines.iter().enumerate() {
        if let Some(captures) = TEST_START_RE.captures(line) {
            let test_name = captures.get(1).unwrap().as_str().to_string();
            let remainder = captures.get(2).unwrap().as_str().trim();
            
            // Skip if we already processed this test
            if passed.contains(&test_name) || failed.contains(&test_name) || ignored.contains(&test_name) {
                continue;
            }
            
            // Look for diagnostic pattern: test starts with diagnostic info but no immediate status
            if remainder.starts_with("error:") || remainder.is_empty() {
                // Search forward for the final status (usually "ok", "failed", etc.)
                let mut found_status = false;
                for j in (i + 1)..std::cmp::min(i + 50, lines.len()) {
                    let search_line = lines[j].trim();
                    
                    // Stop if we hit another test
                    if TEST_START_RE.is_match(lines[j]) {
                        break;
                    }
                    
                    // Look for standalone status words
                    if search_line.eq_ignore_ascii_case("ok") {
                        passed.insert(test_name.clone());
                        found_status = true;
                        break;
                    } else if search_line.eq_ignore_ascii_case("failed") || 
                             search_line.eq_ignore_ascii_case("error") {
                        failed.insert(test_name.clone());
                        found_status = true;
                        break;
                    } else if search_line.eq_ignore_ascii_case("ignored") {
                        ignored.insert(test_name.clone());
                        found_status = true;
                        break;
                    }
                }
                
                if found_status {
                    *freq.entry(test_name).or_insert(0) += 1;
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

// Helper function to check for panic evidence for a specific test
fn has_panic_evidence(test_name: &str, lines: &[&str], search_start: usize, search_end: usize) -> bool {
    let search_range = &lines[search_start..search_end];
    search_range.iter().any(|search_line| {
        let search_lower = search_line.to_lowercase();
        search_lower.contains(&format!("thread '{}'", test_name)) && 
        search_lower.contains("panicked at")
    })
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

// Function to extract clean test name from nextest line
// This tries to intelligently parse different nextest formats without hardcoding specific crates
fn extract_test_name_from_nextest_line(full_line: &str) -> String {
    let trimmed = full_line.trim();
    
    // Simple approach: Just return the full test name as captured by regex
    // The nextest format is: "PASS [time] full_test_name"
    // We should preserve the full test name exactly as it appears
    
    // Special handling for known patterns in main.json:
    // 1. "miden-testing kernel_tests::..." -> keep as is
    // 2. "miden-testing::miden-integration-tests ..." -> keep as is  
    // 3. "miden-lib ..." -> keep as is
    // 4. "miden-objects ..." -> keep as is
    // 5. "miden-tx ..." -> keep as is
    
    // For miden crates, the format in main.json matches exactly what's in the log
    if trimmed.starts_with("miden-") {
        return trimmed.to_string();
    }
    
    // Check for double crate format: "miden-testing::miden-integration-tests scripts::faucet::test"
    if trimmed.contains("::miden-integration-tests ") {
        return trimmed.to_string();
    }
    
    // Check for crate::lib format: "grillon::lib assert::json_path..." -> just the test part
    if trimmed.contains("::lib ") {
        if let Some(lib_pos) = trimmed.find("::lib ") {
            return trimmed[lib_pos + 6..].trim().to_string(); // 6 = len("::lib ")
        }
    }
    
    // For other formats, check if there's a space and we should remove the crate prefix
    if let Some(space_pos) = trimmed.find(' ') {
        let crate_part = &trimmed[..space_pos];
        let test_part = &trimmed[space_pos + 1..];
        
        // If the crate part doesn't contain "::" and the test part does, remove the crate prefix
        if !crate_part.contains("::") && test_part.contains("::") {
            return test_part.trim().to_string();
        }
    }
    
    // If no patterns match, return the original
    trimmed.to_string()
}
