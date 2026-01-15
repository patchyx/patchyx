//! HTTP middleware configuration.

use axum::http::{header, Method};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

/// Create the middleware stack for the HTTP server.
pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_origin(Any)
}

/// Create the trace layer for request logging.
pub fn create_trace_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
}
