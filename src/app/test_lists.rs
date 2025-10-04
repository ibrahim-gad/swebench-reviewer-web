use leptos::prelude::*;
use leptos::task::spawn_local;
use super::types::{ProcessingResult, TestLists};

pub fn load_test_lists(
    result: RwSignal<Option<ProcessingResult>>,
    fail_to_pass_tests: RwSignal<Vec<String>>,
    pass_to_pass_tests: RwSignal<Vec<String>>,
    current_selection: RwSignal<String>,
    search_for_test: impl Fn(String) + Send + Sync + 'static + Copy,
    trigger_log_analysis: impl Fn() + Send + Sync + 'static + Copy,
) {
    if result.get().is_none() {
        return;
    }
    
    let result_data = result.get().unwrap();
    if result_data.file_paths.is_empty() {
        return;
    }
    
    spawn_local(async move {
        #[cfg(feature = "hydrate")]
        if let Ok(response) = gloo_net::http::Request::post("/api/get_test_lists")
            .json(&serde_json::json!({
                "file_paths": result_data.file_paths
            }))
            .unwrap()
            .send()
            .await
        {
            if response.ok() {
                if let Ok(test_lists) = response.json::<TestLists>().await {
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
                    
                    // Trigger log analysis after test lists are loaded
                    leptos::logging::log!("Test lists loaded successfully, triggering log analysis");
                    trigger_log_analysis();
                }
            }
        }
    });
}
