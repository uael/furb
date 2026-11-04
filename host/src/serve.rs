//! The door of the daemon to one peer: its hello, the role of each verb, and the frames of its
//! answers and of what the daemon pushes to it.

use std::future::Future;

use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
use tokio::{
  io::{self, AsyncRead, AsyncWrite},
  sync::mpsc,
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};

use crate::{
  error::{ProtocolError, TransportError},
  protocol::{Answer, BOUND, Frame, Hello, Kind, PROTOCOL, Peer, Request, Verb},
  transport::Who,
};

/// What the daemon pushes to one peer, as the events of a life that it attached.
pub type Outbox = mpsc::UnboundedSender<Frame>;

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
/// allows, answered at once and in any order. A hello of another protocol, a first frame that is
/// no hello, and a frame past the bound close the peer, each with its refusal.
pub async fn serve<H: Host, S: AsyncRead + AsyncWrite>(
  host: &H,
  stream: S,
  who: Who,
) -> Result<(), TransportError> {
  let (read, write) = io::split(stream);
  let mut lines = FramedRead::new(read, LinesCodec::new_with_max_length(BOUND));
  let mut out = FramedWrite::new(write, LinesCodec::new());
  let peer = match heard(&mut lines).await? {
    None => return Ok(()),
    Some(Ok(Request { id, verb: Verb::Hello(Hello { kind: Kind::Skill, name: None, .. }) })) => {
      return closed(&mut out, Some(id), ProtocolError::Nameless).await;
    }
    Some(Ok(Request { id, verb: Verb::Hello(hello) })) if hello.protocol == PROTOCOL => {
      let peer = Peer { login: who.login, role: who.role, kind: hello.kind, name: hello.name };
      let answer = Answer::Welcome { protocol: PROTOCOL, peer: peer.clone() };
      send(&mut out, &Frame::Reply { id, answer }).await?;
      peer
    }
    Some(Ok(Request { id, verb: Verb::Hello(hello) })) => {
      let error = ProtocolError::Protocol { ours: PROTOCOL, theirs: hello.protocol };
      return closed(&mut out, Some(id), error).await;
    }
    Some(Ok(Request { id, .. })) => return closed(&mut out, Some(id), ProtocolError::First).await,
    Some(Err(error)) => return closed(&mut out, None, error).await,
  };
  let (outbox, mut pushed) = mpsc::unbounded_channel();
  let mut answers = FuturesUnordered::new();
  loop {
    tokio::select! {
      heard = heard(&mut lines) => match heard? {
        None => return Ok(()),
        Some(Ok(Request { id, verb })) => match allowed(&peer, &verb) {
          Ok(()) => answers.push(answered(host, &peer, &outbox, id, verb)),
          Err(error) => send(&mut out, &Frame::refused(Some(id), error)).await?,
        },
        Some(Err(error @ ProtocolError::Large { .. })) => return closed(&mut out, None, error).await,
        Some(Err(error)) => send(&mut out, &Frame::refused(None, error)).await?,
      },
      Some(frame) = pushed.recv() => send(&mut out, &frame).await?,
      Some(frame) = answers.next() => send(&mut out, &frame).await?,
    }
  }
}

/// The next request of a peer, none at its end, or the refusal of its frame. A blank line is no
/// frame.
async fn heard<R: AsyncRead + Unpin>(
  lines: &mut FramedRead<R, LinesCodec>,
) -> Result<Option<Result<Request, ProtocolError>>, TransportError> {
  loop {
    return match lines.next().await {
      None => Ok(None),
      Some(Ok(line)) if line.trim().is_empty() => continue,
      Some(Ok(line)) => Ok(Some(
        serde_json::from_str(&line).map_err(|no| ProtocolError::Unread { why: no.to_string() }),
      )),
      Some(Err(LinesCodecError::MaxLineLengthExceeded)) => {
        Ok(Some(Err(ProtocolError::Large { bound: BOUND })))
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
async fn answered<H: Host>(host: &H, peer: &Peer, outbox: &Outbox, id: u64, verb: Verb) -> Frame {
  match host.answer(peer, verb, outbox).await {
    Ok(answer) => Frame::Reply { id, answer },
    Err(error) => Frame::refused(Some(id), error),
  }
}

/// One frame written to a peer, as one line.
async fn send<W: AsyncWrite + Unpin>(
  out: &mut FramedWrite<W, LinesCodec>,
  frame: &Frame,
) -> Result<(), TransportError> {
  Ok(out.send(serde_json::to_string(frame)?).await?)
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
