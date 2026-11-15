//! The door of a host to one peer of the protocol, and the transport of the machine that a peer
//! dials: the daemon serves each window and each kit on it, and each life serves the processes of
//! its kits on it, as [`crate::kits`] says, so the daemon and each door serve with one code.
//!
//! The transport of the machine is a Unix socket in a folder of mode 0700, or a named pipe that
//! only the user reaches on Windows, and the identity of a peer is the login of the user.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  env,
  future::Future,
  io,
  path::PathBuf,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use furb_wire::{
  ProtocolError,
  protocol::{Answer, BOUND, Frame, Hello, KITS, Kind, PROTOCOL, Peer, Request, Role, Verb},
};
use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
#[cfg(unix)]
use rustix::system::uname;
use serde_json::Value;
use thiserror::Error;
use tokio::{
  io::{AsyncRead, AsyncWrite, split},
  sync::{Notify, mpsc, watch},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as local;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as local;

pub use local::{guarded, own, private};

/// Why the transport of the machine gives no peer, or a peer is lost.
#[derive(Debug, Error)]
pub enum Error {
  #[error("the folder {} of the socket cannot be made", folder.display())]
  Folder { folder: PathBuf, source: io::Error },
  /// The folder of the socket is open to another user, so the host puts no socket in it.
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
}

/// Who a peer is, as its transport proved it: its operator, the login that the transport proved,
/// and the role that it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Who {
  pub operator: String,
  pub role: Role,
}

/// The transport of the machine, at a folder: the socket in it, or the pipe named after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
  folder: PathBuf,
}

/// A stream of a peer that the local transport accepted.
pub type Accepted = local::Accepted;

/// A stream that dialed the local transport.
pub type Dialed = local::Dialed;

/// The local transport as it listens, which accepts each peer in turn.
pub struct Listener(local::Listener);

impl Local {
  pub fn new(folder: impl Into<PathBuf>) -> Self {
    Self { folder: folder.into() }
  }

  /// The transport listening, or refused when a daemon listens there already. A socket that no
  /// daemon answers is stale, and gives its place.
  pub async fn listen(&self) -> Result<Listener, Error> {
    local::listen(&self.folder).await.map(Listener)
  }

  /// The transport listening in the place of a daemon that handed its lives over: that daemon still
  /// holds a peer, and a pipe of Windows keeps its name while a peer of it stands, so no first
  /// instance is asked for.
  pub async fn listen_after_handover(&self) -> Result<Listener, Error> {
    local::succeed(&self.folder).await.map(Listener)
  }

  /// A stream to the host that listens at the folder.
  pub async fn dial(&self) -> Result<Dialed, Error> {
    local::dial(&self.folder).await
  }

  /// Who the peer of a stream is: the user, who writes, since no other user reaches the transport.
  pub fn identify(stream: &Accepted) -> Result<Who, Error> {
    local::login(stream).map(|operator| Who { operator, role: Role::Write })
  }
}

impl Listener {
  pub async fn accept(&mut self) -> Result<Accepted, Error> {
    self.0.accept().await
  }
}

/// What the daemon pushes to one peer, as the events of a life that it attached, and how many of
/// those frames wait for the peer to read them.
#[derive(Debug, Clone)]
pub struct Outbox {
  frames: mpsc::UnboundedSender<Frame>,
  waiting: Arc<Waiting>,
}

/// The peer of an outbox is gone, and a frame pushed to it with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gone;

/// How many frames wait in an outbox, and what wakes a sender that waits for room.
#[derive(Debug, Default)]
struct Waiting {
  frames: AtomicUsize,
  read: Notify,
}

impl Outbox {
  /// An outbox, and where its peer reads what it holds.
  pub fn new() -> (Self, mpsc::UnboundedReceiver<Frame>) {
    let (frames, pushed) = mpsc::unbounded_channel();
    (Self { frames, waiting: Arc::default() }, pushed)
  }

  /// A frame pushed to the peer, or dropped once the peer is gone.
  pub fn send(&self, frame: Frame) -> Result<(), Gone> {
    self.waiting.frames.fetch_add(1, Ordering::SeqCst);
    self.frames.send(frame).map_err(|mpsc::error::SendError(_dropped)| {
      self.waiting.frames.fetch_sub(1, Ordering::SeqCst);
      Gone
    })
  }

  /// Once fewer than `most` frames wait for the peer, or the peer is gone. A sender that pushes as
  /// fast as it reads, as a window that attaches at the start of a long log, would else hold a copy
  /// of each event for as long as the peer takes to read it.
  pub async fn room(&self, most: usize) {
    loop {
      let read = self.waiting.read.notified();
      if self.waiting.frames.load(Ordering::SeqCst) < most {
        return;
      }
      tokio::select! {
        () = read => {}
        () = self.frames.closed() => return,
      }
    }
  }

  /// One frame that the peer read.
  pub fn read(&self) {
    self.waiting.frames.fetch_sub(1, Ordering::SeqCst);
    self.waiting.read.notify_waiters();
  }

  /// Once the peer is gone.
  pub async fn closed(&self) {
    self.frames.closed().await;
  }

  /// Whether the peer is gone.
  pub fn is_closed(&self) -> bool {
    self.frames.is_closed()
  }

  /// Whether two outboxes push to the same peer.
  pub fn same_channel(&self, other: &Self) -> bool {
    self.frames.same_channel(&other.frames)
  }
}

/// The role that a peer holds now: the one that its transport gave, which an access list that
/// changes moves, or none once the list drops its operator.
pub type Granted = watch::Receiver<Option<Role>>;

/// The role of a peer that no access list moves.
pub fn fixed(role: Role) -> Granted {
  watch::channel(Some(role)).1
}

/// What a daemon does with each verb of a peer, once the door checked its role.
pub trait Host {
  /// The answer to a verb of a peer, which may push frames to that peer through its outbox.
  fn answer(
    &self,
    peer: &Peer,
    verb: Verb,
    outbox: &Outbox,
  ) -> impl Future<Output = Result<Answer, ProtocolError>> + Send;
}

/// One peer served on a stream until it ends: its hello first, then each verb that its role
/// allows, answered at once and in any order. The role is the one that `granted` gives at the
/// moment of the verb, so a role that moves holds from the next verb, and an operator that the list
/// drops is refused and closed at once. A hello of another protocol, a first frame that is no
/// hello, and a frame past the bound close the peer, each with its refusal.
pub async fn serve<H: Host, S: AsyncRead + AsyncWrite>(
  host: &H,
  stream: S,
  who: Who,
  mut granted: Granted,
) -> Result<(), Error> {
  let (read, write) = split(stream);
  let mut lines = FramedRead::new(read, LinesCodec::new_with_max_length(BOUND));
  let mut out = FramedWrite::new(write, LinesCodec::new());
  let (mut peer, asked) = match heard(&mut lines).await? {
    None => return Ok(()),
    Some(Ok(Request { id, verb: Verb::Hello(Hello { kind: Kind::Skill, name: None, .. }) })) => {
      return closed(&mut out, Some(id), ProtocolError::Nameless).await;
    }
    Some(Ok(Request { id, verb: Verb::Hello(hello) })) if spoken(&hello) => {
      let asked = hello.role;
      let role = asked.map_or(who.role, |asked| asked.min(who.role));
      let peer = Peer { operator: who.operator, role, kind: hello.kind, name: hello.name };
      send(&mut out, &Frame::Reply { id, answer: welcome(&peer) }).await?;
      (peer, asked)
    }
    Some(Ok(Request { id, verb: Verb::Hello(hello) })) => {
      let error = ProtocolError::Protocol { ours: PROTOCOL, theirs: hello.protocol };
      return closed(&mut out, Some(id), error).await;
    }
    Some(Ok(Request { id, .. })) => return closed(&mut out, Some(id), ProtocolError::First).await,
    Some(Err((id, error))) => return closed(&mut out, id, error).await,
  };
  let (outbox, mut pushed) = Outbox::new();
  let mut answers = FuturesUnordered::new();
  loop {
    tokio::select! {
      heard = heard(&mut lines) => match heard? {
        None => return Ok(()),
        Some(Ok(Request { id, verb })) => match allowed(&peer, &verb) {
          Ok(()) => answers.push(answered(host, peer.clone(), &outbox, id, verb)),
          Err(error) => send(&mut out, &Frame::refused(Some(id), error)).await?,
        },
        Some(Err((_, error @ ProtocolError::Large { .. }))) => {
          return closed(&mut out, None, error).await;
        }
        Some(Err((id, error))) => send(&mut out, &Frame::refused(id, error)).await?,
      },
      Ok(()) = granted.changed() => {
        let now = *granted.borrow_and_update();
        let Some(role) = now else {
          let why = format!("the access list no longer names {}", peer.operator);
          return closed(&mut out, None, ProtocolError::Refused { why }).await;
        };
        peer.role = asked.map_or(role, |asked| asked.min(role));
      }
      Some(frame) = pushed.recv() => {
        let sent = send(&mut out, &frame).await;
        outbox.read();
        sent?;
      }
      Some(frame) = answers.next() => send(&mut out, &frame).await?,
    }
  }
}

/// Whether the daemon speaks the protocol of a hello: its own, or for a skill each version from
/// [`KITS`].
fn spoken(hello: &Hello) -> bool {
  match hello.kind {
    Kind::Skill => (KITS..=PROTOCOL).contains(&hello.protocol),
    _ => hello.protocol == PROTOCOL,
  }
}

/// The welcome of a peer: who it is to the daemon, the machine and its build.
fn welcome(peer: &Peer) -> Answer {
  let (machine, build) = (machine(), BUILD.to_owned());
  Answer::Welcome { protocol: PROTOCOL, peer: peer.clone(), machine, build }
}

/// The build of the daemon: the version of furb that serves it.
pub const BUILD: &str = env!("CARGO_PKG_VERSION");

/// The name of the machine of the daemon, without its domain.
pub fn machine() -> String {
  #[cfg(unix)]
  let full = uname().nodename().to_string_lossy().into_owned();
  #[cfg(not(unix))]
  let full = env::var("COMPUTERNAME").unwrap_or_default();
  full.split('.').next().unwrap_or_default().to_owned()
}

/// The next request of a peer, none at its end, or the refusal of its frame, under the id that the
/// frame gives when it gives one, so that a verb that this daemon does not know is refused to the
/// request that waits for it. A blank line is no frame.
async fn heard<R: AsyncRead + Unpin>(
  lines: &mut FramedRead<R, LinesCodec>,
) -> Result<Option<Result<Request, (Option<u64>, ProtocolError)>>, Error> {
  loop {
    return match lines.next().await {
      None => Ok(None),
      Some(Ok(line)) if line.trim().is_empty() => continue,
      Some(Ok(line)) => Ok(Some(serde_json::from_str(&line).map_err(|no| {
        let id = serde_json::from_str::<Value>(&line).ok().and_then(|one| one["id"].as_u64());
        (id, ProtocolError::Unread { why: no.to_string() })
      }))),
      Some(Err(LinesCodecError::MaxLineLengthExceeded)) => {
        Ok(Some(Err((None, ProtocolError::Large { bound: BOUND }))))
      }
      Some(Err(no)) => Err(no.into()),
    };
  }
}

/// Whether a peer may say a verb: it is of the kind that says it, and holds its role.
fn allowed(peer: &Peer, verb: &Verb) -> Result<(), ProtocolError> {
  match (verb.speaker(), verb.needs()) {
    _ if matches!(verb, Verb::Hello(_)) => Err(ProtocolError::Again),
    (Some(needs), _) if needs != peer.kind => Err(ProtocolError::Speaker { needs, has: peer.kind }),
    (_, needs) if needs > peer.role => Err(ProtocolError::Role { needs, has: peer.role }),
    _ => Ok(()),
  }
}

/// The frame of what the host answered to a verb.
async fn answered<H: Host>(host: &H, peer: Peer, outbox: &Outbox, id: u64, verb: Verb) -> Frame {
  match host.answer(&peer, verb, outbox).await {
    Ok(answer) => Frame::Reply { id, answer },
    Err(error) => Frame::refused(Some(id), error),
  }
}

/// One frame written to a peer, as one line.
async fn send<W: AsyncWrite + Unpin>(
  out: &mut FramedWrite<W, LinesCodec>,
  frame: &Frame,
) -> Result<(), Error> {
  Ok(out.send(serde_json::to_string(frame)?).await?)
}

/// The refusal that closes a peer, written before its stream closes.
async fn closed<W: AsyncWrite + Unpin>(
  out: &mut FramedWrite<W, LinesCodec>,
  id: Option<u64>,
  error: ProtocolError,
) -> Result<(), Error> {
  send(out, &Frame::refused(id, error)).await?;
  Ok(SinkExt::<String>::close(out).await?)
}
