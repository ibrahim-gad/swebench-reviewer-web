#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use axum::{Json, response::Response, body::Body};

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct SearchLogsRequest {
    pub file_paths: Vec<String>,
    pub test_name: String,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct AnalyzeLogsRequest {
    pub file_paths: Vec<String>,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub line_number: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct LogSearchResults {
    pub base_results: Vec<SearchResult>,
    pub before_results: Vec<SearchResult>,
    pub after_results: Vec<SearchResult>,
}

#[cfg(feature = "ssr")]
pub fn search_logs(file_paths: Vec<String>, test_name: String) -> Result<LogSearchResults, String> {
    let base_log = file_paths.iter().find(|path| path.to_lowercase().contains("base.log"));
    let before_log = file_paths.iter().find(|path| path.to_lowercase().contains("before.log"));
    let after_log = file_paths.iter().find(|path| path.to_lowercase().contains("after.log"));
    
    let base_results = if let Some(path) = base_log {
        search_in_log_file(path, &test_name)?
    } else {
        Vec::new()
    };
    
    let before_results = if let Some(path) = before_log {
        search_in_log_file(path, &test_name)?
    } else {
        Vec::new()
    };
    
    let after_results = if let Some(path) = after_log {
        search_in_log_file(path, &test_name)?
    } else {
        Vec::new()
    };
    
    Ok(LogSearchResults {
        base_results,
        before_results,
        after_results,
    })
}

#[cfg(feature = "ssr")]
fn search_in_log_file(file_path: &str, test_name: &str) -> Result<Vec<SearchResult>, String> {
    use std::fs;
    
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read log file {}: {}", file_path, e))?;
    
    let lines: Vec<&str> = content.lines().collect();
    let mut results = Vec::new();
    
    let search_terms = get_search_terms(test_name);
    
    for (line_number, line) in lines.iter().enumerate() {
        let mut found_match = false;
        
        for search_term in &search_terms {
            if line.contains(search_term) {
                found_match = true;
                break;
            }
        }
        
        if found_match {
            let context_before: Vec<String> = lines.iter()
                .skip(line_number.saturating_sub(5))
                .take(5.min(line_number))
                .map(|s| s.to_string())
                .collect();
            
            let context_after: Vec<String> = lines.iter()
                .skip(line_number + 1)
                .take(5)
                .map(|s| s.to_string())
                .collect();
            
            results.push(SearchResult {
                line_number: line_number + 1,
                line_content: line.to_string(),
                context_before,
                context_after,
            });
        }
    }
    
    Ok(results)
}

#[cfg(feature = "ssr")]
fn get_search_terms(test_name: &str) -> Vec<String> {
    let mut search_terms = vec![test_name.to_string()];
    
    if let Some(last_part) = test_name.split(" - ").last() {
        if last_part != test_name {
            search_terms.push(last_part.to_string());
        }
    }
    
    search_terms.dedup();
    search_terms
}

// API endpoint handlers
#[cfg(feature = "ssr")]
pub async fn search_logs_endpoint(
    Json(payload): Json<SearchLogsRequest>,
) -> Response {
    match search_logs(payload.file_paths, payload.test_name) {
        Ok(result) => Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&result).unwrap()))
            .unwrap(),
        Err(error) => Response::builder()
            .status(400)
            .body(Body::from(error))
            .unwrap(),
    }
}

#[cfg(feature = "ssr")]
pub async fn analyze_logs_endpoint(
    Json(_payload): Json<AnalyzeLogsRequest>,
) -> Response {
    // For now, return a mock analysis result
    // In a full implementation, this would call the Rust log analysis code
    let mock_result = serde_json::json!({
        "p2p_analysis": {},
        "f2p_analysis": {},
        "rule_checks": {
            "c1_failed_in_base_present_in_P2P": {
                "has_problem": false,
                "examples": []
            },
            "c2_failed_in_after_present_in_F2P_or_P2P": {
                "has_problem": false,
                "examples": []
            },
            "c3_F2P_success_in_before": {
                "has_problem": false,
                "examples": []
            },
            "c4_P2P_missing_in_base_and_not_passing_in_before": {
                "has_problem": false,
                "examples": []
            },
            "c5_duplicates_in_same_log_for_F2P_or_P2P": {
                "has_problem": false,
                "examples": []
            }
        }
    });
    
    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&mock_result).unwrap()))
        .unwrap()
}
