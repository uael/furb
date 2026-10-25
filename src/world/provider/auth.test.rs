use std::{
  io::{Read, Write},
  net::{TcpListener, TcpStream},
  path::PathBuf,
  sync::{Arc, atomic::AtomicBool},
  thread,
  time::Duration,
};

use indexmap::IndexMap;

use super::{
  Catalog as Account, Credential, Effort, Listing, callback_code, directory, now, random, save,
  status,
};
use crate::world::Catalog;

struct Scope(PathBuf);

impl Scope {
  fn profile(account: &str, cached: &str) -> Scope {
    let root =
      Scope(std::env::temp_dir().join(format!("furb-auth-{}", random().expect("random bytes"))));
    let path = directory(Some(&root.0));
    save(
      &path.join("openai-auth.json"),
      &Credential {
        access: "test-access".to_owned(),
        refresh: Some("test-refresh".to_owned()),
        expires: now() + 600_000,
        account: account.to_owned(),
      },
    )
    .expect("credentials");
    save(
      &path.join("openai-auth.models.json"),
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
    std::fs::write(
      self.0.join("config.json"),
      format!(r#"{{"providers":{{"openai-auth":{enabled}}}}}"#),
    )
    .expect("provider settings");
    Catalog::of(&IndexMap::new(), None, None, |_| None)
      .configured(&self.0, Some(&self.0))
      .expect("configured catalog")
  }
}

impl Drop for Scope {
  fn drop(&mut self) {
    let _ = std::fs::remove_dir_all(&self.0);
  }
}

#[test]
fn enabling_subscription_models_uses_the_account_window_and_supported_efforts() {
  let scope = Scope::profile("account", "account");
  assert!(scope.catalog(false).find("openai-auth:account-model").is_none());
  let catalog = scope.catalog(true);
  let model = catalog.find("openai-auth:account-model").expect("account model");
  assert_eq!(model.window, 475_000);
  assert_eq!(model.efforts.iter().map(|one| one.0.as_str()).collect::<Vec<_>>(), ["low", "high"]);
  assert!(model.images);
  assert!(catalog.find("openai:account-model").is_none());
  assert_eq!(status(Some(&scope.0)).state, "connected");
}

#[test]
fn a_model_cache_from_another_account_is_not_offered() {
  let scope = Scope::profile("current", "previous");
  assert!(scope.catalog(true).find("openai-auth:account-model").is_none());
  assert_eq!(status(Some(&scope.0)).state, "disconnected");
}

#[cfg(unix)]
#[test]
fn the_credential_and_its_directory_are_private() {
  use std::os::unix::fs::PermissionsExt;
  let scope = Scope::profile("account", "account");
  let path = directory(Some(&scope.0));
  assert_eq!(
    std::fs::metadata(path.join("openai-auth.json")).expect("credential").permissions().mode()
      & 0o777,
    0o600
  );
  assert_eq!(std::fs::metadata(path).expect("directory").permissions().mode() & 0o777, 0o700);
}

#[test]
fn the_browser_callback_rejects_another_state_and_reads_a_split_request() {
  let listener = TcpListener::bind("127.0.0.1:0").expect("callback port");
  listener.set_nonblocking(true).expect("nonblocking listener");
  let address = listener.local_addr().expect("address");
  let canceled = Arc::new(AtomicBool::new(false));
  let stopping = canceled.clone();
  let callback = thread::spawn(move || callback_code(listener, "expected", &stopping));
  let mut wrong = TcpStream::connect(address).expect("first browser");
  wrong
    .write_all(b"GET /auth/callback?code=wrong&state=another HTTP/1.1\r\n\r\n")
    .expect("wrong state");
  let mut refusal = String::new();
  wrong.read_to_string(&mut refusal).expect("refusal");
  assert!(refusal.starts_with("HTTP/1.1 400 Bad Request"));
  let mut browser = TcpStream::connect(address).expect("browser");
  browser.write_all(b"GET /auth/call").expect("first packet");
  thread::sleep(Duration::from_millis(20));
  browser.write_all(b"back?code=accepted&state=expected HTTP/1.1\r\n\r\n").expect("second packet");
  let mut response = String::new();
  browser.read_to_string(&mut response).expect("response");
  assert!(response.starts_with("HTTP/1.1 200 OK"));
  assert_eq!(callback.join().expect("callback thread").expect("code"), "accepted");
}
