use leptos::prelude::*;
use std::collections::HashMap;
use super::types::LogSearchResults;

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
) -> impl IntoView {
    view! {
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
    }
}
