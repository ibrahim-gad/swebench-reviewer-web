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
    pub programming_language: String,
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

// API endpoint handlers
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
