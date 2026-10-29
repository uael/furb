//! The servers of MCP that the tests ask, with no network: one over stdio, which the test program runs as a process
//! of its own, and one program of HTTP on 127.0.0.1 that serves the streamable HTTP, with JSON or with a stream of
//! events, the older HTTP with SSE, a server that asks for a login, and its authorization server.

use std::{
  collections::HashMap,
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

/// The variable that makes the test program the server over stdio.
pub const FIXTURE: &str = "FURB_MCP_FIXTURE";

/// The variable that names the file where the server over stdio writes a line at each start.
pub const STARTS: &str = "FURB_MCP_FIXTURE_STARTS";

/// The bytes of the image that the tool picture gives.
pub const PICTURE: &[u8] = b"\x89PNG\r\n\x1a\nfixture";

/// What the server holds: whether the tool added stands, which a call of the tool change adds.
#[derive(Default)]
pub struct Fixture {
  added: bool,
}

/// One tool of the server, by its name, its description and the schema of its input.
fn tool(name: &str, description: &str, input: Value) -> Value {
  json!({"name": name, "description": description, "inputSchema": input})
}

/// A result of a tool with one text.
fn said(text: &str) -> Value {
  json!({"content": [{"type": "text", "text": text}]})
}

/// What the server sends for one message: the notifications, then the answer of a request.
pub fn answer(held: &Mutex<Fixture>, message: &Value) -> Vec<Value> {
  let (Some(method), Some(id)) = (message["method"].as_str(), message.get("id").cloned()) else {
    return Vec::new();
  };
  let params = &message["params"];
  let mut sent = Vec::new();
  let result = match method {
    "initialize" => Ok(json!({
      "protocolVersion": params["protocolVersion"],
      "capabilities": {"tools": {"listChanged": true}, "resources": {}, "prompts": {}},
      "serverInfo": {"name": "fixture", "version": "1.2.3"},
      "instructions": "Use the fixture.",
    })),
    "ping" => Ok(json!({})),
    "tools/list" => {
      let mut tools = vec![
        json!({
          "name": "echo",
          "title": "Echo",
          "description": "Echoes a text.",
          "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]},
          "outputSchema": {"type": "object", "properties": {"text": {"type": "string"}}},
          "annotations": {"readOnlyHint": true},
        }),
        tool("picture", "Gives a picture.", json!({"type": "object"})),
        tool("long", "Gives a long text.", json!({"type": "object"})),
        tool("fail", "Fails.", json!({"type": "object"})),
        tool(
          "slow",
          "Sleeps.",
          json!({"type": "object", "properties": {"seconds": {"type": "number"}}}),
        ),
        tool("secret", "Tells a secret.", json!({"type": "object"})),
        tool("change", "Adds a tool.", json!({"type": "object"})),
        tool("crash", "Ends the server.", json!({"type": "object"})),
      ];
      if held.lock().unwrap().added {
        tools.push(tool("added", "Was added.", json!({"type": "object"})));
      }
      match params["cursor"].as_str() {
        None => Ok(json!({"tools": tools[..3], "nextCursor": "2"})),
        Some(_) => Ok(json!({"tools": tools[3..]})),
      }
    }
    "tools/call" => {
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
          thread::sleep(Duration::from_secs_f64(arguments["seconds"].as_f64().unwrap_or(1.0)));
          Ok(said("slept"))
        }
        "secret" => Ok(said("the secret")),
        "change" => {
          held.lock().unwrap().added = true;
          sent.push(json!({"jsonrpc": "2.0", "method": "notifications/tools/list_changed"}));
          Ok(said("changed"))
        }
        "crash" => std::process::exit(3),
        other => Err(format!("no tool {other}")),
      }
    }
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
pub fn stdio() {
  if let Some(path) = std::env::var_os(STARTS) {
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    writeln!(file, "start {}", std::process::id()).unwrap();
  }
  let held = Arc::new(Mutex::new(Fixture::default()));
  let out = Arc::new(Mutex::new(std::io::stdout()));
  writeln!(out.lock().unwrap(), "a line of a log that is no JSON").unwrap();
  for line in std::io::stdin().lock().lines() {
    let Ok(line) = line else { break };
    let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
    let (held, out) = (held.clone(), out.clone());
    thread::spawn(move || {
      for one in answer(&held, &message) {
        let mut out = out.lock().unwrap();
        writeln!(out, "{one}").unwrap();
        out.flush().unwrap();
      }
    });
  }
  std::process::exit(0);
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
pub struct Program {
  pub address: String,
  served: Arc<Served>,
}

impl Program {
  /// The token that the server that asks for a login takes from now on, which revokes the token it took.
  pub fn revoke(&self, token: &str) {
    *self.served.token.lock().unwrap() = token.to_owned();
  }

  /// The value of the header x-test of each request, in order.
  pub fn headers(&self) -> Vec<String> {
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
    head.push_str(&format!("{name}: {value}\r\n"));
  }
  let _ = stream.write_all(format!("{head}\r\n{body}").as_bytes());
}

/// A stream of events of messages.
fn events(messages: &[Value]) -> String {
  messages.iter().map(|one| format!("event: message\ndata: {one}\n\n")).collect()
}

/// The program of HTTP, started on a port of 127.0.0.1.
pub fn served() -> Program {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = format!("http://{}", listener.local_addr().unwrap());
  let served = Arc::new(Served { token: Mutex::new("token-1".to_owned()), ..Served::default() });
  let (held, base) = (served.clone(), address.clone());
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(stream) = stream else { continue };
      let (held, base) = (held.clone(), base.clone());
      thread::spawn(move || handle(stream, &held, &base));
    }
  });
  Program { address, served }
}

/// One request of HTTP to the program, answered.
fn handle(mut stream: TcpStream, held: &Served, base: &str) {
  let Some((method, target, headers, body)) = request(&mut stream) else { return };
  let (path, query) = target.split_once('?').unwrap_or((&target, ""));
  let query: HashMap<String, String> =
    url::form_urlencoded::parse(query.as_bytes()).into_owned().collect();
  let json = |value: Value| ("content-type", value.to_string());
  let _ = json;
  if let Some(value) = headers.get("x-test") {
    held.headers.lock().unwrap().push(value.clone());
  }
  match (method.as_str(), path) {
    ("POST", "/mcp" | "/mcp-sse" | "/secure") => {
      let bearer = format!("Bearer {}", held.token.lock().unwrap());
      if path == "/secure" && headers.get("authorization") != Some(&bearer) {
        let challenge = format!(
          "Bearer resource_metadata=\"{base}/.well-known/oauth-protected-resource/secure\""
        );
        return respond(&mut stream, "401 Unauthorized", &[("www-authenticate", challenge)], "");
      }
      let message: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
      let initialize = message["method"] == "initialize";
      if !initialize && headers.get("mcp-session-id").map(String::as_str) != Some("session-1") {
        return respond(&mut stream, "400 Bad Request", &[], "no session");
      }
      let sent = answer(&held.fixture, &message);
      let session = ("mcp-session-id", "session-1".to_owned());
      if message.get("id").is_none() {
        return respond(&mut stream, "202 Accepted", &[session], "");
      }
      match path {
        "/mcp-sse" => respond(
          &mut stream,
          "200 OK",
          &[session, ("content-type", "text/event-stream".to_owned())],
          &events(&sent),
        ),
        _ => {
          let last = sent.last().cloned().unwrap_or(Value::Null).to_string();
          respond(
            &mut stream,
            "200 OK",
            &[session, ("content-type", "application/json".to_owned())],
            &last,
          )
        }
      }
    }
    ("GET", "/mcp" | "/mcp-sse" | "/secure") => {
      respond(&mut stream, "405 Method Not Allowed", &[], "")
    }
    ("DELETE", _) => respond(&mut stream, "200 OK", &[], ""),
    ("GET", "/sse") => {
      let (send, sent) = mpsc::channel::<String>();
      held.streams.lock().unwrap().push(send);
      let head =
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\n\r\n";
      if stream
        .write_all(format!("{head}event: endpoint\ndata: /messages?session=1\n\n").as_bytes())
        .is_err()
      {
        return;
      }
      for event in sent {
        if stream.write_all(event.as_bytes()).and_then(|()| stream.flush()).is_err() {
          return;
        }
      }
    }
    ("POST", "/messages") => {
      let message: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
      let sent = events(&answer(&held.fixture, &message));
      held
        .streams
        .lock()
        .unwrap()
        .retain(|stream| sent.is_empty() || stream.send(sent.clone()).is_ok());
      respond(&mut stream, "202 Accepted", &[], "");
    }
    ("GET", "/.well-known/oauth-protected-resource/secure") => {
      let metadata = json!({"resource": format!("{base}/secure"), "authorization_servers": [format!("{base}/auth")], "scopes_supported": ["mcp"]});
      respond(
        &mut stream,
        "200 OK",
        &[("content-type", "application/json".to_owned())],
        &metadata.to_string(),
      );
    }
    ("GET", "/.well-known/oauth-authorization-server/auth") => {
      let metadata = json!({
        "issuer": format!("{base}/auth"),
        "authorization_endpoint": format!("{base}/auth/authorize"),
        "token_endpoint": format!("{base}/auth/token"),
        "registration_endpoint": format!("{base}/auth/register"),
        "code_challenge_methods_supported": ["S256"],
      });
      respond(
        &mut stream,
        "200 OK",
        &[("content-type", "application/json".to_owned())],
        &metadata.to_string(),
      );
    }
    ("POST", "/auth/register") => {
      let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
      match asked["redirect_uris"][0].as_str() {
        Some(_) => respond(
          &mut stream,
          "201 Created",
          &[("content-type", "application/json".to_owned())],
          r#"{"client_id": "client-1"}"#,
        ),
        None => respond(&mut stream, "400 Bad Request", &[], ""),
      }
    }
    ("GET", "/auth/authorize") => {
      let fit = query.get("client_id").map(String::as_str) == Some("client-1")
        && query.get("code_challenge_method").map(String::as_str) == Some("S256")
        && query.get("resource").map(String::as_str) == Some(&format!("{base}/secure"))
        && query.get("scope").map(String::as_str) == Some("mcp");
      if !fit {
        return respond(&mut stream, "400 Bad Request", &[], "");
      }
      *held.challenge.lock().unwrap() = query.get("code_challenge").cloned().unwrap_or_default();
      let back = format!("{}?code=code-1&state={}", query["redirect_uri"], query["state"]);
      respond(&mut stream, "302 Found", &[("location", back)], "");
    }
    ("POST", "/auth/token") => {
      let form: HashMap<String, String> = url::form_urlencoded::parse(&body).into_owned().collect();
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
        return respond(
          &mut stream,
          "400 Bad Request",
          &[("content-type", "application/json".to_owned())],
          refused,
        );
      }
      let token = held.token.lock().unwrap().clone();
      let tokens = json!({"access_token": token, "refresh_token": "refresh-1", "expires_in": 3600, "token_type": "Bearer"});
      respond(
        &mut stream,
        "200 OK",
        &[("content-type", "application/json".to_owned())],
        &tokens.to_string(),
      );
    }
    _ => respond(&mut stream, "404 Not Found", &[], ""),
  }
}

/// What a GET of an address answers: its status and its headers in lower case.
pub fn get(address: &str) -> (u16, HashMap<String, String>) {
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
  let _ = stream.read_to_string(&mut got);
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
pub fn consenting() -> super::Opener {
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
