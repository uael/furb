//! The errors of the host: of a life, of the protocol, of a transport, and of the access of a peer
//! of the network.

use std::{io, net::SocketAddr, path::PathBuf};

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

/// Why the journal of the file changes of a life does not hold each change that the life made.
#[derive(Debug, Error)]
pub(crate) enum JournalError {
  /// The disk refused changes, which the life told at once, each as a notice.
  #[error("the journal of file changes at {} lost {count} of them, the first for: {first}", at.display())]
  Lost {
    at: PathBuf,
    count: usize,
    #[source]
    first: io::Error,
  },
  /// The task of the journal ended before the life.
  #[error("the journal of file changes stopped before the life")]
  Gone,
}

impl From<furb::Fault> for LifeError {
  fn from(fault: furb::Fault) -> Self {
    LifeError::Refused(fault.to_string())
  }
}

impl From<furb::world::config::Error> for LifeError {
  fn from(no: furb::world::config::Error) -> Self {
    LifeError::Refused(no.to_string())
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
  /// A turn or a write for a request of a model that another peer answers: only the peer that
  /// opened the life with `answers` speaks for its requests.
  #[error("another peer answers the request {ask} of a model of the life on the record {life}")]
  Foreign { life: String, ask: u64 },
  /// A turn for a life whose models no peer answers.
  #[error("no peer answers the models of the life on the record {life}")]
  Answerless { life: String },
  /// An open with `answers` of a life whose models a peer that lives answers already.
  #[error("a peer that lives answers the models of the life on the record {life}")]
  Answering { life: String },
  /// A frame of a view that only a window says, or only a skill, and the peer is of another kind.
  #[error("the verb is said by a peer of the kind {needs:?}, and the peer is of the kind {has:?}")]
  Speaker { needs: Kind, has: Kind },
  /// A skill says its name at its hello, since a window mounts the views of a skill by it.
  #[error("a skill says its name at its hello")]
  Nameless,
  #[error("no skill {skill} is attached to the life on the record {life}")]
  NoSkill { life: String, skill: String },
  /// A view belongs to the window that mounted it and to the skill that serves it, and to no other
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
  /// The cause is in the text and not in `source`, since the daemon reports an error by its text.
  #[error("the daemon cannot listen at {place}: {cause}")]
  Listen { place: String, cause: io::Error },
  /// The path of a Unix socket is longer than the platform lets a socket address hold.
  #[error(
    "the socket path {place} is {length} bytes, over the bound of {bound} bytes of this platform"
  )]
  TooLong { place: String, length: usize, bound: usize },
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
  /// Tailscale gives this machine no address on the tailnet.
  #[error("Tailscale gives this machine no address on the tailnet")]
  Tailless,
  /// Plain TCP listens only where `WireGuard` or the machine keeps the frames.
  #[error("plain TCP listens only on loopback or on the tailnet, not on {addr}")]
  Open { addr: SocketAddr },
  #[error("the door of the windows draws no token")]
  Random(#[source] getrandom::Error),
  #[error("the address of the door of the windows cannot be written in {}", file.display())]
  Door { file: PathBuf, source: io::Error },
  /// A peer of the door of the windows gave no token of the daemon, or another.
  #[error("a peer of the door of the windows gave no token of the daemon")]
  Tokenless,
  #[error("the WebSocket of a peer did not open")]
  Upgrade(#[source] Box<tokio_tungstenite::tungstenite::Error>),
  #[error(transparent)]
  Access(#[from] AccessError),
}

/// Why the daemon lets in no peer of the network, or reads no settings of the host.
#[derive(Debug, Error)]
pub enum AccessError {
  #[error("{} cannot be read", path.display())]
  Read { path: PathBuf, source: io::Error },
  #[error("{} holds no settings of the host", path.display())]
  Parse { path: PathBuf, source: serde_json::Error },
  /// The access list gives no role to the login, so the daemon refuses its peer.
  #[error("the access list gives no role to {login}")]
  Unlisted { login: String },
  #[error("the local API of Tailscale does not answer at {place}")]
  Api { place: String, source: io::Error },
  #[error("the local API of Tailscale broke the exchange")]
  Http(#[from] hyper::Error),
  #[error("the request to the local API of Tailscale is not valid")]
  Asked(#[from] hyper::http::Error),
  #[error("the local API of Tailscale refused {path} with {status}: {why}")]
  Status { path: String, status: u16, why: String },
  #[error("the local API of Tailscale answered {path} with no JSON that furb reads")]
  Unread { path: String, source: serde_json::Error },
}
