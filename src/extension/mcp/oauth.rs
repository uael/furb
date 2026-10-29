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
  fs,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::{Arc, Mutex},
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpListener,
};

/// How long a login waits for the browser to hand back its code.
pub const LOGIN: Duration = Duration::from_secs(300);

/// How long before its end a token is refreshed.
const EARLY: u64 = 60;

/// The service of the Keychain under which the tokens stand.
const SERVICE: &str = "furb-mcp";

/// What opens an address in the browser of the system, or why it did not.
pub type Opener = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// The tokens of a login, with what refreshes them: the client that furb registered, the endpoint of the tokens, and
/// the address of the server that they serve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tokens {
  pub access: String,
  #[serde(default)]
  pub refresh: Option<String>,
  /// When the access token ends, in seconds since the epoch of Unix.
  #[serde(default)]
  pub expires: Option<u64>,
  pub client: String,
  #[serde(default)]
  pub secret: Option<String>,
  pub endpoint: String,
  pub resource: String,
}

/// A store of secrets by key.
pub trait Vault: Send + Sync {
  fn get(&self, key: &str) -> Option<String>;
  fn put(&self, key: &str, value: &str) -> Result<(), String>;
  fn forget(&self, key: &str) -> Result<(), String>;
}

/// The key of the tokens of a server: its name and its address.
pub fn key(name: &str, url: &str) -> String {
  format!("{name} {url}")
}

/// The store of the tokens of this machine: the Keychain on macOS, and else the file `mcp-tokens.json` of the config
/// directory.
pub fn vault(config: &Path) -> Arc<dyn Vault> {
  match cfg!(target_os = "macos") && Path::new(KEYCHAIN).exists() {
    true => Arc::new(Keychain),
    false => Arc::new(Filed::new(config.join("mcp-tokens.json"))),
  }
}

/// The opener of the system, which opens an address in the default browser.
pub fn opener() -> Opener {
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
    command.spawn().map(drop).map_err(|no| no.to_string())
  })
}

/// The address of the metadata of the resource that a challenge of a server names, in its `resource_metadata`.
pub fn challenged(header: &str) -> Option<String> {
  let at = header.find("resource_metadata=")?;
  let rest = &header[at + "resource_metadata=".len()..];
  let named = match rest.strip_prefix('"') {
    Some(quoted) => quoted.split('"').next()?,
    None => rest.split([',', ' ']).next()?,
  };
  (!named.is_empty()).then(|| named.to_owned())
}

/// The tokens of a server, from a store.
pub fn stored(vault: &dyn Vault, key: &str) -> Option<Tokens> {
  vault.get(key).and_then(|held| serde_json::from_str(&held).ok())
}

/// The tokens of a server, kept in a store.
pub fn store(vault: &dyn Vault, key: &str, tokens: &Tokens) -> Result<(), String> {
  vault.put(key, &serde_json::to_string(tokens).expect("tokens are JSON"))
}

/// The access token of a server, refreshed first when it ends within a minute; none when there is none or it cannot
/// be refreshed.
pub async fn token(vault: Arc<dyn Vault>, key: String) -> Option<String> {
  let (held, place) = (vault.clone(), key.clone());
  let tokens = blocking(move || stored(held.as_ref(), &place)).await?;
  if tokens.expires.is_none_or(|at| at > now() + EARLY) {
    return Some(tokens.access);
  }
  let fresh = refreshed(&tokens).await.ok()?;
  let access = fresh.access.clone();
  let _ = blocking(move || store(vault.as_ref(), &key, &fresh)).await;
  Some(access)
}

/// A work that blocks, done on a thread that may block.
pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
  tokio::task::spawn_blocking(work).await.expect("the work of a store ended")
}

/// The seconds since the epoch of Unix.
fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or_default()
}

/// The endpoints of an authorization server.
#[derive(Debug, PartialEq)]
pub struct Endpoints {
  pub authorize: String,
  pub token: String,
  pub register: Option<String>,
}

/// The tokens of a login to the server at an address, whose challenge named the metadata of its resource, or not:
/// the page of the login opened by the opener, and the code that the browser hands back traded for tokens.
pub async fn login(url: &str, hint: Option<&str>, opener: &Opener) -> Result<Tokens, String> {
  let client = Client::new();
  let (issuer, scope) = protected(&client, url, hint).await?;
  let ends = authority(&client, &issuer).await?;
  let listener =
    TcpListener::bind("127.0.0.1:0").await.map_err(|no| format!("no port for the login: {no}"))?;
  let port = listener.local_addr().map_err(|no| format!("no port for the login: {no}"))?.port();
  let redirect = format!("http://127.0.0.1:{port}/callback");
  let (id, secret) = registered(&client, &issuer, &ends, &redirect).await?;
  let (verifier, state) = (random(48)?, random(24)?);
  let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
  let mut address = url::Url::parse(&ends.authorize)
    .map_err(|no| format!("the endpoint {} is not valid: {no}", ends.authorize))?;
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
    .map_err(|why| format!("the browser did not open ({why}): open {address} to log in"))?;
  let code = tokio::time::timeout(LOGIN, callback(&listener, &state)).await.map_err(|_| {
    format!("no login came back within {} seconds from {address}", LOGIN.as_secs())
  })??;
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
pub async fn refreshed(held: &Tokens) -> Result<Tokens, String> {
  let refresh = held.refresh.as_deref().ok_or("the login gave no refresh token")?;
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
fn random(bytes: usize) -> Result<String, String> {
  let mut held = vec![0u8; bytes];
  getrandom::fill(&mut held).map_err(|no| format!("the machine gave no random bytes: {no}"))?;
  Ok(URL_SAFE_NO_PAD.encode(held))
}

/// The origin of an address, and its path with no slash at its end.
fn split(url: &str) -> Result<(String, String), String> {
  let parsed =
    url::Url::parse(url).map_err(|no| format!("the address {url} is not valid: {no}"))?;
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
pub async fn protected(
  client: &Client,
  url: &str,
  hint: Option<&str>,
) -> Result<(String, Option<String>), String> {
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

/// The endpoints of an authorization server, from its metadata at the well-known paths of OAuth and of OpenID; or the
/// endpoints at the root of its origin when it gives none.
pub async fn authority(client: &Client, issuer: &str) -> Result<Endpoints, String> {
  let (origin, path) = split(issuer)?;
  let candidates = match path.is_empty() {
    true => vec![
      format!("{origin}/.well-known/oauth-authorization-server"),
      format!("{origin}/.well-known/openid-configuration"),
    ],
    false => vec![
      format!("{origin}/.well-known/oauth-authorization-server{path}"),
      format!("{origin}/.well-known/openid-configuration{path}"),
      format!("{origin}{path}/.well-known/openid-configuration"),
    ],
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
) -> Result<(String, Option<String>), String> {
  let register = ends.register.as_deref().ok_or_else(|| {
    format!("the authorization server {issuer} registers no client, so furb cannot log in to it")
  })?;
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
    .map_err(|no| format!("{register}: {no}"))?;
  let status = response.status();
  let text = response.text().await.unwrap_or_default();
  let got: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
  match got["client_id"].as_str() {
    Some(id) if status.is_success() => {
      Ok((id.to_owned(), got["client_secret"].as_str().map(str::to_owned)))
    }
    _ => Err(format!(
      "{register} registered no client: {status} {}",
      text.chars().take(300).collect::<String>()
    )),
  }
}

/// What an endpoint of the tokens grants for a form, or why it refused.
async fn granted(client: &Client, endpoint: &str, form: &[(&str, &str)]) -> Result<Value, String> {
  let body = url::form_urlencoded::Serializer::new(String::new()).extend_pairs(form).finish();
  let response = client
    .post(endpoint)
    .header("content-type", "application/x-www-form-urlencoded")
    .header("accept", "application/json")
    .body(body)
    .send()
    .await
    .map_err(|no| format!("{endpoint}: {no}"))?;
  let status = response.status();
  let got: Value =
    serde_json::from_str(&response.text().await.unwrap_or_default()).unwrap_or(Value::Null);
  if !status.is_success() || got["access_token"].as_str().is_none_or(str::is_empty) {
    let error = got["error"].as_str().unwrap_or("no token");
    let described =
      got["error_description"].as_str().map(|one| format!(": {one}")).unwrap_or_default();
    return Err(format!("{endpoint} answered {status}, {error}{described}"));
  }
  Ok(got)
}

/// The code that the browser hands back to the listener, with the state of this login; or why the login failed. A
/// request to any other path is answered 404, and the listener waits on.
async fn callback(listener: &TcpListener, state: &str) -> Result<String, String> {
  loop {
    let (mut stream, _) =
      listener.accept().await.map_err(|no| format!("the login listened no more: {no}"))?;
    let (mut held, mut buffer) = (Vec::new(), [0u8; 4096]);
    while !held.windows(4).any(|four| four == b"\r\n\r\n") && held.len() < 16384 {
      match stream.read(&mut buffer).await {
        Ok(0) | Err(_) => break,
        Ok(size) => held.extend_from_slice(&buffer[..size]),
      }
    }
    let head = String::from_utf8_lossy(&held);
    let target = head.split_whitespace().nth(1).unwrap_or_default();
    let asked = url::Url::parse(&format!("http://127.0.0.1{target}"))
      .ok()
      .filter(|one| one.path() == "/callback");
    let Some(asked) = asked else {
      let _ = stream
        .write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
        .await;
      continue;
    };
    let pairs: HashMap<String, String> = asked.query_pairs().into_owned().collect();
    let got = match (pairs.get("error"), pairs.get("state"), pairs.get("code")) {
      (Some(error), _, _) => {
        let described =
          pairs.get("error_description").map(|one| format!(": {one}")).unwrap_or_default();
        Err(format!("the login failed, {error}{described}"))
      }
      (None, Some(given), Some(code)) if given == state => Ok(code.clone()),
      (None, Some(_), _) => Err("the login came back with the state of another login".to_owned()),
      _ => Err("the login came back with no code".to_owned()),
    };
    let said = match &got {
      Ok(_) => "You are logged in. Close this window and go back to furb.".to_owned(),
      Err(why) => format!("The login failed: {why}."),
    };
    let page = format!("<!doctype html><title>furb</title><p>{}</p>", said.replace('<', "&lt;"));
    let reply = format!(
      "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{page}",
      page.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
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
    let account = Keychain::account(key);
    let got = Command::new(KEYCHAIN)
      .args(["find-generic-password", "-s", SERVICE, "-a", &account, "-w"])
      .output();
    let got = got.ok().filter(|got| got.status.success())?;
    Some(String::from_utf8_lossy(&got.stdout).trim_end_matches('\n').to_owned())
  }

  fn put(&self, key: &str, value: &str) -> Result<(), String> {
    let hex: String = value.bytes().map(|one| format!("{one:02x}")).collect();
    let line =
      format!("add-generic-password -U -s {SERVICE} -a {} -X {hex}\n", Keychain::account(key));
    let mut child = Command::new(KEYCHAIN)
      .arg("-i")
      .stdin(Stdio::piped())
      .stdout(Stdio::null())
      .stderr(Stdio::piped())
      .spawn()
      .map_err(|no| format!("the Keychain did not open: {no}"))?;
    if let Some(mut stdin) = child.stdin.take() {
      std::io::Write::write_all(&mut stdin, line.as_bytes())
        .map_err(|no| format!("the Keychain took nothing: {no}"))?;
    }
    let got =
      child.wait_with_output().map_err(|no| format!("the Keychain did not answer: {no}"))?;
    match got.status.success() && got.stderr.is_empty() {
      true => Ok(()),
      false => {
        Err(format!("the Keychain kept nothing: {}", String::from_utf8_lossy(&got.stderr).trim()))
      }
    }
  }

  fn forget(&self, key: &str) -> Result<(), String> {
    let account = Keychain::account(key);
    let got = Command::new(KEYCHAIN)
      .args(["delete-generic-password", "-s", SERVICE, "-a", &account])
      .output();
    got.map(drop).map_err(|no| format!("the Keychain did not open: {no}"))
  }
}

/// A file of secrets by key, which only the user reads.
pub struct Filed {
  path: PathBuf,
  lock: Mutex<()>,
}

impl Filed {
  pub fn new(path: PathBuf) -> Self {
    Filed { path, lock: Mutex::new(()) }
  }

  /// The secrets of the file.
  fn held(&self) -> HashMap<String, String> {
    fs::read_to_string(&self.path)
      .ok()
      .and_then(|text| serde_json::from_str(&text).ok())
      .unwrap_or_default()
  }

  /// The secrets, written whole in place of the file, which only the user reads.
  fn wrote(&self, held: &HashMap<String, String>) -> Result<(), String> {
    let failed = |no: std::io::Error| format!("{}: {no}", self.path.display());
    if let Some(parent) = self.path.parent() {
      fs::create_dir_all(parent).map_err(failed)?;
    }
    let next = self.path.with_extension("json.new");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&next).map_err(failed)?;
    let text = serde_json::to_string_pretty(held).expect("secrets are JSON");
    std::io::Write::write_all(&mut file, text.as_bytes()).map_err(failed)?;
    drop(file);
    fs::rename(&next, &self.path).map_err(failed)
  }
}

impl Vault for Filed {
  fn get(&self, key: &str) -> Option<String> {
    let _held = self.lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    self.held().remove(key)
  }

  fn put(&self, key: &str, value: &str) -> Result<(), String> {
    let _held = self.lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut held = self.held();
    held.insert(key.to_owned(), value.to_owned());
    self.wrote(&held)
  }

  fn forget(&self, key: &str) -> Result<(), String> {
    let _held = self.lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
