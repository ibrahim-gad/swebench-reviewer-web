use leptos::prelude::*;
use leptos::task::spawn_local;
use super::types::{ValidationResult, DownloadResult, ProcessingResult, ProcessingStage, StageStatus, FileInfo};
use std::collections::HashMap;

#[server]
pub async fn handle_validate_deliverable(deliverable_link: String) -> Result<ValidationResult, ServerFnError> {
    use crate::api::deliverable::{validate_deliverable_impl};
    match validate_deliverable_impl(deliverable_link).await {
        Ok(result) => Ok(result),
        Err(e) => Err(ServerFnError::ServerError(format!("Failed to validate deliverable: {}", e)))
    }
}


#[server]
pub async fn handle_download_deliverable(files_to_download: Vec<FileInfo>, folder_id: String) -> Result<DownloadResult, ServerFnError> {
    use crate::api::deliverable::{download_deliverable_impl};
    match download_deliverable_impl(files_to_download, folder_id).await {
        Ok(result) => Ok(result),
        Err(e) => Err(ServerFnError::ServerError(format!("Failed to download deliverable: {}", e)))
    }
}


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

        let validation_result = handle_validate_deliverable(link.clone()).await;

        match validation_result {
            Ok(validation_data) => {
                update_stage_status(ProcessingStage::Validating, StageStatus::Completed);

                // Stage 2: Downloading
                current_stage.set(Some(ProcessingStage::Downloading));
                update_stage_status(ProcessingStage::Downloading, StageStatus::Active);

                let download_result = handle_download_deliverable(validation_data.files_to_download, validation_data.folder_id).await;

                match download_result {
                    Ok(download_data) => {
                        update_stage_status(ProcessingStage::Downloading, StageStatus::Completed);

                        let processing_result = ProcessingResult {
                            file_paths: download_data.downloaded_files.iter()
                                .map(|f| f.path.clone())
                                .collect(),
                            deliverable_link: link.clone(),
                            instance_id: String::new(),
                            task_id: String::new(),
                            pr_id: String::new(),
                            issue_id: String::new(),
                            repo: String::new(),
                            problem_statement: String::new(),
                            conversation: Vec::new(),
                            gold_patch: String::new(),
                            test_patch: String::new(),
                            language: String::new(),
                        };

                        result.set(Some(processing_result));
                        
                        // Stage 3: Loading tests
                        current_stage.set(Some(ProcessingStage::LoadingTests));
                        update_stage_status(ProcessingStage::LoadingTests, StageStatus::Active);
                        
                        // After successful download, load additional data
                        load_test_lists();
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        update_stage_status(ProcessingStage::Downloading, StageStatus::Error);
                        current_stage.set(None);
                    }
                }
            }
            Err(e) => {
                error.set(Some(e.to_string()));
                update_stage_status(ProcessingStage::Validating, StageStatus::Error);
                current_stage.set(None);
                is_processing.set(false);
            }
        }
    });
}
