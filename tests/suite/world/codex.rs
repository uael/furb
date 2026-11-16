//! The models of a `ChatGPT` subscription and their sign-in, through a life and the sign-in that
//! `furb::world::codex` gives a host. Each test of the network runs again as a child of itself, as
//! a user starts a host whose `FURB_OPENAI_URL` names a host that stands in for `OpenAI`, which a
//! thread of the parent serves on a port of this machine, keeping each request that it read.

use std::{
  collections::HashMap,
  env,
  fs::{self, File},
  io::{Read, Write},
  net::TcpStream,
  path::{Path, PathBuf},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use furb::{
  life::Opening,
  world::codex::{self, SignIn},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::{Url, form_urlencoded};

use super::{
  catalog::{opened, replied, served},
  provider::{asked, yard},
};
use crate::{
  child::{child, here, ran},
  drive::park,
  yard::Yard,
};

/// The account of each credential of a test.
const ACCOUNT: &str = "acct";

/// The client of furb at the issuer of `OpenAI`.
const CLIENT: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

/// An access token whose account is a claim of it, which is no signed token here.
fn access() -> String {
  let claims = json!({"https://api.openai.com/auth": {"chatgpt_account_id": ACCOUNT}});
  format!("e30.{}.sig", URL_SAFE_NO_PAD.encode(claims.to_string()))
}

/// Now, in milliseconds since the epoch, as a credential and a list of models count time.
fn now() -> u64 {
  let since = SystemTime::now().duration_since(UNIX_EPOCH).expect("a time after the epoch");
  u64::try_from(since.as_millis()).expect("a time in a u64")
}

/// A yard whose folder of configs holds a credential of codex that ends at this time, with a
/// refresh, and a new list of the models of an account, and whose project holds these settings of
/// codex.
fn signed(name: &str, expires: u64, listed: &str, settings: &Value) -> Yard {
  let at = yard(name);
  let auth = at.join("config").join("auth");
  fs::create_dir_all(&auth).expect("the folder of the sign-in");
  let held = json!({"access": access(), "refresh": "r", "expires": expires, "account": ACCOUNT});
  fs::write(auth.join("codex.json"), held.to_string()).expect("the credential");
  let model = json!({
    "slug": "account-model", "context_window": 500_000, "max_context_window": 872_000,
    "supported_reasoning_levels": [{"effort": "low"}, {"effort": "high"}, {"effort": "ultra"}],
    "input_modalities": ["text", "image"], "visibility": "list",
  });
  let models = json!({"account": listed, "fetched": now(), "models": [model]});
  fs::write(auth.join("codex.models.json"), models.to_string()).expect("the list of models");
  project(&at, settings);
  at
}

/// The settings of codex in the config of the project of a yard.
fn project(at: &Path, settings: &Value) {
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  let config = json!({"providers": {"codex": settings}}).to_string();
  fs::write(at.join(".furb").join("config.json"), config).expect("the config of the project");
}

/// The folder of configs of the child.
fn config(at: &Path) -> PathBuf {
  at.join("config")
}

/// The test run again as a child in the yard, whose credential of the codex command line is none
/// of this machine, with the host that stands in for `OpenAI` on this port when one serves.
fn child_of(test: &str, at: &Path, port: Option<u16>) {
  let home = at.join("codex-home").display().to_string();
  let url = port.map(|port| format!("http://127.0.0.1:{port}"));
  let mut vars = vec![("CODEX_HOME", home.as_str())];
  vars.extend(url.as_deref().map(|url| ("FURB_OPENAI_URL", url)));
  ran(test, at, &vars);
}

/// The address of the host that stands in for `OpenAI`, as the child reads it.
fn stand_in() -> String {
  env::var("FURB_OPENAI_URL").expect("a host stands in for OpenAI")
}

/// An answer of JSON with this status.
fn answer(status: &str, body: &Value) -> String {
  let body = body.to_string();
  format!(
    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: \
     close\r\n\r\n{body}",
    body.len()
  )
}

/// The answer of the issuer that gives a credential of the account.
fn token() -> String {
  answer("200 OK", &json!({"access_token": access(), "refresh_token": "r2", "expires_in": 3600}))
}

/// The answer of the API of Codex that lists one model of the account.
fn models() -> String {
  let model = json!({"slug": "gpt-test", "context_window": 200_000, "visibility": "list"});
  answer("200 OK", &json!({"models": [model]}))
}

/// Each state of the sign-in of a folder that a subscriber sees from now, until one that no
/// sign-in runs toward, which comes within ten seconds.
fn settled(config: &Path) -> Vec<SignIn> {
  let mut seen = codex::subscribe(Some(config));
  let mut states = vec![seen.borrow_and_update().clone()];
  let start = Instant::now();
  loop {
    let state = seen.borrow_and_update().clone();
    if states.last() != Some(&state) {
      states.push(state.clone());
    }
    if !matches!(state, SignIn::Connecting | SignIn::Waiting { .. }) {
      return states;
    }
    assert!(start.elapsed() < Duration::from_secs(10), "the sign-in ends: {states:?}");
    park(Duration::from_millis(5));
  }
}

/// The first line and the body of each request that the host read.
fn lines(requests: &[String]) -> Vec<(&str, &str)> {
  requests
    .iter()
    .map(|one| {
      let (head, body) = one.split_once("\r\n\r\n").expect("a head and a body");
      (head.lines().next().expect("a request line"), body)
    })
    .collect()
}

#[test]
fn a_sign_in_shows_the_words_that_the_world_shows() {
  let url = "https://auth.openai.com/oauth/authorize".to_owned();
  let states = [
    SignIn::Disconnected,
    SignIn::Waiting { url: url.clone(), code: Some("ABCD".to_owned()) },
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
  assert_eq!((states[1].url(), states[1].code()), (Some(url.as_str()), Some("ABCD")));
  assert_eq!(states[4].message(), Some("again"));
  assert_eq!(states[5].message(), Some("no"));
  assert_eq!(states[3].message(), None);
}

#[test]
fn an_account_offers_its_models_with_their_window_and_efforts_once_a_config_turns_codex_on() {
  if let Some(at) = child() {
    let offered = |name: &str| {
      let catalog = opened(&at).catalog().expect("the catalog");
      catalog.find(name).map(|one| (one.window(), one.efforts().join(" "), one.sees()))
    };
    assert_eq!(offered("codex:account-model"), None, "codex is off");
    project(&at, &json!(true));
    let held = Some((475_000, "low high".to_owned(), true));
    assert_eq!(offered("codex:account-model"), held, "a share of 95 of the window of the account");
    assert_eq!(offered("openai:account-model"), None);
    project(&at, &json!({"model_context_window": 1_000_000}));
    let wider = offered("codex:account-model").map(|one| one.0);
    assert_eq!(wider, Some(828_400), "a window of the config under the most of the account");
    assert_eq!(codex::status(Some(&config(&at))), SignIn::Connected);
    return;
  }
  let at = signed("codex-offered", u64::MAX / 2, ACCOUNT, &json!(false));
  child_of(here!(), &at, None);
}

#[test]
fn a_list_of_the_models_of_another_account_is_not_offered_and_the_sign_in_reads_disconnected() {
  if let Some(at) = child() {
    let catalog = opened(&at).catalog().expect("the catalog");
    assert!(catalog.find("codex:account-model").is_none());
    assert_eq!(codex::status(Some(&config(&at))), SignIn::Disconnected);
    return;
  }
  let at = signed("codex-another", u64::MAX / 2, "previous", &json!(true));
  child_of(here!(), &at, None);
}

#[test]
fn a_config_of_codex_that_does_not_hold_refuses_the_catalog_with_its_own_cause() {
  if let Some(at) = child() {
    let refused = |settings: Value| {
      project(&at, &settings);
      opened(&at).catalog().err().map(|fault| fault.object().py_repr())
    };
    let zero = refused(json!({"model_context_window": 0})).expect("a window of zero is refused");
    assert!(zero.contains("The codex model_context_window must be positive"), "{zero}");
    let unknown = refused(json!({"window": 1})).expect("an unknown setting is refused");
    assert!(unknown.contains("Invalid codex settings: unknown field `window`"), "{unknown}");
    return;
  }
  let at = signed("codex-settings", u64::MAX / 2, ACCOUNT, &json!(true));
  child_of(here!(), &at, None);
}

#[test]
fn a_sign_in_from_a_device_shows_its_code_asks_its_end_until_the_person_entered_it_and_keeps_a_private_credential()
 {
  if let Some(at) = child() {
    let folder = config(&at);
    assert_eq!(codex::begin(Some(&folder), true).expect("the sign-in starts"), SignIn::Connecting);
    let states = settled(&folder);
    let page = format!("{}/codex/device", stand_in());
    let waiting = SignIn::Waiting { url: page, code: Some("ABCD-1234".to_owned()) };
    let shown: Vec<_> = states.into_iter().filter(|one| *one != SignIn::Connecting).collect();
    assert_eq!(shown, [waiting, SignIn::Connected]);
    assert_eq!(*codex::subscribe(Some(&folder)).borrow(), SignIn::Connected, "a late subscriber");
    assert_eq!(codex::status(Some(&folder)), SignIn::Connected);
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mode = |path: &Path| fs::metadata(path).expect("it stands").permissions().mode() & 0o777;
      let auth = folder.join("auth");
      assert_eq!((mode(&auth), mode(&auth.join("codex.json"))), (0o700, 0o600));
    }
    return;
  }
  let at = yard("codex-device");
  let given = json!({"device_auth_id": "d1", "usercode": "ABCD-1234", "interval": "0"});
  let entered = json!({"authorization_code": "c1", "code_challenge": "h", "code_verifier": "v1"});
  let (port, heard) = served(vec![
    answer("200 OK", &given),
    answer("403 Forbidden", &json!({"error": "authorization_pending"})),
    answer("200 OK", &entered),
    token(),
    models(),
  ]);
  child_of(here!(), &at, Some(port));
  let heard = heard.join().expect("the host ends");
  let asked = r#"{"device_auth_id":"d1","user_code":"ABCD-1234"}"#;
  let redirect = format!("http%3A%2F%2F127.0.0.1%3A{port}%2Fdeviceauth%2Fcallback");
  let exchanged = format!(
    "grant_type=authorization_code&client_id={CLIENT}&code=c1&code_verifier=v1&redirect_uri={redirect}"
  );
  assert_eq!(
    lines(&heard),
    [
      (
        "POST /api/accounts/deviceauth/usercode HTTP/1.1",
        format!(r#"{{"client_id":"{CLIENT}"}}"#).as_str()
      ),
      ("POST /api/accounts/deviceauth/token HTTP/1.1", asked),
      ("POST /api/accounts/deviceauth/token HTTP/1.1", asked),
      ("POST /oauth/token HTTP/1.1", exchanged.as_str()),
      ("GET /backend-api/codex/models?client_version=0.163.0 HTTP/1.1", ""),
    ]
  );
}

/// The lock of the port of the browser callback of a sign-in, which is one for the machine: each
/// run of the tests of any tree takes the same lock of a file before it binds the port.
fn port() -> File {
  let lock = File::create(env::temp_dir().join("furb-sign-in-port.lock")).expect("the lock");
  let start = Instant::now();
  while lock.try_lock().is_err() {
    assert!(start.elapsed() < Duration::from_secs(60), "another test holds the port of a sign-in");
    park(Duration::from_millis(50));
  }
  lock
}

/// What a browser hears at the callback of a sign-in for a request that it writes in these parts.
fn browsed(parts: &[&[u8]]) -> String {
  let mut browser = TcpStream::connect(("127.0.0.1", 1455)).expect("the callback listens");
  for part in parts {
    browser.write_all(part).expect("a part of the request");
    park(Duration::from_millis(20));
  }
  let mut heard = String::new();
  browser.read_to_string(&mut heard).expect("the answer");
  heard
}

#[test]
fn a_sign_in_in_the_browser_refuses_a_callback_of_another_state_and_reads_one_that_comes_in_parts()
{
  if let Some(at) = child() {
    let _port = port();
    let folder = config(&at);
    let Ok(SignIn::Waiting { url, code: None }) = codex::begin(Some(&folder), false) else {
      panic!("the sign-in waits for the browser")
    };
    let url = Url::parse(&url).expect("an address");
    let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(
      url.as_str().split('?').next(),
      Some(format!("{}/oauth/authorize", stand_in()).as_str())
    );
    assert_eq!(pairs["redirect_uri"], "http://localhost:1455/auth/callback");
    assert_eq!(
      (pairs["client_id"].as_str(), pairs["code_challenge_method"].as_str()),
      (CLIENT, "S256")
    );
    let wrong = browsed(&[b"GET /auth/callback?code=wrong&state=another HTTP/1.1\r\n\r\n"]);
    assert!(wrong.starts_with("HTTP/1.1 400 Bad Request"), "{wrong}");
    let state = format!("back?code=accepted&state={} HTTP/1.1\r\n\r\n", pairs["state"]);
    let right = browsed(&[b"GET /auth/call", state.as_bytes()]);
    assert!(right.starts_with("HTTP/1.1 200 OK"), "{right}");
    let states = settled(&folder);
    assert_eq!(states.last(), Some(&SignIn::Connected), "{states:?}");
    // The parent reads the verifier of the exchange against this challenge.
    fs::write(at.join("challenge"), &pairs["code_challenge"]).expect("the challenge");
    return;
  }
  let at = yard("codex-browser");
  let (port, heard) = served(vec![token(), models()]);
  child_of(here!(), &at, Some(port));
  let heard = heard.join().expect("the host ends");
  let (line, body) = lines(&heard)[0];
  assert_eq!(line, "POST /oauth/token HTTP/1.1");
  let form: HashMap<_, _> = form_urlencoded::parse(body.as_bytes()).into_owned().collect();
  assert_eq!(
    (form["code"].as_str(), form["redirect_uri"].as_str()),
    ("accepted", "http://localhost:1455/auth/callback")
  );
  let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes()));
  assert_eq!(fs::read_to_string(at.join("challenge")).expect("the challenge"), challenge);
}

#[test]
fn a_refusal_asks_for_a_new_sign_in_only_when_the_account_is_at_fault() {
  if let Some(at) = child() {
    let mut ends = Vec::new();
    for case in ["refused", "down", "unlisted", "broken"] {
      let folder = at.join(case);
      assert_eq!(codex::begin(Some(&folder), false).expect("a renewal"), SignIn::Connecting);
      ends.push(settled(&folder).pop().expect("an end"));
    }
    let again = |text: &str| SignIn::NeedsLogin(text.to_owned());
    let failed = |text: &str| SignIn::Failed(text.to_owned());
    assert_eq!(
      ends,
      [
        again("OpenAI sign-in failed (HTTP 401). Sign in again."),
        failed("OpenAI sign-in failed (HTTP 503). Try again."),
        again("OpenAI could not list this account's models (HTTP 403)."),
        failed("OpenAI could not list this account's models (HTTP 500)."),
      ]
    );
    return;
  }
  let at = yard("codex-refusals");
  // Each credential ended and holds a refresh, so each sign-in renews it first.
  for case in ["refused", "down", "unlisted", "broken"] {
    let auth = at.join(case).join("auth");
    fs::create_dir_all(&auth).expect("the folder of the sign-in");
    let held = json!({"access": access(), "refresh": "r", "expires": 0, "account": ACCOUNT});
    fs::write(auth.join("codex.json"), held.to_string()).expect("the credential");
  }
  let no = json!({"error": "no"});
  let (port, heard) = served(vec![
    answer("401 Unauthorized", &no),
    answer("503 Service Unavailable", &no),
    token(),
    answer("403 Forbidden", &no),
    token(),
    answer("500 Internal Server Error", &no),
  ]);
  child_of(here!(), &at, Some(port));
  assert_eq!(heard.join().expect("the host ends").len(), 6);
}

/// A stream of the Responses API that writes this word, with no type of content in its head.
fn untyped(word: &str) -> String {
  let delta = json!({"type": "response.output_text.delta", "item_id": "m1", "output_index": 0,
    "content_index": 0, "sequence_number": 1, "delta": word});
  let response = json!({"id": "r1", "object": "response", "created_at": 0, "status": "completed",
    "error": null, "incomplete_details": null, "instructions": null, "max_output_tokens": null,
    "model": "gpt-test", "usage": null, "output": [], "tools": []});
  let completed = json!({"type": "response.completed", "sequence_number": 2, "response": response});
  let body = format!("data: {delta}\n\ndata: {completed}\n\n");
  format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len())
}

#[test]
fn a_turn_of_codex_streams_with_no_type_of_content_and_a_refusal_of_its_credential_asks_for_a_new_sign_in()
 {
  if let Some(at) = child() {
    let turn = || {
      let opening = Opening { actor: Some("codex:account-model".to_owned()), ..opened(&at) };
      let mut life = opening.boot::<String>([]).expect("the life opens").0;
      asked(&mut life);
      replied(&mut life)
    };
    let streamed = turn();
    assert!(streamed.starts_with("('assistant', 'close(3)'"), "{streamed}");
    let refused = turn();
    assert!(refused.contains("Sign in to codex again from Providers."), "{refused}");
    let again = SignIn::NeedsLogin("Sign in to Codex again from Providers.".to_owned());
    assert_eq!(codex::status(Some(&config(&at))), again);
    let catalog = opened(&at).catalog().expect("the catalog");
    assert!(
      catalog.find("codex:account-model").is_none(),
      "an account that must sign in offers none"
    );
    return;
  }
  let at = signed("codex-turn", u64::MAX / 2, ACCOUNT, &json!(true));
  let refused = answer("401 Unauthorized", &json!({"error": "expired"}));
  let (port, heard) = served(vec![untyped("close(3)"), refused]);
  child_of(here!(), &at, Some(port));
  let heard = heard.join().expect("the host ends");
  assert!(lines(&heard).iter().all(|one| one.0 == "POST /backend-api/codex/responses HTTP/1.1"));
}
