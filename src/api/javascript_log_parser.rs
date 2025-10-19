use std::collections::HashMap;
use std::path::Path;
use regex::Regex;
use lazy_static::lazy_static;
use crate::api::log_parser::{LogParserTrait, ParsedLog};
use crate::api::test_detection::detect_js_testing_framework;

pub struct JavaScriptLogParser {
    parser_name: String,
    project_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Pending,
}

impl TestStatus {
    pub fn to_string(&self) -> String {
        match self {
            TestStatus::Passed => "PASSED".to_string(),
            TestStatus::Failed => "FAILED".to_string(),
            TestStatus::Skipped => "SKIPPED".to_string(),
            TestStatus::Pending => "PENDING".to_string(),
        }
    }
}

impl JavaScriptLogParser {
    pub fn new() -> Self {
        Self {
            parser_name: "auto".to_string(),
            project_path: None,
        }
    }

    pub fn new_with_parser(parser_name: &str) -> Self {
        Self {
            parser_name: parser_name.to_string(),
            project_path: None,
        }
    }

    pub fn new_with_project_path(project_path: &str) -> Self {
        Self {
            parser_name: "auto".to_string(),
            project_path: Some(project_path.to_string()),
        }
    }

    fn parse_log_calypso(&self, log: &str) -> HashMap<String, TestStatus> {
        let mut test_status_map = HashMap::new();
        let mut suite: Vec<(String, usize)> = Vec::new();

        lazy_static! {
            static ref JEST_SPLIT_RE: Regex = Regex::new(r" \./node_modules/\.bin/jest ").unwrap();
            static ref PASS_RE: Regex = Regex::new(r"^\s+✓\s(.*?)(?:\(\d+ms\))?$").unwrap();
            static ref FAIL_RE: Regex = Regex::new(r"^\s+✕\s(.*?)(?:\(\d+ms\))?$").unwrap();
        }

        let sections: Vec<&str> = JEST_SPLIT_RE.split(log).collect();
        for section in sections.iter().skip(1) {
            for line in section.lines() {
                if line.starts_with("Test Suites") || line.starts_with("  ● ") {
                    break;
                }

                let trimmed = line.trim();
                if trimmed.starts_with("✓") {
                    if let Some(captures) = PASS_RE.captures(line) {
                        let test_name = captures.get(1).unwrap().as_str();
                        let full_name = self.get_test_name(&suite, test_name);
                        test_status_map.insert(full_name, TestStatus::Passed);
                    }
                } else if trimmed.starts_with("✕") {
                    if let Some(captures) = FAIL_RE.captures(line) {
                        let test_name = captures.get(1).unwrap().as_str();
                        let full_name = self.get_test_name(&suite, test_name);
                        test_status_map.insert(full_name, TestStatus::Failed);
                    }
                } else if line.len() > line.trim_start().len() {
                    // Adjust suite name based on indentation
                    let indent = line.len() - line.trim_start().len();
                    let suite_name = line.trim().to_string();
                    
                    if suite.is_empty() {
                        suite.push((suite_name, indent));
                    } else {
                        while !suite.is_empty() && suite.last().unwrap().1 >= indent {
                            suite.pop();
                        }
                        suite.push((suite_name, indent));
                    }
                }
            }
        }

        test_status_map
    }

    fn parse_log_mocha_v2(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref ANSI_RE: Regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
            static ref PASS_RE: Regex = Regex::new(r"^\s*[✓√✔]\s+(.*?)(?:\s+\(\d+ms\))?\s*$").unwrap();
            static ref FAIL_RE: Regex = Regex::new(r"^\s{4,}\d+\)\s+(.*)").unwrap();
            static ref CROSS_RE: Regex = Regex::new(r"^\s*[×✕]\s+(.*)").unwrap();
            static ref PEND_RE: Regex = Regex::new(r"^\s*[-•]\s+(.*)").unwrap();
            static ref SUMMARY_RE: Regex = Regex::new(r"^\s*\d+\s+(passing|failing|pending)").unwrap();
            static ref DUR_TAIL_RE: Regex = Regex::new(r"\s+\([\d\.]+ ?[a-zA-Z]+\)$").unwrap();
        }

        let mut test_status_map = HashMap::new();
        let mut suite_stack: Vec<String> = Vec::new();
        let mut count_empty_lines = 0;

        for raw_line in log.lines() {
            let line = ANSI_RE.replace_all(raw_line.trim_end(), "").to_string();

            if line.is_empty() {
                count_empty_lines += 1;
                if count_empty_lines >= 2 {
                    count_empty_lines = 0;
                    suite_stack.clear();
                }
                continue;
            }

            // Summary detected
            if SUMMARY_RE.is_match(&line) {
                suite_stack.clear();
                continue;
            }

            // Passing test
            if let Some(captures) = PASS_RE.captures(&line) {
                let mut test_name = captures.get(1).unwrap().as_str().trim().to_string();
                test_name = DUR_TAIL_RE.replace(&test_name, "").to_string();
                let full_name = if suite_stack.is_empty() {
                    test_name
                } else {
                    format!("{} - {}", suite_stack.join(" - "), test_name)
                };
                test_status_map.insert(full_name, TestStatus::Passed);
                continue;
            }

            // Failing test
            if let Some(captures) = FAIL_RE.captures(&line).or_else(|| CROSS_RE.captures(&line)) {
                let mut test_name = captures.get(1).unwrap().as_str().trim().to_string();
                test_name = DUR_TAIL_RE.replace(&test_name, "").to_string();
                let full_name = if suite_stack.is_empty() {
                    test_name
                } else {
                    format!("{} - {}", suite_stack.join(" - "), test_name)
                };
                test_status_map.insert(full_name, TestStatus::Failed);
                continue;
            }

            // Pending test
            if let Some(captures) = PEND_RE.captures(&line) {
                let mut test_name = captures.get(1).unwrap().as_str().trim().to_string();
                test_name = DUR_TAIL_RE.replace(&test_name, "").to_string();
                let full_name = if suite_stack.is_empty() {
                    test_name
                } else {
                    format!("{} - {}", suite_stack.join(" - "), test_name)
                };
                test_status_map.insert(full_name, TestStatus::Pending);
                continue;
            }

            // Suite header
            let indent = line.len() - line.trim_start().len();
            if indent >= 2 {
                let level = (indent / 2) - 1;
                if level < suite_stack.len() {
                    suite_stack.truncate(level);
                }
                if level == suite_stack.len() {
                    suite_stack.push(line.trim().to_string());
                }
            }
        }

        test_status_map
    }

    fn parse_log_jest(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref JEST_RE: Regex = Regex::new(r"^\s*(✓|✕|○)\s(.+?)(?:\s\((\d+\s*m?s)\))?$").unwrap();
        }

        let mut test_status_map = HashMap::new();

        for line in log.lines() {
            if let Some(captures) = JEST_RE.captures(line.trim()) {
                let status_symbol = captures.get(1).unwrap().as_str();
                let test_name = captures.get(2).unwrap().as_str();

                let status = match status_symbol {
                    "✓" => TestStatus::Passed,
                    "✕" => TestStatus::Failed,
                    "○" => TestStatus::Skipped,
                    _ => continue,
                };

                test_status_map.insert(test_name.to_string(), status);
            }
        }

        test_status_map
    }

    fn parse_log_jest_json(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref JEST_JSON_RE: Regex = Regex::new(r"^\[(PASSED|FAILED)\]\s(.+)$").unwrap();
        }

        let mut test_status_map = HashMap::new();

        for line in log.lines() {
            if let Some(captures) = JEST_JSON_RE.captures(line.trim()) {
                let status_str = captures.get(1).unwrap().as_str();
                let test_name = captures.get(2).unwrap().as_str();

                let status = match status_str {
                    "PASSED" => TestStatus::Passed,
                    "FAILED" => TestStatus::Failed,
                    _ => continue,
                };

                test_status_map.insert(test_name.to_string(), status);
            }
        }

        test_status_map
    }

    fn parse_log_vitest(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref ANSI_RE: Regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
            static ref VITEST_TEST_RE: Regex = Regex::new(r"^\s*([✓×↓])\s+(.+?)(?:\s+(?:\d+\s*m?s|\[skipped\]))?$").unwrap();
            static ref TIMING_RE: Regex = Regex::new(r"\s+(?:\d+\s*m?s|\[skipped\])$").unwrap();
        }

        let mut test_status_map = HashMap::new();

        for line in log.lines() {
            // Strip ANSI escape codes first
            let cleaned_line = ANSI_RE.replace_all(line, "");
            let cleaned_line = Self::strip_bracket_codes(&cleaned_line);
            let trimmed = cleaned_line.trim();
            
            if trimmed.is_empty() {
                continue;
            }
            
            // Look for test result lines with status symbols using regex
            if let Some(captures) = VITEST_TEST_RE.captures(&trimmed) {
                let symbol = captures.get(1).unwrap().as_str();
                let test_content = captures.get(2).unwrap().as_str();
                
                // Clean up any remaining timing info
                let mut test_name = TIMING_RE.replace_all(test_content, "").trim().to_string();
                
                // For Vitest format like "packages/esbuild-plugin-env/test/test.spec.js > esbuild-plugin-env > should inject env values"
                // Extract just the meaningful part after the file path
                if test_name.contains(" > ") {
                    let parts: Vec<&str> = test_name.split(" > ").collect();
                    if parts.len() >= 2 {
                        // Skip the file path (first part), keep the rest
                        // e.g., "esbuild-plugin-env > should inject env values"
                        test_name = parts[1..].join(" > ");
                    }
                }
                
                let status = match symbol {
                    "✓" => TestStatus::Passed,
                    "×" => TestStatus::Failed,
                    "↓" => TestStatus::Skipped,
                    _ => continue,
                };

                test_status_map.insert(test_name, status);
                continue;
            }
            
            // Fallback: check for status symbols at the start (for simpler formats)
            let (symbol, rest) = if trimmed.starts_with('✓') {
                ("✓", &trimmed[3..]) // ✓ is 3 bytes in UTF-8
            } else if trimmed.starts_with('×') {
                ("×", &trimmed[3..]) // × is 3 bytes in UTF-8
            } else if trimmed.starts_with('↓') {
                ("↓", &trimmed[3..]) // ↓ is 3 bytes in UTF-8
            } else {
                continue;
            };
            
            let rest = rest.trim_start();
            
            // Remove timing info like "100ms" or "[skipped]" from the end
            let mut test_name = TIMING_RE.replace_all(rest, "").trim().to_string();
            
            // Apply the same hierarchical name processing
            if test_name.contains(" > ") {
                let parts: Vec<&str> = test_name.split(" > ").collect();
                if parts.len() >= 2 {
                    test_name = parts[1..].join(" > ");
                }
            }
            
            let status = match symbol {
                "✓" => TestStatus::Passed,
                "×" => TestStatus::Failed,
                "↓" => TestStatus::Skipped,
                _ => continue,
            };

            test_status_map.insert(test_name, status);
        }

        test_status_map
    }

    fn parse_log_karma(&self, log: &str) -> HashMap<String, TestStatus> {
        let mut test_status_map = HashMap::new();
        let mut current_indent = -1i32;
        let mut current_suite: Vec<String> = Vec::new();
        let mut started = false;

        lazy_static! {
            static ref KARMA_RE: Regex = Regex::new(r"^(\s*)?([✔✖])?\s(.*)$").unwrap();
        }

        for line in log.lines() {
            if line.starts_with("SUMMARY:") {
                return test_status_map;
            }

            if line.contains("Starting browser") {
                started = true;
                continue;
            }

            if !started {
                continue;
            }

            if let Some(captures) = KARMA_RE.captures(line) {
                let indent_str = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                let status = captures.get(2).map(|m| m.as_str());
                let name = captures.get(3).unwrap().as_str();

                if !indent_str.is_empty() && status.is_none() {
                    let new_indent = indent_str.len() as i32;
                    if new_indent > current_indent {
                        current_indent = new_indent;
                        current_suite.push(name.to_string());
                    } else if new_indent < current_indent {
                        current_indent = new_indent;
                        if !current_suite.is_empty() {
                            current_suite.pop();
                        }
                        continue;
                    }
                }

                if let Some(status_symbol) = status {
                    let mut full_test_name = current_suite.clone();
                    full_test_name.push(name.to_string());
                    let full_name = full_test_name.join(" > ");

                    let test_status = match status_symbol {
                        "✔" => TestStatus::Passed,
                        "✖" => TestStatus::Failed,
                        _ => continue,
                    };

                    test_status_map.insert(full_name, test_status);
                }
            }
        }

        test_status_map
    }

    fn parse_log_tap(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref TAP_RE: Regex = Regex::new(r"^(ok|not ok) (\d+) (.+)$").unwrap();
        }

        let mut test_status_map = HashMap::new();

        for line in log.lines() {
            if let Some(captures) = TAP_RE.captures(line.trim()) {
                let status_str = captures.get(1).unwrap().as_str();
                let test_name = captures.get(3).unwrap().as_str();

                let status = match status_str {
                    "ok" => TestStatus::Passed,
                    "not ok" => TestStatus::Failed,
                    _ => continue,
                };

                test_status_map.insert(test_name.to_string(), status);
            }
        }

        test_status_map
    }

    fn parse_log_chart_js(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref CHARTJS_FAIL_RE: Regex = Regex::new(r"Chrome\s[\d\.]+\s\(.*?\)\s(.*)FAILED$").unwrap();
        }

        let mut test_status_map = HashMap::new();
        
        for caps in CHARTJS_FAIL_RE.captures_iter(log) {
            if let Some(test_name) = caps.get(1) {
                test_status_map.insert(test_name.as_str().to_string(), TestStatus::Failed);
            }
        }

        test_status_map
    }

    fn parse_log_marked(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref MARKED_FAIL_RE: Regex = Regex::new(r"^\d+\)\s(.*)").unwrap();
        }

        let mut test_status_map = HashMap::new();
        
        for line in log.lines() {
            if let Some(caps) = MARKED_FAIL_RE.captures(line) {
                if let Some(test_name) = caps.get(1) {
                    test_status_map.insert(test_name.as_str().trim().to_string(), TestStatus::Failed);
                }
            }
        }

        test_status_map
    }

    fn parse_log_react_pdf(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref PASS_MS_RE: Regex = Regex::new(r"^PASS\s(.*)\s\([\d\.]+ms\)").unwrap();
            static ref PASS_SS_RE: Regex = Regex::new(r"^PASS\s(.*)\s\([\d\.]+\ss\)").unwrap();
            static ref PASS_S_RE: Regex = Regex::new(r"^PASS\s(.*)\s\([\d\.]+s\)").unwrap();
            static ref PASS_RE: Regex = Regex::new(r"^PASS\s(.*)").unwrap();
            static ref FAIL_MS_RE: Regex = Regex::new(r"^FAIL\s(.*)\s\([\d\.]+ms\)").unwrap();
            static ref FAIL_SS_RE: Regex = Regex::new(r"^FAIL\s(.*)\s\([\d\.]+\ss\)").unwrap();
            static ref FAIL_S_RE: Regex = Regex::new(r"^FAIL\s(.*)\s\([\d\.]+s\)").unwrap();
            static ref FAIL_RE: Regex = Regex::new(r"^FAIL\s(.*)").unwrap();
        }

        let mut test_status_map = HashMap::new();
        
        for line in log.lines() {
            let mut matched = false;
            
            // Check PASS patterns
            if let Some(caps) = PASS_MS_RE.captures(line).or_else(|| PASS_SS_RE.captures(line))
                .or_else(|| PASS_S_RE.captures(line)).or_else(|| PASS_RE.captures(line)) {
                if let Some(test_name) = caps.get(1) {
                    test_status_map.insert(test_name.as_str().to_string(), TestStatus::Passed);
                    matched = true;
                }
            }
            
            // Check FAIL patterns if not already matched
            if !matched {
                if let Some(caps) = FAIL_MS_RE.captures(line).or_else(|| FAIL_SS_RE.captures(line))
                    .or_else(|| FAIL_S_RE.captures(line)).or_else(|| FAIL_RE.captures(line)) {
                    if let Some(test_name) = caps.get(1) {
                        test_status_map.insert(test_name.as_str().to_string(), TestStatus::Failed);
                    }
                }
            }
        }

        test_status_map
    }

    fn parse_log_p5js(&self, log: &str) -> HashMap<String, TestStatus> {
        lazy_static! {
            static ref ANSI_RE: Regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
            static ref JSON_BLOCK_RE: Regex = Regex::new(r"\{[^}]*\}").unwrap();
            static ref JSON_LIST_RE: Regex = Regex::new(r"\[[^\]]*\]").unwrap();
            static ref XML_BLOCK_RE: Regex = Regex::new(r"<(\w+)>[\s\S]*?</\1>").unwrap();
            static ref FAIL_PATTERN_RE: Regex = Regex::new(r"^\s*(\d+)\)(.{0,1000}?):").unwrap();
        }

        let mut test_status_map = HashMap::new();
        
        // Clean the log content
        let mut cleaned_log = ANSI_RE.replace_all(log, "").to_string();
        cleaned_log = JSON_BLOCK_RE.replace_all(&cleaned_log, "").to_string();
        cleaned_log = JSON_LIST_RE.replace_all(&cleaned_log, "").to_string();
        cleaned_log = XML_BLOCK_RE.replace_all(&cleaned_log, "").to_string();
        
        // Remove JSON and XML blocks more thoroughly
        let lines: Vec<&str> = cleaned_log.lines().collect();
        let mut filtered_lines = Vec::new();
        let mut in_json_block = false;
        let mut in_json_list_block = false;
        
        for line in lines {
            let stripped = line.trim();
            
            if stripped.ends_with('{') {
                in_json_block = true;
                continue;
            }
            if stripped.ends_with('[') {
                in_json_list_block = true;
                continue;
            }
            if stripped == "}" && in_json_block {
                in_json_block = false;
                continue;
            }
            if stripped == "]" && in_json_list_block {
                in_json_list_block = false;
                continue;
            }
            if in_json_block || in_json_list_block {
                continue;
            }
            if (stripped.starts_with('{') && stripped.ends_with('}')) ||
               (stripped.starts_with('[') && stripped.ends_with(']')) {
                continue;
            }
            
            filtered_lines.push(line);
        }
        
        let filtered_log = filtered_lines.join("\n");
        
        // Parse failing tests
        for caps in FAIL_PATTERN_RE.captures_iter(&filtered_log) {
            if let Some(test_content) = caps.get(2) {
                let test_names: Vec<&str> = test_content.as_str()
                    .lines()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                if !test_names.is_empty() {
                    let full_name = test_names.join(":");
                    test_status_map.insert(full_name, TestStatus::Failed);
                }
            }
        }

        test_status_map
    }

    fn parse_log_cypress(&self, log: &str) -> HashMap<String, TestStatus> {
        let mut test_status_map = HashMap::new();

        lazy_static! {
            static ref CYPRESS_PASS_RE: Regex = Regex::new(r"^\s*✓\s+(.+?)(?:\s+\(\d+ms\))?$").unwrap();
            static ref CYPRESS_FAIL_RE: Regex = Regex::new(r"^\s*✕\s+(.+?)$").unwrap();
            static ref CYPRESS_PENDING_RE: Regex = Regex::new(r"^\s*-\s+(.+?)(?:\s+\(pending\))?$").unwrap();
            static ref CYPRESS_SPEC_RE: Regex = Regex::new(r"Running:\s+(.+\.cy\.[jt]s)").unwrap();
        }

        let mut current_spec = String::new();
        let lines: Vec<&str> = log.lines().collect();

        for line in lines {
            // Extract spec file name
            if let Some(captures) = CYPRESS_SPEC_RE.captures(line) {
                current_spec = captures.get(1).unwrap().as_str().to_string();
                continue;
            }

            // Parse test results
            if let Some(captures) = CYPRESS_PASS_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = format!("{}::{}", current_spec, test_name);
                test_status_map.insert(full_name, TestStatus::Passed);
            } else if let Some(captures) = CYPRESS_FAIL_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = format!("{}::{}", current_spec, test_name);
                test_status_map.insert(full_name, TestStatus::Failed);
            } else if let Some(captures) = CYPRESS_PENDING_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = format!("{}::{}", current_spec, test_name);
                test_status_map.insert(full_name, TestStatus::Pending);
            }
        }

        test_status_map
    }

    fn parse_log_playwright(&self, log: &str) -> HashMap<String, TestStatus> {
        let mut test_status_map = HashMap::new();

        lazy_static! {
            static ref PLAYWRIGHT_PASS_RE: Regex = Regex::new(r"^\s*✓\s+(.+?)\s+\[.+?\]\s+\(\d+ms\)").unwrap();
            static ref PLAYWRIGHT_FAIL_RE: Regex = Regex::new(r"^\s*✗\s+(.+?)\s+\[.+?\]\s+\(\d+ms\)").unwrap();
            static ref PLAYWRIGHT_SKIP_RE: Regex = Regex::new(r"^\s*-\s+(.+?)\s+\[.+?\]").unwrap();
            static ref PLAYWRIGHT_SPEC_RE: Regex = Regex::new(r"^\s*(.+?\.spec\.[jt]s)").unwrap();
        }

        let mut current_spec = String::new();
        let lines: Vec<&str> = log.lines().collect();

        for line in lines {
            // Extract spec file name
            if let Some(captures) = PLAYWRIGHT_SPEC_RE.captures(line) {
                let spec_file = captures.get(1).unwrap().as_str();
                if spec_file.contains(".spec.") {
                    current_spec = spec_file.to_string();
                    continue;
                }
            }

            // Parse test results
            if let Some(captures) = PLAYWRIGHT_PASS_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = format!("{}::{}", current_spec, test_name);
                test_status_map.insert(full_name, TestStatus::Passed);
            } else if let Some(captures) = PLAYWRIGHT_FAIL_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = format!("{}::{}", current_spec, test_name);
                test_status_map.insert(full_name, TestStatus::Failed);
            } else if let Some(captures) = PLAYWRIGHT_SKIP_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = format!("{}::{}", current_spec, test_name);
                test_status_map.insert(full_name, TestStatus::Skipped);
            }
        }

        test_status_map
    }

    fn parse_log_jasmine(&self, log: &str) -> HashMap<String, TestStatus> {
        let mut test_status_map = HashMap::new();

        lazy_static! {
            static ref JASMINE_PASS_RE: Regex = Regex::new(r"^\s*✓\s+(.+?)$").unwrap();
            static ref JASMINE_FAIL_RE: Regex = Regex::new(r"^\s*✗\s+(.+?)$").unwrap();
            static ref JASMINE_PENDING_RE: Regex = Regex::new(r"^\s*\*\s+(.+?)$").unwrap();
        }

        let mut current_suite = String::new();
        let lines: Vec<&str> = log.lines().collect();

        for line in lines {
            let trimmed = line.trim();
            
            if trimmed.is_empty() || trimmed.starts_with("Jasmine") || 
               trimmed.starts_with("Finished in") || trimmed.contains(" spec") {
                continue;
            }

            if let Some(captures) = JASMINE_PASS_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_suite.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_suite, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Passed);
            } else if let Some(captures) = JASMINE_FAIL_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_suite.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_suite, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Failed);
            } else if let Some(captures) = JASMINE_PENDING_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_suite.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_suite, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Pending);
            } else if !trimmed.starts_with("✓") && !trimmed.starts_with("✗") && 
                     !trimmed.starts_with("*") && !trimmed.contains("failures") && 
                     !line.starts_with("  ") {
                // Potential suite name - lines that don't start with whitespace and aren't test results
                current_suite = trimmed.to_string();
            }
        }

        test_status_map
    }

    fn parse_log_qunit(&self, log: &str) -> HashMap<String, TestStatus> {
        let mut test_status_map = HashMap::new();

        lazy_static! {
            static ref QUNIT_PASS_RE: Regex = Regex::new(r"^\s*✓\s+(.+?)(?:\s+\(\d+ms\))?$").unwrap();
            static ref QUNIT_FAIL_RE: Regex = Regex::new(r"^\s*✗\s+(.+?)(?:\s+\(\d+ms\))?$").unwrap();
            static ref QUNIT_SKIP_RE: Regex = Regex::new(r"^\s*-\s+(.+?)\s+\(skipped\)$").unwrap();
            static ref QUNIT_MODULE_RE: Regex = Regex::new(r"^# (.+?)$").unwrap();
        }

        let mut current_module = String::new();
        let lines: Vec<&str> = log.lines().collect();

        for line in lines {
            // Extract module name
            if let Some(captures) = QUNIT_MODULE_RE.captures(line) {
                current_module = captures.get(1).unwrap().as_str().to_string();
                continue;
            }

            // Parse test results
            if let Some(captures) = QUNIT_PASS_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_module.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_module, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Passed);
            } else if let Some(captures) = QUNIT_FAIL_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_module.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_module, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Failed);
            } else if let Some(captures) = QUNIT_SKIP_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_module.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_module, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Skipped);
            }
        }

        test_status_map
    }

    fn parse_log_ava(&self, log: &str) -> HashMap<String, TestStatus> {
        let mut test_status_map = HashMap::new();

        lazy_static! {
            static ref AVA_PASS_RE: Regex = Regex::new(r"^\s*✔\s+(.+?)(?:\s+\(\d+ms\))?$").unwrap();
            static ref AVA_FAIL_RE: Regex = Regex::new(r"^\s*✖\s+(.+?)(?:\s+\(\d+ms\))?$").unwrap();
            static ref AVA_SKIP_RE: Regex = Regex::new(r"^\s*-\s+(.+?)\s+\[skip\]$").unwrap();
            static ref AVA_FILE_RE: Regex = Regex::new(r"^\s*(.+?\.(?:test|spec)\.[jt]s)$").unwrap();
        }

        let mut current_file = String::new();
        let lines: Vec<&str> = log.lines().collect();

        for line in lines {
            // Extract test file name
            if let Some(captures) = AVA_FILE_RE.captures(line) {
                current_file = captures.get(1).unwrap().as_str().to_string();
                continue;
            }

            // Parse test results
            if let Some(captures) = AVA_PASS_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_file.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_file, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Passed);
            } else if let Some(captures) = AVA_FAIL_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_file.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_file, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Failed);
            } else if let Some(captures) = AVA_SKIP_RE.captures(line) {
                let test_name = captures.get(1).unwrap().as_str().trim();
                let full_name = if current_file.is_empty() {
                    test_name.to_string()
                } else {
                    format!("{}::{}", current_file, test_name)
                };
                test_status_map.insert(full_name, TestStatus::Skipped);
            }
        }

        test_status_map
    }

    fn get_test_name(&self, suite: &[(String, usize)], test_name: &str) -> String {
        let suite_names: Vec<String> = suite.iter().map(|(name, _)| name.clone()).collect();
        if suite_names.is_empty() {
            test_name.trim().to_string()
        } else {
            format!("{} - {}", suite_names.join(" - "), test_name.trim())
        }
    }

    fn get_parser_by_name(&self, name: &str) -> fn(&JavaScriptLogParser, &str) -> HashMap<String, TestStatus> {
        match name {
            "calypso" => JavaScriptLogParser::parse_log_calypso,
            "mocha" => JavaScriptLogParser::parse_log_mocha_v2,
            "jest" => JavaScriptLogParser::parse_log_jest,
            "jest-json" => JavaScriptLogParser::parse_log_jest_json,
            "vitest" => JavaScriptLogParser::parse_log_vitest,
            "karma" => JavaScriptLogParser::parse_log_karma,
            "tap" => JavaScriptLogParser::parse_log_tap,
            "chartjs" => JavaScriptLogParser::parse_log_chart_js,
            "marked" => JavaScriptLogParser::parse_log_marked,
            "react-pdf" => JavaScriptLogParser::parse_log_react_pdf,
            "p5js" => JavaScriptLogParser::parse_log_p5js,
            "cypress" => JavaScriptLogParser::parse_log_cypress,
            "playwright" => JavaScriptLogParser::parse_log_playwright,
            "jasmine" => JavaScriptLogParser::parse_log_jasmine,
            "qunit" => JavaScriptLogParser::parse_log_qunit,
            "ava" => JavaScriptLogParser::parse_log_ava,
            _ => JavaScriptLogParser::parse_log_vitest, // Default to vitest
        }
    }

    // Helper to strip pseudo-ANSI codes like [31m, [39m that appear as plain text
    fn strip_bracket_codes(text: &str) -> String {
        lazy_static! {
            static ref BRACKET_CODE_RE: Regex = Regex::new(r"\[(\d+;?)+m").unwrap();
        }
        BRACKET_CODE_RE.replace_all(text, "").to_string()
    }

    pub fn detect_test_framework(&self, log_content: &str) -> String {
        // If we have a project path (rare case), use config-based detection
        if let Some(ref project_path) = self.project_path {
            let detected = detect_js_testing_framework(project_path);
            return detected;
        }

        // Strip ANSI codes and bracket-style codes before detection
        lazy_static! {
            static ref ANSI_RE: Regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
        }
        let cleaned_log = ANSI_RE.replace_all(log_content, "");
        let cleaned_log = Self::strip_bracket_codes(&cleaned_log);

        // Primary method: Analyze log content patterns to detect framework
        // Order matters - more specific patterns first
        
        // Vitest detection FIRST - check for command and header
        if cleaned_log.contains("vitest run") || cleaned_log.contains("RUN  v") {
            return "vitest".to_string();
        }
        
        if cleaned_log.contains("Running:") && cleaned_log.contains(".cy.") {
            "cypress".to_string()
        } else if cleaned_log.contains("[chromium]") || cleaned_log.contains("[firefox]") || cleaned_log.contains("[webkit]") {
            "playwright".to_string()
        } else if cleaned_log.contains("./node_modules/.bin/jest") || cleaned_log.contains("Test Suites:") {
            "jest".to_string()
        } else if cleaned_log.contains("Jasmine") || (cleaned_log.contains("spec") && cleaned_log.contains("Finished in")) {
            "jasmine".to_string()
        } else if cleaned_log.contains("QUnit") || (cleaned_log.contains("# ") && cleaned_log.contains("✓") && cleaned_log.contains("✗")) {
            "qunit".to_string()
        } else if cleaned_log.contains("✔") && cleaned_log.contains("✖") {
            "ava".to_string()
        } else if cleaned_log.contains("mocha") || (cleaned_log.contains("passing") && cleaned_log.contains("failing")) {
            "mocha".to_string()
        } else if (cleaned_log.contains("✓") || cleaned_log.contains("×") || cleaned_log.contains("↓")) && (cleaned_log.contains(" > ") || cleaned_log.contains("packages/")) {
            "vitest".to_string()
        } else if cleaned_log.contains("Starting browser") || cleaned_log.contains("SUMMARY:") {
            "karma".to_string()
        } else if cleaned_log.contains("ok ") && cleaned_log.contains("not ok ") {
            "tap".to_string()
        } else {
            "vitest".to_string() // Default fallback
        }
    }

    fn convert_to_parsed_log(&self, test_status_map: HashMap<String, TestStatus>) -> ParsedLog {
        let mut parsed_log = ParsedLog::new();

        for (test_name, status) in test_status_map {
            match status {
                TestStatus::Passed => {
                    parsed_log.passed.insert(test_name);
                }
                TestStatus::Failed => {
                    parsed_log.failed.insert(test_name);
                }
                TestStatus::Skipped | TestStatus::Pending => {
                    parsed_log.ignored.insert(test_name);
                }
            }
        }

        parsed_log.finalize();
        parsed_log
    }
}

impl LogParserTrait for JavaScriptLogParser {
    fn parse_log_file(&self, file_path: &str) -> Result<ParsedLog, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;

        // Try to extract project path from file path
        let project_path = if self.project_path.is_some() {
            self.project_path.clone()
        } else {
            // Try to find project root by looking for package.json
            Path::new(file_path)
                .parent()
                .and_then(|p| {
                    // Look up the directory tree for package.json
                    let mut current = p;
                    loop {
                        if current.join("package.json").exists() {
                            return Some(current.to_string_lossy().to_string());
                        }
                        if let Some(parent) = current.parent() {
                            current = parent;
                        } else {
                            break;
                        }
                    }
                    None
                })
        };

        // Auto-detect framework if parser name is not specifically set
        let framework = if self.parser_name == "auto" {
            if let Some(ref proj_path) = project_path {
                detect_js_testing_framework(proj_path)
            } else {
                self.detect_test_framework(&content)
            }
        } else {
            self.parser_name.clone()
        };

        eprintln!("DEBUG: Detected framework '{}' for file: {}", framework, file_path);
        eprintln!("DEBUG: Content preview (first 500 chars): {}", &content[..content.len().min(500)]);

        let test_status_map = match framework.as_str() {
            "calypso" => self.parse_log_calypso(&content),
            "mocha" => self.parse_log_mocha_v2(&content),
            "jest" => self.parse_log_jest(&content),
            "jest-json" => self.parse_log_jest_json(&content),
            "vitest" => self.parse_log_vitest(&content),
            "karma" => self.parse_log_karma(&content),
            "tap" => self.parse_log_tap(&content),
            "chartjs" => self.parse_log_chart_js(&content),
            "marked" => self.parse_log_marked(&content),
            "react-pdf" => self.parse_log_react_pdf(&content),
            "p5js" => self.parse_log_p5js(&content),
            "cypress" => self.parse_log_cypress(&content),
            "playwright" => self.parse_log_playwright(&content),
            "jasmine" => self.parse_log_jasmine(&content),
            "qunit" => self.parse_log_qunit(&content),
            "ava" => self.parse_log_ava(&content),
            _ => self.parse_log_vitest(&content), // Default fallback
        };

        Ok(self.convert_to_parsed_log(test_status_map))
    }

    fn get_language(&self) -> &'static str {
        "javascript"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jest_parsing() {
        let log = r#"
✓ should pass test 1
✕ should fail test 2
○ should skip test 3
        "#;

        let parser = JavaScriptLogParser::new_with_parser("jest");
        let result = parser.parse_log_jest(log);

        assert_eq!(result.get("should pass test 1"), Some(&TestStatus::Passed));
        assert_eq!(result.get("should fail test 2"), Some(&TestStatus::Failed));
        assert_eq!(result.get("should skip test 3"), Some(&TestStatus::Skipped));
    }

    #[test]
    fn test_vitest_parsing() {
        let log = r#"
✓ should pass test 1 100ms
× should fail test 2
↓ should skip test 3 [skipped]
        "#;

        let parser = JavaScriptLogParser::new_with_parser("vitest");
        let result = parser.parse_log_vitest(log);

        assert_eq!(result.get("should pass test 1"), Some(&TestStatus::Passed));
        assert_eq!(result.get("should fail test 2"), Some(&TestStatus::Failed));
        assert_eq!(result.get("should skip test 3"), Some(&TestStatus::Skipped));
    }

    #[test]
    fn test_tap_parsing() {
        let log = r#"
ok 1 should pass test 1
not ok 2 should fail test 2
ok 3 should pass test 3
        "#;

        let parser = JavaScriptLogParser::new_with_parser("tap");
        let result = parser.parse_log_tap(log);

        assert_eq!(result.get("should pass test 1"), Some(&TestStatus::Passed));
        assert_eq!(result.get("should fail test 2"), Some(&TestStatus::Failed));
        assert_eq!(result.get("should pass test 3"), Some(&TestStatus::Passed));
    }

    #[test]
    fn test_framework_detection() {
        let parser = JavaScriptLogParser::new();

        let jest_log = "Test Suites: 1 passed, 1 total";
        assert_eq!(parser.detect_test_framework(jest_log), "jest");

        let vitest_log = "✓ test 1\n× test 2\n↓ test 3";
        assert_eq!(parser.detect_test_framework(vitest_log), "vitest");

        let karma_log = "Starting browser Chrome\n✔ test 1\nSUMMARY:";
        assert_eq!(parser.detect_test_framework(karma_log), "karma");

        let tap_log = "ok 1 test 1\nnot ok 2 test 2";
        assert_eq!(parser.detect_test_framework(tap_log), "tap");
    }

    #[test]
    fn test_cypress_parsing() {
        let log = r#"
Running: cypress/e2e/example.cy.js

    ✓ should display welcome message (450ms)
    ✕ should handle form submission
    - should handle validation (pending)
        "#;

        let parser = JavaScriptLogParser::new_with_parser("cypress");
        let result = parser.parse_log_cypress(log);

        assert_eq!(result.get("cypress/e2e/example.cy.js::should display welcome message"), Some(&TestStatus::Passed));
        assert_eq!(result.get("cypress/e2e/example.cy.js::should handle form submission"), Some(&TestStatus::Failed));
        assert_eq!(result.get("cypress/e2e/example.cy.js::should handle validation"), Some(&TestStatus::Pending));
    }

    #[test]
    fn test_playwright_parsing() {
        let log = r#"
  login.spec.ts

    ✓ should login successfully [chromium] (1234ms)
    ✗ should fail with invalid credentials [chromium] (567ms)
    - should skip this test [chromium]
        "#;

        let parser = JavaScriptLogParser::new_with_parser("playwright");
        let result = parser.parse_log_playwright(log);

        assert_eq!(result.get("login.spec.ts::should login successfully"), Some(&TestStatus::Passed));
        assert_eq!(result.get("login.spec.ts::should fail with invalid credentials"), Some(&TestStatus::Failed));
        assert_eq!(result.get("login.spec.ts::should skip this test"), Some(&TestStatus::Skipped));
    }

    #[test]
    fn test_jasmine_parsing() {
        let log = r#"
User management
  ✓ should create a new user
  ✗ should handle duplicate email
  * should validate user data

Finished in 0.123 seconds
        "#;

        let parser = JavaScriptLogParser::new_with_parser("jasmine");
        let result = parser.parse_log_jasmine(log);

        assert_eq!(result.get("User management::should create a new user"), Some(&TestStatus::Passed));
        assert_eq!(result.get("User management::should handle duplicate email"), Some(&TestStatus::Failed));
        assert_eq!(result.get("User management::should validate user data"), Some(&TestStatus::Pending));
    }

    #[test]
    fn test_qunit_parsing() {
        let log = r#"
# User Module
✓ should create user (12ms)
✗ should validate email (8ms)
- should handle permissions (skipped)

# Admin Module
✓ should have admin access (5ms)
        "#;

        let parser = JavaScriptLogParser::new_with_parser("qunit");
        let result = parser.parse_log_qunit(log);

        assert_eq!(result.get("User Module::should create user"), Some(&TestStatus::Passed));
        assert_eq!(result.get("User Module::should validate email"), Some(&TestStatus::Failed));
        assert_eq!(result.get("User Module::should handle permissions"), Some(&TestStatus::Skipped));
        assert_eq!(result.get("Admin Module::should have admin access"), Some(&TestStatus::Passed));
    }

    #[test]
    fn test_ava_parsing() {
        let log = r#"
  auth.test.js

  ✔ should authenticate user (15ms)
  ✖ should reject invalid token (8ms)
  - should handle expired token [skip]

  user.test.js

  ✔ should create user profile (12ms)
        "#;

        let parser = JavaScriptLogParser::new_with_parser("ava");
        let result = parser.parse_log_ava(log);

        assert_eq!(result.get("auth.test.js::should authenticate user"), Some(&TestStatus::Passed));
        assert_eq!(result.get("auth.test.js::should reject invalid token"), Some(&TestStatus::Failed));
        assert_eq!(result.get("auth.test.js::should handle expired token"), Some(&TestStatus::Skipped));
        assert_eq!(result.get("user.test.js::should create user profile"), Some(&TestStatus::Passed));
    }

    #[test]
    fn test_extended_framework_detection() {
        let parser = JavaScriptLogParser::new();

        let cypress_log = "Running: cypress/e2e/test.cy.js\n✓ should pass";
        assert_eq!(parser.detect_test_framework(cypress_log), "cypress");

        let playwright_log = "login.spec.ts\n✓ test [chromium] (123ms)";
        assert_eq!(parser.detect_test_framework(playwright_log), "playwright");

        let jasmine_log = "Jasmine started\n✓ should pass\nFinished in 0.123 seconds";
        assert_eq!(parser.detect_test_framework(jasmine_log), "jasmine");

        let qunit_log = "QUnit 2.0\n# Module\n✓ test passes";
        assert_eq!(parser.detect_test_framework(qunit_log), "qunit");

        let ava_log = "✔ test passes\n✖ test fails\ntest";
        assert_eq!(parser.detect_test_framework(ava_log), "ava");
    }

    #[test]
    fn test_vitest_parsing_debug() {
        let log_content = r#"  ✓ packages/esbuild-plugin-env/test/test.spec.js > esbuild-plugin-env > should inject env values
  ✓ packages/esbuild-plugin-env/test/test.spec.js > esbuild-plugin-env > should handle missing values
  ✓ packages/esbuild-plugin-env/test/test.spec.js > esbuild-plugin-env > should handle invalid identifiers
  ✓ packages/esbuild-plugin-env/test/test.spec.js > esbuild-plugin-env > should skip injection for node plaform"#;

        let parser = JavaScriptLogParser::new();
        let results = parser.parse_log_vitest(log_content);
        
        println!("Extracted {} tests:", results.len());
        for (test_name, status) in &results {
            println!("  '{}' -> {:?}", test_name, status);
        }
        
        // Check specific expected tests
        assert!(results.len() > 0, "Should have extracted some tests");
        assert!(results.contains_key("esbuild-plugin-env > should inject env values"), "Should extract test without file path");
        assert!(results.contains_key("esbuild-plugin-env > should handle missing values"), "Should extract second test");
        
        // Verify all tests are marked as passed
        for (_, status) in &results {
            assert_eq!(*status, TestStatus::Passed, "All tests should be marked as passed");
        }
    }
}
