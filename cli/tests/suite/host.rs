//! The daemon of `furb host` on the tailnet, with a fake of the local API of Tailscale: it reads
//! `host.json` again at each change of the file.

#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that tokio starts for it"
)]

use crate::{tailscale::tailscale, yard::Yard};

use std::{
  fs,
  path::Path,
  process::{self, Stdio},
  time::Duration,
};

use furb_host::{
  protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Request, Role, Verb},
  transport::LocalApi,
};
use serde_json::json;
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
  net::{TcpListener, TcpStream},
  process::Command,
  time::{sleep, timeout},
};

/// How long the test waits for the daemon before it fails.
const PATIENCE: Duration = Duration::from_secs(30);

/// A port of loopback that no listener holds now.
async fn free() -> u16 {
  let listener =
    TcpListener::bind("127.0.0.1:0").await.unwrap_or_else(|no| panic!("no port: {no}"));
  listener.local_addr().unwrap_or_else(|no| panic!("no address: {no}")).port()
}

/// The file `host.json` that turns on the tailnet on a port and gives each operator the right to
/// write.
fn listed(path: &Path, tcp: u16, operators: &[&str]) {
  let roles: serde_json::Map<String, serde_json::Value> =
    operators.iter().map(|one| ((*one).to_owned(), json!("write"))).collect();
  let written = json!({ "tailnet": { "tcp": tcp, "websocket": null }, "roles": roles });
  fs::write(path, written.to_string()).unwrap_or_else(|no| panic!("host.json: {no}"));
}

/// The role that the daemon welcomes a peer of the tailnet with, or none when it does not welcome
/// it, once its port listens.
async fn welcomed(tcp: u16) -> Option<(String, Role)> {
  let stream = loop {
    if let Ok(stream) = TcpStream::connect(("127.0.0.1", tcp)).await {
      break stream;
    }
    sleep(Duration::from_millis(20)).await;
  };
  let mut lines = BufReader::new(stream);
  let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None });
  let request = serde_json::to_string(&Request { id: 0, verb: hello }).ok()? + "\n";
  lines.get_mut().write_all(request.as_bytes()).await.ok()?;
  let mut line = String::new();
  lines.read_line(&mut line).await.ok()?;
  match serde_json::from_str(&line).ok()? {
    Frame::Reply { answer: Answer::Welcome { peer, .. }, .. } => Some((peer.operator, peer.role)),
    _ => None,
  }
}

#[tokio::test]
async fn the_daemon_takes_each_change_of_host_json_and_keeps_its_list_when_the_file_does_not_parse()
{
  let (api, _) = tailscale("cat").await;
  let LocalApi::Port { port, token } = api else { unreachable!("the fake listens on a port") };
  let yard = Yard::new(format!("fch{}", process::id()));
  let path = yard.join("config/host.json");
  fs::create_dir_all(yard.join("config")).unwrap_or_else(|no| panic!("config: {no}"));
  let tcp = free().await;
  listed(&path, tcp, &["ann"]);
  let mut daemon = Command::new(env!("CARGO_BIN_EXE_furb"));
  daemon.arg("host").env("FURB_TAILSCALE_API", format!("{port}:{token}"));
  daemon.env("XDG_CACHE_HOME", &*yard).env("FURB_CONFIG_DIR", yard.join("config"));
  daemon.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
  let mut daemon = daemon.spawn().unwrap_or_else(|no| panic!("furb host did not start: {no}"));
  let mut said =
    BufReader::new(daemon.stdout.take().unwrap_or_else(|| panic!("no stdout"))).lines();
  let listens = timeout(PATIENCE, said.next_line()).await.ok().and_then(Result::ok).flatten();
  assert!(listens.is_some_and(|line| line.starts_with("furb host listens in")));
  assert_eq!(welcomed(tcp).await, None, "an operator that the list does not name is refused");
  listed(&path, tcp, &["ann", "cat"]);
  let waited = async {
    while welcomed(tcp).await.is_none() {
      sleep(Duration::from_millis(20)).await;
    }
  };
  timeout(PATIENCE, waited).await.unwrap_or_else(|_| panic!("the daemon never took the new list"));
  fs::write(&path, "{ not json").unwrap_or_else(|no| panic!("host.json: {no}"));
  let mut told =
    BufReader::new(daemon.stderr.take().unwrap_or_else(|| panic!("no stderr"))).lines();
  let kept = async {
    while let Ok(Some(line)) = told.next_line().await {
      if line.contains("keeps the access list before the change of the file") {
        return true;
      }
    }
    false
  };
  assert!(timeout(PATIENCE, kept).await.unwrap_or(false), "the daemon says why it keeps the list");
  assert_eq!(
    welcomed(tcp).await,
    Some(("cat".to_owned(), Role::Write)),
    "a file that does not parse keeps the list"
  );
}
