use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

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
            <div class="bg-white shadow-sm border-b border-gray-200">
                <div class="w-full mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="flex justify-between items-center h-16">
                        <div class="flex items-center">
                            <h1 class="text-xl font-semibold text-gray-900">
                                "SWE Reviewer"
                            </h1>
                        </div>
                        <Show when=move || current_deliverable.get().is_some() fallback=|| view!{ <div></div> }>
                        <span class="text-xl font-black text-gray-700">
                            {move || current_deliverable.get().map_or(String::new(), |d| format!("[{}]", d.instance_id.clone())) }
                        </span>
                        </Show>
                        <Show when=move || current_deliverable.get().is_some() fallback=|| view!{ <div></div> }>
                            <div class="flex items-center space-x-2">
                                <a 
                                    href=move || current_deliverable.get().map_or(String::new(), |d| d.deliverable_link.clone())
                                    target="_blank"
                                    class="text-sm text-blue-600 hover:text-blue-800 underline"
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
                                                    format!("https://github.com/swe-bench/SWE-bench/issues/{}", id)
                                                } else { String::new() }
                                            } else { String::new() }
                                        }
                                        target="_blank"
                                        class="text-sm text-blue-600 hover:text-blue-800 underline"
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
                                                    format!("https://github.com/swe-bench/SWE-bench/pull/{}", id)
                                                } else { String::new() }
                                            } else { String::new() }
                                        }
                                        target="_blank"
                                        class="text-sm text-blue-600 hover:text-blue-800 underline"
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
                                        class="text-sm text-blue-600 hover:text-blue-800 underline"
                                    >
                                        "SWE URL"
                                    </a>
                                </Show>
                            </div>
                        </Show>
                    </div>
                </div>
            </div>

            // Main content
            <div class="w-full bg-white dark:bg-gray-800" style="height: calc(100vh - 65px);">
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=move || DeliverableCheckerPage(DeliverableCheckerPageProps { current_deliverable: current_deliverable.clone() }) />
                </Routes>
            </div>
        </div>
    }
}
