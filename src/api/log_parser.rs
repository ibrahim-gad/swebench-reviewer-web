use std::collections::HashMap;
use std::fs;

use lazy_static::lazy_static;
use regex::Regex;

use crate::api::rust_log_parser::RustLogParser;
use crate::api::python_log_parser::PythonLogParser;
use crate::api::javascript_log_parser::JavaScriptLogParser;
use crate::api::test_detection;
use crate::app::types::{StageStatusSummary, GroupedTestStatuses, LogAnalysisResult, RuleViolations, RuleViolation, DebugInfo, LogCount};



// Trait for language-specific log parsers
pub trait LogParserTrait {
    fn parse_log_file(&self, file_path: &str) -> Result<ParsedLog, String>;
    fn get_language(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ParsedLog {
    pub passed: std::collections::HashSet<String>,
    pub failed: std::collections::HashSet<String>,
    pub ignored: std::collections::HashSet<String>,
    pub all: std::collections::HashSet<String>,
}

impl ParsedLog {
    pub fn new() -> Self {
        Self {
            passed: std::collections::HashSet::new(),
            failed: std::collections::HashSet::new(),
            ignored: std::collections::HashSet::new(),
            all: std::collections::HashSet::new(),
        }
    }

    pub fn finalize(&mut self) {
        self.all.extend(self.passed.iter().cloned());
        self.all.extend(self.failed.iter().cloned());
        self.all.extend(self.ignored.iter().cloned());
    }
}

// Main log checker that coordinates between different language parsers
pub struct LogParser {
    parsers: HashMap<String, Box<dyn LogParserTrait + Send + Sync>>,
}

impl LogParser {
    pub fn new() -> Self {
        let mut parsers: HashMap<String, Box<dyn LogParserTrait + Send + Sync>> = HashMap::new();
        
        // Register Rust parser
        parsers.insert("rust".to_string(), Box::new(RustLogParser::new()));
        
        // Register Python parser
        parsers.insert("python".to_string(), Box::new(PythonLogParser::new()));
        
        // Register JavaScript/TypeScript parsers
        parsers.insert("javascript".to_string(), Box::new(JavaScriptLogParser::new()));
        parsers.insert("typescript".to_string(), Box::new(JavaScriptLogParser::new()));
        parsers.insert("js".to_string(), Box::new(JavaScriptLogParser::new()));
        parsers.insert("ts".to_string(), Box::new(JavaScriptLogParser::new()));
        
        Self { parsers }
    }

    pub fn analyze_logs(
        &self,
        file_paths: &[String],
        language: &str,
        fail_to_pass_tests: &[String],
        pass_to_pass_tests: &[String],
    ) -> Result<LogAnalysisResult, String> {
        println!("=== LOG CHECKER DEBUG ===");
        println!("Language: {}", language);
        println!("File paths provided: {:?}", file_paths);
        println!("Fail to pass tests: {} tests", fail_to_pass_tests.len());
        println!("Pass to pass tests: {} tests", pass_to_pass_tests.len());
        
        // Get the appropriate parser for the language
        let parser = self.parsers.get(language)
            .ok_or_else(|| format!("No parser available for language: {}", language))?;

        // Find log files
        let base_log = file_paths.iter().find(|path| path.to_lowercase().contains("base.log"));
        let before_log = file_paths.iter().find(|path| path.to_lowercase().contains("before.log"));
        let after_log = file_paths.iter().find(|path| path.to_lowercase().contains("after.log"));
        let agent_log = file_paths.iter().find(|path| 
            path.to_lowercase().contains("post_agent_patch.log") || 
            path.to_lowercase().contains("agent.log")
        );

        println!("Found log files:");
        println!("  Base log: {:?}", base_log);
        println!("  Before log: {:?}", before_log);
        println!("  After log: {:?}", after_log);
        println!("  Agent log: {:?}", agent_log);

        if base_log.is_none() || before_log.is_none() || after_log.is_none() {
            return Err("Missing required log files (base.log, before.log, after.log)".to_string());
        }

        // Parse log files
        println!("Parsing log files...");
        let base_parsed = parser.parse_log_file(base_log.unwrap())?;
        println!("Base log parsed: {} passed, {} failed, {} ignored, {} total", 
                 base_parsed.passed.len(), base_parsed.failed.len(), 
                 base_parsed.ignored.len(), base_parsed.all.len());
        
        let before_parsed = parser.parse_log_file(before_log.unwrap())?;
        println!("Before log parsed: {} passed, {} failed, {} ignored, {} total", 
                 before_parsed.passed.len(), before_parsed.failed.len(), 
                 before_parsed.ignored.len(), before_parsed.all.len());
        
        let after_parsed = parser.parse_log_file(after_log.unwrap())?;
        println!("After log parsed: {} passed, {} failed, {} ignored, {} total", 
                 after_parsed.passed.len(), after_parsed.failed.len(), 
                 after_parsed.ignored.len(), after_parsed.all.len());
        
        let agent_parsed = if let Some(agent_path) = agent_log {
            let parsed = parser.parse_log_file(agent_path)?;
            println!("Agent log parsed: {} passed, {} failed, {} ignored, {} total", 
                     parsed.passed.len(), parsed.failed.len(), 
                     parsed.ignored.len(), parsed.all.len());
            Some(parsed)
        } else {
            println!("No agent log found");
            None
        };

        // Find and parse report.json if available
        let report_data = self.find_and_parse_report(file_paths)?;

        // Generate analysis result
        let analysis_result = self.generate_analysis_result(
            &base_parsed,
            &before_parsed,
            &after_parsed,
            agent_parsed.as_ref(),
            fail_to_pass_tests,
            pass_to_pass_tests,
            base_log.unwrap(),
            before_log.unwrap(),
            after_log.unwrap(),
            report_data.as_ref(),
            file_paths,
            language,
        );

        Ok(analysis_result)
    }

    fn find_and_parse_report(&self, file_paths: &[String]) -> Result<Option<serde_json::Value>, String> {
        let report_json_path = file_paths.iter().find(|path| 
            path.to_lowercase().contains("results/report.json") || 
            path.to_lowercase().ends_with("report.json")
        );

        if let Some(report_path) = report_json_path {
            match fs::read_to_string(report_path) {
                Ok(content) => {
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(json) => Ok(Some(json)),
                        Err(e) => {
                            eprintln!("Failed to parse report.json: {}", e);
                            Ok(None)
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read report.json: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn generate_analysis_result(
        &self,
        base_parsed: &ParsedLog,
        before_parsed: &ParsedLog,
        after_parsed: &ParsedLog,
        agent_parsed: Option<&ParsedLog>,
        fail_to_pass_tests: &[String],
        pass_to_pass_tests: &[String],
        base_path: &str,
        before_path: &str,
        after_path: &str,
        report_data: Option<&serde_json::Value>,
        file_paths: &[String],
        language: &str,
    ) -> LogAnalysisResult {
        let universe: Vec<String> = pass_to_pass_tests.iter()
            .chain(fail_to_pass_tests.iter())
            .cloned()
            .collect();

        let base_s = self.status_lookup(&universe, base_parsed);
        let before_s = self.status_lookup(&universe, before_parsed);
        let after_s = self.status_lookup(&universe, after_parsed);
        let agent_s = if let Some(agent_parsed) = agent_parsed {
            self.status_lookup(&universe, agent_parsed)
        } else {
            HashMap::new()
        };

        let report_s = if let Some(report_data) = report_data {
            self.report_status_lookup(&universe, report_data)
        } else {
            HashMap::new()
        };

        // Rule checks
        let (rule_violations, dup_map) = self.perform_rule_checks(
            &base_s, &before_s, &after_s, &agent_s, &report_s,
            fail_to_pass_tests, pass_to_pass_tests,
            base_path, before_path, after_path, file_paths,
            report_data, language
        );

        // Build grouped test statuses structure
        let mut f2p: HashMap<String, StageStatusSummary> = HashMap::new();
        let mut p2p: HashMap<String, StageStatusSummary> = HashMap::new();

        for test_name in fail_to_pass_tests {
            let summary = StageStatusSummary {
                base: base_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                before: before_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                after: after_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                agent: agent_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                report: report_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
            };
            f2p.insert(test_name.clone(), summary);
        }

        for test_name in pass_to_pass_tests {
            let summary = StageStatusSummary {
                base: base_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                before: before_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                after: after_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                agent: agent_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                report: report_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
            };
            p2p.insert(test_name.clone(), summary);
        }

        // Debug info with all stages
        let mut log_counts = vec![
            LogCount {
                label: "base".to_string(),
                passed: base_parsed.passed.len(),
                failed: base_parsed.failed.len(),
                ignored: base_parsed.ignored.len(),
                all: base_parsed.all.len(),
            },
            LogCount {
                label: "before".to_string(),
                passed: before_parsed.passed.len(),
                failed: before_parsed.failed.len(),
                ignored: before_parsed.ignored.len(),
                all: before_parsed.all.len(),
            },
            LogCount {
                label: "after".to_string(),
                passed: after_parsed.passed.len(),
                failed: after_parsed.failed.len(),
                ignored: after_parsed.ignored.len(),
                all: after_parsed.all.len(),
            },
        ];
        
        // Add agent log count if available
        if let Some(agent_parsed) = agent_parsed {
            log_counts.push(LogCount {
                label: "agent".to_string(),
                passed: agent_parsed.passed.len(),
                failed: agent_parsed.failed.len(),
                ignored: agent_parsed.ignored.len(),
                all: agent_parsed.all.len(),
            });
        }

        let debug_info = DebugInfo {
            log_counts,
            duplicate_examples_per_log: dup_map,
        };

        LogAnalysisResult {
            test_statuses: GroupedTestStatuses { f2p, p2p },
            rule_violations,
            debug_info,
        }
    }

    fn status_lookup(&self, names: &[String], parsed: &ParsedLog) -> HashMap<String, String> {
        let mut out = HashMap::new();
        
        println!("=== STATUS LOOKUP DEBUG ===");
        println!("Expected test names ({} total):", names.len());
        for (i, name) in names.iter().enumerate() {
            println!("  {}: '{}'", i + 1, name);
            if i >= 4 { 
                println!("  ... and {} more", names.len() - 5);
                break; 
            }
        }
        
        println!("Parsed test results:");
        println!("  Passed ({} total):", parsed.passed.len());
        for (i, name) in parsed.passed.iter().enumerate() {
            println!("    {}: '{}'", i + 1, name);
            if i >= 2 { 
                println!("    ... and {} more", parsed.passed.len() - 3);
                break; 
            }
        }
        println!("  Failed ({} total):", parsed.failed.len());
        for (i, name) in parsed.failed.iter().enumerate() {
            println!("    {}: '{}'", i + 1, name);
            if i >= 2 { 
                println!("    ... and {} more", parsed.failed.len() - 3);
                break; 
            }
        }
        println!("  Ignored ({} total):", parsed.ignored.len());
        for (i, name) in parsed.ignored.iter().enumerate() {
            println!("    {}: '{}'", i + 1, name);
            if i >= 2 { 
                println!("    ... and {} more", parsed.ignored.len() - 3);
                break; 
            }
        }
        
        for name in names {
            if parsed.failed.contains(name) {
                println!("MATCH: '{}' found in FAILED", name);
                out.insert(name.clone(), "failed".to_string());
            } else if parsed.passed.contains(name) {
                println!("MATCH: '{}' found in PASSED", name);
                out.insert(name.clone(), "passed".to_string());
            } else if parsed.ignored.contains(name) {
                println!("MATCH: '{}' found in IGNORED", name);
                out.insert(name.clone(), "ignored".to_string());
            } else {
                println!("NO MATCH: '{}' not found in any category, marking as MISSING", name);
                out.insert(name.clone(), "missing".to_string());
            }
        }
        println!("=============================");
        out
    }

    fn report_status_lookup(&self, names: &[String], report_data: &serde_json::Value) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let mut report_failed_tests = std::collections::HashSet::new();
        let mut report_passed_tests = std::collections::HashSet::new();
        
        // Parse report.json to extract test results using the same logic as C6 check
        // Try different possible structures for report.json
        if let Some(results_array) = report_data.get("results").and_then(|r| r.as_array()) {
            for result in results_array {
                if let (Some(test_name), Some(status)) = (result.get("test_name").and_then(|t| t.as_str()), result.get("status").and_then(|s| s.as_str())) {
                    match status.to_lowercase().as_str() {
                        "failed" | "fail" => { report_failed_tests.insert(test_name.to_string()); }
                        "passed" | "pass" | "success" => { report_passed_tests.insert(test_name.to_string()); }
                        _ => {}
                    }
                }
            }
        } else if let Some(test_results) = report_data.get("test_results").and_then(|r| r.as_array()) {
            for result in test_results {
                if let (Some(test_name), Some(status)) = (result.get("test_name").and_then(|t| t.as_str()), result.get("status").and_then(|s| s.as_str())) {
                    match status.to_lowercase().as_str() {
                        "failed" | "fail" => { report_failed_tests.insert(test_name.to_string()); }
                        "passed" | "pass" | "success" => { report_passed_tests.insert(test_name.to_string()); }
                        _ => {}
                    }
                }
            }
        } else if let Some(tests_obj) = report_data.get("tests").and_then(|t| t.as_object()) {
            // Format: {"tests": {"test_name": {"status": "failed"}}}
            for (test_name, test_data) in tests_obj {
                if let Some(status) = test_data.get("status").and_then(|s| s.as_str()) {
                    match status.to_lowercase().as_str() {
                        "failed" | "fail" => { report_failed_tests.insert(test_name.clone()); }
                        "passed" | "pass" | "success" => { report_passed_tests.insert(test_name.clone()); }
                        _ => {}
                    }
                }
            }
        } else if let Some(obj) = report_data.as_object() {
            // Check for SWE-bench format first
            let mut found_swe_format = false;
            for (_key, value) in obj {
                if let Some(tests_status) = value.get("tests_status").and_then(|t| t.as_object()) {
                    found_swe_format = true;
                    
                    // Parse all test categories
                    for (_category, category_data) in tests_status {
                        if let Some(category_obj) = category_data.as_object() {
                            // Extract failed tests from "failure" arrays
                            if let Some(failure_array) = category_obj.get("failure").and_then(|f| f.as_array()) {
                                for test_item in failure_array {
                                    if let Some(test_name) = test_item.as_str() {
                                        report_failed_tests.insert(test_name.to_string());
                                    }
                                }
                            }
                            // Extract passed tests from "success" arrays
                            if let Some(success_array) = category_obj.get("success").and_then(|f| f.as_array()) {
                                for test_item in success_array {
                                    if let Some(test_name) = test_item.as_str() {
                                        report_passed_tests.insert(test_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                    break; // Found SWE-bench format, no need to check other keys
                }
            }
            
            // If not SWE-bench format, try direct mapping format: {"test_name": "status"}
            if !found_swe_format {
                for (test_name, status_val) in obj {
                    if let Some(status) = status_val.as_str() {
                        match status.to_lowercase().as_str() {
                            "failed" | "fail" => { report_failed_tests.insert(test_name.clone()); }
                            "passed" | "pass" | "success" => { report_passed_tests.insert(test_name.clone()); }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // Map test names to their status
        for name in names {
            if report_failed_tests.contains(name) {
                out.insert(name.clone(), "failed".to_string());
            } else if report_passed_tests.contains(name) {
                out.insert(name.clone(), "passed".to_string());
            } else {
                out.insert(name.clone(), "missing".to_string());
            }
        }
        
        out
    }

    fn perform_rule_checks(
        &self,
        base_s: &HashMap<String, String>,
        before_s: &HashMap<String, String>,
        after_s: &HashMap<String, String>,
        agent_s: &HashMap<String, String>,
        report_s: &HashMap<String, String>,
        fail_to_pass_tests: &[String],
        pass_to_pass_tests: &[String],
        base_path: &str,
        before_path: &str,
        after_path: &str,
        file_paths: &[String],
        report_data: Option<&serde_json::Value>,
        language: &str,
    ) -> (RuleViolations, HashMap<String, Vec<String>>) {
        println!("Performing rule checks...");
        
        // C1: P2P tests that are failed in base
        let c1_hits: Vec<String> = pass_to_pass_tests.iter()
            .filter(|t| base_s.get(*t) == Some(&"failed".to_string()))
            .cloned()
            .collect();
        let c1 = !c1_hits.is_empty();
        println!("C1 check: {} violations", c1_hits.len());

        // C2: Any test that failed in after (not: "not passed")
        let c2_hits: Vec<String> = fail_to_pass_tests.iter()
            .chain(pass_to_pass_tests.iter())
            .filter(|t| after_s.get(*t) == Some(&"failed".to_string()))
            .cloned()
            .collect();
        let c2 = !c2_hits.is_empty();
        println!("C2 check: {} violations", c2_hits.len());

        // C3: F2P tests that are successful in before
        let c3_hits: Vec<String> = fail_to_pass_tests.iter()
            .filter(|t| before_s.get(*t) == Some(&"passed".to_string()))
            .cloned()
            .collect();
        let c3 = !c3_hits.is_empty();
        println!("C3 check: {} violations", c3_hits.len());

        // C4: P2P tests missing in base and not passing in before
        // Logic:
        // - If P2P passed in base → Skip (don't check)
        // - If P2P is missing in base → Check before:
        //   - If passing in before → No violation
        //   - If missing or failed in before → Violation
        let mut c4_hits: Vec<String> = vec![];
        for t in pass_to_pass_tests {
            let b = base_s.get(t).map(String::as_str).unwrap_or("missing");
            let be = before_s.get(t).map(String::as_str).unwrap_or("missing");
            
            // If P2P passed in base, skip this test (no need to check before)
            if b == "passed" {
                continue;
            }
            
            // If P2P is missing in base, check it in before
            if b == "missing" {
                // If P2P is NOT passing in before (missing or failed), it's a violation
                if be != "passed" {
                    c4_hits.push(format!("{t} (missing in base, {be} in before)"));
                }
            }
        }
        let c4 = !c4_hits.is_empty();
        println!("C4 check: {} violations", c4_hits.len());

        // C5: true duplicates per log using enhanced detection
        let mut dup_map = HashMap::new();
        let base_txt = fs::read_to_string(base_path).unwrap_or_default();
        let before_txt = fs::read_to_string(before_path).unwrap_or_default();
        let after_txt = fs::read_to_string(after_path).unwrap_or_default();
        
        let base_dups = detect_same_file_duplicates(&base_txt);
        let before_dups = detect_same_file_duplicates(&before_txt);
        let after_dups = detect_same_file_duplicates(&after_txt);
        
        if !base_dups.is_empty() {
            dup_map.insert("base".to_string(), base_dups.into_iter().take(50).collect::<Vec<_>>());
        }
        if !before_dups.is_empty() {
            dup_map.insert("before".to_string(), before_dups.into_iter().take(50).collect::<Vec<_>>());
        }
        if !after_dups.is_empty() {
            dup_map.insert("after".to_string(), after_dups.into_iter().take(50).collect::<Vec<_>>());
        }
        let c5 = !dup_map.is_empty();
        println!("C5 check: {} logs with duplicates", dup_map.len());

        // C6: Test marked as failing in report.json but passing in post_agent_log
        // This checks for inconsistencies between report.json and agent log results
        let mut c6_hits: Vec<String> = vec![];
        let c6 = match report_data {
            Some(report_data_ref) => {
                println!("Performing C6 check: comparing report.json with agent log results");
                
                // Parse report.json to extract test results
                let mut report_failed_tests = std::collections::HashSet::new();
                
                // Try different possible structures for report.json
                if let Some(results_array) = report_data_ref.get("results").and_then(|r| r.as_array()) {
                    for result in results_array {
                        if let (Some(test_name), Some(status)) = (result.get("test_name").and_then(|t| t.as_str()), result.get("status").and_then(|s| s.as_str())) {
                            if status.to_lowercase() == "failed" || status.to_lowercase() == "fail" {
                                report_failed_tests.insert(test_name.to_string());
                            }
                        }
                    }
                } else if let Some(test_results) = report_data_ref.get("test_results").and_then(|r| r.as_array()) {
                    for result in test_results {
                        if let (Some(test_name), Some(status)) = (result.get("test_name").and_then(|t| t.as_str()), result.get("status").and_then(|s| s.as_str())) {
                            if status.to_lowercase() == "failed" || status.to_lowercase() == "fail" {
                                report_failed_tests.insert(test_name.to_string());
                            }
                        }
                    }
                } else if let Some(tests_obj) = report_data_ref.get("tests").and_then(|t| t.as_object()) {
                    // Format: {"tests": {"test_name": {"status": "failed"}}}
                    for (test_name, test_data) in tests_obj {
                        if let Some(status) = test_data.get("status").and_then(|s| s.as_str()) {
                            if status.to_lowercase() == "failed" || status.to_lowercase() == "fail" {
                                report_failed_tests.insert(test_name.clone());
                            }
                        }
                    }
                } else if let Some(obj) = report_data_ref.as_object() {
                    // Check for SWE-bench format first
                    let mut found_swe_format = false;
                    for (key, value) in obj {
                        if let Some(tests_status) = value.get("tests_status").and_then(|t| t.as_object()) {
                            println!("Found SWE-bench format report.json for key: {}", key);
                            found_swe_format = true;
                            
                            // Parse all test categories that indicate failure
                            for (category, category_data) in tests_status {
                                if let Some(category_obj) = category_data.as_object() {
                                    // Extract failed tests from "failure" arrays in all categories
                                    if let Some(failure_array) = category_obj.get("failure").and_then(|f| f.as_array()) {
                                        for test_item in failure_array {
                                            if let Some(test_name) = test_item.as_str() {
                                                report_failed_tests.insert(test_name.to_string());
                                                println!("Found failed test in category {}: {}", category, test_name);
                                            }
                                        }
                                    }
                                }
                            }
                            break; // Found SWE-bench format, no need to check other keys
                        }
                    }
                    
                    // If not SWE-bench format, try direct mapping format: {"test_name": "status"}
                    if !found_swe_format {
                        for (test_name, status_val) in obj {
                            if let Some(status) = status_val.as_str() {
                                if status.to_lowercase() == "failed" || status.to_lowercase() == "fail" {
                                    report_failed_tests.insert(test_name.clone());
                                }
                            }
                        }
                    }
                }
                
                println!("Found {} failed tests in report.json", report_failed_tests.len());
                
                // Check F2P and P2P tests for inconsistencies in both directions
                let mut inconsistencies = 0;
                for test_name in fail_to_pass_tests.iter().chain(pass_to_pass_tests.iter()) {
                    let report_status = if report_failed_tests.contains(test_name) {
                        "failed"
                    } else if report_s.get(test_name) == Some(&"passed".to_string()) {
                        "passed"
                    } else {
                        continue; // Skip tests that are missing in report.json
                    };
                    
                    let agent_status = agent_s.get(test_name).map(String::as_str).unwrap_or("missing");
                    
                    // Check for status mismatches (excluding missing cases)
                    if agent_status != "missing" && report_status != agent_status {
                        match (report_status, agent_status) {
                            ("failed", "passed") => {
                                c6_hits.push(format!("{} (marked as failed in report.json but passing in agent log)", test_name));
                                inconsistencies += 1;
                            },
                            ("passed", "failed") => {
                                c6_hits.push(format!("{} (marked as passed in report.json but failing in agent log)", test_name));
                                inconsistencies += 1;
                            },
                            _ => {} // Other combinations like "passed" vs "ignored" could be added if needed
                        }
                    }
                }
                
                println!("C6 check found {} inconsistencies", inconsistencies);
                inconsistencies > 0
            },
            None => {
                println!("C6 check skipped: no report.json available");
                false
            }
        };
        println!("C6 check: {} violations", c6_hits.len());

        // C7: F2P tests found in golden source diff files but not in test diff files
        let mut c7_hits: Vec<String> = vec![];
        let c7 = {
            println!("Performing C7 check: looking for F2P tests in golden source diff files (but not in test diffs)");
            
            // Find diff/patch files from patches folder
            let diff_files: Vec<&String> = file_paths.iter()
                .filter(|path| {
                    let path_lower = path.to_lowercase();
                    path_lower.contains("patches/") && (path_lower.ends_with(".diff") || path_lower.ends_with(".patch"))
                })
                .collect();
            
            println!("Found {} diff/patch files", diff_files.len());
            
            if !diff_files.is_empty() {
                // Separate golden source diffs from test diffs
                let (golden_source_diffs, test_diffs): (Vec<&String>, Vec<&String>) = diff_files.iter()
                    .partition(|path| {
                        let filename = path.split('/').last().unwrap_or("").to_lowercase();
                        // Golden source diffs typically contain "gold", "golden", "src", "source"
                        // Test diffs typically contain "test"
                        (filename.contains("gold") || filename.contains("src") || filename.contains("source")) &&
                        !filename.contains("test")
                    });
                
                println!("Found {} golden source diff files and {} test diff files", 
                         golden_source_diffs.len(), test_diffs.len());
                
                // Read all test diff contents to check if tests appear there
                let mut test_diff_contents = String::new();
                for test_diff in &test_diffs {
                    if let Ok(content) = fs::read_to_string(test_diff) {
                        test_diff_contents.push_str(&content);
                        test_diff_contents.push('\n');
                        println!("Read test diff file: {}", test_diff);
                    }
                }
                
                // Check golden source diffs for F2P tests
                for golden_diff in &golden_source_diffs {
                    println!("Checking golden source diff file: {}", golden_diff);
                    
                    if let Ok(diff_content) = fs::read_to_string(golden_diff) {
                        println!("Read golden source diff successfully, {} bytes", diff_content.len());
                        
                        // Check if any F2P test names appear in this golden source diff
                        for f2p_test in fail_to_pass_tests {
                            // Extract the actual test name from module path (e.g., "tests::test_example" -> "test_example")
                            let test_name_to_search = if f2p_test.contains("::") {
                                f2p_test.split("::").last().unwrap_or(f2p_test)
                            } else {
                                f2p_test
                            };
                            
                            let test_found_in_source = test_detection::contains_exact_test_name(&diff_content, test_name_to_search, language);
                            
                            if test_found_in_source {
                                // Check if this test also appears in test diffs
                                let test_found_in_test_diffs = if !test_diff_contents.is_empty() {
                                    test_detection::contains_exact_test_name(&test_diff_contents, test_name_to_search, language)
                                } else {
                                    false
                                };
                                
                                if test_found_in_test_diffs {
                                    println!("F2P test '{}' found in both golden source and test diffs - not a violation", f2p_test);
                                } else {
                                    let search_term = if language == "python" { f2p_test } else { test_name_to_search };
                                    let violation = format!("{} (found as '{}' in {} but not in test diffs)", 
                                                          f2p_test, search_term, 
                                                          golden_diff.split('/').last().unwrap_or(golden_diff));
                                    c7_hits.push(violation);
                                    println!("C7 violation: F2P test '{}' found as '{}' in golden source diff '{}' but not in test diffs", 
                                             f2p_test, search_term, golden_diff);
                                }
                            }
                        }
                    } else {
                        println!("Failed to read golden source diff file: {}", golden_diff);
                    }
                }
            } else {
                println!("No diff/patch files found in patches folder");
            }
            
            let has_violations = !c7_hits.is_empty();
            println!("C7 check completed: {} violations found", c7_hits.len());
            has_violations
        };
        println!("C7 check: {} violations", c7_hits.len());

        let rule_violations = RuleViolations {
            c1_failed_in_base_present_in_p2p: RuleViolation {
                has_problem: c1,
                examples: c1_hits,
            },
            c2_failed_in_after_present_in_f2p_or_p2p: RuleViolation {
                has_problem: c2,
                examples: c2_hits,
            },
            c3_f2p_success_in_before: RuleViolation {
                has_problem: c3,
                examples: c3_hits,
            },
            c4_p2p_missing_in_base_and_not_passing_in_before: RuleViolation {
                has_problem: c4,
                examples: c4_hits,
            },
            c5_duplicates_in_same_log: RuleViolation {
                has_problem: c5,
                examples: vec![], 
            },
            c6_test_marked_failed_in_report_but_passing_in_agent: RuleViolation {
                has_problem: c6,
                examples: c6_hits,
            },
            c7_f2p_tests_in_golden_source_diff: RuleViolation {
                has_problem: c7,
                examples: c7_hits,
            },
        };

        (rule_violations, dup_map)
    }
}

// ---------------- Duplicate detection (C5) parity----------------
fn detect_file_boundary(line: &str) -> Option<String> {
    // These patterns are now in RustLogParser, but for duplicate detection we need them here
    lazy_static! {
        static ref FILE_BOUNDARY_RE_1: Regex = Regex::new(r"(?i)Running\s+([^\s]+(?:/[^\s]+)*\.(?:rs|fixed))\s*\(").unwrap();
        static ref FILE_BOUNDARY_RE_2: Regex = Regex::new(r"(?i)===\s*Running\s+(.+\.(?:rs|fixed))").unwrap();
        static ref FILE_BOUNDARY_RE_3: Regex = Regex::new(r"(?i)test\s+result:\s+ok\.\s+\d+\s+passed.*for\s+(.+\.(?:rs|fixed))").unwrap();
    }
    
    if let Some(c) = FILE_BOUNDARY_RE_1.captures(line) {
        return Some(c.get(1).unwrap().as_str().to_string());
    }
    if let Some(c) = FILE_BOUNDARY_RE_2.captures(line) {
        return Some(c.get(1).unwrap().as_str().to_string());
    }
    if let Some(c) = FILE_BOUNDARY_RE_3.captures(line) {
        return Some(c.get(1).unwrap().as_str().to_string());
    }
    None
}

fn extract_test_info_enhanced(line: &str) -> Option<(String, String)> {
    lazy_static! {
        static ref ENH_TEST_RE_1: Regex = Regex::new(r"(?i)\btest\s+([^\s]+(?:::[^\s]+)*)\s*\.{2,}\s*(ok|FAILED|ignored|error)").unwrap();
        static ref ENH_TEST_RE_2: Regex = Regex::new(r"(?i)test\s+([^\s]+)\s+\.\.\.\s+(ok|FAILED|ignored|error)").unwrap();
        static ref UI_TEST_PATH_RE: Regex = Regex::new(r"(?i)^([^\s]+(?:/[^\s]+)*\.(?:rs|fixed|toml|txt|md)(?:\s+\(revision\s+[^)]+\))?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$").unwrap();
        static ref UI_TEST_PATH_SIMPLE_RE: Regex = Regex::new(r"(?i)^([^\s]+(?:/[^\s]+)*\.(?:rs|fixed|toml|txt|md)(?:\s+\(revision\s+[^)]+\))?)\s+\.\.\.\s+(ok|FAILED|ignored|error)\s*$").unwrap();
    }
    
    if let Some(c) = ENH_TEST_RE_1.captures(line) {
        return Some((
            c.get(1).unwrap().as_str().trim().to_string(),
            c.get(2).unwrap().as_str().trim().to_string(),
        ));
    }
    if let Some(c) = ENH_TEST_RE_2.captures(line) {
        return Some((
            c.get(1).unwrap().as_str().trim().to_string(),
            c.get(2).unwrap().as_str().trim().to_string(),
        ));
    }
    
    // Check for UI test format patterns
    if let Some(c) = UI_TEST_PATH_RE.captures(line) {
        return Some((
            c.get(1).unwrap().as_str().trim().to_string(),
            c.get(2).unwrap().as_str().trim().to_string(),
        ));
    }
    if let Some(c) = UI_TEST_PATH_SIMPLE_RE.captures(line) {
        return Some((
            c.get(1).unwrap().as_str().trim().to_string(),
            c.get(2).unwrap().as_str().trim().to_string(),
        ));
    }
    
    None
}

#[derive(Clone)]
struct Occur {
    test_name: String,
    status: String,
    line_no: usize,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

fn is_true_duplicate(occ: &[Occur]) -> bool {
    if occ.len() <= 1 { return false; }
    let mut lines: Vec<usize> = occ.iter().map(|o| o.line_no).collect();
    lines.sort_unstable();
    let mut min_dist = usize::MAX;
    for i in 1..lines.len() {
        min_dist = std::cmp::min(min_dist, lines[i] - lines[i-1]);
    }
    if min_dist < 10 { return true; }
    let mut has_fail = false;
    let mut has_ok = false;
    for o in occ {
        let s = o.status.to_lowercase();
        if s == "failed" || s == "error" { has_fail = true; }
        if s == "ok" { has_ok = true; }
    }
    if has_fail && has_ok { return true; }
    let contexts: Vec<String> = occ.iter().map(|o| {
        let mut c = String::new();
        c.push_str(&o.context_before.join(" "));
        c.push_str(&o.context_after.join(" "));
        c.trim().to_string()
    }).collect();
    if !contexts.is_empty() && contexts.iter().all(|c| !c.is_empty() && *c == contexts[0]) {
        return true;
    }
    false
}

fn detect_same_file_duplicates(raw_content: &str) -> Vec<String> {
    if raw_content.is_empty() { return vec![]; }
    let lines: Vec<&str> = raw_content.split('\n').collect();
    let mut current_file = "unknown".to_string();
    let mut per_file: HashMap<String, Vec<Occur>> = HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        if let Some(f) = detect_file_boundary(line) {
            current_file = f;
            continue;
        }
        if let Some((name, status)) = extract_test_info_enhanced(line) {
            let before = if i >= 2 { lines[i-2..i].iter().map(|s| s.to_string()).collect() } else { vec![] };
            let after = if i+1 < lines.len() { lines[i+1..std::cmp::min(lines.len(), i+3)].iter().map(|s| s.to_string()).collect() } else { vec![] };
            per_file.entry(current_file.clone()).or_default().push(Occur{ test_name: name, status, line_no: i, context_before: before, context_after: after });
        }
    }

    let mut out = vec![];
    let mut by_name: HashMap<String, Vec<Occur>> = HashMap::new();
    for (_file, occs) in per_file {
        for o in occs { by_name.entry(o.test_name.clone()).or_default().push(o); }
    }
    for (name, list) in by_name {
        if list.len() > 1 && is_true_duplicate(&list) {
            let places: Vec<String> = list.iter().map(|o| format!("line {}", o.line_no)).collect();
            out.push(format!("{} (appears {} times: {})", name, places.len(), places.join(", ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_log_analysis_with_sample_data() {
        // Create test log files
        let base_log_content = r#"test single_file_stdin ... ok
test single_empty_file ... FAILED
test multiple_files_stdin ... ok
test complex_test_case ... FAILED
test result: ok. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s"#;

        let before_log_content = r#"test single_file_stdin ... FAILED
test single_empty_file ... FAILED
test multiple_files_stdin ... ok
test complex_test_case ... ok
test result: ok. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s"#;

        let after_log_content = r#"test single_file_stdin ... ok
test single_empty_file ... ok
test multiple_files_stdin ... ok
test complex_test_case ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s"#;

        // Create temporary directory
        let temp_dir = std::env::temp_dir().join("swe_reviewer_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Write test log files
        let base_log_path = temp_dir.join("base.log");
        let before_log_path = temp_dir.join("before.log");
        let after_log_path = temp_dir.join("after.log");
        let main_json_path = temp_dir.join("main.json");

        fs::write(&base_log_path, base_log_content).unwrap();
        fs::write(&before_log_path, before_log_content).unwrap();
        fs::write(&after_log_path, after_log_content).unwrap();

        // Create a simple main.json
        let main_json_content = r#"{
            "fail_to_pass": ["single_file_stdin", "single_empty_file"],
            "pass_to_pass": ["multiple_files_stdin", "complex_test_case"]
        }"#;
        fs::write(&main_json_path, main_json_content).unwrap();

        // Test the log checker
        let log_checker = LogParser::new();
        let file_paths = vec![
            base_log_path.to_string_lossy().to_string(),
            before_log_path.to_string_lossy().to_string(),
            after_log_path.to_string_lossy().to_string(),
            main_json_path.to_string_lossy().to_string(),
        ];

        let fail_to_pass_tests = vec!["single_file_stdin".to_string(), "single_empty_file".to_string()];
        let pass_to_pass_tests = vec!["multiple_files_stdin".to_string(), "complex_test_case".to_string()];

        println!("Testing log analysis with file paths: {:?}", file_paths);
        
        match log_checker.analyze_logs(&file_paths, "rust", &fail_to_pass_tests, &pass_to_pass_tests) {
            Ok(result) => {
                println!("Log analysis successful!");
                let total = result.test_statuses.f2p.len() + result.test_statuses.p2p.len();
                println!("Test statuses: {} tests (grouped)", total);
                assert!(total > 0, "Should have found test statuses");
            },
            Err(e) => {
                panic!("Log analysis failed: {}", e);
            }
        }

        // Clean up
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
