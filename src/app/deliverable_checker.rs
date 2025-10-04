use leptos::prelude::*;
use std::collections::HashMap;
use crate::components::language_selector::{ProgrammingLanguage, LanguageSelector};

use super::types::*;
use super::processing::handle_submit;
use super::file_operations::load_file_contents;
use super::test_lists::load_test_lists;
use super::search_results::search_for_test;
use super::deliverable_checker_interface::ReportCheckerInterface;

// Import for log analysis
use leptos::task::spawn_local;

#[component]
pub fn DeliverableCheckerPage() -> impl IntoView {
    let deliverable_link = RwSignal::new(String::new());
    let selected_language = RwSignal::new(ProgrammingLanguage::default());
    let is_processing = RwSignal::new(false);
    let current_stage = RwSignal::new(None::<ProcessingStage>);
    let stages = RwSignal::new(HashMap::from([
        (ProcessingStage::Validating, StageStatus::Pending),
        (ProcessingStage::Downloading, StageStatus::Pending),
        (ProcessingStage::LoadingTests, StageStatus::Pending),
    ]));
    let result = RwSignal::new(None::<ProcessingResult>);
    let error = RwSignal::new(None::<String>);
    
    // Analysis processing state
    let log_analysis_result = RwSignal::new(None::<LogAnalysisResult>);
    let log_analysis_loading = RwSignal::new(false);
    
    // Additional state for the full Report Checker functionality
    let active_tab = RwSignal::new("base".to_string());
    let active_main_tab = RwSignal::new("manual_checker".to_string());
    let file_contents = RwSignal::new(FileContents::default());
    let loading_files = RwSignal::new(false);
    let loaded_file_types = RwSignal::new(LoadedFileTypes::default());
    
    // Manual checker state
    let fail_to_pass_tests = RwSignal::new(Vec::<String>::new());
    let pass_to_pass_tests = RwSignal::new(Vec::<String>::new());
    let selected_fail_to_pass_index = RwSignal::new(0usize);
    let selected_pass_to_pass_index = RwSignal::new(0usize);
    let current_selection = RwSignal::new("fail_to_pass".to_string());
    
    // Filter state
    let fail_to_pass_filter = RwSignal::new(String::new());
    let pass_to_pass_filter = RwSignal::new(String::new());
    
    // Search results state
    let search_results = RwSignal::new(LogSearchResults {
        base_results: Vec::new(),
        before_results: Vec::new(),
        after_results: Vec::new(),
    });
    let search_result_indices = RwSignal::new(HashMap::from([
        ("base".to_string(), 0usize),
        ("before".to_string(), 0usize),
        ("after".to_string(), 0usize),
    ]));

    let _update_stage_status = move |stage: ProcessingStage, status: StageStatus| {
        stages.update(|stages| {
            stages.insert(stage, status);
        });
    };
    
    // Function to trigger log analysis when language is Rust
    let trigger_log_analysis_fn = move || {
        let language = selected_language.get();
        leptos::logging::log!("trigger_log_analysis_fn called - Language: {:?}", language);
        
        if language == ProgrammingLanguage::Rust {
            if let Some(processing_result) = result.get() {
                let file_paths = processing_result.file_paths.clone();
                leptos::logging::log!("Starting log analysis for Rust with {} files", file_paths.len());
                
                // Set loading state and clear previous results
                log_analysis_loading.set(true);
                log_analysis_result.set(None);
                
                // Call the API endpoint
                spawn_local(async move {
                    leptos::logging::log!("Calling analyze_logs API endpoint...");
                    
                    #[cfg(feature = "hydrate")]
                    {
                        let resp = gloo_net::http::Request::post("/api/analyze_logs")
                            .json(&serde_json::json!({
                                "file_paths": file_paths
                            }))
                            .unwrap()
                            .send()
                            .await;
                        
                        match resp {
                            Ok(resp) => {
                                let is_success = resp.status() >= 200 && resp.status() < 300;
                                
                                if is_success {
                                    match resp.json::<LogAnalysisResult>().await {
                                        Ok(analysis_result) => {
                                            leptos::logging::log!("Log analysis successful, got {} test statuses", analysis_result.test_statuses.len());
                                            log_analysis_result.set(Some(analysis_result));
                                        },
                                        Err(e) => {
                                            leptos::logging::log!("Failed to parse log analysis response: {:?}", e);
                                            log_analysis_result.set(None);
                                        }
                                    }
                                } else {
                                    let error_text = resp.text().await.map_err(|e| format!("Error response: {}", e));
                                    match error_text {
                                        Ok(text) => leptos::logging::log!("Analyze logs failed: {}", text),
                                        Err(e) => leptos::logging::log!("Analyze logs failed: {}", e),
                                    }
                                    log_analysis_result.set(None);
                                }
                            }
                            Err(e) => {
                                leptos::logging::log!("Analyze logs request failed: {}", e);
                                log_analysis_result.set(None);
                            }
                        }
                    }
                    
                    #[cfg(not(feature = "hydrate"))]
                    {
                        leptos::logging::log!("Client-side only operation");
                        log_analysis_result.set(None);
                    }
                    
                    log_analysis_loading.set(false);
                });
            } else {
                leptos::logging::log!("No processing result available for log analysis");
            }
        } else {
            leptos::logging::log!("Language is not Rust, clearing log analysis result");
            // Clear log analysis result for non-Rust languages
            log_analysis_result.set(None);
            log_analysis_loading.set(false);
        }
    };

    // Helper functions for the enhanced functionality
    let load_file_contents_fn = move || {
        load_file_contents(result.clone(), file_contents.clone(), loading_files.clone(), loaded_file_types.clone());
    };
    
    let search_for_test_fn = move |test_name: String| {
        search_for_test(result, test_name, search_results, search_result_indices);
    };
    
    let load_test_lists_fn = move || {
        load_test_lists(result, fail_to_pass_tests, pass_to_pass_tests, current_selection, search_for_test_fn, trigger_log_analysis_fn, is_processing, current_stage, stages);
    };

    let handle_submit_fn = move || {
        handle_submit(
            deliverable_link,
            selected_language,
            is_processing,
            current_stage,
            stages,
            result,
            error,
            load_test_lists_fn,
        );
    };

    let reset_state = move || {
        deliverable_link.set(String::new());
        selected_language.set(ProgrammingLanguage::default());
        is_processing.set(false);
        current_stage.set(None);
        stages.set(HashMap::from([
            (ProcessingStage::Validating, StageStatus::Pending),
            (ProcessingStage::Downloading, StageStatus::Pending),
            (ProcessingStage::LoadingTests, StageStatus::Pending),
        ]));
        result.set(None);
        error.set(None);
        
        // Reset additional state
        active_tab.set("base".to_string());
        active_main_tab.set("manual_checker".to_string());
        file_contents.set(FileContents::default());
        loading_files.set(false);
        loaded_file_types.set(LoadedFileTypes::default());
        fail_to_pass_tests.set(Vec::new());
        pass_to_pass_tests.set(Vec::new());
        selected_fail_to_pass_index.set(0);
        selected_pass_to_pass_index.set(0);
        current_selection.set("fail_to_pass".to_string());
        fail_to_pass_filter.set(String::new());
        pass_to_pass_filter.set(String::new());
        search_results.set(LogSearchResults {
            base_results: Vec::new(),
            before_results: Vec::new(),
            after_results: Vec::new(),
        });
        search_result_indices.set(HashMap::from([
            ("base".to_string(), 0usize),
            ("before".to_string(), 0usize),
            ("after".to_string(), 0usize),
        ]));
        log_analysis_result.set(None);
        log_analysis_loading.set(false);
    };


    view! {
        <div class="w-full h-full">
            <Show
                when=move || result.get().is_some() && (!fail_to_pass_tests.get().is_empty() || !pass_to_pass_tests.get().is_empty())
                fallback=move || view! {
                    <div class="w-full flex flex-col h-full items-center justify-center">
            <div class="w-full">
                <div class="p-8">

                    <div class="text-center">
                        <h2 class="text-3xl font-bold text-gray-900 dark:text-white mb-8">
                            Deliverable Checker
                        </h2>

                        <div class="mb-8 space-y-6 flex flex-col items-center">
                            <div>
                                <LanguageSelector 
                                    selected_language=selected_language
                                    disabled=is_processing
                                />
                            </div>
                            <div class="w-full max-w-2xl">
                                <input
                                    type="text"
                                    prop:value=move || deliverable_link.get()
                                    on:input=move |ev| deliverable_link.set(event_target_value(&ev))
                                    placeholder="Enter Google Drive folder link"
                                    class="w-full px-6 py-4 text-lg border-2 border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:border-blue-500 dark:focus:border-blue-400 transition-colors"
                                    disabled=move || is_processing.get()
                                />
                            </div>
                        </div>

                        <div class="flex gap-4 justify-center">
                            <button
                                on:click=move |_| handle_submit_fn()
                                disabled=move || is_processing.get() || deliverable_link.get().trim().is_empty()
                                class="px-8 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded-full text-lg font-semibold shadow-lg transition-colors disabled:cursor-not-allowed"
                            >
                                Submit
                            </button>
                        </div>

                        {move || error.get().map(|err|
                            view! {
                                <div class="mt-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                                    <p class="text-red-600 dark:text-red-400">{err}</p>
                                </div>
                            }
                        )}
                    </div>

                    {move || {
                                    if is_processing.get() {
                        view! {
                            <div class="text-center mt-12 pt-8 border-t border-gray-200 dark:border-gray-700">
                                <h3 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">
                                    Processing Deliverable
                                </h3>

                                <div class="space-y-6">
                                    <div class="flex items-center justify-center gap-4">
                                        {render_icon(ProcessingStage::Validating, stages.get().get(&ProcessingStage::Validating).cloned().unwrap_or(StageStatus::Pending))}
                                        <span class=move || {
                                            let status = stages.get().get(&ProcessingStage::Validating).cloned().unwrap_or(StageStatus::Pending);
                                            format!("text-lg font-medium {}", get_stage_text_class(status))
                                        }>
                                            Validating
                                        </span>
                                    </div>

                                    <div class="flex items-center justify-center gap-4">
                                        {render_icon(ProcessingStage::Downloading, stages.get().get(&ProcessingStage::Downloading).cloned().unwrap_or(StageStatus::Pending))}
                                        <span class=move || {
                                            let status = stages.get().get(&ProcessingStage::Downloading).cloned().unwrap_or(StageStatus::Pending);
                                            format!("text-lg font-medium {}", get_stage_text_class(status))
                                        }>
                                            Downloading
                                        </span>
                                    </div>

                                    <div class="flex items-center justify-center gap-4">
                                        {render_icon(ProcessingStage::LoadingTests, stages.get().get(&ProcessingStage::LoadingTests).cloned().unwrap_or(StageStatus::Pending))}
                                        <span class=move || {
                                            let status = stages.get().get(&ProcessingStage::LoadingTests).cloned().unwrap_or(StageStatus::Pending);
                                            format!("text-lg font-medium {}", get_stage_text_class(status))
                                        }>
                                            Loading tests
                                        </span>
                                    </div>
                                </div>
                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                        }
                    }}

                            </div>
                        </div>
                    </div>
                }
            >
                // Report Checker Interface after successful download
                <ReportCheckerInterface 
                    fail_to_pass_tests=fail_to_pass_tests
                    pass_to_pass_tests=pass_to_pass_tests
                    current_selection=current_selection
                    selected_fail_to_pass_index=selected_fail_to_pass_index
                    selected_pass_to_pass_index=selected_pass_to_pass_index
                    fail_to_pass_filter=fail_to_pass_filter
                    pass_to_pass_filter=pass_to_pass_filter
                    search_for_test=search_for_test_fn
                    active_tab=active_tab
                    active_main_tab=active_main_tab
                    search_results=search_results
                    search_result_indices=search_result_indices
                    file_contents=file_contents
                    loading_files=loading_files
                    loaded_file_types=loaded_file_types
                    result=result
                    reset_state=reset_state
                    selected_language=selected_language
                    log_analysis_result=log_analysis_result
                    log_analysis_loading=log_analysis_loading
                />
            </Show>
        </div>
    }
}


fn render_icon(_stage: ProcessingStage, status: StageStatus) -> impl IntoView {
    match status {
        StageStatus::Completed => view! {
            <div class="w-5 h-5">
                <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
            </div>
        },
        StageStatus::Active => view! {
            <div class="w-5 h-5">
                <svg class="animate-spin text-blue-500" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
            </div>
        },
        StageStatus::Error => view! {
            <div class="w-5 h-5">
                <svg class="w-5 h-5 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </div>
        },
        StageStatus::Pending => view! {
            <div class="w-5 h-5">
                <svg class="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
            </div>
        },
    }
}

fn get_stage_text_class(status: StageStatus) -> &'static str {
    match status {
        StageStatus::Completed => "text-green-600 dark:text-green-400",
        StageStatus::Active => "text-blue-600 dark:text-blue-400",
        StageStatus::Error => "text-red-600 dark:text-red-400",
        StageStatus::Pending => "text-gray-400 dark:text-gray-500",
    }
}