#[cfg(feature = "ssr")]
use reqwest::header::AUTHORIZATION;
use anyhow::{Result, anyhow};

pub fn extract_drive_folder_id(link: &str) -> Option<String> {
    let patterns = [
        ("/folders/", "?"),
        ("/folders/", "&"),
        ("/folders/", "#"),
        ("open?id=", "&"),
        ("open?id=", "#"),
    ];

    for (start_pat, end_pat) in patterns.iter() {
        if let Some(start) = link.find(start_pat) {
            let after = &link[start + start_pat.len()..];
            let end = after.find(end_pat).unwrap_or(after.len());
            return Some(after[..end].to_string());
        }
    }
    None
}

pub async fn get_shared_drives(access_token: &str) -> Result<Vec<(String, String)>> {
    let client = reqwest::Client::new();
    let url = "https://www.googleapis.com/drive/v3/drives?fields=drives(id,name)";

    let resp = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {}", access_token))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let result: serde_json::Value = resp.json().await?;
    let drives = result["drives"].as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|drive| {
            let name = drive["name"].as_str()?;
            let id = drive["id"].as_str()?;
            Some((name.to_string(), id.to_string()))
        })
        .collect();

    Ok(drives)
}

pub async fn get_folder_contents(folder_id: &str, access_token: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let query = format!("'{}' in parents", folder_id);
    let encoded_query = urlencoding::encode(&query);

    let personal_url = format!(
        "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType)&supportsAllDrives=true",
        encoded_query
    );

    let resp = client
        .get(&personal_url)
        .header(AUTHORIZATION, format!("Bearer {}", access_token))
        .send()
        .await?;

    if resp.status().is_success() {
        let result: serde_json::Value = resp.json().await?;
        if let Some(files) = result["files"].as_array() {
            if !files.is_empty() {
                return Ok(serde_json::json!({
                    "files": files,
                    "debug_info": {
                        "successful_query": query,
                        "drive": "personal",
                        "files_count": files.len()
                    }
                }));
            }
        }
    }

    let shared_drives = get_shared_drives(access_token).await.unwrap_or_else(|_| vec![]);

    for (drive_name, drive_id) in shared_drives {
        let shared_url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType)&driveId={}&includeItemsFromAllDrives=true&supportsAllDrives=true&corpora=drive",
            encoded_query, drive_id
        );

        let resp = client
            .get(&shared_url)
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?;

        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            if let Some(files) = result["files"].as_array() {
                if !files.is_empty() {
                    return Ok(serde_json::json!({
                        "files": files,
                        "debug_info": {
                            "successful_query": query,
                            "drive": drive_name,
                            "drive_id": drive_id,
                            "files_count": files.len()
                        }
                    }));
                }
            }
        }
    }

    Err(anyhow!("Folder not found in personal drive or any accessible shared drives"))
}

pub async fn get_folder_metadata(folder_id: &str, access_token: &str) -> Result<serde_json::Value> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType&supportsAllDrives=true",
        folder_id
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", access_token))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Failed to get folder metadata: {}", resp.status()));
    }

    resp.json().await.map_err(|e| anyhow!("JSON parse error: {}", e))
}