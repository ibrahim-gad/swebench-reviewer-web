use serde::{Deserialize, Serialize};
use crate::app::types::TestLists;

#[derive(Serialize, Deserialize)]
pub struct GetFileContentRequest {
    pub file_type: String,
    pub file_paths: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetTestListsRequest {
    pub file_paths: Vec<String>,
}


pub fn get_file_contents(file_type: String, file_paths: Vec<String>) -> Result<String, String> {
    use std::fs;
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    let file_extensions = match file_type.as_str() {
        "base" => vec!["base.log", "base.txt"],
        "before" => vec!["before.log", "before.txt"],
        "after" => vec!["after.log", "after.txt"],
        "agent" => vec!["post_agent_patch"],
        "main_json" => vec!["main/", "report.json", "summary.json"],
        "report" => vec!["report.json", "analysis.json", "results.json"],
        _ => return Err(format!("Unknown file type: {}", file_type)),
    };

    // Build absolute path candidates from relative paths: base_temp_dir/folder_id/<rel>
    // We reconstruct base_temp_dir using the TempDir parent pattern used in download_deliverable_impl
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp directory: {}", e))?;
    let temp_path = temp_dir.path().to_string_lossy().to_string();
    let base_temp_dir = std::path::Path::new(&temp_path).parent().unwrap().join("swe-reviewer-temp");

    for rel in &file_paths {
        let abs_path: PathBuf = base_temp_dir.join(rel);
        let path_lower = abs_path.to_string_lossy().to_lowercase();
        for extension in &file_extensions {
            if path_lower.contains(extension) {
                match fs::read_to_string(&abs_path) {
                    Ok(content) => return Ok(content),
                    Err(e) => {
                        eprintln!("Failed to read file {}: {}", abs_path.display(), e);
                        continue;
                    }
                }
            }
        }
    }
    
    Ok(format!("No {} file found in the provided paths", file_type))
}

pub fn get_test_lists(file_paths: Vec<String>) -> Result<TestLists, String> {
    use std::fs;
    use tempfile::TempDir;
    
    // Resolve relative paths to absolute under base_temp_dir
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp directory: {}", e))?;
    let temp_path = temp_dir.path().to_string_lossy().to_string();
    let base_temp_dir = std::path::Path::new(&temp_path).parent().unwrap().join("swe-reviewer-temp");

    let mut main_json_abs: Option<std::path::PathBuf> = None;
    for rel in &file_paths {
        let abs = base_temp_dir.join(rel);
        let lower = abs.to_string_lossy().to_lowercase();
        if lower.contains("main.json") || lower.contains("main/") {
            main_json_abs = Some(abs);
            break;
        }
    }
    let main_json_path = main_json_abs.ok_or("main.json file not found in provided paths".to_string())?;
    
    let main_json_content = fs::read_to_string(&main_json_path)
        .map_err(|e| format!("Failed to read main.json: {}", e))?;
    
    let main_json: serde_json::Value = serde_json::from_str(&main_json_content)
        .map_err(|e| format!("Failed to parse main.json: {}", e))?;
    
    let empty_vec: Vec<serde_json::Value> = vec![];
    let fail_to_pass: Vec<String> = main_json.get("fail_to_pass")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();
    
    let pass_to_pass: Vec<String> = main_json.get("pass_to_pass")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();
    
    Ok(TestLists {
        fail_to_pass,
        pass_to_pass,
    })
}
