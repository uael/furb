//! `OpenAI` models on a `ChatGPT` subscription, with credentials kept outside the project.

use std::{
  collections::HashMap,
  fmt,
  fs::{self, File},
  net::TcpListener,
  path::{Path, PathBuf},
  sync::{Mutex, MutexGuard, OnceLock, PoisonError},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rig_core::http_client::ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  sync::watch,
};

use crate::{
  ear::runtime,
  skill::Places,
  world::disk::{self, Access},
};
use serde::de;
use std::env;
use std::io;
use tokio::net;
use tokio::task;
use tokio::time;

/// The provider of models paid for by a `ChatGPT` subscription.
pub const NAME: &str = "codex";
const API: &str = "https://chatgpt.com/backend-api/codex";
const AUTH: &str = "https://auth.openai.com";
/// The address of a host that stands in for `OpenAI`, as one that a capture of the window serves:
/// the sign-in at its root and the API of Codex under `/backend-api/codex`.
const STAND_IN: &str = "FURB_OPENAI_URL";
const CLIENT: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const REDIRECT: &str = "http://localhost:1455/auth/callback";
/// The page where a person enters the code of a sign-in from a device, and the callback that the
/// code of such a sign-in names, as the codex command line names them.
const DEVICE: &str = "/codex/device";
const DEVICE_REDIRECT: &str = "/deviceauth/callback";
/// How long a code of a sign-in from a device holds.
const DEVICE_WAIT: Duration = Duration::from_mins(15);
const CLIENT_VERSION: &str = "0.163.0";
const AGAIN: &str = "Sign in to Codex again from Providers.";

/// The state of a sign-in. It shows as the word that the World shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SignIn {
  #[default]
  Disconnected,
  /// The browser address to open while the sign-in waits for its callback, and the code to enter
  /// there for a sign-in from a device.
  Waiting {
    url: String,
    code: Option<String>,
  },
  Connecting,
  Connected,
  NeedsLogin(String),
  Failed(String),
  Canceled,
}

impl SignIn {
  /// The word of the state, as the World shows it.
  pub fn state(&self) -> &'static str {
    match self {
      Self::Disconnected => "disconnected",
      Self::Waiting { .. } => "waiting",
      Self::Connecting => "connecting",
      Self::Connected => "connected",
      Self::NeedsLogin(_) => "needs-login",
      Self::Failed(_) => "failed",
      Self::Canceled => "canceled",
    }
  }

  /// The browser address, while the sign-in waits.
  pub fn url(&self) -> Option<&str> {
    match self {
      Self::Waiting { url, .. } => Some(url),
      _ => None,
    }
  }

  /// The code to enter on the page of the address, while a sign-in from a device waits.
  pub fn code(&self) -> Option<&str> {
    match self {
      Self::Waiting { code, .. } => code.as_deref(),
      _ => None,
    }
  }

  /// What to tell the operator when the sign-in needs him or failed.
  pub fn message(&self) -> Option<&str> {
    match self {
      Self::NeedsLogin(message) | Self::Failed(message) => Some(message),
      _ => None,
    }
  }

  /// Whether a browser sign-in or a connection runs now.
  fn running(&self) -> bool {
    matches!(self, Self::Waiting { .. } | Self::Connecting)
  }
}

impl fmt::Display for SignIn {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(self.state())
  }
}

/// Why a sign-in, a refresh or a listing of models did not go.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
  #[error("The credential has no directory")]
  NoDirectory,
  #[error("{0}")]
  Io(#[from] io::Error),
  #[error("{0}")]
  Json(#[from] serde_json::Error),
  #[error("{0}")]
  Random(#[from] getrandom::Error),
  #[error("{0}")]
  Write(#[from] disk::Error),
  #[error("{0}")]
  Task(#[from] task::JoinError),
  #[error("{0}")]
  Url(#[from] url::ParseError),
  #[error("{0}")]
  Request(#[from] reqwest::Error),
  #[error("Sign in to codex from Providers")]
  SignInNeeded,
  #[error("Sign in to codex again")]
  SignInAgain,
  #[error("OpenAI did not return a ChatGPT account")]
  NoAccount,
  #[error("OpenAI did not return a refresh credential. Sign in again.")]
  NoRefresh,
  #[error("OpenAI sign-in failed (HTTP {status}). {}", advice(*.status))]
  Rejected { status: u16 },
  #[error("OpenAI returned an invalid sign-in response")]
  InvalidToken(#[source] serde_json::Error),
  #[error("OpenAI could not list this account's models (HTTP {status}).")]
  ListRefused { status: u16 },
  #[error("OpenAI returned a model list that is too large")]
  ListTooLarge,
  #[error("OpenAI returned an invalid model list")]
  InvalidList(#[source] serde_json::Error),
  #[error("This ChatGPT account offers no model to furb")]
  NoModels,
  #[error("Another OpenAI sign-in uses port 1455. Finish it, then try again.")]
  PortBusy(#[source] io::Error),
  #[error("Sign-in canceled")]
  Canceled,
  #[error("OpenAI sign-in was canceled or timed out")]
  Ended,
  #[error("OpenAI sign-in was refused")]
  Declined,
  #[error(
    "OpenAI offers no sign-in with a code here. Sign in from a browser on the machine of furb."
  )]
  NoDevice,
}

fn advice(status: u16) -> &'static str {
  if matches!(status, 400 | 401 | 403) { "Sign in again." } else { "Try again." }
}

impl AuthError {
  /// Whether the account must sign in again for this to go.
  fn login(&self) -> bool {
    match self {
      Self::Rejected { status } => matches!(status, 400 | 401 | 403),
      Self::ListRefused { status } => matches!(status, 401 | 403),
      _ => false,
    }
  }
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Credential {
  pub access: String,
  refresh: Option<String>,
  expires: u64,
  pub account: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Listing {
  pub slug: String,
  #[serde(default)]
  pub context_window: Option<u64>,
  #[serde(default)]
  pub max_context_window: Option<u64>,
  #[serde(default)]
  pub effective_context_window_percent: Option<u64>,
  #[serde(default)]
  pub supported_reasoning_levels: Vec<Effort>,
  #[serde(default)]
  pub input_modalities: Vec<String>,
  #[serde(default)]
  pub visibility: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Effort {
  pub effort: String,
}

#[derive(Deserialize, Serialize)]
struct Catalog {
  account: String,
  #[serde(default)]
  fetched: u64,
  models: Vec<Listing>,
}

#[derive(Deserialize)]
struct Listed {
  models: Vec<Listing>,
}

/// The sign-in of a folder, with the way to cancel it. It outlives the call that began it: the
/// host asks for the state in later calls, with no object of its own to hold it between them. Its
/// state lives in the watch alone, which each writer sends to and each reader borrows from, so a
/// host that subscribes sees each change at once and polls nothing.
struct Flow {
  state: watch::Sender<SignIn>,
  canceled: watch::Sender<bool>,
}

fn fresh(state: SignIn) -> Flow {
  Flow { state: watch::channel(state).0, canceled: watch::channel(false).0 }
}

/// The new state of the sign-in of a folder, which each subscriber sees.
fn publish(path: &Path, state: SignIn) {
  flows().entry(path.to_owned()).or_insert_with(|| fresh(state.clone())).state.send_replace(state);
}

/// What a sign-in waits with: in the browser of this machine, its port, its state and its
/// verifier; from a device, nothing until `OpenAI` gives its code.
enum Pending {
  Browser { listener: TcpListener, state: String, verifier: String },
  Device,
}

fn now() -> u64 {
  let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
}

pub(super) fn directory(config: Option<&Path>) -> PathBuf {
  config.map_or_else(|| Places::here().config, Path::to_path_buf).join("auth")
}

fn flows() -> MutexGuard<'static, HashMap<PathBuf, Flow>> {
  static FLOWS: OnceLock<Mutex<HashMap<PathBuf, Flow>>> = OnceLock::new();
  FLOWS.get_or_init(Mutex::default).lock().unwrap_or_else(PoisonError::into_inner)
}

fn read<T: de::DeserializeOwned>(path: &Path) -> Option<T> {
  serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn save(path: &Path, value: &impl Serialize) -> Result<(), AuthError> {
  let parent = path.parent().ok_or(AuthError::NoDirectory)?;
  fs::create_dir_all(parent)?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
  }
  disk::write(path, &serde_json::to_vec(value)?, Access::Owner)?;
  Ok(())
}

fn random() -> Result<String, AuthError> {
  let mut bytes = [0; 32];
  getrandom::fill(&mut bytes)?;
  Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn claims(token: &str) -> Option<Value> {
  let payload = token.split('.').nth(1)?;
  serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?).ok()
}

fn account(access: &str) -> Option<String> {
  let claims = claims(access)?;
  claims.get("https://api.openai.com/auth")?.get("chatgpt_account_id")?.as_str().map(str::to_owned)
}

fn borrowed() -> Option<Credential> {
  let home = env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| {
    env::var_os("HOME")
      .or_else(|| env::var_os("USERPROFILE"))
      .map(|one| PathBuf::from(one).join(".codex"))
  })?;
  let value: Value = read(&home.join("auth.json"))?;
  if value.get("auth_mode")?.as_str()? != "chatgpt" {
    return None;
  }
  let access = value.get("tokens")?.get("access_token")?.as_str()?.to_owned();
  let expires = claims(&access)?.get("exp")?.as_u64()?.saturating_mul(1000);
  if expires <= now() + 60_000 {
    return None;
  }
  Some(Credential { account: account(&access)?, access, refresh: None, expires })
}

fn held(config: &Path) -> Option<Credential> {
  read(&config.join("codex.json")).or_else(borrowed)
}

/// The credential that serves now, or that a refresh can renew.
fn usable(path: &Path) -> Option<Credential> {
  held(path).filter(|one| one.expires > now() + 60_000 || one.refresh.is_some())
}

/// The current sign-in state. This view starts no sign-in and makes no request.
pub fn status(config: Option<&Path>) -> SignIn {
  let path = directory(config);
  if let Some(flow) = flows().get(&path) {
    let state = flow.state.borrow().clone();
    if !matches!(state, SignIn::Connected | SignIn::Disconnected) {
      return state;
    }
  }
  let connected = usable(&path).is_some_and(|one| {
    read::<Catalog>(&path.join("codex.models.json"))
      .is_some_and(|catalog| catalog.account == one.account && !catalog.models.is_empty())
  });
  if connected { SignIn::Connected } else { SignIn::Disconnected }
}

/// The state of the sign-in of a folder as it changes: the current state at once, then each change
/// that a writer of the sign-in makes, so a host tells its windows with no poll.
pub fn subscribe(config: Option<&Path>) -> watch::Receiver<SignIn> {
  let current = status(config);
  let mut all = flows();
  let flow = all.entry(directory(config)).or_insert_with(|| fresh(current.clone()));
  // A state that no writer set, as the account that connects or disconnects, is read from the disk.
  flow.state.send_if_modified(|state| {
    let settled = matches!(state, SignIn::Connected | SignIn::Disconnected) && *state != current;
    if settled {
      *state = current;
    }
    settled
  });
  flow.state.subscribe()
}

/// Cancel a sign-in that has not finished. Saved credentials stay available.
pub fn cancel(config: Option<&Path>) {
  let all = flows();
  if let Some(flow) = all.get(&directory(config)).filter(|flow| flow.state.borrow().running()) {
    flow.canceled.send_replace(true);
    flow.state.send_replace(SignIn::Canceled);
  }
}

fn update(path: &Path, canceled: &watch::Receiver<bool>, status: SignIn) {
  if *canceled.borrow() {
    return;
  }
  publish(path, status);
}

pub(super) fn models(config: Option<&Path>) -> Vec<Listing> {
  let path = directory(config);
  if flows().get(&path).is_some_and(|flow| matches!(*flow.state.borrow(), SignIn::NeedsLogin(_))) {
    return Vec::new();
  }
  let Some(credential) = held(&path) else { return Vec::new() };
  read::<Catalog>(&path.join("codex.models.json"))
    .filter(|one| one.account == credential.account)
    .map_or_else(Vec::new, |one| one.models)
}

pub(super) fn needs_login(path: &Path) {
  if flows().get(path).is_some_and(|flow| flow.state.borrow().running()) {
    return;
  }
  publish(path, SignIn::NeedsLogin(AGAIN.to_owned()));
}

pub(super) fn failed(path: &Path, failure: &AuthError) {
  if failure.login() {
    needs_login(path);
  }
}

/// Refresh an account's model list when it is more than five minutes old. This never starts a browser sign-in.
pub fn refresh(config: Option<&Path>) -> Result<SignIn, AuthError> {
  let path = directory(config);
  let status = status(config);
  if status.running() || matches!(status, SignIn::NeedsLogin(_)) {
    return Ok(status);
  }
  let Some(credential) = usable(&path) else { return Ok(SignIn::Disconnected) };
  let fresh = read::<Catalog>(&path.join("codex.models.json")).is_some_and(|one| {
    one.account == credential.account && now().saturating_sub(one.fetched) < 300_000
  });
  if fresh && !matches!(status, SignIn::Failed(_)) {
    return Ok(SignIn::Connected);
  }
  begin(config, false)
}

pub(super) async fn credential(path: &Path) -> Result<Credential, AuthError> {
  let held = held(path).ok_or(AuthError::SignInNeeded)?;
  if held.expires > now() + 60_000 {
    return Ok(held);
  }
  fs::create_dir_all(path)?;
  let path_of_lock = path.join("codex.lock");
  let _lock = task::spawn_blocking(move || -> Result<File, AuthError> {
    let file = File::options().create(true).truncate(false).write(true).open(path_of_lock)?;
    file.lock()?;
    Ok(file)
  })
  .await??;
  let held: Credential = read(&path.join("codex.json")).ok_or(AuthError::SignInAgain)?;
  if held.expires > now() + 60_000 {
    return Ok(held);
  }
  let refresh = held.refresh.ok_or(AuthError::SignInAgain)?;
  let token =
    token(&[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", CLIENT)])
      .await?;
  let renewed = token.credential(Some(refresh))?;
  save(&path.join("codex.json"), &renewed)?;
  Ok(renewed)
}

#[derive(Deserialize)]
struct Token {
  #[serde(rename = "access_token")]
  access: String,
  #[serde(rename = "refresh_token")]
  refresh: Option<String>,
  expires_in: u64,
}

impl Token {
  fn credential(self, refresh: Option<String>) -> Result<Credential, AuthError> {
    let account = account(&self.access).ok_or(AuthError::NoAccount)?;
    let refresh = self.refresh.or(refresh).ok_or(AuthError::NoRefresh)?;
    Ok(Credential {
      access: self.access,
      refresh: Some(refresh),
      expires: now().saturating_add(self.expires_in.saturating_mul(1000)),
      account,
    })
  }
}

async fn token(fields: &[(&str, &str)]) -> Result<Token, AuthError> {
  let body: String = url::form_urlencoded::Serializer::new(String::new())
    .extend_pairs(fields.iter().copied())
    .finish();
  let response = ReqwestClient::new()
    .post(format!("{}/oauth/token", issuer()))
    .timeout(Duration::from_secs(20))
    .header("content-type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await?;
  if !response.status().is_success() {
    return Err(AuthError::Rejected { status: response.status().as_u16() });
  }
  serde_json::from_slice(&response.bytes().await?).map_err(AuthError::InvalidToken)
}

async fn listed(path: &Path, credential: &Credential) -> Result<(), AuthError> {
  let response = ReqwestClient::new()
    .get(format!("{}/models?client_version={CLIENT_VERSION}", api()))
    .bearer_auth(&credential.access)
    .header("chatgpt-account-id", &credential.account)
    .header("originator", "furb")
    .header("user-agent", concat!("furb/", env!("CARGO_PKG_VERSION")))
    .timeout(Duration::from_secs(20))
    .send()
    .await?;
  if !response.status().is_success() {
    return Err(AuthError::ListRefused { status: response.status().as_u16() });
  }
  let bytes = response.bytes().await?;
  if bytes.len() > 1024 * 1024 {
    return Err(AuthError::ListTooLarge);
  }
  let mut listed: Listed = serde_json::from_slice(&bytes).map_err(AuthError::InvalidList)?;
  listed.models.retain(|one| {
    one.visibility == "list"
      && one.context_window.or(one.max_context_window).is_some_and(|size| size > 0)
  });
  if listed.models.is_empty() {
    return Err(AuthError::NoModels);
  }
  save(
    &path.join("codex.models.json"),
    &Catalog { account: credential.account.clone(), fetched: now(), models: listed.models },
  )
}

/// Start a `ChatGPT` sign-in, or connect with an existing sign-in. The browser address is returned
/// to the host. A sign-in from a device, for a person who is not at this machine, needs no port
/// here: `OpenAI` gives a code that he enters on a page in the browser of any device, which the
/// state shows once it came.
pub fn begin(config: Option<&Path>, device: bool) -> Result<SignIn, AuthError> {
  let path = directory(config);
  let retry;
  {
    let all = flows();
    if let Some(flow) = all.get(&path).filter(|one| one.state.borrow().running()) {
      return Ok(flow.state.borrow().clone());
    }
    retry =
      all.get(&path).is_some_and(|flow| matches!(*flow.state.borrow(), SignIn::NeedsLogin(_)));
  }
  let runtime = runtime().inspect_err(|no| publish(&path, SignIn::Failed(no.to_string())))?;
  let (stop, canceled) = watch::channel(false);
  let (status, pending) = if !retry && usable(&path).is_some() {
    (SignIn::Connecting, None)
  } else if device {
    (SignIn::Connecting, Some(Pending::Device))
  } else {
    let (url, pending) =
      authorize().inspect_err(|failure| publish(&path, SignIn::Failed(failure.to_string())))?;
    (SignIn::Waiting { url, code: None }, Some(pending))
  };
  {
    let mut all = flows();
    let flow = all.entry(path.clone()).or_insert_with(|| fresh(status.clone()));
    flow.canceled = stop;
    flow.state.send_replace(status.clone());
  }
  runtime.spawn(finish(path, pending, canceled));
  Ok(status)
}

/// The browser address of a new sign-in, with the port that waits for its callback.
fn authorize() -> Result<(String, Pending), AuthError> {
  let listener = TcpListener::bind(("127.0.0.1", 1455)).map_err(AuthError::PortBusy)?;
  listener.set_nonblocking(true)?;
  let verifier = random()?;
  let state = random()?;
  let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
  let mut url = url::Url::parse(&format!("{}/oauth/authorize", issuer()))?;
  url.query_pairs_mut().extend_pairs([
    ("response_type", "code"),
    ("client_id", CLIENT),
    ("redirect_uri", REDIRECT),
    ("scope", "openid profile email offline_access"),
    ("code_challenge", &challenge),
    ("code_challenge_method", "S256"),
    ("state", &state),
    ("id_token_add_organizations", "true"),
    ("codex_cli_simplified_flow", "true"),
    ("originator", "furb"),
  ]);
  Ok((url.to_string(), Pending::Browser { listener, state, verifier }))
}

async fn finish(path: PathBuf, pending: Option<Pending>, canceled: watch::Receiver<bool>) {
  let status = match signed(&path, pending, &canceled).await {
    Ok(()) => SignIn::Connected,
    Err(error) if error.login() => SignIn::NeedsLogin(error.to_string()),
    Err(error) => SignIn::Failed(error.to_string()),
  };
  update(&path, &canceled, status);
}

async fn signed(
  path: &Path,
  pending: Option<Pending>,
  canceled: &watch::Receiver<bool>,
) -> Result<(), AuthError> {
  let credential = match pending {
    Some(pending) => {
      let (code, verifier, redirect) = match pending {
        Pending::Browser { listener, state, verifier } => {
          (callback_code(listener, &state, canceled.clone()).await?, verifier, REDIRECT.to_owned())
        }
        Pending::Device => {
          let issuer = issuer();
          let device = device_code(&issuer, path, canceled).await?;
          (device.authorization_code, device.code_verifier, format!("{issuer}{DEVICE_REDIRECT}"))
        }
      };
      let token = token(&[
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT),
        ("code", &code),
        ("code_verifier", &verifier),
        ("redirect_uri", &redirect),
      ])
      .await?;
      let credential = token.credential(None)?;
      if *canceled.borrow() {
        return Err(AuthError::Canceled);
      }
      save(&path.join("codex.json"), &credential)?;
      credential
    }
    None => credential(path).await?,
  };
  listed(path, &credential).await
}

/// The code of the sign-in that the browser gives back on the callback of furb, awaited on the
/// runtime of the ears until it comes, the sign-in is canceled or ten minutes pass.
async fn callback_code(
  listener: TcpListener,
  state: &str,
  mut canceled: watch::Receiver<bool>,
) -> Result<String, AuthError> {
  let listener = net::TcpListener::from_std(listener)?;
  let heard = async {
    loop {
      let (mut stream, _) = listener.accept().await?;
      let mut bytes = [0; 8192];
      let mut count = 0;
      while !bytes.get(..count).unwrap_or_default().windows(4).any(|part| part == b"\r\n\r\n") {
        let Some(free) = bytes.get_mut(count..).filter(|free| !free.is_empty()) else { break };
        let read = time::timeout(Duration::from_secs(2), stream.read(free));
        match read.await {
          Ok(Ok(0)) | Err(_) => break,
          Ok(Ok(read)) => count += read,
          Ok(Err(no)) => return Err(AuthError::from(no)),
        }
      }
      let text = String::from_utf8_lossy(bytes.get(..count).unwrap_or_default());
      let target = text
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|line| line.split_once(' '))
        .map(|one| one.0);
      let url =
        target.and_then(|target| url::Url::parse(&format!("http://localhost{target}")).ok());
      let pairs: HashMap<_, _> =
        url.as_ref().map(|url| url.query_pairs().into_owned().collect()).unwrap_or_default();
      let valid = url.as_ref().is_some_and(|url| url.path() == "/auth/callback")
        && pairs.get("state").is_some_and(|one| one == state);
      let code = valid.then(|| pairs.get("code").cloned()).flatten();
      let (status, body) = if code.is_some() {
        ("200 OK", "Finishing OpenAI sign-in. Return to furb.")
      } else {
        ("400 Bad Request", "This sign-in response is invalid. Return to furb and try again.")
      };
      let answer = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
      );
      // The browser shows the answer when it can; the code holds whether or not it does.
      drop(stream.write_all(answer.as_bytes()).await);
      if let Some(code) = code {
        return Ok(code);
      }
      if valid && pairs.contains_key("error") {
        return Err(AuthError::Declined);
      }
    }
  };
  tokio::select! {
    got = time::timeout(Duration::from_secs(600), heard) => got.unwrap_or(Err(AuthError::Ended)),
    _ = canceled.wait_for(|one| *one) => Err(AuthError::Ended),
  }
}

/// What `OpenAI` gives for a sign-in from a device: its id, the code that the person enters, and
/// the seconds to wait between two asks of its end, which it gives as text.
#[derive(Deserialize)]
struct Given {
  device_auth_id: String,
  #[serde(alias = "usercode")]
  user_code: String,
  #[serde(default, deserialize_with = "seconds")]
  interval: u64,
}

/// The code of a sign-in from a device once the person entered it, with the verifier of its
/// exchange.
#[derive(Deserialize)]
struct Authorized {
  authorization_code: String,
  code_verifier: String,
}

fn seconds<'de, D: de::Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
  String::deserialize(deserializer)?.trim().parse().map_err(de::Error::custom)
}

/// A sign-in from a device at an issuer: the code that it gives, told as the state of the sign-in
/// of the folder, then an ask of its end at the pace that it names, until the person entered the
/// code, the sign-in is canceled or the code ends.
async fn device_code(
  issuer: &str,
  path: &Path,
  canceled: &watch::Receiver<bool>,
) -> Result<Authorized, AuthError> {
  let api = format!("{issuer}/api/accounts/deviceauth");
  let response = posted(&format!("{api}/usercode"), &json!({ "client_id": CLIENT })).await?;
  match response.status().as_u16() {
    404 => return Err(AuthError::NoDevice),
    status if !response.status().is_success() => return Err(AuthError::Rejected { status }),
    _ => {}
  }
  let given: Given =
    serde_json::from_slice(&response.bytes().await?).map_err(AuthError::InvalidToken)?;
  let code = Some(given.user_code.clone());
  update(path, canceled, SignIn::Waiting { url: format!("{issuer}{DEVICE}"), code });
  let asked = json!({ "device_auth_id": given.device_auth_id, "user_code": given.user_code });
  // A server that names no pace is asked once a second, not as fast as the network goes.
  let pace = Duration::from_secs(given.interval.max(1));
  let start = Instant::now();
  let mut canceled = canceled.clone();
  loop {
    let response = posted(&format!("{api}/token"), &asked).await?;
    let status = response.status().as_u16();
    if response.status().is_success() {
      return serde_json::from_slice(&response.bytes().await?).map_err(AuthError::InvalidToken);
    }
    // The code waits for the person while the server answers 403 or 404.
    if !matches!(status, 403 | 404) {
      return Err(AuthError::Rejected { status });
    }
    let left = DEVICE_WAIT.saturating_sub(start.elapsed());
    if left.is_zero() {
      return Err(AuthError::Ended);
    }
    tokio::select! {
      () = time::sleep(pace.min(left)) => {}
      _ = canceled.wait_for(|one| *one) => return Err(AuthError::Ended),
    }
  }
}

/// The issuer of the sign-in: `OpenAI`, or the host that stands in for it.
fn issuer() -> String {
  env::var(STAND_IN).unwrap_or_else(|_| AUTH.to_owned())
}

/// The API of Codex: that of `OpenAI`, or that of the host that stands in for it.
pub(super) fn api() -> String {
  env::var(STAND_IN).map_or_else(|_| API.to_owned(), |one| format!("{one}/backend-api/codex"))
}

/// A JSON body posted to an address of the sign-in.
async fn posted(url: &str, body: &Value) -> Result<reqwest::Response, AuthError> {
  let request = ReqwestClient::new().post(url).timeout(Duration::from_secs(20));
  let request = request.header("content-type", "application/json").body(body.to_string());
  Ok(request.send().await?)
}
