use leptos::prelude::*;
use std::collections::HashMap;
use super::types::LogSearchResults;
use super::search_results::navigate_search_result;

#[component]
pub fn LogColumn(
    log_key: &'static str,
    title: &'static str,
    search_results: RwSignal<LogSearchResults>,
    search_result_indices: RwSignal<HashMap<String, usize>>,
    container_class: &'static str,
) -> impl IntoView {
    view! {
        <div class=container_class>
            <div class="bg-gray-50 dark:bg-gray-700 px-4 py-2 border-b border-gray-200 dark:border-gray-600 flex items-center justify-between">
                <h4 class="font-medium text-gray-900 dark:text-white text-sm">
                    {title} " (" {move || {
                        let results = search_results.get();
                        match log_key {
                            "base" => results.base_results.len().to_string(),
                            "before" => results.before_results.len().to_string(),
                            "after" => results.after_results.len().to_string(),
                            _ => "0".to_string(),
                        }
                    }} " results)"
                </h4>
                {move || {
                    let results = search_results.get();
                    let indices = search_result_indices.get();
                    let current_index = indices.get(log_key).copied().unwrap_or(0);
                    let total_results = match log_key {
                        "base" => results.base_results.len(),
                        "before" => results.before_results.len(),
                        "after" => results.after_results.len(),
                        _ => 0,
                    };
                    
                    if total_results > 1 {
                        view! {
                            <div class="flex items-center gap-1">
                                <button
                                    on:click=move |_| navigate_search_result(log_key, "prev", search_results, search_result_indices)
                                    class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                >
                                    "←"
                                </button>
                                <span class="text-xs text-gray-500">
                                    {format!("{}/{}", current_index + 1, total_results)}
                                </span>
                                <button
                                    on:click=move |_| navigate_search_result(log_key, "next", search_results, search_result_indices)
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
                    let current_index = indices.get(log_key).copied().unwrap_or(0);

                    let items = match log_key {
                        "base" => results.base_results.clone(),
                        "before" => results.before_results.clone(),
                        "after" => results.after_results.clone(),
                        _ => Vec::new(),
                    };

                    if items.is_empty() {
                        return view! { <div class="text-gray-500 dark:text-gray-400 text-sm">No matches found</div> }.into_any();
                    }

                    if let Some(result) = items.get(current_index) {
                        let start_line_number = result.line_number - result.context_before.len();
                        let context_before_items = result.context_before.clone().into_iter().enumerate().collect::<Vec<_>>();
                        let context_after_items = result.context_after.clone().into_iter().enumerate().collect::<Vec<_>>();
                        let context_before_len = context_before_items.len();
                        let line_content = result.line_content.clone();

                        view! {
                            <div class="font-mono text-xs">
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
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
pub fn LogSearchResults(
    search_results: RwSignal<LogSearchResults>,
    search_result_indices: RwSignal<HashMap<String, usize>>,
) -> impl IntoView {
    view! {
        <div class="h-1/2 flex flex-row">
            <LogColumn
                log_key="base"
                title="Base Log"
                search_results=search_results
                search_result_indices=search_result_indices
                container_class="w-1/3 border-r border-gray-200 dark:border-gray-700 flex flex-col"
            />
            <LogColumn
                log_key="before"
                title="Before Log"
                search_results=search_results
                search_result_indices=search_result_indices
                container_class="w-1/3 border-r border-gray-200 dark:border-gray-700 flex flex-col"
            />
            <LogColumn
                log_key="after"
                title="After Log"
                search_results=search_results
                search_result_indices=search_result_indices
                container_class="w-1/3 flex flex-col"
            />
        </div>
    }.into_any()
}
