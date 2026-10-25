//! OpenAI models on a ChatGPT subscription, with credentials kept outside the project.

use std::{
  collections::HashMap,
  fs::{self, File, OpenOptions},
  io::{Read, Write},
  net::TcpListener,
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rig_core::http_client::ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{ear::reactor, extension::Places};

/// The provider of models paid for by a ChatGPT subscription.
pub const NAME: &str = "openai-auth";
pub(super) const API: &str = "https://chatgpt.com/backend-api/codex";
const AUTH: &str = "https://auth.openai.com";
const CLIENT: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const REDIRECT: &str = "http://localhost:1455/auth/callback";
const CLIENT_VERSION: &str = "0.163.0";

/// The state of a sign-in, with a browser address while it waits and a message when it fails.
#[derive(Clone, Default)]
pub struct Status {
  pub state: String,
  pub url: Option<String>,
  pub message: Option<String>,
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

struct Flow {
  status: Status,
  canceled: Arc<AtomicBool>,
}

pub(super) struct Failure {
  message: String,
  login: bool,
}

impl std::fmt::Display for Failure {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl From<String> for Failure {
  fn from(message: String) -> Self {
    Self { message, login: false }
  }
}

impl From<&str> for Failure {
  fn from(message: &str) -> Self {
    message.to_owned().into()
  }
}

fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

pub(super) fn directory(config: Option<&Path>) -> PathBuf {
  config.map(Path::to_path_buf).unwrap_or_else(|| Places::here().config).join("auth")
}

fn flows() -> &'static Mutex<HashMap<PathBuf, Flow>> {
  static FLOWS: OnceLock<Mutex<HashMap<PathBuf, Flow>>> = OnceLock::new();
  FLOWS.get_or_init(Mutex::default)
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
  serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn save(path: &Path, value: &impl Serialize) -> Result<(), String> {
  let parent = path.parent().ok_or("The credential has no directory")?;
  fs::create_dir_all(parent).map_err(|no| no.to_string())?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|no| no.to_string())?;
  }
  let mut options = OpenOptions::new();
  options.write(true).create_new(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
  }
  let temporary = path.with_extension(format!("{}.tmp", random()?));
  let mut file = options.open(&temporary).map_err(|no| no.to_string())?;
  let result = (|| {
    file
      .write_all(&serde_json::to_vec(value).map_err(|no| no.to_string())?)
      .map_err(|no| no.to_string())?;
    file.sync_all().map_err(|no| no.to_string())?;
    fs::rename(&temporary, path).map_err(|no| no.to_string())
  })();
  if result.is_err() {
    let _ = fs::remove_file(temporary);
  }
  result
}

fn random() -> Result<String, String> {
  let mut bytes = [0; 32];
  getrandom::fill(&mut bytes).map_err(|no| no.to_string())?;
  Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn claims(token: &str) -> Option<Value> {
  let payload = token.split('.').nth(1)?;
  serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?).ok()
}

fn account(access: &str) -> Option<String> {
  claims(access)?["https://api.openai.com/auth"]["chatgpt_account_id"].as_str().map(str::to_owned)
}

fn borrowed() -> Option<Credential> {
  let home = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| {
    std::env::var_os("HOME")
      .or_else(|| std::env::var_os("USERPROFILE"))
      .map(|one| PathBuf::from(one).join(".codex"))
  })?;
  let value: Value = read(&home.join("auth.json"))?;
  if value["auth_mode"].as_str()? != "chatgpt" {
    return None;
  }
  let access = value["tokens"]["access_token"].as_str()?.to_owned();
  let expires = claims(&access)?["exp"].as_u64()?.saturating_mul(1000);
  if expires <= now() + 60_000 {
    return None;
  }
  Some(Credential { account: account(&access)?, access, refresh: None, expires })
}

fn held(config: &Path) -> Option<Credential> {
  read(&config.join("openai-auth.json")).or_else(borrowed)
}

/// The current sign-in state. This view starts no sign-in and makes no request.
pub fn status(config: Option<&Path>) -> Status {
  let path = directory(config);
  if let Some(flow) = flows().lock().unwrap_or_else(|no| no.into_inner()).get(&path)
    && flow.status.state != "connected"
  {
    return flow.status.clone();
  }
  let connected = held(&path).is_some_and(|one| {
    (one.expires > now() + 60_000 || one.refresh.is_some())
      && read::<Catalog>(&path.join("openai-auth.models.json"))
        .is_some_and(|catalog| catalog.account == one.account && !catalog.models.is_empty())
  });
  Status {
    state: if connected { "connected" } else { "disconnected" }.to_owned(),
    ..Status::default()
  }
}

/// Cancel a sign-in that has not finished. Saved credentials stay available.
pub fn cancel(config: Option<&Path>) {
  let path = directory(config);
  if let Some(flow) = flows()
    .lock()
    .unwrap_or_else(|no| no.into_inner())
    .get_mut(&path)
    .filter(|flow| ["waiting", "connecting"].contains(&flow.status.state.as_str()))
  {
    flow.canceled.store(true, Ordering::Relaxed);
    flow.status = Status { state: "canceled".to_owned(), ..Status::default() };
  }
}

fn update(path: &Path, canceled: &AtomicBool, status: Status) {
  if canceled.load(Ordering::Relaxed) {
    return;
  }
  if let Some(flow) = flows().lock().unwrap_or_else(|no| no.into_inner()).get_mut(path) {
    flow.status = status;
  }
}

pub(super) fn models(config: Option<&Path>) -> Vec<Listing> {
  let path = directory(config);
  if flows()
    .lock()
    .unwrap_or_else(|no| no.into_inner())
    .get(&path)
    .is_some_and(|flow| flow.status.state == "needs-login")
  {
    return Vec::new();
  }
  let Some(credential) = held(&path) else { return Vec::new() };
  read::<Catalog>(&path.join("openai-auth.models.json"))
    .filter(|one| one.account == credential.account)
    .map(|one| one.models)
    .unwrap_or_default()
}

pub(super) fn needs_login(path: &Path) {
  let mut flows = flows().lock().unwrap_or_else(|no| no.into_inner());
  if flows
    .get(path)
    .is_some_and(|flow| ["waiting", "connecting"].contains(&flow.status.state.as_str()))
  {
    return;
  }
  flows.insert(
    path.to_owned(),
    Flow {
      status: Status {
        state: "needs-login".to_owned(),
        url: None,
        message: Some("Sign in to OpenAI again from Providers.".to_owned()),
      },
      canceled: Arc::new(AtomicBool::new(false)),
    },
  );
}

pub(super) fn failed(path: &Path, failure: &Failure) {
  if failure.login {
    needs_login(path);
  }
}

/// Refresh an account's model list when it is more than five minutes old. This never starts a browser sign-in.
pub fn refresh(config: Option<&Path>) -> Result<Status, String> {
  let path = directory(config);
  let status = status(config);
  if ["waiting", "connecting", "needs-login"].contains(&status.state.as_str()) {
    return Ok(status);
  }
  let Some(credential) =
    held(&path).filter(|one| one.expires > now() + 60_000 || one.refresh.is_some())
  else {
    return Ok(Status { state: "disconnected".to_owned(), ..Status::default() });
  };
  let fresh = read::<Catalog>(&path.join("openai-auth.models.json")).is_some_and(|one| {
    one.account == credential.account && now().saturating_sub(one.fetched) < 300_000
  });
  if fresh && status.state != "failed" {
    return Ok(Status { state: "connected".to_owned(), ..Status::default() });
  }
  begin(config)
}

pub(super) async fn credential(path: &Path) -> Result<Credential, Failure> {
  let held = held(path).ok_or("Sign in to openai-auth from Providers")?;
  if held.expires > now() + 60_000 {
    return Ok(held);
  }
  fs::create_dir_all(path).map_err(|no| no.to_string())?;
  let path_of_lock = path.join("openai-auth.lock");
  let _lock = tokio::task::spawn_blocking(move || {
    let file = File::options()
      .create(true)
      .truncate(false)
      .write(true)
      .open(path_of_lock)
      .map_err(|no| no.to_string())?;
    file.lock().map_err(|no| no.to_string())?;
    Ok::<_, String>(file)
  })
  .await
  .map_err(|no| no.to_string())??;
  let held: Credential =
    read(&path.join("openai-auth.json")).ok_or("Sign in to openai-auth again")?;
  if held.expires > now() + 60_000 {
    return Ok(held);
  }
  let refresh = held.refresh.ok_or("Sign in to openai-auth again")?;
  let token =
    token(&[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", CLIENT)])
      .await?;
  let renewed = token.credential(Some(refresh))?;
  save(&path.join("openai-auth.json"), &renewed)?;
  Ok(renewed)
}

#[derive(Deserialize)]
struct Token {
  access_token: String,
  refresh_token: Option<String>,
  expires_in: u64,
}

impl Token {
  fn credential(self, refresh: Option<String>) -> Result<Credential, String> {
    let account = account(&self.access_token).ok_or("OpenAI did not return a ChatGPT account")?;
    let refresh = self
      .refresh_token
      .or(refresh)
      .ok_or("OpenAI did not return a refresh credential. Sign in again.")?;
    Ok(Credential {
      access: self.access_token,
      refresh: Some(refresh),
      expires: now().saturating_add(self.expires_in.saturating_mul(1000)),
      account,
    })
  }
}

async fn token(fields: &[(&str, &str)]) -> Result<Token, Failure> {
  let body: String = url::form_urlencoded::Serializer::new(String::new())
    .extend_pairs(fields.iter().copied())
    .finish();
  let response = ReqwestClient::new()
    .post(format!("{AUTH}/oauth/token"))
    .timeout(Duration::from_secs(20))
    .header("content-type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await
    .map_err(|no| no.to_string())?;
  if !response.status().is_success() {
    let login = matches!(response.status().as_u16(), 400 | 401 | 403);
    return Err(Failure {
      message: format!(
        "OpenAI sign-in failed (HTTP {}). {}",
        response.status().as_u16(),
        if login { "Sign in again." } else { "Try again." }
      ),
      login,
    });
  }
  serde_json::from_slice(&response.bytes().await.map_err(|no| no.to_string())?)
    .map_err(|_| Failure::from("OpenAI returned an invalid sign-in response"))
}

async fn listed(path: &Path, credential: &Credential) -> Result<(), Failure> {
  let response = ReqwestClient::new()
    .get(format!("{API}/models?client_version={CLIENT_VERSION}"))
    .bearer_auth(&credential.access)
    .header("chatgpt-account-id", &credential.account)
    .header("originator", "furb")
    .header("user-agent", concat!("furb/", env!("CARGO_PKG_VERSION")))
    .timeout(Duration::from_secs(20))
    .send()
    .await
    .map_err(|no| no.to_string())?;
  if !response.status().is_success() {
    return Err(Failure {
      message: format!(
        "OpenAI could not list this account's models (HTTP {}).",
        response.status().as_u16()
      ),
      login: matches!(response.status().as_u16(), 401 | 403),
    });
  }
  let bytes = response.bytes().await.map_err(|no| no.to_string())?;
  if bytes.len() > 1024 * 1024 {
    return Err("OpenAI returned a model list that is too large".into());
  }
  #[derive(Deserialize)]
  struct Response {
    models: Vec<Listing>,
  }
  let mut response: Response =
    serde_json::from_slice(&bytes).map_err(|_| "OpenAI returned an invalid model list")?;
  response.models.retain(|one| {
    one.visibility == "list"
      && one.context_window.or(one.max_context_window).is_some_and(|size| size > 0)
  });
  if response.models.is_empty() {
    return Err("This ChatGPT account offers no model to furb".into());
  }
  save(
    &path.join("openai-auth.models.json"),
    &Catalog { account: credential.account.clone(), fetched: now(), models: response.models },
  )
  .map_err(Failure::from)
}

/// Start a ChatGPT sign-in, or connect with an existing sign-in. The browser address is returned to the host.
pub fn begin(config: Option<&Path>) -> Result<Status, String> {
  let path = directory(config);
  let retry;
  {
    let flows = flows().lock().unwrap_or_else(|no| no.into_inner());
    if let Some(flow) =
      flows.get(&path).filter(|one| ["connecting", "waiting"].contains(&one.status.state.as_str()))
    {
      return Ok(flow.status.clone());
    }
    retry = flows.get(&path).is_some_and(|flow| flow.status.state == "needs-login");
  }
  let canceled = Arc::new(AtomicBool::new(false));
  let existing =
    held(&path).filter(|one| !retry && (one.expires > now() + 60_000 || one.refresh.is_some()));
  let (status, callback) = if existing.is_some() {
    (Status { state: "connecting".to_owned(), ..Status::default() }, None)
  } else {
    let listener = TcpListener::bind(("127.0.0.1", 1455))
      .map_err(|_| "Another OpenAI sign-in uses port 1455. Finish it, then try again.")?;
    listener.set_nonblocking(true).map_err(|no| no.to_string())?;
    let verifier = random()?;
    let state = random()?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url =
      url::Url::parse(&format!("{AUTH}/oauth/authorize")).map_err(|no| no.to_string())?;
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
    (
      Status { state: "waiting".to_owned(), url: Some(url.to_string()), message: None },
      Some((listener, state, verifier)),
    )
  };
  flows()
    .lock()
    .unwrap_or_else(|no| no.into_inner())
    .insert(path.clone(), Flow { status: status.clone(), canceled: canceled.clone() });
  reactor().spawn(async move {
    let result = async {
      let credential = match callback {
        Some((listener, state, verifier)) => {
          let stopping = canceled.clone();
          let code =
            tokio::task::spawn_blocking(move || callback_code(listener, &state, &stopping))
              .await
              .map_err(|no| no.to_string())??;
          let token = token(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", REDIRECT),
          ])
          .await?;
          let credential = token.credential(None)?;
          if canceled.load(Ordering::Relaxed) {
            return Err(Failure::from("Sign-in canceled"));
          }
          save(&path.join("openai-auth.json"), &credential)?;
          credential
        }
        None => credential(&path).await?,
      };
      listed(&path, &credential).await
    }
    .await;
    let status = match result {
      Ok(()) => Status { state: "connected".to_owned(), ..Status::default() },
      Err(error) => Status {
        state: if error.login { "needs-login" } else { "failed" }.to_owned(),
        message: Some(error.message),
        url: None,
      },
    };
    update(&path, &canceled, status);
  });
  Ok(status)
}

fn callback_code(
  listener: TcpListener,
  state: &str,
  canceled: &AtomicBool,
) -> Result<String, String> {
  let stop = Instant::now() + Duration::from_secs(600);
  while Instant::now() < stop && !canceled.load(Ordering::Relaxed) {
    let (mut stream, _) = match listener.accept() {
      Ok(one) => one,
      Err(no) if no.kind() == std::io::ErrorKind::WouldBlock => {
        std::thread::sleep(Duration::from_millis(100));
        continue;
      }
      Err(no) => return Err(no.to_string()),
    };
    stream.set_nonblocking(false).map_err(|no| no.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|no| no.to_string())?;
    let mut bytes = [0; 8192];
    let mut count = 0;
    while count < bytes.len() && !bytes[..count].windows(4).any(|part| part == b"\r\n\r\n") {
      match stream.read(&mut bytes[count..]) {
        Ok(0) => break,
        Ok(read) => count += read,
        Err(no)
          if matches!(no.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock) =>
        {
          break;
        }
        Err(no) => return Err(no.to_string()),
      }
    }
    let text = String::from_utf8_lossy(&bytes[..count]);
    let target = text
      .lines()
      .next()
      .and_then(|line| line.strip_prefix("GET "))
      .and_then(|line| line.split_once(' '))
      .map(|one| one.0);
    let url = target.and_then(|target| url::Url::parse(&format!("http://localhost{target}")).ok());
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
    let _ = write!(
      stream,
      "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
      body.len()
    );
    if let Some(code) = code {
      return Ok(code);
    }
    if valid && pairs.contains_key("error") {
      return Err("OpenAI sign-in was refused".to_owned());
    }
  }
  Err("OpenAI sign-in was canceled or timed out".to_owned())
}

#[cfg(test)]
#[path = "auth.test.rs"]
mod test;
