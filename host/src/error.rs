//! The errors of the host: of a life, of the protocol, of a transport, and of the access of a peer
//! of the network.

use std::{io, net::SocketAddr, path::PathBuf};

use furb::world::config;
use hyper::http;
use thiserror::Error;
use tokio_tungstenite::tungstenite;
use tokio_util::codec::LinesCodecError;

pub use furb_wire::ProtocolError;

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
    Self::Refused(fault.to_string())
  }
}

impl From<config::Error> for LifeError {
  fn from(no: config::Error) -> Self {
    Self::Refused(no.to_string())
  }
}

/// Why the daemon stops, or does not start.
#[derive(Debug, Error)]
pub enum DaemonError {
  #[error(transparent)]
  Transport(#[from] TransportError),
  /// The daemon that runs refused a verb of this peer: the handover, or a verb of the command line.
  #[error("the daemon that runs refused: {0}")]
  Refused(#[from] ProtocolError),
  #[error("the daemon that runs closed before it answered")]
  Unanswered,
  #[error("the daemon that runs answered no frame of the protocol")]
  Unread(#[source] serde_json::Error),
}

impl From<LifeError> for ProtocolError {
  fn from(no: LifeError) -> Self {
    Self::Refused { why: no.to_string() }
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
  Upgrade(#[source] Box<tungstenite::Error>),
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
  /// The access list gives no role to the operator, so the daemon refuses its peer.
  #[error("the access list gives no role to {operator}")]
  Unlisted { operator: String },
  #[error("the local API of Tailscale does not answer at {place}")]
  Api { place: String, source: io::Error },
  #[error("the local API of Tailscale broke the exchange")]
  Http(#[from] hyper::Error),
  #[error("the request to the local API of Tailscale is not valid")]
  Asked(#[from] http::Error),
  #[error("the local API of Tailscale refused {path} with {status}: {why}")]
  Status { path: String, status: u16, why: String },
  #[error("the local API of Tailscale answered {path} with no JSON that furb reads")]
  Unread { path: String, source: serde_json::Error },
}
