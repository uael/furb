//! The errors of the host: of a life, of the protocol, and of a transport.

use std::{io, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_util::codec::LinesCodecError;

use crate::protocol::{Kind, Role};

/// Why a life gave no reply to a command.
#[derive(Debug, Error)]
pub enum LifeError {
  /// The life refused the command, with what the engine raised.
  #[error("{0}")]
  Refused(String),
  /// The life ended before it replied, so it replies to no command more.
  #[error("the life ended")]
  Ended,
  /// The machine gave the life no thread.
  #[error("the life has no thread: {0}")]
  Thread(#[source] io::Error),
}

impl From<furb::Fault> for LifeError {
  fn from(fault: furb::Fault) -> Self {
    LifeError::Refused(fault.to_string())
  }
}

/// Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum ProtocolError {
  /// The hello names another protocol, and the daemon closes the peer.
  #[error("the daemon speaks protocol {ours}, and the peer speaks protocol {theirs}")]
  Protocol { ours: u32, theirs: u32 },
  /// The first frame of the peer was no hello, and the daemon closes the peer.
  #[error("the first frame of a peer is its hello")]
  First,
  /// A second hello of the same peer.
  #[error("a peer says its hello once")]
  Again,
  #[error("the verb needs the role {needs:?}, and the peer holds the role {has:?}")]
  Role { needs: Role, has: Role },
  /// A frame that is no frame of the protocol, with what the reader found.
  #[error("the frame is no frame of the protocol: {why}")]
  Unread { why: String },
  /// A frame past the bound, and the daemon closes the peer.
  #[error("a frame holds more than {bound} bytes")]
  Large { bound: usize },
  /// The daemon or the life refused the verb, with why.
  #[error("{why}")]
  Refused { why: String },
  /// The daemon gave its place to the daemon of a new build, or stops.
  #[error("the daemon stops")]
  Stopped,
  #[error("no life of the daemon lives on the record {life}")]
  Closed { life: String },
  /// A life is named by the absolute path of its record, the same for each peer.
  #[error("the record {life} is not an absolute path")]
  Relative { life: String },
  #[error("the peer did not attach the life on the record {life}")]
  Unattached { life: String },
  #[error("no pin of the holder {holder} holds the life on the record {life}")]
  Unpinned { life: String, holder: String },
  /// A turn or a write for a request of a model that the life does not wait for.
  #[error("no model of the life on the record {life} waits for the turn of the request {ask}")]
  Unasked { life: String, ask: u64 },
  /// A turn for a life whose models no peer answers.
  #[error("no peer answers the models of the life on the record {life}")]
  Answerless { life: String },
  /// A frame of a view that only a window says, or only a kit, and the peer is of another kind.
  #[error("the verb is said by a peer of the kind {needs:?}, and the peer is of the kind {has:?}")]
  Speaker { needs: Kind, has: Kind },
  /// A kit says its name at its hello, since a window mounts the views of a kit by it.
  #[error("a kit says its name at its hello")]
  Nameless,
  #[error("no kit {kit} is attached to the life on the record {life}")]
  NoKit { life: String, kit: String },
  /// A view belongs to the window that mounted it and to the kit that serves it, and to no other
  /// peer.
  #[error("the peer holds no view {viewer}")]
  Unmounted { viewer: String },
  /// A viewer names one view of the daemon.
  #[error("a view {viewer} is mounted already")]
  Mounted { viewer: String },
}

/// Why the daemon stops, or does not start.
#[derive(Debug, Error)]
pub enum DaemonError {
  #[error(transparent)]
  Transport(#[from] TransportError),
  /// The daemon that runs refused the handover.
  #[error("the daemon that runs refused the handover: {0}")]
  Refused(#[from] ProtocolError),
  #[error("the daemon that runs closed before it gave its lives")]
  Unanswered,
  #[error("the daemon that runs answered no frame of the protocol")]
  Unread(#[source] serde_json::Error),
}

impl From<LifeError> for ProtocolError {
  fn from(no: LifeError) -> Self {
    ProtocolError::Refused { why: no.to_string() }
  }
}

/// Why a transport gives no peer, or loses one.
#[derive(Debug, Error)]
pub enum TransportError {
  #[error("the folder {} of the socket cannot be made", folder.display())]
  Folder { folder: PathBuf, source: io::Error },
  /// The folder of the socket is open to another user, so the daemon puts no socket in it.
  #[error("the folder {} is not of mode 0700 and of this user alone", folder.display())]
  Exposed { folder: PathBuf },
  #[error("a daemon already listens at {place}")]
  Taken { place: String },
  #[error("the daemon cannot listen at {place}")]
  Listen { place: String, source: io::Error },
  #[error("no daemon answers at {place}")]
  Dial { place: String, source: io::Error },
  #[error("the transport accepts no peer")]
  Accept(#[source] io::Error),
  /// A peer of another user reached the socket.
  #[error("a peer of the user {uid} is not of this user")]
  Stranger { uid: u32 },
  #[error("the login of the user cannot be read")]
  Login(#[source] io::Error),
  #[error("the stream of a peer broke")]
  Peer(#[from] LinesCodecError),
  #[error("a frame cannot be written as JSON")]
  Unwritten(#[from] serde_json::Error),
}
