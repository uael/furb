//! The transports of the network (#983, D6): TCP and WebSocket, which listen only on loopback or
//! on the tailnet, where `WireGuard` encrypts. A frame crosses TCP as one line, as on the machine,
//! and WebSocket as one text message. The identity of a peer of the tailnet is the login that the
//! local API of Tailscale gives for its address, with the role that the access list gives it.

use std::{
  io,
  net::{IpAddr, Ipv4Addr, SocketAddr},
  path::PathBuf,
  pin::Pin,
  task::{Context, Poll, ready},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use futures::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Empty};
use hyper::{
  Request, StatusCode,
  body::Bytes,
  header::{AUTHORIZATION, HOST},
};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, de::DeserializeOwned};
use tokio::{
  io::{AsyncRead, AsyncWrite, ReadBuf},
  net::{TcpListener, TcpStream},
};
use tokio_tungstenite::{
  WebSocketStream,
  tungstenite::{Message, protocol::WebSocketConfig},
};

use super::{Dialer, Identity, Who};
use crate::{
  access::Access,
  error::{AccessError, TransportError},
  protocol::BOUND,
};

/// The transport of TCP at an address on loopback or on the tailnet. A listener of WebSocket is a
/// listener of TCP whose streams [`upgraded`] opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tcp {
  addr: SocketAddr,
}

/// The transport of WebSocket at an address on loopback or on the tailnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebSocket {
  addr: SocketAddr,
}

impl Tcp {
  pub fn new(addr: SocketAddr) -> Self {
    Tcp { addr }
  }

  /// The transport listening, or refused at an address that is neither on loopback nor on the
  /// tailnet, since no other network encrypts the frames.
  pub async fn listen(&self) -> Result<TcpListener, TransportError> {
    if !reachable(self.addr.ip()) {
      return Err(TransportError::Open { addr: self.addr });
    }
    let place = self.addr.to_string();
    TcpListener::bind(self.addr).await.map_err(|source| TransportError::Listen { place, source })
  }
}

impl Dialer for Tcp {
  type Stream = TcpStream;

  async fn dial(&self) -> Result<TcpStream, TransportError> {
    reached(self.addr).await
  }
}

impl WebSocket {
  pub fn new(addr: SocketAddr) -> Self {
    WebSocket { addr }
  }
}

impl Dialer for WebSocket {
  type Stream = Messages<TcpStream>;

  async fn dial(&self) -> Result<Self::Stream, TransportError> {
    let stream = reached(self.addr).await?;
    let url = format!("ws://{}/", self.addr);
    let opened = tokio_tungstenite::client_async_with_config(url, stream, Some(config())).await;
    Ok(Messages::new(opened.map_err(|no| TransportError::Upgrade(Box::new(no)))?.0))
  }
}

/// The WebSocket of a stream that a listener accepted, once its peer said the upgrade.
pub async fn upgraded(stream: TcpStream) -> Result<Messages<TcpStream>, TransportError> {
  let opened = tokio_tungstenite::accept_async_with_config(stream, Some(config())).await;
  Ok(Messages::new(opened.map_err(|no| TransportError::Upgrade(Box::new(no)))?))
}

/// A message past the bound of a frame breaks the stream.
fn config() -> WebSocketConfig {
  WebSocketConfig::default().max_message_size(Some(BOUND)).max_frame_size(Some(BOUND))
}

async fn reached(addr: SocketAddr) -> Result<TcpStream, TransportError> {
  let place = addr.to_string();
  TcpStream::connect(addr).await.map_err(|source| TransportError::Dial { place, source })
}

/// Whether an address is on loopback or on the tailnet: `100.64.0.0/10` or `fd7a:115c:a1e0::/48`.
fn reachable(ip: IpAddr) -> bool {
  match ip.to_canonical() {
    IpAddr::V4(ip) => ip.is_loopback() || (ip.octets()[0] == 100 && ip.octets()[1] & 0xc0 == 64),
    IpAddr::V6(ip) => ip.is_loopback() || ip.segments()[..3] == [0xfd7a, 0x115c, 0xa1e0],
  }
}

/// The frames of a WebSocket as the lines of a stream of bytes: each message that it reads is a
/// line, and each line that it writes is a text message.
pub struct Messages<S> {
  socket: WebSocketStream<S>,
  read: Vec<u8>,
  at: usize,
  written: Vec<u8>,
}

impl<S> Messages<S> {
  fn new(socket: WebSocketStream<S>) -> Self {
    Messages { socket, read: Vec::new(), at: 0, written: Vec::new() }
  }
}

impl<S: AsyncRead + AsyncWrite + Unpin> Messages<S> {
  /// Each whole line written so far, sent as a message.
  fn poll_lines(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    while let Some(end) = self.written.iter().position(|&byte| byte == b'\n') {
      ready!(self.socket.poll_ready_unpin(cx)).map_err(io::Error::other)?;
      let mut line: Vec<u8> = self.written.drain(..=end).collect();
      line.pop();
      let text =
        String::from_utf8(line).map_err(|no| io::Error::new(io::ErrorKind::InvalidData, no))?;
      self.socket.start_send_unpin(Message::text(text)).map_err(io::Error::other)?;
    }
    Poll::Ready(Ok(()))
  }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for Messages<S> {
  fn poll_read(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<io::Result<()>> {
    let this = &mut *self;
    while this.at == this.read.len() {
      let text = match ready!(this.socket.poll_next_unpin(cx)) {
        None | Some(Ok(Message::Close(_))) => return Poll::Ready(Ok(())),
        Some(Ok(Message::Text(text))) => text,
        Some(Ok(_)) => continue,
        Some(Err(no)) => return Poll::Ready(Err(io::Error::other(no))),
      };
      this.read.clear();
      this.read.extend_from_slice(text.as_bytes());
      this.read.push(b'\n');
      this.at = 0;
    }
    let taken = buf.remaining().min(this.read.len() - this.at);
    buf.put_slice(&this.read[this.at..this.at + taken]);
    this.at += taken;
    Poll::Ready(Ok(()))
  }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for Messages<S> {
  fn poll_write(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    ready!(self.poll_lines(cx))?;
    self.written.extend_from_slice(buf);
    Poll::Ready(Ok(buf.len()))
  }

  fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    ready!(self.poll_lines(cx))?;
    self.socket.poll_flush_unpin(cx).map_err(io::Error::other)
  }

  fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    ready!(self.as_mut().poll_flush(cx))?;
    self.socket.poll_close_unpin(cx).map_err(io::Error::other)
  }
}

/// The local API of the Tailscale of this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalApi {
  /// The socket of tailscaled, or its pipe on Windows.
  Socket(PathBuf),
  /// A port of loopback and its token, as the app of Tailscale for macOS gives them.
  Port { port: u16, token: String },
}

/// The identity of a peer of the tailnet: the login that the local API of Tailscale gives for its
/// address, with the role that the access list gives that login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tailscale {
  api: LocalApi,
  access: Access,
}

#[derive(Deserialize)]
struct Status {
  #[serde(rename = "TailscaleIPs", default)]
  ips: Vec<IpAddr>,
}

#[derive(Deserialize)]
struct Whois {
  #[serde(rename = "UserProfile")]
  profile: Profile,
}

#[derive(Deserialize)]
struct Profile {
  #[serde(rename = "LoginName")]
  login: String,
}

impl LocalApi {
  /// The local API of the Tailscale that runs on this machine: the socket of tailscaled, or the
  /// port of the app for macOS. None when no Tailscale runs here, or when it is the app of the App
  /// Store, whose port no file names.
  pub fn find() -> Option<LocalApi> {
    #[cfg(unix)]
    const SOCKETS: [&str; 2] = ["/var/run/tailscale/tailscaled.sock", "/var/run/tailscaled.socket"];
    #[cfg(windows)]
    const SOCKETS: [&str; 1] = [r"\\.\pipe\ProtectedPrefix\Administrators\Tailscale\tailscaled"];
    if let Some(socket) = SOCKETS.iter().map(PathBuf::from).find(|one| one.exists()) {
      return Some(LocalApi::Socket(socket));
    }
    let port = std::fs::read_link("/Library/Tailscale/ipnport").ok()?;
    let port: u16 = port.to_str()?.parse().ok()?;
    let token = std::fs::read_to_string(format!("/Library/Tailscale/sameuserproof-{port}")).ok()?;
    Some(LocalApi::Port { port, token: token.trim().to_owned() })
  }

  /// The answer of the local API to a path, as JSON.
  async fn asked<T: DeserializeOwned>(&self, path: &str) -> Result<T, AccessError> {
    let request = Request::get(path).header(HOST, "local-tailscaled.sock");
    let body = match self {
      LocalApi::Socket(socket) => {
        let place = || socket.display().to_string();
        let stream =
          connected(socket).await.map_err(|source| AccessError::Api { place: place(), source })?;
        fetched(stream, request.body(Empty::new())?).await?
      }
      LocalApi::Port { port, token } => {
        let place = (Ipv4Addr::LOCALHOST, *port);
        let stream = TcpStream::connect(place).await;
        let stream =
          stream.map_err(|source| AccessError::Api { place: format!("port {port}"), source })?;
        let basic = format!("Basic {}", STANDARD.encode(format!(":{token}")));
        fetched(stream, request.header(AUTHORIZATION, basic).body(Empty::new())?).await?
      }
    };
    let (status, body) = body;
    if status != StatusCode::OK {
      let why = String::from_utf8_lossy(&body).trim().to_owned();
      return Err(AccessError::Status { path: path.to_owned(), status: status.as_u16(), why });
    }
    serde_json::from_slice(&body)
      .map_err(|source| AccessError::Unread { path: path.to_owned(), source })
  }
}

#[cfg(unix)]
async fn connected(socket: &std::path::Path) -> io::Result<tokio::net::UnixStream> {
  tokio::net::UnixStream::connect(socket).await
}

#[cfg(windows)]
async fn connected(
  socket: &std::path::Path,
) -> io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
  tokio::net::windows::named_pipe::ClientOptions::new().open(socket)
}

/// The status and the body of the answer to a request on a stream of the local API.
async fn fetched<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
  stream: S,
  request: Request<Empty<Bytes>>,
) -> Result<(StatusCode, Bytes), AccessError> {
  let (mut sender, connection) =
    hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;
  let asked = async move {
    let response = sender.send_request(request).await?;
    let status = response.status();
    Ok::<_, hyper::Error>((status, response.into_body().collect().await?.to_bytes()))
  };
  // The API closes its end once it answered, so a shutdown of ours would fail on a closed socket.
  let (answer, ended) = tokio::join!(asked, connection.without_shutdown());
  let answer = answer?;
  ended?;
  Ok(answer)
}

impl Tailscale {
  pub fn new(api: LocalApi, access: Access) -> Self {
    Tailscale { api, access }
  }

  /// The addresses of this machine on the tailnet.
  pub async fn addresses(&self) -> Result<Vec<IpAddr>, AccessError> {
    Ok(self.api.asked::<Status>("/localapi/v0/status?peers=false").await?.ips)
  }
}

impl Identity for Tailscale {
  type Stream = TcpStream;

  /// The login of the peer at the address of the stream, refused unless the access list gives it a
  /// role.
  async fn identify(&self, stream: &TcpStream) -> Result<Who, TransportError> {
    let addr = stream.peer_addr().map_err(TransportError::Accept)?;
    let addr: String = form_urlencoded::byte_serialize(addr.to_string().as_bytes()).collect();
    let whois: Whois = self.api.asked(&format!("/localapi/v0/whois?addr={addr}")).await?;
    let role = self.access.role(&whois.profile.login)?;
    Ok(Who { login: whois.profile.login, role })
  }
}

#[cfg(test)]
mod tests {
  use super::reachable;

  #[test]
  fn only_loopback_and_the_tailnet_are_reachable() {
    let reached = ["127.0.0.1", "::1", "100.64.0.1", "100.127.255.254", "fd7a:115c:a1e0::1"];
    let refused =
      ["0.0.0.0", "::", "100.128.0.1", "100.63.255.255", "192.168.1.2", "fd7a:115c:a1e1::1"];
    for one in reached {
      assert!(reachable(one.parse().unwrap()), "{one}");
    }
    for one in refused {
      assert!(!reachable(one.parse().unwrap()), "{one}");
    }
  }
}
