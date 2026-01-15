//! Patchyx Pijul Server
//!
//! A production-grade server for hosting Pijul repositories.
//! Supports SSH for push/pull operations and HTTP for web UI and API.

pub mod config;
pub mod error;
pub mod http;
pub mod ssh;

pub use config::ServerConfig;
pub use error::{Result, ServerError};
