use serde::{Deserialize, Serialize};
use axum::{Json, response::Response, body::Body};
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
    
    let file_extensions = match file_type.as_str() {
        "base" => vec!["base.log", "base.txt"],
        "before" => vec!["before.log", "before.txt"],
        "after" => vec!["after.log", "after.txt"],
        "agent" => vec!["post_agent_patch"],
        "main_json" => vec!["main/", "report.json", "summary.json"],
        "report" => vec!["report.json", "analysis.json", "results.json"],
        _ => return Err(format!("Unknown file type: {}", file_type)),
    };

    for path in &file_paths {
        let path_lower = path.to_lowercase();
        for extension in &file_extensions {
            if path_lower.contains(extension) {
                match fs::read_to_string(path) {
                    Ok(content) => return Ok(content),
                    Err(e) => {
                        eprintln!("Failed to read file {}: {}", path, e);
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
    
    let main_json_path = file_paths.iter()
        .find(|path| path.to_lowercase().contains("main.json") || path.to_lowercase().contains("main/"))
        .ok_or("main.json file not found in provided paths".to_string())?;
    
    let main_json_content = fs::read_to_string(main_json_path)
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
