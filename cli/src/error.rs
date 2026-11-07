//! The errors of the command line, each with the words that the operator reads on stderr after
//! `furb: `.

use std::{io, num::ParseFloatError, path::PathBuf, time::TryFromFloatSecsError};

use furb::Fault;
use furb_host::{AccessError, DaemonError};
use thiserror::Error;
use tokio::task::JoinError;

/// Why a command of the operator failed.
#[derive(Debug, Error)]
pub(crate) enum CliError {
  /// The grace of `furb host` is no number.
  #[error("{given} is no number of seconds: {source}")]
  Seconds { given: String, source: ParseFloatError },
  /// The grace of `furb host` is a number that no duration holds.
  #[error("{given} s is no grace: {source}")]
  Grace { given: String, source: TryFromFloatSecsError },
  /// A source of the catalog failed its update.
  #[error("the update failed for a source, and the last valid catalog stays")]
  Update,
  /// The update of the catalog ended with no status.
  #[error("the update of the catalog did not end: {0}")]
  Updating(#[source] JoinError),
  /// The record that `facts` reads is no file.
  #[error("No record at {}.", .0.display())]
  NoRecord(PathBuf),
  /// The store cannot read the record that `facts` reads, by what it says.
  #[error("{}", .0.message())]
  Unread(Fault),
  /// The reader of stdout closed the pipe, which ends the command with no word.
  #[error("the reader of stdout closed the pipe")]
  Closed,
  /// Stdout cannot be written.
  #[error("stdout cannot be written: {0}")]
  Printed(#[source] io::Error),
  /// The engine refused, with what it raised.
  #[error(transparent)]
  Refused(#[from] Fault),
  /// The act of the command is paused, with the refusal of a reply that paused it, if any.
  #[error(
    "{id} is paused{}. A wake from a client of the daemon makes it go on.",
    .refused.as_deref().map_or_else(String::new, |why| format!(": {why}"))
  )]
  Paused { id: String, refused: Option<String> },
  /// The machine gave the command no runtime.
  #[error("furb has no runtime: {0}")]
  Runtime(#[source] io::Error),
  /// The machine has no cache of furb, where the daemon lives.
  #[error("this machine has no cache of furb, so the daemon has no place")]
  NoCache,
  /// The daemon stopped.
  #[error(transparent)]
  Daemon(#[from] DaemonError),
  /// `host.json` cannot be read.
  #[error(transparent)]
  Access(#[from] AccessError),
  /// `host.json` turns on the tailnet on a machine with no Tailscale.
  #[error("{} turns on the tailnet, and no Tailscale runs on this machine", .0.display())]
  NoTailscale(PathBuf),
  /// No TUI is found.
  #[error(
    "no TUI is found: set FURB_TUI to the program of the furb TUI, put furb-tui on PATH, or run \
     furb from a checkout of furb after `bun install && bun run build`"
  )]
  NoTui,
  /// The TUI of a checkout runs on bun, and no bun is on PATH.
  #[error(
    "the TUI of {} runs on bun 1.4.2 or later, and no bun is on PATH: see https://bun.sh",
    .0.display()
  )]
  NoBun(PathBuf),
  /// The TUI of a checkout is not built.
  #[error("the TUI of {} is not built: run `bun install && bun run build` there", .0.display())]
  Unbuilt(PathBuf),
  /// The program of the TUI did not start.
  #[error("{} did not start: {source}", program.display())]
  Started { program: PathBuf, source: io::Error },
}
