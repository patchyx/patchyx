//! HTTP server module.
//!
//! Provides the REST API for repository management, health checks,
//! and web UI serving.

mod middleware;
pub mod routes;

pub use middleware::{create_cors_layer, create_trace_layer};
pub use routes::create_router;
