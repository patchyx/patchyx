use clap::Parser;
use libpijul::{Base32, ChannelTxnT, MutTxnT, MutTxnTExt, TxnT, TxnTExt};
use log::debug;

use crate::commands::common_opts::RepoPath;
use crate::commands::load_channel;
use pijul_repository::Repository;

#[derive(Parser, Debug)]
pub struct Fork {
    #[clap(flatten)]
    base: RepoPath,
    /// Make the new channel from this state instead of the current channel
    #[clap(long = "state", conflicts_with = "change", conflicts_with = "channel")]
    state: Option<String>,
    /// Make the new channel from this channel instead of the current channel
    #[clap(long = "channel", conflicts_with = "change", conflicts_with = "state")]
    channel: Option<String>,
    /// Apply this change after creating the channel
    #[clap(long = "change", conflicts_with = "channel", conflicts_with = "state")]
    change: Option<String>,
    /// The name of the new channel
    to: String,
}

impl Fork {
    pub fn run(self) -> Result<(), anyhow::Error> {
        let repo = Repository::find_root(self.base.repo_path())?;
        debug!("{:?}", repo.config);
        let mut txn = repo.pristine.mut_txn_begin()?;
        if let Some(ref ch) = self.change {
            let (hash, _) = txn.hash_from_prefix(ch)?;
            let channel = txn.open_or_create_channel(&self.to)?;
            let mut channel = channel.write();
            txn.apply_change_rec(&repo.changes, &mut channel, &hash)?
        } else {
            let (channel, _) = load_channel(self.channel.as_deref(), &txn)?;
            let mut fork = txn.fork(&channel, &self.to)?;

            if let Some(ref state) = self.state {
                if let Some(state) = libpijul::Merkle::from_base32(state.as_bytes()) {
                    let ch = fork.write();
                    if let Some(n) = txn.channel_has_state(&ch.states, &state.into())? {
                        let n: u64 = n.into();

                        let mut v = Vec::new();
                        for l in txn.reverse_log(&ch, None)? {
                            let (n_, h) = l?;
                            if n_ > n {
                                v.push(h.0.into())
                            } else {
                                break;
                            }
                        }
                        std::mem::drop(ch);
                        for h in v {
                            txn.unrecord(&repo.changes, &mut fork, &h, 0, &repo.working_copy)?;
                        }
                    }
                }
            }
        }
        txn.commit()?;
        Ok(())
    }
}
