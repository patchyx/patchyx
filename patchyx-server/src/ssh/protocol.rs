//! Pijul protocol command parsing.
//!
//! Parses SSH exec requests into Pijul commands.

use crate::error::{Result, ServerError};

/// Pijul protocol commands that can be executed over SSH.
#[derive(Debug, Clone, PartialEq)]
pub enum PijulCommand {
    /// Clone a repository channel
    Clone {
        repo: String,
        channel: Option<String>,
    },
    /// Pull changes from a repository
    Pull {
        repo: String,
        channel: Option<String>,
    },
    /// Push changes to a repository
    Push {
        repo: String,
        channel: Option<String>,
    },
    /// Check if a repository exists
    Ping { repo: String },
}

impl PijulCommand {
    /// Parse an SSH exec command into a PijulCommand.
    ///
    /// Expected formats:
    /// - `pijul clone REPO [CHANNEL]`
    /// - `pijul pull REPO [CHANNEL]`
    /// - `pijul push REPO [CHANNEL]`
    /// - `pijul ping REPO`
    pub fn parse(command: &str) -> Result<Self> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Err(ServerError::protocol("Empty command"));
        }

        // Handle both "pijul <cmd>" and just "<cmd>" formats
        let (cmd, args) = if parts[0] == "pijul" {
            if parts.len() < 2 {
                return Err(ServerError::protocol("Missing pijul subcommand"));
            }
            (parts[1], &parts[2..])
        } else {
            (parts[0], &parts[1..])
        };

        match cmd {
            "clone" => {
                if args.is_empty() {
                    return Err(ServerError::protocol("Clone requires repository name"));
                }
                Ok(PijulCommand::Clone {
                    repo: args[0].to_string(),
                    channel: args.get(1).map(|s| s.to_string()),
                })
            }
            "pull" => {
                if args.is_empty() {
                    return Err(ServerError::protocol("Pull requires repository name"));
                }
                Ok(PijulCommand::Pull {
                    repo: args[0].to_string(),
                    channel: args.get(1).map(|s| s.to_string()),
                })
            }
            "push" => {
                if args.is_empty() {
                    return Err(ServerError::protocol("Push requires repository name"));
                }
                Ok(PijulCommand::Push {
                    repo: args[0].to_string(),
                    channel: args.get(1).map(|s| s.to_string()),
                })
            }
            "ping" => {
                if args.is_empty() {
                    return Err(ServerError::protocol("Ping requires repository name"));
                }
                Ok(PijulCommand::Ping {
                    repo: args[0].to_string(),
                })
            }
            _ => Err(ServerError::protocol(format!("Unknown command: {}", cmd))),
        }
    }

    /// Get the repository name for this command.
    pub fn repo(&self) -> &str {
        match self {
            PijulCommand::Clone { repo, .. } => repo,
            PijulCommand::Pull { repo, .. } => repo,
            PijulCommand::Push { repo, .. } => repo,
            PijulCommand::Ping { repo } => repo,
        }
    }

    /// Get the channel name, defaulting to "main".
    pub fn channel(&self) -> &str {
        match self {
            PijulCommand::Clone { channel, .. } => channel.as_deref().unwrap_or("main"),
            PijulCommand::Pull { channel, .. } => channel.as_deref().unwrap_or("main"),
            PijulCommand::Push { channel, .. } => channel.as_deref().unwrap_or("main"),
            PijulCommand::Ping { .. } => "main",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clone() {
        let cmd = PijulCommand::parse("pijul clone myrepo").unwrap();
        assert_eq!(
            cmd,
            PijulCommand::Clone {
                repo: "myrepo".to_string(),
                channel: None
            }
        );
    }

    #[test]
    fn test_parse_clone_with_channel() {
        let cmd = PijulCommand::parse("pijul clone myrepo feature").unwrap();
        assert_eq!(
            cmd,
            PijulCommand::Clone {
                repo: "myrepo".to_string(),
                channel: Some("feature".to_string())
            }
        );
    }

    #[test]
    fn test_parse_error() {
        assert!(PijulCommand::parse("pijul").is_err());
        assert!(PijulCommand::parse("pijul clone").is_err());
    }
}
