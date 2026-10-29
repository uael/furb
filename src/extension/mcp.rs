//! The ear of the extension mcp, which holds the servers of the Model Context Protocol of a life and answers each act
//! of the extension.
//!
//! The configs define the servers in three scopes, which [`config`] reads: the user scope, `mcp.json` of the config
//! directory of the user; the project scope, `.mcp.json` of the project, in the form of Claude Code; and the local
//! scope, `.furb/mcp.json` of the project, which holds the choices of the operator too. A server of the project scope
//! runs only once the operator approved it. A server reaches furb over stdio, the streamable HTTP or the older HTTP
//! with SSE, and a remote server logs in by OAuth 2.1. A server connects when an act first needs it, so a life that
//! replays its record asks no server, since the record holds the answer of each act. The process of each stdio
//! server ends with the life, with every process that it started.
//!
//! The acts, each of words after the chain that it is on, and what each comes to:
//!
//! - `mcp_servers()`: the list of the dicts of the servers, once each enabled and approved server connected or failed,
//!   or the bound of a connect passed. A dict is `{"name", "scope": "user" | "project" | "local", "transport":
//!   "stdio" | "http" | "sse", "target": the command line or the address, "state": "connected" | "failed" |
//!   "needs-auth" | "disabled" | "unapproved" | "pending", "error", "version", "instructions", "tools": [{"name",
//!   "title", "description", "input": its JSON Schema, "output": its schema or None, "annotations"}], "resources":
//!   [{"uri", "name", "description", "mime"}], "templates": [{"uri", "name", "description"}], "prompts": [{"name",
//!   "description", "arguments": [{"name", "description", "required"}]}], "binding": the word of Python that binds
//!   its tools, or "" when it has none}`. A tool that the config denies is in no dict.
//! - `mcp_call(server, tool, arguments)`: `{"text": the texts of its blocks joined, "content": its blocks,
//!   "structured": its structured content or None}`. An image block is `{"type": "image", "image":
//!   "furb-image://<sha256>.<extension>", "mime"}`, kept as the images of a turn are, and the text names it as a
//!   Markdown image. A result that is an error, a tool that the config denies, a server that is not connected and a
//!   call past the timeout of its server are refused with why. A text past 100000 characters is cut, and says so.
//! - `mcp_read(server, uri)`: `{"text", "content"}` of the resource.
//! - `mcp_prompt(server, name, arguments)`: `{"text", "messages": [{"role", "content": a block}]}`.
//! - `mcp_control(action, server)`, for an action of reconnect, enable, disable, approve, login and logout: the dict
//!   of the server after it. enable, disable and approve write `.furb/mcp.json`; login runs OAuth 2.1 in the browser
//!   of the system, and logout forgets the tokens of the server.
//!
//! The ear says the fact `("mcp_changed", "chain1", "mcp", server)` of its own accord when a server changes with no
//! act that asked: a list that the server says has changed, a connection that ends, and one that comes back.

pub mod binding;
mod config;
mod content;
#[cfg(test)]
mod fixture;
mod hub;
mod oauth;
mod session;
mod wire;

use std::{collections::HashMap, future::Future, path::PathBuf, sync::Arc};

use futures::{FutureExt, StreamExt, stream};
use serde_json::{Map, Number, Value, json};
use tokio::sync::mpsc;
use unsync::oneshot;

pub use self::{
  config::Places,
  hub::Settings,
  oauth::{Filed, Opener, Vault},
};
use self::{
  config::Transport,
  hub::Hub,
  wire::{Failure, seconds},
};
use crate::{
  ear::{Co, Ear, Next, hearing, reactor, say},
  fact::Fact,
  value::{Fault, Object, ObjectRef},
};

/// The kinds of the acts that the ear answers.
const KINDS: [&str; 5] = ["mcp_servers", "mcp_call", "mcp_read", "mcp_prompt", "mcp_control"];

/// The kind of the fact that a server changed with no act that asked.
pub const CHANGED: &str = "mcp_changed";

/// The root chain, which the fact of a change is about, so its transcript holds it.
const ROOT: &str = "chain1";

/// What a work of the ear came to: a change of a server, or an act and its answer or why it has none.
enum Worked {
  Changed(String),
  Done(String, Result<Value, String>),
}

impl Settings {
  /// The settings of a life on the directory of a project, the config directory of the user and the directory of
  /// the images of the life: the tokens in the store of this machine, the browser of the system, and the variables
  /// of this process.
  pub fn of(project: PathBuf, config: PathBuf, gallery: Option<PathBuf>) -> Settings {
    Settings {
      vault: oauth::vault(&config),
      places: Places { config, project },
      gallery,
      opener: oauth::opener(),
      variables: Arc::new(|name| std::env::var(name).ok()),
    }
  }
}

/// The ends of the servers, which come with the end of the ear.
struct Ends(Arc<Hub>);

impl Drop for Ends {
  fn drop(&mut self) {
    self.0.end();
  }
}

/// The ear of MCP: it takes each act of the extension, says it started, and says it done later with its answer or
/// with the refusal; and it says the fact of a change of a server that no act asked for. A control that ends an act
/// first ends its work, and the ear says nothing more of it.
pub fn mcp(settings: Settings) -> Box<dyn Ear> {
  hearing(&KINDS, move |mut co: Co<Worked>| async move {
    let (told, changes) = mpsc::unbounded_channel();
    let hub = Hub::new(settings, told);
    let _ends = Ends(hub.clone());
    co.work(stream::unfold(changes, |mut changes| async move {
      changes.recv().await.map(|name| (Worked::Changed(name), changes))
    }));
    let mut stops: HashMap<String, oneshot::Sender<()>> = HashMap::new();
    loop {
      let a = match co.next().await {
        Next::Heard(a) => a,
        Next::Worked(Worked::Changed(name)) => {
          say(&mut co, Fact::says(CHANGED, ROOT, [Object::string(name)]));
          continue;
        }
        Next::Worked(Worked::Done(id, got)) => {
          if stops.remove(&id).is_some() {
            let got = got.map_or_else(|no| Fault::refused(no).object(), |one| object(&one));
            say(&mut co, Fact::says("done", &id, [got]));
          }
          continue;
        }
      };
      let about = a.about().to_owned();
      if a.kind() == "done" {
        stops.remove(&about);
        continue;
      }
      if !a.question() || !KINDS.contains(&a.kind()) {
        continue;
      }
      // A list of servers that no server makes wait is answered at once, so a life whose servers are all settled,
      // or that has none, says the same facts in the same order whenever it runs.
      if a.kind() == "mcp_servers"
        && let Some(listed) = hub.settled_now()
      {
        say(&mut co, Fact::says("done", &about, [object(&Value::Array(listed))]));
        continue;
      }
      let words: Vec<Value> = (1..4).map(|at| a.word(at).map_or(Value::Null, valued)).collect();
      let work = running(answered(hub.clone(), a.kind().to_owned(), words));
      say(&mut co, Fact::says("started", &about, []));
      let (stop, stopped) = oneshot::channel();
      let id = about.clone();
      co.work(stream::once(work.map(move |got| Worked::Done(id, got))).take_until(stopped));
      stops.insert(about, stop);
    }
  })
}

/// A work on the reactor, which ends when what awaits it goes.
fn running<T: Send + 'static>(
  work: impl Future<Output = Result<T, String>> + Send + 'static,
) -> impl Future<Output = Result<T, String>> {
  /// The task, which ends when its handle goes.
  struct Aborted(tokio::task::AbortHandle);
  impl Drop for Aborted {
    fn drop(&mut self) {
      self.0.abort();
    }
  }
  let task = reactor().spawn(work);
  let aborted = Aborted(task.abort_handle());
  async move {
    let got = task.await;
    drop(aborted);
    got.unwrap_or_else(|no| Err(no.to_string()))
  }
}

/// A word of an act as a text.
fn text(words: &[Value], at: usize) -> String {
  words.get(at).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// What an act of the extension comes to, or why it comes to nothing.
async fn answered(hub: Arc<Hub>, kind: String, words: Vec<Value>) -> Result<Value, String> {
  let server = text(&words, 0);
  let arguments = || match words.get(2) {
    Some(Value::Object(held)) => Value::Object(held.clone()),
    _ => json!({}),
  };
  match kind.as_str() {
    "mcp_servers" => Ok(Value::Array(hub.listed().await)),
    "mcp_control" => hub.control(&text(&words, 0), &text(&words, 1)).await,
    "mcp_call" => {
      let tool = text(&words, 1);
      let (_, held) = hub.session(&server).await?;
      if held.deny.contains(&tool) {
        return Err(format!("the config of the server {server} denies its tool {tool}"));
      }
      let asked = json!({"name": tool, "arguments": arguments()});
      let got =
        asked_again(&hub, &server, "tools/call", asked, &format!("{server}.{tool}")).await?;
      content::called(&got, hub.gallery())
    }
    "mcp_read" => {
      let uri = text(&words, 1);
      let got = asked_again(
        &hub,
        &server,
        "resources/read",
        json!({"uri": uri}),
        &format!("{server} {uri}"),
      )
      .await?;
      Ok(content::read(&got, hub.gallery()))
    }
    "mcp_prompt" => {
      let name = text(&words, 1);
      let given: Map<String, Value> = match arguments() {
        Value::Object(held) => held
          .into_iter()
          .map(|(key, one)| {
            (key, Value::String(one.as_str().map_or_else(|| one.to_string(), str::to_owned)))
          })
          .collect(),
        _ => Map::new(),
      };
      let asked = json!({"name": name, "arguments": given});
      let got =
        asked_again(&hub, &server, "prompts/get", asked, &format!("the prompt {name} of {server}"))
          .await?;
      Ok(content::prompted(&got, hub.gallery()))
    }
    other => Err(format!("the ear of MCP takes no {other}")),
  }
}

/// What a server answers a request with, within the timeout of its server; a server that answers 401 is asked again
/// once its tokens are refreshed, and a server whose tokens cannot be refreshed needs a login.
async fn asked_again(
  hub: &Arc<Hub>,
  server: &str,
  method: &str,
  params: Value,
  what: &str,
) -> Result<Value, String> {
  let mut tried = false;
  loop {
    let (session, held) = hub.session(server).await?;
    match session.peer.request(method, params.clone(), held.timeout).await {
      Ok(got) => return Ok(got),
      Err(Failure::Unauthorized(hint))
        if !tried && matches!(held.transport, Transport::Http { .. } | Transport::Sse { .. }) =>
      {
        tried = true;
        drop(session);
        if !hub.renewed(server, hint).await {
          return Err(format!(
            "the server {server} asks for a login: mcp_control('login', {server:?})"
          ));
        }
      }
      Err(Failure::Late(bound)) => {
        return Err(format!(
          "{what} took more than {} seconds, so furb cancelled it",
          seconds(bound)
        ));
      }
      Err(no) => return Err(format!("{what} failed: {}", no.said())),
    }
  }
}

/// A value of the sandbox as JSON: a map with keys that are no text has each key as its text.
fn valued(one: ObjectRef<'_>) -> Value {
  match one.type_name() {
    "NoneType" => Value::Null,
    "bool" => one.as_bool().map_or(Value::Null, Value::Bool),
    "int" => {
      one.as_int().map_or_else(|| Value::String(one.py_repr()), |held| Value::Number(held.into()))
    }
    "float" => one.as_float().and_then(Number::from_f64).map_or(Value::Null, Value::Number),
    "str" => Value::String(one.as_str().unwrap_or_default().to_owned()),
    _ => match (one.pairs(), one.items()) {
      (Some(pairs), _) => Value::Object(
        pairs
          .into_iter()
          .map(|(key, held)| {
            (key.as_str().map_or_else(|| key.py_repr(), str::to_owned), valued(held))
          })
          .collect(),
      ),
      (None, Some(items)) => Value::Array(items.into_iter().map(valued).collect()),
      (None, None) => Value::String(one.py_repr()),
    },
  }
}

/// JSON as a value of the sandbox.
fn object(one: &Value) -> Object {
  match one {
    Value::Null => Object::none(),
    Value::Bool(held) => Object::bool(*held),
    Value::Number(held) => {
      held.as_i64().map_or_else(|| Object::float(held.as_f64().unwrap_or_default()), Object::int)
    }
    Value::String(held) => Object::string(held.clone()),
    Value::Array(held) => Object::list(held.iter().map(object)),
    Value::Object(held) => {
      Object::dict(held.iter().map(|(key, one)| (Object::string(key.clone()), object(one))))
    }
  }
}

#[cfg(test)]
#[path = "mcp.test.rs"]
mod test;
