use std::io::Write;

use crate::commands::common_opts::RepoPath;
use crate::commands::{load_channel, load_channel_exact};
use anyhow::anyhow;
use anyhow::bail;
use clap::Parser;
use libpijul::{ChannelTxnT, MutTxnT, TxnT};
use log::debug;
use pijul_repository::Repository;

#[derive(Parser, Debug)]
pub struct Channel {
    #[clap(flatten)]
    base: RepoPath,
    #[clap(subcommand)]
    subcmd: Option<SubCommand>,
}

#[derive(Parser, Debug)]
pub enum SubCommand {
    /// Delete a channel.
    /// The channel must not be the current channel.
    #[clap(name = "delete")]
    Delete { delete: String },
    /// Rename a channel.
    #[clap(name = "rename", allow_missing_positional = true)]
    Rename {
        /// Name of the channel to be renamed. Defaults to current channel.
        from: Option<String>,
        /// New name of the channel to rename to.
        to: String,
    },
    /// Switch to a channel.
    /// There must not be unrecorded changes in the working copy.
    #[clap(name = "switch")]
    Switch {
        /// Name of the channel to switch to. If not specified, defaults to current channel.
        to: Option<String>,
        #[clap(long = "force", short = 'f')]
        force: bool,
    },
    /// Create a new, empty channel.
    #[clap(name = "new")]
    New {
        name: String,
        #[clap(long = "empty")]
        empty: bool,
        #[clap(long = "force", short = 'f')]
        force: bool,
    },
}

impl Channel {
    pub fn run(self) -> Result<(), anyhow::Error> {
        let mut stdout = std::io::stdout();
        match self.subcmd {
            None => {
                let repo = Repository::find_root(self.base.repo_path())?;
                let txn = repo.pristine.txn_begin()?;
                let current = txn.current_channel().ok();
                for channel in txn.channels("")? {
                    let channel = channel.read();
                    let name = txn.name(&*channel);
                    if current == Some(name) {
                        writeln!(stdout, "* {}", name)?;
                    } else {
                        writeln!(stdout, "  {}", name)?;
                    }
                }
            }
            Some(SubCommand::Delete { ref delete }) => {
                let repo = Repository::find_root(self.base.repo_path())?;
                let mut txn = repo.pristine.mut_txn_begin()?;
                let current = txn.current_channel().ok();
                if Some(delete.as_str()) == current {
                    bail!("Cannot delete current channel")
                }
                if !txn.drop_channel(delete)? {
                    return Err(anyhow!("No such channel: {}", delete));
                }
                txn.commit()?;
            }
            Some(SubCommand::Switch { to, force }) => {
                (crate::commands::reset::Reset {
                    base: self.base,
                    channel: to,
                    dry_run: false,
                    files: Vec::new(),
                    force,
                })
                .switch()?;
            }
            Some(SubCommand::Rename { ref from, ref to }) => {
                let repo = Repository::find_root(self.base.repo_path())?;
                let mut txn = repo.pristine.mut_txn_begin()?;
                let current = txn.current_channel().ok();
                let (from, to) = if let Some(from) = from {
                    (from.as_str(), to.as_str())
                } else if let Some(current) = current {
                    (current, to.as_str())
                } else {
                    bail!("No current channel")
                };

                let mut channel = load_channel_exact(from, &txn)?;

                txn.rename_channel(&mut channel, to)?;
                txn.set_current_channel(&to)?;
                txn.commit()?;
            }
            Some(SubCommand::New { name, empty, force }) => {
                if empty && !force {
                    bail!("If creating an empty channel is really what you want, please use -f.")
                }
                let repo = Repository::find_root(self.base.repo_path())?;
                let mut txn = repo.pristine.mut_txn_begin()?;
                if txn.load_channel(&name)?.is_some() {
                    bail!("Channel {} already exists", name)
                }
                let new = txn.open_or_create_channel(&name)?;
                if !empty {
                    // Safeguard: apply the root patch if we're creating a new channel.
                    let (channel, _) = load_channel(None, &txn)?;
                    let ch = channel.read();
                    use libpijul::{GraphTxnT, MutTxnTExt};
                    let h = if let Some(Ok((k, v))) =
                        libpijul::pristine::changeid_log(&txn, &ch, 0u64.into())?.next()
                    {
                        debug!("initial patch on current channel: {:?} {:?}", k, v);
                        Some(txn.get_external(&v.a)?.unwrap().into())
                    } else {
                        None
                    };
                    if let Some(h) = h {
                        let mut new = new.write();
                        txn.apply_change(&repo.changes, &mut new, &h)?;
                    }
                }
                txn.commit()?;
            }
        }
        Ok(())
    }
}
