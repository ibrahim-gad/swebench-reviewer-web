use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ValidationResult {
    pub files_to_download: Vec<FileInfo>,
    pub folder_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadResult {
    pub temp_directory: String,
    pub downloaded_files: Vec<FileInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProcessingResult {
    pub status: String,
    pub message: String,
    pub files_processed: usize,
    pub issues_found: usize,
    pub score: usize,
    pub file_paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TestLists {
    pub fail_to_pass: Vec<String>,
    pub pass_to_pass: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub line_number: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogSearchResults {
    pub base_results: Vec<SearchResult>,
    pub before_results: Vec<SearchResult>,
    pub after_results: Vec<SearchResult>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileContent {
    pub content: String,
    pub file_type: String, // "text" | "json"
}

#[derive(Clone, Default)]
pub struct FileContents {
    pub base: Option<FileContent>,
    pub before: Option<FileContent>,
    pub after: Option<FileContent>,
    pub agent: Option<FileContent>,
    pub main_json: Option<FileContent>,
    pub analysis: Option<FileContent>,
    pub base_analysis: Option<FileContent>,
    pub before_analysis: Option<FileContent>,
    pub after_analysis: Option<FileContent>,
    pub agent_analysis: Option<FileContent>,
}

impl FileContents {
    pub fn get(&self, key: &str) -> Option<&FileContent> {
        match key {
            "base" => self.base.as_ref(),
            "before" => self.before.as_ref(),
            "after" => self.after.as_ref(),
            "agent" => self.agent.as_ref(),
            "main_json" => self.main_json.as_ref(),
            "analysis" => self.analysis.as_ref(),
            "base_analysis" => self.base_analysis.as_ref(),
            "before_analysis" => self.before_analysis.as_ref(),
            "after_analysis" => self.after_analysis.as_ref(),
            "agent_analysis" => self.agent_analysis.as_ref(),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ProcessingStage {
    Validating,
    Downloading,
}

#[derive(Clone, PartialEq, Eq)]
enum StageStatus {
    Pending,
    Active,
    Completed,
    Error,
}

#[component]
pub fn ReportCheckerPage() -> impl IntoView {
    let deliverable_link = RwSignal::new(String::new());
    let is_processing = RwSignal::new(false);
    let current_stage = RwSignal::new(None::<ProcessingStage>);
    let stages = RwSignal::new(HashMap::from([
        (ProcessingStage::Validating, StageStatus::Pending),
        (ProcessingStage::Downloading, StageStatus::Pending),
    ]));
    let result = RwSignal::new(None::<ProcessingResult>);
    let error = RwSignal::new(None::<String>);
    
    // Additional state for the full Report Checker functionality
    let active_tab = RwSignal::new("base".to_string());
    let active_main_tab = RwSignal::new("manual_checker".to_string());
    let file_contents = RwSignal::new(FileContents::default());
    let loading_files = RwSignal::new(false);
    
    // Manual checker state
    let fail_to_pass_tests = RwSignal::new(Vec::<String>::new());
    let pass_to_pass_tests = RwSignal::new(Vec::<String>::new());
    let selected_fail_to_pass_index = RwSignal::new(0usize);
    let selected_pass_to_pass_index = RwSignal::new(0usize);
    let current_selection = RwSignal::new("fail_to_pass".to_string());
    
    // Analysis state
    let is_analyzing = RwSignal::new(false);
    let analysis_result = RwSignal::new(None::<serde_json::Value>);
    
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

    let update_stage_status = move |stage: ProcessingStage, status: StageStatus| {
        stages.update(|stages| {
            stages.insert(stage, status);
        });
    };
    
    // Helper functions for the enhanced functionality
    let load_file_contents = move || {
        if result.get().is_none() {
            return;
        }
        
        let result_data = result.get().unwrap();
        if result_data.file_paths.is_empty() {
            return;
        }
        
        loading_files.set(true);
        
        spawn_local(async move {
            let mut contents = FileContents::default();
            
            // Load each file type
            let _file_types = vec!["base", "before", "after", "agent", "main_json"];
            
            for _file_type in _file_types {
                #[cfg(feature = "hydrate")]
                if let Ok(response) = gloo_net::http::Request::post("/api/get_file_content")
                    .json(&serde_json::json!({
                        "file_type": _file_type,
                        "file_paths": result_data.file_paths
                    }))
                    .unwrap()
                    .send()
                    .await
                {
                    if response.ok() {
                        if let Ok(content) = response.text().await {
                            let file_content = FileContent {
                                content,
                                file_type: if _file_type.contains("json") { "json" } else { "text" }.to_string(),
                            };
                            
                            match _file_type {
                                "base" => contents.base = Some(file_content),
                                "before" => contents.before = Some(file_content),
                                "after" => contents.after = Some(file_content),
                                "agent" => contents.agent = Some(file_content),
                                "main_json" => contents.main_json = Some(file_content),
                                _ => {}
                            }
                        }
                    }
                }
            }
            
            file_contents.set(contents);
            loading_files.set(false);
        });
    };
    
    let search_for_test = move |test_name: String| {
        if result.get().is_none() {
            return;
        }
        
        let result_data = result.get().unwrap();
        if result_data.file_paths.is_empty() {
            return;
        }
        
        spawn_local(async move {
            #[cfg(feature = "hydrate")]
            if let Ok(response) = gloo_net::http::Request::post("/api/search_logs")
                .json(&serde_json::json!({
                    "file_paths": result_data.file_paths,
                    "test_name": test_name
                }))
                .unwrap()
                .send()
                .await
            {
                if response.ok() {
                    if let Ok(results) = response.json::<LogSearchResults>().await {
                        search_results.set(results);
                        search_result_indices.set(HashMap::from([
                            ("base".to_string(), 0usize),
                            ("before".to_string(), 0usize),
                            ("after".to_string(), 0usize),
                        ]));
                    }
                }
            }
        });
    };
    
    let load_test_lists = move || {
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
                            search_for_test.clone()(f2p_tests[0].clone());
                        } else if !p2p_tests.is_empty() {
                            current_selection.set("pass_to_pass".to_string());
                            search_for_test.clone()(p2p_tests[0].clone());
                        }
                    }
                }
            }
        });
    };
    
    let _start_analysis = move || {
        if result.get().is_none() {
            return;
        }
        
        let result_data = result.get().unwrap();
        if result_data.file_paths.is_empty() {
            return;
        }
        
        is_analyzing.set(true);
        error.set(None);
        
        spawn_local(async move {
            #[cfg(feature = "hydrate")]
            if let Ok(response) = gloo_net::http::Request::post("/api/analyze_logs")
                .json(&serde_json::json!({
                    "file_paths": result_data.file_paths
                }))
                .unwrap()
                .send()
                .await
            {
                if response.ok() {
                    if let Ok(analysis_data) = response.json::<serde_json::Value>().await {
                        analysis_result.set(Some(analysis_data));
                    }
                } else {
                    error.set(Some("Analysis failed".to_string()));
                }
            } else {
                error.set(Some("Failed to start analysis".to_string()));
            }
            
            is_analyzing.set(false);
        });
    };

    let handle_submit = move || {
        let link = deliverable_link.get().trim().to_string();
        if link.is_empty() {
            error.set(Some("Please enter a deliverable link".to_string()));
            return;
        }

        is_processing.set(true);
        error.set(None);
        result.set(None);

        spawn_local(async move {
            // Stage 1: Validating
            current_stage.set(Some(ProcessingStage::Validating));
            update_stage_status(ProcessingStage::Validating, StageStatus::Active);

            let validation_result: Result<ValidationResult, String> = async {
                #[cfg(feature = "hydrate")]
                {
                    let resp = gloo_net::http::Request::post("/api/validate")
                        .json(&serde_json::json!({ "folder_link": link }))
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
                            };

                            result.set(Some(processing_result.clone()));
                            current_stage.set(None);
                            
                            // After successful download, load additional data
                            load_file_contents();
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
                }
            }

            is_processing.set(false);
        });
    };

    let reset_state = move || {
        deliverable_link.set(String::new());
        is_processing.set(false);
        current_stage.set(None);
        stages.set(HashMap::from([
            (ProcessingStage::Validating, StageStatus::Pending),
            (ProcessingStage::Downloading, StageStatus::Pending),
        ]));
        result.set(None);
        error.set(None);
        
        // Reset additional state
        active_tab.set("base".to_string());
        active_main_tab.set("manual_checker".to_string());
        file_contents.set(FileContents::default());
        loading_files.set(false);
        fail_to_pass_tests.set(Vec::new());
        pass_to_pass_tests.set(Vec::new());
        selected_fail_to_pass_index.set(0);
        selected_pass_to_pass_index.set(0);
        current_selection.set("fail_to_pass".to_string());
        is_analyzing.set(false);
        analysis_result.set(None);
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
    };

    view! {
        <div class="w-full h-full">
            <Show
                when=move || result.get().is_some() && (!fail_to_pass_tests.get().is_empty() || !pass_to_pass_tests.get().is_empty())
                fallback=move || view! {
                    <div class="w-full flex flex-col h-full items-center justify-center">
            <div class="w-full max-w-2xl">
                <div class="p-8">

                    <div class="text-center">
                        <h2 class="text-3xl font-bold text-gray-900 dark:text-white mb-8">
                            Deliverable Checker
                        </h2>

                        <div class="mb-8">
                            <input
                                type="text"
                                prop:value=move || deliverable_link.get()
                                on:input=move |ev| deliverable_link.set(event_target_value(&ev))
                                placeholder="Deliverable Link"
                                class="w-full px-6 py-4 text-lg border-2 border-gray-300 dark:border-gray-600 rounded-full bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:border-blue-500 dark:focus:border-blue-400 transition-colors"
                                disabled=move || is_processing.get()
                            />
                        </div>

                        <button
                            on:click=move |_| handle_submit()
                            disabled=move || is_processing.get() || deliverable_link.get().trim().is_empty()
                            class="px-8 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded-full text-lg font-semibold shadow-lg transition-colors disabled:cursor-not-allowed"
                        >
                            Submit
                        </button>

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
                                </div>
                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                        }
                    }}

                                {move || result.get().map(|res| {
                                    // Only show the completion message if we haven't loaded test lists yet
                                    if fail_to_pass_tests.get().is_empty() && pass_to_pass_tests.get().is_empty() {
                        view! {
                            <div class="mt-8 p-6 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
                                <h3 class="text-xl font-semibold text-green-900 dark:text-green-100 mb-4">
                                                    Processing Complete - Loading Test Data...
                                </h3>
                                <p class="text-green-700 dark:text-green-300 mb-2">
                                    Status: {res.status}
                                </p>
                                <p class="text-green-700 dark:text-green-300 mb-2">
                                    Message: {res.message}
                                </p>
                                <p class="text-green-700 dark:text-green-300 mb-2">
                                    Files Processed: {res.files_processed}
                                </p>
                                <p class="text-green-700 dark:text-green-300 mb-4">
                                    Issues Found: {res.issues_found}
                                </p>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }
                                })}
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
                    search_for_test=search_for_test
                    active_tab=active_tab
                    search_results=search_results
                    search_result_indices=search_result_indices
                    reset_state=reset_state
                />
            </Show>
        </div>
    }
}

#[component]
fn ReportCheckerInterface(
    fail_to_pass_tests: RwSignal<Vec<String>>,
    pass_to_pass_tests: RwSignal<Vec<String>>,
    current_selection: RwSignal<String>,
    selected_fail_to_pass_index: RwSignal<usize>,
    selected_pass_to_pass_index: RwSignal<usize>,
    fail_to_pass_filter: RwSignal<String>,
    pass_to_pass_filter: RwSignal<String>,
    search_for_test: impl Fn(String) + Send + Sync + 'static + Copy,
    active_tab: RwSignal<String>,
    search_results: RwSignal<LogSearchResults>,
    search_result_indices: RwSignal<HashMap<String, usize>>,
    reset_state: impl Fn() + Send + Sync + 'static + Copy,
) -> impl IntoView {
    // Navigation functions for search results
    let navigate_search_result = move |log_type: &str, direction: &str| {
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
    };

    view! {
        <div class="flex flex-col h-full overflow-hidden">
            <div class="flex-none bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 px-4 py-1 shadow-sm mb-1">
                // Single line with back button, centered title, and copy functionality
                <div class="flex items-center justify-between relative">
                    // Back button
                                <button
                                    on:click=move |_| reset_state()
                        class="flex items-center gap-2 transition-colors text-sm whitespace-nowrap text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300"
                                >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
                        </svg>
                        Check another
                                </button>

                    // Title - Centered
                    <div class="flex justify-center absolute left-1/2 transform -translate-x-1/2">
                        <h1 class="text-lg font-semibold text-gray-900 dark:text-white">
                            Tests Checker
                        </h1>
                </div>

                    // Copy Selected Test Name
                    <div class="flex flex-col gap-0">
                        <div class="flex items-center gap-2">
                            <span class="text-sm text-gray-600 dark:text-gray-400 font-mono max-w-xs truncate">
                                {move || {
                                    if current_selection.get() == "fail_to_pass" {
                                        let f2p_tests = fail_to_pass_tests.get();
                                        let index = selected_fail_to_pass_index.get();
                                        if index < f2p_tests.len() {
                                            f2p_tests[index].clone()
                                        } else {
                                            String::new()
                                        }
                                    } else {
                                        let p2p_tests = pass_to_pass_tests.get();
                                        let index = selected_pass_to_pass_index.get();
                                        if index < p2p_tests.len() {
                                            p2p_tests[index].clone()
                                        } else {
                                            String::new()
                                        }
                                    }
                                }}
                            </span>
                            <button
                                class="p-1.5 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
                                title="Copy test name"
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            // Main Content
            <div class="flex-1 overflow-hidden bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm">
                // Test Lists Section (Top Half)
                <div class="h-1/2 border-b border-gray-200 dark:border-gray-700">
                    <div class="h-full flex flex-row">
                        // Fail to Pass Tests
                        <div class="w-1/2 border-r border-gray-200 dark:border-gray-700 flex flex-col">
                            <div class="bg-gray-50 dark:bg-gray-700 px-4 py-2 border-b border-gray-200 dark:border-gray-600">
                                <div class="flex items-center justify-between gap-3">
                                    <h4 class="font-medium text-gray-900 dark:text-white text-sm flex-shrink-0">
                                        "Fail to Pass Tests (" {move || fail_to_pass_tests.get().len().to_string()} ")"
                                    </h4>
                                <input
                                    type="text"
                                    placeholder="Filter tests..."
                                    prop:value=move || fail_to_pass_filter.get()
                                    on:input=move |ev| fail_to_pass_filter.set(event_target_value(&ev))
                                        class="flex-1 min-w-0 px-2 py-1 text-xs border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:focus:ring-blue-400"
                                />
                            </div>
                            </div>
                            <div class="flex-1 overflow-auto">
                                <For
                                    each=move || {
                                        let filter = fail_to_pass_filter.get().to_lowercase();
                                        fail_to_pass_tests.get()
                                            .into_iter()
                                            .enumerate()
                                            .filter(|(_, test)| filter.is_empty() || test.to_lowercase().contains(&filter))
                                            .collect::<Vec<_>>()
                                    }
                                    key=|(i, _)| *i
                                            children=move |(index, test_name)| {
                                                let test_name_for_display = test_name.clone();
                                                let test_name_for_click = test_name.clone();
                                                let is_selected = move || {
                                                    current_selection.get() == "fail_to_pass" &&
                                                    selected_fail_to_pass_index.get() == index
                                                };
                                                view! {
                                                    <div
                                                        id=format!("fail_to_pass-item-{}", index)
                                                        class=move || format!(
                                                            "px-4 py-1 text-sm border-b border-gray-100 dark:border-gray-600 cursor-pointer flex items-center {}",
                                                            if is_selected() {
                                                                "bg-blue-100 dark:bg-blue-900/50 text-blue-900 dark:text-blue-100"
                                                            } else {
                                                                "text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
                                                            }
                                                        )
                                                        on:click=move |_| {
                                                            current_selection.set("fail_to_pass".to_string());
                                                            selected_fail_to_pass_index.set(index);
                                                            search_for_test(test_name_for_click.clone());
                                                        }
                                                    >
                                                        <span class="w-8 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0 font-mono text-xs">
                                                            {index + 1}
                                                        </span>
                                                        <span class="flex-1 truncate">{test_name_for_display}</span>
                                                        <div class="flex items-center gap-1 ml-2 flex-shrink-0">
                                                            // Status icons would go here
                                                        </div>
                                                    </div>
                                                }
                                            }
                                />
                            </div>
                        </div>

                        // Pass to Pass Tests
                        <div class="w-1/2 flex flex-col">
                            <div class="bg-gray-50 dark:bg-gray-700 px-4 py-2 border-b border-gray-200 dark:border-gray-600">
                                <div class="flex items-center justify-between gap-3">
                                    <h4 class="font-medium text-gray-900 dark:text-white text-sm flex-shrink-0">
                                        "Pass to Pass Tests (" {move || pass_to_pass_tests.get().len().to_string()} ")"
                                    </h4>
                                <input
                                    type="text"
                                    placeholder="Filter tests..."
                                    prop:value=move || pass_to_pass_filter.get()
                                    on:input=move |ev| pass_to_pass_filter.set(event_target_value(&ev))
                                        class="flex-1 min-w-0 px-2 py-1 text-xs border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:focus:ring-blue-400"
                                />
                            </div>
                            </div>
                            <div class="flex-1 overflow-auto">
                                <For
                                    each=move || {
                                        let filter = pass_to_pass_filter.get().to_lowercase();
                                        pass_to_pass_tests.get()
                                            .into_iter()
                                            .enumerate()
                                            .filter(|(_, test)| filter.is_empty() || test.to_lowercase().contains(&filter))
                                            .collect::<Vec<_>>()
                                    }
                                    key=|(i, _)| *i
                                            children=move |(index, test_name)| {
                                                let test_name_for_display = test_name.clone();
                                                let test_name_for_click = test_name.clone();
                                                let is_selected = move || {
                                                    current_selection.get() == "pass_to_pass" &&
                                                    selected_pass_to_pass_index.get() == index
                                                };
                                                view! {
                                                    <div
                                                        id=format!("pass_to_pass-item-{}", index)
                                                        class=move || format!(
                                                            "px-4 py-1 text-sm border-b border-gray-100 dark:border-gray-600 cursor-pointer flex items-center {}",
                                                            if is_selected() {
                                                                "bg-green-100 dark:bg-green-900/50 text-green-900 dark:text-green-100"
                                                            } else {
                                                                "text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
                                                            }
                                                        )
                                                        on:click=move |_| {
                                                            current_selection.set("pass_to_pass".to_string());
                                                            selected_pass_to_pass_index.set(index);
                                                            search_for_test(test_name_for_click.clone());
                                                        }
                                                    >
                                                        <span class="w-8 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0 font-mono text-xs">
                                                            {index + 1}
                                                        </span>
                                                        <span class="flex-1 truncate">{test_name_for_display}</span>
                                                        <div class="flex items-center gap-1 ml-2 flex-shrink-0">
                                                            // Status icons would go here
                                                        </div>
                                                    </div>
                                                }
                                            }
                                />
                            </div>
                        </div>
                    </div>
                    </div>

                    // Log Search Results Section (Bottom Half)
                    <div class="h-1/2 flex flex-row">
                        // Base Log Results
                        <div class="w-1/3 border-r border-gray-200 dark:border-gray-700 flex flex-col">
                            <div class="bg-gray-50 dark:bg-gray-700 px-4 py-2 border-b border-gray-200 dark:border-gray-600 flex items-center justify-between">
                                <h4 class="font-medium text-gray-900 dark:text-white text-sm">
                                    "Base Log (" {move || search_results.get().base_results.len().to_string()} " results)"
                                </h4>
                                {move || {
                                    let results = search_results.get();
                                    let indices = search_result_indices.get();
                                    let current_index = indices.get("base").copied().unwrap_or(0);
                                    let total_results = results.base_results.len();
                                    
                                    if total_results > 1 {
                                view! {
                                            <div class="flex items-center gap-1">
                                    <button
                                                    on:click=move |_| navigate_search_result("base", "prev")
                                                    class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                                >
                                                    "←"
                                                </button>
                                                <span class="text-xs text-gray-500">
                                                    {format!("{}/{}", current_index + 1, total_results)}
                                                </span>
                                                <button
                                                    on:click=move |_| navigate_search_result("base", "next")
                                                    class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                                >
                                                    "→"
                                                </button>
                                            </div>
                                        }.into_any()
                                            } else {
                                        view! {}.into_any()
                                    }
                                }}
                            </div>
                            <div class="flex-1 overflow-auto p-4">
                                {move || {
                                    let results = search_results.get();
                                    let indices = search_result_indices.get();
                                    let current_index = indices.get("base").copied().unwrap_or(0);
                                    
                                    if results.base_results.is_empty() {
                                        view! {
                                            <div class="text-gray-500 dark:text-gray-400 text-sm">No matches found</div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="font-mono text-xs">
                                                {move || {
                                                    if let Some(result) = results.base_results.get(current_index) {
                                                        let start_line_number = result.line_number - result.context_before.len();
                                                        let context_before = result.context_before.clone();
                                                        let context_after = result.context_after.clone();
                                                        let line_content = result.line_content.clone();
                                                        let context_before_items = context_before.into_iter().enumerate().collect::<Vec<_>>();
                                                        let context_after_items = context_after.into_iter().enumerate().collect::<Vec<_>>();
                                                        let context_before_len = context_before_items.len();

                                                        view! {
                                                            <>
                                                                // Context before
                                                                <For
                                                                    each=move || context_before_items.clone()
                                                                    key=|(i, _)| *i
                                                                    children=move |(i, line)| {
                                                                        let line_number = start_line_number + i;
                                                                        view! {
                                                                            <div class="flex text-gray-500 dark:text-gray-400">
                                                                                <span class="w-12 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0">
                                                                                    {line_number}
                                                                                </span>
                                                                                <span class="flex-1">{line}</span>
                                                                            </div>
                                                                        }
                                                                    }
                                                                />
                                                                // Highlighted match
                                                                <div class="flex bg-yellow-200 dark:bg-yellow-800 text-gray-900 dark:text-gray-100 font-bold">
                                                                    <span class="w-12 text-right pr-2 text-gray-700 dark:text-gray-300 flex-shrink-0">
                                                                        {start_line_number + context_before_len}
                                                                    </span>
                                                                    <span class="flex-1">{line_content}</span>
                                                                </div>
                                                                // Context after
                                                                <For
                                                                    each=move || context_after_items.clone()
                                                                    key=|(i, _)| *i
                                                                    children=move |(i, line)| {
                                                                        let line_number = start_line_number + context_before_len + 1 + i;
                                                                        view! {
                                                                            <div class="flex text-gray-500 dark:text-gray-400">
                                                                                <span class="w-12 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0">
                                                                                    {line_number}
                                                                                </span>
                                                                                <span class="flex-1">{line}</span>
                                                                            </div>
                                                                        }
                                                                    }
                                                                />
                                                            </>
                                                        }.into_any()
                                                    } else {
                                                        view! {}.into_any()
                                                    }
                                                }}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                        </div>
                        </div>

                        // Before Log Results
                        <div class="w-1/3 border-r border-gray-200 dark:border-gray-700 flex flex-col">
                            <div class="bg-gray-50 dark:bg-gray-700 px-4 py-2 border-b border-gray-200 dark:border-gray-600 flex items-center justify-between">
                                <h4 class="font-medium text-gray-900 dark:text-white text-sm">
                                    "Before Log (" {move || search_results.get().before_results.len().to_string()} " results)"
                                </h4>
                            {move || {
                                let results = search_results.get();
                                    let indices = search_result_indices.get();
                                    let current_index = indices.get("before").copied().unwrap_or(0);
                                    let total_results = results.before_results.len();
                                    
                                    if total_results > 1 {
                                    view! {
                                            <div class="flex items-center gap-1">
                                                <button
                                                    on:click=move |_| navigate_search_result("before", "prev")
                                                    class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                                >
                                                    "←"
                                                </button>
                                                <span class="text-xs text-gray-500">
                                                    {format!("{}/{}", current_index + 1, total_results)}
                                                </span>
                                                <button
                                                    on:click=move |_| navigate_search_result("before", "next")
                                                    class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                                >
                                                    "→"
                                                </button>
                                        </div>
                                    }.into_any()
                                } else {
                                        view! {}.into_any()
                                    }
                                }}
                            </div>
                            <div class="flex-1 overflow-auto p-4">
                            {move || {
                                let results = search_results.get();
                                let indices = search_result_indices.get();
                                let current_index = indices.get("before").copied().unwrap_or(0);
                                
                                if results.before_results.is_empty() {
                                    view! {
                                        <div class="text-gray-500 dark:text-gray-400 text-sm">No matches found</div>
                                    }.into_any()
                                } else {
                                                    view! {
                                        <div class="font-mono text-xs">
                                            {move || {
                                                if let Some(result) = results.before_results.get(current_index) {
                                                        let start_line_number = result.line_number - result.context_before.len();
                                                        let context_before = result.context_before.clone();
                                                        let context_after = result.context_after.clone();
                                                        let line_content = result.line_content.clone();
                                                        let context_before_items = context_before.into_iter().enumerate().collect::<Vec<_>>();
                                                        let context_after_items = context_after.into_iter().enumerate().collect::<Vec<_>>();
                                                        let context_before_len = context_before_items.len();

                                                    view! {
                                                            <>
                                                                // Context before
                                                                <For
                                                                    each=move || context_before_items.clone()
                                                                    key=|(i, _)| *i
                                                                    children=move |(i, line)| {
                                                                        let line_number = start_line_number + i;
                                                                        view! {
                                                                            <div class="flex text-gray-500 dark:text-gray-400">
                                                                                <span class="w-12 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0">
                                                                                    {line_number}
                                                                                </span>
                                                                                <span class="flex-1">{line}</span>
                                                                            </div>
                                                                        }
                                                                    }
                                                                />
                                                                // Highlighted match
                                                                <div class="flex bg-yellow-200 dark:bg-yellow-800 text-gray-900 dark:text-gray-100 font-bold">
                                                                    <span class="w-12 text-right pr-2 text-gray-700 dark:text-gray-300 flex-shrink-0">
                                                                        {start_line_number + context_before_len}
                                                                    </span>
                                                                    <span class="flex-1">{line_content}</span>
                                                                </div>
                                                                // Context after
                                                                <For
                                                                    each=move || context_after_items.clone()
                                                                    key=|(i, _)| *i
                                                                    children=move |(i, line)| {
                                                                        let line_number = start_line_number + context_before_len + 1 + i;
                                                                        view! {
                                                                            <div class="flex text-gray-500 dark:text-gray-400">
                                                                                <span class="w-12 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0">
                                                                                    {line_number}
                                                                                </span>
                                                                                <span class="flex-1">{line}</span>
                                                                            </div>
                                                                        }
                                                                    }
                                                                />
                                                            </>
                                                        }.into_any()
                                                    } else {
                                                        view! {}.into_any()
                                                    }
                                                }}
                                                            </div>
                                        }.into_any()
                                    }
                                }}
                                                        </div>
                        </div>
                        
                        // After Log Results
                        <div class="w-1/3 flex flex-col">
                            <div class="bg-gray-50 dark:bg-gray-700 px-4 py-2 border-b border-gray-200 dark:border-gray-600 flex items-center justify-between">
                                <h4 class="font-medium text-gray-900 dark:text-white text-sm">
                                    "After Log (" {move || search_results.get().after_results.len().to_string()} " results)"
                                </h4>
                                {move || {
                                    let results = search_results.get();
                                    let indices = search_result_indices.get();
                                    let current_index = indices.get("after").copied().unwrap_or(0);
                                    let total_results = results.after_results.len();
                                    
                                    if total_results > 1 {
                                        view! {
                                            <div class="flex items-center gap-1">
                                                <button
                                                    on:click=move |_| navigate_search_result("after", "prev")
                                                    class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                                >
                                                    "←"
                                                </button>
                                                <span class="text-xs text-gray-500">
                                                    {format!("{}/{}", current_index + 1, total_results)}
                                                </span>
                                                <button
                                                    on:click=move |_| navigate_search_result("after", "next")
                                                    class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                                >
                                                    "→"
                                                </button>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }
                                }}
                            </div>
                            <div class="flex-1 overflow-auto p-4">
                                {move || {
                                    let results = search_results.get();
                                    let indices = search_result_indices.get();
                                    let current_index = indices.get("after").copied().unwrap_or(0);
                                    
                                    if results.after_results.is_empty() {
                                        view! {
                                            <div class="text-gray-500 dark:text-gray-400 text-sm">No matches found</div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="font-mono text-xs">
                                                {move || {
                                                    if let Some(result) = results.after_results.get(current_index) {
                                                        let start_line_number = result.line_number - result.context_before.len();
                                                        let context_before = result.context_before.clone();
                                                        let context_after = result.context_after.clone();
                                                        let line_content = result.line_content.clone();
                                                        let context_before_items = context_before.into_iter().enumerate().collect::<Vec<_>>();
                                                        let context_after_items = context_after.into_iter().enumerate().collect::<Vec<_>>();
                                                        let context_before_len = context_before_items.len();

                                                        view! {
                                                            <>
                                                                // Context before
                                                                <For
                                                                    each=move || context_before_items.clone()
                                                                    key=|(i, _)| *i
                                                                    children=move |(i, line)| {
                                                                        let line_number = start_line_number + i;
                                                                        view! {
                                                                            <div class="flex text-gray-500 dark:text-gray-400">
                                                                                <span class="w-12 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0">
                                                                                    {line_number}
                                                                                </span>
                                                                                <span class="flex-1">{line}</span>
                                                        </div>
                                                    }
                                                }
                                            />
                                                                // Highlighted match
                                                                <div class="flex bg-yellow-200 dark:bg-yellow-800 text-gray-900 dark:text-gray-100 font-bold">
                                                                    <span class="w-12 text-right pr-2 text-gray-700 dark:text-gray-300 flex-shrink-0">
                                                                        {start_line_number + context_before_len}
                                                                    </span>
                                                                    <span class="flex-1">{line_content}</span>
                                        </div>
                                                                // Context after
                                                                <For
                                                                    each=move || context_after_items.clone()
                                                                    key=|(i, _)| *i
                                                                    children=move |(i, line)| {
                                                                        let line_number = start_line_number + context_before_len + 1 + i;
                                                                        view! {
                                                                            <div class="flex text-gray-500 dark:text-gray-400">
                                                                                <span class="w-12 text-right pr-2 text-gray-400 dark:text-gray-500 flex-shrink-0">
                                                                                    {line_number}
                                                                                </span>
                                                                                <span class="flex-1">{line}</span>
                                                                            </div>
                                                                        }
                                                                    }
                                                                />
                                                            </>
                                    }.into_any()
                                                    } else {
                                                        view! {}.into_any()
                                                    }
                                                }}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
            </div>
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