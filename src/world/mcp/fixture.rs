//! The servers of MCP that the tests ask, with no network: one over stdio, which the test program runs as a process
//! of its own, and one program of HTTP on 127.0.0.1 that serves the streamable HTTP, with JSON or with a stream of
//! events, the older HTTP with SSE, a server that asks for a login, and its authorization server.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  collections::HashMap,
  fmt::Write as _,
  io::{BufRead, BufReader, Read, Write},
  net::{TcpListener, TcpStream},
  sync::{Arc, Mutex, mpsc},
  thread,
  time::Duration,
};

use base64::{
  Engine,
  engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io;
use std::process;
use url::form_urlencoded;

/// The variable that makes the test program the server over stdio.
pub(crate) const FIXTURE: &str = "FURB_MCP_FIXTURE";

/// The variable that names the file where the server over stdio writes a line at each start.
pub(crate) const STARTS: &str = "FURB_MCP_FIXTURE_STARTS";

/// The bytes of the image that the tool picture gives.
pub(crate) const PICTURE: &[u8] = b"\x89PNG\r\n\x1a\nfixture";

/// What the server holds: whether the tool added stands, which a call of the tool change adds.
#[derive(Default)]
pub(crate) struct Fixture {
  added: bool,
}

/// One tool of the server, by its name, its description and the schema of its input.
fn tool(name: &str, description: &str, input: &Value) -> Value {
  json!({"name": name, "description": description, "inputSchema": input})
}

/// A result of a tool with one text.
fn said(text: &str) -> Value {
  json!({"content": [{"type": "text", "text": text}]})
}

/// What the server answers to initialize.
fn initialize(params: &Value) -> Value {
  json!({
    "protocolVersion": params["protocolVersion"],
    "capabilities": {"tools": {"listChanged": true}, "resources": {}, "prompts": {}},
    "serverInfo": {"name": "fixture", "version": "1.2.3"},
    "instructions": "Use the fixture.",
  })
}

/// The tools of the server, in two pages.
fn tools_list(held: &Mutex<Fixture>, params: &Value) -> Value {
  let mut tools = vec![
    json!({
      "name": "echo",
      "title": "Echo",
      "description": "Echoes a text.",
      "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]},
      "outputSchema": {"type": "object", "properties": {"text": {"type": "string"}}},
      "annotations": {"readOnlyHint": true},
    }),
    tool("picture", "Gives a picture.", &json!({"type": "object"})),
    tool("long", "Gives a long text.", &json!({"type": "object"})),
    tool("fail", "Fails.", &json!({"type": "object"})),
    tool(
      "slow",
      "Sleeps.",
      &json!({"type": "object", "properties": {"seconds": {"type": "number"}}}),
    ),
    tool("secret", "Tells a secret.", &json!({"type": "object"})),
    tool("change", "Adds a tool.", &json!({"type": "object"})),
    tool("crash", "Ends the server.", &json!({"type": "object"})),
  ];
  if held.lock().unwrap().added {
    tools.push(tool("added", "Was added.", &json!({"type": "object"})));
  }
  match params["cursor"].as_str() {
    None => json!({"tools": tools[..3], "nextCursor": "2"}),
    Some(_) => json!({"tools": tools[3..]}),
  }
}

/// The result of a call of a tool; a tool may add notifications to those sent.
fn call(held: &Mutex<Fixture>, params: &Value, sent: &mut Vec<Value>) -> Result<Value, String> {
  let arguments = &params["arguments"];
  match params["name"].as_str().unwrap_or_default() {
    "echo" => {
      let text = arguments["text"].as_str().unwrap_or_default();
      Ok(
        json!({"content": [{"type": "text", "text": format!("echo: {text}")}], "structuredContent": {"text": text}}),
      )
    }
    "picture" => Ok(
      json!({"content": [{"type": "image", "data": STANDARD.encode(PICTURE), "mimeType": "image/png"}]}),
    ),
    "long" => Ok(said(&"a".repeat(150_000))),
    "fail" => Ok(json!({"content": [{"type": "text", "text": "it failed"}], "isError": true})),
    "slow" => {
      #[expect(
        clippy::disallowed_methods,
        reason = "the tool slow of the fixture sleeps as asked, on the thread of its own request"
      )]
      thread::sleep(Duration::from_secs_f64(arguments["seconds"].as_f64().unwrap_or(1.0)));
      Ok(said("slept"))
    }
    "secret" => Ok(said("the secret")),
    "change" => {
      held.lock().unwrap().added = true;
      sent.push(json!({"jsonrpc": "2.0", "method": "notifications/tools/list_changed"}));
      Ok(said("changed"))
    }
    "crash" => process::exit(3),
    other => Err(format!("no tool {other}")),
  }
}

/// What the server sends for one message: the notifications, then the answer of a request.
pub(crate) fn answer(held: &Mutex<Fixture>, message: &Value) -> Vec<Value> {
  let (Some(method), Some(id)) = (message["method"].as_str(), message.get("id").cloned()) else {
    return Vec::new();
  };
  let params = &message["params"];
  let mut sent = Vec::new();
  let result = match method {
    "initialize" => Ok(initialize(params)),
    "ping" => Ok(json!({})),
    "tools/list" => Ok(tools_list(held, params)),
    "tools/call" => call(held, params, &mut sent),
    "resources/list" => Ok(json!({"resources": [
      {"uri": "fixture://readme", "name": "readme", "description": "The readme.", "mimeType": "text/plain"},
    ]})),
    "resources/templates/list" => Ok(json!({"resourceTemplates": [
      {"uriTemplate": "fixture://file/{name}", "name": "file", "description": "A file."},
    ]})),
    "resources/read" => {
      let uri = params["uri"].as_str().unwrap_or_default();
      Ok(json!({"contents": [{"uri": uri, "mimeType": "text/plain", "text": "Read me."}]}))
    }
    "prompts/list" => Ok(json!({"prompts": [
      {"name": "greet", "description": "Greets.", "arguments": [{"name": "who", "description": "Whom.", "required": true}]},
    ]})),
    "prompts/get" => {
      let who = params["arguments"]["who"].as_str().unwrap_or_default();
      Ok(
        json!({"messages": [{"role": "user", "content": {"type": "text", "text": format!("Greet {who}.")}}]}),
      )
    }
    other => Err(format!("no method {other}")),
  };
  sent.push(match result {
    Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
    Err(why) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": why}}),
  });
  sent
}

/// The server over stdio: it writes a line to the file of its starts, then answers each line of its stdin on a
/// thread of its own, and ends at the end of its stdin.
pub(crate) fn stdio() {
  if let Some(path) = env::var_os(STARTS) {
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    writeln!(file, "start {}", process::id()).unwrap();
  }
  let held = Arc::new(Mutex::new(Fixture::default()));
  let out = Arc::new(Mutex::new(io::stdout()));
  writeln!(out.lock().unwrap(), "a line of a log that is no JSON").unwrap();
  for line in io::stdin().lock().lines() {
    let Ok(line) = line else { break };
    let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
    let (held, out) = (Arc::clone(&held), Arc::clone(&out));
    thread::spawn(move || {
      for one in answer(&held, &message) {
        let mut out = out.lock().unwrap();
        writeln!(out, "{one}").unwrap();
        out.flush().unwrap();
      }
    });
  }
  process::exit(0);
}

/// What the program of HTTP holds: the server, the streams of the older HTTP with SSE, the token that the server
/// takes, the challenge of the login that waits, and the value of the header x-test of each request.
#[derive(Default)]
struct Served {
  fixture: Mutex<Fixture>,
  streams: Mutex<Vec<mpsc::Sender<String>>>,
  token: Mutex<String>,
  challenge: Mutex<String>,
  headers: Mutex<Vec<String>>,
}

/// The program of HTTP: its address and what it holds.
pub(crate) struct Program {
  pub(crate) address: String,
  served: Arc<Served>,
}

impl Program {
  /// The token that the server that asks for a login takes from now on, which revokes the token it took.
  pub(crate) fn revoke(&self, token: &str) {
    *self.served.token.lock().unwrap() = token.to_owned();
  }

  /// The value of the header x-test of each request, in order.
  pub(crate) fn headers(&self) -> Vec<String> {
    self.served.headers.lock().unwrap().clone()
  }
}

/// A request of HTTP: its method, its target, its headers in lower case, and its body.
type Request = (String, String, HashMap<String, String>, Vec<u8>);

/// One request of HTTP, read.
fn request(stream: &mut TcpStream) -> Option<Request> {
  let mut reader = BufReader::new(stream.try_clone().ok()?);
  let mut line = String::new();
  reader.read_line(&mut line).ok()?;
  let mut parts = line.split_whitespace();
  let (method, target) = (parts.next()?.to_owned(), parts.next()?.to_owned());
  let mut headers = HashMap::new();
  loop {
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let line = line.trim_end();
    if line.is_empty() {
      break;
    }
    let (name, value) = line.split_once(':')?;
    headers.insert(name.trim().to_lowercase(), value.trim().to_owned());
  }
  let size = headers.get("content-length").and_then(|one| one.parse().ok()).unwrap_or(0);
  let mut body = vec![0; size];
  reader.read_exact(&mut body).ok()?;
  Some((method, target, headers, body))
}

/// A whole response of HTTP.
fn respond(stream: &mut TcpStream, status: &str, headers: &[(&str, String)], body: &str) {
  let mut head =
    format!("HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n", body.len());
  for (name, value) in headers {
    write!(head, "{name}: {value}\r\n").unwrap();
  }
  if let Err(why) = stream.write_all(format!("{head}\r\n{body}").as_bytes()) {
    eprintln!("fixture: no response written: {why}");
  }
}

/// A stream of events of messages.
fn events(messages: &[Value]) -> String {
  messages.iter().fold(String::new(), |mut all, one| {
    write!(all, "event: message\ndata: {one}\n\n").unwrap();
    all
  })
}

/// The program of HTTP, started on a port of 127.0.0.1.
pub(crate) fn served() -> Program {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = format!("http://{}", listener.local_addr().unwrap());
  let served = Arc::new(Served { token: Mutex::new("token-1".to_owned()), ..Served::default() });
  let (held, base) = (Arc::clone(&served), address.clone());
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(stream) = stream else { continue };
      let (held, base) = (Arc::clone(&held), base.clone());
      thread::spawn(move || handle(stream, &held, &base));
    }
  });
  Program { address, served }
}

/// A POST to a server of streamable HTTP: the messages of the server, in JSON or in events.
fn post(
  stream: &mut TcpStream,
  held: &Served,
  base: &str,
  path: &str,
  headers: &HashMap<String, String>,
  body: &[u8],
) {
  let bearer = format!("Bearer {}", held.token.lock().unwrap());
  if path == "/secure" && headers.get("authorization") != Some(&bearer) {
    let challenge =
      format!("Bearer resource_metadata=\"{base}/.well-known/oauth-protected-resource/secure\"");
    return respond(stream, "401 Unauthorized", &[("www-authenticate", challenge)], "");
  }
  let message: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
  let initialize = message["method"] == "initialize";
  if !initialize && headers.get("mcp-session-id").map(String::as_str) != Some("session-1") {
    return respond(stream, "400 Bad Request", &[], "no session");
  }
  let sent = answer(&held.fixture, &message);
  let session = ("mcp-session-id", "session-1".to_owned());
  if message.get("id").is_none() {
    return respond(stream, "202 Accepted", &[session], "");
  }
  if path == "/mcp-sse" {
    let kind = ("content-type", "text/event-stream".to_owned());
    respond(stream, "200 OK", &[session, kind], &events(&sent));
  } else {
    let last = sent.last().cloned().unwrap_or(Value::Null).to_string();
    let kind = ("content-type", "application/json".to_owned());
    respond(stream, "200 OK", &[session, kind], &last);
  }
}

/// A response of 200 with a JSON body.
fn json_ok(stream: &mut TcpStream, body: &Value) {
  respond(stream, "200 OK", &[("content-type", "application/json".to_owned())], &body.to_string());
}

/// The older HTTP with SSE: a stream of events that stays open and carries the answers.
fn sse(stream: &mut TcpStream, held: &Served) {
  let (sender, receiver) = mpsc::channel::<String>();
  held.streams.lock().unwrap().push(sender);
  let preamble =
    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\n\r\n";
  if stream
    .write_all(format!("{preamble}event: endpoint\ndata: /messages?session=1\n\n").as_bytes())
    .is_err()
  {
    return;
  }
  for event in receiver {
    if stream.write_all(event.as_bytes()).and_then(|()| stream.flush()).is_err() {
      return;
    }
  }
}

/// A message of the older HTTP with SSE: its answer goes to the streams that stand.
fn relay(stream: &mut TcpStream, held: &Served, body: &[u8]) {
  let message: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
  let sent = events(&answer(&held.fixture, &message));
  held.streams.lock().unwrap().retain(|one| sent.is_empty() || one.send(sent.clone()).is_ok());
  respond(stream, "202 Accepted", &[], "");
}

/// The registration of a client by the authorization server.
fn register(stream: &mut TcpStream, body: &[u8]) {
  let asked: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
  if asked["redirect_uris"][0].as_str().is_some() {
    let kind = ("content-type", "application/json".to_owned());
    respond(stream, "201 Created", &[kind], r#"{"client_id": "client-1"}"#);
  } else {
    respond(stream, "400 Bad Request", &[], "");
  }
}

/// The page of the login, which sends the user back with a code.
fn authorize(stream: &mut TcpStream, held: &Served, base: &str, query: &HashMap<String, String>) {
  let fit = query.get("client_id").map(String::as_str) == Some("client-1")
    && query.get("code_challenge_method").map(String::as_str) == Some("S256")
    && query.get("resource").map(String::as_str) == Some(&format!("{base}/secure"))
    && query.get("scope").map(String::as_str) == Some("mcp");
  if !fit {
    return respond(stream, "400 Bad Request", &[], "");
  }
  *held.challenge.lock().unwrap() = query.get("code_challenge").cloned().unwrap_or_default();
  let back = format!("{}?code=code-1&state={}", query["redirect_uri"], query["state"]);
  respond(stream, "302 Found", &[("location", back)], "");
}

/// The token endpoint: a token for a code that the login gave, or for the refresh token.
fn token(stream: &mut TcpStream, held: &Served, body: &[u8]) {
  let form: HashMap<String, String> = form_urlencoded::parse(body).into_owned().collect();
  let field = |key: &str| form.get(key).map(String::as_str).unwrap_or_default();
  let verified = URL_SAFE_NO_PAD.encode(Sha256::digest(field("code_verifier").as_bytes()));
  let granted = match field("grant_type") {
    "authorization_code" => {
      field("code") == "code-1" && verified == *held.challenge.lock().unwrap()
    }
    "refresh_token" => field("refresh_token") == "refresh-1",
    _ => false,
  };
  if !granted {
    let refused = r#"{"error": "invalid_grant", "error_description": "no such grant"}"#;
    let kind = ("content-type", "application/json".to_owned());
    return respond(stream, "400 Bad Request", &[kind], refused);
  }
  let token = held.token.lock().unwrap().clone();
  json_ok(
    stream,
    &json!({"access_token": token, "refresh_token": "refresh-1", "expires_in": 3600, "token_type": "Bearer"}),
  );
}

/// One request of HTTP to the program, answered.
fn handle(mut stream: TcpStream, held: &Served, base: &str) {
  let Some((method, target, headers, body)) = request(&mut stream) else { return };
  let (path, query) = target.split_once('?').unwrap_or((&target, ""));
  let query: HashMap<String, String> =
    form_urlencoded::parse(query.as_bytes()).into_owned().collect();
  if let Some(value) = headers.get("x-test") {
    held.headers.lock().unwrap().push(value.clone());
  }
  let stream = &mut stream;
  match (method.as_str(), path) {
    ("POST", "/mcp" | "/mcp-sse" | "/secure") => post(stream, held, base, path, &headers, &body),
    ("GET", "/mcp" | "/mcp-sse" | "/secure") => {
      respond(stream, "405 Method Not Allowed", &[], "");
    }
    ("DELETE", _) => respond(stream, "200 OK", &[], ""),
    ("GET", "/sse") => sse(stream, held),
    ("POST", "/messages") => relay(stream, held, &body),
    ("GET", "/.well-known/oauth-protected-resource/secure") => json_ok(
      stream,
      &json!({"resource": format!("{base}/secure"), "authorization_servers": [format!("{base}/auth")], "scopes_supported": ["mcp"]}),
    ),
    ("GET", "/.well-known/oauth-authorization-server/auth") => json_ok(
      stream,
      &json!({
        "issuer": format!("{base}/auth"),
        "authorization_endpoint": format!("{base}/auth/authorize"),
        "token_endpoint": format!("{base}/auth/token"),
        "registration_endpoint": format!("{base}/auth/register"),
        "code_challenge_methods_supported": ["S256"],
      }),
    ),
    ("POST", "/auth/register") => register(stream, &body),
    ("GET", "/auth/authorize") => authorize(stream, held, base, &query),
    ("POST", "/auth/token") => token(stream, held, &body),
    _ => respond(stream, "404 Not Found", &[], ""),
  }
}

/// What a GET of an address answers: its status and its headers in lower case.
pub(crate) fn get(address: &str) -> (u16, HashMap<String, String>) {
  let parsed = url::Url::parse(address).unwrap();
  let mut stream =
    TcpStream::connect((parsed.host_str().unwrap(), parsed.port_or_known_default().unwrap()))
      .unwrap();
  let target = match parsed.query() {
    Some(query) => format!("{}?{query}", parsed.path()),
    None => parsed.path().to_owned(),
  };
  let asked = format!(
    "GET {target} HTTP/1.1\r\nhost: {}\r\nconnection: close\r\n\r\n",
    parsed.host_str().unwrap()
  );
  stream.write_all(asked.as_bytes()).unwrap();
  let mut got = String::new();
  if let Err(why) = stream.read_to_string(&mut got) {
    eprintln!("fixture: no response read from {address}: {why}");
  }
  let mut lines = got.lines();
  let status = lines
    .next()
    .and_then(|line| line.split_whitespace().nth(1))
    .and_then(|one| one.parse().ok())
    .unwrap_or(0);
  let headers = lines
    .take_while(|line| !line.is_empty())
    .filter_map(|line| line.split_once(':'))
    .map(|(name, value)| (name.trim().to_lowercase(), value.trim().to_owned()))
    .collect();
  (status, headers)
}

/// An opener of the tests, which acts as the browser of a user who consents: it asks the page of the login, and
/// follows its redirect to the listener of furb, on a thread of its own.
pub(crate) fn consenting() -> super::Opener {
  Arc::new(|address: &str| {
    let address = address.to_owned();
    thread::spawn(move || {
      let (_, headers) = get(&address);
      if let Some(back) = headers.get("location") {
        get(back);
      }
    });
    Ok(())
  })
}
