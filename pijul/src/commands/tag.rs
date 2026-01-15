use std::io::Write;

use anyhow::bail;
use clap::Parser;
use jiff::Timestamp;
use log::*;

use crate::commands::common_opts::RepoPath;
use crate::commands::load_channel;
use libpijul::change::ChangeHeader;
use libpijul::{ArcTxn, Base32, ChannelMutTxnT, ChannelRef, ChannelTxnT, MutTxnT, TxnT, TxnTExt};
use pijul_repository::Repository;

#[derive(Parser, Debug)]
pub struct Tag {
    #[clap(flatten)]
    base: RepoPath,
    #[clap(subcommand)]
    subcmd: Option<SubCommand>,
    #[clap(long = "channel")]
    channel: Option<String>,
}

#[derive(Parser, Debug)]
pub enum SubCommand {
    /// Create a tag, which are compressed channels. Note that tags
    /// are not independent from the changes they contain.
    #[clap(name = "create")]
    Create {
        #[clap(flatten)]
        base: RepoPath,
        #[clap(short = 'm', long = "message")]
        message: Option<String>,
        /// Set the author field
        #[clap(long = "author")]
        author: Option<String>,
        /// Tag the current state of this channel instead of the
        /// current channel.
        #[clap(long = "channel")]
        channel: Option<String>,
        #[clap(long = "timestamp")]
        timestamp: Option<Timestamp>,
    },
    /// Restore a tag into a new channel.
    #[clap(name = "checkout")]
    Checkout {
        #[clap(flatten)]
        base: RepoPath,
        tag: String,
        /// Optional new channel name. If not given, the base32
        /// representation of the tag hash is used.
        #[clap(long = "to-channel")]
        to_channel: Option<String>,
    },
    /// Reset the working copy to a tag.
    #[clap(name = "reset")]
    Reset {
        #[clap(flatten)]
        base: RepoPath,
        tag: String,
    },
    /// Delete a tag from a channel. If the same state isn't tagged in
    /// other channels, delete the tag file.
    #[clap(name = "delete")]
    Delete {
        #[clap(flatten)]
        base: RepoPath,
        /// Delete the tag in this channel instead of the current channel
        #[clap(long = "channel")]
        channel: Option<String>,
        tag: String,
    },
}

impl Tag {
    pub async fn run(self) -> Result<(), anyhow::Error> {
        let mut stdout = std::io::stdout();
        match self.subcmd {
            Some(SubCommand::Create {
                base,
                message,
                author,
                channel,
                timestamp,
            }) => {
                let mut repo = Repository::find_root(base.repo_path())?;
                let txn = repo.pristine.arc_txn_begin()?;
                let (channel, _) = load_channel(channel.as_deref(), &*txn.read())?;
                debug!("channel_name = {:?}", channel.read().name);
                try_record(&mut repo, txn.clone(), channel.clone())?;

                let last_t = {
                    let txn = txn.read();
                    let Some(n) = txn.reverse_log(&*channel.read(), None)?.next() else {
                        bail!("Channel {} is empty", channel.read().name.as_str());
                    };
                    n?.0.into()
                };

                log::debug!("last_t = {:?}", last_t);
                if txn.read().is_tagged(&channel.read().tags, last_t)? {
                    bail!("Current state is already tagged")
                }
                let mut tag_path = repo.changes_dir.clone();
                std::fs::create_dir_all(&tag_path)?;

                let mut temp_path = tag_path.clone();
                temp_path.push("tmp");

                let mut w = std::fs::File::create(&temp_path)?;
                let header = header(author.as_deref(), message, timestamp).await?;
                let h: libpijul::Merkle = libpijul::tag::from_channel(
                    &*txn.read(),
                    channel.read().name.as_str(),
                    &header,
                    &mut w,
                )?;
                libpijul::changestore::filesystem::push_tag_filename(&mut tag_path, &h);
                std::fs::create_dir_all(tag_path.parent().unwrap())?;
                std::fs::rename(&temp_path, &tag_path)?;

                txn.write()
                    .put_tags(&mut channel.write().tags, last_t.into(), &h)?;
                txn.commit()?;
                writeln!(stdout, "{}", h.to_base32())?;
            }
            Some(SubCommand::Checkout {
                base,
                mut tag,
                to_channel,
            }) => {
                let repo = Repository::find_root(base.repo_path())?;
                let mut tag_path = repo.changes_dir.clone();
                let h = if let Some(h) = libpijul::Merkle::from_base32(tag.as_bytes()) {
                    libpijul::changestore::filesystem::push_tag_filename(&mut tag_path, &h);
                    h
                } else {
                    super::find_hash(&mut tag_path, &tag)?
                };

                let mut txn = repo.pristine.mut_txn_begin()?;
                tag = h.to_base32();
                let channel_name = to_channel.as_deref().unwrap_or(&tag);
                if txn.load_channel(channel_name)?.is_some() {
                    bail!("Channel {:?} already exists", channel_name)
                }
                let f = libpijul::tag::OpenTagFile::open(&tag_path, &h)?;
                libpijul::tag::restore_channel(f, &mut txn, &channel_name)?;
                txn.commit()?;
                writeln!(stdout, "Tag {} restored as channel {}", tag, channel_name)?;
            }
            Some(SubCommand::Reset { base, tag }) => {
                let repo = Repository::find_root(base.repo_path())?;
                let mut tag_path = repo.changes_dir.clone();
                let h = if let Some(h) = libpijul::Merkle::from_base32(tag.as_bytes()) {
                    libpijul::changestore::filesystem::push_tag_filename(&mut tag_path, &h);
                    h
                } else {
                    super::find_hash(&mut tag_path, &tag)?
                };

                let tag = libpijul::tag::txn::TagTxn::new(&tag_path, &h)?;
                let txn = libpijul::tag::txn::WithTag {
                    tag,
                    txn: repo.pristine.mut_txn_begin()?,
                };
                let channel = txn.channel();
                let txn = ArcTxn::new(txn);

                libpijul::output::output_repository_no_pending_(
                    &repo.working_copy,
                    &repo.changes,
                    &txn,
                    &channel,
                    "",
                    true,
                    None,
                    std::thread::available_parallelism()?.get(),
                    0,
                )?;
                if let Ok(txn) = std::sync::Arc::try_unwrap(txn.0) {
                    txn.into_inner().txn.commit()?
                }
                writeln!(stdout, "Reset to tag {}", h.to_base32())?;
            }
            Some(SubCommand::Delete { base, channel, tag }) => {
                let repo = Repository::find_root(base.repo_path())?;
                let mut tag_path = repo.changes_dir.clone();
                let h = if let Some(h) = libpijul::Merkle::from_base32(tag.as_bytes()) {
                    libpijul::changestore::filesystem::push_tag_filename(&mut tag_path, &h);
                    h
                } else {
                    super::find_hash(&mut tag_path, &tag)?
                };

                let mut txn = repo.pristine.mut_txn_begin()?;
                let (channel, _) = load_channel(channel.as_deref(), &txn)?;

                {
                    let mut ch = channel.write();
                    if let Some(n) = txn.channel_has_state(txn.states(&*ch), &h.into())? {
                        let tags = txn.tags_mut(&mut *ch);
                        txn.del_tags(tags, n.into())?;
                    }
                }
                txn.commit()?;
                writeln!(stdout, "Deleted tag {}", h.to_base32())?;
            }
            None => {
                let repo = Repository::find_root(self.base.repo_path())?;
                let txn = repo.pristine.txn_begin()?;
                let (channel, _) = load_channel(self.channel.as_deref(), &txn)?;
                let mut tag_path = repo.changes_dir.clone();
                super::pager(repo.config.pager.as_ref());
                for t in txn.rev_iter_tags(txn.tags(&*channel.read()), None)? {
                    let (t, _) = t?;
                    let (_, m) = txn.get_changes(&channel, (*t).into())?.unwrap();
                    libpijul::changestore::filesystem::push_tag_filename(&mut tag_path, &m);
                    debug!("tag path {:?}", tag_path);
                    let mut f = libpijul::tag::OpenTagFile::open(&tag_path, &m)?;
                    let header = f.header()?;
                    writeln!(stdout, "State {}", m.to_base32())?;
                    writeln!(stdout, "Author: {:?}", header.authors)?;
                    writeln!(stdout, "Date: {}", header.timestamp)?;
                    writeln!(stdout, "\n    {}\n", header.message)?;
                    libpijul::changestore::filesystem::pop_filename(&mut tag_path);
                }
            }
        }
        Ok(())
    }
}

async fn header(
    author: Option<&str>,
    message: Option<String>,
    timestamp: Option<Timestamp>,
) -> Result<ChangeHeader, anyhow::Error> {
    let mut authors = Vec::new();
    use libpijul::change::Author;
    let mut b = std::collections::BTreeMap::new();
    if let Some(ref a) = author {
        b.insert("name".to_string(), a.to_string());
    } else if let Some(_dir) = pijul_config::global_config_dir() {
        let k = pijul_identity::public_key(&pijul_identity::choose_identity_name().await?)?;
        b.insert("key".to_string(), k.key);
    }
    authors.push(Author(b));
    let header = ChangeHeader {
        message: message.clone().unwrap_or_else(String::new),
        authors,
        description: None,
        timestamp: timestamp.unwrap_or_else(Timestamp::now),
    };
    if header.message.is_empty() {
        let toml = toml::to_string_pretty(&header)?;
        loop {
            let edited_header = edit::edit(&toml)?;
            if let Ok(header) = toml::from_str(&edited_header) {
                return Ok(header);
            }
        }
    } else {
        Ok(header)
    }
}

fn try_record<T: ChannelMutTxnT + TxnT + Send + Sync + 'static>(
    repo: &mut Repository,
    txn: ArcTxn<T>,
    channel: ChannelRef<T>,
) -> Result<(), anyhow::Error> {
    let mut state = libpijul::RecordBuilder::new();
    state.record(
        txn,
        libpijul::Algorithm::default(),
        false,
        &libpijul::DEFAULT_SEPARATOR,
        channel,
        &repo.working_copy,
        &repo.changes,
        "",
        std::thread::available_parallelism()?.get(),
    )?;
    let rec = state.finish();
    if !rec.actions.is_empty() {
        bail!("Cannot change channel, as there are unrecorded changes.")
    }
    Ok(())
}
