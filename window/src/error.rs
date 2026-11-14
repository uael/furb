//! Why the window fails, each with the words that the operator reads.

use std::{io, path::PathBuf, process::ExitStatus, time::Duration};

use furb_host::TransportError;
use tokio_util::codec::LinesCodecError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("the window takes no word yet, and it was given `{0}`")]
  Word(String),
  #[error("the window has no folder: {0}")]
  Folder(io::Error),
  #[error("this machine has no cache of furb, so no daemon has a place on it")]
  NoCache,
  #[error("no program furb stands beside the window or on PATH, so nothing serves the daemon")]
  NoFurb,
  #[error("the terminal failed: {0}")]
  Terminal(#[from] opentui::Error),
  #[error(transparent)]
  Transport(#[from] TransportError),
  #[error("the daemon cannot start in {}: {source}", folder.display())]
  Start { folder: PathBuf, source: io::Error },
  #[error("the daemon ended ({status}) before it listened: {said}")]
  Ended { status: ExitStatus, said: String },
  #[error("the daemon did not listen in {} s", within.as_secs())]
  Slow { within: Duration },
  #[error("the daemon closed the connection")]
  Closed,
  #[error("a frame of the daemon cannot be read: {0}")]
  Frame(#[from] serde_json::Error),
  #[error("the connection to the daemon failed: {0}")]
  Lines(#[from] LinesCodecError),
}
