//! The door of the daemon to one peer: its hello, the role of each verb, and the frames of its
//! answers and of what the daemon pushes to it.
#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  env, fs,
  future::Future,
  path::{Path, PathBuf},
  sync::{
    Arc, LazyLock,
    atomic::{AtomicUsize, Ordering},
  },
  time::UNIX_EPOCH,
};

use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
#[cfg(unix)]
use rustix::system::uname;
use serde_json::Value;
use tokio::{
  io::{self, AsyncRead, AsyncWrite},
  sync::{Notify, mpsc, watch},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};

use crate::{
  error::{ProtocolError, TransportError},
  protocol::{Answer, BOUND, Frame, Hello, KITS, Kind, PROTOCOL, Peer, Request, Role, Verb},
  transport::Who,
};

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
  pub(crate) fn new() -> (Self, mpsc::UnboundedReceiver<Frame>) {
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
  pub(crate) fn read(&self) {
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
  /// The home folder of the user of the machine, which the welcome tells each peer.
  fn home(&self) -> Option<PathBuf> {
    home_folder()
  }

  /// The answer to a verb of a peer, which may push frames to that peer through its outbox.
  fn answer(
    &self,
    peer: &Peer,
    verb: Verb,
    outbox: &Outbox,
  ) -> impl Future<Output = Result<Answer, ProtocolError>> + Send;
}

/// One peer served on a stream until it ends: its hello first, then each verb that its role allows,
/// answered at once and in any order. The role is the one that `granted` gives at the moment of the
/// verb, so a role that moves holds from the next verb, and the peer hears it as a frame; an
/// operator that the list drops is refused and closed at once. A hello of another protocol, a first
/// frame that is no hello, and a frame past the bound close the peer, each with its refusal.
pub async fn serve<H: Host, S: AsyncRead + AsyncWrite>(
  host: &H,
  stream: S,
  who: Who,
  mut granted: Granted,
) -> Result<(), TransportError> {
  let (read, write) = io::split(stream);
  let mut lines = FramedRead::new(read, LinesCodec::new_with_max_length(BOUND));
  let mut out = FramedWrite::new(write, LinesCodec::new());
  let (mut peer, asked, version) = match heard(&mut lines).await? {
    None => return Ok(()),
    Some(Ok(Request { id, verb: Verb::Hello(Hello { kind: Kind::Skill, name: None, .. }) })) => {
      return closed(&mut out, Some(id), ProtocolError::Nameless).await;
    }
    Some(Ok(Request { id, verb: Verb::Hello(hello) })) if spoken(&hello) => {
      let asked = hello.role;
      let role = asked.map_or(who.role, |asked| asked.min(who.role));
      let peer = Peer { operator: who.operator, role, kind: hello.kind, name: hello.name };
      send(&mut out, &Frame::Reply { id, answer: welcome(&peer, host.home()) }).await?;
      (peer, asked, hello.protocol)
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
        let role = asked.map_or(role, |asked| asked.min(role));
        if role != peer.role {
          peer.role = role;
          told(&mut out, Frame::Granted { role }, version).await?;
        }
      }
      Some(frame) = pushed.recv() => {
        let sent = told(&mut out, frame, version).await;
        outbox.read();
        sent?;
      }
      Some(frame) = answers.next() => told(&mut out, frame, version).await?,
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

/// The welcome of a peer: who it is to the daemon, the machine, its build and the home folder of
/// its user.
fn welcome(peer: &Peer, home: Option<PathBuf>) -> Answer {
  let (machine, build) = (machine(), BUILD.to_owned());
  Answer::Welcome {
    protocol: PROTOCOL,
    peer: peer.clone(),
    machine,
    build,
    program: PROGRAM.clone(),
    capabilities: Vec::new(),
    folders: None,
    home: home.map(|one| one.display().to_string()),
  }
}

/// The home folder of the user of this machine, with no link, as the daemon names each record by
/// its path with no link.
pub(crate) fn home_folder() -> Option<PathBuf> {
  env::home_dir().map(|home| dunce::canonicalize(&home).unwrap_or(home))
}

/// The build of the daemon: the version of furb that serves it.
pub const BUILD: &str = env!("CARGO_PKG_VERSION");

/// The program of this process as it stood when the daemon started, as [`built`] gives it: a build
/// may replace the file while the daemon runs.
pub(crate) static PROGRAM: LazyLock<Option<String>> =
  LazyLock::new(|| env::current_exe().ok().as_deref().and_then(built));

/// What says which build a program is: its size and the time of its last write, none while it
/// cannot be read, as while a build replaces it. Two builds of furb never have both the same.
#[must_use]
pub fn built(program: &Path) -> Option<String> {
  let meta = fs::metadata(program).ok()?;
  let written = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
  Some(format!("{}-{}", meta.len(), written.as_nanos()))
}

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
) -> Result<Option<Result<Request, (Option<u64>, ProtocolError)>>, TransportError> {
  loop {
    return match lines.next().await {
      None => Ok(None),
      Some(Ok(line)) if line.trim().is_empty() => continue,
      Some(Ok(line)) => Ok(Some(serde_json::from_str(&line).map_err(|no| {
        let id = serde_json::from_str::<Value>(&line).ok().and_then(|one| one.get("id")?.as_u64());
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

/// A frame written to a peer as the version of its hello reads it, or nothing for a frame that the
/// version did not have: a kit speaks each version from [`KITS`].
async fn told<W: AsyncWrite + Unpin>(
  out: &mut FramedWrite<W, LinesCodec>,
  frame: Frame,
  version: u32,
) -> Result<(), TransportError> {
  match frame.told_to(version) {
    Some(frame) => send(out, &frame).await,
    None => Ok(()),
  }
}

/// One frame written to a peer, as one line. An answer past [`BOUND`] goes as its refusal, since
/// the peer reads no line past it and closes the connection.
async fn send<W: AsyncWrite + Unpin>(
  out: &mut FramedWrite<W, LinesCodec>,
  frame: &Frame,
) -> Result<(), TransportError> {
  let line = serde_json::to_string(frame)?;
  let line = match frame {
    Frame::Reply { id, .. } if line.len() > BOUND => {
      serde_json::to_string(&Frame::refused(Some(*id), ProtocolError::Large { bound: BOUND }))?
    }
    _ => line,
  };
  Ok(out.send(line).await?)
}

/// The refusal that closes a peer, written before its stream closes.
async fn closed<W: AsyncWrite + Unpin>(
  out: &mut FramedWrite<W, LinesCodec>,
  id: Option<u64>,
  error: ProtocolError,
) -> Result<(), TransportError> {
  send(out, &Frame::refused(id, error)).await?;
  Ok(SinkExt::<String>::close(out).await?)
}
