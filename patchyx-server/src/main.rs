//! Patchyx Pijul Server - Main Entry Point
//!
//! This binary starts both the SSH and HTTP servers for hosting Pijul repositories.

use std::sync::Arc;

use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use patchyx_server::config::ServerConfig;
use patchyx_server::http::routes::AppState;
use patchyx_server::ssh::SshServerFactory;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = match ServerConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize logging
    let log_level = match config.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Patchyx Pijul Server");
    info!("Configuration: {:?}", config);

    let config = Arc::new(config);

    // --- Load or generate SSH host key ---
    let host_key = if config.host_key_path.exists() {
        info!("Loading host key from {:?}", config.host_key_path);
        thrussh_keys::load_secret_key(&config.host_key_path, None)?
    } else if config.generate_host_key {
        info!("Generating new host key");
        let key = thrussh_keys::key::KeyPair::generate_ed25519()
            .ok_or_else(|| anyhow::anyhow!("Failed to generate key"))?;

        // Save the key for persistence
        // Note: thrussh_keys doesn't have a direct save function, 
        // so in production you'd want to handle this properly
        info!("Generated ephemeral host key (not persisted)");
        key
    } else {
        return Err(anyhow::anyhow!(
            "Host key not found at {:?} and generation disabled",
            config.host_key_path
        ));
    };

    // --- SSH Server Setup ---
    let mut ssh_config = thrussh::server::Config::default();
    ssh_config.connection_timeout = Some(std::time::Duration::from_secs(600));
    ssh_config.auth_rejection_time = std::time::Duration::from_secs(3);
    ssh_config.keys.push(host_key);
    let ssh_config = Arc::new(ssh_config);

    let ssh_factory = SshServerFactory::new(config.clone());
    let ssh_addr = config.ssh_addr();

    info!("SSH server listening on {}", ssh_addr);
    let ssh_handle = tokio::spawn(async move {
        if let Err(e) = thrussh::server::run(ssh_config, &ssh_addr, ssh_factory).await {
            error!("SSH server error: {}", e);
        }
    });

    // --- HTTP Server Setup ---
    let app_state = AppState {
        config: config.clone(),
        start_time: std::time::Instant::now(),
    };

    let router = patchyx_server::http::create_router(app_state);
    let http_addr = config.http_addr();

    info!("HTTP server listening on {}", http_addr);
    let listener = tokio::net::TcpListener::bind(http_addr).await?;

    // --- Graceful Shutdown ---
    let shutdown_signal = async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("Shutdown signal received, starting graceful shutdown...");
    };

    // Run HTTP server with graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("HTTP server stopped");

    // Abort SSH server (thrussh doesn't have graceful shutdown built-in)
    ssh_handle.abort();
    info!("SSH server stopped");

    info!("Server shutdown complete");
    Ok(())
}
