use leptos::prelude::*;
use leptos::prelude::Effect;
use super::types::{FileContents, LoadedFileTypes};
use super::file_operations::load_file_contents;

#[component]
pub fn FileViewer(
    active_tab: RwSignal<String>,
    file_contents: RwSignal<FileContents>,
    loading_files: RwSignal<bool>,
    loaded_file_types: RwSignal<LoadedFileTypes>,
    result: RwSignal<Option<super::types::ProcessingResult>>,
) -> impl IntoView {
    let input_tabs = vec![
        ("base", "Base"),
        ("before", "Before"),
        ("after", "After"),
        ("agent", "Agent"),
        ("main_json", "Main JSON"),
        ("report", "Report JSON"),
    ];

    // Effect to trigger loading when tab changes to an unloaded one
    Effect::new(move |_| {
        let current_tab = active_tab.get();
        let contents = file_contents.get();
        let loaded = loaded_file_types.get();
        
        if contents.get(&current_tab).is_none() && !loaded.is_loaded(&current_tab) {
            if let Some(_) = result.get() {
                load_file_contents(result.clone(), file_contents.clone(), loading_files.clone(), loaded_file_types.clone(), None);
            }
        }
    });

    view! {
        <div class="flex h-full">
            <div class="w-48 bg-gray-100 dark:bg-gray-700 border-r border-gray-200 dark:border-gray-600 flex flex-col">
                <For
                    each=move || input_tabs.clone()
                    key=|(key, _)| *key
                    children=move |(key, label)| {
                        let key_clone = key.to_string();
                        view! {
                            <button
                                class=move || {
                                    if active_tab.get() == key {
                                        "px-4 py-3 text-left text-sm font-medium transition-all duration-200 bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 border-r-2 border-blue-500"
                                            .to_string()
                                    } else {
                                        "px-4 py-3 text-left text-sm font-medium transition-all duration-200 text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-gray-600"
                                            .to_string()
                                    }
                                }
                                on:click=move |_| {
                                    active_tab.set(key_clone.clone());
                                }
                            >
                                {label}
                            </button>
                        }
                    }
                />
            </div>
            <div class="flex-1 flex flex-col p-4 overflow-hidden">
                <Show
                    when=move || loading_files.get()
                    fallback=move || {
                        let active_tab_value = active_tab.get();
                        let contents = file_contents.get();
                        match contents.get(&active_tab_value) {
                            Some(file_content) => {
                                let text = file_content.content.clone();
                                let file_type = file_content.file_type.clone();
                                view! {
                                    <>
                                        <div class="flex-1 min-h-0 overflow-auto rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-900 text-gray-100">
                                            <pre class=move || {
                                                if file_type == "json" {
                                                    "p-4 text-sm font-mono whitespace-pre-wrap text-green-300"
                                                        .to_string()
                                                } else {
                                                    "p-4 text-sm font-mono whitespace-pre-wrap"
                                                        .to_string()
                                                }
                                            }>
                                                {text}
                                            </pre>
                                        </div>
                                    </>
                                }.into_any()
                            }
                            None => {
                                view! {
                                    <div class="flex items-center justify-center h-full">
                                        <div class="text-center text-gray-500 dark:text-gray-400">
                                            No content available for {active_tab_value.replace('_', " ")}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }
                    }
                >
                    <div class="flex items-center justify-center h-full">
                        <div class="flex items-center gap-3 text-gray-600 dark:text-gray-300">
                            <svg class="animate-spin w-6 h-6 text-blue-500" fill="none" viewBox="0 0 24 24">
                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                            </svg>
                            <span>Loading file contents...</span>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    }
}
