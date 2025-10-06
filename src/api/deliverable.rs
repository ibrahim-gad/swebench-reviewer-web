use std::fs;
use tempfile::TempDir;
use crate::app::types::{FileInfo, ValidationResult, DownloadResult};
use crate::drive::{extract_drive_folder_id, get_folder_metadata, get_folder_contents};
use crate::auth::get_access_token;


async fn validate_cached_folder(
    folder_id: &str,
    instance_name: &str,
    cached_path: &std::path::Path,
) -> Result<ValidationResult, String> {
    let instance_json_name = format!("{}.json", instance_name);
    let instance_json_path = cached_path.join("main").join(&instance_json_name);
    
    if !instance_json_path.exists() {
        return Err(format!(
            "Missing required file in cache: {}. Cached files: [{}]",
            instance_json_name,
            get_cached_file_list(cached_path).join(", ")
        ));
    }

    let logs_path = cached_path.join("logs");
    if !logs_path.exists() || !logs_path.is_dir() {
        return Err("Missing required 'logs' folder in cache".to_string());
    }

    let required_suffixes = vec![
        "_after.log",
        "_before.log", 
        "_base.log",
        "_post_agent_patch.log",
    ];

    for suffix in &required_suffixes {
        let suffix_lower = suffix.to_lowercase();
        let has_file = std::fs::read_dir(&logs_path)
            .map_err(|e| format!("Failed to read logs directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .any(|entry| {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                file_name.ends_with(&suffix_lower) && entry.path().is_file()
            });

        if !has_file {
            return Err(format!("Missing required log file ending with: {} in cache", suffix));
        }
    }

    let results_path = cached_path.join("results");
    if !results_path.exists() || !results_path.is_dir() {
        return Err("Missing required 'results' folder in cache".to_string());
    }

    let report_path = results_path.join("report.json");
    if !report_path.exists() || !report_path.is_file() {
        return Err("Missing required file: report.json in results folder cache".to_string());
    }
    let patches_path = cached_path.join("patches");
    if !patches_path.exists() || !patches_path.is_dir() {
        return Err("Missing required 'patches' folder in cache".to_string());
    }
    // make sure the patches folder has the required files
    let possible_suffixes = vec![".diff", ".patch"];
  
    let has_file = std::fs::read_dir(&patches_path)
        .map_err(|e| format!("Failed to read patches directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            let file_name = entry.file_name().to_string_lossy().to_lowercase();
            possible_suffixes.iter().any(|suffix| file_name.ends_with(suffix)) && entry.path().is_file()
        });

    if !has_file {
        return Err(format!("Missing required patch file ending with: {} in cache", possible_suffixes.join(", ")));
    }


    let mut files_to_download = Vec::new();

    files_to_download.push(FileInfo {
        id: "cached".to_string(),
        name: instance_json_name.clone(),
        path: format!("main/{}", instance_json_name),
    });

    for suffix in &required_suffixes {
        if let Some(log_file) = std::fs::read_dir(&logs_path)
            .map_err(|e| format!("Failed to read logs directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                file_name.ends_with(&suffix.to_lowercase()) && entry.path().is_file()
            }) {
            files_to_download.push(FileInfo {
                id: "cached".to_string(),
                name: log_file.file_name().to_string_lossy().to_string(),
                path: format!("logs/{}", log_file.file_name().to_string_lossy()),
            });
        }
    }
    let patches_files = std::fs::read_dir(&patches_path)
    .map_err(|e| format!("Failed to read patches directory: {}", e))?
    .filter_map(|entry| entry.ok())
    .filter(|entry| entry.path().is_file())
    .collect::<Vec<_>>();
for patch_file in patches_files {
    files_to_download.push(FileInfo {
        id: "cached".to_string(),
        name: patch_file.file_name().to_string_lossy().to_string(),
        path: format!("patches/{}", patch_file.file_name().to_string_lossy()),
    });
}
    files_to_download.push(FileInfo {
        id: "cached".to_string(),
        name: "report.json".to_string(),
        path: "results/report.json".to_string(),
    });

    Ok(ValidationResult {
        files_to_download,
        folder_id: folder_id.to_string(),
    })
}


fn get_cached_file_list(cached_path: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(cached_path) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                files.push(entry.file_name().to_string_lossy().to_string());
            } else if entry.path().is_dir() {
                if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                    for sub_entry in sub_entries.flatten() {
                        if sub_entry.path().is_file() {
                            files.push(format!("{}/{}", 
                                entry.file_name().to_string_lossy(),
                                sub_entry.file_name().to_string_lossy()
                            ));
                        }
                    }
                }
            }
        }
    }
    
    files
}


pub async fn validate_deliverable_impl(
    folder_link: String,
) -> Result<ValidationResult, String> {
    let folder_id = extract_drive_folder_id(&folder_link)
        .ok_or("Invalid Google Drive folder link. Please provide a valid folder URL.")?;

    // Check if we have a cached folder first
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp directory: {}", e))?;
    let temp_path = temp_dir.path().to_string_lossy().to_string();
    let base_temp_dir = std::path::Path::new(&temp_path).parent().unwrap().join("swe-reviewer-temp");
    let persist_dir = base_temp_dir.join(&folder_id);

    if persist_dir.exists() {
        let access_token = get_access_token()
            .await
            .map_err(|e| format!("Failed to get access token: {}", e))?;

        let folder_meta = get_folder_metadata(&folder_id, &access_token).await
            .map_err(|e| format!("Failed to get folder metadata: {}", e))?;

        let folder_name = folder_meta["name"].as_str().unwrap_or("");
        let instance_name = folder_name.split_whitespace()
            .next()
            .ok_or("Could not extract instance name from folder name")?;

        match validate_cached_folder(&folder_id, instance_name, &persist_dir).await {
            Ok(result) => {
                return Ok(result);
            }
            Err(cached_error) => {
                eprintln!("Cached validation failed: {}. Removing cache and retrying with remote validation.", cached_error);
                if let Err(remove_error) = std::fs::remove_dir_all(&persist_dir) {
                    eprintln!("Warning: Failed to remove cached folder: {}", remove_error);
                }
            }
        }
    }

    let access_token = get_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

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
    let patches_folder = files.iter().find(|file| {
        let file_name = file["name"].as_str().unwrap_or("").to_lowercase();
        file_name == "patches" &&
        file["mimeType"].as_str() == Some("application/vnd.google-apps.folder")
    });
    let patches_folder_id = match patches_folder {
        Some(folder) => folder["id"].as_str().ok_or("Invalid patches folder ID")?,
        None => return Err("Missing required 'patches' folder (case insensitive search)".to_string()),
    };
    let patches_contents = get_folder_contents(patches_folder_id, &access_token).await
        .map_err(|e| format!("Failed to get patches folder contents: {}", e))?;
    let patches_files = patches_contents["files"].as_array()
        .ok_or("Invalid patches folder contents response")?;
    for diff_file in patches_files.iter().filter(|file| {
        let file_name = file["name"].as_str().unwrap_or("").to_lowercase();
        (file_name.ends_with(".diff") || file_name.ends_with(".patch")) &&
        file["mimeType"].as_str() != Some("application/vnd.google-apps.folder")
    }) {
        println!("Found diff file: {}, adding to download list", diff_file["name"].as_str().unwrap_or(""));
        files_to_download.push(FileInfo {
            id: diff_file["id"].as_str().unwrap_or("").to_string(),
            name: diff_file["name"].as_str().unwrap_or("").to_string(),
            path: format!("patches/{}", diff_file["name"].as_str().unwrap_or("")),
        });
    }
    Ok(ValidationResult {
        files_to_download,
        folder_id: folder_id.to_string(),
    })
}


pub async fn download_deliverable_impl(
    files_to_download: Vec<FileInfo>,
    folder_id: String,
) -> Result<DownloadResult, String> {
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

    let persist_dir = base_temp_dir.join(&folder_id);

    if persist_dir.exists() {
        let mut cached_files = Vec::new();
        let mut all_files_cached = true;
        
        for file_info in &files_to_download {
            let cached_file_path = persist_dir.join(&file_info.path);
            if cached_file_path.exists() {
                cached_files.push(FileInfo {
                    id: file_info.id.clone(),
                    name: file_info.name.clone(),
                    path: cached_file_path.to_string_lossy().to_string(),
                });
            } else {
                all_files_cached = false;
                break;
            }
        }

        if all_files_cached && !cached_files.is_empty() {
            return Ok(DownloadResult {
                temp_directory: persist_dir.to_string_lossy().to_string(),
                downloaded_files: cached_files,
            });
        }
    }

    let mut downloaded_files = Vec::new();
    let client = reqwest::Client::new();
    
    // Store files_to_download for later use with cached files
    let files_to_download = files_to_download.clone();

    for file_info in &files_to_download {
        // Skip files that are already cached (have placeholder ID)
        if file_info.id == "cached" {
            continue;
        }

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
            id: file_info.id.clone(),
            name: file_info.name.clone(),
            path: file_path.to_string_lossy().to_string(),
        });
    }

    fs::create_dir_all(&persist_dir).map_err(|e| format!("Failed to create persist dir: {}", e))?;

    // Copy newly downloaded files to persist directory
    for file_info in &downloaded_files {
        let source = std::path::Path::new(&file_info.path);
        let relative_path = source.strip_prefix(&temp_path).unwrap();
        let dest = persist_dir.join(relative_path);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dest dir: {}", e))?;
        }

        fs::copy(source, &dest).map_err(|e| format!("Failed to copy file: {}", e))?;
    }

    // Build final file list including both cached and newly downloaded files
    let mut updated_files = Vec::new();
    
    // Add newly downloaded files
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

    // Add cached files (those with placeholder IDs)
    for file_info in &files_to_download {
        if file_info.id == "cached" {
            let cached_file_path = persist_dir.join(&file_info.path);
            if cached_file_path.exists() {
                updated_files.push(FileInfo {
                    id: file_info.id.clone(),
                    name: file_info.name.clone(),
                    path: cached_file_path.to_string_lossy().to_string(),
                });
            }
        }
    }

    Ok(DownloadResult {
        temp_directory: persist_dir.to_string_lossy().to_string(),
        downloaded_files: updated_files,
    })
}

