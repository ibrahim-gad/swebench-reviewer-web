use leptos::prelude::*;
use leptos::prelude::Effect;
use std::collections::HashMap;

use super::types::*;
use super::processing::handle_submit;
use super::file_operations::load_file_contents;
use super::test_lists::load_test_lists;
use super::search_results::search_for_test;
use super::deliverable_checker_interface::DeliverableCheckerInterface;
use leptos::Params;
use leptos_router::params::Params;
use leptos_router::hooks::use_params;
use leptos_router::hooks::use_navigate;

use leptos::task::spawn_local;

#[derive(Params, PartialEq)]
struct DeliverableCheckerParams {
    deliverable_id: Option<String>,
}
#[server]
pub async fn handle_analyze_logs(file_paths: Vec<String>) -> Result<LogAnalysisResult, ServerFnError> {
    use crate::api::log_analysis::{analyze_logs};
    Ok(analyze_logs(file_paths).unwrap())
}

#[component]
pub fn DeliverableCheckerPage(current_deliverable: RwSignal<Option<ProcessingResult>>) -> impl IntoView {
    let params = use_params::<DeliverableCheckerParams>();
    let deliverable_id = 
        params
            .read()
            .as_ref()
            .ok()
            .and_then(|params| if let Some(deliverable_id) = &params.deliverable_id {
                Some(format!("https://drive.google.com/drive/folders/{}", deliverable_id))
            } else {
                None
            })
            .unwrap_or_default();
            leptos::logging::log!("Deliverable ID: {}", deliverable_id);
    let initial_deliverable_link = RwSignal::new(deliverable_id.clone());
    let deliverable_link = RwSignal::new(deliverable_id);
    let is_processing = RwSignal::new(false);
    let current_stage = RwSignal::new(None::<ProcessingStage>);
    let stages = RwSignal::new(HashMap::from([
        (ProcessingStage::Validating, StageStatus::Pending),
        (ProcessingStage::Downloading, StageStatus::Pending),
        (ProcessingStage::LoadingTests, StageStatus::Pending),
    ]));
    let result = RwSignal::new(None::<ProcessingResult>);
    let error = RwSignal::new(None::<String>);

    let log_analysis_result = RwSignal::new(None::<LogAnalysisResult>);
    let log_analysis_loading = RwSignal::new(false);
    
    let active_tab = RwSignal::new("base".to_string());
    let active_main_tab = RwSignal::new("manual_checker".to_string());
    let file_contents = RwSignal::new(FileContents::default());
    let loading_files = RwSignal::new(false);
    let loaded_file_types = RwSignal::new(LoadedFileTypes::default());
    
    let fail_to_pass_tests = RwSignal::new(Vec::<String>::new());
    let pass_to_pass_tests = RwSignal::new(Vec::<String>::new());
    let selected_fail_to_pass_index = RwSignal::new(0usize);
    let selected_pass_to_pass_index = RwSignal::new(0usize);
    let current_selection = RwSignal::new("fail_to_pass".to_string());
    
    let fail_to_pass_filter = RwSignal::new(String::new());
    let pass_to_pass_filter = RwSignal::new(String::new());
    
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
    
    let report_selected_test_name = RwSignal::new(String::new());

    let _update_stage_status = move |stage: ProcessingStage, status: StageStatus| {
        stages.update(|stages| {
            stages.insert(stage, status);
        });
    };
    
    let trigger_log_analysis_fn = move || {
            if let Some(processing_result) = result.get() {
                let file_paths = processing_result.file_paths.clone();
                leptos::logging::log!("Starting log analysis for Rust with {} files", file_paths.len());
                
                log_analysis_loading.set(true);
                log_analysis_result.set(None);
                
                spawn_local(async move {
                    leptos::logging::log!("Calling analyze_logs API endpoint...");
                    let resp = handle_analyze_logs(file_paths).await;
                    match resp {
                        Ok(analysis_result) => {
                            log_analysis_result.set(Some(analysis_result));
                        },
                        Err(e) => {
                            leptos::logging::log!("Failed to parse log analysis response: {:?}", e);
                            log_analysis_result.set(None);
                        }
                    }
                    log_analysis_loading.set(false);
                });
            } else {
                leptos::logging::log!("No processing result available for log analysis");
            }
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
            is_processing,
            current_stage,
            stages,
            result,
            error,
            load_test_lists_fn,
        );
    };

    let manual_submit_fn = move |_| {
        let link = deliverable_link.get().trim().to_string();
        if link.is_empty() {
            return;
        }

        if link.contains("drive.google.com/drive/folders/") {
            let folder_id = link.split("folders/").nth(1)
                .and_then(|s| s.split(|c| c == '/' || c == '?').next())
                .unwrap_or("")
                .to_string();

            if !folder_id.is_empty() && folder_id.len() >= 20 { // Basic ID length check
                leptos::logging::log!("Manual submit: extracted folder ID {}, redirecting", folder_id);
                
                let navigate_fn = use_navigate();
                
                is_processing.set(true);
                error.set(None);
                result.set(None);
                initial_deliverable_link.set(format!("https://drive.google.com/drive/folders/{}", folder_id.clone()));
                navigate_fn(&format!("/{}", folder_id), Default::default());
                is_processing.set(false);
            } else {
                error.set(Some("Invalid folder ID extracted from link".to_string()));
            }
        } else {
            error.set(Some("Please enter a valid Google Drive folder link (https://drive.google.com/drive/folders/...".to_string()));
        }
    };

    let reset_state = move || {
        deliverable_link.set(String::new());
        is_processing.set(false);
        current_stage.set(None);
        stages.set(HashMap::from([
            (ProcessingStage::Validating, StageStatus::Pending),
            (ProcessingStage::Downloading, StageStatus::Pending),
            (ProcessingStage::LoadingTests, StageStatus::Pending),
        ]));
        result.set(None);
        error.set(None);
        
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
        report_selected_test_name.set(String::new());
    };

    Effect::new(move |_| {
        let link = deliverable_link.get();
        let initial_link = initial_deliverable_link.get();
        if !initial_link.is_empty() 
            && link == initial_link 
            && !is_processing.get() 
            && result.get().is_none() 
            && deliverable_link.get().starts_with("https://drive.google.com/drive/folders/") {
            leptos::logging::log!("Auto-submitting for deliverable from route: {}", link);
            initial_deliverable_link.set(String::new());
            handle_submit_fn();
        }
    });

    Effect::new(move |_| {
        if result.with_untracked(|r| r.is_some()) {
            let is_loaded = loaded_file_types.with_untracked(|loaded| loaded.is_loaded("main_json"));
            let is_loading = loading_files.with_untracked(|l| *l);
            let has_main_json = file_contents.with_untracked(|fc| fc.main_json.is_some());
            
            if !is_loading && !has_main_json && !is_loaded {
                leptos::logging::log!("Loading main json");
                load_file_contents(result.clone(), file_contents.clone(), loading_files.clone(), loaded_file_types.clone(), Some(vec!["main_json".to_string()]));
            }
        }
    });

    Effect::new(move |_| {
        if let Some(r) = result.get() {
            current_deliverable.set(Some(r.clone()));
        } else {
            current_deliverable.set(None);
        }
    });

    Effect::new(move |_| {
        if let Some(mut r) = result.get().clone() {
            let has_main_json = file_contents.with_untracked(|fc| fc.main_json.clone());
            
            if r.instance_id.is_empty() && has_main_json.is_some() {
                if let Some(main_json) = &has_main_json {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&main_json.content) {
                        let instance_id = json.get("instance_id").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
                        let task_id = json.get("task_id").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
                        let repo = json.get("repo").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
                        let problem_statement = json.get("problem_statement").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
                        let conversation: Vec<super::types::ConversationEntry> = json
                            .get("conversation")
                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                            .unwrap_or_default();
                        let gold_patch = json.get("gold_patch").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
                        let test_patch = json.get("test_patch").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
                        
                        r.instance_id = instance_id;
                        r.task_id = task_id;
                        r.repo = repo;
                        r.problem_statement = problem_statement;
                        r.conversation = conversation;
                        r.gold_patch = gold_patch;
                        r.test_patch = test_patch;
                        // Persist parsed identifiers for convenience
                        r.pr_id = r
                            .instance_id
                            .split('-')
                            .last()
                            .unwrap_or("")
                            .to_string();
                        r.issue_id = r
                            .task_id
                            .split('#')
                            .last()
                            .unwrap_or("")
                            .to_string();
                        r.language = json.get("language").and_then(|v| v.as_str()).map(|s| s.to_string().to_lowercase()).unwrap_or_default();
                        result.set(Some(r));
                    }
                }
            }
        }
    });

    // Reduce nested Show closure depth by erasing types on branches
    // Build landing view as a type-erased boundary to reduce monomorphization depth
    let landing_view = move || -> AnyView {
        view! {
            <div class="w-full flex flex-col h-full items-center justify-center pb-20">
                <div class="w-full">
                    <div class="p-8">

                        <div class="text-center">
                            <h2 class="text-3xl font-bold text-gray-900 dark:text-white mb-8">
                                Deliverable Checker
                            </h2>

                            <div class="mb-8 space-y-6 flex flex-col items-center">

                                <div class="w-full max-w-2xl">
                                    <input
                                        type="text"
                                        prop:value=move || deliverable_link.get()
                                        on:input=move |ev| {
                                            deliverable_link.set(event_target_value(&ev))
                                        }
                                        placeholder="Enter Google Drive folder link"
                                        class="w-full px-4 py-2 text-md border-2 border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:border-blue-500 dark:focus:border-blue-400 transition-colors"
                                        disabled=move || is_processing.get()
                                    />
                                </div>
                            </div>

                            <div class="flex gap-4 justify-center">
                                <button
                                    on:click=manual_submit_fn
                                    disabled=move || {
                                        is_processing.get()
                                            || deliverable_link.get().trim().is_empty()
                                    }
                                    class="px-8 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded-full text-lg font-semibold shadow-lg transition-colors disabled:cursor-not-allowed"
                                >
                                    Submit
                                </button>
                            </div>

                            {move || {
                                error
                                    .get()
                                    .map(|err| {
                                        view! {
                                            <div class="flex gap-4 justify-center">
                                            <div class="w-full max-w-2xl mt-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                                                <p class="text-red-600 dark:text-red-400">{err}</p>
                                            </div>
                                            </div>
                                        }
                                    }).into_any()
                            }}
                        </div>

                        {move || {
                            if is_processing.get() {
                                view! {
                                    <div class="max-w-2xl mx-auto text-center mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
                                        <div class="space-x-6 flex flex-row justify-center">
                                            <div class="flex items-center justify-center gap-2">
                                                {render_icon(
                                                    ProcessingStage::Validating,
                                                    stages
                                                        .get()
                                                        .get(&ProcessingStage::Validating)
                                                        .cloned()
                                                        .unwrap_or(StageStatus::Pending),
                                                )}
                                                <span class=move || {
                                                    let status = stages
                                                        .get()
                                                        .get(&ProcessingStage::Validating)
                                                        .cloned()
                                                        .unwrap_or(StageStatus::Pending);
                                                    format!(
                                                        "text-lg font-medium {}",
                                                        get_stage_text_class(status),
                                                    )
                                                }>Validating</span>
                                            </div>

                                            <div class="flex items-center justify-center gap-2">
                                                {render_icon(
                                                    ProcessingStage::Downloading,
                                                    stages
                                                        .get()
                                                        .get(&ProcessingStage::Downloading)
                                                        .cloned()
                                                        .unwrap_or(StageStatus::Pending),
                                                )}
                                                <span class=move || {
                                                    let status = stages
                                                        .get()
                                                        .get(&ProcessingStage::Downloading)
                                                        .cloned()
                                                        .unwrap_or(StageStatus::Pending);
                                                    format!(
                                                        "text-lg font-medium {}",
                                                        get_stage_text_class(status),
                                                    )
                                                }>Downloading</span>
                                            </div>

                                            <div class="flex items-center justify-center gap-2">
                                                {render_icon(
                                                    ProcessingStage::LoadingTests,
                                                    stages
                                                        .get()
                                                        .get(&ProcessingStage::LoadingTests)
                                                        .cloned()
                                                        .unwrap_or(StageStatus::Pending),
                                                )}
                                                <span class=move || {
                                                    let status = stages
                                                        .get()
                                                        .get(&ProcessingStage::LoadingTests)
                                                        .cloned()
                                                        .unwrap_or(StageStatus::Pending);
                                                    format!(
                                                        "text-lg font-medium {}",
                                                        get_stage_text_class(status),
                                                    )
                                                }>Loading tests</span>
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
        }.into_any()
    };

    view! {
        <div class="w-full h-full">
            <Show
                when=move || {
                    result.get().is_some()
                        && (!fail_to_pass_tests.get().is_empty()
                            || !pass_to_pass_tests.get().is_empty())
                }
                fallback=move || landing_view()
            >
                // Report Checker Interface after successful download
                <DeliverableCheckerInterface
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
                    reset_state=reset_state
                    log_analysis_result=log_analysis_result
                    log_analysis_loading=log_analysis_loading
                    loaded_file_types=loaded_file_types
                    result=result
                    report_selected_test_name=report_selected_test_name
                />
            </Show>
        </div>
    }
}

fn render_icon(_stage: ProcessingStage, status: StageStatus) -> AnyView {
    match status {
        StageStatus::Completed => view! {
            <div class="w-5 h-5">
                <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M5 13l4 4L19 7"
                    />
                </svg>
            </div>
        }.into_any(),
        StageStatus::Active => view! {
            <div class="w-5 h-5">
                <svg class="animate-spin text-blue-500" fill="none" viewBox="0 0 24 24">
                    <circle
                        class="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        stroke-width="4"
                    ></circle>
                    <path
                        class="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    ></path>
                </svg>
            </div>
        }.into_any(),
        StageStatus::Error => view! {
            <div class="w-5 h-5">
                <svg
                    class="w-5 h-5 text-red-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                    />
                </svg>
            </div>
        }.into_any(),
        StageStatus::Pending => view! {
            <div class="w-5 h-5">
                <svg
                    class="w-5 h-5 text-gray-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                </svg>
            </div>
        }.into_any(),
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