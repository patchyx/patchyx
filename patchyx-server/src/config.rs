//! Server configuration management.
//!
//! Configuration is loaded from environment variables with sensible defaults.
//! This allows for easy deployment in containerized environments.

use std::env;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use tracing::info;

use crate::error::{Result, ServerError};

/// Server configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// SSH server bind address
    pub ssh_host: IpAddr,
    /// SSH server port (default: 2222)
    pub ssh_port: u16,
    /// HTTP server bind address
    pub http_host: IpAddr,
    /// HTTP server port (default: 3000)
    pub http_port: u16,
    /// Path to SSH host key file
    pub host_key_path: PathBuf,
    /// Directory containing repositories
    pub repos_dir: PathBuf,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Whether to generate host key if missing
    pub generate_host_key: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ssh_host: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            ssh_port: 2222,
            http_host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            http_port: 3000,
            host_key_path: PathBuf::from("./host_key"),
            repos_dir: PathBuf::from("./repos"),
            log_level: String::from("info"),
            generate_host_key: true,
        }
    }
}

impl ServerConfig {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `PATCHYX_SSH_HOST`: SSH bind address (default: 0.0.0.0)
    /// - `PATCHYX_SSH_PORT`: SSH port (default: 2222)
    /// - `PATCHYX_HTTP_HOST`: HTTP bind address (default: 127.0.0.1)
    /// - `PATCHYX_HTTP_PORT`: HTTP port (default: 3000)
    /// - `PATCHYX_HOST_KEY_PATH`: Path to host key file
    /// - `PATCHYX_REPOS_DIR`: Repository storage directory
    /// - `PATCHYX_LOG_LEVEL`: Logging level
    /// - `PATCHYX_GENERATE_HOST_KEY`: Generate key if missing (default: true)
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(val) = env::var("PATCHYX_SSH_HOST") {
            config.ssh_host = val
                .parse()
                .map_err(|_| ServerError::config(format!("Invalid SSH host: {}", val)))?;
        }

        if let Ok(val) = env::var("PATCHYX_SSH_PORT") {
            config.ssh_port = val
                .parse()
                .map_err(|_| ServerError::config(format!("Invalid SSH port: {}", val)))?;
        }

        if let Ok(val) = env::var("PATCHYX_HTTP_HOST") {
            config.http_host = val
                .parse()
                .map_err(|_| ServerError::config(format!("Invalid HTTP host: {}", val)))?;
        }

        if let Ok(val) = env::var("PATCHYX_HTTP_PORT") {
            config.http_port = val
                .parse()
                .map_err(|_| ServerError::config(format!("Invalid HTTP port: {}", val)))?;
        }

        if let Ok(val) = env::var("PATCHYX_HOST_KEY_PATH") {
            config.host_key_path = PathBuf::from(val);
        }

        if let Ok(val) = env::var("PATCHYX_REPOS_DIR") {
            config.repos_dir = PathBuf::from(val);
        }

        if let Ok(val) = env::var("PATCHYX_LOG_LEVEL") {
            config.log_level = val;
        }

        if let Ok(val) = env::var("PATCHYX_GENERATE_HOST_KEY") {
            config.generate_host_key = val.to_lowercase() == "true" || val == "1";
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    fn validate(&self) -> Result<()> {
        // Ensure repos directory exists or can be created
        if !self.repos_dir.exists() {
            std::fs::create_dir_all(&self.repos_dir).map_err(|e| {
                ServerError::config(format!(
                    "Cannot create repos directory {:?}: {}",
                    self.repos_dir, e
                ))
            })?;
            info!("Created repositories directory: {:?}", self.repos_dir);
        }

        Ok(())
    }

    /// Get the SSH socket address as a string.
    pub fn ssh_addr(&self) -> String {
        format!("{}:{}", self.ssh_host, self.ssh_port)
    }

    /// Get the HTTP socket address.
    pub fn http_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::new(self.http_host, self.http_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.ssh_port, 2222);
        assert_eq!(config.http_port, 3000);
    }
}
