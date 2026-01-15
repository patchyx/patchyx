use clap::Parser;
use libpijul::TxnT;
use pijul_repository::Repository;
use std::io::Write;
use std::path::PathBuf;

use crate::commands::common_opts::RepoAndChannel;

#[derive(Parser, Debug)]
pub struct Status {
    #[clap(flatten)]
    pub base: RepoAndChannel,
    /// Add all the changes of this channel as dependencies (except changes implied transitively), instead of the minimal dependencies.
    #[clap(long = "tag")]
    pub tag: bool,
    /// Include the untracked files
    #[clap(short = 'u', long = "untracked")]
    pub untracked: bool,
    /// Show only untracked files
    #[clap(short = 'U', long = "only-untracked")]
    pub only_untracked: bool,
    /// Only diff those paths (files or directories). If missing, diff the entire repository.
    pub prefixes: Vec<PathBuf>,
}

impl Status {
    pub fn run(self) -> Result<(), anyhow::Error> {
        let repo = Repository::find_root(self.base.repo_path())?;
        let mut stdout = std::io::stdout();

        {
            let txn = repo.pristine.txn_begin()?;
            let current = txn.current_channel().ok();
            writeln!(
                stdout,
                "{}",
                current.map_or_else(|| "Not on a channel".into(), |c| format!("On channel: {c}"))
            )?;
        }

        if self.only_untracked {
            let txn = repo.pristine.arc_txn_begin()?;
            return super::diff::print_untracked_files(&repo, txn);
        }

        // Status is just diff with benefits.
        let diff = super::Diff {
            base: self.base,
            json: false,
            tag: self.tag,
            short: true,
            untracked: self.untracked,
            prefixes: self.prefixes,
            patience: false,
            histogram: false,
        };

        diff.run()
    }
}
