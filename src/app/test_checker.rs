use leptos::prelude::*;
use std::collections::HashMap;
use super::types::{LogSearchResults, LogAnalysisResult};

#[derive(Clone, Debug, PartialEq)]
pub struct RuleViolationInfo {
    pub rule_name: String,
    pub description: String,
    pub examples: Vec<String>,
}

impl RuleViolationInfo {
    pub fn new(rule_name: &str, description: &str, examples: &[String]) -> Self {
        Self {
            rule_name: rule_name.to_string(),
            description: description.to_string(),
            examples: examples.to_vec(),
        }
    }
}

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
    log_analysis_result: RwSignal<Option<LogAnalysisResult>>,
    _log_analysis_loading: RwSignal<bool>,
) -> impl IntoView {
    if let Some(analysis) = log_analysis_result.get() {
        let total = analysis.test_statuses.f2p.len() + analysis.test_statuses.p2p.len();
        leptos::logging::log!("Analysis has {} test statuses", total);
        leptos::logging::log!("Rule violations in analysis:");
        
        if analysis.rule_violations.c1_failed_in_base_present_in_p2p.has_problem {
            leptos::logging::log!("C1 violations: {:?}", analysis.rule_violations.c1_failed_in_base_present_in_p2p.examples);
        }
        if analysis.rule_violations.c2_failed_in_after_present_in_f2p_or_p2p.has_problem {
            leptos::logging::log!("C2 violations: {:?}", analysis.rule_violations.c2_failed_in_after_present_in_f2p_or_p2p.examples);
        }
        if analysis.rule_violations.c3_f2p_success_in_before.has_problem {
            leptos::logging::log!("C3 violations: {:?}", analysis.rule_violations.c3_f2p_success_in_before.examples);
        }
        if analysis.rule_violations.c6_test_marked_failed_in_report_but_passing_in_agent.has_problem {
            leptos::logging::log!("C6 violations: {:?}", analysis.rule_violations.c6_test_marked_failed_in_report_but_passing_in_agent.examples);
        }
    }
    
    // Log test lists
    leptos::logging::log!("F2P tests count: {}", fail_to_pass_tests.get().len());
    leptos::logging::log!("P2P tests count: {}", pass_to_pass_tests.get().len());

    // Helper to fetch a stage value from grouped statuses
    let get_grouped_stage = move |test_name: &str, test_type: &str, stage: &str, analysis: &Option<LogAnalysisResult>| -> String {
        if let Some(analysis) = analysis {
            let opt = if test_type == "fail_to_pass" {
                analysis.test_statuses.f2p.get(test_name)
            } else {
                analysis.test_statuses.p2p.get(test_name)
            };
            if let Some(summary) = opt {
                match stage {
                    "base" => summary.base.clone(),
                    "before" => summary.before.clone(),
                    "after" => summary.after.clone(),
                    "agent" => summary.agent.clone(),
                    "report" => summary.report.clone(),
                    _ => "missing".to_string(),
                }
            } else {
                "missing".to_string()
            }
        } else {
            "missing".to_string()
        }
    };

    let get_violated_rules = move |test_name: &str, test_type: &str, analysis: &Option<LogAnalysisResult>| -> Vec<RuleViolationInfo> {
        
        if let Some(analysis) = analysis {
            let rule_checks = &analysis.rule_violations;
            let mut violated_rules = Vec::new();
            
            // C1: P2P tests that are failed in base
            if test_type == "pass_to_pass" && rule_checks.c1_failed_in_base_present_in_p2p.has_problem {
                let matches = rule_checks.c1_failed_in_base_present_in_p2p.examples.iter().any(|example| {
                    let match_result = example == test_name;
                    match_result
                });
                if matches {
                    violated_rules.push(RuleViolationInfo::new(
                        "c1_failed_in_base_present_in_p2p",
                        "Pass-to-pass tests that failed in base but are present in P2P",
                        &rule_checks.c1_failed_in_base_present_in_p2p.examples,
                    ));
                }
            }
            
            // C2: Tests that failed in after but present in F2P or P2P
            if rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.has_problem {
                let matches = rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.examples.iter().any(|example| {
                    let match_result = example == test_name;
                    match_result
                });
                if matches {
                    violated_rules.push(RuleViolationInfo::new(
                        "c2_failed_in_after_present_in_f2p_or_p2p",
                        "Tests that failed in after but are present in F2P or P2P",
                        &rule_checks.c2_failed_in_after_present_in_f2p_or_p2p.examples,
                    ));
                }
            }
            
            // C3: F2P tests that are successful in before
            if test_type == "fail_to_pass" && rule_checks.c3_f2p_success_in_before.has_problem {
                let matches = rule_checks.c3_f2p_success_in_before.examples.iter().any(|example| {
                    let match_result = example == test_name;
                    match_result
                });
                if matches {
                    violated_rules.push(RuleViolationInfo::new(
                        "c3_f2p_success_in_before",
                        "Fail-to-pass tests that succeeded in before",
                        &rule_checks.c3_f2p_success_in_before.examples,
                    ));
                }
            }
            
            // C4: P2P tests missing in base and not passing in before
            if test_type == "pass_to_pass" && rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.has_problem {
                if rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.examples.iter()
                    .any(|example| example.contains(test_name)) {
                    violated_rules.push(RuleViolationInfo::new(
                        "c4_p2p_missing_in_base_and_not_passing_in_before",
                        "Pass-to-pass tests missing in base and not passing in before",
                        &rule_checks.c4_p2p_missing_in_base_and_not_passing_in_before.examples,
                    ));
                }
            }
            
            // C6: Test marked as failed in report but passing in agent
            if rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.has_problem {
                let matches = rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.examples.iter().any(|example| {
                    // For C6, the example contains extra text, so we need to check if test_name is contained
                    let clean_example = example.split(" (").next().unwrap_or(example);
                    let match_result = clean_example == test_name;
                    match_result
                });
                if matches {
                    violated_rules.push(RuleViolationInfo::new(
                        "c6_test_marked_failed_in_report_but_passing_in_agent",
                        "Tests marked as failed in report but passing in agent log",
                        &rule_checks.c6_test_marked_failed_in_report_but_passing_in_agent.examples,
                    ));
                }
            }
            
            // C7: F2P tests in golden source diff
            if test_type == "fail_to_pass" && rule_checks.c7_f2p_tests_in_golden_source_diff.has_problem {
                let matches = rule_checks.c7_f2p_tests_in_golden_source_diff.examples.iter()
                    .any(|example| {
                        // C7 examples have format: "test_name (found as 'function_name' in file but not in test diffs)"
                        // Extract the test name before the first " (" to get exact match
                        if let Some(test_part) = example.split(" (").next() {
                            test_part == test_name
                        } else {
                            example == test_name
                        }
                    });
                if matches {
                    violated_rules.push(RuleViolationInfo::new(
                        "c7_f2p_tests_in_golden_source_diff",
                        "Fail-to-pass tests present in golden source diff",
                        &rule_checks.c7_f2p_tests_in_golden_source_diff.examples,
                    ));
                }
            }
            
            violated_rules
        } else {
            Vec::new()
        }
    };

    // Memoized status computation for all fail_to_pass tests - updated to include violated rules
    let fail_to_pass_statuses = Memo::new({
        let get_violated_rules = get_violated_rules.clone();
        let fail_to_pass_tests = fail_to_pass_tests.clone();
        move |_| {
            let analysis = log_analysis_result.get();
            let mut statuses = HashMap::new();
            for test_name in &fail_to_pass_tests.get() {
                let violated_rules = get_violated_rules(test_name, "fail_to_pass", &analysis);
                
                let base_status = get_grouped_stage(test_name, "fail_to_pass", "base", &analysis);
                let before_status = get_grouped_stage(test_name, "fail_to_pass", "before", &analysis);
                let after_status = get_grouped_stage(test_name, "fail_to_pass", "after", &analysis);
                
                statuses.insert(
                    test_name.clone(),
                    (base_status, before_status, after_status, violated_rules)
                );
            }
            statuses
        }
    });

    // Memoized status computation for all pass_to_pass tests - updated to include violated rules
    let pass_to_pass_statuses = Memo::new({
        let get_violated_rules = get_violated_rules.clone();
        let pass_to_pass_tests = pass_to_pass_tests.clone();
        move |_| {
            let analysis = log_analysis_result.get();
            let mut statuses = HashMap::new();
            for test_name in &pass_to_pass_tests.get() {
                let violated_rules = get_violated_rules(test_name, "pass_to_pass", &analysis);
                
                let base_status = get_grouped_stage(test_name, "pass_to_pass", "base", &analysis);
                let before_status = get_grouped_stage(test_name, "pass_to_pass", "before", &analysis);
                let after_status = get_grouped_stage(test_name, "pass_to_pass", "after", &analysis);
                
                statuses.insert(
                    test_name.clone(),
                    (base_status, before_status, after_status, violated_rules)
                );
            }
            statuses
        }
    });

    // Helper function to render status icon with type erasure to reduce monomorphization depth
    let render_status_icon = move |status: &str| {
        match status {
            "passed" => view! {
                <div class="w-4 h-4 flex items-center justify-center bg-green-100 dark:bg-green-300 rounded-full">
                    <img 
                        src="https://img.icons8.com/?id=11695&format=png&size=16" 
                        alt="Passed" 
                        class="w-3 h-3"
                    />
                </div>
            }.into_any(),
            "failed" => view! {
                <div class="w-4 h-4 flex items-center justify-center bg-red-100 dark:bg-red-300 rounded-full">
                    <img 
                        src="https://img.icons8.com/?id=3062&format=png&size=16" 
                        alt="Failed" 
                        class="w-3 h-3"
                    />
                </div>
            }.into_any(),
            "missing" => view! {
                <div class="w-4 h-4 flex items-center justify-center bg-yellow-100 dark:bg-yellow-300 rounded-full">
                    <img 
                        src="https://img.icons8.com/?id=Kc1iMzD0T01B&format=png&size=16" 
                        alt="Ignored" 
                        class="w-3 h-3"
                    />
                </div>
            }.into_any(),
            _ => view! {
                <div class=""><div class=""></div></div>
            }.into_any(),
        }
    };

    // Refactored helper function to render status row using precomputed statuses - with type erasure
    let render_status_row = move |test_name: String, test_type: &str| {
        if true {
            let statuses_map = if test_type == "fail_to_pass" {
                &fail_to_pass_statuses.get()
            } else {
                &pass_to_pass_statuses.get()
            };
            
            if let Some((base_status, before_status, after_status, _violated_rules)) = statuses_map.get(&test_name) {
                view! {
                    <div class="flex items-center gap-1" title="Base | Before | After">
                        {render_status_icon(base_status)}
                        {render_status_icon(before_status)}
                        {render_status_icon(after_status)}
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="flex items-center gap-1" title="Base | Before | After">
                        {render_status_icon("missing")}
                        {render_status_icon("missing")}
                        {render_status_icon("missing")}
                    </div>
                }.into_any()
            }
        } else {
            view! { 
                <div class="flex items-center gap-1" title="Base | Before | After">
                    {render_status_icon("ignored")}
                    {render_status_icon("ignored")}
                    {render_status_icon("ignored")}
                </div>
            }.into_any()
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
                            let mut tests = fail_to_pass_tests.get()
                                .into_iter()
                                .enumerate()
                                .filter(|(_, test)| filter.is_empty() || test.to_lowercase().contains(&filter))
                                .collect::<Vec<_>>();
                            
                            
                            // Sort tests with violations to the top - use current analysis state
                            let analysis = log_analysis_result.get();
                            if let Some(analysis) = &analysis {
                                tests.sort_by(|(i1, t1), (i2, t2)| {
                                    // Compute violations reactively for sorting
                                    let violations1 = get_violated_rules(t1, "fail_to_pass", &Some(analysis.clone()));
                                    let violations2 = get_violated_rules(t2, "fail_to_pass", &Some(analysis.clone()));
                                    let has_violation1 = !violations1.is_empty();
                                    let has_violation2 = !violations2.is_empty();
                                    
                                    has_violation2.cmp(&has_violation1).then(i1.cmp(i2))
                                });
                            }
                            
                            tests
                        }
                        key=|(i, _)| *i
                        children=move |(index, test_name)| {
                            let test_name_for_display = test_name.clone();
                            let test_name_for_click = test_name.clone();
                            let test_name_for_status = test_name.clone();
                            let is_selected = move || {
                                current_selection.get() == "fail_to_pass" &&
                                selected_fail_to_pass_index.get() == index
                            };
                            
                            // Make violation computation reactive to analysis changes
                            let test_name_for_violations = test_name.clone();
                            let test_name_for_violations_memo = test_name_for_violations.clone(); // Clone for memo
                            
                            let violated_rules_signal = Memo::new(move |_| {
                                let analysis = log_analysis_result.get();
                                get_violated_rules(&test_name_for_violations_memo, "fail_to_pass", &analysis)
                            });
                            
                            let has_violations = move || {
                                let rules = violated_rules_signal.get();
                                let has = !rules.is_empty();
                                has
                            };
                            
                            
                            view! {
                                <div
                                    id=format!("fail_to_pass-item-{}", index)
                                    class=move || {
                                        let base_class = if is_selected() {
                                            if current_selection.get() == "fail_to_pass" {
                                                "bg-blue-100 dark:bg-blue-900/50 text-blue-900 dark:text-blue-100"
                                            } else {
                                                "bg-green-100 dark:bg-green-900/50 text-green-900 dark:text-green-100"
                                            }
                                        } else {
                                            "text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
                                        };
                                        
                                        // Show red border for ALL tests with violations; apply red background only when not selected
                                        let violation_class = if has_violations() {
                                            if is_selected() {
                                                "border-l-4 border-red-500"
                                            } else {
                                                "border-l-4 border-red-500 bg-red-50/50 dark:bg-red-900/20"
                                            }
                                        } else {
                                            ""
                                        };
                                        
                                        format!("px-4 py-1 text-sm border-b border-gray-100 dark:border-gray-600 cursor-pointer flex items-center {} {}", base_class, violation_class)
                                    }
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
                                        {move || render_status_row(test_name_for_status.clone(), "fail_to_pass")}
                                    </div>
                                </div>
                            }.into_any()
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
                            let mut tests = pass_to_pass_tests.get()
                                .into_iter()
                                .enumerate()
                                .filter(|(_, test)| filter.is_empty() || test.to_lowercase().contains(&filter))
                                .collect::<Vec<_>>();
                            
                            
                            // Sort tests with violations to the top - use current analysis state
                            let analysis = log_analysis_result.get();
                            if let Some(analysis) = &analysis {
                                tests.sort_by(|(i1, t1), (i2, t2)| {
                                    // Compute violations reactively for sorting
                                    let violations1 = get_violated_rules(t1, "pass_to_pass", &Some(analysis.clone()));
                                    let violations2 = get_violated_rules(t2, "pass_to_pass", &Some(analysis.clone()));
                                    let has_violation1 = !violations1.is_empty();
                                    let has_violation2 = !violations2.is_empty();
                                    
                                    has_violation2.cmp(&has_violation1).then(i1.cmp(i2))
                                });
                            }
                            
                            tests
                        }
                        key=|(i, _)| *i
                        children=move |(index, test_name)| {
                            let test_name_for_display = test_name.clone();
                            let test_name_for_click = test_name.clone();
                            let test_name_for_status = test_name.clone();
                            let is_selected = move || {
                                current_selection.get() == "pass_to_pass" &&
                                selected_pass_to_pass_index.get() == index
                            };
                            
                            // Make violation computation reactive to analysis changes
                            let test_name_for_violations = test_name.clone();
                            let test_name_for_violations_memo = test_name_for_violations.clone(); // Clone for memo
                            
                            let violated_rules_signal = Memo::new(move |_| {
                                let analysis = log_analysis_result.get();
                                get_violated_rules(&test_name_for_violations_memo, "pass_to_pass", &analysis)
                            });
                            
                            let has_violations = move || {
                                let rules = violated_rules_signal.get();
                                let has = !rules.is_empty();
                                has
                            };
                            
                            view! {
                                <div
                                    id=format!("pass_to_pass-item-{}", index)
                                    class=move || {
                                        let base_class = if is_selected() {
                                            "bg-green-100 dark:bg-green-900/50 text-green-900 dark:text-green-100"
                                        } else {
                                            "text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
                                        };
                                        
                                        // Show red border for ALL tests with violations; apply red background only when not selected
                                        let violation_class = if has_violations() {
                                            if is_selected() {
                                                "border-l-4 border-red-500"
                                            } else {
                                                "border-l-4 border-red-500 bg-red-50/50 dark:bg-red-900/20"
                                            }
                                        } else {
                                            ""
                                        };
                                        
                                        format!("px-4 py-1 text-sm border-b border-gray-100 dark:border-gray-600 cursor-pointer flex items-center {} {}", base_class, violation_class)
                                    }
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
                                        {move || render_status_row(test_name_for_status.clone(), "pass_to_pass")}
                                    </div>
                                </div>
                            }.into_any()
                        }
                    />
                </div>
            </div>
            </div>
    }
}
