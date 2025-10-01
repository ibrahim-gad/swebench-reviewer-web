
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{Router, routing::{post, get}};
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use swe_reviewer_web::app::*;
    use swe_reviewer_web::session::SessionManager;
    use swe_reviewer_web::api::{validate_deliverable, download_deliverable, login, auth_callback, logout, user_info, get_file_content_endpoint, get_test_lists_endpoint, search_logs_endpoint, analyze_logs_endpoint};

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Initialize session manager with file-based persistence
    let session_manager = SessionManager::new_with_persistence()
        .await
        .unwrap_or_else(|e| {
            log!("Warning: Failed to load sessions from file, starting fresh: {}", e);
            SessionManager::new()
        });

    // Start periodic session cleanup task
    let cleanup_manager = session_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Run every hour
        loop {
            interval.tick().await;
            cleanup_manager.cleanup_expired_sessions();
        }
    });

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Create API router with SessionManager state
    let api_router = Router::new()
        .route("/api/login", get(login))
        .route("/google-auth", get(auth_callback))
        .route("/api/logout", post(logout))
        .route("/api/user", get(user_info))
        .route("/api/validate", post(validate_deliverable))
        .route("/api/download", post(download_deliverable))
        .route("/api/get_file_content", post(get_file_content_endpoint))
        .route("/api/get_test_lists", post(get_test_lists_endpoint))
        .route("/api/search_logs", post(search_logs_endpoint))
        .route("/api/analyze_logs", post(analyze_logs_endpoint))
        .with_state(session_manager);

    // Create main router with LeptosOptions state
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options)
        .merge(api_router);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
