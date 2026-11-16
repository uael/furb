//! A host that stands in for `OpenAI` in a capture, which the daemon of the window asks through
//! `FURB_OPENAI_URL`: it serves the sign-in of Codex from a device and the models of its account,
//! so a walk signs in as a new user does, with no network. The code of the sign-in holds once the
//! browser of the walk opened its page, as a person who enters the code there.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
  task::JoinHandle,
};

use crate::error::Error;

/// The code that the person enters on the page of the sign-in.
const CODE: &str = "FURB-7Q2K";

/// The access token of the account, whose account is a claim of it, which is no signed token here.
const ACCESS: &str =
  "e30.eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjdCJ9fQ.s";

/// The models of the account.
const MODELS: &str = r#"{"models":[{"slug":"gpt-5.5","context_window":400000,"visibility":"list","supported_reasoning_levels":[{"effort":"medium"},{"effort":"high"}],"input_modalities":["text","image"]}]}"#;

/// The host while it serves, at its address; it stops when it drops.
pub(crate) struct OpenAi {
  pub(crate) url: String,
  serves: JoinHandle<()>,
}

impl Drop for OpenAi {
  fn drop(&mut self) {
    self.serves.abort();
  }
}

/// The host, which serves each request on a port of its own.
pub(crate) async fn served() -> Result<OpenAi, Error> {
  let listener = TcpListener::bind("127.0.0.1:0")
    .await
    .map_err(|source| Error::Process { what: "the stand-in of OpenAI", source })?;
  let url = listener
    .local_addr()
    .map(|address| format!("http://{address}"))
    .map_err(|source| Error::Process { what: "the stand-in of OpenAI", source })?;
  let entered = Arc::new(AtomicBool::new(false));
  let serves = tokio::spawn(async move {
    while let Ok((stream, _)) = listener.accept().await {
      tokio::spawn(answered(stream, Arc::clone(&entered)));
    }
  });
  Ok(OpenAi { url, serves })
}

/// One request answered, as the host of `OpenAI` answers it.
async fn answered(mut stream: TcpStream, entered: Arc<AtomicBool>) {
  let Some(line) = request(&mut stream).await else { return };
  let path = line.split(' ').nth(1).unwrap_or_default();
  let path = path.split('?').next().unwrap_or_default();
  let (status, body) = match path {
    "/api/accounts/deviceauth/usercode" => {
      (200, format!(r#"{{"device_auth_id":"device","user_code":"{CODE}","interval":"1"}}"#))
    }
    "/codex/device" => {
      entered.store(true, Ordering::SeqCst);
      (200, "{}".to_owned())
    }
    "/api/accounts/deviceauth/token" if entered.load(Ordering::SeqCst) => (
      200,
      r#"{"authorization_code":"code","code_challenge":"challenge","code_verifier":"verifier"}"#
        .to_owned(),
    ),
    // The code waits for the person.
    "/api/accounts/deviceauth/token" => (403, "{}".to_owned()),
    "/oauth/token" => {
      (200, format!(r#"{{"access_token":"{ACCESS}","refresh_token":"r","expires_in":3600}}"#))
    }
    "/backend-api/codex/models" => (200, MODELS.to_owned()),
    _ => (404, "{}".to_owned()),
  };
  let reason = match status {
    200 => "OK",
    403 => "Forbidden",
    _ => "Not Found",
  };
  let head = format!(
    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
    body.len()
  );
  stream.write_all(format!("{head}{body}").as_bytes()).await.ok();
}

/// The first line of a request, once its head and its body came.
async fn request(stream: &mut TcpStream) -> Option<String> {
  let mut read = Vec::new();
  let mut buffer = [0; 4096];
  loop {
    let count = stream.read(&mut buffer).await.ok().filter(|count| *count > 0)?;
    read.extend_from_slice(buffer.get(..count).unwrap_or_default());
    let text = String::from_utf8_lossy(&read);
    let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };
    let length = head
      .lines()
      .filter_map(|one| one.split_once(':'))
      .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
      .and_then(|(_, value)| value.trim().parse::<usize>().ok())
      .unwrap_or_default();
    if body.len() >= length {
      return head.lines().next().map(str::to_owned);
    }
  }
}
