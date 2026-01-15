//! SSH server handler implementation.
//!
//! Implements the thrussh Server and Handler traits for handling
//! SSH connections and Pijul protocol commands.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use thrussh::{server, ChannelId, CryptoVec};
use thrussh_keys::key::PublicKey;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::protocol::PijulCommand;
use crate::config::ServerConfig;

/// Per-channel session state.
#[derive(Debug)]
struct ChannelState {
    /// The authenticated username
    user: String,
    /// The command being executed, if any
    command: Option<PijulCommand>,
    /// Buffer for incoming data
    buffer: Vec<u8>,
}

/// SSH server state.
#[derive(Clone)]
pub struct SshServer {
    /// Server configuration
    config: Arc<ServerConfig>,
    /// Active channel sessions
    channels: Arc<Mutex<HashMap<ChannelId, ChannelState>>>,
    /// Connection ID for logging
    conn_id: u64,
}

impl SshServer {
    /// Create a new SSH server instance.
    pub fn new(config: Arc<ServerConfig>, conn_id: u64) -> Self {
        Self {
            config,
            channels: Arc::new(Mutex::new(HashMap::new())),
            conn_id,
        }
    }

    /// Get the repository path for a given repo name.
    fn repo_path(&self, name: &str) -> PathBuf {
        self.config.repos_dir.join(name)
    }

    /// Check if the repository exists.
    fn repo_exists(&self, name: &str) -> bool {
        let path = self.repo_path(name);
        path.exists() && path.is_dir()
    }

    /// Handle a Pijul command execution.
    async fn handle_command(
        &self,
        channel: ChannelId,
        cmd: &PijulCommand,
        session: &mut server::Session,
    ) -> anyhow::Result<()> {
        info!(
            conn = self.conn_id,
            channel = ?channel,
            cmd = ?cmd,
            "Executing Pijul command"
        );

        match cmd {
            PijulCommand::Ping { repo } => {
                if self.repo_exists(repo) {
                    session.data(channel, CryptoVec::from_slice(b"pong\n"));
                    session.exit_status_request(channel, 0);
                } else {
                    session.data(
                        channel,
                        CryptoVec::from_slice(format!("Repository not found: {}\n", repo).as_bytes()),
                    );
                    session.exit_status_request(channel, 1);
                }
            }
            PijulCommand::Clone { repo, channel: ch } => {
                if !self.repo_exists(repo) {
                    session.data(
                        channel,
                        CryptoVec::from_slice(format!("Repository not found: {}\n", repo).as_bytes()),
                    );
                    session.exit_status_request(channel, 1);
                    return Ok(());
                }

                // TODO: Implement actual clone using libpijul
                let msg = format!(
                    "PIJUL_CLONE {} {}\n",
                    repo,
                    ch.as_deref().unwrap_or("main")
                );
                session.data(channel, CryptoVec::from_slice(msg.as_bytes()));
                session.exit_status_request(channel, 0);
            }
            PijulCommand::Pull { repo, channel: ch } => {
                if !self.repo_exists(repo) {
                    session.data(
                        channel,
                        CryptoVec::from_slice(format!("Repository not found: {}\n", repo).as_bytes()),
                    );
                    session.exit_status_request(channel, 1);
                    return Ok(());
                }

                // TODO: Implement actual pull using libpijul
                let msg = format!(
                    "PIJUL_PULL {} {}\n",
                    repo,
                    ch.as_deref().unwrap_or("main")
                );
                session.data(channel, CryptoVec::from_slice(msg.as_bytes()));
                session.exit_status_request(channel, 0);
            }
            PijulCommand::Push { repo, channel: ch } => {
                if !self.repo_exists(repo) {
                    session.data(
                        channel,
                        CryptoVec::from_slice(format!("Repository not found: {}\n", repo).as_bytes()),
                    );
                    session.exit_status_request(channel, 1);
                    return Ok(());
                }

                // TODO: Implement actual push using libpijul
                let msg = format!(
                    "PIJUL_PUSH {} {}\n",
                    repo,
                    ch.as_deref().unwrap_or("main")
                );
                session.data(channel, CryptoVec::from_slice(msg.as_bytes()));
                session.exit_status_request(channel, 0);
            }
        }

        session.close(channel);
        Ok(())
    }
}

/// Factory for creating new SSH server handlers per connection.
pub struct SshServerFactory {
    config: Arc<ServerConfig>,
    next_conn_id: Arc<Mutex<u64>>,
}

impl SshServerFactory {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        Self {
            config,
            next_conn_id: Arc::new(Mutex::new(0)),
        }
    }
}

impl server::Server for SshServerFactory {
    type Handler = SshServer;

    fn new(&mut self, _peer_addr: Option<SocketAddr>) -> SshServer {
        // Generate connection ID synchronously for simplicity
        let conn_id = {
            let mut guard = self.next_conn_id.blocking_lock();
            let id = *guard;
            *guard += 1;
            id
        };
        info!(conn = conn_id, peer = ?_peer_addr, "New SSH connection");
        SshServer::new(self.config.clone(), conn_id)
    }
}

impl server::Handler for SshServer {
    type Error = anyhow::Error;
    type FutureAuth = futures::future::Ready<std::result::Result<(Self, server::Auth), anyhow::Error>>;
    type FutureUnit = futures::future::Ready<std::result::Result<(Self, server::Session), anyhow::Error>>;
    type FutureBool = futures::future::Ready<std::result::Result<(Self, server::Session, bool), anyhow::Error>>;

    fn finished_auth(self, auth: server::Auth) -> Self::FutureAuth {
        futures::future::ready(Ok((self, auth)))
    }

    fn finished_bool(self, b: bool, session: server::Session) -> Self::FutureBool {
        futures::future::ready(Ok((self, session, b)))
    }

    fn finished(self, session: server::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, session)))
    }

    fn auth_publickey(self, user: &str, public_key: &PublicKey) -> Self::FutureAuth {
        info!(
            conn = self.conn_id,
            user = user,
            key_type = ?public_key.name(),
            "Public key authentication attempt"
        );

        // TODO: Validate public key against stored keys for user
        warn!(
            conn = self.conn_id,
            user = user,
            "Accepting all keys (development mode)"
        );

        self.finished_auth(server::Auth::Accept)
    }

    fn auth_none(self, user: &str) -> Self::FutureAuth {
        debug!(conn = self.conn_id, user = user, "Auth none rejected");
        self.finished_auth(server::Auth::Reject)
    }

    fn channel_open_session(
        self,
        channel: ChannelId,
        session: server::Session,
    ) -> Self::FutureUnit {
        debug!(conn = self.conn_id, channel = ?channel, "Channel opened");
        futures::future::ready(Ok((self, session)))
    }

    fn exec_request(
        self,
        channel: ChannelId,
        data: &[u8],
        mut session: server::Session,
    ) -> Self::FutureUnit {
        let command_str = String::from_utf8_lossy(data);
        info!(
            conn = self.conn_id,
            channel = ?channel,
            command = %command_str,
            "Exec request"
        );

        match PijulCommand::parse(&command_str) {
            Ok(cmd) => {
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.handle_command(channel, &cmd, &mut session).await
                    })
                });

                if let Err(e) = result {
                    error!(
                        conn = self.conn_id,
                        channel = ?channel,
                        error = %e,
                        "Command execution failed"
                    );
                    session.data(
                        channel,
                        CryptoVec::from_slice(format!("Error: {}\n", e).as_bytes()),
                    );
                    session.exit_status_request(channel, 1);
                    session.close(channel);
                }
            }
            Err(e) => {
                warn!(
                    conn = self.conn_id,
                    channel = ?channel,
                    error = %e,
                    "Invalid command"
                );
                session.data(
                    channel,
                    CryptoVec::from_slice(format!("Invalid command: {}\n", e).as_bytes()),
                );
                session.exit_status_request(channel, 1);
                session.close(channel);
            }
        }

        futures::future::ready(Ok((self, session)))
    }

    fn data(
        self,
        channel: ChannelId,
        data: &[u8],
        session: server::Session,
    ) -> Self::FutureUnit {
        debug!(
            conn = self.conn_id,
            channel = ?channel,
            len = data.len(),
            "Received data"
        );
        futures::future::ready(Ok((self, session)))
    }

    fn channel_close(self, channel: ChannelId, session: server::Session) -> Self::FutureUnit {
        debug!(conn = self.conn_id, channel = ?channel, "Channel closed");
        futures::future::ready(Ok((self, session)))
    }

    fn channel_eof(self, channel: ChannelId, session: server::Session) -> Self::FutureUnit {
        debug!(conn = self.conn_id, channel = ?channel, "Channel EOF");
        futures::future::ready(Ok((self, session)))
    }
}
