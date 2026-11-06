//! The wires to a server of MCP, and the JSON-RPC over them: a process that reads requests on its stdin and writes
//! its answers on its stdout, one JSON a line; a server of the streamable HTTP, which answers each POST with JSON or
//! with a stream of events and tells more on a stream that a GET opens; and a server of the older HTTP with SSE,
//! which tells every answer on the stream of events that a GET opens, and takes each request at the endpoint that the
//! stream names first.
//!
//! Each wire hands what the server says to its peer, which answers a request of the server, gives an answer to the
//! request that waits for it, and hands each notification on. A wire that drops ends what it holds: the process and
//! every process that it started, and each stream.

use std::{
  collections::{HashMap, VecDeque},
  path::Path,
  process::Stdio,
  sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicI64, Ordering},
  },
  time::Duration,
};

use reqwest::{Client, RequestBuilder, Response};
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
  process::{Child, ChildStderr, ChildStdin, ChildStdout},
  sync::{mpsc, oneshot},
  task::JoinSet,
};

use super::{config::Transport, oauth::challenged};
use crate::{
  ear::runtime,
  world::child::{self, Group},
};

/// The most lines of the stderr of a process that the reason of its end quotes.
const TAIL: usize = 20;

/// Why a request to a server came to nothing.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Failure {
  /// The server answered with an error of JSON-RPC.
  Answered { code: i64, message: String },
  /// The server asks for a login: it answered 401, with the address of the metadata of its resource when it named one.
  Unauthorized(Option<String>),
  /// The request took more than its bound.
  Late(Duration),
  /// The wire failed or closed, and why.
  Broken(String),
}

impl Failure {
  /// Why, in words.
  pub(super) fn said(&self) -> String {
    match self {
      Self::Answered { code, message } => format!("{message} (error {code})"),
      Self::Unauthorized(_) => "the server asks for a login".to_owned(),
      Self::Late(bound) => format!("it took more than {} seconds", seconds(*bound)),
      Self::Broken(why) => why.clone(),
    }
  }
}

/// A span of time in seconds, with no fraction when it has none.
pub(super) fn seconds(span: Duration) -> String {
  let held = span.as_secs_f64();
  if held.fract() == 0.0 { format!("{held:.0}") } else { format!("{held}") }
}

/// What a peer hears of its own accord: a notification of the server, by its method, or the end of the wire, with
/// why.
#[derive(Debug)]
pub(super) enum Heard {
  Notified(String),
  Closed(String),
}

/// What a wire hands its peer: a message of the server, or its end with why.
enum Inbound {
  Message(Value),
  Closed(String),
}

type Inbox = mpsc::UnboundedSender<Inbound>;

/// The requests that wait for an answer, by their id.
type Pending = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, Failure>>>>>;

/// A lock that a panic of another holder does not poison.
fn locked<T>(held: &Mutex<T>) -> MutexGuard<'_, T> {
  held.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How a message goes to a server: the three ways that a server is reached.
enum Wire {
  Process(Process),
  Streamable(Streamable),
  Legacy(Legacy),
}

impl Wire {
  /// The message, sent; a request whose answer the server gives at once hands that answer to the peer.
  async fn send(&self, message: Value) -> Result<(), Failure> {
    match self {
      Self::Process(wire) => wire.send(message).await,
      Self::Streamable(wire) => wire.send(message).await,
      Self::Legacy(wire) => wire.send(message).await,
    }
  }

  /// The version of the protocol that the server agreed to, after which the wire may open more.
  fn agreed(&self, version: &str) {
    if let Self::Streamable(wire) = self {
      wire.agreed(version);
    }
  }
}

/// The client end of one connection to a server.
pub(super) struct Peer {
  wire: Arc<Wire>,
  pending: Pending,
  next: AtomicI64,
  tasks: Mutex<JoinSet<()>>,
}

impl Peer {
  /// A connection to the server of a transport, with the token of its login, which answers a request of the server
  /// for its roots with these; and what it hears of its own accord.
  pub(super) async fn open(
    transport: &Transport,
    token: Option<String>,
    roots: Value,
  ) -> Result<(Self, mpsc::UnboundedReceiver<Heard>), Failure> {
    let (inbox, inbound) = mpsc::unbounded_channel();
    let wire = Arc::new(match transport {
      Transport::Stdio { command, args, env, cwd } => {
        Wire::Process(Process::start(command, args, env, cwd.as_deref(), inbox)?)
      }
      Transport::Http { url, headers, .. } => {
        Wire::Streamable(Streamable::new(url, headers, token, inbox))
      }
      Transport::Sse { url, headers, .. } => {
        Wire::Legacy(Legacy::connect(url, headers, token, inbox).await?)
      }
    });
    let (told, heard) = mpsc::unbounded_channel();
    let pending: Pending = Arc::default();
    let mut tasks = JoinSet::new();
    tasks.spawn_on(
      dispatch(Arc::clone(&wire), Arc::clone(&pending), inbound, told, roots),
      &runtime(),
    );
    Ok((Self { wire, pending, next: AtomicI64::new(1), tasks: Mutex::new(tasks) }, heard))
  }

  /// What the server answers a request with, within a bound. A request past its bound, or that its caller drops, is
  /// cancelled at the server.
  pub(super) async fn request(
    &self,
    method: &str,
    params: Value,
    bound: Duration,
  ) -> Result<Value, Failure> {
    let id = self.next.fetch_add(1, Ordering::SeqCst);
    let (answer, answered) = oneshot::channel();
    locked(&self.pending).insert(id, answer);
    let mut waiting =
      Waiting { wire: Arc::clone(&self.wire), pending: Arc::clone(&self.pending), id, done: false };
    let message = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    let got = tokio::time::timeout(bound, async {
      self.wire.send(message).await?;
      answered
        .await
        .unwrap_or_else(|_| Err(Failure::Broken("the server closed the connection".to_owned())))
    })
    .await;
    match got {
      Ok(got) => {
        waiting.done = true;
        got
      }
      Err(_) => Err(Failure::Late(bound)),
    }
  }

  /// A notification to the server.
  pub(super) async fn notify(&self, method: &str, params: Option<Value>) -> Result<(), Failure> {
    let mut message = json!({"jsonrpc": "2.0", "method": method});
    if let Some(params) = params {
      message["params"] = params;
    }
    self.wire.send(message).await
  }

  /// The version of the protocol that the server agreed to.
  pub(super) fn agreed(&self, version: &str) {
    self.wire.agreed(version);
  }
}

impl Drop for Peer {
  fn drop(&mut self) {
    locked(&self.tasks).abort_all();
  }
}

/// A request that waits for its answer: one that its caller drops before the answer is forgotten, and cancelled at
/// the server.
struct Waiting {
  wire: Arc<Wire>,
  pending: Pending,
  id: i64,
  done: bool,
}

impl Drop for Waiting {
  fn drop(&mut self) {
    locked(&self.pending).remove(&self.id);
    if !self.done {
      let params = json!({"requestId": self.id, "reason": "the client stopped waiting"});
      let message =
        json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": params});
      let wire = Arc::clone(&self.wire);
      runtime().spawn(async move {
        // A server that is gone takes no cancel.
        wire.send(message).await.ok();
      });
    }
  }
}

/// The reply to a request of the server: furb answers a ping and the roots, and no other method.
fn replied(method: &str, id: &Value, roots: &Value) -> Value {
  match method {
    "ping" => json!({"jsonrpc": "2.0", "id": id, "result": {}}),
    "roots/list" => json!({"jsonrpc": "2.0", "id": id, "result": {"roots": roots}}),
    _ => {
      let error = json!({"code": -32601, "message": format!("furb does not answer {method}")});
      json!({"jsonrpc": "2.0", "id": id, "error": error})
    }
  }
}

/// What an answer of the server gives the request that waits for it.
fn answered(message: &Value) -> Result<Value, Failure> {
  match message.get("error") {
    Some(error) => Err(Failure::Answered {
      code: error.get("code").and_then(Value::as_i64).unwrap_or_default(),
      message: error.get("message").and_then(Value::as_str).unwrap_or("an error").to_owned(),
    }),
    None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
  }
}

/// Each message of a server, handed on: an answer to the request that waits for it, a request of the server answered,
/// and a notification to what the peer hears. At the end of the wire, each request that waits fails.
async fn dispatch(
  wire: Arc<Wire>,
  pending: Pending,
  mut inbound: mpsc::UnboundedReceiver<Inbound>,
  told: mpsc::UnboundedSender<Heard>,
  roots: Value,
) {
  let why = loop {
    let message = match inbound.recv().await {
      Some(Inbound::Message(message)) => message,
      Some(Inbound::Closed(why)) => break why,
      None => break "the connection closed".to_owned(),
    };
    let messages = match message {
      Value::Array(all) => all,
      one => vec![one],
    };
    for message in messages {
      let method = message.get("method").and_then(Value::as_str).map(str::to_owned);
      match (method, message.get("id").cloned()) {
        (Some(method), Some(id)) => {
          let (wire, reply) = (Arc::clone(&wire), replied(&method, &id, &roots));
          tokio::spawn(async move {
            // A server that is gone takes no reply.
            wire.send(reply).await.ok();
          });
        }
        (Some(method), None) => {
          // A peer that is dropped hears no more.
          told.send(Heard::Notified(method)).ok();
        }
        (None, Some(id)) => {
          if let Some(answer) = id.as_i64().and_then(|id| locked(&pending).remove(&id)) {
            // A caller that gave up waits for no answer.
            answer.send(answered(&message)).ok();
          }
        }
        (None, None) => {}
      }
    }
  };
  for (_, answer) in locked(&pending).drain() {
    answer.send(Err(Failure::Broken(why.clone()))).ok();
  }
  told.send(Heard::Closed(why)).ok();
}

/// A server that runs as a process of its own group, which ends with the wire, with every process that it started.
struct Process {
  stdin: tokio::sync::Mutex<ChildStdin>,
  tasks: Mutex<JoinSet<()>>,
  /// The group of the server, which ends each process of it once the tasks that read it are gone.
  _group: Group,
}

impl Process {
  /// The process of a command, started, whose stdout the wire reads and hands on, a line at a time. A line that is no
  /// JSON is a log of the server, which the wire leaves. At the end of its stdout, the wire closes with the status of
  /// the process and the last lines of its stderr.
  fn start(
    command: &str,
    args: &[String],
    env: &[(String, String)],
    cwd: Option<&Path>,
    inbox: Inbox,
  ) -> Result<Self, Failure> {
    let mut made = tokio::process::Command::new(command);
    made.args(args).envs(env.iter().map(|(name, value)| (name, value)));
    if let Some(cwd) = cwd {
      made.current_dir(cwd);
    }
    made.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let (mut child, group) =
      child::spawn(&mut made).map_err(|no| Failure::Broken(format!("{command} {no}")))?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
      (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
      return Err(Failure::Broken(format!("{command} gave no pipes")));
    };
    let mut tasks = JoinSet::new();
    tasks.spawn_on(ended(child, stdout, stderr, inbox), &runtime());
    Ok(Self { stdin: tokio::sync::Mutex::new(stdin), tasks: Mutex::new(tasks), _group: group })
  }

  async fn send(&self, message: Value) -> Result<(), Failure> {
    let line = message.to_string() + "\n";
    let mut stdin = self.stdin.lock().await;
    let broken = |no: std::io::Error| {
      Failure::Broken(format!("the process of the server took no request: {no}"))
    };
    stdin.write_all(line.as_bytes()).await.map_err(broken)?;
    stdin.flush().await.map_err(broken)
  }
}

impl Drop for Process {
  fn drop(&mut self) {
    locked(&self.tasks).abort_all();
  }
}

/// The messages of a process, handed on until its stdout ends. Then the wire ends.
/// The end holds the status of the process and the last lines of its stderr.
/// The wire reads the stderr to its end first, so that no line is lost.
async fn ended(mut child: Child, stdout: ChildStdout, stderr: ChildStderr, inbox: Inbox) {
  let ((), tail) = tokio::join!(forwarded(stdout, &inbox), tailed(stderr));
  let status = child.wait().await.map(|status| format!(" with {status}")).unwrap_or_default();
  let said = tail.into_iter().collect::<Vec<_>>().join("\n");
  let said = if said.is_empty() { said } else { format!(": {said}") };
  // A peer that is dropped hears no more.
  inbox.send(Inbound::Closed(format!("the process of the server ended{status}{said}"))).ok();
}

/// Each line of the stdout of a process that is JSON, handed on as a message.
async fn forwarded(stdout: ChildStdout, inbox: &Inbox) {
  let mut lines = BufReader::new(stdout).lines();
  while let Ok(Some(line)) = lines.next_line().await {
    if let Ok(message) = serde_json::from_str::<Value>(line.trim()) {
      inbox.send(Inbound::Message(message)).ok();
    }
  }
}

/// The last lines of the stderr of a process, to its end.
async fn tailed(stderr: ChildStderr) -> VecDeque<String> {
  let (mut lines, mut tail) = (BufReader::new(stderr).lines(), VecDeque::new());
  while let Ok(Some(line)) = lines.next_line().await {
    tail.push_back(line);
    if tail.len() > TAIL {
      tail.pop_front();
    }
  }
  tail
}

/// What a server answered with a status that is no success: a login it asks for, or why it failed.
async fn refused(url: &str, response: Response) -> Failure {
  let status = response.status();
  if status.as_u16() == 401 {
    let header = response.headers().get("www-authenticate").and_then(|one| one.to_str().ok());
    return Failure::Unauthorized(header.and_then(challenged));
  }
  let text = response.text().await.unwrap_or_default();
  Failure::Broken(format!(
    "{url} answered {status}: {}",
    text.chars().take(300).collect::<String>()
  ))
}

/// Each event of a stream of events of a server handed on as a message, and nothing at its end.
async fn streamed(mut response: Response, inbox: Inbox) {
  let mut events = Events::default();
  while let Ok(Some(chunk)) = response.chunk().await {
    for (_, data) in events.feed(&chunk) {
      if let Ok(message) = serde_json::from_str::<Value>(&data) {
        inbox.send(Inbound::Message(message)).ok();
      }
    }
  }
}

/// A request of HTTP with the headers of a server and the token of its login.
fn headed(
  mut request: RequestBuilder,
  headers: &[(String, String)],
  token: Option<&str>,
) -> RequestBuilder {
  for (name, value) in headers {
    request = request.header(name.as_str(), value.as_str());
  }
  if let Some(token) = token {
    request = request.header("authorization", format!("Bearer {token}"));
  }
  request
}

/// A server of the streamable HTTP: the session it named, the version of the protocol it agreed to, and the streams
/// of its answers and of the GET it took.
struct Streamable {
  url: String,
  headers: Vec<(String, String)>,
  token: Option<String>,
  client: Client,
  session: Mutex<Option<String>>,
  version: Mutex<Option<String>>,
  inbox: Inbox,
  tasks: Mutex<JoinSet<()>>,
}

impl Streamable {
  fn new(url: &str, headers: &[(String, String)], token: Option<String>, inbox: Inbox) -> Self {
    Self {
      url: url.to_owned(),
      headers: headers.to_vec(),
      token,
      client: Client::new(),
      session: Mutex::default(),
      version: Mutex::default(),
      inbox,
      tasks: Mutex::default(),
    }
  }

  /// A request of HTTP to the server, with its headers, its session and the version it agreed to.
  fn request(&self, made: RequestBuilder) -> RequestBuilder {
    let mut request = headed(made, &self.headers, self.token.as_deref());
    if let Some(session) = locked(&self.session).clone() {
      request = request.header("mcp-session-id", session);
    }
    if let Some(version) = locked(&self.version).clone() {
      request = request.header("mcp-protocol-version", version);
    }
    request
  }

  async fn send(&self, message: Value) -> Result<(), Failure> {
    let request = self
      .request(self.client.post(&self.url))
      .header("content-type", "application/json")
      .header("accept", "application/json, text/event-stream")
      .body(message.to_string());
    let had = locked(&self.session).is_some();
    let url = &self.url;
    let response = request.send().await.map_err(|no| Failure::Broken(format!("{url}: {no}")))?;
    let status = response.status();
    if status.as_u16() == 404 && had {
      let why = "the server ended the session".to_owned();
      // A peer that is dropped hears no more.
      self.inbox.send(Inbound::Closed(why.clone())).ok();
      return Err(Failure::Broken(why));
    }
    if !status.is_success() {
      return Err(refused(url, response).await);
    }
    if let Some(named) = response.headers().get("mcp-session-id").and_then(|one| one.to_str().ok())
    {
      *locked(&self.session) = Some(named.to_owned());
    }
    let kind =
      response.headers().get("content-type").and_then(|one| one.to_str().ok()).unwrap_or_default();
    if kind.starts_with("text/event-stream") {
      locked(&self.tasks).spawn_on(streamed(response, self.inbox.clone()), &runtime());
      return Ok(());
    }
    let text = response.text().await.map_err(|no| Failure::Broken(format!("{url}: {no}")))?;
    if text.trim().is_empty() {
      return Ok(());
    }
    let message = serde_json::from_str::<Value>(&text)
      .map_err(|no| Failure::Broken(format!("{url} answered no JSON: {no}")))?;
    self.inbox.send(Inbound::Message(message)).ok();
    Ok(())
  }

  /// The stream of what the server tells of its own accord, opened by a GET once the server agreed to a version. A
  /// server that opens none answers 405, and tells only in the answers of its requests.
  fn agreed(&self, version: &str) {
    *locked(&self.version) = Some(version.to_owned());
    let request = self.request(self.client.get(&self.url)).header("accept", "text/event-stream");
    let inbox = self.inbox.clone();
    locked(&self.tasks).spawn_on(
      async move {
        let Ok(response) = request.send().await else { return };
        let kind = response
          .headers()
          .get("content-type")
          .and_then(|one| one.to_str().ok())
          .unwrap_or_default();
        if response.status().is_success() && kind.starts_with("text/event-stream") {
          streamed(response, inbox).await;
        }
      },
      &runtime(),
    );
  }
}

impl Drop for Streamable {
  /// The session ended at the server, which may then free what it holds for it.
  fn drop(&mut self) {
    locked(&self.tasks).abort_all();
    if locked(&self.session).is_some() {
      let request = self.request(self.client.delete(&self.url));
      runtime().spawn(async move {
        // A server that does not answer in time frees the session by itself.
        tokio::time::timeout(Duration::from_secs(5), request.send()).await.ok();
      });
    }
  }
}

/// A server of the older HTTP with SSE: the endpoint that takes each request, and the stream of events of its answers.
struct Legacy {
  endpoint: String,
  headers: Vec<(String, String)>,
  token: Option<String>,
  client: Client,
  tasks: Mutex<JoinSet<()>>,
}

impl Legacy {
  /// The stream of events of the server, opened, once it named its endpoint.
  async fn connect(
    url: &str,
    headers: &[(String, String)],
    token: Option<String>,
    inbox: Inbox,
  ) -> Result<Self, Failure> {
    let client = Client::new();
    let request =
      headed(client.get(url), headers, token.as_deref()).header("accept", "text/event-stream");
    let mut response =
      request.send().await.map_err(|no| Failure::Broken(format!("{url}: {no}")))?;
    if !response.status().is_success() {
      return Err(refused(url, response).await);
    }
    let (found, named) = oneshot::channel::<String>();
    let mut tasks = JoinSet::new();
    tasks.spawn_on(
      async move {
        let (mut found, mut events) = (Some(found), Events::default());
        while let Ok(Some(chunk)) = response.chunk().await {
          for (event, data) in events.feed(&chunk) {
            if event == "endpoint" {
              if let Some(found) = found.take() {
                found.send(data).ok();
              }
            } else if let Ok(message) = serde_json::from_str::<Value>(&data) {
              inbox.send(Inbound::Message(message)).ok();
            }
          }
        }
        inbox.send(Inbound::Closed("the stream of events of the server ended".to_owned())).ok();
      },
      &runtime(),
    );
    let Ok(named) = named.await else {
      return Err(Failure::Broken(format!("{url} named no endpoint before its stream ended")));
    };
    let endpoint = url::Url::parse(url).and_then(|base| base.join(&named)).map_err(|no| {
      Failure::Broken(format!("{url} named the endpoint {named}, which is not valid: {no}"))
    })?;
    Ok(Self {
      endpoint: endpoint.to_string(),
      headers: headers.to_vec(),
      token,
      client,
      tasks: Mutex::new(tasks),
    })
  }

  async fn send(&self, message: Value) -> Result<(), Failure> {
    let request = headed(self.client.post(&self.endpoint), &self.headers, self.token.as_deref())
      .header("content-type", "application/json")
      .body(message.to_string());
    let endpoint = &self.endpoint;
    let response =
      request.send().await.map_err(|no| Failure::Broken(format!("{endpoint}: {no}")))?;
    if response.status().is_success() { Ok(()) } else { Err(refused(endpoint, response).await) }
  }
}

impl Drop for Legacy {
  fn drop(&mut self) {
    locked(&self.tasks).abort_all();
  }
}

/// The reader of a stream of events, which takes its bytes as they come and gives each event whole: its name,
/// `message` when it names none, and its data, its lines joined.
#[derive(Default)]
pub(super) struct Events {
  held: Vec<u8>,
  event: String,
  data: Vec<String>,
}

impl Events {
  /// Each event that these bytes end.
  pub(super) fn feed(&mut self, bytes: &[u8]) -> Vec<(String, String)> {
    self.held.extend_from_slice(bytes);
    let mut out = Vec::new();
    while let Some(at) = self.held.iter().position(|one| *one == b'\n') {
      let line: Vec<u8> = self.held.drain(..=at).collect();
      let line = String::from_utf8_lossy(&line);
      let line = line.trim_end_matches(['\n', '\r']);
      if line.is_empty() {
        if !self.data.is_empty() {
          let event = std::mem::take(&mut self.event);
          let event = if event.is_empty() { "message".to_owned() } else { event };
          out.push((event, std::mem::take(&mut self.data).join("\n")));
        }
        self.event.clear();
        continue;
      }
      if line.starts_with(':') {
        continue;
      }
      let (field, value) = line.split_once(':').unwrap_or((line, ""));
      let value = value.strip_prefix(' ').unwrap_or(value);
      match field {
        "event" => value.clone_into(&mut self.event),
        "data" => self.data.push(value.to_owned()),
        _ => {}
      }
    }
    out
  }
}

#[cfg(test)]
#[path = "wire.test.rs"]
mod test;
