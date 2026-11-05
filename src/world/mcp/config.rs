//! The config of the servers of MCP: three scopes, the expansion of the variables in a server, the approval of the
//! servers of the project, and the choices of the operator that a control writes to the local scope.
//!
//! Each scope is a file `{"mcpServers": {name: server}}`: the user scope in the config directory of the user, the
//! project scope in `.mcp.json` of the project, as Claude Code writes it, and the local scope in `.furb/mcp.json` of
//! the project. A later scope wins for a name, and the server keeps the place where a scope first named it. The local
//! scope holds the choices of the operator too: `"approved"` and `"approveAll"` approve the servers of the project
//! scope, `"disabled"` turns servers off, and `"enabled"` turns on a server whose definition says `"disabled": true`.

use std::{
  fmt, fs, io,
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};

use serde_json::{Map, Value};

use crate::world::disk::{self, Access};

/// How long a call of a tool may take when its server names no `"timeout"`.
pub(super) const CALL: Duration = Duration::from_secs(60);

/// How long a server may take to connect.
pub(super) const CONNECT: Duration = Duration::from_secs(30);

/// What a field of a definition was meant to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
  String,
  Map,
  List,
  Port,
  Seconds,
}

impl fmt::Display for Kind {
  fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
    out.write_str(match self {
      Kind::String => "string",
      Kind::Map => "map",
      Kind::List => "list",
      Kind::Port => "port",
      Kind::Seconds => "number of seconds above 0",
    })
  }
}

/// Why a config cannot be read or written, or a server cannot run as it is defined.
#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
  #[error("{}: {source}", path.display())]
  Read { path: PathBuf, source: io::Error },
  #[error("{}: {source}", path.display())]
  Parse { path: PathBuf, source: serde_json::Error },
  #[error("{}: the config is no map", .0.display())]
  NoMap(PathBuf),
  #[error("{}: mcpServers is no map", .0.display())]
  NoServers(PathBuf),
  #[error("{}: {source}", path.display())]
  Write { path: PathBuf, source: io::Error },
  #[error("{}: {source}", path.display())]
  Encode { path: PathBuf, source: serde_json::Error },
  #[error("the server is no map")]
  Server,
  #[error("{key} is no {kind}")]
  Shape { key: String, kind: Kind },
  #[error("args holds a value that is no string")]
  Arg,
  #[error("the server names no command")]
  NoCommand,
  #[error("the server names no url")]
  NoUrl,
  #[error("the url {url} is not valid: {source}")]
  Url { url: String, source: url::ParseError },
  #[error("the type {0} is none of stdio, http and sse")]
  Type(String),
  #[error("the variable {0} is not set")]
  Unset(String),
}

impl Error {
  fn shape(key: &str, kind: Kind) -> Error {
    Error::Shape { key: key.to_owned(), kind }
  }
}

/// Two errors are the same when they say the same, since no source of an error compares.
impl PartialEq for Error {
  fn eq(&self, other: &Error) -> bool {
    self.to_string() == other.to_string()
  }
}

/// Where a server is defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scope {
  User,
  Project,
  Local,
}

impl Scope {
  /// The name of the scope, as a server dict gives it.
  pub(super) fn name(self) -> &'static str {
    match self {
      Scope::User => "user",
      Scope::Project => "project",
      Scope::Local => "local",
    }
  }
}

/// The client that a server of HTTP wants for its login, when it registers none: its id and the port of the redirect.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Login {
  pub(super) client: Option<String>,
  pub(super) port: Option<u16>,
}

/// How furb reaches a server.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Transport {
  /// A process that furb starts, which reads requests on its stdin and writes answers on its stdout, one JSON a line.
  Stdio { command: String, args: Vec<String>, env: Vec<(String, String)>, cwd: Option<PathBuf> },
  /// A server of the streamable HTTP at an address.
  Http { url: String, headers: Vec<(String, String)>, oauth: Login },
  /// A server of the older HTTP with SSE at the address of its stream of events.
  Sse { url: String, headers: Vec<(String, String)>, oauth: Login },
}

impl Transport {
  /// The client of its login that the definition names; none for a server that runs on this machine.
  pub(super) fn oauth(&self) -> Login {
    match self {
      Transport::Http { oauth, .. } | Transport::Sse { oauth, .. } => oauth.clone(),
      Transport::Stdio { .. } => Login::default(),
    }
  }

  /// The kind of the transport, as a server dict gives it.
  pub(super) fn kind(&self) -> &'static str {
    match self {
      Transport::Stdio { .. } => "stdio",
      Transport::Http { .. } => "http",
      Transport::Sse { .. } => "sse",
    }
  }

  /// What the transport reaches: the command line of a process, or the address of a server.
  pub(super) fn target(&self) -> String {
    match self {
      Transport::Stdio { command, args, .. } => [command.as_str()]
        .into_iter()
        .chain(args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" "),
      Transport::Http { url, .. } | Transport::Sse { url, .. } => url.clone(),
    }
  }

  /// The address of a remote server, and none for a process.
  pub(super) fn url(&self) -> Option<&str> {
    match self {
      Transport::Stdio { .. } => None,
      Transport::Http { url, .. } | Transport::Sse { url, .. } => Some(url),
    }
  }
}

/// How a server that can run runs.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Run {
  pub(super) transport: Transport,
  /// How long a call of one of its tools may take.
  pub(super) timeout: Duration,
  /// The names of its tools that no chain may call.
  pub(super) deny: Vec<String>,
}

/// One server as the configs define it, after the choices of the operator.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Server {
  pub(super) name: String,
  pub(super) scope: Scope,
  /// Whether the server is off.
  pub(super) disabled: bool,
  /// Whether the server may run: a server of the project scope runs only once the operator approved it.
  pub(super) approved: bool,
  /// How the server runs, or why it cannot as it is defined, as a variable that is not set.
  pub(super) run: Result<Run, Arc<Error>>,
}

impl Server {
  /// A server that cannot run, under a name, with why.
  fn broken(name: String, scope: Scope, why: Error) -> Server {
    Server { name, scope, disabled: false, approved: true, run: Err(Arc::new(why)) }
  }

  /// The client of its login that the definition names.
  pub(super) fn oauth(&self) -> Login {
    self.run.as_ref().map(|run| run.transport.oauth()).unwrap_or_default()
  }

  /// The address of a remote server that can run.
  pub(super) fn url(&self) -> Option<&str> {
    self.run.as_ref().ok().and_then(|run| run.transport.url())
  }
}

/// The files of the three scopes.
#[derive(Clone, Debug)]
pub struct Places {
  /// The config directory of the user.
  pub config: PathBuf,
  /// The directory of the project.
  pub project: PathBuf,
}

impl Places {
  /// The file of a scope.
  pub(super) fn file(&self, scope: Scope) -> PathBuf {
    match scope {
      Scope::User => self.config.join("mcp.json"),
      Scope::Project => self.project.join(".mcp.json"),
      Scope::Local => self.project.join(".furb").join("mcp.json"),
    }
  }
}

/// A choice of the operator about one server, which a control writes to the local scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Choice {
  Enable,
  Disable,
  Approve,
}

/// The value of an environment variable, or none when it is not set.
pub(super) type Variables<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The choices of the operator that the local scope holds.
struct Choices {
  approved: Vec<String>,
  disabled: Vec<String>,
  enabled: Vec<String>,
  all: bool,
}

impl Choices {
  fn of(local: &Result<Option<Map<String, Value>>, Error>) -> Choices {
    let empty = Map::new();
    let held = local.as_ref().ok().and_then(Option::as_ref).unwrap_or(&empty);
    let listed = |key: &str| -> Vec<String> {
      let names = held.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
      names.iter().filter_map(Value::as_str).map(str::to_owned).collect()
    };
    Choices {
      approved: listed("approved"),
      disabled: listed("disabled"),
      enabled: listed("enabled"),
      all: held.get("approveAll").and_then(Value::as_bool) == Some(true),
    }
  }

  /// A server of a scope. The operator can turn it off or approve it.
  /// When he chose nothing, its definition decides.
  fn judged(&self, mut server: Server, off: bool) -> Server {
    server.disabled =
      self.disabled.contains(&server.name) || (off && !self.enabled.contains(&server.name));
    server.approved =
      server.scope != Scope::Project || self.all || self.approved.contains(&server.name);
    server
  }
}

/// The servers that the configs define, in the order they first name them, and, as a server with the path of its file
/// as its name, each file that is not of the form, whose error says why.
pub(super) fn servers(places: &Places, variables: Variables<'_>) -> Vec<Server> {
  let files = [Scope::User, Scope::Project, Scope::Local]
    .map(|scope| (scope, places.file(scope), read(&places.file(scope))));
  let choices = Choices::of(&files[2].2);
  let mut found: Vec<Server> = Vec::new();
  for (scope, path, held) in files {
    let name = path.display().to_string();
    let held = match held {
      Ok(Some(held)) => held,
      Ok(None) => continue,
      Err(why) => {
        found.push(Server::broken(name, scope, why));
        continue;
      }
    };
    let defined = match held.get("mcpServers") {
      None => continue,
      Some(Value::Object(defined)) => defined,
      Some(_) => {
        found.push(Server::broken(name, scope, Error::NoServers(path)));
        continue;
      }
    };
    for (name, one) in defined {
      let run = defined_run(one, &places.project, variables).map_err(Arc::new);
      let server = Server { name: name.clone(), scope, disabled: false, approved: true, run };
      let off = one.get("disabled").and_then(Value::as_bool) == Some(true);
      placed(&mut found, choices.judged(server, off));
    }
  }
  found
}

/// A server in the list: in the place of the one of its name, or last.
fn placed(found: &mut Vec<Server>, server: Server) {
  match found.iter_mut().find(|held| held.name == server.name) {
    Some(held) => *held = server,
    None => found.push(server),
  }
}

/// A choice of the operator about a server, written to the local scope with every other key of that file kept.
pub(super) fn choose(places: &Places, choice: Choice, name: &str) -> Result<(), Error> {
  let path = places.file(Scope::Local);
  let mut held = read(&path)?.unwrap_or_default();
  match choice {
    Choice::Enable => {
      listed(&mut held, "disabled", name, false);
      listed(&mut held, "enabled", name, true);
    }
    Choice::Disable => {
      listed(&mut held, "enabled", name, false);
      listed(&mut held, "disabled", name, true);
    }
    Choice::Approve => listed(&mut held, "approved", name, true),
  }
  let text = serde_json::to_string_pretty(&Value::Object(held))
    .map_err(|source| Error::Encode { path: path.clone(), source })?;
  disk::write(&path, (text + "\n").as_bytes(), Access::Shared)
    .map_err(|no| Error::Write { path: no.path, source: no.source })
}

/// Adds a name to a list of the file once, or removes it. A key that holds no list gets a new list.
fn listed(held: &mut Map<String, Value>, key: &str, name: &str, add: bool) {
  let slot = held.entry(key.to_owned()).or_insert_with(|| Value::Array(Vec::new()));
  if !slot.is_array() {
    *slot = Value::Array(Vec::new());
  }
  if let Value::Array(list) = slot {
    list.retain(|one| one.as_str() != Some(name));
    if add {
      list.push(Value::String(name.to_owned()));
    }
  }
}

/// The map that a file of a scope holds, none when there is no file, or why it is not of the form.
fn read(path: &Path) -> Result<Option<Map<String, Value>>, Error> {
  let text = match fs::read_to_string(path) {
    Ok(text) => text,
    Err(no) if no.kind() == io::ErrorKind::NotFound => return Ok(None),
    Err(source) => return Err(Error::Read { path: path.to_owned(), source }),
  };
  match serde_json::from_str::<Value>(&text) {
    Ok(Value::Object(held)) => Ok(Some(held)),
    Ok(_) => Err(Error::NoMap(path.to_owned())),
    Err(source) => Err(Error::Parse { path: path.to_owned(), source }),
  }
}

/// How the server of a definition runs, with each variable expanded, or why it cannot.
fn defined_run(one: &Value, project: &Path, variables: Variables<'_>) -> Result<Run, Error> {
  let Value::Object(one) = one else { return Err(Error::Server) };
  let fields = Fields { one, variables };
  let transport = match fields.text("type")?.unwrap_or_default().as_str() {
    "" | "stdio" => fields.process(project)?,
    "http" | "streamable-http" => fields.remote(false)?,
    "sse" => fields.remote(true)?,
    other => return Err(Error::Type(other.to_owned())),
  };
  Ok(Run { transport, timeout: fields.timeout()?, deny: fields.deny()? })
}

/// The fields of a definition, read with each variable expanded.
struct Fields<'a> {
  one: &'a Map<String, Value>,
  variables: Variables<'a>,
}

impl Fields<'_> {
  fn text(&self, key: &str) -> Result<Option<String>, Error> {
    match self.one.get(key) {
      None | Some(Value::Null) => Ok(None),
      Some(Value::String(said)) => expand(said, self.variables).map(Some),
      Some(_) => Err(Error::shape(key, Kind::String)),
    }
  }

  fn pairs(&self, key: &str) -> Result<Vec<(String, String)>, Error> {
    let Some(held) = self.one.get(key).filter(|held| !held.is_null()) else {
      return Ok(Vec::new());
    };
    let Value::Object(held) = held else { return Err(Error::shape(key, Kind::Map)) };
    held
      .iter()
      .map(|(name, value)| {
        let value = value
          .as_str()
          .ok_or_else(|| Error::Shape { key: format!("{key}.{name}"), kind: Kind::String })?;
        Ok((name.clone(), expand(value, self.variables)?))
      })
      .collect()
  }

  fn args(&self) -> Result<Vec<String>, Error> {
    match self.one.get("args") {
      None | Some(Value::Null) => Ok(Vec::new()),
      Some(Value::Array(args)) => {
        args.iter().map(|arg| expand(arg.as_str().ok_or(Error::Arg)?, self.variables)).collect()
      }
      Some(_) => Err(Error::shape("args", Kind::List)),
    }
  }

  /// A process that furb starts.
  fn process(&self, project: &Path) -> Result<Transport, Error> {
    let command =
      self.text("command")?.filter(|command| !command.is_empty()).ok_or(Error::NoCommand)?;
    let args = self.args()?;
    let cwd = self.text("cwd")?.map(|cwd| crate::world::resolved(project, &cwd));
    Ok(Transport::Stdio { command, args, env: self.pairs("env")?, cwd })
  }

  /// A server at an address, of the older HTTP with SSE or of the streamable HTTP.
  fn remote(&self, sse: bool) -> Result<Transport, Error> {
    let url = self.text("url")?.filter(|url| !url.is_empty()).ok_or(Error::NoUrl)?;
    url::Url::parse(&url).map_err(|source| Error::Url { url: url.clone(), source })?;
    let headers = self.pairs("headers")?;
    let oauth = self.login()?;
    Ok(if sse {
      Transport::Sse { url, headers, oauth }
    } else {
      Transport::Http { url, headers, oauth }
    })
  }

  fn login(&self) -> Result<Login, Error> {
    match self.one.get("oauth") {
      None | Some(Value::Null) => Ok(Login::default()),
      Some(Value::Object(held)) => Ok(Login {
        client: held.get("clientId").and_then(Value::as_str).map(str::to_owned),
        port: match held.get("callbackPort") {
          None | Some(Value::Null) => None,
          Some(port) => Some(
            port
              .as_u64()
              .and_then(|p| u16::try_from(p).ok())
              .filter(|p| *p > 0)
              .ok_or_else(|| Error::shape("oauth.callbackPort", Kind::Port))?,
          ),
        },
      }),
      Some(_) => Err(Error::shape("oauth", Kind::Map)),
    }
  }

  fn timeout(&self) -> Result<Duration, Error> {
    match self.one.get("timeout") {
      None | Some(Value::Null) => Ok(CALL),
      Some(seconds) => seconds
        .as_f64()
        .filter(|seconds| *seconds > 0.0)
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .ok_or_else(|| Error::shape("timeout", Kind::Seconds)),
    }
  }

  fn deny(&self) -> Result<Vec<String>, Error> {
    match self.one.get("deny") {
      None | Some(Value::Null) => Ok(Vec::new()),
      Some(Value::Array(names)) => {
        Ok(names.iter().filter_map(Value::as_str).map(str::to_owned).collect())
      }
      Some(_) => Err(Error::shape("deny", Kind::List)),
    }
  }
}

/// A text with each `${VAR}` replaced by the value of the variable, and each `${VAR:-default}` by the default when
/// the variable is not set or is empty; or why it cannot be, as a variable that is not set and has no default.
pub(super) fn expand(text: &str, variables: Variables<'_>) -> Result<String, Error> {
  let mut out = String::new();
  let mut rest = text;
  while let Some(at) = rest.find("${") {
    out.push_str(&rest[..at]);
    let inside = &rest[at + 2..];
    let Some(end) = inside.find('}') else {
      out.push_str(&rest[at..]);
      return Ok(out);
    };
    let (name, default) = match inside[..end].split_once(":-") {
      Some((name, default)) => (name, Some(default)),
      None => (&inside[..end], None),
    };
    match (variables(name).filter(|value| !value.is_empty() || default.is_none()), default) {
      (Some(value), _) => out.push_str(&value),
      (None, Some(default)) => out.push_str(default),
      (None, None) => return Err(Error::Unset(name.to_owned())),
    }
    rest = &inside[end + 1..];
  }
  out.push_str(rest);
  Ok(out)
}

#[cfg(test)]
#[path = "config.test.rs"]
mod test;
