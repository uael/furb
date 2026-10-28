//! The servers of a life: what the configs define, the connection to each that runs, its state, and the reconnect of
//! a server whose connection ended.
//!
//! A server connects when a question first needs it, so a life that replays its record asks no server. Its first
//! connect gives it its state: connected, failed, or needs-auth when it asks for a login. A connection that ends
//! connects again after a wait that doubles from one second, at most thirty, and a server that fails five times in a
//! row stays failed until a control reconnects it. Each change that no question asked for, a list that the server
//! says has changed, a connection that ends and one that comes back, is told to the ear, which tells the life.

use std::{
  path::PathBuf,
  sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
  },
  time::Duration,
};

use serde_json::{Value, json};
use tokio::{
  sync::{Notify, mpsc},
  task::AbortHandle,
  time::Instant,
};

use super::{
  binding::binding,
  config::{self, CONNECT, Choice, Places, Server},
  oauth::{self, Opener, Vault},
  session::{self, Changed, Info, Offered, Session},
  wire::{Failure, Heard},
};
use crate::ear::reactor;

/// How many times in a row a server whose connection ended may fail to connect again.
const RETRIES: u32 = 5;

/// The value of each variable of the environment, or none when it is not set.
pub type Environment = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// What the servers of a life stand on: the files of the configs, the directory of the images, the store of the
/// tokens, the opener of the browser, and the variables of the environment.
#[derive(Clone)]
pub struct Settings {
  pub places: Places,
  pub gallery: Option<PathBuf>,
  pub vault: Arc<dyn Vault>,
  pub opener: Opener,
  pub variables: Environment,
}

/// The state of a server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
  Pending,
  Connected,
  Failed,
  NeedsAuth,
  Disabled,
  Unapproved,
}

impl State {
  /// The name of the state, as a server dict gives it.
  pub fn name(self) -> &'static str {
    match self {
      State::Pending => "pending",
      State::Connected => "connected",
      State::Failed => "failed",
      State::NeedsAuth => "needs-auth",
      State::Disabled => "disabled",
      State::Unapproved => "unapproved",
    }
  }
}

/// One server of the life: its definition, its state and why, its session while it is connected, what it said of
/// itself and offers, the metadata of the resource that its last challenge named, the generation of its keeper, and
/// the keeper, which ends with the entry.
struct Entry {
  server: Server,
  state: State,
  error: String,
  session: Option<Arc<Session>>,
  info: Info,
  offered: Offered,
  hint: Option<String>,
  generation: u64,
  keeper: Option<AbortHandle>,
}

impl Drop for Entry {
  fn drop(&mut self) {
    if let Some(keeper) = self.keeper.take() {
      keeper.abort();
    }
  }
}

/// The servers of a life.
pub struct Hub {
  settings: Settings,
  entries: Mutex<Vec<Entry>>,
  loaded: Mutex<bool>,
  moved: Notify,
  told: mpsc::UnboundedSender<String>,
  generations: AtomicU64,
}

impl Hub {
  /// The servers of a life, none loaded yet, which tell each change that no question asked for to `told`.
  pub fn new(settings: Settings, told: mpsc::UnboundedSender<String>) -> Arc<Hub> {
    Arc::new(Hub {
      settings,
      entries: Mutex::new(Vec::new()),
      loaded: Mutex::new(false),
      moved: Notify::new(),
      told,
      generations: AtomicU64::new(1),
    })
  }

  fn entries(&self) -> MutexGuard<'_, Vec<Entry>> {
    self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
  }

  /// The servers that the configs define, read again: a server whose definition did not change keeps its connection,
  /// a new or changed one connects when it may run, and one that the configs no longer define ends.
  pub fn reload(self: &Arc<Self>) {
    *self.loaded.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = true;
    let variables = self.settings.variables.clone();
    let servers = config::servers(&self.settings.places, &|name| variables(name));
    let mut entries = self.entries();
    let mut held: Vec<Entry> = std::mem::take(&mut *entries);
    for server in servers {
      let kept = held.iter().position(|one| one.server == server).map(|at| held.remove(at));
      let entry = kept.unwrap_or_else(|| {
        let state = match () {
          () if !server.error.is_empty() => State::Failed,
          () if server.disabled => State::Disabled,
          () if !server.approved => State::Unapproved,
          () => State::Pending,
        };
        let error = server.error.clone();
        let mut entry = Entry {
          server,
          state,
          error,
          session: None,
          info: Info::default(),
          offered: Offered::default(),
          hint: None,
          generation: 0,
          keeper: None,
        };
        if state == State::Pending {
          self.launch(&mut entry);
        }
        entry
      });
      entries.push(entry);
    }
    drop(entries);
    drop(held);
    self.moved.notify_waiters();
  }

  /// The servers, loaded once: a question that comes before any list of the servers loads them.
  fn loaded(self: &Arc<Self>) {
    if !*self.loaded.lock().unwrap_or_else(std::sync::PoisonError::into_inner) {
      self.reload();
    }
  }

  /// A keeper started for an entry, which connects it, in place of the keeper it had.
  fn launch(self: &Arc<Self>, entry: &mut Entry) {
    let generation = self.generations.fetch_add(1, Ordering::SeqCst);
    entry.generation = generation;
    entry.state = State::Pending;
    entry.error.clear();
    entry.session = None;
    if let Some(keeper) = entry.keeper.take() {
      keeper.abort();
    }
    let task = reactor().spawn(keep(self.clone(), entry.server.name.clone(), generation));
    entry.keeper = Some(task.abort_handle());
  }

  /// A change to the entry of a server while its keeper is of this generation, and whether it was.
  fn set(&self, name: &str, generation: u64, change: impl FnOnce(&mut Entry)) -> bool {
    let mut entries = self.entries();
    let Some(entry) =
      entries.iter_mut().find(|one| one.server.name == name && one.generation == generation)
    else {
      return false;
    };
    change(entry);
    drop(entries);
    self.moved.notify_waiters();
    true
  }

  /// A change of a server that no question asked for, told.
  fn tell(&self, name: &str) {
    let _ = self.told.send(name.to_owned());
  }

  /// Once no server of these names, or no server at all when it names none, is pending, or the bound passed.
  pub async fn settled(&self, names: &[&str], bound: Duration) {
    let deadline = Instant::now() + bound;
    loop {
      let moved = self.moved.notified();
      tokio::pin!(moved);
      moved.as_mut().enable();
      let pending = self.entries().iter().any(|one| {
        one.state == State::Pending
          && (names.is_empty() || names.contains(&one.server.name.as_str()))
      });
      if !pending || tokio::time::timeout_at(deadline, moved).await.is_err() {
        return;
      }
    }
  }

  /// The dict of each server, once each server that runs connected or failed, within the bound of a connect.
  pub async fn listed(self: &Arc<Self>) -> Vec<Value> {
    self.reload();
    self.settled(&[], CONNECT + Duration::from_secs(5)).await;
    self.entries().iter().map(dict).collect()
  }

  /// The dict of each server, read again from the configs, when no server is pending; none while one is.
  pub fn settled_now(self: &Arc<Self>) -> Option<Vec<Value>> {
    self.reload();
    let entries = self.entries();
    match entries.iter().any(|one| one.state == State::Pending) {
      true => None,
      false => Some(entries.iter().map(dict).collect()),
    }
  }

  /// The dict of a server.
  pub fn dict(&self, name: &str) -> Option<Value> {
    self.entries().iter().find(|one| one.server.name == name).map(dict)
  }

  /// The session of a server and its definition, once it settled, or why it has none.
  pub async fn session(self: &Arc<Self>, name: &str) -> Result<(Arc<Session>, Server), String> {
    self.loaded();
    self.settled(&[name], CONNECT + Duration::from_secs(5)).await;
    let entries = self.entries();
    let entry = entries
      .iter()
      .find(|one| one.server.name == name)
      .ok_or_else(|| format!("there is no server {name}"))?;
    match (&entry.session, entry.state) {
      (Some(session), State::Connected) => Ok((session.clone(), entry.server.clone())),
      (_, State::NeedsAuth) => {
        Err(format!("the server {name} asks for a login: mcp_control('login', {name:?})"))
      }
      (_, state) => {
        let why = if entry.error.is_empty() { String::new() } else { format!(": {}", entry.error) };
        Err(format!("the server {name} is not connected, it is {}{why}", state.name()))
      }
    }
  }

  /// A server whose request answered 401: its tokens refreshed and the server connected again, and whether it is
  /// connected then. A server whose tokens cannot be refreshed needs a login, which it tells.
  pub async fn renewed(self: &Arc<Self>, name: &str, hint: Option<String>) -> bool {
    let url = self
      .entries()
      .iter()
      .find(|one| one.server.name == name)
      .and_then(|one| one.server.transport.url().map(str::to_owned));
    let Some(url) = url else { return false };
    let (vault, key) = (self.settings.vault.clone(), oauth::key(name, &url));
    let held = {
      let (vault, key) = (vault.clone(), key.clone());
      oauth::blocking(move || oauth::stored(vault.as_ref(), &key)).await
    };
    let fresh = match held {
      Some(held) => oauth::refreshed(&held).await.ok(),
      None => None,
    };
    let Some(fresh) = fresh else {
      let mut entries = self.entries();
      if let Some(entry) = entries.iter_mut().find(|one| one.server.name == name) {
        entry.state = State::NeedsAuth;
        entry.error = format!("the server asks for a login: mcp_control('login', {name:?})");
        entry.session = None;
        entry.hint = hint;
      }
      drop(entries);
      self.moved.notify_waiters();
      self.tell(name);
      return false;
    };
    let _ = oauth::blocking(move || oauth::store(vault.as_ref(), &key, &fresh)).await;
    self.restart(name).is_ok() && {
      self.settled(&[name], CONNECT + Duration::from_secs(5)).await;
      self.entries().iter().any(|one| one.server.name == name && one.state == State::Connected)
    }
  }

  /// A server that may run, connected again from the start.
  fn restart(self: &Arc<Self>, name: &str) -> Result<(), String> {
    let mut entries = self.entries();
    let entry = entries
      .iter_mut()
      .find(|one| one.server.name == name)
      .ok_or_else(|| format!("there is no server {name}"))?;
    match () {
      () if !entry.server.error.is_empty() => {
        Err(format!("the server {name} cannot run: {}", entry.server.error))
      }
      () if entry.server.disabled => {
        Err(format!("the server {name} is disabled: mcp_control('enable', {name:?})"))
      }
      () if !entry.server.approved => Err(format!(
        "the server {name} of the project waits for approval: mcp_control('approve', {name:?})"
      )),
      () => {
        self.launch(entry);
        Ok(())
      }
    }
  }

  /// What a control of the operator does to a server, and its dict after it, once it settled.
  pub async fn control(self: &Arc<Self>, action: &str, name: &str) -> Result<Value, String> {
    self.loaded();
    let found = self
      .entries()
      .iter()
      .find(|one| one.server.name == name)
      .map(|one| (one.server.clone(), one.hint.clone()));
    let (server, hint) = found.ok_or_else(|| format!("there is no server {name}"))?;
    match action {
      "reconnect" => self.restart(name)?,
      "enable" => self.chose(Choice::Enable, name)?,
      "disable" => self.chose(Choice::Disable, name)?,
      "approve" => self.chose(Choice::Approve, name)?,
      "login" | "logout" => {
        let url = server
          .transport
          .url()
          .ok_or_else(|| format!("the server {name} runs on this machine and takes no login"))?;
        let (vault, key) = (self.settings.vault.clone(), oauth::key(name, url));
        if action == "login" {
          let tokens = oauth::login(url, hint.as_deref(), &self.settings.opener).await?;
          oauth::blocking(move || oauth::store(vault.as_ref(), &key, &tokens)).await?;
        } else {
          oauth::blocking(move || vault.forget(&key)).await?;
        }
        self.restart(name)?;
      }
      other => {
        return Err(format!(
          "the action {other} is none of reconnect, enable, disable, approve, login and logout"
        ));
      }
    }
    self.settled(&[name], CONNECT + Duration::from_secs(5)).await;
    self.dict(name).ok_or_else(|| format!("there is no server {name}"))
  }

  /// A choice of the operator, written to the local scope, and the servers read again.
  fn chose(self: &Arc<Self>, choice: Choice, name: &str) -> Result<(), String> {
    config::choose(&self.settings.places, choice, name)?;
    self.reload();
    Ok(())
  }

  /// The roots that furb gives a server that asks for them: the directory of the project.
  fn roots(&self) -> Value {
    let project = &self.settings.places.project;
    let uri = url::Url::from_directory_path(project).map(String::from).unwrap_or_default();
    let name =
      project.file_name().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
    json!([{"uri": uri, "name": name}])
  }

  /// The directory of the images of the life.
  pub fn gallery(&self) -> Option<&std::path::Path> {
    self.settings.gallery.as_deref()
  }

  /// Every server ended: each connection closed, and each process of a server ended with every process it started.
  pub fn end(&self) {
    let held = std::mem::take(&mut *self.entries());
    drop(held);
  }
}

/// The dict of a server, as the act mcp_servers gives it. A tool that the config denies is not in it.
fn dict(entry: &Entry) -> Value {
  let server = &entry.server;
  let allowed: Vec<Value> = entry
    .offered
    .tools
    .iter()
    .filter(|tool| !server.deny.iter().any(|denied| tool["name"].as_str() == Some(denied)))
    .cloned()
    .collect();
  let text = |one: &Value, key: &str| one[key].as_str().unwrap_or_default().to_owned();
  let tools: Vec<Value> = allowed
    .iter()
    .map(|tool| {
      let title = tool["title"].as_str().or_else(|| tool["annotations"]["title"].as_str()).unwrap_or_default();
      json!({
        "name": text(tool, "name"),
        "title": title,
        "description": text(tool, "description"),
        "input": tool.get("inputSchema").cloned().unwrap_or_else(|| json!({"type": "object"})),
        "output": tool.get("outputSchema").cloned().unwrap_or(Value::Null),
        "annotations": tool.get("annotations").cloned().filter(Value::is_object).unwrap_or_else(|| json!({})),
      })
    })
    .collect();
  let resources: Vec<Value> = entry
    .offered
    .resources
    .iter()
    .map(|one| {
      json!({"uri": text(one, "uri"), "name": text(one, "name"), "description": text(one, "description"), "mime": text(one, "mimeType")})
    })
    .collect();
  let templates: Vec<Value> = entry
    .offered
    .templates
    .iter()
    .map(|one| json!({"uri": text(one, "uriTemplate"), "name": text(one, "name"), "description": text(one, "description")}))
    .collect();
  let prompts: Vec<Value> = entry
    .offered
    .prompts
    .iter()
    .map(|one| {
      let arguments: Vec<Value> = one["arguments"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|argument| {
          json!({
            "name": text(argument, "name"),
            "description": text(argument, "description"),
            "required": argument["required"].as_bool().unwrap_or(false),
          })
        })
        .collect();
      json!({"name": text(one, "name"), "description": text(one, "description"), "arguments": arguments})
    })
    .collect();
  let connected = entry.state == State::Connected;
  let word = if connected && !allowed.is_empty() {
    binding(&server.name, &allowed).word
  } else {
    String::new()
  };
  json!({
    "name": server.name,
    "scope": server.scope.name(),
    "transport": server.transport.kind(),
    "target": server.transport.target(),
    "state": entry.state.name(),
    "error": entry.error,
    "version": entry.info.version,
    "instructions": entry.info.instructions,
    "tools": tools,
    "resources": resources,
    "templates": templates,
    "prompts": prompts,
    "binding": word,
  })
}

/// The wait before a server connects again, after this many failures in a row.
fn backoff(failures: u32) -> Duration {
  Duration::from_secs(1u64 << failures.min(5)).min(Duration::from_secs(30))
}

/// The keeper of a server: it connects the server, keeps what the server offers as the server changes it, and
/// connects it again when its connection ends. A keeper whose entry changed generation ends.
async fn keep(hub: Arc<Hub>, name: String, generation: u64) {
  let (mut failures, mut quiet) = (0u32, true);
  loop {
    let Some(server) = hub
      .entries()
      .iter()
      .find(|one| one.server.name == name && one.generation == generation)
      .map(|one| one.server.clone())
    else {
      return;
    };
    let token = match server.transport.url() {
      Some(url) => oauth::token(hub.settings.vault.clone(), oauth::key(&name, url)).await,
      None => None,
    };
    let started = async {
      let (session, heard) = session::start(&server.transport, token, hub.roots(), CONNECT).await?;
      let offered = session::offered(&session, CONNECT).await?;
      Ok::<_, Failure>((session, heard, offered))
    };
    match started.await {
      Ok((session, mut heard, offered)) => {
        let session = Arc::new(session);
        let info = session.info.clone();
        let up = hub.set(&name, generation, |entry| {
          (entry.state, entry.session, entry.info, entry.offered) =
            (State::Connected, Some(session.clone()), info, offered);
          entry.error.clear();
        });
        if !up {
          return;
        }
        if !quiet {
          hub.tell(&name);
        }
        failures = 0;
        let why = loop {
          match heard.recv().await {
            Some(Heard::Notified(method)) => {
              let Some(changed) = Changed::of(&method) else { continue };
              let mut offered = hub
                .entries()
                .iter()
                .find(|one| one.generation == generation)
                .map(|one| one.offered.clone())
                .unwrap_or_default();
              if session::relisted(&session, changed, &mut offered, CONNECT).await.is_ok()
                && hub.set(&name, generation, |entry| entry.offered = offered)
              {
                hub.tell(&name);
              }
            }
            Some(Heard::Closed(why)) => break why,
            None => break "the connection closed".to_owned(),
          }
        };
        drop(session);
        let wait = backoff(0);
        let said = format!("{why}; furb connects again in {} s", wait.as_secs());
        if !hub.set(&name, generation, |entry| {
          (entry.state, entry.error, entry.session) = (State::Failed, said, None)
        }) {
          return;
        }
        hub.tell(&name);
        quiet = false;
        tokio::time::sleep(wait).await;
      }
      Err(Failure::Unauthorized(hint)) => {
        let said = format!("the server asks for a login: mcp_control('login', {name:?})");
        hub.set(&name, generation, |entry| {
          (entry.state, entry.error, entry.hint) = (State::NeedsAuth, said, hint)
        });
        if !quiet {
          hub.tell(&name);
        }
        return;
      }
      Err(no) => {
        let said = no.said();
        if quiet {
          hub.set(&name, generation, |entry| (entry.state, entry.error) = (State::Failed, said));
          return;
        }
        failures += 1;
        if failures >= RETRIES {
          let said = format!(
            "{said}; furb gave up after {RETRIES} tries: mcp_control('reconnect', {name:?})"
          );
          hub.set(&name, generation, |entry| (entry.state, entry.error) = (State::Failed, said));
          hub.tell(&name);
          return;
        }
        let wait = backoff(failures);
        let said = format!("{said}; furb connects again in {} s", wait.as_secs());
        if !hub.set(&name, generation, |entry| (entry.state, entry.error) = (State::Failed, said)) {
          return;
        }
        hub.tell(&name);
        tokio::time::sleep(wait).await;
      }
    }
  }
}
