//! The config of the servers of MCP: three scopes, the expansion of the variables in a server, the approval of the
//! servers of the project, and the choices of the operator that a control writes to the local scope.
//!
//! Each scope is a file `{"mcpServers": {name: server}}`: the user scope in the config directory of the user, the
//! project scope in `.mcp.json` of the project, as Claude Code writes it, and the local scope in `.furb/mcp.json` of
//! the project. A later scope wins for a name, and the server keeps the place where a scope first named it. The local
//! scope holds the choices of the operator too: `"approved"` and `"approveAll"` approve the servers of the project
//! scope, `"disabled"` turns servers off, and `"enabled"` turns on a server whose definition says `"disabled": true`.

use std::{
  fs,
  path::{Path, PathBuf},
  time::Duration,
};

use serde_json::{Map, Value};

/// How long a call of a tool may take when its server names no `"timeout"`.
pub const CALL: Duration = Duration::from_secs(60);

/// How long a server may take to connect.
pub const CONNECT: Duration = Duration::from_secs(30);

/// Where a server is defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
  User,
  Project,
  Local,
}

impl Scope {
  /// The name of the scope, as a server dict gives it.
  pub fn name(self) -> &'static str {
    match self {
      Scope::User => "user",
      Scope::Project => "project",
      Scope::Local => "local",
    }
  }
}

/// How furb reaches a server.
#[derive(Clone, Debug, PartialEq)]
pub enum Transport {
  /// A process that furb starts, which reads requests on its stdin and writes answers on its stdout, one JSON a line.
  Stdio { command: String, args: Vec<String>, env: Vec<(String, String)>, cwd: Option<PathBuf> },
  /// A server of the streamable HTTP at an address.
  Http { url: String, headers: Vec<(String, String)> },
  /// A server of the older HTTP with SSE at the address of its stream of events.
  Sse { url: String, headers: Vec<(String, String)> },
}

impl Transport {
  /// The kind of the transport, as a server dict gives it.
  pub fn kind(&self) -> &'static str {
    match self {
      Transport::Stdio { .. } => "stdio",
      Transport::Http { .. } => "http",
      Transport::Sse { .. } => "sse",
    }
  }

  /// What the transport reaches: the command line of a process, or the address of a server.
  pub fn target(&self) -> String {
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
  pub fn url(&self) -> Option<&str> {
    match self {
      Transport::Stdio { .. } => None,
      Transport::Http { url, .. } | Transport::Sse { url, .. } => Some(url),
    }
  }
}

/// One server as the configs define it, after the choices of the operator.
#[derive(Clone, Debug, PartialEq)]
pub struct Server {
  pub name: String,
  pub scope: Scope,
  pub transport: Transport,
  /// Whether the server is off.
  pub disabled: bool,
  /// Whether the server may run: a server of the project scope runs only once the operator approved it.
  pub approved: bool,
  /// How long a call of one of its tools may take.
  pub timeout: Duration,
  /// The names of its tools that no chain may call.
  pub deny: Vec<String>,
  /// Why the server cannot run as it is defined, as a variable that is not set; empty when it can run.
  pub error: String,
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
  pub fn file(&self, scope: Scope) -> PathBuf {
    match scope {
      Scope::User => self.config.join("mcp.json"),
      Scope::Project => self.project.join(".mcp.json"),
      Scope::Local => self.project.join(".furb").join("mcp.json"),
    }
  }
}

/// A choice of the operator about one server, which a control writes to the local scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
  Enable,
  Disable,
  Approve,
}

/// The value of an environment variable, or none when it is not set.
pub type Variables<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The servers that the configs define, in the order they first name them, and, as a server with the path of its file
/// as its name, each file that is not of the form, whose error says why.
pub fn servers(places: &Places, variables: Variables<'_>) -> Vec<Server> {
  let mut found: Vec<Server> = Vec::new();
  let local = read(&places.file(Scope::Local));
  let choices = local.as_ref().ok().cloned().flatten().unwrap_or_default();
  let listed = |key: &str| -> Vec<String> {
    let names = choices.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    names.iter().filter_map(Value::as_str).map(str::to_owned).collect()
  };
  let (approved, disabled, enabled) = (listed("approved"), listed("disabled"), listed("enabled"));
  let all = choices.get("approveAll").and_then(Value::as_bool) == Some(true);
  for scope in [Scope::User, Scope::Project, Scope::Local] {
    let path = places.file(scope);
    let held = match if scope == Scope::Local { local.clone() } else { read(&path) } {
      Ok(Some(held)) => held,
      Ok(None) => continue,
      Err(why) => {
        found.push(broken(path.display().to_string(), scope, why));
        continue;
      }
    };
    let Some(defined) = held.get("mcpServers") else { continue };
    let Some(defined) = defined.as_object() else {
      found.push(broken(
        path.display().to_string(),
        scope,
        format!("{}: mcpServers is no map", path.display()),
      ));
      continue;
    };
    for (name, one) in defined {
      let mut server = server(name, scope, one, &places.project, variables);
      let off = one.get("disabled").and_then(Value::as_bool) == Some(true);
      server.disabled = disabled.contains(name) || (off && !enabled.contains(name));
      server.approved = scope != Scope::Project || all || approved.contains(name);
      match found.iter_mut().find(|held| held.name == *name) {
        Some(held) => *held = server,
        None => found.push(server),
      }
    }
  }
  found
}

/// A choice of the operator about a server, written to the local scope with every other key of that file kept.
pub fn choose(places: &Places, choice: Choice, name: &str) -> Result<(), String> {
  let path = places.file(Scope::Local);
  let mut held = read(&path)?.unwrap_or_default();
  let mut edit = |key: &str, add: bool| {
    let list = held.entry(key.to_owned()).or_insert_with(|| Value::Array(Vec::new()));
    if !list.is_array() {
      *list = Value::Array(Vec::new());
    }
    let list = list.as_array_mut().expect("a list");
    list.retain(|one| one.as_str() != Some(name));
    if add {
      list.push(Value::String(name.to_owned()));
    }
  };
  match choice {
    Choice::Enable => {
      edit("disabled", false);
      edit("enabled", true);
    }
    Choice::Disable => {
      edit("enabled", false);
      edit("disabled", true);
    }
    Choice::Approve => edit("approved", true),
  }
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|no| format!("{}: {no}", parent.display()))?;
  }
  let text = serde_json::to_string_pretty(&Value::Object(held)).expect("a map of JSON") + "\n";
  fs::write(&path, text).map_err(|no| format!("{}: {no}", path.display()))
}

/// The map that a file of a scope holds, none when there is no file, or why it is not of the form.
fn read(path: &Path) -> Result<Option<Map<String, Value>>, String> {
  let text = match fs::read_to_string(path) {
    Ok(text) => text,
    Err(no) if no.kind() == std::io::ErrorKind::NotFound => return Ok(None),
    Err(no) => return Err(format!("{}: {no}", path.display())),
  };
  match serde_json::from_str::<Value>(&text) {
    Ok(Value::Object(held)) => Ok(Some(held)),
    Ok(_) => Err(format!("{}: the config is no map", path.display())),
    Err(no) => Err(format!("{}: {no}", path.display())),
  }
}

/// A server that cannot run, under a name, with why.
fn broken(name: String, scope: Scope, error: String) -> Server {
  let transport =
    Transport::Stdio { command: String::new(), args: Vec::new(), env: Vec::new(), cwd: None };
  Server {
    name,
    scope,
    transport,
    disabled: false,
    approved: true,
    timeout: CALL,
    deny: Vec::new(),
    error,
  }
}

/// One server of a scope from its definition, with each variable expanded; a definition that cannot run has why in
/// its error.
fn server(
  name: &str,
  scope: Scope,
  one: &Value,
  project: &Path,
  variables: Variables<'_>,
) -> Server {
  match defined(one, project, variables) {
    Ok((transport, timeout, deny)) => Server {
      name: name.to_owned(),
      scope,
      transport,
      disabled: false,
      approved: true,
      timeout,
      deny,
      error: String::new(),
    },
    Err(why) => broken(name.to_owned(), scope, why),
  }
}

/// The transport, the timeout of a call and the denied tools of a definition, or why it cannot run.
fn defined(
  one: &Value,
  project: &Path,
  variables: Variables<'_>,
) -> Result<(Transport, Duration, Vec<String>), String> {
  let one = one.as_object().ok_or("the server is no map")?;
  let text = |key: &str| -> Result<Option<String>, String> {
    match one.get(key) {
      None | Some(Value::Null) => Ok(None),
      Some(Value::String(said)) => expand(said, variables).map(Some),
      Some(_) => Err(format!("{key} is no string")),
    }
  };
  let pairs = |key: &str| -> Result<Vec<(String, String)>, String> {
    let Some(held) = one.get(key).filter(|held| !held.is_null()) else { return Ok(Vec::new()) };
    let held = held.as_object().ok_or_else(|| format!("{key} is no map"))?;
    held
      .iter()
      .map(|(name, value)| {
        let value = value.as_str().ok_or_else(|| format!("{key}.{name} is no string"))?;
        Ok((name.clone(), expand(value, variables)?))
      })
      .collect()
  };
  let kind = text("type")?.unwrap_or_default();
  let transport = match kind.as_str() {
    "" | "stdio" => {
      let command = text("command")?
        .filter(|command| !command.is_empty())
        .ok_or("the server names no command")?;
      let args = match one.get("args") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(args)) => args
          .iter()
          .map(|arg| arg.as_str().ok_or_else(|| "args holds a value that is no string".to_owned()))
          .map(|arg| arg.and_then(|arg| expand(arg, variables)))
          .collect::<Result<_, _>>()?,
        Some(_) => return Err("args is no list".to_owned()),
      };
      let cwd = text("cwd")?.map(|cwd| crate::world::resolved(project, &cwd));
      Transport::Stdio { command, args, env: pairs("env")?, cwd }
    }
    "http" | "streamable-http" | "sse" => {
      let url = text("url")?.filter(|url| !url.is_empty()).ok_or("the server names no url")?;
      url::Url::parse(&url).map_err(|no| format!("the url {url} is not valid: {no}"))?;
      let headers = pairs("headers")?;
      if kind == "sse" { Transport::Sse { url, headers } } else { Transport::Http { url, headers } }
    }
    other => return Err(format!("the type {other} is none of stdio, http and sse")),
  };
  let timeout = match one.get("timeout") {
    None | Some(Value::Null) => CALL,
    Some(seconds) => seconds
      .as_f64()
      .filter(|seconds| *seconds > 0.0)
      .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
      .ok_or("timeout is no number of seconds above 0")?,
  };
  let deny = match one.get("deny") {
    None | Some(Value::Null) => Vec::new(),
    Some(Value::Array(names)) => {
      names.iter().filter_map(Value::as_str).map(str::to_owned).collect()
    }
    Some(_) => return Err("deny is no list".to_owned()),
  };
  Ok((transport, timeout, deny))
}

/// A text with each `${VAR}` replaced by the value of the variable, and each `${VAR:-default}` by the default when
/// the variable is not set or is empty; or why it cannot be, as a variable that is not set and has no default.
pub fn expand(text: &str, variables: Variables<'_>) -> Result<String, String> {
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
      (None, None) => return Err(format!("the variable {name} is not set")),
    }
    rest = &inside[end + 1..];
  }
  out.push_str(rest);
  Ok(out)
}

#[cfg(test)]
#[path = "config.test.rs"]
mod test;
