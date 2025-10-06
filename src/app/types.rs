use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ValidationResult {
    pub files_to_download: Vec<FileInfo>,
    pub folder_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct DownloadRequest {
    pub files_to_download: Vec<FileInfo>,
    pub folder_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadResult {
    pub temp_directory: String,
    pub downloaded_files: Vec<FileInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProcessingResult {
    pub status: String,
    pub message: String,
    pub files_processed: usize,
    pub issues_found: usize,
    pub score: usize,
    pub file_paths: Vec<String>,
    pub deliverable_link: String,
    pub instance_id: String,
    pub task_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TestLists {
    pub fail_to_pass: Vec<String>,
    pub pass_to_pass: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub line_number: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogSearchResults {
    pub base_results: Vec<SearchResult>,
    pub before_results: Vec<SearchResult>,
    pub after_results: Vec<SearchResult>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileContent {
    pub content: String,
    pub file_type: String, // "text" | "json"
}

#[derive(Clone, Default)]
pub struct FileContents {
    pub base: Option<FileContent>,
    pub before: Option<FileContent>,
    pub after: Option<FileContent>,
    pub agent: Option<FileContent>,
    pub main_json: Option<FileContent>,
    pub report: Option<FileContent>,
    pub analysis: Option<FileContent>,
    pub base_analysis: Option<FileContent>,
    pub before_analysis: Option<FileContent>,
    pub after_analysis: Option<FileContent>,
    pub agent_analysis: Option<FileContent>,
}

impl FileContents {
    pub fn get(&self, key: &str) -> Option<&FileContent> {
        match key {
            "base" => self.base.as_ref(),
            "before" => self.before.as_ref(),
            "after" => self.after.as_ref(),
            "agent" => self.agent.as_ref(),
            "main_json" => self.main_json.as_ref(),
            "report" => self.report.as_ref(),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ProcessingStage {
    Validating,
    Downloading,
    LoadingTests,
}

#[derive(Clone, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Active,
    Completed,
    Error,
}

// Log analysis types
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogAnalysisResult {
    pub test_statuses: Vec<TestStatus>,
    pub rule_violations: RuleViolations,
    pub debug_info: DebugInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RuleViolations {
    pub c1_failed_in_base_present_in_p2p: RuleViolation,
    pub c2_failed_in_after_present_in_f2p_or_p2p: RuleViolation,
    pub c3_f2p_success_in_before: RuleViolation,
    pub c4_p2p_missing_in_base_and_not_passing_in_before: RuleViolation,
    pub c5_duplicates_in_same_log: RuleViolation,
    pub c6_test_marked_failed_in_report_but_passing_in_agent: RuleViolation,
    pub c7_f2p_tests_in_golden_source_diff: RuleViolation,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RuleViolation {
    pub has_problem: bool,
    pub examples: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DebugInfo {
    pub log_counts: Vec<LogCount>,
    pub duplicate_examples_per_log: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LogCount {
    pub label: String,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub all: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TestStatus {
    pub test_name: String,
    pub status: String, // "passed", "failed", "ignored", "missing"
    pub r#type: String, // "fail_to_pass" or "pass_to_pass"
}

#[derive(Clone, Default)]
pub struct LoadedFileTypes {
    pub base: bool,
    pub before: bool,
    pub after: bool,
    pub agent: bool,
    pub main_json: bool,
    pub report: bool,
}

impl LoadedFileTypes {
    pub fn is_loaded(&self, key: &str) -> bool {
        match key {
            "base" => self.base,
            "before" => self.before,
            "after" => self.after,
            "agent" => self.agent,
            "main_json" => self.main_json,
            "report" => self.report,
            _ => false,
        }
    }

    pub fn set_loaded(&mut self, key: &str) {
        match key {
            "base" => self.base = true,
            "before" => self.before = true,
            "after" => self.after = true,
            "agent" => self.agent = true,
            "main_json" => self.main_json = true,
            "report" => self.report = true,
            _ => {},
        }
    }
}

#[derive(Clone)]
pub struct DeliverableInfo {
    pub deliverable_link: String,
    pub instance_id: String,
    pub task_id: String,
}
