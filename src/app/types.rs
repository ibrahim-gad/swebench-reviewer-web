use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
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
            "analysis" => self.analysis.as_ref(),
            "base_analysis" => self.base_analysis.as_ref(),
            "before_analysis" => self.before_analysis.as_ref(),
            "after_analysis" => self.after_analysis.as_ref(),
            "agent_analysis" => self.agent_analysis.as_ref(),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ProcessingStage {
    Validating,
    Downloading,
}

#[derive(Clone, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Active,
    Completed,
    Error,
}
