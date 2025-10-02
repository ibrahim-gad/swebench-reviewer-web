use leptos::prelude::*;
use std::collections::HashMap;
use super::types::LogSearchResults;
use super::search_results::navigate_search_result;

#[component]
pub fn LogSearchResults(
    search_results: RwSignal<LogSearchResults>,
    search_result_indices: RwSignal<HashMap<String, usize>>,
) -> impl IntoView {
    view! {
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
                                        on:click=move |_| navigate_search_result("base", "prev", search_results, search_result_indices)
                                        class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                    >
                                        "←"
                                    </button>
                                    <span class="text-xs text-gray-500">
                                        {format!("{}/{}", current_index + 1, total_results)}
                                    </span>
                                    <button
                                        on:click=move |_| navigate_search_result("base", "next", search_results, search_result_indices)
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
                                        on:click=move |_| navigate_search_result("before", "prev", search_results, search_result_indices)
                                        class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                    >
                                        "←"
                                    </button>
                                    <span class="text-xs text-gray-500">
                                        {format!("{}/{}", current_index + 1, total_results)}
                                    </span>
                                    <button
                                        on:click=move |_| navigate_search_result("before", "next", search_results, search_result_indices)
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
                                        on:click=move |_| navigate_search_result("after", "prev", search_results, search_result_indices)
                                        class="px-1 py-0 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                    >
                                        "←"
                                    </button>
                                    <span class="text-xs text-gray-500">
                                        {format!("{}/{}", current_index + 1, total_results)}
                                    </span>
                                    <button
                                        on:click=move |_| navigate_search_result("after", "next", search_results, search_result_indices)
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
    }
}
