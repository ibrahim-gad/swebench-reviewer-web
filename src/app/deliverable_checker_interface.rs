use leptos::prelude::*;
use std::collections::HashMap;
use super::types::{LogSearchResults, FileContents, LogAnalysisResult};
use super::test_checker::TestChecker;
use super::log_search_results::LogSearchResults as LogSearchResultsComponent;
use super::file_viewer::FileViewer;
use crate::components::language_selector::ProgrammingLanguage;

#[component]
pub fn ReportCheckerInterface(
    fail_to_pass_tests: RwSignal<Vec<String>>,
    pass_to_pass_tests: RwSignal<Vec<String>>,
    current_selection: RwSignal<String>,
    selected_fail_to_pass_index: RwSignal<usize>,
    selected_pass_to_pass_index: RwSignal<usize>,
    fail_to_pass_filter: RwSignal<String>,
    pass_to_pass_filter: RwSignal<String>,
    search_for_test: impl Fn(String) + Send + Sync + 'static + Copy,
    active_tab: RwSignal<String>,
    active_main_tab: RwSignal<String>,
    search_results: RwSignal<LogSearchResults>,
    search_result_indices: RwSignal<HashMap<String, usize>>,
    file_contents: RwSignal<FileContents>,
    loading_files: RwSignal<bool>,
    reset_state: impl Fn() + Send + Sync + 'static + Copy,
    selected_language: RwSignal<ProgrammingLanguage>,
    log_analysis_result: RwSignal<Option<LogAnalysisResult>>,
    log_analysis_loading: RwSignal<bool>,
) -> impl IntoView {
    let manual_tab_active = move || active_main_tab.get() == "manual_checker";
    let input_tab_active = move || active_main_tab.get() == "input";

    view! {
        <div class="flex flex-col h-full overflow-hidden">
            <div class="flex-none bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 px-4 py-1 shadow-sm mb-1">
                // Single line with back button, centered title, and copy functionality
                <div class="flex items-center justify-between gap-4 relative">
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
                        <div class="flex space-x-1 bg-gray-100 dark:bg-gray-700 p-1 rounded">
                            <button
                                on:click=move |_| {
                                    active_main_tab.set("manual_checker".to_string());
                                }
                                class=move || {
                                    if manual_tab_active() {
                                        "px-5 py-2 rounded font-medium text-sm transition-all duration-200 bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 shadow-sm"
                                            .to_string()
                                    } else {
                                        "px-5 py-2 rounded font-medium text-sm transition-all duration-200 text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-gray-600"
                                            .to_string()
                                    }
                                }
                            >
                                <div class="flex items-center gap-2">
                                    <span>"Tests Checker"</span>
                                    <Show
                                        when=move || selected_language.get() == ProgrammingLanguage::Rust && log_analysis_loading.get()
                                        fallback=|| view! { <div></div> }
                                    >
                                        <div class="w-4 h-4">
                                            <svg class="animate-spin text-blue-500" fill="none" viewBox="0 0 24 24">
                                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                            </svg>
                                        </div>
                                    </Show>
                                </div>
                            </button>
                            <button
                                on:click=move |_| {
                                    active_main_tab.set("input".to_string());
                                    active_tab.set("base".to_string());
                                }
                                class=move || {
                                    if input_tab_active() {
                                        "px-5 py-2 rounded font-medium text-sm transition-all duration-200 bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 shadow-sm"
                                            .to_string()
                                    } else {
                                        "px-5 py-2 rounded font-medium text-sm transition-all duration-200 text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-gray-600"
                                            .to_string()
                                    }
                                }
                            >
                                Input
                            </button>
                        </div>
                    </div>

                    // Copy Selected Test Name
                    <Show
                        when=manual_tab_active
                        fallback=|| view! { <div></div> }
                    >
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
                    </Show>
                </div>
            </div>

            // Main Content
            <div class="flex-1 overflow-hidden bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm">
                <Show
                    when=input_tab_active
                    fallback=move || view! {
                        // Test Lists Section (Top Half)
                        <div class="h-1/2 border-b border-gray-200 dark:border-gray-700">
                            <TestChecker
                                fail_to_pass_tests=fail_to_pass_tests
                                pass_to_pass_tests=pass_to_pass_tests
                                current_selection=current_selection
                                selected_fail_to_pass_index=selected_fail_to_pass_index
                                selected_pass_to_pass_index=selected_pass_to_pass_index
                                fail_to_pass_filter=fail_to_pass_filter
                                pass_to_pass_filter=pass_to_pass_filter
                                search_for_test=search_for_test
                                _search_results=search_results
                                _search_result_indices=search_result_indices
                                selected_language=selected_language
                                log_analysis_result=log_analysis_result
                                _log_analysis_loading=log_analysis_loading
                            />
                        </div>

                        // Log Search Results Section (Bottom Half)
                        <LogSearchResultsComponent
                            search_results=search_results
                            search_result_indices=search_result_indices
                        />
                    }
                >
                    <FileViewer
                        active_tab=active_tab
                        file_contents=file_contents
                        loading_files=loading_files
                    />
                </Show>
            </div>
        </div>
    }
}
