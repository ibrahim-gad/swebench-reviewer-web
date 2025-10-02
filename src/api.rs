#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use std::fs;
#[cfg(feature = "ssr")]
use tempfile::TempDir;
#[cfg(feature = "ssr")]
use axum::{Json, response::Response, body::Body};
#[cfg(feature = "ssr")]
use crate::drive::{extract_drive_folder_id, get_folder_metadata, get_folder_contents};
#[cfg(feature = "ssr")]
use crate::auth::get_access_token;

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct ValidationResult {
    pub files_to_download: Vec<FileInfo>,
    pub folder_id: String,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct DownloadResult {
    pub temp_directory: String,
    pub downloaded_files: Vec<FileInfo>,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct ValidateRequest {
    pub folder_link: String,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct DownloadRequest {
    pub files_to_download: Vec<FileInfo>,
    pub folder_id: String,
}

#[cfg(feature = "ssr")]
async fn validate_deliverable_impl(
    payload: ValidateRequest,
) -> Result<ValidationResult, String> {
    let access_token = get_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

    let folder_id = extract_drive_folder_id(&payload.folder_link)
        .ok_or("Invalid Google Drive folder link. Please provide a valid folder URL.")?;

    let folder_meta = get_folder_metadata(&folder_id, &access_token).await
        .map_err(|e| format!("Failed to get folder metadata: {}", e))?;

    let mime_type = folder_meta["mimeType"].as_str().unwrap_or("");
    let folder_name = folder_meta["name"].as_str().unwrap_or("");

    if mime_type != "application/vnd.google-apps.folder" {
        return Err("The provided link is not a folder. Please provide a Google Drive folder link.".to_string());
    }

    let instance_name = folder_name.split_whitespace()
        .next()
        .ok_or("Could not extract instance name from folder name")?;

    let folder_contents = get_folder_contents(&folder_id, &access_token).await
        .map_err(|e| format!("Failed to get folder contents: {}", e))?;

    let files = folder_contents["files"].as_array()
        .ok_or("Invalid folder contents response")?;

    let instance_json_name = format!("{}.json", instance_name);
    let file_names: Vec<String> = files.iter()
        .filter_map(|file| file["name"].as_str())
        .map(|name| name.to_string())
        .collect();

    let has_instance_json = files.iter().any(|file| {
        let file_name = file["name"].as_str().unwrap_or("");
        let file_mime = file["mimeType"].as_str().unwrap_or("");
        file_name == instance_json_name && file_mime != "application/vnd.google-apps.folder"
    });

    if !has_instance_json {
        return Err(format!(
            "Missing required file: {}. Found files: [{}]",
            instance_json_name,
            file_names.join(", ")
        ));
    }

    let logs_folder = files.iter().find(|file| {
        let file_name = file["name"].as_str().unwrap_or("").to_lowercase();
        file_name == "logs" &&
        file["mimeType"].as_str() == Some("application/vnd.google-apps.folder")
    });

    let logs_folder_id = match logs_folder {
        Some(folder) => folder["id"].as_str().ok_or("Invalid logs folder ID")?,
        None => return Err("Missing required 'logs' folder (case insensitive search)".to_string()),
    };

    let logs_contents = get_folder_contents(logs_folder_id, &access_token).await
        .map_err(|e| format!("Failed to get logs folder contents: {}", e))?;

    let log_files = logs_contents["files"].as_array()
        .ok_or("Invalid logs folder contents response")?;

    let required_suffixes = vec![
        "_after.log",
        "_before.log",
        "_base.log",
        "_post_agent_patch.log",
    ];

    for suffix in &required_suffixes {
        let suffix_lower = suffix.to_lowercase();
        let has_file = log_files.iter().any(|file| {
            let file_name = file["name"].as_str().unwrap_or("").to_lowercase();
            file_name.ends_with(&suffix_lower) &&
            file["mimeType"].as_str() != Some("application/vnd.google-apps.folder")
        });

        if !has_file {
            return Err(format!("Missing required log file ending with: {} (case insensitive search)", suffix));
        }
    }

    let results_folder = files.iter().find(|file| {
        let file_name = file["name"].as_str().unwrap_or("").to_lowercase();
        file_name == "results" && file["mimeType"].as_str() == Some("application/vnd.google-apps.folder")
    }).ok_or("Missing required 'results' folder (case insensitive search)".to_string())?;

    let results_folder_id = results_folder["id"].as_str().ok_or("Invalid results folder ID")?;

    let results_contents = get_folder_contents(results_folder_id, &access_token).await
        .map_err(|e| format!("Failed to get results folder contents: {}", e))?;

    let results_files = results_contents["files"].as_array()
        .ok_or("Invalid results folder contents response")?;

    let report_file = results_files.iter().find(|file| {
        let file_name = file["name"].as_str().unwrap_or("").to_lowercase();
        file_name == "report.json" && file["mimeType"].as_str() != Some("application/vnd.google-apps.folder")
    }).ok_or("Missing required file: report.json in results folder".to_string())?;

    let mut files_to_download = Vec::new();

    if let Some(instance_file) = files.iter().find(|file| {
        let file_name = file["name"].as_str().unwrap_or("");
        file_name == instance_json_name
    }) {
        files_to_download.push(FileInfo {
            id: instance_file["id"].as_str().unwrap_or("").to_string(),
            name: instance_file["name"].as_str().unwrap_or("").to_string(),
            path: format!("main/{}", instance_file["name"].as_str().unwrap_or("")),
        });
    }

    for suffix in &required_suffixes {
        if let Some(log_file) = log_files.iter().find(|file| {
            let file_name = file["name"].as_str().unwrap_or("").to_lowercase();
            file_name.ends_with(&suffix.to_lowercase())
        }) {
            files_to_download.push(FileInfo {
                id: log_file["id"].as_str().unwrap_or("").to_string(),
                name: log_file["name"].as_str().unwrap_or("").to_string(),
                path: format!("logs/{}", log_file["name"].as_str().unwrap_or("")),
            });
        }
    }

    files_to_download.push(FileInfo {
        id: report_file["id"].as_str().unwrap_or("").to_string(),
        name: report_file["name"].as_str().unwrap_or("").to_string(),
        path: format!("results/{}", report_file["name"].as_str().unwrap_or("")),
    });

    Ok(ValidationResult {
        files_to_download,
        folder_id: folder_id.to_string(),
    })
}

#[cfg(feature = "ssr")]
pub async fn validate_deliverable(
    Json(payload): Json<ValidateRequest>,
) -> Response {
    match validate_deliverable_impl(payload).await {
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
async fn download_deliverable_impl(
    payload: DownloadRequest,
) -> Result<DownloadResult, String> {
    #[cfg(feature = "ssr")]
    use reqwest::header::AUTHORIZATION;

    let access_token = get_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp directory: {}", e))?;
    let temp_path = temp_dir.path().to_string_lossy().to_string();

    let base_temp_dir = std::path::Path::new(&temp_path).parent().unwrap().join("swe-reviewer-temp");
    if !base_temp_dir.exists() {
        fs::create_dir_all(&base_temp_dir).map_err(|e| format!("Failed to create base temp dir: {}", e))?;
    }

    let persist_dir = base_temp_dir.join(&payload.folder_id);

    if persist_dir.exists() {
        let mut cached_files = Vec::new();
        for file_info in &payload.files_to_download {
            let cached_file_path = persist_dir.join(&file_info.path);
            if cached_file_path.exists() {
                cached_files.push(FileInfo {
                    id: file_info.id.clone(),
                    name: file_info.name.clone(),
                    path: cached_file_path.to_string_lossy().to_string(),
                });
            }
        }

        if !cached_files.is_empty() {
            return Ok(DownloadResult {
                temp_directory: persist_dir.to_string_lossy().to_string(),
                downloaded_files: cached_files,
            });
        }
    }

    let mut downloaded_files = Vec::new();
    let client = reqwest::Client::new();

    for file_info in payload.files_to_download {
        let file_path = std::path::Path::new(&temp_path).join(&file_info.path);
        let file_dir_path = file_path.parent().unwrap_or(std::path::Path::new(""));
        if !file_dir_path.exists() {
            fs::create_dir_all(&file_dir_path)
                .map_err(|e| format!("Failed to create directory {}: {}", file_dir_path.display(), e))?;
        }

        let download_url = format!("https://www.googleapis.com/drive/v3/files/{}?alt=media&supportsAllDrives=true", file_info.id);
        let file_resp = client
            .get(&download_url)
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Download error for {}: {}", file_info.name, e))?;

        if !file_resp.status().is_success() {
            return Err(format!("Failed to download file {}: {}", file_info.name, file_resp.status()));
        }

        let content = file_resp.bytes().await
            .map_err(|e| format!("File read error for {}: {}", file_info.name, e))?;

        fs::write(&file_path, content)
            .map_err(|e| format!("Failed to write file {}: {}", file_info.name, e))?;

        downloaded_files.push(FileInfo {
            id: file_info.id,
            name: file_info.name,
            path: file_path.to_string_lossy().to_string(),
        });
    }

    fs::create_dir_all(&persist_dir).map_err(|e| format!("Failed to create persist dir: {}", e))?;

    for file_info in &downloaded_files {
        let source = std::path::Path::new(&file_info.path);
        let relative_path = source.strip_prefix(&temp_path).unwrap();
        let dest = persist_dir.join(relative_path);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dest dir: {}", e))?;
        }

        fs::copy(source, &dest).map_err(|e| format!("Failed to copy file: {}", e))?;
    }

    let mut updated_files = Vec::new();
    for file_info in downloaded_files {
        let source = std::path::Path::new(&file_info.path);
        let relative_path = source.strip_prefix(&temp_path).unwrap();
        let new_path = persist_dir.join(relative_path);

        updated_files.push(FileInfo {
            id: file_info.id,
            name: file_info.name,
            path: new_path.to_string_lossy().to_string(),
        });
    }

    Ok(DownloadResult {
        temp_directory: persist_dir.to_string_lossy().to_string(),
        downloaded_files: updated_files,
    })
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct GetFileContentRequest {
    pub file_type: String,
    pub file_paths: Vec<String>,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct GetTestListsRequest {
    pub file_paths: Vec<String>,
}

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
pub struct TestLists {
    pub fail_to_pass: Vec<String>,
    pub pass_to_pass: Vec<String>,
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
pub fn get_file_content(file_type: String, file_paths: Vec<String>) -> Result<String, String> {
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

#[cfg(feature = "ssr")]
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
pub async fn get_file_content_endpoint(
    Json(payload): Json<GetFileContentRequest>,
) -> Response {
    match get_file_content(payload.file_type, payload.file_paths) {
        Ok(content) => Response::builder()
            .status(200)
            .header("Content-Type", "text/plain")
            .body(Body::from(content))
            .unwrap(),
        Err(error) => Response::builder()
            .status(400)
            .body(Body::from(error))
            .unwrap(),
    }
}

#[cfg(feature = "ssr")]
pub async fn get_test_lists_endpoint(
    Json(payload): Json<GetTestListsRequest>,
) -> Response {
    match get_test_lists(payload.file_paths) {
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

#[cfg(feature = "ssr")]
pub async fn download_deliverable(
    Json(payload): Json<DownloadRequest>,
) -> Response {
    match download_deliverable_impl(payload).await {
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
