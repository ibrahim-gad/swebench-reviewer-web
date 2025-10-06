use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use super::types::{LogSearchResults, ProcessingResult};

#[server]
pub async fn handle_search_logs(file_paths: Vec<String>, test_name: String) -> Result<LogSearchResults, ServerFnError> {
    use crate::api::log_analysis::{search_logs};
    Ok(search_logs(file_paths, test_name).unwrap())
}

pub fn search_for_test(
    result: RwSignal<Option<ProcessingResult>>,
    test_name: String,
    search_results: RwSignal<LogSearchResults>,
    search_result_indices: RwSignal<HashMap<String, usize>>,
) {
    if result.get().is_none() {
        return;
    }
    
    let result_data = result.get().unwrap();
    if result_data.file_paths.is_empty() {
        return;
    }
    
    spawn_local(async move {
            let results = handle_search_logs(result_data.file_paths, test_name).await;
            if let Ok(results) = results {
                search_results.set(results);
                search_result_indices.set(HashMap::from([
                    ("base".to_string(), 0usize),
                    ("before".to_string(), 0usize),
                    ("after".to_string(), 0usize),
                ]));
            }
    });
}

pub fn navigate_search_result(
    log_type: &str,
    direction: &str,
    search_results: RwSignal<LogSearchResults>,
    search_result_indices: RwSignal<HashMap<String, usize>>,
) {
    let mut indices = search_result_indices.get();
    let current_index = indices.get(log_type).copied().unwrap_or(0);
    let results = search_results.get();
    
    let max_index = match log_type {
        "base" => results.base_results.len().saturating_sub(1),
        "before" => results.before_results.len().saturating_sub(1),
        "after" => results.after_results.len().saturating_sub(1),
        _ => 0,
    };
    
    let new_index = match direction {
        "prev" => current_index.saturating_sub(1),
        "next" => (current_index + 1).min(max_index),
        _ => current_index,
    };
    
    indices.insert(log_type.to_string(), new_index);
    search_result_indices.set(indices);
}
