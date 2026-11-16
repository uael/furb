//! A fake of the local API of Tailscale on a port of loopback, which gives one login for each
//! address, names other machines of the tailnet online on loopback, and says the path of each
//! request that it heard. The suites of the host and of the
//! command line share it by a path, as `common/yard.rs`.

#![allow(dead_code, reason = "a suite takes the part of the fake that it needs")]

use furb_host::transport::LocalApi;
use serde_json::json;
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
  net::TcpListener,
  sync::mpsc,
};

/// The fake on a port of loopback, for a login, and the paths of the requests that it heard.
pub(crate) async fn tailscale(operator: &str) -> (LocalApi, mpsc::UnboundedReceiver<String>) {
  tailnet(operator, &[]).await
}

/// The fake for a login on a tailnet whose other machines, each by its name, stand online at the
/// address of loopback, and the paths of the requests that it heard.
pub(crate) async fn tailnet(
  operator: &str,
  machines: &[&str],
) -> (LocalApi, mpsc::UnboundedReceiver<String>) {
  let peers: serde_json::Map<String, serde_json::Value> = machines
    .iter()
    .map(|name| {
      let node = json!({
        "DNSName": format!("{name}.tail.ts.net."),
        "HostName": name,
        "TailscaleIPs": ["127.0.0.1"],
        "Online": true,
      });
      (format!("nodekey:{name}"), node)
    })
    .collect();
  let listener =
    TcpListener::bind("127.0.0.1:0").await.unwrap_or_else(|no| panic!("no port: {no}"));
  let port = listener.local_addr().unwrap_or_else(|no| panic!("no address: {no}")).port();
  let (tell, heard) = mpsc::unbounded_channel();
  let operator = operator.to_owned();
  tokio::spawn(async move {
    while let Ok((mut stream, _)) = listener.accept().await {
      let (read, mut write) = stream.split();
      let mut lines = BufReader::new(read);
      let mut first = String::new();
      if lines.read_line(&mut first).await.is_err() {
        continue;
      }
      let mut header = String::from("-");
      while header.trim() != "" {
        header.clear();
        if lines.read_line(&mut header).await.is_err() {
          break;
        }
      }
      let path = first.split(' ').nth(1).unwrap_or_default().to_owned();
      let body = if path.starts_with("/localapi/v0/status") {
        json!({"TailscaleIPs": ["127.0.0.1"], "Peer": peers})
      } else {
        json!({"UserProfile": {"ID": 1, "LoginName": operator}})
      };
      let body = body.to_string();
      let answer = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
       Connection: close\r\n\r\n{body}",
        body.len()
      );
      write.write_all(answer.as_bytes()).await.ok();
      // A test that heard what it needed drops its receiver.
      tell.send(path).ok();
    }
  });
  (LocalApi::Port { port, token: "proof".to_owned() }, heard)
}
