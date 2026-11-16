//! The models that the catalog of a life makes of the configs and of the environment of its
//! process, through an Opening in a process of its own: each test runs again as a child of itself,
//! with the variables that it gives, as a user starts a host with them, and a provider on a port of
//! this machine answers the child and keeps each request that it read.

use std::{
  fs,
  io::{BufRead, BufReader, Read, Write},
  net::TcpListener,
  path::Path,
  sync::{Arc, Mutex},
  thread, time,
  time::Duration,
};

use furb::{
  Act, Engine, Fact, Object, SYSTEM, Text,
  life::{Opening, Retrying, Stream},
  verbs,
  world::{Class, Retries, Writes, images},
};
use serde_json::{Value, json};

use super::provider::{asked, counted, on, said, taken, until, yard};
use crate::{
  child::{child, here, ran},
  drive::{block_on, pause},
  opened,
  yard::Yard,
};

/// The yard of a test, with these providers in the config of its project.
pub(super) fn configured(name: &str, providers: &Value) -> Yard {
  let at = yard(name);
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  let config = json!({"providers": providers}).to_string();
  fs::write(at.join(".furb/config.json"), config).expect("the config of the project");
  at
}

/// The opening of a life of the child in its yard, which enables no extension, since an extension
/// as naming asks a model of its own.
pub(super) fn opened(at: &Path) -> Opening {
  Opening { skills: Some(false), ..opened::opening(at, false) }
}

/// The life of the child in its yard, whose root prompts the actor for an int, and whom it tells
/// what a model writes; and what the prompt came to.
pub(super) fn prompted(at: &Path, actor: &str, writes: Option<Writes>) -> (Engine, Object) {
  let opening = Opening { actor: Some(actor.to_owned()), stream: writes.map(Stream), ..opened(at) };
  let mut engine = opening.boot::<String>([]).expect("the life opens").0;
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).unwrap_or_else(|fault| fault.object());
  (engine, got)
}

/// A provider on a port of this machine, which answers each request with the next of these
/// streams, and the requests that it read, each its head and its body.
pub(super) fn served(answers: Vec<String>) -> (u16, thread::JoinHandle<Vec<String>>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let asked = thread::spawn(move || answers.iter().map(|one| exchanged(&listener, one)).collect());
  (port, asked)
}

/// The mark in an answer of [`served`] where its stream is cut in two writes.
pub(super) const CUT: u8 = 1;

/// One request that a provider on a port of this machine reads, and the events it streams in
/// answer; and the request it read.
fn exchanged(listener: &TcpListener, events: &str) -> String {
  // The provider waits for its request as long as the child takes to send it, which a loaded
  // machine stretches.
  let (stream, _) = listener.accept().expect("the provider is asked");
  stream.set_read_timeout(Some(Duration::from_secs(10))).expect("a stream that waits a time");
  let mut reader = BufReader::new(stream);
  let mut head = String::new();
  while !head.ends_with("\r\n\r\n") {
    let read = reader.read_line(&mut head).expect("the head of the request");
    assert!(read > 0, "the request ended before its head: {head}");
  }
  let length = head
    .lines()
    .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length: ").map(str::to_owned))
    .and_then(|length| length.trim().parse::<usize>().ok())
    .unwrap_or_default();
  let mut body = vec![0; length];
  reader.read_exact(&mut body).expect("the body of the request");
  let mut stream = reader.into_inner();
  // An answer that holds its own head goes as it stands, as a refusal of the provider.
  let head_out = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
  let head_out = if events.starts_with("HTTP/") { "" } else { head_out };
  // A mark of a cut ends a first write one byte after it, which can fall inside a letter, and the
  // rest goes in a second write a moment later, so the client reads two chunks.
  let mut out = format!("{head_out}{events}").into_bytes();
  let cut = out.iter().position(|one| *one == CUT).map_or(out.len(), |at| {
    out.remove(at);
    at + 1
  });
  stream.write_all(&out[..cut]).expect("the turn is streamed");
  stream.flush().expect("the first part goes");
  if cut < out.len() {
    pause(Duration::from_millis(100));
    stream.write_all(&out[cut..]).expect("the rest of the turn is streamed");
  }
  format!("{head}{}", String::from_utf8_lossy(&body))
}

/// The value of a header of a request, by its name in lower case.
pub(super) fn header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
  let head = request.split("\r\n\r\n").next().unwrap_or_default();
  head.lines().find_map(|line| {
    let (key, value) = line.split_once(':')?;
    key.eq_ignore_ascii_case(name).then(|| value.trim())
  })
}

/// The body of a request, as JSON.
pub(super) fn body(request: &str) -> Value {
  serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body")
}

/// One turn as chat completions stream it: a thought, a word in two parts, and its usage.
fn chatted() -> String {
  let events = [
    chunk(&json!({"role": "assistant", "reasoning_content": "hm"}), &Value::Null),
    chunk(&json!({"content": "close("}), &Value::Null),
    chunk(&json!({"content": "3)"}), &json!("stop")),
    ended(5, Some(40)),
  ];
  events.concat()
}

/// Whom a life tells what a model writes, and what it was told.
fn heard() -> (Writes, Arc<Mutex<Vec<[String; 4]>>>) {
  let heard: Arc<Mutex<Vec<[String; 4]>>> = Arc::default();
  let kept = Arc::clone(&heard);
  let writes: Writes = Arc::new(move |rung, chain, text, thinking| {
    kept.lock().expect("the parts told").push([rung, chain, text, thinking].map(str::to_owned));
  });
  (writes, heard)
}

#[test]
fn a_provider_of_the_network_is_asked_with_its_credential_and_streams_its_turn() {
  if let Some(at) = child() {
    let (writes, told) = heard();
    let (mut engine, got) = prompted(&at, "m/medium", Some(writes));
    assert_eq!(got.as_ref().as_int(), Some(3));
    let standing = engine.standing().expect("the life stands").py_repr();
    assert!(standing.contains("'compat:m/high'"), "the actor takes the nearest effort: {standing}");
    let expected = "('assistant', 'close(3)', (100, 5, 40, 0, 0.7), [{'type': 'reasoning', ";
    assert!(said(&mut engine)[1].2.starts_with(expected), "{:?}", said(&mut engine)[1]);
    let root = engine.root().to_owned();
    let (_, rung) = taken(&mut engine);
    let parts = [["", "hm"], ["close(", ""], ["3)", ""]]
      .map(|[text, thinking]| [rung.as_str(), root.as_str(), text, thinking].map(str::to_owned));
    assert_eq!(told.lock().expect("the parts told").clone(), parts);
    return;
  }
  let (port, asked_of) = served(vec![chatted()]);
  let model = json!({"window": 1000, "reasoning": true, "efforts": ["low", "high"],
    "cost": {"input": 1e4, "output": 2e4}});
  let compat = json!({"protocol": "chat", "url": format!("http://127.0.0.1:{port}/v1"),
    "key_env": "FURB_TEST_KEY", "models": {"m": model}});
  let at = configured("network", &json!({"compat": compat}));
  ran(here!(), &at, &[("FURB_TEST_KEY", "sekrit")]);
  let [request] = &asked_of.join().expect("the provider read the request")[..] else {
    panic!("one request")
  };
  assert!(request.starts_with("POST /v1/chat/completions "), "{request}");
  assert_eq!(header(request, "authorization"), Some("Bearer sekrit"));
  let body = body(request);
  assert_eq!(
    (&body["model"], &body["reasoning_effort"], &body["stream"]),
    (&json!("m"), &json!("high"), &json!(true))
  );
  assert_eq!(body["messages"][0]["content"][0]["text"], SYSTEM);
}

/// The events of the messages of anthropic for one turn whose word is `close(1)`.
fn messages() -> String {
  let events = [
    json!({"type": "message_start", "message": {"id": "m", "type": "message", "role": "assistant", "model": "c",
      "content": [], "stop_reason": null, "stop_sequence": null, "usage": {"input_tokens": 10, "output_tokens": 1}}}),
    json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
    json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "close(1)"}}),
    json!({"type": "content_block_stop", "index": 0}),
    json!({"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": null},
      "usage": {"output_tokens": 3}}),
    json!({"type": "message_stop"}),
  ];
  let event =
    |one: &Value| format!("event: {}\ndata: {one}\n\n", one["type"].as_str().unwrap_or_default());
  events.iter().map(event).collect()
}

#[test]
fn a_bearer_of_anthropic_goes_as_its_authorization_and_no_key_of_an_api_goes_with_it() {
  if let Some(at) = child() {
    assert_eq!(prompted(&at, "anthropic:c", None).1.as_ref().as_int(), Some(1));
    return;
  }
  let (port, asked_of) = served(vec![messages()]);
  let anthropic = json!({"url": format!("http://127.0.0.1:{port}"),
    "models": {"c": {"window": 1000, "output": 100}}});
  let at = configured("bearer", &json!({"anthropic": anthropic}));
  ran(here!(), &at, &[("ANTHROPIC_AUTH_TOKEN", "token"), ("ANTHROPIC_API_KEY", "key")]);
  let [request] = &asked_of.join().expect("the provider read the request")[..] else {
    panic!("one request")
  };
  assert!(request.starts_with("POST /v1/messages "), "{request}");
  assert_eq!(
    (header(request, "authorization"), header(request, "x-api-key")),
    (Some("Bearer token"), None)
  );
}

#[test]
fn the_gateway_of_cloudflare_takes_the_credential_of_cloudflare_alone_at_the_address_of_the_account()
 {
  if let Some(at) = child() {
    let model = "cloudflare-ai-gateway:openai/gpt-5";
    assert_eq!(prompted(&at, model, None).1.as_ref().as_int(), Some(3));
    return;
  }
  let (port, asked_of) = served(vec![chatted()]);
  let url = format!(
    "http://127.0.0.1:{port}/v1/${{CLOUDFLARE_ACCOUNT_ID}}/${{CLOUDFLARE_GATEWAY_ID}}/compat"
  );
  let gateway = json!({"url": url, "models": {"openai/gpt-5": {"window": 1000}}});
  let at = configured("gateway", &json!({"cloudflare-ai-gateway": gateway}));
  let env = [
    ("CLOUDFLARE_API_KEY", "cf"),
    ("CLOUDFLARE_ACCOUNT_ID", "acc"),
    ("CLOUDFLARE_GATEWAY_ID", "gw"),
  ];
  ran(here!(), &at, &env);
  let [request] = &asked_of.join().expect("the provider read the request")[..] else {
    panic!("one request")
  };
  assert!(request.starts_with("POST /v1/acc/gw/compat/chat/completions "), "{request}");
  let credentials = (header(request, "cf-aig-authorization"), header(request, "authorization"));
  assert_eq!(credentials, (Some("Bearer cf"), None));
}

/// The yard of a test whose config names the gateway radius at a port of this machine.
fn radius(name: &str, port: u16) -> Yard {
  configured(name, &json!({"radius": {"url": format!("http://127.0.0.1:{port}/v1")}}))
}

#[test]
fn a_gateway_of_pi_ai_gets_an_image_in_the_middle_of_a_message_and_reads_a_letter_that_two_chunks_split()
 {
  if let Some(at) = child() {
    let gallery = at.join("images");
    fs::write(at.join("a.png"), b"\x89PNG\r\n\x1a\nimage").expect("the image is written");
    let kept = images::attach(&gallery, &at.join("a.png")).expect("the image is attached");
    let message = format!("before {} after", images::reference("a.png", &kept.uri));
    let opening = Opening {
      actor: Some("radius:balanced/high".to_owned()),
      images: Some(gallery.display().to_string()),
      ..opened(&at)
    };
    let mut engine = opening.boot::<String>([]).expect("the life opens").0;
    let root = engine.root().to_owned();
    let with = verbs::Prompt { message: Some(message), on: on(&root), ..Default::default() };
    let id = engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned();
    let got = block_on(Act::<Object>::of(&mut engine, &id)).unwrap_or_else(|fault| fault.object());
    assert_eq!(got.as_ref().as_int(), Some(1), "the letter is one letter: {got:?}");
    return;
  }
  let delta = |text: &str| json!({"type": "text_delta", "contentIndex": 0, "delta": text});
  let done = json!({"type": "done", "reason": "stop", "usage": {}});
  let events = [delta("close(len('"), delta("\u{e9}"), delta("'))"), done]
    .map(|one| format!("data: {one}\n\n"))
    .concat();
  // The cut falls between the two bytes of the letter.
  let letter = events.find('\u{e9}').expect("the letter");
  let events = format!("{}{}{}", &events[..letter], char::from(CUT), &events[letter..]);
  let (port, asked_of) = served(vec![events]);
  ran(here!(), &radius("pi-image", port), &[("RADIUS_API_KEY", "r")]);
  let [request] = &asked_of.join().expect("the gateway read the request")[..] else {
    panic!("one request")
  };
  let sent = body(request);
  let messages = sent["context"]["messages"].as_array().expect("the messages of the context");
  let parts = messages
    .iter()
    .filter_map(|one| one["content"].as_array())
    .find(|parts| parts.iter().any(|part| part["type"] == "image"))
    .expect("a message holds the image");
  let at = parts.iter().position(|part| part["type"] == "image").expect("the image");
  let text = |part: &Value| part["text"].as_str().map(str::to_owned).unwrap_or_default();
  // The text before the image ends with the name that the message gives it.
  let before = text(&parts[at - 1]);
  assert!(before.contains("'before ![a.png](") && before.ends_with(".png)"), "{before}");
  assert!(text(&parts[at + 1]).starts_with(" after"), "{parts:?}");
  assert_eq!(parts[at]["data"], json!("iVBORw0KGgppbWFnZQ=="));
}

#[test]
fn an_event_of_pi_ai_that_lacks_a_field_its_protocol_gives_fails_the_reply() {
  if let Some(at) = child() {
    let (_, got) = prompted(&at, "radius:balanced/high", None);
    assert_eq!(got.as_ref().as_int(), Some(2), "the reply that failed is asked again: {got:?}");
    return;
  }
  let delta = |text: &str| json!({"type": "text_delta", "contentIndex": 0, "delta": text});
  let done = json!({"type": "done", "reason": "stop", "usage": {}});
  let turn = |events: &[Value]| {
    events.iter().fold(String::new(), |all, one| format!("{all}data: {one}\n\n"))
  };
  // A reply that read past the event that lacks its delta would close with 1.
  let lacking =
    turn(&[json!({"type": "text_delta", "contentIndex": 0}), delta("close(1)"), done.clone()]);
  let (port, asked_of) = served(vec![lacking, turn(&[delta("close(2)"), done])]);
  ran(here!(), &radius("pi-field", port), &[("RADIUS_API_KEY", "r")]);
  assert_eq!(asked_of.join().expect("the gateway read the requests").len(), 2);
}

#[test]
fn a_gateway_of_the_protocol_of_pi_ai_streams_the_turn_of_its_model_and_what_it_cost() {
  if let Some(at) = child() {
    let (mut engine, got) = prompted(&at, "radius:balanced/high", None);
    assert_eq!(got.as_ref().as_int(), Some(2));
    let expected = "('assistant', 'close(2)', (14, 2, 4, 0, 0.25), [";
    assert!(said(&mut engine)[1].2.starts_with(expected), "{:?}", said(&mut engine)[1]);
    return;
  }
  let events = [
    json!({"type": "start"}),
    json!({"type": "thinking_start", "contentIndex": 0}),
    json!({"type": "thinking_delta", "contentIndex": 0, "delta": "hm"}),
    json!({"type": "thinking_end", "contentIndex": 0, "content": "hm", "contentSignature": "s"}),
    json!({"type": "text_start", "contentIndex": 1}),
    json!({"type": "text_delta", "contentIndex": 1, "delta": "close("}),
    json!({"type": "text_delta", "contentIndex": 1, "delta": "2)"}),
    json!({"type": "text_end", "contentIndex": 1, "content": "close(2)"}),
    json!({"type": "done", "reason": "stop", "usage": {"input": 10, "output": 2, "cacheRead": 4, "cacheWrite": 0,
      "totalTokens": 16, "cost": {"total": 0.25}}}),
  ];
  let (port, asked_of) =
    served(vec![events.iter().map(|one| format!("data: {one}\n\n")).collect::<Vec<_>>().concat()]);
  // The catalog lists the gateway radius and its models: the config gives its address alone.
  let radius = json!({"url": format!("http://127.0.0.1:{port}/v1")});
  let at = configured("pi", &json!({"radius": radius}));
  ran(here!(), &at, &[("RADIUS_API_KEY", "r")]);
  let [request] = &asked_of.join().expect("the gateway read the request")[..] else {
    panic!("one request")
  };
  assert!(request.starts_with("POST /v1/messages "), "{request}");
  assert_eq!(header(request, "authorization"), Some("Bearer r"));
  let body = body(request);
  assert_eq!((&body["model"], &body["options"]["reasoning"]), (&json!("balanced"), &json!("high")));
  assert_eq!(body["context"]["systemPrompt"], SYSTEM);
  assert!(body["options"]["sessionId"].is_string(), "the conversation of a chain is its session");
}

#[test]
fn azure_is_asked_for_the_deployment_of_a_model_at_the_address_and_the_version_of_the_environment()
{
  if let Some(at) = child() {
    assert_eq!(prompted(&at, "azure:gpt-5", None).1.as_ref().as_int(), Some(3));
    return;
  }
  let (port, asked_of) = served(vec![chatted()]);
  let at = configured("azure", &json!({"azure": {"models": {"gpt-5": {"window": 1000}}}}));
  let base = format!("http://127.0.0.1:{port}/openai/v1");
  let env = [
    ("AZURE_OPENAI_API_KEY", "az"),
    ("AZURE_OPENAI_BASE_URL", base.as_str()),
    ("AZURE_OPENAI_DEPLOYMENT_NAME_MAP", "gpt-4=old, gpt-5=mine"),
    ("AZURE_OPENAI_API_VERSION", "2025-01-01"),
  ];
  ran(here!(), &at, &env);
  let [request] = &asked_of.join().expect("azure read the request")[..] else {
    panic!("one request")
  };
  let asked = "POST /openai/deployments/mine/chat/completions?api-version=2025-01-01 ";
  assert!(request.starts_with(asked), "{request}");
  assert_eq!(header(request, "api-key"), Some("az"));
}

/// One part of a stream of chat completions.
pub(super) fn chunk(delta: &Value, finish: &Value) -> String {
  let one = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen",
    "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]});
  format!("data: {one}\n\n")
}

/// The end of a stream of chat completions, with the usage of the turn and the tokens that it read
/// from the cache when it names them.
pub(super) fn ended(completion: u64, cached: Option<u64>) -> String {
  let mut usage = json!({"prompt_tokens": 100, "completion_tokens": completion,
    "total_tokens": 100 + completion});
  if let Some(cached) = cached {
    usage["prompt_tokens_details"] = json!({"cached_tokens": cached});
  }
  let one = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen",
    "choices": [], "usage": usage});
  format!("data: {one}\n\ndata: [DONE]\n\n")
}

/// The yard of a test on a local server on a port of this machine, which lists the model qwen with
/// a window of 1000 and answers each request after with the next of these streams; the most tokens
/// of thought of a turn in the config of its project, when one is given; and the requests that the
/// server read.
fn local(
  name: &str,
  most: Option<u64>,
  answers: Vec<String>,
) -> (Yard, String, thread::JoinHandle<Vec<String>>) {
  let listing =
    json!({"object": "list", "data": [{"id": "qwen", "object": "model", "meta": {"n_ctx": 1000}}]});
  let (port, asked) = served([vec![listing.to_string()], answers].concat());
  let at = yard(name);
  if let Some(most) = most {
    fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
    let config = json!({"max_thinking_tokens": most}).to_string();
    fs::write(at.join(".furb/config.json"), config).expect("the config of the project");
  }
  (at, format!("http://127.0.0.1:{port}"), asked)
}

/// The life of the child on the local server that its environment names, whose root prompts the
/// model qwen at this effort for an int; and the id of the prompt.
fn on_local(at: &Path, effort: &str) -> (Engine, String) {
  let actor = format!("local:qwen/{effort}");
  let opening = Opening { actor: Some(actor.clone()), ..opened(at) };
  let mut engine = opening.boot::<String>([]).expect("the life opens").0;
  let standing = engine.standing().expect("the life stands").py_repr();
  assert!(standing.contains(&format!("'{actor}'")), "the actor keeps its effort: {standing}");
  let id = asked(&mut engine);
  (engine, id)
}

/// The turn that the first reply of the root was done with.
pub(super) fn replied(engine: &mut Engine) -> String {
  let done = |engine: &mut Engine| {
    let facts = said(engine).into_iter();
    facts.into_iter().find(|(kind, about, _)| kind == "done" && about == "reply1").map(|one| one.2)
  };
  assert!(until(engine, |engine| done(engine).is_some()), "the reply is done");
  done(engine).unwrap_or_default()
}

/// Whether the root ran a word that holds this text.
fn ran_word(engine: &mut Engine, text: &str) -> bool {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
    .expect("the root has a transcript");
  let holds = |one: &Fact| {
    (0..4)
      .any(|at| one.word(at).and_then(|word| word.as_str()).is_some_and(|word| word.contains(text)))
  };
  facts.iter().any(|one| one.kind() == "run" && holds(one))
}

/// The messages of a request of the network after the setup of its chain, which ends at the turn
/// `stand()` that the setup crafts.
fn chat_of(messages: &[Value]) -> &[Value] {
  let stands = |one: &Value| {
    one["role"] == "assistant"
      && serde_json::to_string(&one["content"]).is_ok_and(|text| text.contains("stand()"))
  };
  &messages[messages.iter().position(stands).map_or(0, |at| at + 1)..]
}

/// The last message of a request.
pub(super) fn last(request: &str) -> Value {
  let body = body(request);
  body["messages"].as_array().and_then(|messages| messages.last()).cloned().unwrap_or_default()
}

/// What the prompt of the child came to, once it is answered.
pub(super) fn answered(engine: &mut Engine, id: &str) -> Option<i64> {
  block_on(Act::<Object>::of(engine, id)).expect("the prompt is answered").as_ref().as_int()
}

#[test]
fn a_model_of_a_local_server_is_asked_with_no_key_and_a_cap_and_the_thought_in_its_text_is_a_block_of_reasoning()
 {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    let reasoning =
      "{'type': 'reasoning', 'id': None, 'content': [{'type': 'text', 'content': {'text': 'hm'}}]}";
    // A model of a local server costs nothing.
    let expected = format!(
      "('assistant', 'close(3)', (100, 5, 40, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
    );
    assert_eq!(replied(&mut engine), expected);
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let thought = [
    chunk(&json!({"role": "assistant", "content": "<think>hm</think>\n"}), &Value::Null),
    chunk(&json!({"content": "close(3)"}), &json!("stop")),
    ended(5, Some(40)),
  ];
  let (at, url, asked_of) = local("local", None, vec![thought.concat()]);
  // A count of zero says nothing, so the model writes a quarter of its window.
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url), ("FURB_LOCAL_TOKENS", "0")]);
  let [listed, request] = &asked_of.join().expect("the server read the requests")[..] else {
    panic!("a list and a request")
  };
  assert!(listed.starts_with("GET /v1/models "), "{listed}");
  assert!(request.starts_with("POST /v1/chat/completions "), "{request}");
  assert_eq!(
    (header(listed, "authorization"), header(request, "authorization")),
    (None, None),
    "a server that takes no key is sent none"
  );
  let body = body(request);
  let asked =
    (&body["model"], &body["max_tokens"], body.get("cache_prompt"), body.get("reasoning_effort"));
  assert_eq!(
    asked,
    (&json!("qwen"), &json!(250), None, Some(&json!("high"))),
    "a model writes a quarter of its window at most, and thinks at the effort of its actor"
  );
}

#[test]
fn a_model_of_a_local_server_is_asked_with_the_key_and_the_most_tokens_that_the_environment_names()
{
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let turn =
    [chunk(&json!({"role": "assistant", "content": "close(3)"}), &json!("stop")), ended(5, None)];
  let (at, url, asked_of) = local("local-tokens", None, vec![turn.concat()]);
  ran(
    here!(),
    &at,
    &[("FURB_LOCAL_URL", &url), ("FURB_LOCAL_KEY", "k"), ("FURB_LOCAL_TOKENS", "100")],
  );
  let asked = asked_of.join().expect("the server read the requests");
  let keys: Vec<_> = asked.iter().map(|one| header(one, "authorization")).collect();
  assert_eq!(keys, [Some("Bearer k"); 2], "the list and the turn go with the key");
  assert_eq!(body(&asked[1])["max_tokens"], json!(100));
}

#[test]
fn a_turn_whose_stream_stops_before_its_end_is_refused_and_no_part_of_its_word_runs() {
  if let Some(at) = child() {
    let (mut engine, _) = on_local(&at, "high");
    let word = replied(&mut engine);
    assert!(word.contains("the stream of local:qwen stopped before its end"), "{word}");
    assert!(!ran_word(&mut engine, "x = 1"), "no part of the word runs");
    return;
  }
  let part = chunk(&json!({"role": "assistant", "content": "x = 1"}), &Value::Null);
  let (at, url, asked_of) = local("cut", None, vec![part]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  asked_of.join().expect("the server read the requests");
}

#[test]
fn a_turn_that_the_most_it_writes_ended_is_refused_and_no_part_of_its_word_runs() {
  if let Some(at) = child() {
    let (mut engine, _) = on_local(&at, "high");
    let word = replied(&mut engine);
    assert!(
      word.contains("the turn of local:qwen reached the most it writes before its end: it wrote"),
      "{word}"
    );
    assert!(
      word.contains(r#"0 characters of thought and 5 of text, which end with "x = 1""#),
      "{word}"
    );
    assert!(!ran_word(&mut engine, "x = 1"), "no part of the word runs");
    return;
  }
  let part = chunk(&json!({"role": "assistant", "content": "x = 1"}), &json!("length"));
  let (at, url, asked_of) = local("length", None, vec![format!("{part}data: [DONE]\n\n")]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  asked_of.join().expect("the server read the requests");
}

#[test]
fn a_model_whose_thought_passes_its_most_is_asked_again_with_that_thought_and_no_thought_and_its_one_fence_goes()
 {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    let reasoning = "{'type': 'reasoning', 'id': 'cut', 'content': [{'type': 'text', 'content': {'text': 'abc'}}]}";
    let expected = format!(
      "('assistant', 'close(3)', (100, 8, 0, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
    );
    assert_eq!(
      replied(&mut engine),
      expected,
      "the turn holds the thought, counts its parts, and loses its one fence"
    );
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let thought =
    ["a", "b", "c", "d"].map(|one| chunk(&json!({"reasoning_content": one}), &Value::Null));
  let again = [
    chunk(&json!({"role": "assistant", "content": "```python\nclose(3)\n```"}), &json!("stop")),
    ended(5, Some(0)),
  ];
  let (at, url, asked_of) = local("rethought", Some(2), vec![thought.concat(), again.concat()]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  let got = asked_of.join().expect("the server read the requests");
  assert_eq!(
    body(&got[1]).get("chat_template_kwargs"),
    None,
    "the first request thinks as the server says"
  );
  let (last, second) = (last(&got[2]), body(&got[2]));
  assert_eq!(
    (
      &last["role"],
      &last["reasoning_content"],
      &second["chat_template_kwargs"],
      second.get("reasoning_effort")
    ),
    (&json!("assistant"), &json!("abc"), &json!({"enable_thinking": false}), None),
    "the model is asked again with its thought and no more thought"
  );
}

#[test]
fn a_model_of_a_local_server_with_a_most_thought_of_zero_is_asked_with_its_thought_off_from_the_first_request()
 {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    replied(&mut engine);
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let events =
    [chunk(&json!({"role": "assistant", "content": "close(3)"}), &json!("stop")), ended(5, None)];
  let (at, url, asked_of) = local("unthought", Some(0), vec![events.concat()]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  let got = asked_of.join().expect("the server read the requests");
  let first = body(&got[1]);
  assert_eq!(
    (&first["chat_template_kwargs"], first.get("reasoning_effort")),
    (&json!({"enable_thinking": false}), None),
    "the first request turns the thought off"
  );
}

#[test]
fn a_local_model_reads_its_earlier_turn_with_no_thought() {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    assert_eq!(answered(&mut engine, &id), Some(1));
    return;
  }
  let first = [
    chunk(&json!({"role": "assistant", "reasoning_content": "a"}), &Value::Null),
    chunk(&json!({"content": "x = 1\ndebug(t'{x}')"}), &json!("stop")),
    ended(5, None),
  ];
  let again =
    [chunk(&json!({"role": "assistant", "content": "close(x)"}), &json!("stop")), ended(5, None)];
  let (at, url, asked_of) = local("earlier", Some(2), vec![first.concat(), again.concat()]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  let got = asked_of.join().expect("the server read the requests");
  let messages = body(&got[2])["messages"].as_array().cloned().unwrap_or_default();
  let earlier =
    chat_of(&messages).iter().find(|one| one["role"] == "assistant").cloned().unwrap_or_default();
  assert_eq!(
    (&earlier["content"], earlier.get("reasoning_content")),
    (&json!([{"type": "text", "text": "x = 1\ndebug(t'{x}')"}]), None),
    "the earlier turn goes with its word and no thought"
  );
}

#[test]
fn a_turn_of_a_local_model_loses_the_fence_that_opens_it_with_no_closing_fence() {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    let turn = replied(&mut engine);
    assert!(
      turn.starts_with(
        "('assistant', 'close(3)', (100, 5, 0, 0, 0.0), [{'type': 'text', 'text': 'close(3)'}]"
      ),
      "{turn}"
    );
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let events = [
    chunk(&json!({"role": "assistant", "content": "```python\nclose(3)\n"}), &json!("stop")),
    ended(5, None),
  ];
  let (at, url, asked_of) = local("fenced", None, vec![events.concat()]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  asked_of.join().expect("the server read the requests");
}

#[test]
fn a_turn_with_a_thought_and_no_word_is_asked_again_with_that_thought_and_no_thought() {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    let reasoning = "{'type': 'reasoning', 'id': 'cut', 'content': [{'type': 'text', 'content': {'text': 'ab'}}]}";
    let expected = format!(
      "('assistant', 'close(3)', (100, 7, 0, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
    );
    assert_eq!(
      replied(&mut engine),
      expected,
      "the turn holds the thought and the word, and counts what both wrote"
    );
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let thought = ["a", "b"].map(|one| chunk(&json!({"reasoning_content": one}), &Value::Null));
  let first = [thought.concat(), chunk(&json!({"content": ""}), &json!("stop")), ended(2, None)];
  let again = [
    chunk(&json!({"role": "assistant", "content": "close(3)"}), &json!("stop")),
    ended(5, Some(0)),
  ];
  let (at, url, asked_of) = local("wordless", Some(2), vec![first.concat(), again.concat()]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  let got = asked_of.join().expect("the server read the requests");
  let (last, second) = (last(&got[2]), body(&got[2]));
  assert_eq!(
    (&last["role"], &last["reasoning_content"], &second["chat_template_kwargs"]),
    (&json!("assistant"), &json!("ab"), &json!({"enable_thinking": false})),
    "the model is asked again with its thought and no more thought"
  );
}

#[test]
fn a_model_at_medium_is_cut_at_a_quarter_of_the_most_thought() {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "medium");
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let thought = ["a", "b", "c", "d", "e", "f"]
    .map(|one| chunk(&json!({"reasoning_content": one}), &Value::Null));
  let again =
    [chunk(&json!({"role": "assistant", "content": "close(3)"}), &json!("stop")), ended(5, None)];
  let (at, url, asked_of) = local("medium", Some(16), vec![thought.concat(), again.concat()]);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  let got = asked_of.join().expect("the server read the requests");
  let (last, second) = (last(&got[2]), body(&got[2]));
  assert_eq!(
    (&last["reasoning_content"], &second["chat_template_kwargs"]),
    (&json!("abcde"), &json!({"enable_thinking": false})),
    "a most of 16 at medium cuts the thought after its fifth part"
  );
}

#[test]
fn a_local_model_reads_the_end_of_a_thought_that_the_provider_cut_the_same_in_each_later_turn() {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let word = |text: &str| {
    [chunk(&json!({"role": "assistant", "content": text}), &json!("stop")), ended(5, None)].concat()
  };
  let thought =
    ["a", "b", "c", "d"].map(|one| chunk(&json!({"reasoning_content": one}), &Value::Null));
  let answers = vec![
    thought.concat(),
    word("x = 1\ndebug(t'{x}')"),
    word("y = 2\ndebug(t'{y}')"),
    word("close(x + y)"),
  ];
  let (at, url, asked_of) = local("ends", Some(2), answers);
  ran(here!(), &at, &[("FURB_LOCAL_URL", &url)]);
  let got = asked_of.join().expect("the server read the requests");
  let thoughts = |request: &str| -> Vec<Value> {
    let messages = body(request)["messages"].as_array().cloned().unwrap_or_default();
    chat_of(&messages)
      .iter()
      .filter(|one| one["role"] == "assistant")
      .map(|one| one.get("reasoning_content").cloned().unwrap_or_default())
      .collect()
  };
  assert_eq!(thoughts(&got[3])[0], json!("abc"), "the next turn reads the thought that was cut");
  // A request that phrased an earlier turn otherwise would start with other bytes than the request
  // before it.
  assert_eq!(thoughts(&got[4])[..2], [json!("abc"), Value::Null], "a later turn reads it the same");
}

/// The yard of a test with these files, each a path in it and its JSON.
fn holding(name: &str, files: &[(&str, Value)]) -> Yard {
  let at = yard(name);
  for (path, value) in files {
    let file = at.join(path);
    fs::create_dir_all(file.parent().expect("a folder of the file")).expect("the folder");
    fs::write(file, value.to_string()).expect("the file is written");
  }
  at
}

/// A model of a provider that no one asks in a config, with this price of a million tokens read.
fn priced(input: f64) -> Value {
  json!({"window": 1000, "cost": {"input": input, "output": 1.0}})
}

#[test]
fn a_model_takes_the_class_of_the_catalog_else_the_one_of_its_price_else_dear() {
  if let Some(at) = child() {
    let catalog = opened(&at).catalog().expect("the catalog");
    let class = |name: &str| catalog.find(name).map(|one| one.class());
    assert_eq!(class("claude-cli:haiku"), Some(Class::Cheap));
    assert_eq!(class("claude-cli:opus"), Some(Class::Dear));
    assert_eq!(class("codex:gpt-5-mini"), Some(Class::Cheap));
    assert_eq!(class("codex:gpt-5"), Some(Class::Dear));
    assert_eq!(
      class("codex:ghost"),
      Some(Class::Dear),
      "a model of no family and no price is dear"
    );
    let priced = ["p:under", "p:even", "p:over"].map(|name| class(name).expect("a model of p"));
    assert_eq!(priced, [Class::Cheap, Class::Cheap, Class::Dear]);
    assert_eq!(class("p:free"), Some(Class::Dear), "a model with no price is dear");
    let classed = catalog.find("p:under").expect("a model of p").classed(Class::Dear);
    assert_eq!(classed.class(), Class::Dear, "a class that is given wins over the price");
    return;
  }
  let expires = 4_102_444_800_000_u64;
  let listing = |slug: &str| json!({"slug": slug, "context_window": 1000, "visibility": "list"});
  let models = ["gpt-5-mini", "gpt-5", "ghost"].map(listing);
  let p = json!({"protocol": "chat", "url": "http://127.0.0.1:9/v1", "key": "k",
    "models": {"under": priced(0.25), "even": priced(1.0), "over": priced(3.0),
      "free": {"window": 1000}}});
  let at = holding(
    "classes",
    &[
      (".furb/config.json", json!({"providers": {"p": p, "codex": true}})),
      ("config/auth/codex.json", json!({"access": "a", "expires": expires, "account": "acc"})),
      ("config/auth/codex.models.json", json!({"account": "acc", "models": models})),
    ],
  );
  ran(here!(), &at, &[]);
}

#[test]
fn the_configs_of_the_user_and_of_the_project_give_the_classes_and_the_list_of_automatic_work() {
  if let Some(at) = child() {
    let roster = |directory: &Path| {
      let names = ["claude-cli:haiku", "claude-cli:opus", "claude-cli:sonnet", "claude-cli:fable"];
      let opening = Opening {
        directory: directory.display().to_string(),
        roster: Some(names.map(str::to_owned).to_vec().into()),
        actor: Some("claude-cli:haiku".to_owned()),
        ..opened(&at)
      };
      let mut engine = opening.boot::<String>([]).expect("the life opens").0;
      let names = engine.standing().expect("the life stands").py_repr();
      names
        .split("], ['")
        .map(|one| one.rsplit("], ").next().unwrap_or_default().to_owned())
        .collect::<Vec<_>>()
    };
    // The catalog gives haiku the class cheap and the others dear, the user turns haiku dear and
    // opus cheap, and the project turns opus dear again; the list of the user ranks sonnet first
    // and opus after it.
    assert_eq!(
      roster(&at.join("project"))[..4],
      ["200000, 'dear'", "1000000, 'dear', 2", "1000000, 'dear', 0", "1000000, 'dear'"]
        .map(str::to_owned)
    );
    assert_eq!(
      roster(&at)[..4],
      ["200000, 'dear'", "1000000, 'cheap', 2", "1000000, 'dear', 0", "1000000, 'dear'"]
        .map(str::to_owned)
    );
    fs::remove_file(at.join("config/config.json")).expect("the config of the user goes");
    assert_eq!(roster(&at)[..2], ["200000, 'cheap'", "1000000, 'dear'"].map(str::to_owned));
    return;
  }
  let mine = json!({"classes": {"claude-cli:haiku": "dear", "claude-cli:opus": "cheap"},
    "automatic": ["claude-cli:sonnet", "ghost", "claude-cli:opus"]});
  let project = json!({"classes": {"claude-cli:opus": "dear"}});
  let at =
    holding("configured", &[("config/config.json", mine), ("project/.furb/config.json", project)]);
  ran(here!(), &at, &[]);
}

#[test]
fn a_reply_comes_to_the_turn_of_the_model_with_its_usage_its_dollars_and_its_blocks() {
  if let Some(at) = child() {
    let (mut engine, got) = prompted(&at, "a:c", None);
    assert_eq!(got.as_ref().as_int(), Some(3));
    // The price counts each token once: ten read fresh, five written, twenty read from the cache
    // and three written to it.
    let expected =
      "('assistant', 'close(3)', (33, 5, 20, 3, 0.235), [{'type': 'text', 'text': 'close(3)'}])";
    assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), expected.into()));
    return;
  }
  let usage = json!({"input_tokens": 10, "output_tokens": 1, "cache_read_input_tokens": 20,
    "cache_creation_input_tokens": 3});
  let events = [
    json!({"type": "message_start", "message": {"id": "m", "type": "message", "role": "assistant", "model": "c",
      "content": [], "stop_reason": null, "stop_sequence": null, "usage": usage}}),
    json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
    json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "close(3)"}}),
    json!({"type": "content_block_stop", "index": 0}),
    json!({"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": null},
      "usage": {"output_tokens": 5, "input_tokens": 10, "cache_read_input_tokens": 20,
        "cache_creation_input_tokens": 3}}),
    json!({"type": "message_stop"}),
  ];
  let event =
    |one: &Value| format!("event: {}\ndata: {one}\n\n", one["type"].as_str().unwrap_or_default());
  let (port, asked_of) = served(vec![events.iter().map(event).collect()]);
  let cost = json!({"input": 1e4, "output": 2e4, "cache_read": 1e3, "cache_write": 5e3});
  let a = json!({"protocol": "anthropic", "url": format!("http://127.0.0.1:{port}"), "key": "k",
    "models": {"c": {"window": 1000, "output": 100, "cost": cost}}});
  let at = configured("turn", &json!({"a": a}));
  ran(here!(), &at, &[]);
  let [request] = &asked_of.join().expect("the provider read the request")[..] else {
    panic!("one request")
  };
  let body = body(request);
  assert_eq!(body["system"][0]["text"], SYSTEM, "{body}");
  let user = body["messages"].as_array().and_then(|all| all.last()).cloned().unwrap_or_default();
  assert!(user.to_string().contains("count"), "the turn holds the message of the prompt: {user}");
}

/// The parts of a thought of six parts, then the word `close(3)`, as a local server streams them.
pub(super) fn six(word: bool) -> String {
  let parts = ["a", "b", "c", "d", "e", "f"]
    .map(|one| chunk(&json!({"reasoning_content": one}), &Value::Null));
  if word { [parts.concat(), worded("close(3)")].concat() } else { parts.concat() }
}

/// The word of a turn with no thought, as chat completions stream it.
fn worded(word: &str) -> String {
  [chunk(&json!({"role": "assistant", "content": word}), &json!("stop")), ended(5, None)].concat()
}

/// The first request that the local server of a test reads, and the thought that the request after
/// it gives back when there is one, after these answers, on these values of `max_thinking_tokens`
/// in the config and the local config of the project and on this one of the environment.
fn thought(
  test: &str,
  configs: [Option<Value>; 2],
  env: Option<&str>,
  answers: Vec<String>,
) -> (Value, Value) {
  let (at, url, asked_of) = local("thought", None, answers);
  for (path, value) in [".furb/config.json", ".furb/config.local.json"].into_iter().zip(configs) {
    let Some(value) = value else { continue };
    fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
    fs::write(at.join(path), json!({"max_thinking_tokens": value}).to_string()).expect("a config");
  }
  let mut vars = vec![("FURB_LOCAL_URL", url.as_str())];
  vars.extend(env.map(|most| ("MAX_THINKING_TOKENS", most)));
  ran(test, &at, &vars);
  let got = asked_of.join().expect("the server read the requests");
  let again = got.get(2).map_or(Value::Null, |one| last(one)["reasoning_content"].clone());
  (body(&got[1]), again)
}

/// Whether a request turns the thought of its model off.
fn off(request: &Value) -> bool {
  request["chat_template_kwargs"] == json!({"enable_thinking": false})
}

#[test]
fn the_most_thought_of_a_turn_is_that_of_the_environment_else_that_of_the_config_else_32768() {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  // The most of 32768 itself, and that the claude command line inherits a most of the environment,
  // show to the claude command line, which
  // the_claude_command_line_is_told_the_most_thought_of_the_provider_unless_the_environment_gave_it
  // proves.
  let (first, _) = thought(here!(), [None, None], None, vec![six(true)]);
  assert!(!off(&first), "a most that no one gives cuts no thought of six parts: {first}");
  let (first, _) = thought(here!(), [Some(json!(0)), None], None, vec![worded("close(3)")]);
  assert!(off(&first), "the config gives it: {first}");
  let (_, again) =
    thought(here!(), [Some(json!(0)), None], Some("4"), vec![six(false), worded("close(3)")]);
  assert_eq!(again, json!("abcde"), "the environment wins over the config");
  let (first, _) = thought(here!(), [Some(json!(0)), None], Some("much"), vec![worded("close(3)")]);
  assert!(off(&first), "a value that is no count is as unsaid: {first}");
}

#[test]
fn the_local_config_of_the_project_gives_the_most_thought_over_its_config() {
  if let Some(at) = child() {
    let (mut engine, id) = on_local(&at, "high");
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let (_, again) =
    thought(here!(), [Some(json!(0)), Some(json!(3))], None, vec![six(false), worded("close(3)")]);
  assert_eq!(again, json!("abcd"), "the local config wins");
  let (first, _) =
    thought(here!(), [Some(json!(0)), Some(json!("much"))], None, vec![worded("close(3)")]);
  assert!(off(&first), "a local value that is no count is as unsaid: {first}");
}

/// The config of the project of the yard, with a provider `p` of chat completions at this port,
/// which offers these models.
fn provide(at: &Path, port: u16, models: &[&str]) {
  let models: serde_json::Map<String, Value> =
    models.iter().map(|one| ((*one).to_owned(), json!({"window": 1000}))).collect();
  let p = json!({"protocol": "chat", "url": format!("http://127.0.0.1:{port}/v1"), "key": "k",
    "models": models});
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  let config = json!({"providers": {"p": p}}).to_string();
  fs::write(at.join(".furb/config.json"), config).expect("the config of the project");
}

/// The life of the child on the provider `p` of the config of its project, whose roster names
/// these models, the first its actor.
fn on_p(at: &Path, roster: &[&str]) -> Engine {
  let opening = Opening {
    actor: Some(roster[0].to_owned()),
    roster: Some(roster.iter().map(|one| (*one).to_owned()).collect::<Vec<_>>().into()),
    ..opened(at)
  };
  opening.boot::<String>([]).expect("the life opens").0
}

/// What a stand of the root came to, once it is done.
fn stood(engine: &mut Engine) -> Object {
  let root = engine.root().to_owned();
  let stand = engine.stand(verbs::Stand { on: Some(root) }).expect("the life stands");
  let stand = stand.id().to_owned();
  block_on(Act::<Object>::of(engine, &stand)).unwrap_or_else(|fault| fault.object())
}

#[test]
fn a_refresh_that_reads_no_settings_refuses_the_stand_with_its_cause() {
  let Some(at) = child() else {
    return ran(here!(), &yard("unrefreshed"), &[]);
  };
  let (port, _) = served(Vec::new());
  provide(&at, port, &["m"]);
  let mut engine = on_p(&at, &["p:m"]);
  // A provider of the settings with no address reads as no settings.
  fs::write(at.join(".furb/config.json"), r#"{"providers": {"p": {"protocol": "chat"}}}"#)
    .expect("the config of the project");
  let refused =
    "{'is': 'Refused', 'args': ['A custom provider p needs a URL and at least one model']}";
  assert_eq!(stood(&mut engine).py_repr(), refused);
}

#[test]
fn a_stand_on_new_settings_asks_a_reply_that_waits_for_a_usage_limit_again_on_the_new_command_line()
{
  let Some(at) = child() else {
    return ran(here!(), &yard("limit-switch"), &[]);
  };
  let later =
    time::SystemTime::now().duration_since(time::UNIX_EPOCH).expect("a time").as_secs() + 86_400;
  let body = json!({"error": {"type": "usage_limit_reached", "resets_at": later}}).to_string();
  let refused = format!(
    "HTTP/1.1 429 Too Many Requests\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
     connection: close\r\n\r\n{body}",
    body.len()
  );
  let (old, before) = served(vec![refused]);
  let (new, after) = served(vec![worded("close(7)")]);
  provide(&at, old, &["m"]);
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let kept = Arc::clone(&told);
  let retries: Retries = Arc::new(move |_, _, retry| {
    kept.lock().expect("the retries told").push(retry.why.clone());
  });
  let opening = Opening { retrying: Some(Retrying(retries)), ..opened(&at) };
  let opening = Opening {
    actor: Some("p:m".to_owned()),
    roster: Some(vec!["p:m".to_owned()].into()),
    ..opening
  };
  let mut engine = opening.boot::<String>([]).expect("the life opens").0;
  let id = asked(&mut engine);
  let heard = Arc::clone(&told);
  assert!(until(&mut engine, move |_| !heard.lock().expect("the retries told").is_empty()));
  let why = told.lock().expect("the retries told")[0].clone();
  assert!(why.starts_with("usage limit until "), "{why}");
  assert_eq!(before.join().expect("the old provider read the request").len(), 1);
  provide(&at, new, &["m"]);
  stood(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7), "the reply asks the new provider");
  assert_eq!(after.join().expect("the new provider read the request").len(), 1);
}

#[test]
fn each_request_of_a_chain_starts_with_the_request_before_it_across_a_stand_an_edit_and_a_fork() {
  let Some(at) = child() else {
    return ran(here!(), &yard("prefix"), &[]);
  };
  let words = ["x = 1\ndebug(t'{x}')", "close(1)", "close(2)", "close(3)", "close(4)", "close(5)"];
  let (port, asked_of) = served(words.map(worded).to_vec());
  provide(&at, port, &["m"]);
  let mut engine = on_p(&at, &["p:m", "p:n"]);
  let root = engine.root().to_owned();
  let first = counted(&mut engine, &root, 1);
  // The stand reads the settings again, where the roster grows by a second model.
  provide(&at, port, &["m", "n"]);
  stood(&mut engine);
  counted(&mut engine, &root, 2);
  let door = Text::new(first, "x = 4\ndebug(t'{x}')");
  engine.write(&door, verbs::Write { on: Some(root.clone()) }).expect("the door is written");
  counted(&mut engine, &root, 3);
  let fork =
    verbs::Chain { label: Some("side".to_owned()), source: on(&root), ..Default::default() };
  let side = engine.chain(fork).expect("the fork is made").id().to_owned();
  counted(&mut engine, &side, 4);
  counted(&mut engine, &root, 5);
  let requests: Vec<Vec<Value>> = asked_of
    .join()
    .expect("the provider read the requests")
    .iter()
    .map(|one| body(one)["messages"].as_array().cloned().unwrap_or_default())
    .collect();
  assert_eq!(requests.len(), words.len(), "each word answers one request");
  let holds = |at: usize, word: &str| {
    requests[at].iter().filter(|one| one.to_string().contains(word)).count()
  };
  assert_eq!(holds(2, "stand()"), holds(1, "stand()") + 1, "the stand plays its rung on the root");
  assert_eq!((holds(2, "x = 4"), holds(3, "x = 4")), (0, 1), "the edit tells its word on the root");
  // The fork is asked fifth: it reads what the root read before it, then its own turns.
  for (before, after) in [(0, 1), (1, 2), (2, 3), (3, 5), (3, 4)] {
    let (was, now) = (&requests[before], &requests[after]);
    assert!(now.len() > was.len(), "request {after} grows from request {before}");
    assert_eq!(now[..was.len()], was[..], "request {after} starts with request {before}");
  }
}
