//! The proofs of the transports of the network: TCP and WebSocket on loopback, the identity that
//! a fake of the local API of Tailscale gives, the access list, the daemon on the tailnet, and the
//! door of the windows. Each runs on each system: the fake of Tailscale serves on a port of
//! loopback, as the app of Tailscale for macOS does.

#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

#[path = "../../tests/common/yard.rs"]
mod yard;

#[cfg(test)]
mod net {
  use super::yard::Yard;
  use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
  };

  use furb::life::Opening;
  use furb_host::{
    AccessError, Host, Outbox, ProtocolError, STALL, Settings, Tailnet, TransportError,
    access::{Access, Config, Ports},
    protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Peer, Request, Role, Verb},
    run, serve,
    transport::{Dialer, Identity, Local, LocalApi, Tailscale, Tcp, WebSocket, upgraded},
  };
  use serde_json::json;
  use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::mpsc,
  };

  /// A host that answers each verb with the verb itself, as JSON.
  struct Echo;

  impl Host for Echo {
    fn answer(
      &self,
      _: &Peer,
      verb: Verb,
      _: &Outbox,
    ) -> impl Future<Output = Result<Answer, ProtocolError>> + Send {
      std::future::ready(Ok(Answer::Value(serde_json::to_value(&verb).unwrap())))
    }
  }

  /// A peer of a stream: it writes lines and reads frames.
  struct Client<S> {
    lines: BufReader<S>,
  }

  impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    async fn ask(&mut self, id: u64, verb: Verb) -> Option<Frame> {
      let stream = self.lines.get_mut();
      let line = serde_json::to_string(&Request { id, verb }).unwrap() + "\n";
      stream.write_all(line.as_bytes()).await.unwrap();
      stream.flush().await.unwrap();
      let mut line = String::new();
      let read = self.lines.read_line(&mut line).await.unwrap();
      (read > 0).then(|| serde_json::from_str(&line).unwrap())
    }

    async fn hello(&mut self) -> Option<Frame> {
      self.ask(0, Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None })).await
    }
  }

  /// A folder of its own for one test, short enough for the path of a socket.
  fn folder() -> Yard {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Yard::new(format!("fn{}-{n}", std::process::id()))
  }

  /// A fake of the local API of Tailscale on a port of loopback, which gives one login for each
  /// address, and says the path of each request that it heard.
  async fn tailscale(login: &str) -> (LocalApi, mpsc::UnboundedReceiver<String>) {
    let listener = TcpListener::bind(loopback()).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tell, heard) = mpsc::unbounded_channel();
    let login = login.to_owned();
    tokio::spawn(async move {
      loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.split();
        let mut lines = BufReader::new(read);
        let mut first = String::new();
        lines.read_line(&mut first).await.unwrap();
        let mut header = String::from("-");
        while header.trim() != "" {
          header.clear();
          lines.read_line(&mut header).await.unwrap();
        }
        let path = first.split(' ').nth(1).unwrap().to_owned();
        let body = if path.starts_with("/localapi/v0/status") {
          json!({"TailscaleIPs": ["127.0.0.1"]})
        } else {
          json!({"UserProfile": {"ID": 1, "LoginName": login}})
        };
        let body = body.to_string();
        let answer = format!(
          "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
          body.len()
        );
        write.write_all(answer.as_bytes()).await.unwrap();
        // A test that heard what it needed drops its receiver.
        tell.send(path).ok();
      }
    });
    (LocalApi::Port { port, token: "proof".to_owned() }, heard)
  }

  fn access(login: &str, role: Role) -> Access {
    [(login.to_owned(), role)].into_iter().collect()
  }

  fn loopback() -> SocketAddr {
    "127.0.0.1:0".parse().unwrap()
  }

  fn welcomed(login: &str, role: Role) -> Frame {
    let peer = Peer { login: login.to_owned(), role, kind: Kind::Window, name: None };
    Frame::Reply { id: 0, answer: Answer::Welcome { protocol: PROTOCOL, peer } }
  }

  /// The path that asks the login of a peer at an address.
  fn whois(addr: SocketAddr) -> String {
    format!("/localapi/v0/whois?addr={}", addr.to_string().replace(':', "%3A"))
  }

  /// A listener of TCP on loopback that serves one peer under the identity of Tailscale, upgraded
  /// to WebSocket when `websocket` says so.
  async fn door(identity: Tailscale, websocket: bool) -> SocketAddr {
    let listener = Tcp::new(loopback()).listen().await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
      let (stream, _) = listener.accept().await.unwrap();
      let who = identity.identify(&stream).await.unwrap();
      if websocket {
        serve(&Echo, upgraded(stream).await.unwrap(), who).await
      } else {
        serve(&Echo, stream, who).await
      }
    });
    addr
  }

  #[tokio::test]
  async fn a_peer_of_tcp_is_the_login_that_tailscale_gives_for_its_address() {
    let (api, mut heard) = tailscale("alice@github").await;
    let addr = door(Tailscale::new(api, access("alice@github", Role::Write)), false).await;
    let stream = Tcp::new(addr).dial().await.unwrap();
    let from = stream.local_addr().unwrap();
    let mut client = Client { lines: BufReader::new(stream) };
    assert_eq!(client.hello().await, Some(welcomed("alice@github", Role::Write)));
    assert_eq!(heard.recv().await, Some(whois(from)));
    let answer = Answer::Value(serde_json::to_value(Verb::List { config: None }).unwrap());
    assert_eq!(
      client.ask(1, Verb::List { config: None }).await,
      Some(Frame::Reply { id: 1, answer })
    );
  }

  #[tokio::test]
  async fn a_peer_of_websocket_says_each_frame_as_a_message() {
    let (api, _) = tailscale("alice@github").await;
    let addr = door(Tailscale::new(api, access("alice@github", Role::Write)), true).await;
    let mut client = Client { lines: BufReader::new(WebSocket::new(addr).dial().await.unwrap()) };
    assert_eq!(client.hello().await, Some(welcomed("alice@github", Role::Write)));
    let verb = Verb::Pin { life: "/a.jsonl".to_owned(), holder: "job".to_owned() };
    let answer = Answer::Value(serde_json::to_value(&verb).unwrap());
    assert_eq!(client.ask(1, verb).await, Some(Frame::Reply { id: 1, answer }));
  }

  #[tokio::test]
  async fn a_login_with_no_entry_is_refused() {
    let (api, _) = tailscale("mallory@github").await;
    let identity = Tailscale::new(api, access("alice@github", Role::Write));
    let listener = Tcp::new(loopback()).listen().await.unwrap();
    let dialed = Tcp::new(listener.local_addr().unwrap()).dial().await.unwrap();
    let (stream, _) = listener.accept().await.unwrap();
    let refused = identity.identify(&stream).await;
    assert!(
      matches!(
        &refused,
        Err(TransportError::Access(AccessError::Unlisted { login })) if login == "mallory@github"
      ),
      "{refused:?}"
    );
    drop(dialed);
  }

  #[tokio::test]
  async fn a_reader_of_the_tailnet_is_refused_each_write() {
    let (api, _) = tailscale("bob@github").await;
    let addr = door(Tailscale::new(api, access("bob@github", Role::Read)), false).await;
    let mut client = Client { lines: BufReader::new(Tcp::new(addr).dial().await.unwrap()) };
    assert_eq!(client.hello().await, Some(welcomed("bob@github", Role::Read)));
    let verb = Verb::Run { life: "/a.jsonl".to_owned(), word: "1".to_owned(), on: None };
    let error = ProtocolError::Role { needs: Role::Write, has: Role::Read };
    assert_eq!(client.ask(1, verb).await, Some(Frame::refused(Some(1), error)));
  }

  #[tokio::test]
  async fn plain_tcp_listens_on_no_address_off_loopback_and_the_tailnet() {
    let open: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let refused = Tcp::new(open).listen().await;
    assert!(matches!(refused, Err(TransportError::Open { addr }) if addr == open));
  }

  #[test]
  fn host_json_gives_the_ports_of_the_tailnet_and_the_role_of_each_login() {
    let held = folder();
    let path = held.join("host.json");
    assert_eq!(Config::load(&path).unwrap(), Config::default());
    let written = json!({
      "tailnet": {"tcp": 7860, "websocket": 7861},
      "roles": {"alice@github": "write", "bob@github": "read"},
    });
    std::fs::write(&path, written.to_string()).unwrap();
    let config = Config::load(&path).unwrap();
    assert_eq!(config.tailnet, Some(Ports { tcp: Some(7860), websocket: Some(7861) }));
    assert_eq!(config.roles.role("bob@github").unwrap(), Role::Read);
    assert!(matches!(config.roles.role("carol@github"), Err(AccessError::Unlisted { .. })));
  }

  /// A port of loopback that no listener holds now.
  async fn free() -> u16 {
    TcpListener::bind(loopback()).await.unwrap().local_addr().unwrap().port()
  }

  #[tokio::test]
  async fn the_daemon_serves_the_tailnet_by_tcp_and_by_websocket() {
    let (api, _) = tailscale("alice@github").await;
    let ports = Ports { tcp: Some(free().await), websocket: Some(free().await) };
    let identity = Tailscale::new(api, access("alice@github", Role::Read));
    let held = folder();
    let config = Some(held.join("config").display().to_string());
    let opening = Opening { extensions: Some(false), config, ..Opening::default() };
    let settings = Settings {
      opening,
      grace: Duration::from_secs(1),
      stall: STALL,
      tailnet: Some(Tailnet { identity, ports }),
      door: None,
      host: None,
    };
    let local = Local::new(held.join("host"));
    let listener = local.listen().await.unwrap();
    tokio::spawn(async move { run(&local, listener, settings).await });
    let at = |port: Option<u16>| SocketAddr::new([127, 0, 0, 1].into(), port.unwrap());
    let mut tries = 0..250;
    let mut tcp = loop {
      match Tcp::new(at(ports.tcp)).dial().await {
        Ok(stream) => break Client { lines: BufReader::new(stream) },
        Err(no) if tries.next().is_none() => panic!("the daemon never listened: {no}"),
        Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
      }
    };
    let ws = WebSocket::new(at(ports.websocket)).dial().await.unwrap();
    let mut ws = Client { lines: BufReader::new(ws) };
    assert_eq!(tcp.hello().await, Some(welcomed("alice@github", Role::Read)));
    assert_eq!(
      tcp.ask(1, Verb::List { config: None }).await,
      Some(Frame::Reply { id: 1, answer: Answer::Lives(vec![]) })
    );
    assert_eq!(ws.hello().await, Some(welcomed("alice@github", Role::Read)));
    assert_eq!(
      ws.ask(1, Verb::List { config: None }).await,
      Some(Frame::Reply { id: 1, answer: Answer::Lives(vec![]) })
    );
  }
  /// The login of the user of this test, from the same call of the system as the daemon makes.
  fn me() -> String {
    #[cfg(unix)]
    {
      std::env::var("USER").unwrap()
    }
    #[cfg(windows)]
    {
      use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;
      let mut name = [0u16; 257];
      let mut size: u32 = 257;
      // SAFETY: name holds size units, and the call writes at most that many.
      assert_ne!(unsafe { GetUserNameW(name.as_mut_ptr(), &raw mut size) }, 0);
      String::from_utf16_lossy(&name[..usize::try_from(size).unwrap() - 1])
    }
  }

  /// A daemon with no tailnet, which opens its door in `file`.
  async fn daemon(yard: &Yard, file: &std::path::Path) -> tokio::task::JoinHandle<()> {
    let opening = Opening { extensions: Some(false), ..Opening::default() };
    let settings = Settings {
      opening,
      grace: Duration::from_secs(1),
      stall: STALL,
      tailnet: None,
      door: Some(file.to_owned()),
      host: None,
    };
    let local = Local::new(yard.join("host"));
    let listener = local.listen().await.unwrap();
    tokio::spawn(async move {
      run(&local, listener, settings).await.unwrap();
    })
  }

  /// The address that the file of the door says once it says another than `before`.
  async fn said(file: &std::path::Path, before: Option<&str>) -> String {
    for _ in 0..250 {
      if let Ok(bytes) = std::fs::read(file) {
        let said: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let url = said["url"].as_str().unwrap().to_owned();
        if before != Some(url.as_str()) {
          return url;
        }
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the daemon never opened its door in {}", file.display());
  }

  /// The token of an address with its first character changed, so it is never the same token.
  fn forged(url: &str) -> String {
    let (base, token) = url.split_once("?token=").unwrap();
    let first = if token.starts_with('A') { 'B' } else { 'A' };
    format!("{base}?token={first}{}", &token[1..])
  }

  /// Whether the door at an address answers the upgrade with a refusal of HTTP.
  async fn refused(url: &str) -> bool {
    let addr = url.trim_start_matches("ws://").split('/').next().unwrap().to_owned();
    let Ok(stream) = tokio::net::TcpStream::connect(&addr).await else { return true };
    match tokio_tungstenite::client_async(url, stream).await {
      Ok(_) => false,
      Err(no) => no.to_string().contains("403"),
    }
  }

  /// The welcome of the daemon to a window that dials the door at an address.
  async fn welcome(url: &str) -> Frame {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let addr = url.trim_start_matches("ws://").split('/').next().unwrap().to_owned();
    let stream = tokio::net::TcpStream::connect(&addr).await.unwrap();
    let (mut socket, _) = tokio_tungstenite::client_async(url, stream).await.unwrap();
    let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None });
    let request = serde_json::to_string(&Request { id: 0, verb: hello }).unwrap();
    socket.send(Message::text(request)).await.unwrap();
    let Some(Ok(Message::Text(heard))) = socket.next().await else { panic!("no welcome") };
    serde_json::from_str(&heard).unwrap()
  }

  #[test]
  fn a_forged_token_is_never_the_token() {
    for first in ['0', 'A', 'B', '_'] {
      let url = format!("ws://127.0.0.1:1/?token={first}bcd");
      assert_ne!(forged(&url), url);
    }
  }

  #[tokio::test]
  async fn the_door_of_the_windows_lets_in_only_the_peer_that_gives_its_token() {
    let yard = folder();
    // A cold cache: no folder of the door stands before the daemon.
    let file = yard.join("door").join("window.json");
    let _daemon = daemon(&yard, &file).await;
    let url = said(&file, None).await;
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mode = |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode();
      assert_eq!(mode(&file) & 0o777, 0o600);
      assert_eq!(mode(file.parent().unwrap()) & 0o777, 0o700);
    }
    let (base, _) = url.split_once("?token=").unwrap();
    for no in [base.to_owned(), forged(&url)] {
      assert!(refused(&no).await, "{no}");
    }
    assert_eq!(welcome(&url).await, welcomed(&me(), Role::Write));
  }

  #[tokio::test]
  async fn a_new_daemon_opens_its_door_with_a_new_token_in_the_place_of_the_old() {
    let yard = folder();
    let file = yard.join("door").join("window.json");
    let first = daemon(&yard, &file).await;
    let old = said(&file, None).await;
    assert_eq!(welcome(&old).await, welcomed(&me(), Role::Write));
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    let _second = daemon(&yard, &file).await;
    let new = said(&file, Some(&old)).await;
    assert_ne!(new.split_once("?token=").unwrap().1, old.split_once("?token=").unwrap().1);
    let (base, _) = new.split_once("?token=").unwrap();
    let (_, token) = old.split_once("?token=").unwrap();
    assert!(refused(&format!("{base}?token={token}")).await);
    assert_eq!(welcome(&new).await, welcomed(&me(), Role::Write));
  }
}
