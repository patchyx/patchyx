//! Error types for the Patchyx server.
//!
//! This module provides a unified error type for all server operations,
//! with proper context and conversion from underlying library errors.

use std::io;
use thiserror::Error;

/// The main error type for server operations.
#[derive(Debug, Error)]
pub enum ServerError {
    /// I/O errors (file operations, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// SSH protocol or connection errors
    #[error("SSH error: {0}")]
    Ssh(String),

    /// Configuration errors (missing values, invalid format)
    #[error("Configuration error: {0}")]
    Config(String),

    /// Repository operation errors
    #[error("Repository error: {0}")]
    Repository(String),

    /// Pijul protocol errors (invalid commands, malformed data)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Authentication/authorization errors
    #[error("Auth error: {0}")]
    Auth(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ServerError {
    /// Create an SSH error with a message.
    pub fn ssh(msg: impl Into<String>) -> Self {
        Self::Ssh(msg.into())
    }

    /// Create a config error with a message.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a repository error with a message.
    pub fn repository(msg: impl Into<String>) -> Self {
        Self::Repository(msg.into())
    }

    /// Create a protocol error with a message.
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    /// Create an auth error with a message.
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    /// Create a not found error with a message.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an internal error with a message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

/// A specialized Result type for server operations.
pub type Result<T> = std::result::Result<T, ServerError>;

// Conversion from anyhow::Error for compatibility
impl From<anyhow::Error> for ServerError {
    fn from(err: anyhow::Error) -> Self {
        ServerError::Internal(err.to_string())
    }
}

// Conversion from thrussh::Error
impl From<thrussh::Error> for ServerError {
    fn from(err: thrussh::Error) -> Self {
        ServerError::Ssh(err.to_string())
    }
}

// Conversion from thrussh_keys::Error
impl From<thrussh_keys::Error> for ServerError {
    fn from(err: thrussh_keys::Error) -> Self {
        ServerError::Ssh(format!("Key error: {}", err))
    }
}
