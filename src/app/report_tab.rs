use leptos::prelude::*;
use leptos::task::spawn_local;

use super::types::{ProcessingResult, FileContents, LoadedFileTypes, LogAnalysisResult, SearchResult};
use super::file_operations::load_file_contents;
use super::search_results::handle_search_agent_logs;

fn render_status_icon(status: &str) -> AnyView {
    match status {
        "passed" => view! {
            <div class="w-4 h-4 flex items-center justify-center bg-green-100 dark:bg-green-300 rounded-full">
                <img src="https://img.icons8.com/?id=11695&format=png&size=16" alt="Passed" class="w-3 h-3" />
            </div>
        }.into_any(),
        "failed" => view! {
            <div class="w-4 h-4 flex items-center justify-center bg-red-100 dark:bg-red-300 rounded-full">
                <img src="https://img.icons8.com/?id=3062&format=png&size=16" alt="Failed" class="w-3 h-3" />
            </div>
        }.into_any(),
        "missing" => view! {
            <div class="w-4 h-4 flex items-center justify-center bg-yellow-100 dark:bg-yellow-300 rounded-full">
                <img src="https://img.icons8.com/?id=Kc1iMzD0T01B&format=png&size=16" alt="Ignored" class="w-3 h-3" />
            </div>
        }.into_any(),
        _ => view! { <div class=""></div> }.into_any(),
    }
}

fn parse_report_lists(content: &str) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut f2p_success: Vec<String> = Vec::new();
    let mut f2p_failure: Vec<String> = Vec::new();
    let mut p2p_success: Vec<String> = Vec::new();
    let mut p2p_failure: Vec<String> = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        // Try to find tests_status at root or one level nested
        let mut tests_status: Option<serde_json::Value> = None;
        if let Some(ts) = json.get("tests_status").cloned() { tests_status = Some(ts); }
        if tests_status.is_none() {
            if let Some(obj) = json.as_object() {
                for (_k, v) in obj {
                    if let Some(ts) = v.get("tests_status") { tests_status = Some(ts.clone()); break; }
                }
            }
        }

        if let Some(ts) = tests_status {
            let empty: Vec<serde_json::Value> = vec![];
            // FAIL_TO_PASS
            if let Some(f2p) = ts.get("FAIL_TO_PASS") {
                f2p_success = f2p.get("success").and_then(|a| a.as_array()).unwrap_or(&empty)
                    .iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();
                f2p_failure = f2p.get("failure").and_then(|a| a.as_array()).unwrap_or(&empty)
                    .iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();
            }
            // PASS_TO_PASS
            if let Some(p2p) = ts.get("PASS_TO_PASS") {
                p2p_success = p2p.get("success").and_then(|a| a.as_array()).unwrap_or(&empty)
                    .iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();
                p2p_failure = p2p.get("failure").and_then(|a| a.as_array()).unwrap_or(&empty)
                    .iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();
            }
        }
    }

    (f2p_success, f2p_failure, p2p_success, p2p_failure)
}

fn get_stage_status(
    test_name: &str,
    stage: &str,
    analysis: &Option<LogAnalysisResult>,
    test_type: &str,
) -> String {
    if let Some(analysis) = analysis {
        let opt = if test_type == "fail_to_pass" {
            analysis.test_statuses.f2p.get(test_name)
        } else {
            analysis.test_statuses.p2p.get(test_name)
        };
        if let Some(summary) = opt {
            match stage {
                "agent" => summary.agent.clone(),
                "report" => summary.report.clone(),
                _ => "missing".to_string(),
            }
        } else { "not_supported".to_string() }
    } else { "not_supported".to_string() }
}

#[component]
pub fn ReportTab(
    result: RwSignal<Option<ProcessingResult>>,
    file_contents: RwSignal<FileContents>,
    loading_files: RwSignal<bool>,
    loaded_file_types: RwSignal<LoadedFileTypes>,
    log_analysis_result: RwSignal<Option<LogAnalysisResult>>,
    selected_test_name: RwSignal<String>,
) -> impl IntoView {
    let selected_test_type = RwSignal::new(String::from("fail_to_pass"));

    let agent_results = RwSignal::new(Vec::<SearchResult>::new());
    let agent_index = RwSignal::new(0usize);

    // Ensure report and agent are loaded when this tab is visible
    Effect::new({
        let result = result.clone();
        let file_contents = file_contents.clone();
        let loading_files = loading_files.clone();
        let loaded_file_types = loaded_file_types.clone();
        move |_| {
            if let Some(_) = result.get() {
                let loaded = loaded_file_types.get();
                let contents = file_contents.get();
                let need_report = contents.report.is_none() && !loaded.is_loaded("report");
                let need_agent = contents.agent.is_none() && !loaded.is_loaded("agent");
                if need_report || need_agent {
                    load_file_contents(result.clone(), file_contents.clone(), loading_files.clone(), loaded_file_types.clone(), Some(vec!["report".to_string(), "agent".to_string()]));
                }
            }
        }
    });

    // Parse lists from report json when it is available
    let f2p_success = RwSignal::new(Vec::<String>::new());
    let f2p_failure = RwSignal::new(Vec::<String>::new());
    let p2p_success = RwSignal::new(Vec::<String>::new());
    let p2p_failure = RwSignal::new(Vec::<String>::new());

    // Search agent log when selected test changes (declare before using below)
    let trigger_agent_search = move |test_name: String| {
        if result.get().is_none() || test_name.is_empty() { return; }
        let res = result.get().unwrap();
        spawn_local(async move {
            if let Ok(items) = handle_search_agent_logs(res.file_paths, test_name).await {
                agent_results.set(items);
                agent_index.set(0);
            }
        });
    };

    Effect::new({
        let file_contents = file_contents.clone();
        let selected_test_name = selected_test_name.clone();
        let selected_test_type = selected_test_type.clone();
        move |_| {
            if let Some(report) = &file_contents.get().report {
                let (a,b,c,d) = parse_report_lists(&report.content);
                f2p_success.set(a.clone());
                f2p_failure.set(b.clone());
                p2p_success.set(c.clone());
                p2p_failure.set(d.clone());

                // Auto-select first available test in priority order if nothing selected yet
                if selected_test_name.get().is_empty() {
                    let pick = if let Some(first) = a.first() { Some((first.clone(), "fail_to_pass")) }
                        else if let Some(first) = b.first() { Some((first.clone(), "fail_to_pass")) }
                        else if let Some(first) = c.first() { Some((first.clone(), "pass_to_pass")) }
                        else if let Some(first) = d.first() { Some((first.clone(), "pass_to_pass")) }
                        else { None };
                    if let Some((name, ty)) = pick {
                        selected_test_name.set(name.clone());
                        selected_test_type.set(ty.to_string());
                        trigger_agent_search(name);
                    }
                }
            }
        }
    });

    // Filters
    let f2p_success_filter = RwSignal::new(String::new());
    let p2p_success_filter = RwSignal::new(String::new());
    let f2p_failure_filter = RwSignal::new(String::new());
    let p2p_failure_filter = RwSignal::new(String::new());

    // Helper: does C6 violation apply to a test name?
    let is_c6_violation = move |test_name: &str| -> bool {
        let analysis_opt = log_analysis_result.get();
        if let Some(analysis) = analysis_opt {
            let c6 = &analysis.rule_violations.c6_test_marked_failed_in_report_but_passing_in_agent;
            if !c6.has_problem { return false; }
            for example in &c6.examples {
                let clean_example = example.split(" (").next().unwrap_or(example);
                if clean_example == test_name { return true; }
            }
        }
        false
    };

    let render_list = move |tests_signal: RwSignal<Vec<String>>, test_type: &'static str, title: &'static str, filter_signal: RwSignal<String>| -> AnyView {
        view! {
            <div class="h-full overflow-hidden bg-white dark:bg-gray-800 flex flex-col">
                <div class="bg-gray-50 dark:bg-gray-700 px-4 py-2 border-b border-gray-200 dark:border-gray-600">
                    <div class="flex items-center justify-between gap-3">
                        <h4 class="font-medium text-gray-900 dark:text-white text-sm flex-shrink-0">
                            {title} " (" {move || tests_signal.get().len().to_string()} ")"
                        </h4>
                        <input
                            type="text"
                            placeholder="Filter tests..."
                            prop:value=move || filter_signal.get()
                            on:input=move |ev| filter_signal.set(event_target_value(&ev))
                            class="flex-1 min-w-0 px-2 py-1 text-xs border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:focus:ring-blue-400"
                        />
                    </div>
                </div>
                <div class="flex-1 overflow-auto">
                    <ul class="divide-y divide-gray-200 dark:divide-gray-600">
                        <For
                            each=move || {
                                let filter = filter_signal.get().to_lowercase();
                                let mut items: Vec<String> = tests_signal
                                    .get()
                                    .into_iter()
                                    .filter(|name| filter.is_empty() || name.to_lowercase().contains(&filter))
                                    .collect::<Vec<_>>();
                                // Sort C6 violations to the top
                                items.sort_by(|a, b| {
                                    let va = is_c6_violation(a);
                                    let vb = is_c6_violation(b);
                                    vb.cmp(&va)
                                });
                                items
                            }
                            key=|name| name.clone()
                            children=move |name: String| {
                                let name_for_click = name.clone();
                                let name_for_is_selected = name.clone();
                                let t_name_for_status = name.clone();
                                // Pre-clone for separate move closures to avoid moving the same value twice
                                let t_name_for_status_for_class = t_name_for_status.clone();
                                let t_name_for_status_for_report = t_name_for_status.clone();
                                let t_name_for_status_for_agent = t_name_for_status.clone();
                                let analysis = log_analysis_result.clone();
                                let is_selected = move || selected_test_name.get() == name_for_is_selected;
                                view! {
                                    <li
                                        class=move || {
                                            let base_class = if is_selected() {
                                                "px-4 py-1 text-sm bg-blue-100 dark:bg-blue-900/30 text-blue-900 dark:text-blue-100 flex items-center justify-between cursor-pointer"
                                            } else {
                                                "px-4 py-1 text-sm text-gray-800 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 flex items-center justify-between cursor-pointer"
                                            };
                                            let violation_class = if is_c6_violation(&t_name_for_status_for_class) {
                                                if is_selected() { "border-l-4 border-red-500" } else { "border-l-4 border-red-500 bg-red-50/50 dark:bg-red-900/20" }
                                            } else { "" };
                                            format!("{} {}", base_class, violation_class)
                                        }
                                        on:click=move |_| {
                                            let click_name = name_for_click.clone();
                                            selected_test_name.set(click_name.clone());
                                            selected_test_type.set(test_type.to_string());
                                            trigger_agent_search(click_name);
                                        }
                                    >
                                        <div class="truncate pr-2">{name}</div>
                                        <div class="flex items-center gap-1 flex-shrink-0" title="Report | Agent">
                                            {move || {
                                                let status_report = get_stage_status(&t_name_for_status_for_report, "report", &analysis.get(), test_type);
                                                let status_agent = get_stage_status(&t_name_for_status_for_agent, "agent", &analysis.get(), test_type);
                                                view! {
                                                    <div class="flex items-center gap-1">
                                                        {render_status_icon(&status_report)}
                                                        {render_status_icon(&status_agent)}
                                                    </div>
                                                }.into_any()
                                            }}
                                        </div>
                                    </li>
                                }.into_any()
                            }
                        />
                    </ul>
                </div>
            </div>
        }.into_any()
    };

    let render_agent_highlight = move || -> AnyView {
        let items = agent_results.get();
        let idx = agent_index.get();
        let content: AnyView = if items.is_empty() {
            view! { <div class="text-gray-500 dark:text-gray-400 text-sm">No matches found</div> }.into_any()
        } else {
            let result = items[idx].clone();
            let context_before_items = result.context_before.clone();
            let context_after_items = result.context_after.clone();
            view! {
                <div>
                    {context_before_items.iter().map(|l| view! { <div class="text-gray-500 dark:text-gray-400">{l.clone()}</div> }).collect_view()}
                    <div class="bg-yellow-200 dark:bg-yellow-800 text-gray-900 dark:text-gray-100 font-bold px-1">{result.line_content.clone()}</div>
                    {context_after_items.iter().map(|l| view! { <div class="text-gray-500 dark:text-gray-400">{l.clone()}</div> }).collect_view()}
                </div>
            }.into_any()
        };
        view! {
            <div class="flex flex-col h-full bg-white dark:bg-gray-800">
                <div class="p-3 font-mono text-xs text-gray-800 dark:text-gray-200">
                    {content}
                </div>
            </div>
        }.into_any()
    };

    let show_empty_message = move || {
        // Consider empty when report lists all empty AND agent content missing or empty fallback
        let lists_empty = f2p_success.get().is_empty() && f2p_failure.get().is_empty() && p2p_success.get().is_empty() && p2p_failure.get().is_empty();
        let agent_empty = match &file_contents.get().agent {
            Some(content) => content.content.trim().is_empty() || content.content.contains("No agent") || content.content.contains("No post_agent_patch"),
            None => true,
        };
        lists_empty && agent_empty
    };

    view! {
        <div class="w-full h-full">
            <Show
                when=move || !show_empty_message()
                fallback=move || view! {
                    <div class="w-full h-full flex items-center justify-center">
                        <div class="max-w-xl text-center text-gray-700 dark:text-gray-300">
                            "Looks like Report.JSON and post_agent_log are empty, this is completly fine, you should proceed with the task normally"
                        </div>
                    </div>
                }
            >
                <div class="w-full h-full grid grid-cols-3 grid-rows-2 gap-0 divide-x divide-y divide-gray-200 dark:divide-gray-700">
                    <div class="p-0 overflow-hidden">
                        {render_list(f2p_success, "fail_to_pass", "F2P Success", f2p_success_filter)}
                    </div>
                    <div class="p-0 overflow-hidden">
                        <div class="bg-gray-50 dark:bg-gray-700 px-3 py-1 border-b border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-900 dark:text-white">Status</div>
                        <div class="p-3 text-sm text-gray-700 dark:text-gray-200">
                            {move || {
                                let name = selected_test_name.get();
                                if !name.is_empty() && is_c6_violation(&name) {
                                    return view! {
                                        <div class="p-2 bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-800 rounded text-xs text-red-800 dark:text-red-200">
                                            <div class="font-medium">{"C6: Test marked failed in report but passing in agent"}</div>
                                            <div class="opacity-80">{name.clone()}</div>
                                        </div>
                                    }.into_any();
                                }

                                // No selection yet — show a green success hint
                                if name.is_empty() {
                                    return view! {
                                        <div class="p-2 bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-800 rounded text-xs text-green-800 dark:text-green-200">
                                            <span>"Looks Good."</span>
                                        </div>
                                    }.into_any();
                                }

                                let analysis = log_analysis_result.get();
                                let test_type = selected_test_type.get();
                                let report_status = get_stage_status(&name, "report", &analysis, &test_type);
                                let agent_status = get_stage_status(&name, "agent", &analysis, &test_type);

                                if report_status == "not_supported" || agent_status == "not_supported" {
                                    view! {
                                        <div class="p-2 bg-yellow-50 dark:bg-yellow-900/30 border border-yellow-200 dark:border-yellow-800 rounded text-xs text-yellow-800 dark:text-yellow-200">
                                            <span>"Looks like we don't support this log format yet, but you can go through them manually."</span>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="p-2 bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-800 rounded text-xs text-green-800 dark:text-green-200">
                                            <span>{format!(
                                                "Test {} is {} in report.json and {} in agent log, so this looks good",
                                                name, report_status, agent_status
                                            )}</span>
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                    <div class="p-0 overflow-hidden">
                        {render_list(p2p_success, "pass_to_pass", "P2P Success", p2p_success_filter)}
                    </div>

                    <div class="p-0 overflow-hidden">
                        {render_list(f2p_failure, "fail_to_pass", "F2P Failures", f2p_failure_filter)}
                    </div>
                    <div class="p-0 overflow-hidden">
                        <div class="bg-gray-50 dark:bg-gray-700 px-3 py-1 border-b border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-900 dark:text-white flex items-center justify-between">
                            <h4 class="font-medium text-gray-900 dark:text-white text-sm">
                                {move || {
                                    let count = agent_results.get().len();
                                    format!("Agent Log ({} results)", count)
                                }}
                            </h4>
                            {move || {
                                let total = agent_results.get().len();
                                let current = agent_index.get();
                                if total > 1 {
                                    view! {
                                        <div class="flex items-center gap-1">
                                            <button
                                                on:click=move |_| {
                                                    let len = agent_results.get().len();
                                                    if len > 0 { agent_index.set((agent_index.get() + len - 1) % len); }
                                                }
                                                class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                            >
                                                "←"
                                            </button>
                                            <span class="text-xs text-gray-500">{format!("{}/{}", current + 1, total)}</span>
                                            <button
                                                on:click=move |_| {
                                                    let len = agent_results.get().len();
                                                    if len > 0 { agent_index.set((agent_index.get() + 1) % len); }
                                                }
                                                class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                            >
                                                "→"
                                            </button>
                                        </div>
                                    }.into_any()
                                } else { view! { <div></div> }.into_any() }
                            }}
                        </div>
                        <div class="h-full">{render_agent_highlight()}</div>
                    </div>
                    <div class="p-0 overflow-hidden">
                        {render_list(p2p_failure, "pass_to_pass", "P2P Failures", p2p_failure_filter)}
                    </div>
                </div>
            </Show>
        </div>
    }.into_any()
}
