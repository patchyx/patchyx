use clap::{Parser, ValueHint};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct RepoPath {
    /// Work with the repository at PATH instead of the one containing the current directory.
    #[clap(long = "repository", value_name = "PATH", value_hint = ValueHint::DirPath)]
    repo_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct RepoAndChannel {
    #[clap(flatten)]
    base: RepoPath,
    /// Work with CHANNEL instead of the current channel
    #[clap(long = "channel")]
    channel: Option<String>,
}

impl RepoPath {
    pub fn repo_path(&self) -> Option<&Path> {
        self.repo_path.as_deref()
    }
}

impl RepoAndChannel {
    pub fn repo_path(&self) -> Option<&Path> {
        self.base.repo_path()
    }

    pub fn channel(&self) -> Option<&str> {
        self.channel.as_deref()
    }
}
