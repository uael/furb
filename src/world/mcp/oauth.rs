//! The login of a remote server of MCP, by OAuth 2.1 as the protocol says, and the store of its tokens.
//!
//! A server that answers 401 names the metadata of its resource, or furb asks it at its well-known path: it names the
//! authorization server, whose metadata names its endpoints. furb registers a client there, opens the page of the
//! login in the browser of the system with a challenge of PKCE, takes the code that the browser hands back to a
//! listener on 127.0.0.1, and trades it for tokens, which it refreshes when they end. A server that names no metadata
//! is its own authorization server, at the endpoints that the earlier version of the protocol gives.
//!
//! The tokens of a server stand in the Keychain of macOS, and elsewhere in a file of the config directory that only
//! the user reads.

use std::{
  collections::HashMap,
  fs, io,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::{Arc, Mutex},
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
  task::JoinError,
};

use super::config::Login;
use crate::value;
use crate::world::disk;
use std::sync;
use tokio::task;
use tokio::time;

/// How long a login waits for the browser to hand back its code.
pub(super) const LOGIN: Duration = Duration::from_secs(300);

/// How long before its end a token is refreshed.
const EARLY: u64 = 60;

/// The service of the Keychain under which the tokens stand.
const SERVICE: &str = "furb-mcp";

/// What opens an address in the browser of the system, or why it did not.
pub type Opener = Arc<dyn Fn(&str) -> io::Result<()> + Send + Sync>;

/// Why a store of secrets kept or forgot nothing.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
  #[error("{}: {source}", path.display())]
  File { path: PathBuf, source: io::Error },
  #[error("the secrets are not JSON: {0}")]
  Json(#[from] serde_json::Error),
  #[error("the Keychain did not open: {0}")]
  Open(io::Error),
  #[error("the Keychain took nothing: {0}")]
  Write(io::Error),
  #[error("the Keychain did not answer: {0}")]
  Wait(io::Error),
  /// The program of the Keychain failed, and what it wrote on its stderr.
  #[error("the Keychain kept nothing: {0}")]
  Kept(String),
}

/// Why a login or a refresh came to nothing.
#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
  #[error("the address {url} is not valid: {source}")]
  Address { url: String, source: url::ParseError },
  #[error("the endpoint {url} is not valid: {source}")]
  Endpoint { url: String, source: url::ParseError },
  #[error("no port for the login: {0}")]
  Port(io::Error),
  #[error("the login listened no more: {0}")]
  Listen(io::Error),
  #[error("the machine gave no random bytes: {0}")]
  Random(getrandom::Error),
  #[error("the browser did not open ({source}): open {address} to log in")]
  Browser { address: String, source: io::Error },
  #[error("no login came back within {} seconds from {address}", LOGIN.as_secs())]
  Waited { address: String },
  #[error("the login failed, {error}{}", described(.description.as_deref()))]
  Denied { error: String, description: Option<String> },
  #[error("the login came back with the state of another login")]
  Forged,
  #[error("the login came back with no code")]
  NoCode,
  #[error("the authorization server {issuer} registers no client, so furb cannot log in to it")]
  Unregistered { issuer: String },
  #[error("{url}: {source}")]
  Unreachable { url: String, source: reqwest::Error },
  #[error("{url} registered no client: {status} {text}")]
  NotRegistered { url: String, status: StatusCode, text: String },
  #[error("{url} answered {status}, {error}{}", described(.description.as_deref()))]
  Refused { url: String, status: StatusCode, error: String, description: Option<String> },
  #[error("the login gave no refresh token")]
  NoRefresh,
  #[error(transparent)]
  Vault(#[from] VaultError),
}

/// The description that an authorization server gave of an error, set after it.
fn described(description: Option<&str>) -> String {
  description.map(|one| format!(": {one}")).unwrap_or_default()
}

/// The tokens of a login, with what refreshes them: the client that furb registered, the endpoint of the tokens, and
/// the address of the server that they serve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Tokens {
  pub(super) access: String,
  #[serde(default)]
  pub(super) refresh: Option<String>,
  /// When the access token ends, in seconds since the epoch of Unix.
  #[serde(default)]
  pub(super) expires: Option<u64>,
  pub(super) client: String,
  #[serde(default)]
  pub(super) secret: Option<String>,
  pub(super) endpoint: String,
  pub(super) resource: String,
}

/// A store of secrets by key.
pub trait Vault: Send + Sync {
  fn get(&self, key: &str) -> Option<String>;
  /// # Errors
  /// When the store keeps nothing.
  fn put(&self, key: &str, value: &str) -> Result<(), VaultError>;
  /// # Errors
  /// When the store cannot forget.
  fn forget(&self, key: &str) -> Result<(), VaultError>;
}

/// The key of the tokens of a server: its name and its address.
pub(super) fn key(name: &str, url: &str) -> String {
  format!("{name} {url}")
}

/// The store of the tokens of this machine: the Keychain on macOS, and else the file `mcp-tokens.json` of the config
/// directory.
pub(super) fn vault(config: &Path) -> Arc<dyn Vault> {
  if cfg!(target_os = "macos") && Path::new(KEYCHAIN).exists() {
    Arc::new(Keychain)
  } else {
    Arc::new(Filed::new(config.join("mcp-tokens.json")))
  }
}

/// The opener of the system, which opens an address in the default browser.
pub(super) fn opener() -> Opener {
  Arc::new(|address: &str| {
    let mut command = if cfg!(target_os = "macos") {
      Command::new("open")
    } else if cfg!(windows) {
      let mut command = Command::new("rundll32");
      command.arg("url.dll,FileProtocolHandler");
      command
    } else {
      Command::new("xdg-open")
    };
    command.arg(address).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    command.spawn().map(drop)
  })
}

/// The address of the metadata of the resource that a challenge of a server names, in its `resource_metadata`.
pub(super) fn challenged(header: &str) -> Option<String> {
  let at = header.find("resource_metadata=")?;
  let rest = &header[at + "resource_metadata=".len()..];
  let named = match rest.strip_prefix('"') {
    Some(quoted) => quoted.split('"').next()?,
    None => rest.split([',', ' ']).next()?,
  };
  (!named.is_empty()).then(|| named.to_owned())
}

/// The tokens of a server, from a store.
pub(super) fn stored(vault: &dyn Vault, key: &str) -> Option<Tokens> {
  vault.get(key).and_then(|held| serde_json::from_str(&held).ok())
}

/// The tokens of a server, kept in a store.
pub(super) fn store(vault: &dyn Vault, key: &str, tokens: &Tokens) -> Result<(), VaultError> {
  vault.put(key, &serde_json::to_string(tokens)?)
}

/// The access token of a server.
/// It also holds the failure of the store, if the store kept no refreshed tokens.
pub(super) struct Access {
  pub(super) token: String,
  pub(super) unsaved: Option<VaultError>,
}

/// The access token of a server. It is refreshed first when it ends within a minute.
/// The result is none when there is no token or it cannot be refreshed.
/// A refreshed token still serves when the store does not keep it.
/// The failure of the store stays with it.
pub(super) async fn token(vault: Arc<dyn Vault>, key: String) -> Option<Access> {
  let (held, place) = (Arc::clone(&vault), key.clone());
  let tokens = blocking(move || stored(held.as_ref(), &place)).await.ok()??;
  if tokens.expires.is_none_or(|at| at > now() + EARLY) {
    return Some(Access { token: tokens.access, unsaved: None });
  }
  let fresh = refreshed(&tokens).await.ok()?;
  let token = fresh.access.clone();
  let unsaved = match blocking(move || store(vault.as_ref(), &key, &fresh)).await {
    Ok(Err(no)) => Some(no),
    Ok(Ok(())) | Err(_) => None,
  };
  Some(Access { token, unsaved })
}

/// A work that blocks, done on a thread that may block.
pub(super) async fn blocking<T: Send + 'static>(
  work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, JoinError> {
  task::spawn_blocking(work).await
}

/// The seconds since the epoch of Unix.
fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or_default()
}

/// The endpoints of an authorization server.
#[derive(Debug, PartialEq)]
pub(super) struct Endpoints {
  pub(super) authorize: String,
  pub(super) token: String,
  pub(super) register: Option<String>,
}

/// The tokens of a login to the server at an address, whose challenge named the metadata of its resource, or not:
/// the page of the login opened by the opener, and the code that the browser hands back traded for tokens.
pub(super) async fn login(
  url: &str,
  hint: Option<&str>,
  opener: &Opener,
  preset: &Login,
) -> Result<Tokens, Error> {
  let client = Client::new();
  let (issuer, scope) = protected(&client, url, hint).await?;
  let ends = authority(&client, &issuer).await?;
  let listener =
    TcpListener::bind(("127.0.0.1", preset.port.unwrap_or(0))).await.map_err(Error::Port)?;
  let port = listener.local_addr().map_err(Error::Port)?.port();
  let host = if preset.port.is_some() { "localhost" } else { "127.0.0.1" };
  let redirect = format!("http://{host}:{port}/callback");
  let (id, secret) = match &preset.client {
    Some(id) => (id.clone(), None),
    None => registered(&client, &issuer, &ends, &redirect).await?,
  };
  let (verifier, state) = (random(48)?, random(24)?);
  let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
  let mut address = url::Url::parse(&ends.authorize)
    .map_err(|source| Error::Endpoint { url: ends.authorize.clone(), source })?;
  {
    let mut query = address.query_pairs_mut();
    query.append_pair("response_type", "code").append_pair("client_id", &id);
    query.append_pair("redirect_uri", &redirect).append_pair("code_challenge", &challenge);
    query.append_pair("code_challenge_method", "S256").append_pair("state", &state);
    query.append_pair("resource", url);
    if let Some(scope) = &scope {
      query.append_pair("scope", scope);
    }
  }
  opener(address.as_str())
    .map_err(|source| Error::Browser { address: address.to_string(), source })?;
  let Ok(code) = time::timeout(LOGIN, callback(&listener, &state)).await else {
    return Err(Error::Waited { address: address.to_string() });
  };
  let code = code?;
  let mut form = vec![
    ("grant_type", "authorization_code"),
    ("code", &code),
    ("redirect_uri", &redirect),
    ("client_id", &id),
    ("code_verifier", &verifier),
    ("resource", url),
  ];
  if let Some(secret) = &secret {
    form.push(("client_secret", secret));
  }
  let got = granted(&client, &ends.token, &form).await?;
  Ok(tokens(&got, None, id.clone(), secret.clone(), ends.token.clone(), url.to_owned()))
}

/// Tokens refreshed with their refresh token, or why they cannot be.
pub(super) async fn refreshed(held: &Tokens) -> Result<Tokens, Error> {
  let refresh = held.refresh.as_deref().ok_or(Error::NoRefresh)?;
  let mut form = vec![
    ("grant_type", "refresh_token"),
    ("refresh_token", refresh),
    ("client_id", &held.client),
    ("resource", &held.resource),
  ];
  if let Some(secret) = &held.secret {
    form.push(("client_secret", secret));
  }
  let got = granted(&Client::new(), &held.endpoint, &form).await?;
  Ok(tokens(
    &got,
    held.refresh.clone(),
    held.client.clone(),
    held.secret.clone(),
    held.endpoint.clone(),
    held.resource.clone(),
  ))
}

/// The tokens that an endpoint granted, with what refreshes them.
fn tokens(
  got: &Value,
  refresh: Option<String>,
  client: String,
  secret: Option<String>,
  endpoint: String,
  resource: String,
) -> Tokens {
  Tokens {
    access: got["access_token"].as_str().unwrap_or_default().to_owned(),
    refresh: got["refresh_token"].as_str().map(str::to_owned).or(refresh),
    expires: got["expires_in"].as_u64().map(|seconds| now() + seconds),
    client,
    secret,
    endpoint,
    resource,
  }
}

/// A text of random bytes, as base64 that an address carries.
fn random(bytes: usize) -> Result<String, Error> {
  let mut held = vec![0u8; bytes];
  getrandom::fill(&mut held).map_err(Error::Random)?;
  Ok(URL_SAFE_NO_PAD.encode(held))
}

/// The origin of an address, and its path with no slash at its end.
fn split(url: &str) -> Result<(String, String), Error> {
  let parsed =
    url::Url::parse(url).map_err(|source| Error::Address { url: url.to_owned(), source })?;
  Ok((parsed.origin().ascii_serialization(), parsed.path().trim_end_matches('/').to_owned()))
}

/// The JSON at an address, when it answers with a success.
async fn fetched(client: &Client, url: &str) -> Option<Value> {
  let response = client.get(url).header("accept", "application/json").send().await.ok()?;
  if !response.status().is_success() {
    return None;
  }
  serde_json::from_str(&response.text().await.ok()?).ok()
}

/// The authorization server of the server at an address and the scopes it asks for, from the metadata of its
/// resource that its challenge named, or at the well-known path of its address; or the origin of the address when it
/// names none.
pub(super) async fn protected(
  client: &Client,
  url: &str,
  hint: Option<&str>,
) -> Result<(String, Option<String>), Error> {
  let (origin, path) = split(url)?;
  let mut candidates: Vec<String> = hint.map(str::to_owned).into_iter().collect();
  if !path.is_empty() {
    candidates.push(format!("{origin}/.well-known/oauth-protected-resource{path}"));
  }
  candidates.push(format!("{origin}/.well-known/oauth-protected-resource"));
  for one in candidates {
    let Some(held) = fetched(client, &one).await else { continue };
    let Some(issuer) = held["authorization_servers"][0].as_str() else { continue };
    let scopes = held["scopes_supported"]
      .as_array()
      .map(|all| all.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "));
    return Ok((issuer.to_owned(), scopes.filter(|scopes| !scopes.is_empty())));
  }
  Ok((origin, None))
}

/// The endpoints of an authorization server.
/// They come from its metadata, at the well-known paths of OAuth and of `OpenID`.
/// When the server gives none, the endpoints are at the root of its origin.
pub(super) async fn authority(client: &Client, issuer: &str) -> Result<Endpoints, Error> {
  let (origin, path) = split(issuer)?;
  let candidates = if path.is_empty() {
    vec![
      format!("{origin}/.well-known/oauth-authorization-server"),
      format!("{origin}/.well-known/openid-configuration"),
    ]
  } else {
    vec![
      format!("{origin}/.well-known/oauth-authorization-server{path}"),
      format!("{origin}/.well-known/openid-configuration{path}"),
      format!("{origin}{path}/.well-known/openid-configuration"),
    ]
  };
  for one in candidates {
    let Some(held) = fetched(client, &one).await else { continue };
    let (Some(authorize), Some(token)) =
      (held["authorization_endpoint"].as_str(), held["token_endpoint"].as_str())
    else {
      continue;
    };
    let register = held["registration_endpoint"].as_str().map(str::to_owned);
    return Ok(Endpoints { authorize: authorize.to_owned(), token: token.to_owned(), register });
  }
  Ok(Endpoints {
    authorize: format!("{origin}/authorize"),
    token: format!("{origin}/token"),
    register: Some(format!("{origin}/register")),
  })
}

/// The client that furb registered at an authorization server for this redirect: its id and its secret when it gave
/// one.
async fn registered(
  client: &Client,
  issuer: &str,
  ends: &Endpoints,
  redirect: &str,
) -> Result<(String, Option<String>), Error> {
  let register =
    ends.register.as_deref().ok_or_else(|| Error::Unregistered { issuer: issuer.to_owned() })?;
  let body = json!({
    "client_name": "furb",
    "redirect_uris": [redirect],
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "token_endpoint_auth_method": "none",
  });
  let response = client
    .post(register)
    .header("content-type", "application/json")
    .header("accept", "application/json")
    .body(body.to_string())
    .send()
    .await
    .map_err(|source| Error::Unreachable { url: register.to_owned(), source })?;
  let status = response.status();
  let text = response.text().await.unwrap_or_default();
  let got: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
  match got["client_id"].as_str() {
    Some(id) if status.is_success() => {
      Ok((id.to_owned(), got["client_secret"].as_str().map(str::to_owned)))
    }
    _ => Err(Error::NotRegistered {
      url: register.to_owned(),
      status,
      text: text.chars().take(300).collect(),
    }),
  }
}

/// What an endpoint of the tokens grants for a form, or why it refused.
async fn granted(client: &Client, endpoint: &str, form: &[(&str, &str)]) -> Result<Value, Error> {
  let body = url::form_urlencoded::Serializer::new(String::new()).extend_pairs(form).finish();
  let response = client
    .post(endpoint)
    .header("content-type", "application/x-www-form-urlencoded")
    .header("accept", "application/json")
    .body(body)
    .send()
    .await
    .map_err(|source| Error::Unreachable { url: endpoint.to_owned(), source })?;
  let status = response.status();
  let got: Value =
    serde_json::from_str(&response.text().await.unwrap_or_default()).unwrap_or(Value::Null);
  if !status.is_success() || got["access_token"].as_str().is_none_or(str::is_empty) {
    return Err(Error::Refused {
      url: endpoint.to_owned(),
      status,
      error: got["error"].as_str().unwrap_or("no token").to_owned(),
      description: got["error_description"].as_str().map(str::to_owned),
    });
  }
  Ok(got)
}

/// The path and the query of the request that a stream holds, as far as its head reaches.
async fn requested(stream: &mut TcpStream) -> String {
  let (mut held, mut buffer) = (Vec::new(), [0u8; 4096]);
  while !held.windows(4).any(|four| four == b"\r\n\r\n") && held.len() < 16384 {
    match stream.read(&mut buffer).await {
      Ok(0) | Err(_) => break,
      Ok(size) => held.extend_from_slice(&buffer[..size]),
    }
  }
  let text = String::from_utf8_lossy(&held);
  text.split_whitespace().nth(1).unwrap_or_default().to_owned()
}

/// The page that tells the person at the browser how the login went.
fn page(got: &Result<String, Error>) -> String {
  let said = match got {
    Ok(_) => "You are logged in. Close this window and go back to furb.".to_owned(),
    Err(why) => format!("The login failed: {why}."),
  };
  let page = format!("<!doctype html><title>furb</title><p>{}</p>", said.replace('<', "&lt;"));
  format!(
    "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{page}",
    page.len()
  )
}

/// The code that the browser hands back to the listener, with the state of this login; or why the login failed. A
/// request to any other path is answered 404, and the listener waits on.
async fn callback(listener: &TcpListener, state: &str) -> Result<String, Error> {
  loop {
    let (mut stream, _) = listener.accept().await.map_err(Error::Listen)?;
    let target = requested(&mut stream).await;
    let asked = url::Url::parse(&format!("http://127.0.0.1{target}"))
      .ok()
      .filter(|one| one.path() == "/callback");
    let Some(asked) = asked else {
      // A browser that went away is answered no more.
      stream
        .write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
        .await
        .ok();
      continue;
    };
    let pairs: HashMap<String, String> = asked.query_pairs().into_owned().collect();
    let got = match (pairs.get("error"), pairs.get("state"), pairs.get("code")) {
      (Some(error), _, _) => Err(Error::Denied {
        error: error.clone(),
        description: pairs.get("error_description").cloned(),
      }),
      (None, Some(given), Some(code)) if given == state => Ok(code.clone()),
      (None, Some(_), _) => Err(Error::Forged),
      _ => Err(Error::NoCode),
    };
    // A browser that went away needs no page.
    stream.write_all(page(&got).as_bytes()).await.ok();
    return got;
  }
}

/// The program of the Keychain of macOS.
const KEYCHAIN: &str = "/usr/bin/security";

/// The Keychain of macOS, under the service furb-mcp. A secret goes to it on the stdin of the program as hex, so it
/// stands in the arguments of no process.
struct Keychain;

impl Keychain {
  /// The account of a key in the Keychain: the key, with each sign that the stdin of the program would read as a
  /// break as `_`.
  fn account(key: &str) -> String {
    key
      .chars()
      .map(|one| if one.is_ascii_alphanumeric() || ":/._@-".contains(one) { one } else { '_' })
      .collect()
  }
}

impl Vault for Keychain {
  fn get(&self, key: &str) -> Option<String> {
    let account = Self::account(key);
    let got = Command::new(KEYCHAIN)
      .args(["find-generic-password", "-s", SERVICE, "-a", &account, "-w"])
      .output();
    let got = got.ok().filter(|got| got.status.success())?;
    Some(String::from_utf8_lossy(&got.stdout).trim_end_matches('\n').to_owned())
  }

  fn put(&self, key: &str, value: &str) -> Result<(), VaultError> {
    let hex = value::hex(value.as_bytes());
    let line = format!("add-generic-password -U -s {SERVICE} -a {} -X {hex}\n", Self::account(key));
    let mut child = Command::new(KEYCHAIN)
      .arg("-i")
      .stdin(Stdio::piped())
      .stdout(Stdio::null())
      .stderr(Stdio::piped())
      .spawn()
      .map_err(VaultError::Open)?;
    if let Some(mut stdin) = child.stdin.take() {
      io::Write::write_all(&mut stdin, line.as_bytes()).map_err(VaultError::Write)?;
    }
    let got = child.wait_with_output().map_err(VaultError::Wait)?;
    if got.status.success() && got.stderr.is_empty() {
      Ok(())
    } else {
      Err(VaultError::Kept(String::from_utf8_lossy(&got.stderr).trim().to_owned()))
    }
  }

  fn forget(&self, key: &str) -> Result<(), VaultError> {
    let account = Self::account(key);
    let got = Command::new(KEYCHAIN)
      .args(["delete-generic-password", "-s", SERVICE, "-a", &account])
      .output();
    got.map(drop).map_err(VaultError::Open)
  }
}

/// A file of secrets by key, which only the user reads.
pub struct Filed {
  path: PathBuf,
  lock: Mutex<()>,
}

impl Filed {
  pub fn new(path: PathBuf) -> Self {
    Self { path, lock: Mutex::new(()) }
  }

  /// The secrets of the file.
  fn held(&self) -> HashMap<String, String> {
    fs::read_to_string(&self.path)
      .ok()
      .and_then(|text| serde_json::from_str(&text).ok())
      .unwrap_or_default()
  }

  /// The secrets, written whole in place of the file, which only the user reads.
  fn wrote(&self, held: &HashMap<String, String>) -> Result<(), VaultError> {
    let text = serde_json::to_string_pretty(held)?;
    disk::write(&self.path, text.as_bytes(), disk::Access::Owner)
      .map_err(|no| VaultError::File { path: no.path, source: no.source })
  }
}

impl Vault for Filed {
  fn get(&self, key: &str) -> Option<String> {
    let _held = self.lock.lock().unwrap_or_else(sync::PoisonError::into_inner);
    self.held().remove(key)
  }

  fn put(&self, key: &str, value: &str) -> Result<(), VaultError> {
    let _held = self.lock.lock().unwrap_or_else(sync::PoisonError::into_inner);
    let mut held = self.held();
    held.insert(key.to_owned(), value.to_owned());
    self.wrote(&held)
  }

  fn forget(&self, key: &str) -> Result<(), VaultError> {
    let _held = self.lock.lock().unwrap_or_else(sync::PoisonError::into_inner);
    let mut held = self.held();
    match held.remove(key) {
      Some(_) => self.wrote(&held),
      None => Ok(()),
    }
  }
}

#[cfg(test)]
#[path = "oauth.test.rs"]
mod test;
