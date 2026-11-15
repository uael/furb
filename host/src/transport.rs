//! The transports of the protocol, each a dialer that reaches a daemon and an identity that says
//! who a peer is. On the machine, the transport is a Unix socket in a folder of mode 0700, or a
//! named pipe that only the user reaches on Windows, and the identity of a peer is the login of the
//! user. On the network, it is TCP or WebSocket on loopback or on the tailnet, and the identity
//! of a peer is the login that Tailscale gives, as `net` says.

use std::future::{self, Future};

pub use furb::serve::{Accepted, Dialed, Listener, Local, Who};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::TransportError;

mod net;

pub(crate) use net::{Door, Ticket};
pub use net::{LocalApi, Messages, Tailscale, Tcp, WebSocket, upgraded};

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

impl Dialer for Local {
  type Stream = Dialed;

  async fn dial(&self) -> Result<Self::Stream, TransportError> {
    Ok(Self::dial(self).await?)
  }
}

impl Identity for Local {
  type Stream = Accepted;

  /// The login of the user, who writes: no other user reaches the transport.
  fn identify(
    &self,
    stream: &Accepted,
  ) -> impl Future<Output = Result<Who, TransportError>> + Send {
    future::ready(Self::identify(stream).map_err(TransportError::from))
  }
}
