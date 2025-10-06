
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{Router, routing::post};
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use swe_reviewer_web::app::*;
    use swe_reviewer_web::api::file_operations::{get_file_content_endpoint, get_test_lists_endpoint};
    use swe_reviewer_web::api::log_analysis::{search_logs_endpoint, analyze_logs_endpoint};
    use swe_reviewer_web::auth::init_service_account_auth;

    // Initialize service account authentication
    if let Err(e) = init_service_account_auth().await {
        log!("Warning: Failed to initialize service account authentication: {}", e);
        log!("Make sure GOOGLE_APPLICATION_CREDENTIALS environment variable is set");
    } else {
        log!("Service account authentication initialized successfully");
    }

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Create API router
    let api_router = Router::new()
        .route("/api/get_file_content", post(get_file_content_endpoint))
        .route("/api/get_test_lists", post(get_test_lists_endpoint))
        .route("/api/search_logs", post(search_logs_endpoint))
        .route("/api/analyze_logs", post(analyze_logs_endpoint));

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
