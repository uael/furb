//! The provider of models in a life of the real engine: models of rig that answer from a script, a model that never
//! answers, and the claude command line of the tests.

use std::{
  fs,
  future::Future,
  io::{BufRead, BufReader, Read, Write},
  net::TcpListener,
  path::PathBuf,
  pin::pin,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  task::{Context, Poll, Wake, Waker},
  thread,
  time::{Duration, Instant},
};

use futures::FutureExt;
use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse, Usage},
  message::{AssistantContent, Message, Reasoning, UserContent},
  streaming::{StreamFinal, StreamingCompletionResponse},
  test_utils::{MockCompletionModel, MockError, MockStreamEvent},
};
use serde_json::{Value, json};

use super::{Hosted, Model, Provider, Writes, catalog::Catalog, images};
use crate::{Act, Ear, Engine, Object, SYSTEM, verbs, world};

/// A waker that unparks the thread of the test, so a voice spoken from another thread wakes the poll.
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

/// A yard of one test, empty.
fn yard(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-provider-{name}"));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).expect("a yard of the test");
  at
}

/// A life of the real engine on a provider in a yard of its own, and the ears of the World.
fn lived(provider: Provider) -> Engine {
  let ears: [(&str, Box<dyn Ear>); 4] = [
    ("provider", provider.ear()),
    ("files", world::files()),
    ("bash", world::bash()),
    ("time", world::time()),
  ];
  Engine::boot(Vec::<Object>::new(), ears).expect("the life opens")
}

/// A life of the real engine on the provider of these models and the ears of the World, in a yard of its own.
fn life(name: &str, models: Vec<Model>) -> (Engine, PathBuf) {
  let at = yard(name);
  (lived(Provider::new(at.display().to_string(), models)), at)
}

/// The model of the tests, which answers from its script, with two efforts.
fn scripted(model: &MockCompletionModel) -> Model {
  Model::new("m", 1000, model.clone())
    .effort("low", json!({"thinking": 1024}))
    .effort("high", json!({"thinking": 8192}))
}

/// A model of rig that streams these turns, each its text and the usage it ends with.
fn streaming(turns: &[(&str, Usage)]) -> MockCompletionModel {
  MockCompletionModel::from_stream_turns(turns.iter().map(|(text, usage)| {
    [MockStreamEvent::text(*text), MockStreamEvent::FinalResponse(StreamFinal::new("mock", *usage))]
  }))
}

fn on(root: &str) -> Option<String> {
  Some(root.to_owned())
}

/// A prompt on the root that wants an int.
fn asked(engine: &mut Engine) -> String {
  let root = engine.root().to_owned();
  let with =
    verbs::Prompt { message: Some("count".to_owned()), on: on(&root), ..Default::default() };
  engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned()
}

/// The facts of the root that the provider said about replies and chains: each kind, what it is about, and the
/// representation of its first word.
fn said(engine: &mut Engine) -> Vec<(String, String, String)> {
  let root = engine.root().to_owned();
  let facts =
    engine.transcript(verbs::Transcript { on: on(&root) }).expect("the root has a transcript");
  facts
    .iter()
    .filter(|one| one.by() == "provider" && !one.about().starts_with("stand"))
    .map(|one| {
      let word = one.word(0).map(|one| one.py_repr()).unwrap_or_default();
      (one.kind().to_owned(), one.about().to_owned(), word)
    })
    .collect()
}

/// The engine driven until a condition of it holds, or ten seconds are up.
fn until(engine: &mut Engine, holds: impl Fn(&mut Engine) -> bool) -> bool {
  within(engine, Duration::from_secs(10), holds)
}

/// The engine driven until a condition of it holds, or a time is up.
fn within(engine: &mut Engine, time: Duration, holds: impl Fn(&mut Engine) -> bool) -> bool {
  let waker = Waker::from(Arc::new(Parked(thread::current())));
  let end = Instant::now() + time;
  while Instant::now() < end {
    engine.pump(&waker).expect("the engine is driven");
    if holds(engine) {
      return true;
    }
    thread::park_timeout(Duration::from_millis(20));
  }
  false
}

#[test]
fn the_provider_answers_a_stand_with_its_roster_its_directory_and_its_default_actor() {
  let (mut engine, at) = life("stand", vec![scripted(&MockCompletionModel::new([]))]);
  let standing = engine.standing().expect("the life stands");
  let expected = format!(
    "[[['m', ['low', 'high'], 1000], ['operator', [], 200000]], '{}', 'm/low']",
    at.display()
  );
  assert_eq!(standing.py_repr(), expected);
}

#[test]
fn a_reply_comes_to_the_turn_of_the_model_with_its_usage_its_dollars_and_its_blocks() {
  let mut usage = Usage::new();
  (usage.input_tokens, usage.output_tokens, usage.total_tokens) = (33, 5, 38);
  (usage.cached_input_tokens, usage.cache_creation_input_tokens) = (20, 3);
  let model = streaming(&[("close(3)", usage)]);
  let (mut engine, _) = life("turn", vec![scripted(&model).price(1e4, 2e4, 1e3, 5e3)]);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  // The price counts each token once: ten read fresh, five written, twenty read from the cache and three written to
  // it.
  let expected =
    "('assistant', 'close(3)', (33, 5, 20, 3, 0.235), [{'type': 'text', 'text': 'close(3)'}])";
  assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), expected.into()));
  let request = model.requests().remove(0);
  assert_eq!(request.preamble.as_deref(), Some(SYSTEM));
  assert_eq!(request.additional_params, Some(json!({"thinking": 1024})));
  let [Message::User { content }] = &request.chat_history[..] else {
    panic!("one user turn: {:?}", request.chat_history)
  };
  assert!(format!("{content:?}").contains("count"), "the turn holds the message of the prompt");
}

#[test]
fn an_assistant_turn_goes_again_as_the_blocks_its_provider_gave() {
  let thought = Reasoning::new_with_signature("hmm", Some("sig".into()));
  // The stream says the id of the thought, which the turn keeps and the provider reads again.
  let thought = AssistantContent::Reasoning(Reasoning { id: Some("r".into()), ..thought });
  let first = [thought, AssistantContent::text("x = 1")];
  let reasoning =
    rig_core::message::ReasoningContent::Text { text: "hmm".into(), signature: Some("sig".into()) };
  let turns = [
    vec![
      MockStreamEvent::Reasoning { id: "r".into(), content: reasoning },
      MockStreamEvent::text("x = 1"),
    ],
    vec![MockStreamEvent::text("close(x)")],
  ];
  let model = MockCompletionModel::from_stream_turns(turns);
  let (mut engine, _) = life("again", vec![scripted(&model)]);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(1));
  let requests = model.requests();
  assert_eq!(requests.len(), 2);
  let assistant = requests[1].chat_history.iter().find_map(|one| match one {
    Message::Assistant { content, .. } => Some(content.clone()),
    _ => None,
  });
  assert_eq!(assistant.as_deref(), Some(&first[..]));
}

#[test]
fn a_second_refusal_in_a_row_of_an_actor_on_a_chain_pauses_the_chain_before_its_done() {
  let failed = |why: &str| vec![MockStreamEvent::Error(MockError::provider(why))];
  let model = MockCompletionModel::from_stream_turns([failed("boom"), failed("boom again")]);
  let (mut engine, _) = life("refusals", vec![scripted(&model)]);
  let root = engine.root().to_owned();
  asked(&mut engine);
  assert!(until(&mut engine, |engine| said(engine).len() == 5), "{:?}", said(&mut engine));
  let refused =
    |why: &str| format!("Refused(args=('m/low answered nothing: ProviderError: {why}'))");
  let expected = [
    ("started", "reply1", String::new()),
    ("done", "reply1", refused("boom")),
    ("started", "reply2", String::new()),
    ("pause", root.as_str(), format!("['#{root} paused']")),
    ("done", "reply2", refused("boom again")),
  ];
  let expected: Vec<_> =
    expected.into_iter().map(|(kind, about, word)| (kind.into(), about.into(), word)).collect();
  assert_eq!(said(&mut engine), expected);
  let asked_again = within(&mut engine, Duration::from_millis(300), |_| model.request_count() > 2);
  assert!(!asked_again, "the paused chain asks no model");
}

/// A model that never answers, and says when it is asked and when its call is dropped.
#[derive(Clone, Default)]
struct Silent {
  asked: Arc<AtomicBool>,
  dropped: Arc<AtomicBool>,
}

/// What sets a flag when it goes.
struct Flag(Arc<AtomicBool>);

impl Drop for Flag {
  fn drop(&mut self) {
    self.0.store(true, Ordering::SeqCst);
  }
}

impl CompletionModel for Silent {
  async fn completion(&self, _: CompletionRequest) -> Result<CompletionResponse, CompletionError> {
    Err(CompletionError::ProviderError("the provider streams".into()))
  }

  async fn stream(
    &self,
    _: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let _flag = Flag(Arc::clone(&self.dropped));
    self.asked.store(true, Ordering::SeqCst);
    std::future::pending().await
  }
}

#[test]
fn a_cancel_over_a_reply_ends_the_call_of_its_model_and_the_ear_says_nothing_more_of_it() {
  let model = Silent::default();
  let (mut engine, _) = life("cancel", vec![Model::new("m", 1000, model.clone())]);
  let id = asked(&mut engine);
  assert!(until(&mut engine, |_| model.asked.load(Ordering::SeqCst)), "the model is asked");
  engine.cancel(&id).expect("the prompt is cancelled");
  let no = block_on(Act::<Object>::of(&mut engine, "reply1")).expect_err("the reply ends");
  assert_eq!(no.name, "CancelledError");
  assert!(
    until(&mut engine, |_| model.dropped.load(Ordering::SeqCst)),
    "the call of the model ended"
  );
  let dones = said(&mut engine).into_iter().filter(|(kind, ..)| kind == "done").count();
  assert_eq!(dones, 0, "the provider said no done of the reply a control ended");
}

#[cfg(unix)]
#[test]
fn the_replies_of_a_chain_keep_one_process_of_the_claude_command_line() {
  use super::claude::test::Yard;
  let yard = Yard::new("chain");
  let claude = yard.claude(10_000);
  let (mut engine, _) = life("claude", claude.models());
  let root = engine.root().to_owned();
  let mut answer = |message: &str| {
    let with = verbs::Prompt { message: Some(message.into()), on: on(&root), ..Default::default() };
    let act = engine.prompt(Object::string("str"), with).expect("a prompt is made");
    block_on(act).map(|got| got.py_repr())
  };
  assert_eq!(answer("first task").expect("the first prompt"), "'reply 1'");
  assert_eq!(answer("second task").expect("the second prompt"), "'reply 2'");
  let pids = yard.pids();
  assert_eq!(pids.len(), 1, "one process holds the conversation of the chain");
  let args = yard.args(&pids[0]);
  let after = |flag: &str| args.iter().position(|one| one == flag).map(|at| args[at + 1].as_str());
  assert_eq!((after("--model"), after("--effort")), (Some("opus"), Some("low")));
  assert_eq!(after("--system-prompt"), Some(SYSTEM));
  let heard = yard.heard();
  assert_eq!(heard.len(), 2);
  assert!(heard[1].contains("second task") && !heard[1].contains("first task"), "{}", heard[1]);
  drop((engine, claude));
  assert!(
    super::claude::test::gone(&pids[0]),
    "the process ends with the last hold of the command line"
  );
}

/// A model that writes a thought and a word in parts.
fn writing() -> MockCompletionModel {
  MockCompletionModel::from_stream_turns([[
    MockStreamEvent::ReasoningDelta { id: "r".into(), reasoning: "let me".into() },
    MockStreamEvent::text("close("),
    MockStreamEvent::text("3)"),
  ]])
}

/// Whom a test tells what a model writes, and what it was told, each as its rung, its chain, its text and its
/// thought.
fn heard() -> (Writes, Arc<Mutex<Vec<[String; 4]>>>) {
  let heard: Arc<Mutex<Vec<[String; 4]>>> = Arc::default();
  let kept = Arc::clone(&heard);
  let writes: Writes = Arc::new(move |rung, chain, text, thinking| {
    kept.lock().expect("the parts told").push([rung, chain, text, thinking].map(str::to_owned));
  });
  (writes, heard)
}

#[test]
fn what_a_model_writes_reaches_the_host_as_it_writes_it_under_its_rung_and_its_chain() {
  let (writes, told) = heard();
  let at = yard("writes");
  let provider = Provider::new(at.display().to_string(), vec![scripted(&writing())]).writes(writes);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let root = engine.root().to_owned();
  let parts = [["", "let me"], ["close(", ""], ["3)", ""]]
    .map(|[text, thinking]| ["rung1", root.as_str(), text, thinking].map(str::to_owned));
  assert_eq!(told.lock().expect("the parts told").clone(), parts);
}

#[test]
fn a_turn_hands_its_images_to_a_model_that_takes_them_and_a_model_that_takes_none_refuses_it() {
  let at = yard("images");
  fs::write(at.join("a.png"), b"\x89PNG\r\n\x1a\nimage").expect("the image is written");
  let kept = images::attach(&at.join("images"), &at.join("a.png")).expect("the image is attached");
  let message = format!("look at {}", images::reference("a.png", &kept.uri));
  let prompted = |engine: &mut Engine| {
    let root = engine.root().to_owned();
    let with =
      verbs::Prompt { message: Some(message.clone()), on: on(&root), ..Default::default() };
    engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned()
  };
  let model = streaming(&[("close(1)", Usage::new())]);
  let provider = Provider::new(at.display().to_string(), vec![scripted(&model).images()])
    .images(at.join("images"));
  let mut engine = lived(provider);
  let id = prompted(&mut engine);
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  let [Message::User { content }] = &model.requests()[0].chat_history[..] else {
    panic!("one user turn")
  };
  assert!(matches!(
    content.iter().collect::<Vec<_>>()[..],
    [UserContent::Text(_), UserContent::Image(_)]
  ));
  let blind = streaming(&[]);
  let provider =
    Provider::new(at.display().to_string(), vec![scripted(&blind)]).images(at.join("images"));
  let mut engine = lived(provider);
  prompted(&mut engine);
  assert!(until(&mut engine, |engine| said(engine).len() >= 2), "{:?}", said(&mut engine));
  let refused =
    "Refused(args=('m/low answered nothing: ProviderError: m does not accept images.'))";
  assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), refused.into()));
  assert_eq!(blind.request_count(), 0, "the model is asked nothing");
}

#[test]
fn a_function_of_the_host_answers_a_request_with_a_turn_in_place_of_the_model() {
  let requests: Arc<Mutex<Vec<Value>>> = Arc::default();
  let kept = Arc::clone(&requests);
  let host: Hosted = Arc::new(move |request, told| {
    kept.lock().expect("the requests").push(request);
    told("close(7)", "");
    let usage = Object::list([10, 2, 0, 0, 1].map(Object::int));
    let turn = [Object::string("assistant"), Object::string("close(7)"), usage, Object::none()];
    async move { Ok(Object::list(turn)) }.boxed()
  });
  let (writes, told) = heard();
  let at = yard("hosted");
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let mut engine = lived(Provider::new(at.display().to_string(), vec![model]).writes(writes));
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7));
  let root = engine.root().to_owned();
  let request = requests.lock().expect("the requests")[0].clone();
  assert_eq!((&request["actor"], &request["chain"]), (&json!("m/low"), &json!(root)));
  assert_eq!(request["settings"], json!({"thinking": 1024}));
  assert_eq!(request["messages"][0]["role"], "user");
  assert!(
    request["messages"][0]["content"][0]["text"]
      .as_str()
      .is_some_and(|text| text.contains("#prompt1 count"))
  );
  let expected = "('assistant', 'close(7)', (10, 2, 0, 0, 1.0), None)";
  assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), expected.into()));
  assert_eq!(told.lock().expect("the parts told").len(), 1);
}

/// A provider of chat completions on a port of this machine, which reads one request and streams one turn: a thought,
/// a word in two parts, and its usage. It gives the request it read.
fn served(listener: TcpListener) -> std::thread::JoinHandle<String> {
  std::thread::spawn(move || {
    let (stream, _) = listener.accept().expect("the provider is asked");
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    while !head.ends_with("\r\n\r\n") {
      reader.read_line(&mut head).expect("the head of the request");
    }
    let length = head
      .lines()
      .find_map(|line| {
        line.to_ascii_lowercase().strip_prefix("content-length: ").map(str::to_owned)
      })
      .and_then(|length| length.trim().parse::<usize>().ok())
      .unwrap_or_default();
    let mut body = vec![0; length];
    reader.read_exact(&mut body).expect("the body of the request");
    let chunk = |delta: Value, finish: Value| {
      let one = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "m",
        "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]});
      format!("data: {one}\n\n")
    };
    let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "m", "choices": [],
      "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105,
        "prompt_tokens_details": {"cached_tokens": 40}}});
    let events = [
      chunk(json!({"role": "assistant", "reasoning_content": "hm"}), Value::Null),
      chunk(json!({"content": "close("}), Value::Null),
      chunk(json!({"content": "3)"}), json!("stop")),
      format!("data: {usage}\n\ndata: [DONE]\n\n"),
    ];
    let mut stream = reader.into_inner();
    let head_out =
      "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
    stream
      .write_all(format!("{head_out}{}", events.concat()).as_bytes())
      .expect("the turn is streamed");
    format!("{head}{}", String::from_utf8_lossy(&body))
  })
}

#[test]
fn a_provider_of_the_network_is_asked_with_its_credential_and_streams_its_turn() {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let asked_of = served(listener);
  let listed = json!({"local": {"rig": "chat", "env": ["FURB_TEST_KEY"], "api": format!("http://127.0.0.1:{port}/v1"),
    "models": {"m": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}],
      "limit": {"context": 1000}, "cost": {"input": 1e4, "output": 2e4},
      "modalities": {"input": ["text"], "output": ["text"]}}}}});
  let catalog = Catalog::of(&super::catalog::parsed(&listed.to_string(), None), None, |name| {
    (name == "FURB_TEST_KEY").then(|| "sekrit".to_owned())
  });
  let (models, actor) = catalog.roster(None, Some("m/medium")).expect("the model is offered");
  assert_eq!(actor.as_deref(), Some("local:m/high"));
  let (writes, told) = heard();
  let at = yard("network");
  let mut engine =
    lived(Provider::new(at.display().to_string(), models).actor(actor).writes(writes));
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let request = asked_of.join().expect("the provider read the request");
  assert!(request.starts_with("POST /v1/chat/completions "), "{request}");
  assert!(request.to_ascii_lowercase().contains("authorization: bearer sekrit"), "{request}");
  let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default())
    .expect("a JSON body");
  assert_eq!(
    (&body["model"], &body["reasoning_effort"], &body["stream"]),
    (&json!("m"), &json!("high"), &json!(true))
  );
  assert_eq!(body["messages"][0]["content"][0]["text"], SYSTEM);
  let expected = "('assistant', 'close(3)', (100, 5, 40, 0, 0.7), [{'type': 'reasoning', ";
  assert!(said(&mut engine)[1].2.starts_with(expected), "{:?}", said(&mut engine)[1]);
  let parts: Vec<[String; 4]> = told.lock().expect("the parts told").clone();
  let texts: Vec<(&str, &str)> =
    parts.iter().map(|[_, _, text, thinking]| (text.as_str(), thinking.as_str())).collect();
  assert_eq!(texts, [("", "hm"), ("close(", ""), ("3)", "")]);
}
