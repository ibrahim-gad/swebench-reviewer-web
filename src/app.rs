use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use serde::{Deserialize, Serialize};

pub mod report_checker;
use report_checker::ReportCheckerPage;

#[derive(Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub name: String,
    pub email: String,
    pub authenticated: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthResponse {
    pub auth_url: String,
}

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
                <AuthenticatedApp/>
            </main>
        </Router>
    }
}

#[component]
pub fn AuthenticatedApp() -> impl IntoView {
    let (user_info, set_user_info) = signal::<Option<UserInfo>>(None);
    let (is_loading, set_is_loading) = signal(true);

    Effect::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                match check_auth_status().await {
                    Ok(user) => {
                        set_user_info.set(Some(user));
                        set_is_loading.set(false);
                    }
                    Err(_) => {
                        set_user_info.set(None);
                        set_is_loading.set(false);
                    }
                }
            });
        }
        
        #[cfg(not(feature = "hydrate"))]
        {
            // SSR fallback
            set_user_info.set(None);
            set_is_loading.set(false);
        }
    });

    view! {
        <div class="min-h-screen bg-gray-50">
            <Show
                when=move || !is_loading.get()
                fallback=|| view! { <LoadingSpinner/> }
            >
                <Show
                    when=move || user_info.get().map(|u| u.authenticated).unwrap_or(false)
                    fallback=|| view! { <LoginPage/> }
                >
                    {move || {
                        user_info.get().map(|user| view! { <MainApp user/> })
                    }}
                </Show>
            </Show>
        </div>
    }
}

#[component]
pub fn LoadingSpinner() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center min-h-screen">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span class="ml-2 text-gray-600">"Checking authentication..."</span>
        </div>
    }
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let (is_logging_in, set_is_logging_in) = signal(false);

    let handle_login = move |_| {
        set_is_logging_in.set(true);
        
        #[cfg(feature = "hydrate")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                match get_auth_url().await {
                    Ok(auth_url) => {
                        use web_sys;
                        let _ = web_sys::window()
                            .and_then(|w| w.location().set_href(&auth_url).ok());
                    }
                    Err(e) => {
                        leptos::logging::log!("Login error: {}", e);
                        set_is_logging_in.set(false);
                    }
                }
            });
        }
        
        #[cfg(not(feature = "hydrate"))]
        {
            // SSR fallback
            set_is_logging_in.set(false);
        }
    };

    view! {
        <div class="flex items-center justify-center min-h-screen">
            <div class="max-w-md w-full space-y-8">
                <div class="text-center">
                    <h1 class="text-3xl font-bold text-gray-900 mb-4">
                        "Welcome to SWE Reviewer"
                    </h1>
                    <p class="text-gray-600 mb-8">
                        "Please sign in with Google to access the application"
                    </p>
                </div>
                
                <button
                    class="w-full flex justify-center py-3 px-4 border border-transparent rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
                    on:click=handle_login
                    disabled=move || is_logging_in.get()
                >
                    <Show
                        when=move || is_logging_in.get()
                        fallback=|| view! { "Sign in with Google" }
                    >
                        "Redirecting to Google..."
                    </Show>
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn MainApp(user: UserInfo) -> impl IntoView {
    let handle_logout = move |_| {
        #[cfg(feature = "hydrate")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                match logout().await {
                    Ok(_) => {
                        use web_sys;
                        let _ = web_sys::window()
                            .and_then(|w| w.location().reload().ok());
                    }
                    Err(e) => {
                        leptos::logging::log!("Logout error: {}", e);
                    }
                }
            });
        }
        
        #[cfg(not(feature = "hydrate"))]
        {
            // SSR fallback - just log the action
            leptos::logging::log!("Logout clicked");
        }
    };

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
                        <div class="flex items-center space-x-4">
                            <span class="text-sm text-gray-600">
                                "Hello, " {user.name.clone()}
                            </span>
                            <button
                                class="text-sm text-red-600 hover:text-red-700"
                                on:click=handle_logout
                            >
                                "Logout"
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            // Main content
            <div class="w-full bg-white dark:bg-gray-800" style="height: calc(100vh - 65px);">
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=ReportCheckerPage/>
                </Routes>
            </div>
        </div>
    }
}

// API functions
#[cfg(feature = "hydrate")]
async fn check_auth_status() -> Result<UserInfo, String> {
    use gloo_net::http::Request;
    
    Request::get("/api/user")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<UserInfo>()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))
}

#[cfg(feature = "hydrate")]
async fn get_auth_url() -> Result<String, String> {
    use gloo_net::http::Request;
    
    let response = Request::get("/api/login")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<AuthResponse>()
        .await
        .map_err(|e| format!("Failed to parse auth response: {}", e))?;
    
    Ok(response.auth_url)
}

#[cfg(feature = "hydrate")]
async fn logout() -> Result<(), String> {
    use gloo_net::http::Request;
    
    Request::post("/api/logout")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    Ok(())
}

#[cfg(not(feature = "hydrate"))]
async fn check_auth_status() -> Result<UserInfo, String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "hydrate"))]
async fn get_auth_url() -> Result<String, String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "hydrate"))]
async fn logout() -> Result<(), String> {
    Err("Not available on server".to_string())
}

