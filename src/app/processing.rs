use leptos::prelude::*;
use leptos::task::spawn_local;
use super::types::{ValidationResult, DownloadResult, ProcessingResult, ProcessingStage, StageStatus};
use std::collections::HashMap;

pub fn handle_submit(
    deliverable_link: RwSignal<String>,
    is_processing: RwSignal<bool>,
    current_stage: RwSignal<Option<ProcessingStage>>,
    stages: RwSignal<HashMap<ProcessingStage, StageStatus>>,
    result: RwSignal<Option<ProcessingResult>>,
    error: RwSignal<Option<String>>,
    load_test_lists: impl Fn() + Send + Sync + 'static + Copy,
) {
    let link = deliverable_link.get().trim().to_string();
    if link.is_empty() {
        error.set(Some("Please enter a deliverable link".to_string()));
        return;
    }

    is_processing.set(true);
    error.set(None);
    result.set(None);

    let update_stage_status = move |stage: ProcessingStage, status: StageStatus| {
        stages.update(|stages| {
            stages.insert(stage, status);
        });
    };

    spawn_local(async move {
        // Stage 1: Validating
        current_stage.set(Some(ProcessingStage::Validating));
        update_stage_status(ProcessingStage::Validating, StageStatus::Active);

        let validation_result: Result<ValidationResult, String> = async {
            #[cfg(feature = "hydrate")]
            {
                let resp = gloo_net::http::Request::post("/api/validate")
                    .json(&serde_json::json!({ 
                        "folder_link": link,
                        "programming_language": "rust".to_string()
                    }))
                    .unwrap()
                    .send()
                    .await;
                
                match resp {
                    Ok(resp) => {
                        let is_success = resp.status() >= 200 && resp.status() < 300;
                        
                        if is_success {
                            resp.json().await.map_err(|e| format!("JSON parse error: {}", e))
                        } else {
                            let error_text = resp.text().await.map_err(|e| format!("Error response: {}", e));
                            match error_text {
                                Ok(text) => Err(format!("Validation failed: {}", text)),
                                Err(e) => Err(e),
                            }
                        }
                    }
                    Err(e) => Err(format!("Validation request failed: {}", e)),
                }
            }
            
            #[cfg(not(feature = "hydrate"))]
            {
                // On SSR, this won't be called as it's a client-side action
                Err("Client-side only operation".to_string())
            }
        }.await;

        match validation_result {
            Ok(validation_data) => {
                update_stage_status(ProcessingStage::Validating, StageStatus::Completed);

                // Stage 2: Downloading
                current_stage.set(Some(ProcessingStage::Downloading));
                update_stage_status(ProcessingStage::Downloading, StageStatus::Active);

                let download_result: Result<DownloadResult, String> = async {
                    #[cfg(feature = "hydrate")]
                    {
                        let resp = gloo_net::http::Request::post("/api/download")
                            .json(&serde_json::json!({
                                "files_to_download": validation_data.files_to_download,
                                "folder_id": validation_data.folder_id
                            }))
                            .unwrap()
                            .send()
                            .await;
                        
                        match resp {
                            Ok(resp) => {
                                let is_success = resp.status() >= 200 && resp.status() < 300;
                                
                                if is_success {
                                    resp.json().await.map_err(|e| format!("JSON parse error: {}", e))
                                } else {
                                    let error_text = resp.text().await.map_err(|e| format!("Error response: {}", e));
                                    match error_text {
                                        Ok(text) => Err(format!("Download failed: {}", text)),
                                        Err(e) => Err(e),
                                    }
                                }
                            }
                            Err(e) => Err(format!("Download request failed: {}", e)),
                        }
                    }
                    
                    #[cfg(not(feature = "hydrate"))]
                    {
                        // On SSR, this won't be called as it's a client-side action
                        Err("Client-side only operation".to_string())
                    }
                }.await;

                match download_result {
                    Ok(download_data) => {
                        update_stage_status(ProcessingStage::Downloading, StageStatus::Completed);

                        let processing_result = ProcessingResult {
                            status: "downloaded".to_string(),
                            message: "Files downloaded successfully".to_string(),
                            files_processed: download_data.downloaded_files.len(),
                            issues_found: 0,
                            score: 0,
                            file_paths: download_data.downloaded_files.iter()
                                .map(|f| f.path.clone())
                                .collect(),
                            deliverable_link: link.clone(),
                            instance_id: String::new(),
                            task_id: String::new(),
                        };

                        result.set(Some(processing_result));
                        
                        // Stage 3: Loading tests
                        current_stage.set(Some(ProcessingStage::LoadingTests));
                        update_stage_status(ProcessingStage::LoadingTests, StageStatus::Active);
                        
                        // After successful download, load additional data
                        load_test_lists();
                    }
                    Err(e) => {
                        error.set(Some(e));
                        update_stage_status(ProcessingStage::Downloading, StageStatus::Error);
                        current_stage.set(None);
                    }
                }
            }
            Err(e) => {
                error.set(Some(e));
                update_stage_status(ProcessingStage::Validating, StageStatus::Error);
                current_stage.set(None);
                is_processing.set(false);
            }
        }
    });
}
