use leptos::prelude::*;
use std::collections::HashMap;
use super::types::{LogSearchResults, LogAnalysisResult, TestStatus};
use crate::components::language_selector::ProgrammingLanguage;

#[component]
pub fn TestChecker(
    fail_to_pass_tests: RwSignal<Vec<String>>,
    pass_to_pass_tests: RwSignal<Vec<String>>,
    current_selection: RwSignal<String>,
    selected_fail_to_pass_index: RwSignal<usize>,
    selected_pass_to_pass_index: RwSignal<usize>,
    fail_to_pass_filter: RwSignal<String>,
    pass_to_pass_filter: RwSignal<String>,
    search_for_test: impl Fn(String) + Send + Sync + 'static + Copy,
    _search_results: RwSignal<LogSearchResults>,
    _search_result_indices: RwSignal<HashMap<String, usize>>,
    selected_language: RwSignal<ProgrammingLanguage>,
    log_analysis_result: RwSignal<Option<LogAnalysisResult>>,
    _log_analysis_loading: RwSignal<bool>,
) -> impl IntoView {
    // Helper function to get test status from analysis result for a specific stage
    let get_test_status_for_stage = move |test_name: &str, test_type: &str, stage: &str| -> Option<TestStatus> {
        if let Some(analysis) = log_analysis_result.get() {
            let stage_test_name = format!("{}_{}", test_name, stage);
            analysis.test_statuses.iter()
                .find(|status| status.test_name == stage_test_name && status.r#type == test_type)
                .cloned()
        } else {
            None
        }
    };

    // Helper function to get all stage statuses for a test
    let get_all_stage_statuses = move |test_name: &str, test_type: &str| -> HashMap<String, Option<TestStatus>> {
        let mut statuses = HashMap::new();
        let stages = ["base", "before", "after", "agent", "report"];
        
        for stage in &stages {
            statuses.insert(stage.to_string(), get_test_status_for_stage(test_name, test_type, stage));
        }
        
        statuses
    };

    // Helper function to check if test has rule violations
    let has_rule_violations = move |test_name: &str, test_type: &str| -> bool {
        if let Some(analysis) = log_analysis_result.get() {
            let rule_checks = &analysis.rule_violations;
            
            // Check if test appears in any rule violation examples
            let mut has_violation = false;
            
            // C1: P2P tests that are failed in base
            if test_type == "pass_to_pass" && rule_checks.c1_failed_in_base_present_in_p2p.has_problem {
                has_violation = rule_checks.c1_failed_in_base_present_in_p2p.examples.iter()
                    .any(|example| example == test_name);
            }
            
            // C2: Any test that failed in after
            if rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.has_problem {
                has_violation = rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.examples.iter()
                    .any(|example| example == test_name);
            }
            
            // C3: F2P tests that are successful in before
            if test_type == "fail_to_pass" && rule_checks.c3_f2p_success_in_before.has_problem {
                has_violation = rule_checks.c3_f2p_success_in_before.examples.iter()
                    .any(|example| example == test_name);
            }
            
            // C4: P2P tests missing in base and not passing in before
            if test_type == "pass_to_pass" && rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.has_problem {
                has_violation = rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.examples.iter()
                    .any(|example| example.contains(test_name));
            }
            
            // C6: Test marked as failed in report but passing in agent
            if rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.has_problem {
                has_violation = rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.examples.iter()
                    .any(|example| example.contains(test_name));
            }
            
            // C7: F2P tests in golden source diff
            if test_type == "fail_to_pass" && rule_checks.c7_f2p_tests_in_golden_source_diff.has_problem {
                has_violation = rule_checks.c7_f2p_tests_in_golden_source_diff.examples.iter()
                    .any(|example| example.contains(test_name));
            }
            
            has_violation
        } else {
            false
        }
    };

    // Helper function to render status icon
    let render_status_icon = move |status: &str| {
        match status {
            "passed" => view! {
                <div class="w-4 h-4 flex items-center justify-center bg-green-100 dark:bg-green-900/50 rounded-full">
                    <img 
                        src="https://img.icons8.com/?id=11695&format=png&size=16" 
                        alt="Passed" 
                        class="w-3 h-3"
                    />
                </div>
            },
            "failed" => view! {
                <div class="w-4 h-4 flex items-center justify-center bg-red-100 dark:bg-red-900/50 rounded-full">
                    <img 
                        src="https://img.icons8.com/?id=3062&format=png&size=16" 
                        alt="Failed" 
                        class="w-3 h-3"
                    />
                </div>
            },
            "missing" => view! {
                <div class="w-4 h-4 flex items-center justify-center bg-yellow-100 dark:bg-yellow-900/50 rounded-full">
                    <img 
                        src="https://img.icons8.com/?id=6645&format=png&size=16" 
                        alt="Ignored" 
                        class="w-3 h-3"
                    />
                </div>
            },
            _ => view! {
                <div class=""><div class=""></div></div>
            }
        }
    };

    // Helper function to render status row for both Rust and non-Rust languages
    let render_status_row = move |test_name: String, test_type: &str| {
        if selected_language.get() == ProgrammingLanguage::Rust {
            let stage_statuses = get_all_stage_statuses(&test_name, test_type);
            let has_violations = has_rule_violations(&test_name, test_type);
            
            // Get status for each stage
            let base_status = stage_statuses.get("base")
                .and_then(|s| s.as_ref())
                .map(|s| s.status.as_str())
                .unwrap_or("missing");
            
            let before_status = stage_statuses.get("before")
                .and_then(|s| s.as_ref())
                .map(|s| s.status.as_str())
                .unwrap_or("missing");
            
            let after_status = stage_statuses.get("after")
                .and_then(|s| s.as_ref())
                .map(|s| s.status.as_str())
                .unwrap_or("missing");
            
            
            view! {
                <div class="flex items-center gap-1" title="Base | Before | After">
                    {render_status_icon(base_status)}
                    {render_status_icon(before_status)}
                    {render_status_icon(after_status)}
                </div>
            }
        } else {
            view! { 
                <div class="flex items-center gap-1" title="Base | Before | After">
                    {render_status_icon("missing")}
                    {render_status_icon("missing")}
                    {render_status_icon("missing")}
                </div>
            }
        }
    };
    view! {
        <div class="h-full flex">
            // Fail to Pass Tests
            <div class="w-1/2 border-r border-gray-200 dark:border-gray-700 flex flex-col h-full">
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
                <div class="flex-1 overflow-auto min-h-0">
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
                                        {move || render_status_row(test_name.clone(), "fail_to_pass")}
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>

            // Pass to Pass Tests
            <div class="w-1/2 flex flex-col h-full">
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
                <div class="flex-1 overflow-auto min-h-0">
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
                                        {move || render_status_row(test_name.clone(), "pass_to_pass")}
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
            </div>
    }
}
