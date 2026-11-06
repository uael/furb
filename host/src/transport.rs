//! The transports of the protocol, each a dialer that reaches a daemon and an identity that says
//! who a peer is. On the machine, the transport is a Unix socket in a folder of mode 0700, or a
//! named pipe that only the user reaches on Windows, and the identity of a peer is the login of the
//! user. On the network, it is TCP or WebSocket on loopback or on the tailnet, and the identity
//! of a peer is the login that Tailscale gives, as [`net`] says.

use std::{
  future::{self, Future},
  path::PathBuf,
};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::{error::TransportError, protocol::Role};

mod net;
#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as local;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as local;

pub(crate) use net::{Door, Ticket};
pub use net::{LocalApi, Messages, Tailscale, Tcp, WebSocket, upgraded};

/// Who a peer is, as its transport proved it: its login, and the role that it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Who {
  pub login: String,
  pub role: Role,
}

/// A way to reach a daemon.
pub trait Dialer {
  type Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static;

  fn dial(&self) -> impl Future<Output = Result<Self::Stream, TransportError>> + Send;
}

/// Who a peer is, read from the stream that it came by.
pub trait Identity {
  type Stream;

  fn identify(
    &self,
    stream: &Self::Stream,
  ) -> impl Future<Output = Result<Who, TransportError>> + Send;
}

/// The transport of the machine, at a folder: the socket in it, or the pipe named after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
  folder: PathBuf,
}

/// A stream of a peer that the local transport accepted.
pub type Accepted = local::Accepted;

pub(crate) use local::own;

/// The local transport as it listens, which accepts each peer in turn.
pub struct Listener(local::Listener);

impl Local {
  pub fn new(folder: impl Into<PathBuf>) -> Self {
    Self { folder: folder.into() }
  }

  /// The transport listening, or refused when a daemon listens there already. A socket that no
  /// daemon answers is stale, and gives its place.
  pub async fn listen(&self) -> Result<Listener, TransportError> {
    local::listen(&self.folder).await.map(Listener)
  }
}

impl Local {
  /// The transport listening in the place of a daemon that handed its lives over: that daemon still
  /// holds a peer, and a pipe of Windows keeps its name while a peer of it stands, so no first
  /// instance is asked for.
  pub async fn listen_after_handover(&self) -> Result<Listener, TransportError> {
    local::succeed(&self.folder).await.map(Listener)
  }
}

impl Listener {
  pub async fn accept(&mut self) -> Result<Accepted, TransportError> {
    self.0.accept().await
  }
}

impl Dialer for Local {
  type Stream = local::Dialed;

  async fn dial(&self) -> Result<Self::Stream, TransportError> {
    local::dial(&self.folder).await
  }
}

impl Identity for Local {
  type Stream = Accepted;

  /// The login of the user, who writes: no other user reaches the transport.
  fn identify(
    &self,
    stream: &Accepted,
  ) -> impl Future<Output = Result<Who, TransportError>> + Send {
    future::ready(local::login(stream).map(|login| Who { login, role: Role::Write }))
  }
}
