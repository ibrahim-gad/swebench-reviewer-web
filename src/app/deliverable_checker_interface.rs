use leptos::prelude::*;
use std::collections::HashMap;
use leptos_router::hooks::use_navigate;
use super::types::{LogSearchResults, FileContents, LogAnalysisResult};
use super::test_checker::TestChecker;
use super::log_search_results::LogSearchResults as LogSearchResultsComponent;
use super::file_viewer::FileViewer;
use super::types::LoadedFileTypes;
use super::test_checker::RuleViolationInfo;

#[component]
pub fn DeliverableCheckerInterface(
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
    log_analysis_result: RwSignal<Option<LogAnalysisResult>>,
    log_analysis_loading: RwSignal<bool>,
    loaded_file_types: RwSignal<LoadedFileTypes>,
    result: RwSignal<Option<super::types::ProcessingResult>>,
) -> impl IntoView {
    let navigate_fn = use_navigate();
    let manual_tab_active = move || active_main_tab.get() == "manual_checker";
    let playground_tab_active = move || active_main_tab.get() == "playground";
    let input_tab_active = move || active_main_tab.get() == "input";
    let get_selected_test_violations = move || -> Vec<RuleViolationInfo> {
        let analysis = log_analysis_result.get();
        if let Some(analysis) = analysis {
            let selected_test_name = if current_selection.get() == "fail_to_pass" {
                let f2p_tests = fail_to_pass_tests.get();
                let index = selected_fail_to_pass_index.get();
                if index < f2p_tests.len() {
                    Some(f2p_tests[index].clone())
                } else {
                    None
                }
            } else {
                let p2p_tests = pass_to_pass_tests.get();
                let index = selected_pass_to_pass_index.get();
                if index < p2p_tests.len() {
                    Some(p2p_tests[index].clone())
                } else {
                    None
                }
            };
            
            if let Some(test_name) = selected_test_name {
                let test_type = if current_selection.get() == "fail_to_pass" { "fail_to_pass" } else { "pass_to_pass" };
                
                let mut violated_rules = Vec::new();
                let rule_checks = &analysis.rule_violations;
                
                if test_type == "pass_to_pass" && rule_checks.c1_failed_in_base_present_in_p2p.has_problem {
                    if rule_checks.c1_failed_in_base_present_in_p2p.examples.iter().any(|example| *example == test_name) {
                        violated_rules.push(RuleViolationInfo {
                            rule_name: "c1_failed_in_base_present_in_p2p".to_string(),
                            description: "Pass-to-pass tests that failed in base but are present in P2P".to_string(),
                            examples: rule_checks.c1_failed_in_base_present_in_p2p.examples.clone(),
                        });
                    }
                }
                
                if rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.has_problem {
                    if rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.examples.iter().any(|example| *example == test_name) {
                        violated_rules.push(RuleViolationInfo {
                            rule_name: "c2_failed_in_after_present_in_f2p_or_p2p".to_string(),
                            description: "Tests that failed in after but are present in F2P or P2P".to_string(),
                            examples: rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.examples.clone(),
                        });
                    }
                }
                
                if test_type == "fail_to_pass" && rule_checks.c3_f2p_success_in_before.has_problem {
                    if rule_checks.c3_f2p_success_in_before.examples.iter().any(|example| *example == test_name) {
                        violated_rules.push(RuleViolationInfo {
                            rule_name: "c3_f2p_success_in_before".to_string(),
                            description: "Fail-to-pass tests that succeeded in before".to_string(),
                            examples: rule_checks.c3_f2p_success_in_before.examples.clone(),
                        });
                    }
                }
                
                if test_type == "pass_to_pass" && rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.has_problem {
                    if rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.examples.iter().any(|example| example.contains(&test_name)) {
                        violated_rules.push(RuleViolationInfo {
                            rule_name: "c4_p2p_missing_in_base_and_not_passing_in_before".to_string(),
                            description: "Pass-to-pass tests missing in base and not passing in before".to_string(),
                            examples: rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.examples.clone(),
                        });
                    }
                }
                
                if rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.has_problem {
                    if rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.examples.iter().any(|example| {
                        let clean_example = example.split(" (").next().unwrap_or(example);
                        *clean_example == test_name
                    }) {
                        violated_rules.push(RuleViolationInfo {
                            rule_name: "c6_test_marked_failed_in_report_but_passing_in_agent".to_string(),
                            description: "Tests marked as failed in report but passing in agent log".to_string(),
                            examples: rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.examples.clone(),
                        });
                    }
                }
                
                if test_type == "fail_to_pass" && rule_checks.c7_f2p_tests_in_golden_source_diff.has_problem {
                    let matches = rule_checks.c7_f2p_tests_in_golden_source_diff.examples.iter()
                        .any(|example| {
                            // C7 examples have format: "test_name (found as 'function_name' in file but not in test diffs)"
                            // Extract the test name before the first " (" to get exact match
                            if let Some(test_part) = example.split(" (").next() {
                                test_part == test_name
                            } else {
                                example == &test_name
                            }
                        });
                    if matches {
                        violated_rules.push(RuleViolationInfo {
                            rule_name: "c7_f2p_tests_in_golden_source_diff".to_string(),
                            description: "Fail-to-pass tests present in golden source diff".to_string(),
                            examples: rule_checks.c7_f2p_tests_in_golden_source_diff.examples.clone(),
                        });
                    }
                }
                
                violated_rules
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };
    
    // Flatten nested Show blocks in main content to reduce monomorphization depth
    let main_section = {
        let input_tab_active = input_tab_active.clone();
        let playground_tab_active = playground_tab_active.clone();
        move || {
            if input_tab_active() {
                view! {
                    <FileViewer
                        active_tab=active_tab
                        file_contents=file_contents
                        loading_files=loading_files
                        loaded_file_types=loaded_file_types
                        result=result
                    />
                }.into_any()
            } else if playground_tab_active() {
                use super::playground::Playground;
                view! {
                    <Playground
                        result=result
                        fail_to_pass_tests=fail_to_pass_tests
                        pass_to_pass_tests=pass_to_pass_tests
                    />
                }.into_any()
            } else {
                view! {
                    <>
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
                                log_analysis_result=log_analysis_result
                                _log_analysis_loading=log_analysis_loading
                            />
                        </div>
                        <LogSearchResultsComponent
                            search_results=search_results
                            search_result_indices=search_result_indices
                        />
                    </>
                }.into_any()
            }
        }
    };

    view! {
        <div class="flex flex-col h-full overflow-hidden">
            <div class="flex-row flex justify-between bg-white dark:bg-gray-800 h-12 rounded-lg border border-gray-200 dark:border-gray-700 px-4 py-1 shadow-sm mb-1">
                // Single line with back button, centered title, and copy functionality
                <div class="flex flex-row items-center justify-between gap-4 w-full relative">
                    // Back button - now navigates to root
                    <button
                        on:click=move |_| {
                            reset_state();
                            navigate_fn("/", Default::default());
                        }
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
                                        "px-5 py-1 rounded font-medium text-sm transition-all duration-200 bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 shadow-sm"
                                            .to_string()
                                    } else {
                                        "px-5 py-1 rounded font-medium text-sm transition-all duration-200 text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-gray-600"
                                            .to_string()
                                    }
                                }
                            >
                                <div class="flex items-center gap-2">
                                    <span>"Tests"</span>
                                    <Show
                                        when=move || log_analysis_loading.get()
                                        fallback=|| view! { <div></div> }.into_any()
                                    >
                                        {view! {
                                            <div class="w-4 h-4">
                                                <svg class="animate-spin text-blue-500" fill="none" viewBox="0 0 24 24">
                                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                </svg>
                                            </div>
                                        }.into_any()}
                                    </Show>
                                </div>
                            </button>
                                <button
                                    on:click=move |_| {
                                        active_main_tab.set("playground".to_string());
                                    }
                                    class=move || {
                                        if playground_tab_active() {
                                            "px-5 py-1 rounded font-medium text-sm transition-all duration-200 bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 shadow-sm"
                                                .to_string()
                                        } else {
                                            "px-5 py-1 rounded font-medium text-sm transition-all duration-200 text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-gray-600"
                                                .to_string()
                                        }
                                    }
                                >
                                    Playground
                                </button>
                            <button
                                on:click=move |_| {
                                    active_main_tab.set("input".to_string());
                                    active_tab.set("base".to_string());
                                }
                                class=move || {
                                    if input_tab_active() {
                                        "px-5 py-1 rounded font-medium text-sm transition-all duration-200 bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 shadow-sm"
                                            .to_string()
                                    } else {
                                        "px-5 py-1 rounded font-medium text-sm transition-all duration-200 text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-gray-600"
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
                        fallback=|| view! { <div></div> }.into_any()
                    >
                        {view! {
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
                                        on:click=move |_| {
                                            let test_name = if current_selection.get() == "fail_to_pass" {
                                                let f2p_tests = fail_to_pass_tests.get();
                                                let index = selected_fail_to_pass_index.get();
                                                if index < f2p_tests.len() {
                                                    Some(f2p_tests[index].clone())
                                                } else {
                                                    None
                                                }
                                            } else {
                                                let p2p_tests = pass_to_pass_tests.get();
                                                let index = selected_pass_to_pass_index.get();
                                                if index < p2p_tests.len() {
                                                    Some(p2p_tests[index].clone())
                                                } else {
                                                    None
                                                }
                                            };
                                            
                                            if let Some(name) = test_name {
                                                leptos::logging::log!("Copying test name: {}", name);
                                            }
                                        }
                                    >
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                        </svg>
                                    </button>
                                </div>
                                <div class="ml-2 space-y-0 max-h-24 overflow-y-hidden">
                                    {move || {
                                        let violations = get_selected_test_violations();
                                        violations.into_iter().map(|rule| view! {
                                            <div class="p-0 bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-800 rounded text-xs text-red-800 dark:text-red-200">
                                                <div class="text-red-700 dark:text-red-300">{rule.description}</div>
                                            </div>
                                        }).collect_view()
                                    }}
                                </div>
                            </div>
                        }.into_any()}
                    </Show>
                </div>
            </div>

            // Main Content
            <div class="flex-1 overflow-hidden bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm">
                {main_section}
            </div>
        </div>
    }.into_any()
}
