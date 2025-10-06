use leptos::prelude::*;
use leptos::task::spawn_local;
use super::types::{ProcessingResult, TestLists, ProcessingStage, StageStatus};
use std::collections::HashMap;

#[server]
pub async fn handle_get_test_lists(file_paths: Vec<String>) -> Result<TestLists, ServerFnError> {
    use crate::api::file_operations::{get_test_lists};
    Ok(get_test_lists(file_paths).unwrap())
}

pub fn load_test_lists(
    result: RwSignal<Option<ProcessingResult>>,
    fail_to_pass_tests: RwSignal<Vec<String>>,
    pass_to_pass_tests: RwSignal<Vec<String>>,
    current_selection: RwSignal<String>,
    search_for_test: impl Fn(String) + Send + Sync + 'static + Copy,
    trigger_log_analysis: impl Fn() + Send + Sync + 'static + Copy,
    is_processing: RwSignal<bool>,
    current_stage: RwSignal<Option<ProcessingStage>>,
    stages: RwSignal<HashMap<ProcessingStage, StageStatus>>,
) {
    if result.get().is_none() {
        return;
    }
    
    let result_data = result.get().unwrap();
    if result_data.file_paths.is_empty() {
        return;
    }
    
    spawn_local(async move {
        let test_lists = handle_get_test_lists(result_data.file_paths).await;
        if let Ok(test_lists) = test_lists {
            fail_to_pass_tests.set(test_lists.fail_to_pass);
            pass_to_pass_tests.set(test_lists.pass_to_pass);
            
            // Auto-search for the first test
            let f2p_tests = fail_to_pass_tests.get();
            let p2p_tests = pass_to_pass_tests.get();
            
            if !f2p_tests.is_empty() {
                search_for_test(f2p_tests[0].clone());
            } else if !p2p_tests.is_empty() {
                current_selection.set("pass_to_pass".to_string());
                search_for_test(p2p_tests[0].clone());
            }
            
            // Complete the loading tests stage
            stages.update(|stages| {
                stages.insert(ProcessingStage::LoadingTests, StageStatus::Completed);
            });
            current_stage.set(None);
            is_processing.set(false);
            
            // Trigger log analysis after test lists are loaded
            leptos::logging::log!("Test lists loaded successfully, triggering log analysis");
            trigger_log_analysis();
        }
    });
}
