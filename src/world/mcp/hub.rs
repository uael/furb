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
  Error,
  binding::binding,
  config::{self, CONNECT, Choice, Places, Run, Server},
  oauth::{self, Opener, Vault, VaultError},
  session::{self, Changed, Info, Offered, Session},
  wire::{Failure, Heard},
};
use crate::ear::runtime;

/// How many times in a row a server whose connection ended may fail to connect again.
const RETRIES: u32 = 5;

/// The value of each variable of the environment, or none when it is not set.
pub(super) type Environment = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

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

/// The link of furb to a server, which holds what each state needs: the session while it is
/// connected, why it failed, and the metadata of the resource that the challenge of a login named.
enum Link {
  Pending,
  Connected { session: Arc<Session> },
  Failed(String),
  NeedsAuth { hint: Option<String> },
  Disabled,
  Unapproved,
}

impl Link {
  /// The link of a server that the configs define, before any connect: failed when its definition
  /// is broken.
  fn defined(server: &Server) -> Link {
    match &server.run {
      Err(why) => Link::Failed(why.to_string()),
      Ok(_) if server.disabled => Link::Disabled,
      Ok(_) if !server.approved => Link::Unapproved,
      Ok(_) => Link::Pending,
    }
  }

  /// The name of the state, as a server dict gives it.
  fn name(&self) -> &'static str {
    match self {
      Link::Pending => "pending",
      Link::Connected { .. } => "connected",
      Link::Failed(_) => "failed",
      Link::NeedsAuth { .. } => "needs-auth",
      Link::Disabled => "disabled",
      Link::Unapproved => "unapproved",
    }
  }

  /// Why the server of this name is in the state, as a server dict gives it, or none.
  fn why(&self, name: &str) -> Option<String> {
    match self {
      Link::Failed(why) => Some(why.clone()),
      Link::NeedsAuth { .. } => {
        Some(format!("the server asks for a login: mcp_control('login', {name:?})"))
      }
      Link::Pending | Link::Connected { .. } | Link::Disabled | Link::Unapproved => None,
    }
  }
}

/// An action of the operator on a server.
#[derive(Clone, Copy)]
enum Action {
  Reconnect,
  Enable,
  Disable,
  Approve,
  Login,
  Logout,
}

impl Action {
  /// The action of this name, or why there is none.
  fn of(name: &str) -> Result<Action, Error> {
    match name {
      "reconnect" => Ok(Action::Reconnect),
      "enable" => Ok(Action::Enable),
      "disable" => Ok(Action::Disable),
      "approve" => Ok(Action::Approve),
      "login" => Ok(Action::Login),
      "logout" => Ok(Action::Logout),
      other => Err(Error::Action(other.to_owned())),
    }
  }
}

/// One server of the life: its definition, its link, what it said of itself and offers, the
/// generation of its keeper, and the keeper, which ends with the entry.
struct Entry {
  server: Server,
  link: Link,
  info: Info,
  offered: Offered,
  /// Why the store kept no tokens that were refreshed for the connection, which serve all the same.
  unsaved: Option<VaultError>,
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
pub(super) struct Hub {
  settings: Settings,
  entries: Mutex<Vec<Entry>>,
  loaded: Mutex<bool>,
  moved: Notify,
  told: mpsc::UnboundedSender<String>,
  generations: AtomicU64,
}

impl Hub {
  /// The servers of a life, none loaded yet, which tell each change that no question asked for to `told`.
  pub(super) fn new(settings: Settings, told: mpsc::UnboundedSender<String>) -> Arc<Hub> {
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
  pub(super) fn reload(self: &Arc<Self>) {
    *self.loaded.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = true;
    let variables = Arc::clone(&self.settings.variables);
    let servers = config::servers(&self.settings.places, &|name| variables(name));
    let mut entries = self.entries();
    let mut held: Vec<Entry> = std::mem::take(&mut *entries);
    for server in servers {
      let kept = held.iter().position(|one| one.server == server).map(|at| held.remove(at));
      let entry = kept.unwrap_or_else(|| {
        let link = Link::defined(&server);
        let mut entry = Entry {
          server,
          link,
          info: Info::default(),
          offered: Offered::default(),
          unsaved: None,
          generation: 0,
          keeper: None,
        };
        if matches!(entry.link, Link::Pending) {
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
    entry.link = Link::Pending;
    if let Some(keeper) = entry.keeper.take() {
      keeper.abort();
    }
    let task = runtime().spawn(keep(Arc::clone(self), entry.server.name.clone(), generation));
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

  /// A change of a server that no question asked for, told. An ear that ended hears no more.
  fn tell(&self, name: &str) {
    self.told.send(name.to_owned()).ok();
  }

  /// Once no server of these names, or no server at all when it names none, is pending, or the bound passed.
  async fn settled(&self, names: &[&str], bound: Duration) {
    let deadline = Instant::now() + bound;
    loop {
      let moved = self.moved.notified();
      tokio::pin!(moved);
      moved.as_mut().enable();
      let pending = self.entries().iter().any(|one| {
        matches!(one.link, Link::Pending)
          && (names.is_empty() || names.contains(&one.server.name.as_str()))
      });
      if !pending || tokio::time::timeout_at(deadline, moved).await.is_err() {
        return;
      }
    }
  }

  /// The dict of each server, once each server that runs connected or failed, within the bound of a connect.
  pub(super) async fn listed(self: &Arc<Self>) -> Vec<Value> {
    self.reload();
    self.settled(&[], CONNECT + Duration::from_secs(5)).await;
    self.entries().iter().map(dict).collect()
  }

  /// The dict of each server, read again from the configs, when no server is pending; none while one is.
  pub(super) fn settled_now(self: &Arc<Self>) -> Option<Vec<Value>> {
    self.reload();
    let entries = self.entries();
    if entries.iter().any(|one| matches!(one.link, Link::Pending)) {
      None
    } else {
      Some(entries.iter().map(dict).collect())
    }
  }

  /// The dict of a server.
  fn dict(&self, name: &str) -> Option<Value> {
    self.entries().iter().find(|one| one.server.name == name).map(dict)
  }

  /// The session of a server and its definition, once it settled, or why it has none.
  pub(super) async fn session(self: &Arc<Self>, name: &str) -> Result<(Arc<Session>, Run), Error> {
    self.loaded();
    self.settled(&[name], CONNECT + Duration::from_secs(5)).await;
    let entries = self.entries();
    let entry = entries
      .iter()
      .find(|one| one.server.name == name)
      .ok_or_else(|| Error::NoServer(name.to_owned()))?;
    match &entry.link {
      Link::Connected { session } => match &entry.server.run {
        Ok(run) => Ok((Arc::clone(session), run.clone())),
        Err(why) => Err(Error::Broken { name: name.to_owned(), source: Arc::clone(why) }),
      },
      Link::NeedsAuth { .. } => Err(Error::Login(name.to_owned())),
      other => Err(Error::Down {
        name: name.to_owned(),
        state: other.name(),
        why: match other {
          Link::Failed(why) => Some(why.clone()),
          _ => None,
        },
      }),
    }
  }

  /// A server whose request answered 401: its tokens refreshed and the server connected again, and whether it is
  /// connected then. A server whose tokens cannot be refreshed needs a login, which it tells.
  pub(super) async fn renewed(self: &Arc<Self>, name: &str, hint: Option<String>) -> bool {
    let url = self
      .entries()
      .iter()
      .find(|one| one.server.name == name)
      .and_then(|one| one.server.url().map(str::to_owned));
    let Some(url) = url else { return false };
    let (vault, key) = (Arc::clone(&self.settings.vault), oauth::key(name, &url));
    let held = {
      let (vault, key) = (Arc::clone(&vault), key.clone());
      oauth::blocking(move || oauth::stored(vault.as_ref(), &key)).await.ok().flatten()
    };
    let fresh = match held {
      Some(held) => oauth::refreshed(&held).await.ok(),
      None => None,
    };
    let Some(fresh) = fresh else { return self.relinked(name, Link::NeedsAuth { hint }) };
    let kept = match oauth::blocking(move || oauth::store(vault.as_ref(), &key, &fresh)).await {
      Ok(stored) => stored.map_err(Error::from),
      Err(no) => Err(Error::from(no)),
    };
    if let Err(no) = kept {
      let said = format!("the refreshed tokens were not kept: {no}");
      return self.relinked(name, Link::Failed(said));
    }
    self.restart(name).is_ok() && {
      self.settled(&[name], CONNECT + Duration::from_secs(5)).await;
      self
        .entries()
        .iter()
        .any(|one| one.server.name == name && matches!(one.link, Link::Connected { .. }))
    }
  }

  /// The link of a server changed, and told; always false, since the server is not connected then.
  fn relinked(&self, name: &str, link: Link) -> bool {
    let mut entries = self.entries();
    if let Some(entry) = entries.iter_mut().find(|one| one.server.name == name) {
      entry.link = link;
    }
    drop(entries);
    self.moved.notify_waiters();
    self.tell(name);
    false
  }

  /// A server that may run, connected again from the start.
  fn restart(self: &Arc<Self>, name: &str) -> Result<(), Error> {
    let mut entries = self.entries();
    let entry = entries
      .iter_mut()
      .find(|one| one.server.name == name)
      .ok_or_else(|| Error::NoServer(name.to_owned()))?;
    let name = name.to_owned();
    if let Err(why) = &entry.server.run {
      return Err(Error::Broken { name, source: Arc::clone(why) });
    }
    match () {
      () if entry.server.disabled => Err(Error::Disabled(name)),
      () if !entry.server.approved => Err(Error::Unapproved(name)),
      () => {
        self.launch(entry);
        Ok(())
      }
    }
  }

  /// What a control of the operator does to a server, and its dict after it, once it settled.
  pub(super) async fn control(self: &Arc<Self>, action: &str, name: &str) -> Result<Value, Error> {
    self.loaded();
    let found = self.entries().iter().find(|one| one.server.name == name).map(|one| {
      let hint = match &one.link {
        Link::NeedsAuth { hint } => hint.clone(),
        _ => None,
      };
      (one.server.clone(), hint)
    });
    let (server, hint) = found.ok_or_else(|| Error::NoServer(name.to_owned()))?;
    match Action::of(action)? {
      Action::Reconnect => self.restart(name)?,
      Action::Enable => self.chose(Choice::Enable, name)?,
      Action::Disable => self.chose(Choice::Disable, name)?,
      Action::Approve => self.chose(Choice::Approve, name)?,
      Action::Login => {
        let (url, vault, key) = self.door(name, &server)?;
        let tokens =
          oauth::login(url, hint.as_deref(), &self.settings.opener, &server.oauth()).await?;
        oauth::blocking(move || oauth::store(vault.as_ref(), &key, &tokens)).await??;
        self.restart(name)?;
      }
      Action::Logout => {
        let (_, vault, key) = self.door(name, &server)?;
        oauth::blocking(move || vault.forget(&key)).await??;
        self.restart(name)?;
      }
    }
    self.settled(&[name], CONNECT + Duration::from_secs(5)).await;
    self.dict(name).ok_or_else(|| Error::NoServer(name.to_owned()))
  }

  /// The address of a remote server, the store of its tokens and their key, or why a server on this
  /// machine has none.
  fn door<'a>(
    &self,
    name: &str,
    server: &'a Server,
  ) -> Result<(&'a str, Arc<dyn Vault>, String), Error> {
    let url = server.url().ok_or_else(|| Error::Local(name.to_owned()))?;
    Ok((url, Arc::clone(&self.settings.vault), oauth::key(name, url)))
  }

  /// A choice of the operator, written to the local scope, and the servers read again.
  fn chose(self: &Arc<Self>, choice: Choice, name: &str) -> Result<(), Error> {
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
  pub(super) fn gallery(&self) -> Option<&std::path::Path> {
    self.settings.gallery.as_deref()
  }

  /// Every server ended: each connection closed, and each process of a server ended with every process it started.
  pub(super) fn end(&self) {
    let held = std::mem::take(&mut *self.entries());
    drop(held);
  }
}

/// A text of a JSON value, or an empty text.
fn text(one: &Value, key: &str) -> String {
  one[key].as_str().unwrap_or_default().to_owned()
}

/// The dict of each tool of a server that its config allows.
fn tools(allowed: &[Value]) -> Vec<Value> {
  allowed
    .iter()
    .map(|tool| {
      let title = tool["title"]
        .as_str()
        .or_else(|| tool["annotations"]["title"].as_str())
        .unwrap_or_default();
      json!({
        "name": text(tool, "name"),
        "title": title,
        "description": text(tool, "description"),
        "input": tool.get("inputSchema").cloned().unwrap_or_else(|| json!({"type": "object"})),
        "output": tool.get("outputSchema").cloned().unwrap_or(Value::Null),
        "annotations": tool
          .get("annotations")
          .cloned()
          .filter(Value::is_object)
          .unwrap_or_else(|| json!({})),
      })
    })
    .collect()
}

/// The dict of each prompt of a server.
fn prompts(offered: &[Value]) -> Vec<Value> {
  offered
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
    .collect()
}

/// The dict of a server, as the act `mcp_servers` gives it. A tool that the config denies is not
/// in it.
fn dict(entry: &Entry) -> Value {
  let server = &entry.server;
  let deny = server.run.as_ref().map(|run| run.deny.as_slice()).unwrap_or_default();
  let (kind, target) = server
    .run
    .as_ref()
    .map_or(("", String::new()), |run| (run.transport.kind(), run.transport.target()));
  let allowed: Vec<Value> = entry
    .offered
    .tools
    .iter()
    .filter(|tool| !deny.iter().any(|denied| tool["name"].as_str() == Some(denied)))
    .cloned()
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
  let connected = matches!(entry.link, Link::Connected { .. });
  let word = if connected && !allowed.is_empty() {
    binding(&server.name, &allowed).word
  } else {
    String::new()
  };
  json!({
    "name": server.name,
    "scope": server.scope.name(),
    "transport": kind,
    "target": target,
    "state": entry.link.name(),
    "error": entry
      .link
      .why(&server.name)
      .or_else(|| {
        entry.unsaved.as_ref().map(|no| format!("the refreshed tokens were not kept: {no}"))
      })
      .unwrap_or_default(),
    "version": entry.info.version,
    "instructions": entry.info.instructions,
    "tools": tools(&allowed),
    "resources": resources,
    "templates": templates,
    "prompts": prompts(&entry.offered.prompts),
    "binding": word,
  })
}

/// The wait before a server connects again, after this many failures in a row.
fn backoff(failures: u32) -> Duration {
  Duration::from_secs(1u64 << failures.min(5)).min(Duration::from_secs(30))
}

/// A server connected: its session, what it hears of its own accord, and what it offers.
struct Connected {
  session: Session,
  heard: mpsc::UnboundedReceiver<Heard>,
  offered: Offered,
  unsaved: Option<VaultError>,
}

/// A server connected: its session, started with the token of its login.
async fn connected(hub: &Hub, name: &str, run: &Run) -> Result<Connected, Failure> {
  let access = match run.transport.url() {
    Some(url) => oauth::token(Arc::clone(&hub.settings.vault), oauth::key(name, url)).await,
    None => None,
  };
  let (token, unsaved) = access.map_or((None, None), |one| (Some(one.token), one.unsaved));
  let (session, heard) = session::start(&run.transport, token, hub.roots(), CONNECT).await?;
  let offered = session::offered(&session, CONNECT).await?;
  Ok(Connected { session, heard, offered, unsaved })
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
    let Ok(run) = &server.run else { return };
    let wait = match connected(&hub, &name, run).await {
      Ok(Connected { session, heard, offered, unsaved }) => {
        let info = session.info.clone();
        let session = Arc::new(session);
        if !hub.set(&name, generation, |entry| {
          (entry.info, entry.offered, entry.unsaved) = (info, offered, unsaved);
          entry.link = Link::Connected { session: Arc::clone(&session) };
        }) {
          return;
        }
        if !quiet {
          hub.tell(&name);
        }
        failures = 0;
        let why = heeded(&hub, &name, generation, &session, heard).await;
        drop(session);
        let wait = backoff(0);
        let said = format!("{why}; furb connects again in {} s", wait.as_secs());
        if !hub.set(&name, generation, |entry| entry.link = Link::Failed(said)) {
          return;
        }
        hub.tell(&name);
        quiet = false;
        wait
      }
      Err(Failure::Unauthorized(hint)) => {
        hub.set(&name, generation, |entry| entry.link = Link::NeedsAuth { hint });
        if !quiet {
          hub.tell(&name);
        }
        return;
      }
      Err(no) => match failed(&hub, &name, generation, &no.said(), quiet, &mut failures) {
        Some(wait) => wait,
        None => return,
      },
    };
    tokio::time::sleep(wait).await;
  }
}

/// What a connected server says of its own accord, heeded until its connection ends, and why it
/// ended: each list that it says has changed is listed again.
async fn heeded(
  hub: &Hub,
  name: &str,
  generation: u64,
  session: &Session,
  mut heard: mpsc::UnboundedReceiver<Heard>,
) -> String {
  loop {
    match heard.recv().await {
      Some(Heard::Notified(method)) => {
        let Some(changed) = Changed::of(&method) else { continue };
        let mut offered = hub
          .entries()
          .iter()
          .find(|one| one.generation == generation)
          .map(|one| one.offered.clone())
          .unwrap_or_default();
        if session::relisted(session, changed, &mut offered, CONNECT).await.is_ok()
          && hub.set(name, generation, |entry| entry.offered = offered)
        {
          hub.tell(name);
        }
      }
      Some(Heard::Closed(why)) => return why,
      None => return "the connection closed".to_owned(),
    }
  }
}

/// A connect that failed, kept as the state of its server, and the wait before the next one, or
/// none when the keeper gives up: at once on the first connect, and after the last of the retries
/// of a connection that ended.
fn failed(
  hub: &Hub,
  name: &str,
  generation: u64,
  said: &str,
  quiet: bool,
  failures: &mut u32,
) -> Option<Duration> {
  if quiet {
    hub.set(name, generation, |entry| entry.link = Link::Failed(said.to_owned()));
    return None;
  }
  *failures += 1;
  if *failures >= RETRIES {
    let said =
      format!("{said}; furb gave up after {RETRIES} tries: mcp_control('reconnect', {name:?})");
    hub.set(name, generation, |entry| entry.link = Link::Failed(said));
    hub.tell(name);
    return None;
  }
  let wait = backoff(*failures);
  let said = format!("{said}; furb connects again in {} s", wait.as_secs());
  if !hub.set(name, generation, |entry| entry.link = Link::Failed(said)) {
    return None;
  }
  hub.tell(name);
  Some(wait)
}
