use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    ParamSegment, StaticSegment,
};
use crate::components::ThemeToggle;

use crate::app::types::ProcessingResult;

pub mod types;
pub mod processing;
pub mod file_operations;
pub mod test_lists;
pub mod search_results;
pub mod file_viewer;
pub mod test_checker;
pub mod log_search_results;
pub mod deliverable_checker_interface;
pub mod deliverable_checker;
pub mod playground;
use deliverable_checker::{DeliverableCheckerPage, DeliverableCheckerPageProps};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/swe-reviewer-web.css"/>

        // sets the document title
        <Title text="SWE Reviewer"/>

        // content for this welcome page
        <Router>
            <main>
                <MainApp/>
            </main>
        </Router>
    }
}

#[component]
pub fn MainApp() -> impl IntoView {
    let current_deliverable = RwSignal::new(None::<ProcessingResult>);

    view! {
        <div class="min-h-screen bg-gray-50 h-screen">
            // Header
            <div class="bg-white dark:bg-gray-900 shadow-sm border-b border-gray-200 dark:border-gray-700">
                <div class="w-full mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="flex justify-between items-center h-14">
                        <div class="flex items-center">
                            <h1 class="text-xl font-semibold text-gray-900 dark:text-white">
                                "SWE Reviewer"
                            </h1>
                        </div>
                        <Show when=move || current_deliverable.get().is_some() fallback=|| view!{ <div></div> }>
                            <span class="text-xl font-black text-gray-700 dark:text-white">
                                <Show when=move || {
                                    if let Some(d) = current_deliverable.get() {
                                        !d.instance_id.is_empty()
                                    } else { false }
                                }>
                                    <span class="inline-flex items-center space-x-1">
                                        <span>"["</span>
                                        <img
                                            class="inline-block w-6 h-6 align-text-bottom"
                                            src=move || {
                                                current_deliverable.get().map_or(String::from("/icons/empty.png"), |d| {
                                                    match d.language.to_lowercase().as_str() {
                                                        "rust" => "/icons/rust.png".to_string(),
                                                        "javascript" | "typescript" => "/icons/javascript.png".to_string(),
                                                        "python" => "/icons/python.png".to_string(),
                                                        "go" => "/icons/go.png".to_string(),
                                                        "java" => "/icons/java.png".to_string(),
                                                        "ruby" => "/icons/ruby.png".to_string(),
                                                        "c++" => "/icons/cpp.png".to_string(),
                                                        "c#" => "/icons/csharp.png".to_string(),
                                                        _ => "/icons/empty.png".to_string(),
                                                    }
                                                })
                                            }
                                            alt=move || current_deliverable.get().map_or(String::new(), |d| d.language.clone())
                                            title=move || current_deliverable.get().map_or(String::new(), |d| d.language.clone())
                                        />
                                        <span>{move || current_deliverable.get().map_or(String::new(), |d| d.instance_id.clone())}</span>
                                        <span>"]"</span>
                                    </span>
                                </Show>
                            </span>
                        </Show>
                        
                        <div class="flex items-center">
                        <Show when=move || current_deliverable.get().is_some()>
                            <div class="flex items-center space-x-2">
                                <a 
                                    href=move || current_deliverable.get().map_or(String::new(), |d| d.deliverable_link.clone())
                                    target="_blank"
                                    class="text-sm text-blue-600 hover:text-blue-800 underline dark:text-blue-400 dark:hover:text-blue-300"
                                >
                                    "Deliverable"
                                </a>
                                <Show when=move || {
                                    if let Some(d) = current_deliverable.get() {
                                        !d.task_id.is_empty()
                                    } else { false }
                                }>
                                    <a 
                                        href=move || {
                                            if let Some(d) = current_deliverable.get() {
                                                if let Some(id) = d.task_id.split('#').last() {
                                                    let repo = if !d.repo.is_empty() { d.repo.clone() } else { "swe-bench/SWE-bench".to_string() };
                                                    format!("https://github.com/{}/issues/{}", repo, id)
                                                } else { String::new() }
                                            } else { String::new() }
                                        }
                                        target="_blank"
                                        class="text-sm text-blue-600 hover:text-blue-800 underline dark:text-blue-400 dark:hover:text-blue-300"
                                    >
                                        {move || {
                                            if let Some(d) = current_deliverable.get() {
                                                if let Some(id) = d.task_id.split('#').last() {
                                                    format!("Issue #{}", id)
                                                } else { String::new() }
                                            } else { String::new() }
                                        }}
                                    </a>
                                </Show>
                                <Show when=move || {
                                    if let Some(d) = current_deliverable.get() {
                                        !d.instance_id.is_empty()
                                    } else { false }
                                }>
                                    <a 
                                        href=move || {
                                            if let Some(d) = current_deliverable.get() {
                                                if let Some(id) = d.instance_id.split('-').last() {
                                                    let repo = if !d.repo.is_empty() { d.repo.clone() } else { "swe-bench/SWE-bench".to_string() };
                                                    format!("https://github.com/{}/pull/{}", repo, id)
                                                } else { String::new() }
                                            } else { String::new() }
                                        }
                                        target="_blank"
                                        class="text-sm text-blue-600 hover:text-blue-800 underline dark:text-blue-400 dark:hover:text-blue-300"
                                    >
                                        {move || {
                                            if let Some(d) = current_deliverable.get() {
                                                if let Some(id) = d.instance_id.split('-').last() {
                                                    format!("PR #{}", id)
                                                } else { String::new() }
                                            } else { String::new() }
                                        }}
                                    </a>
                                    <a 
                                        href=move || current_deliverable.get().map_or(String::new(), |d| format!("https://swe-bench-plus.turing.com/instances/{}", d.instance_id))
                                        target="_blank"
                                        class="text-sm text-blue-600 hover:text-blue-800 underline dark:text-blue-400 dark:hover:text-blue-300"
                                    >
                                        "SWE URL"
                                    </a>
                                </Show>
                            </div>
                        </Show>
                        <div class="ml-2">
                                    <ThemeToggle/>
                                    </div>
                                </div>
                    </div>
                </div>
            </div>

            // Main content
            <div class="w-full bg-white dark:bg-gray-800" style="height: calc(100vh - 57px);">
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=move || DeliverableCheckerPage(DeliverableCheckerPageProps { current_deliverable: current_deliverable.clone() }) />
                    <Route path=ParamSegment("deliverable_id") view=move || DeliverableCheckerPage(DeliverableCheckerPageProps { current_deliverable: current_deliverable.clone() }) />
                </Routes>
            </div>
        </div>
    }
}
