//! The ear of MCP in a life of the real engine: the servers of the tests over stdio, over the streamable HTTP and
//! over the older HTTP with SSE, each act and each state, the controls that write the local scope, the changes that
//! the ear tells, a login by OAuth, and a later life that replays its record and asks no server.

use std::{
  collections::HashMap,
  fs,
  future::Future,
  path::{Path, PathBuf},
  pin::pin,
  sync::Arc,
  task::{Context, Poll, Wake, Waker},
  thread,
  time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::{
  CHANGED, Filed, Places, Settings,
  fixture::{self, FIXTURE, PICTURE, STARTS},
  mcp, valued,
};
use crate::{
  Ear, Engine, Fact, Object,
  ear::{ear, hear, say},
  life::Opening,
  value::Fault,
  verbs,
  world::images,
};

/// The name of the test that runs the server of the tests over stdio, which the test program runs as a process.
const SERVING: &str = "extension::mcp::test::the_test_program_is_the_server_of_the_tests_over_stdio_when_it_is_asked_to_be";

#[test]
fn the_test_program_is_the_server_of_the_tests_over_stdio_when_it_is_asked_to_be() {
  if std::env::var_os(FIXTURE).is_some() {
    fixture::stdio();
  }
}

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-mcp-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(at.join("project")).unwrap();
  at
}

/// A file of JSON, written with its folder.
fn config(path: &Path, value: Value) {
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

/// The definition of the server of the tests over stdio, which writes a line at each start in the file `starts` of a
/// directory, with more keys.
fn stdio(at: &Path, more: Value) -> Value {
  let exe = std::env::current_exe().unwrap().display().to_string();
  let starts = at.join("starts").display().to_string();
  let mut server = json!({
    "command": exe,
    "args": ["--exact", SERVING, "--nocapture", "--test-threads", "1"],
    "env": {FIXTURE: "1", STARTS: starts},
  });
  for (key, value) in more.as_object().cloned().unwrap_or_default() {
    server[key] = value;
  }
  server
}

/// How many times the server of the tests over stdio started.
fn starts(at: &Path) -> usize {
  fs::read_to_string(at.join("starts")).map(|text| text.lines().count()).unwrap_or(0)
}

/// The settings of a life in a directory of the tests: its configs, its images, a file of tokens, an opener that
/// consents, and the variables of the test.
fn settings(at: &Path, variables: &[(&str, &str)]) -> Settings {
  let variables: HashMap<String, String> =
    variables.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned())).collect();
  Settings {
    places: Places { config: at.join("config"), project: at.join("project") },
    gallery: Some(at.join("images")),
    vault: Arc::new(Filed::new(at.join("config").join("mcp-tokens.json"))),
    opener: fixture::consenting(),
    variables: Arc::new(move |name| variables.get(name).cloned()),
  }
}

/// A waker that unparks the thread of the test.
struct Parked(thread::Thread);

impl Wake for Parked {
  fn wake(self: Arc<Self>) {
    self.0.unpark();
  }
}

/// One future driven to its end on this thread.
fn block_on<T>(fut: impl Future<Output = T>) -> T {
  let waker = Waker::from(Arc::new(Parked(thread::current())));
  let mut cx = Context::from_waker(&waker);
  let mut fut = pin!(fut);
  loop {
    if let Poll::Ready(got) = fut.as_mut().poll(&mut cx) {
      return got;
    }
    thread::park_timeout(Duration::from_millis(20));
  }
}

/// The ear that answers a stand with the operator alone, in the directory of the project.
fn standing(at: &Path) -> Box<dyn Ear> {
  let here = at.join("project").display().to_string();
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() == "stand" && a.question() {
        let operator = Object::list([Object::string("operator"), Object::list([]), Object::int(1)]);
        let standing = Object::list([
          Object::list([operator]),
          Object::string(here.clone()),
          Object::string("operator"),
        ]);
        say(&mut co, Fact::says("done", a.about(), [standing]));
      }
    }
  })
}

/// A life on the ear of MCP, in a directory of the tests.
fn life(at: &Path, variables: &[(&str, &str)]) -> Engine {
  let ears = [("stand", standing(at)), ("mcp", mcp(settings(at, variables)))];
  Engine::boot(Vec::<Object>::new(), ears).unwrap()
}

/// What a word that a rung runs on the root closes with, as JSON, or the message of why it raised.
fn closed(engine: &mut Engine, word: &str) -> Result<Value, String> {
  let root = engine.root().to_owned();
  let rung = verbs::Rung { word: Some(word.to_owned()), on: Some(root), ..Default::default() };
  match block_on(engine.rung(rung).unwrap()) {
    Ok(got) => Ok(valued(got.as_ref())),
    Err(no) => Err(Fault::message(&no)),
  }
}

/// What an act of the extension on the root comes to, with its words in python.
fn asked(engine: &mut Engine, kind: &str, words: &str) -> Result<Value, String> {
  closed(engine, &format!("close(await act({kind:?}, '', None, {words}))"))
}

/// The dict of each server.
fn servers(engine: &mut Engine) -> Vec<Value> {
  asked(engine, "mcp_servers", "").unwrap().as_array().cloned().unwrap()
}

/// The dict of one server.
fn server(engine: &mut Engine, name: &str) -> Value {
  servers(engine).into_iter().find(|one| one["name"] == name).unwrap()
}

/// The facts of the changes of the servers that the root chain holds, as the name of each server.
fn changes(engine: &mut Engine) -> Vec<String> {
  let of = verbs::Transcript { on: Some("chain1".to_owned()), of: Some(CHANGED.to_owned()) };
  let facts = engine.transcript(of).unwrap();
  facts
    .iter()
    .map(|fact| fact.word(0).and_then(|one| one.as_str().map(str::to_owned)).unwrap())
    .collect()
}

/// The life driven until a test of it holds, within ten seconds.
fn until(engine: &mut Engine, mut holds: impl FnMut(&mut Engine) -> bool) {
  let deadline = Instant::now() + Duration::from_secs(10);
  while !holds(engine) {
    assert!(Instant::now() < deadline, "the life did not come to what the test waits for");
    closed(engine, "close(1)").unwrap();
    thread::sleep(Duration::from_millis(100));
  }
}

#[test]
fn a_server_over_stdio_offers_what_it_lists_but_its_denied_tools_and_answers_each_act() {
  let at = place("stdio");
  let defined = stdio(&at, json!({"deny": ["secret"], "timeout": 1}));
  config(&at.join("config").join("mcp.json"), json!({"mcpServers": {"fixture": defined}}));
  let mut engine = life(&at, &[]);
  let one = server(&mut engine, "fixture");
  assert_eq!(one["state"], "connected", "{one}");
  assert_eq!(
    (&one["scope"], &one["transport"], &one["error"]),
    (&json!("user"), &json!("stdio"), &json!(""))
  );
  assert!(
    one["target"]
      .as_str()
      .unwrap()
      .ends_with(&format!("--exact {SERVING} --nocapture --test-threads 1"))
  );
  assert_eq!(
    (&one["version"], &one["instructions"]),
    (&json!("1.2.3"), &json!("Use the fixture."))
  );
  let names: Vec<&str> =
    one["tools"].as_array().unwrap().iter().map(|tool| tool["name"].as_str().unwrap()).collect();
  assert_eq!(
    names,
    ["echo", "picture", "long", "fail", "slow", "change", "crash"],
    "every page, less the denied tool"
  );
  assert_eq!(
    one["tools"][0],
    json!({
      "name": "echo",
      "title": "Echo",
      "description": "Echoes a text.",
      "input": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]},
      "output": {"type": "object", "properties": {"text": {"type": "string"}}},
      "annotations": {"readOnlyHint": true},
    })
  );
  assert_eq!(
    one["tools"][1],
    json!({"name": "picture", "title": "", "description": "Gives a picture.", "input": {"type": "object"}, "output": null, "annotations": {}})
  );
  assert_eq!(
    one["resources"],
    json!([{"uri": "fixture://readme", "name": "readme", "description": "The readme.", "mime": "text/plain"}])
  );
  assert_eq!(
    one["templates"],
    json!([{"uri": "fixture://file/{name}", "name": "file", "description": "A file."}])
  );
  assert_eq!(
    one["prompts"],
    json!([{"name": "greet", "description": "Greets.", "arguments": [{"name": "who", "description": "Whom.", "required": true}]}])
  );
  assert!(one["binding"].is_string());
  assert_eq!(
    asked(&mut engine, "mcp_call", "'fixture', 'echo', {'text': 'hi'}").unwrap(),
    json!({"text": "echo: hi", "content": [{"type": "text", "text": "echo: hi"}], "structured": {"text": "hi"}})
  );
  let picture = asked(&mut engine, "mcp_call", "'fixture', 'picture'").unwrap();
  let uri = picture["content"][0]["image"].as_str().unwrap();
  assert_eq!(picture["content"][0], json!({"type": "image", "image": uri, "mime": "image/png"}));
  assert_eq!(picture["text"], images::reference("image", uri));
  assert_eq!(images::read(&at.join("images"), uri).unwrap(), ("image/png", PICTURE.to_vec()));
  let long = asked(&mut engine, "mcp_call", "'fixture', 'long'").unwrap();
  let cut =
    format!("{}\n\n[furb cut this text at 100000 of its 150000 characters]", "a".repeat(100_000));
  assert_eq!(long["text"], cut);
  assert_eq!(asked(&mut engine, "mcp_call", "'fixture', 'fail'").unwrap_err(), "it failed");
  assert_eq!(
    asked(&mut engine, "mcp_call", "'fixture', 'secret'").unwrap_err(),
    "the config of the server fixture denies its tool secret"
  );
  assert_eq!(
    asked(&mut engine, "mcp_call", "'fixture', 'slow', {'seconds': 3}").unwrap_err(),
    "fixture.slow took more than 1 seconds, so furb cancelled it"
  );
  assert_eq!(
    asked(&mut engine, "mcp_read", "'fixture', 'fixture://readme'").unwrap(),
    json!({"text": "Read me.", "content": [{"type": "resource", "uri": "fixture://readme", "mime": "text/plain", "text": "Read me."}]})
  );
  assert_eq!(
    asked(&mut engine, "mcp_prompt", "'fixture', 'greet', {'who': 'Ada'}").unwrap(),
    json!({"text": "Greet Ada.", "messages": [{"role": "user", "content": {"type": "text", "text": "Greet Ada."}}]})
  );
  assert_eq!(
    asked(&mut engine, "mcp_call", "'nobody', 'echo'").unwrap_err(),
    "there is no server nobody"
  );
  assert_eq!(starts(&at), 1, "one process served every act");
}

#[test]
fn each_server_has_its_state_and_a_control_writes_the_choice_of_the_operator_to_the_local_scope() {
  let at = place("states");
  let user = json!({"mcpServers": {
    "off": stdio(&at, json!({"disabled": true})),
    "unset": {"command": "${MISSING}"},
    "absent": {"command": "/nonexistent/furb-mcp-server"},
    "pigeon": {"type": "carrier-pigeon", "url": "http://127.0.0.1"},
  }});
  config(&at.join("config").join("mcp.json"), user);
  config(
    &at.join("project").join(".mcp.json"),
    json!({"mcpServers": {"shared": stdio(&at, json!({}))}}),
  );
  let local = at.join("project").join(".furb").join("mcp.json");
  config(&local, json!({"theme": "kept"}));
  let mut engine = life(&at, &[]);
  let states: Vec<(String, String, String)> = servers(&mut engine)
    .iter()
    .map(|one| {
      let text = |key: &str| one[key].as_str().unwrap().to_owned();
      (text("name"), text("state"), text("scope"))
    })
    .collect();
  let triple =
    |name: &str, state: &str, scope: &str| (name.to_owned(), state.to_owned(), scope.to_owned());
  assert_eq!(
    states,
    [
      triple("off", "disabled", "user"),
      triple("unset", "failed", "user"),
      triple("absent", "failed", "user"),
      triple("pigeon", "failed", "user"),
      triple("shared", "unapproved", "project"),
    ]
  );
  assert_eq!(server(&mut engine, "unset")["error"], "the variable MISSING is not set");
  assert!(
    server(&mut engine, "absent")["error"]
      .as_str()
      .unwrap()
      .starts_with("/nonexistent/furb-mcp-server did not start")
  );
  assert_eq!(
    server(&mut engine, "pigeon")["error"],
    "the type carrier-pigeon is none of stdio, http and sse"
  );
  assert_eq!(
    asked(&mut engine, "mcp_call", "'shared', 'echo'").unwrap_err(),
    "the server shared is not connected, it is unapproved"
  );
  assert_eq!(
    asked(&mut engine, "mcp_control", "'approve', 'shared'").unwrap()["state"],
    "connected"
  );
  let written =
    |local: &Path| serde_json::from_str::<Value>(&fs::read_to_string(local).unwrap()).unwrap();
  assert_eq!(written(&local), json!({"theme": "kept", "approved": ["shared"]}));
  assert_eq!(
    asked(&mut engine, "mcp_control", "'disable', 'shared'").unwrap()["state"],
    "disabled"
  );
  assert_eq!(
    written(&local),
    json!({"theme": "kept", "approved": ["shared"], "enabled": [], "disabled": ["shared"]})
  );
  assert_eq!(asked(&mut engine, "mcp_control", "'enable', 'off'").unwrap()["state"], "connected");
  assert_eq!(
    written(&local),
    json!({"theme": "kept", "approved": ["shared"], "enabled": ["off"], "disabled": ["shared"]})
  );
  assert_eq!(
    asked(&mut engine, "mcp_control", "'reconnect', 'off'").unwrap()["state"],
    "connected"
  );
  assert_eq!(starts(&at), 3, "the approval, the enable and the reconnect each started a process");
  assert_eq!(
    asked(&mut engine, "mcp_control", "'reconnect', 'shared'").unwrap_err(),
    "the server shared is disabled: mcp_control('enable', \"shared\")"
  );
  assert_eq!(
    asked(&mut engine, "mcp_control", "'login', 'off'").unwrap_err(),
    "the server off runs on this machine and takes no login"
  );
  assert_eq!(
    asked(&mut engine, "mcp_control", "'approve', 'nobody'").unwrap_err(),
    "there is no server nobody"
  );
  assert_eq!(
    asked(&mut engine, "mcp_control", "'fly', 'off'").unwrap_err(),
    "the action fly is none of reconnect, enable, disable, approve, login and logout"
  );
}

#[test]
fn a_server_that_changes_a_list_or_ends_tells_the_root_chain_and_connects_again() {
  let at = place("changes");
  config(
    &at.join("config").join("mcp.json"),
    json!({"mcpServers": {"fixture": stdio(&at, json!({}))}}),
  );
  let mut engine = life(&at, &[]);
  assert_eq!(asked(&mut engine, "mcp_call", "'fixture', 'change'").unwrap()["text"], "changed");
  until(&mut engine, |engine| changes(engine) == ["fixture"]);
  let tools = server(&mut engine, "fixture")["tools"].clone();
  assert_eq!(tools.as_array().unwrap().last().unwrap()["name"], "added");
  let ended = asked(&mut engine, "mcp_call", "'fixture', 'crash'").unwrap_err();
  assert!(
    ended.starts_with("fixture.crash failed: the process of the server ended with exit"),
    "{ended}"
  );
  until(&mut engine, |engine| {
    server(engine, "fixture")["state"] == "connected" && changes(engine).len() >= 3
  });
  assert_eq!(starts(&at), 2);
  assert_eq!(
    asked(&mut engine, "mcp_call", "'fixture', 'echo', {'text': 'back'}").unwrap()["text"],
    "echo: back"
  );
}

#[test]
fn servers_over_the_streamable_http_and_the_older_sse_answer_as_a_server_over_stdio() {
  let at = place("http");
  let program = fixture::served();
  let base = &program.address;
  let user = json!({"mcpServers": {
    "plain": {"type": "http", "url": format!("{base}/mcp"), "headers": {"x-test": "${TOKEN:-none}"}},
    "streamed": {"type": "http", "url": format!("{base}/mcp-sse")},
    "legacy": {"type": "sse", "url": format!("{base}/sse")},
  }});
  config(&at.join("config").join("mcp.json"), user);
  let mut engine = life(&at, &[]);
  for (name, transport, path) in
    [("plain", "http", "/mcp"), ("streamed", "http", "/mcp-sse"), ("legacy", "sse", "/sse")]
  {
    let one = server(&mut engine, name);
    assert_eq!(
      (&one["state"], &one["transport"]),
      (&json!("connected"), &json!(transport)),
      "{one}"
    );
    assert_eq!(one["target"], format!("{base}{path}"));
    assert_eq!(one["tools"].as_array().unwrap().len(), 8);
    let echoed =
      asked(&mut engine, "mcp_call", &format!("'{name}', 'echo', {{'text': '{name}'}}")).unwrap();
    assert_eq!(echoed["text"], format!("echo: {name}"));
    let read = asked(&mut engine, "mcp_read", &format!("'{name}', 'fixture://readme'")).unwrap();
    assert_eq!(read["text"], "Read me.");
  }
  let headers = program.headers();
  assert!(!headers.is_empty() && headers.iter().all(|one| one == "none"), "{headers:?}");
}

#[test]
fn a_server_that_asks_for_a_login_needs_auth_until_a_login_by_oauth_and_refreshes_its_token() {
  let at = place("oauth");
  let program = fixture::served();
  let url = format!("{}/secure", program.address);
  config(
    &at.join("config").join("mcp.json"),
    json!({"mcpServers": {"secure": {"type": "http", "url": url}}}),
  );
  let mut engine = life(&at, &[]);
  let one = server(&mut engine, "secure");
  assert_eq!(one["state"], "needs-auth");
  assert_eq!(one["error"], "the server asks for a login: mcp_control('login', \"secure\")");
  assert_eq!(
    asked(&mut engine, "mcp_call", "'secure', 'echo'").unwrap_err(),
    "the server secure asks for a login: mcp_control('login', \"secure\")"
  );
  assert_eq!(asked(&mut engine, "mcp_control", "'login', 'secure'").unwrap()["state"], "connected");
  let tokens = at.join("config").join("mcp-tokens.json");
  let held: Value = serde_json::from_str(&fs::read_to_string(&tokens).unwrap()).unwrap();
  let kept: Value = serde_json::from_str(held[format!("secure {url}")].as_str().unwrap()).unwrap();
  assert_eq!(
    (&kept["access"], &kept["refresh"], &kept["client"]),
    (&json!("token-1"), &json!("refresh-1"), &json!("client-1"))
  );
  #[cfg(unix)]
  assert_eq!(
    std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&tokens).unwrap().permissions()) & 0o777,
    0o600
  );
  assert_eq!(
    asked(&mut engine, "mcp_call", "'secure', 'echo', {'text': 'in'}").unwrap()["text"],
    "echo: in"
  );
  program.revoke("token-2");
  assert_eq!(
    asked(&mut engine, "mcp_call", "'secure', 'echo', {'text': 'again'}").unwrap()["text"],
    "echo: again"
  );
  assert_eq!(
    asked(&mut engine, "mcp_control", "'logout', 'secure'").unwrap()["state"],
    "needs-auth"
  );
  assert_eq!(fs::read_to_string(&tokens).unwrap(), "{}");
}

#[test]
fn a_later_life_that_replays_a_record_with_acts_of_mcp_asks_no_server() {
  let at = place("replay");
  config(
    &at.join("config").join("mcp.json"),
    json!({"mcpServers": {"fixture": stdio(&at, json!({}))}}),
  );
  let opening = || Opening {
    directory: at.join("project").display().to_string(),
    record: Some(at.join("record.jsonl").display().to_string()),
    config: Some(at.join("config").display().to_string()),
    roster: Some(Vec::new()),
    extensions: Some(false),
    snapshot: Some(false),
    ..Opening::default()
  };
  let (mut first, _) = opening().boot::<String>([]).unwrap();
  let word = "close(await act('mcp_call', '', None, 'fixture', 'echo', {'text': 'kept'}))";
  assert_eq!(closed(&mut first, word).unwrap()["text"], "echo: kept");
  drop(first);
  assert_eq!(starts(&at), 1);
  let (mut later, _) = opening().boot::<String>([]).unwrap();
  let of = verbs::Transcript { on: Some("chain1".to_owned()), of: Some("mcp_call".to_owned()) };
  let call = later.transcript(of).unwrap().into_iter().find(Fact::question).unwrap();
  let kept = later.peek(call.about(), verbs::Peek::default()).unwrap();
  assert_eq!(valued(kept.as_ref())["text"], "echo: kept");
  assert_eq!(starts(&at), 1, "the replay asked no server");
}

#[cfg(unix)]
#[test]
fn the_process_of_a_server_over_stdio_ends_with_the_life() {
  let at = place("ends");
  config(
    &at.join("config").join("mcp.json"),
    json!({"mcpServers": {"fixture": stdio(&at, json!({}))}}),
  );
  let mut engine = life(&at, &[]);
  assert_eq!(server(&mut engine, "fixture")["state"], "connected");
  let pid =
    fs::read_to_string(at.join("starts")).unwrap().split_whitespace().nth(1).unwrap().to_owned();
  let alive = |pid: &str| {
    let got = std::process::Command::new("ps").args(["-o", "stat=", "-p", pid]).output().unwrap();
    let state = String::from_utf8_lossy(&got.stdout).trim().to_owned();
    !state.is_empty() && !state.starts_with('Z')
  };
  assert!(alive(&pid));
  drop(engine);
  let deadline = Instant::now() + Duration::from_secs(5);
  while alive(&pid) {
    assert!(Instant::now() < deadline, "the process {pid} outlived its life");
    thread::sleep(Duration::from_millis(50));
  }
}
