use crate::testing::pause;
use std::{
  io::{Read, Write},
  net::{TcpListener, TcpStream},
  sync::mpsc,
  thread,
  time::Duration,
};

use indexmap::IndexMap;
use tokio::sync::watch;

use super::{
  AuthError, Catalog as Account, Credential, Effort, Listing, SignIn, callback_code, device_code,
  directory, now, publish, random, save, status, subscribe,
};
use crate::testing;
use crate::{ear::reactor, world::Catalog};
use std::fs;

struct Scope(testing::Yard);

impl Scope {
  fn profile(account: &str, cached: &str) -> Self {
    let root =
      Self(testing::Yard::unmade(format!("furb-auth-{}", random().expect("random bytes"))));
    let path = directory(Some(&root.0));
    save(
      &path.join("codex.json"),
      &Credential {
        access: "test-access".to_owned(),
        refresh: Some("test-refresh".to_owned()),
        expires: now() + 600_000,
        account: account.to_owned(),
      },
    )
    .expect("credentials");
    save(
      &path.join("codex.models.json"),
      &Account {
        account: cached.to_owned(),
        fetched: now(),
        models: vec![Listing {
          slug: "account-model".to_owned(),
          context_window: Some(500_000),
          max_context_window: Some(872_000),
          effective_context_window_percent: None,
          supported_reasoning_levels: ["low", "high", "ultra"]
            .map(|effort| Effort { effort: effort.to_owned() })
            .to_vec(),
          input_modalities: vec!["text".to_owned(), "image".to_owned()],
          visibility: "list".to_owned(),
        }],
      },
    )
    .expect("model cache");
    root
  }

  fn catalog(&self, enabled: bool) -> Catalog {
    fs::write(self.0.join("config.json"), format!(r#"{{"providers":{{"codex":{enabled}}}}}"#))
      .expect("provider settings");
    Catalog::of(&IndexMap::new(), None, None, |_| None)
      .configured(&self.0, Some(&self.0))
      .expect("configured catalog")
  }
}

#[test]
fn enabling_subscription_models_uses_the_account_window_and_supported_efforts() {
  let scope = Scope::profile("account", "account");
  assert!(scope.catalog(false).find("codex:account-model").is_none());
  let catalog = scope.catalog(true);
  let model = catalog.find("codex:account-model").expect("account model");
  assert_eq!(model.window, 475_000);
  assert_eq!(model.efforts.iter().map(|one| one.0.as_str()).collect::<Vec<_>>(), ["low", "high"]);
  assert!(model.images);
  fs::write(
    scope.0.join("config.json"),
    r#"{"providers":{"codex":{"model_context_window":1000000}}}"#,
  )
  .expect("window override");
  let catalog = Catalog::of(&IndexMap::new(), None, None, |_| None)
    .configured(&scope.0, Some(&scope.0))
    .expect("configured window");
  assert_eq!(catalog.find("codex:account-model").expect("account model").window, 828_400);

  assert!(catalog.find("openai:account-model").is_none());
  assert_eq!(status(Some(&scope.0)), SignIn::Connected);
}

#[test]
fn a_model_cache_from_another_account_is_not_offered() {
  let scope = Scope::profile("current", "previous");
  assert!(scope.catalog(true).find("codex:account-model").is_none());
  assert_eq!(status(Some(&scope.0)), SignIn::Disconnected);
}

#[cfg(unix)]
#[test]
fn the_credential_and_its_directory_are_private() {
  use std::os::unix::fs::PermissionsExt;
  let scope = Scope::profile("account", "account");
  let path = directory(Some(&scope.0));
  assert_eq!(
    fs::metadata(path.join("codex.json")).expect("credential").permissions().mode() & 0o777,
    0o600
  );
  assert_eq!(fs::metadata(path).expect("directory").permissions().mode() & 0o777, 0o700);
}

#[test]
fn the_browser_callback_rejects_another_state_and_reads_a_split_request() {
  let listener = TcpListener::bind("127.0.0.1:0").expect("callback port");
  listener.set_nonblocking(true).expect("nonblocking listener");
  let address = listener.local_addr().expect("address");
  let (_stop, canceled) = watch::channel(false);
  let (send, callback) = mpsc::channel();
  reactor().spawn(async move { send.send(callback_code(listener, "expected", canceled).await) });
  let mut wrong = TcpStream::connect(address).expect("first browser");
  wrong
    .write_all(b"GET /auth/callback?code=wrong&state=another HTTP/1.1\r\n\r\n")
    .expect("wrong state");
  let mut refusal = String::new();
  wrong.read_to_string(&mut refusal).expect("refusal");
  assert!(refusal.starts_with("HTTP/1.1 400 Bad Request"));
  let mut browser = TcpStream::connect(address).expect("browser");
  browser.write_all(b"GET /auth/call").expect("first packet");
  pause(Duration::from_millis(20));
  browser.write_all(b"back?code=accepted&state=expected HTTP/1.1\r\n\r\n").expect("second packet");
  let mut response = String::new();
  browser.read_to_string(&mut response).expect("response");
  assert!(response.starts_with("HTTP/1.1 200 OK"));
  let code = callback.recv_timeout(Duration::from_secs(10)).expect("the callback ends");
  assert_eq!(code.expect("code"), "accepted");
}

#[test]
fn a_sign_in_shows_the_words_that_the_world_shows() {
  let url = "https://auth.openai.com/oauth/authorize".to_owned();
  let states = [
    SignIn::Disconnected,
    SignIn::Waiting { url: url.clone(), code: None },
    SignIn::Connecting,
    SignIn::Connected,
    SignIn::NeedsLogin("again".to_owned()),
    SignIn::Failed("no".to_owned()),
    SignIn::Canceled,
  ];
  assert_eq!(
    states.iter().map(ToString::to_string).collect::<Vec<_>>(),
    ["disconnected", "waiting", "connecting", "connected", "needs-login", "failed", "canceled"]
  );
  assert_eq!(states[1].url(), Some(url.as_str()));
  assert_eq!(states[4].message(), Some("again"));
  assert_eq!(states[5].message(), Some("no"));
  assert_eq!(states[3].message(), None);
}

#[test]
fn a_refusal_asks_for_a_new_sign_in_only_when_the_account_is_at_fault() {
  let refused = AuthError::Rejected { status: 401 };
  assert!(refused.login());
  assert_eq!(refused.to_string(), "OpenAI sign-in failed (HTTP 401). Sign in again.");
  let down = AuthError::Rejected { status: 503 };
  assert!(!down.login());
  assert_eq!(down.to_string(), "OpenAI sign-in failed (HTTP 503). Try again.");
  assert!(AuthError::ListRefused { status: 403 }.login());
  assert!(!AuthError::ListRefused { status: 500 }.login());
}

#[test]
fn a_subscriber_sees_each_change_of_a_sign_in_in_order_and_a_late_one_sees_the_current_state() {
  let scope = Scope::profile("account", "another");
  let path = directory(Some(&scope.0));
  let mut seen = subscribe(Some(&scope.0));
  assert_eq!(*seen.borrow_and_update(), SignIn::Disconnected, "the state at once");
  let waiting = SignIn::Waiting { url: "https://auth.example/login".to_owned(), code: None };
  publish(&path, waiting.clone());
  assert!(seen.has_changed().expect("the sign-in stands"));
  assert_eq!(*seen.borrow_and_update(), waiting);
  publish(&path, SignIn::Connecting);
  assert_eq!(*seen.borrow_and_update(), SignIn::Connecting);
  super::cancel(Some(&scope.0));
  assert!(seen.has_changed().expect("the sign-in stands"));
  assert_eq!(*seen.borrow_and_update(), SignIn::Canceled);
  publish(&path, SignIn::Failed("no network".to_owned()));
  assert_eq!(*seen.borrow_and_update(), SignIn::Failed("no network".to_owned()));
  super::needs_login(&path);
  let again = SignIn::NeedsLogin(super::AGAIN.to_owned());
  assert_eq!(*seen.borrow_and_update(), again);
  assert!(!seen.has_changed().expect("the sign-in stands"), "no change is left unseen");
  let late = subscribe(Some(&scope.0));
  assert_eq!(*late.borrow(), again, "a late subscriber sees the current state at once");
  assert_eq!(
    status(Some(&scope.0)),
    again,
    "the state that status reads is the one that the watch holds"
  );
}

/// One request of a server of a test: its line and its body, read up to the length that its head
/// names.
fn request(stream: &mut TcpStream) -> (String, String) {
  let mut bytes = Vec::new();
  let mut chunk = [0; 4096];
  loop {
    let read = stream.read(&mut chunk).expect("a request");
    bytes.extend_from_slice(&chunk[..read]);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if let Some((head, body)) = text.split_once("\r\n\r\n") {
      let length = head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().ok())?
      });
      if read == 0 || body.len() >= length.unwrap_or(0) {
        return (head.lines().next().unwrap_or_default().to_owned(), body.to_owned());
      }
    }
  }
}

#[test]
fn a_sign_in_from_a_device_shows_its_code_and_asks_its_end_until_the_person_entered_it() {
  let scope = Scope::profile("account", "account");
  let path = directory(Some(&scope.0));
  let issuer = TcpListener::bind("127.0.0.1:0").expect("a port");
  let address = format!("http://{}", issuer.local_addr().expect("address"));
  let answers = [
    ("200 OK", r#"{"device_auth_id":"d1","usercode":"ABCD-1234","interval":"0"}"#),
    ("403 Forbidden", r#"{"error":"authorization_pending"}"#),
    ("200 OK", r#"{"authorization_code":"c1","code_challenge":"h","code_verifier":"v1"}"#),
  ];
  let served = thread::spawn(move || {
    let mut heard = Vec::new();
    for (status, body) in answers {
      let (mut stream, _) = issuer.accept().expect("a request");
      heard.push(request(&mut stream));
      let answer = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
      );
      stream.write_all(answer.as_bytes()).expect("an answer");
    }
    heard
  });
  let mut seen = subscribe(Some(&scope.0));
  let (_stop, canceled) = watch::channel(false);
  let (send, ended) = mpsc::channel();
  let (base, at) = (address.clone(), path);
  reactor().spawn(async move { send.send(device_code(&base, &at, &canceled).await) });
  let authorized = ended.recv_timeout(Duration::from_secs(10)).expect("the sign-in ends");
  let authorized = authorized.expect("the person entered the code");
  assert_eq!(
    (authorized.authorization_code.as_str(), authorized.code_verifier.as_str()),
    ("c1", "v1")
  );
  let page = format!("{address}/codex/device");
  let waiting = SignIn::Waiting { url: page, code: Some("ABCD-1234".to_owned()) };
  assert_eq!(*seen.borrow_and_update(), waiting, "the state shows the page and the code");
  assert_eq!(waiting.code(), Some("ABCD-1234"));
  let asked = r#"{"device_auth_id":"d1","user_code":"ABCD-1234"}"#;
  let heard = served.join().expect("the server ends");
  let heard: Vec<_> = heard.iter().map(|(line, body)| (line.as_str(), body.as_str())).collect();
  assert_eq!(
    heard,
    [
      (
        "POST /api/accounts/deviceauth/usercode HTTP/1.1",
        r#"{"client_id":"app_EMoamEEZ73f0CkXaXp7hrann"}"#
      ),
      ("POST /api/accounts/deviceauth/token HTTP/1.1", asked),
      ("POST /api/accounts/deviceauth/token HTTP/1.1", asked),
    ]
  );
}
