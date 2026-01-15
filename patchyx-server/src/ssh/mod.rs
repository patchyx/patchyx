//! SSH server module.
//!
//! Implements the SSH protocol handler for Pijul operations.
//! Supports `pijul clone`, `pijul pull`, and `pijul push` over SSH.

pub mod handler;
pub mod protocol;

pub use handler::{SshServer, SshServerFactory};
pub use protocol::PijulCommand;
