use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

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
use deliverable_checker::DeliverableCheckerPage;

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
    view! {
        <div class="min-h-screen bg-gray-50 h-screen">
            // Header
            <div class="bg-white shadow-sm border-b border-gray-200">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="flex justify-between items-center h-16">
                        <div class="flex items-center">
                            <h1 class="text-xl font-semibold text-gray-900">
                                "SWE Reviewer"
                            </h1>
                        </div>
                    </div>
                </div>
            </div>

            // Main content
            <div class="w-full bg-white dark:bg-gray-800" style="height: calc(100vh - 65px);">
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=DeliverableCheckerPage/>
                </Routes>
            </div>
        </div>
    }
}
