use std::collections::HashMap;
use std::fs;

use crate::api::rust_log_parser::RustLogParser;
use crate::app::types::{TestStatus, LogAnalysisResult, RuleViolations, RuleViolation, DebugInfo, LogCount};

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
            agent_log,
            report_data.as_ref(),
            file_paths,
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
        agent_path: Option<&String>, // TODO: Use in rule checks
        report_data: Option<&serde_json::Value>,
        file_paths: &[String], // TODO: Use in rule checks
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
        let rule_violations = self.perform_rule_checks(
            &base_s, &before_s, &after_s, &agent_s, &report_s,
            fail_to_pass_tests, pass_to_pass_tests,
            base_path, before_path, after_path, file_paths
        );

        // Generate comprehensive test statuses for all stages
        let mut test_statuses = Vec::new();
        
        // Generate test statuses for all stages (base, before, after, agent, report)
        for test_name in fail_to_pass_tests {
            // Base stage
            test_statuses.push(TestStatus {
                test_name: format!("{}_base", test_name),
                status: base_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                r#type: "fail_to_pass".to_string(),
            });
            
            // Before stage
            test_statuses.push(TestStatus {
                test_name: format!("{}_before", test_name),
                status: before_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                r#type: "fail_to_pass".to_string(),
            });
            
            // After stage
            test_statuses.push(TestStatus {
                test_name: format!("{}_after", test_name),
                status: after_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                r#type: "fail_to_pass".to_string(),
            });
            
            // Agent stage (if available)
            if agent_parsed.is_some() {
                test_statuses.push(TestStatus {
                    test_name: format!("{}_agent", test_name),
                    status: agent_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                    r#type: "fail_to_pass".to_string(),
                });
            }
            
            // Report stage (if available)
            if report_data.is_some() {
                test_statuses.push(TestStatus {
                    test_name: format!("{}_report", test_name),
                    status: report_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                    r#type: "fail_to_pass".to_string(),
                });
            }
        }
        
        for test_name in pass_to_pass_tests {
            // Base stage
            test_statuses.push(TestStatus {
                test_name: format!("{}_base", test_name),
                status: base_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                r#type: "pass_to_pass".to_string(),
            });
            
            // Before stage
            test_statuses.push(TestStatus {
                test_name: format!("{}_before", test_name),
                status: before_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                r#type: "pass_to_pass".to_string(),
            });
            
            // After stage
            test_statuses.push(TestStatus {
                test_name: format!("{}_after", test_name),
                status: after_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                r#type: "pass_to_pass".to_string(),
            });
            
            // Agent stage (if available)
            if agent_parsed.is_some() {
                test_statuses.push(TestStatus {
                    test_name: format!("{}_agent", test_name),
                    status: agent_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                    r#type: "pass_to_pass".to_string(),
                });
            }
            
            // Report stage (if available)
            if report_data.is_some() {
                test_statuses.push(TestStatus {
                    test_name: format!("{}_report", test_name),
                    status: report_s.get(test_name).unwrap_or(&"missing".to_string()).clone(),
                    r#type: "pass_to_pass".to_string(),
                });
            }
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
            duplicate_examples_per_log: HashMap::new(), // TODO: Implement duplicate detection
        };

        LogAnalysisResult {
            test_statuses,
            rule_violations,
            debug_info,
        }
    }

    fn status_lookup(&self, names: &[String], parsed: &ParsedLog) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for name in names {
            if parsed.failed.contains(name) {
                out.insert(name.clone(), "failed".to_string());
            } else if parsed.passed.contains(name) {
                out.insert(name.clone(), "passed".to_string());
            } else if parsed.ignored.contains(name) {
                out.insert(name.clone(), "ignored".to_string());
            } else {
                out.insert(name.clone(), "missing".to_string());
            }
        }
        out
    }

    fn report_status_lookup(&self, names: &[String], report_data: &serde_json::Value) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let mut report_failed_tests = std::collections::HashSet::new();
        let mut report_passed_tests = std::collections::HashSet::new();
        
        // Parse report.json to extract test results
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
        }
        
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
        base_path: &str, // TODO: Use in rule checks
        before_path: &str, // TODO: Use in rule checks
        after_path: &str, // TODO: Use in rule checks
        file_paths: &[String], // TODO: Use in rule checks
    ) -> RuleViolations {
        // C1: P2P tests that are failed in base
        let c1_hits: Vec<String> = pass_to_pass_tests.iter()
            .filter(|t| base_s.get(*t) == Some(&"failed".to_string()))
            .cloned()
            .collect();

        // C2: Any test that failed in after
        let c2_hits: Vec<String> = fail_to_pass_tests.iter()
            .chain(pass_to_pass_tests.iter())
            .filter(|t| after_s.get(*t) == Some(&"failed".to_string()))
            .cloned()
            .collect();

        // C3: F2P tests that are successful in before
        let c3_hits: Vec<String> = fail_to_pass_tests.iter()
            .filter(|t| before_s.get(*t) == Some(&"passed".to_string()))
            .cloned()
            .collect();

        // C4: P2P tests missing in base and not passing in before
        let mut c4_hits: Vec<String> = Vec::new();
        for t in pass_to_pass_tests {
            let b = base_s.get(t).map(String::as_str).unwrap_or("missing");
            let be = before_s.get(t).map(String::as_str).unwrap_or("missing");
            
            if b == "missing" && be != "passed" {
                c4_hits.push(format!("{} (missing in base, {} in before)", t, be));
            }
        }

        // C5: Duplicates (simplified for now)
        let c5_hits: Vec<String> = Vec::new(); // TODO: Implement duplicate detection

        // C6: Test marked as failed in report but passing in agent
        let mut c6_hits: Vec<String> = Vec::new();
        for test_name in fail_to_pass_tests.iter().chain(pass_to_pass_tests.iter()) {
            let report_status = report_s.get(test_name).map(String::as_str).unwrap_or("missing");
            let agent_status = agent_s.get(test_name).map(String::as_str).unwrap_or("missing");
            
            if report_status == "failed" && agent_status == "passed" {
                c6_hits.push(format!("{} (marked as failed in report.json but passing in agent log)", test_name));
            }
        }

        // C7: F2P tests in golden source diff (simplified for now)
        let c7_hits: Vec<String> = Vec::new(); // TODO: Implement diff analysis

        RuleViolations {
            c1_failed_in_base_present_in_p2p: RuleViolation {
                has_problem: !c1_hits.is_empty(),
                examples: c1_hits,
            },
            c2_failed_in_after_present_in_f2p_or_p2p: RuleViolation {
                has_problem: !c2_hits.is_empty(),
                examples: c2_hits,
            },
            c3_f2p_success_in_before: RuleViolation {
                has_problem: !c3_hits.is_empty(),
                examples: c3_hits,
            },
            c4_p2p_missing_in_base_and_not_passing_in_before: RuleViolation {
                has_problem: !c4_hits.is_empty(),
                examples: c4_hits,
            },
            c5_duplicates_in_same_log: RuleViolation {
                has_problem: !c5_hits.is_empty(),
                examples: c5_hits,
            },
            c6_test_marked_failed_in_report_but_passing_in_agent: RuleViolation {
                has_problem: !c6_hits.is_empty(),
                examples: c6_hits,
            },
            c7_f2p_tests_in_golden_source_diff: RuleViolation {
                has_problem: !c7_hits.is_empty(),
                examples: c7_hits,
            },
        }
    }
}

// Rust-specific log parser is now in rust_log_parser.rs

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
                println!("Test statuses: {} tests", result.test_statuses.len());
                for status in &result.test_statuses {
                    println!("  {}: {} ({})", status.test_name, status.status, status.r#type);
                }
                assert!(result.test_statuses.len() > 0, "Should have found test statuses");
            },
            Err(e) => {
                panic!("Log analysis failed: {}", e);
            }
        }

        // Clean up
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
