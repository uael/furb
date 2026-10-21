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
    atomic::{AtomicBool, AtomicUsize, Ordering},
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

use super::{Hosted, Model, Provider, Writes, catalog::Catalog, images, local::Server, unfenced};
use crate::{Act, Ear, Engine, Fact, Object, SYSTEM, verbs, world};

/// A waker that unparks the thread of the test, so what an ear waits for wakes the poll when it is ready.
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
  let at = std::env::temp_dir().join(format!("furb-provider-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).expect("a yard of the test");
  at
}

/// A life of the real engine on a provider in a yard of its own, and the ears of the World.
fn lived(provider: Provider) -> Engine {
  let scratch = world::Scratch::own();
  let ears: [(&str, Box<dyn Ear>); 4] = [
    ("provider", provider.ear()),
    ("files", world::files(scratch.clone())),
    ("bash", world::bash(scratch)),
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
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None })
    .expect("the root has a transcript");
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
  // The default actor is the first model, at the level of an actor that names none. Python shows each backslash of a path of Windows twice.
  let expected = format!(
    "[[['m', ['low', 'high'], 1000], ['operator', [], 200000]], '{}', 'm/high']",
    at.display().to_string().replace('\\', "\\\\")
  );
  assert_eq!(standing.py_repr(), expected);
}

#[test]
fn the_entry_of_a_model_of_a_family_that_reads_a_doctrine_holds_that_doctrine_after_its_window() {
  let qwen = Model::new("local:qwen3", 1000, MockCompletionModel::new([]));
  let (mut engine, _) = life("doctrine", vec![qwen, scripted(&MockCompletionModel::new([]))]);
  let standing = engine.standing().expect("the life stands");
  let taught =
    Object::string(super::catalog::doctrine("local:qwen3").expect("qwen reads a doctrine"))
      .py_repr();
  let expected = format!(
    "[[['local:qwen3', [], 1000, {taught}], ['m', ['low', 'high'], 1000], ['operator', [], 200000]]"
  );
  assert!(standing.py_repr().starts_with(&expected), "{}", standing.py_repr());
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
  assert_eq!(request.additional_params, Some(json!({"thinking": 8192})));
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
  assert!(until(&mut engine, |engine| said(engine).len() == 6), "{:?}", said(&mut engine));
  let refused =
    |why: &str| format!("Refused(args=('m/high answered nothing: ProviderError: {why}'))");
  let expected = [
    ("started", "reply1", String::new()),
    ("done", "reply1", refused("boom")),
    ("started", "reply2", String::new()),
    ("tell", root.as_str(), format!("['#{root} paused']")),
    ("pause", root.as_str(), String::new()),
    ("done", "reply2", refused("boom again")),
  ];
  let expected: Vec<_> =
    expected.into_iter().map(|(kind, about, word)| (kind.into(), about.into(), word)).collect();
  assert_eq!(said(&mut engine), expected);
  let asked_again = within(&mut engine, Duration::from_millis(300), |_| model.request_count() > 2);
  assert!(!asked_again, "the paused chain asks no model");
}

#[test]
fn a_function_of_the_host_that_refuses_later_twice_in_a_row_pauses_the_chain_and_is_asked_no_more()
{
  let calls = Arc::new(AtomicUsize::new(0));
  let counted = Arc::clone(&calls);
  let host: Hosted = Arc::new(move |_, _| {
    counted.fetch_add(1, Ordering::SeqCst);
    // It refuses once the life is driven again, so the ear says so of its own accord.
    async move {
      tokio::time::sleep(Duration::from_millis(1)).await;
      Err("unavailable".to_owned())
    }
    .boxed()
  });
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let mut engine = lived(Provider::new(yard("refused").display().to_string(), vec![model]));
  let root = engine.root().to_owned();
  asked(&mut engine);
  assert!(until(&mut engine, |engine| said(engine).len() == 6), "{:?}", said(&mut engine));
  let kinds: Vec<_> = said(&mut engine).into_iter().map(|(kind, about, _)| (kind, about)).collect();
  let expected = [
    ("started", "reply1"),
    ("done", "reply1"),
    ("started", "reply2"),
    ("tell", root.as_str()),
    ("pause", root.as_str()),
    ("done", "reply2"),
  ];
  assert_eq!(kinds, expected.map(|(kind, about)| (kind.to_owned(), about.to_owned())));
  assert_eq!(calls.load(Ordering::SeqCst), 2, "the paused chain asks the function no more");
}

#[test]
fn a_turn_between_two_refusals_of_an_actor_on_a_chain_ends_their_row() {
  let failed = || vec![MockStreamEvent::Error(MockError::provider("boom"))];
  let turn = |word: &str| vec![MockStreamEvent::text(word)];
  let model = MockCompletionModel::from_stream_turns([
    failed(),
    turn("x = 1"),
    failed(),
    turn("close(x + 2)"),
  ]);
  let (mut engine, _) = life("row", vec![scripted(&model)]);
  let id = asked(&mut engine);
  let answered = until(&mut engine, |engine| engine.outcome(&id).is_ok_and(|got| got.is_some()));
  assert!(answered, "a row of two refusals paused the chain: {:?}", said(&mut engine));
  let got = engine.outcome(&id).expect("the prompt is an act").expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
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
fn the_replies_of_a_chain_keep_one_process_of_the_claude_command_line_and_each_life_its_own() {
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
  assert_eq!((after("--model"), after("--effort")), (Some("opus"), Some("high")));
  let prompt = after("--system-prompt-file").expect("the system prompt goes by a file");
  assert_eq!(std::fs::read_to_string(prompt).expect("the file of the system prompt"), SYSTEM);
  let heard = yard.heard();
  assert_eq!(heard.len(), 2);
  assert!(heard[1].contains("second task") && !heard[1].contains("first task"), "{}", heard[1]);
  // The ids of chains repeat in every life, and a life on the same command line keeps a conversation of its own.
  let (mut other, _) = life("claude-other", claude.models());
  let with =
    verbs::Prompt { message: Some("first task".into()), on: on(&root), ..Default::default() };
  block_on(other.prompt(Object::string("str"), with).expect("a prompt is made"))
    .expect("an answer");
  let session = |pid: &str| super::claude::test::after(&yard.args(pid), "--session-id");
  let both = yard.pids();
  assert_eq!(both.len(), 2);
  assert_ne!(session(&both[0]), session(&both[1]));
  drop((engine, other, claude));
  assert!(
    super::claude::test::gone(&pids[0]),
    "the process ends with the last hold of the command line"
  );
}

#[cfg(unix)]
#[test]
fn the_word_of_the_claude_command_line_is_the_text_it_settled_over_the_parts_it_streamed() {
  use super::claude::test::Yard;
  let yard = Yard::new("gap");
  let claude = yard.claude(10_000);
  let (mut engine, _) = life("claude-gap", claude.models());
  let root = engine.root().to_owned();
  // The command line streams the text of its reply less its first part, and then says the text whole.
  let with =
    verbs::Prompt { message: Some("GAP task".into()), on: on(&root), ..Default::default() };
  let act = engine.prompt(Object::string("str"), with).expect("a prompt is made");
  assert_eq!(block_on(act).map(|got| got.py_repr()).expect("an answer"), "'reply 1'");
  assert_eq!(yard.heard().len(), 1, "the first reply answered the prompt");
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
    "Refused(args=('m/high answered nothing: ProviderError: m does not accept images.'))";
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
  assert_eq!((&request["actor"], &request["chain"]), (&json!("m/high"), &json!(root)));
  assert_eq!(request["settings"], json!({"thinking": 8192}));
  assert_eq!(request["messages"][0]["role"], "user");
  assert!(
    request["messages"][0]["content"][0]["text"]
      .as_str()
      .is_some_and(|text| text.contains("#prompt1\nprompt1_message = 'count'\n"))
  );
  let expected = "('assistant', 'close(7)', (10, 2, 0, 0, 1.0), None)";
  assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), expected.into()));
  assert_eq!(told.lock().expect("the parts told").len(), 1);
}

/// A provider on a port of this machine, which reads one request and streams these events. It gives the request it
/// read.
pub(super) fn served(listener: TcpListener, events: String) -> std::thread::JoinHandle<String> {
  std::thread::spawn(move || exchanged(&listener, &events))
}

/// One request that a provider on a port of this machine reads, and the events it streams in answer; and the request
/// it read.
fn exchanged(listener: &TcpListener, events: &str) -> String {
  // A provider that is never asked, or a request that never ends, fails the test and holds it not.
  listener.set_nonblocking(true).expect("a listener that waits not");
  let end = Instant::now() + Duration::from_secs(10);
  let stream = loop {
    match listener.accept() {
      Ok((stream, _)) => break stream,
      Err(no) if no.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < end => {
        thread::sleep(Duration::from_millis(10));
      }
      Err(no) => panic!("the provider is asked: {no}"),
    }
  };
  stream.set_nonblocking(false).expect("a stream that waits");
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
  let head_out = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
  stream.write_all(format!("{head_out}{events}").as_bytes()).expect("the turn is streamed");
  format!("{head}{}", String::from_utf8_lossy(&body))
}

/// One turn as chat completions stream it: a thought, a word in two parts, and its usage.
fn chatted() -> String {
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
  events.concat()
}

/// A life whose root prompts the one model of a provider of the network, which the catalog makes of this listing
/// with this environment; and what the prompt came to.
fn networked(name: &str, listed: &Value, env: &[(&str, &str)]) -> (Engine, Object) {
  let catalog =
    Catalog::of(&super::catalog::parsed(&listed.to_string(), None), None, None, |name| {
      env.iter().find(|(held, _)| *held == name).map(|(_, value)| (*value).to_owned())
    });
  let (models, actor) = catalog.roster(None, None).expect("the model is offered");
  let mut engine = lived(Provider::new(yard(name).display().to_string(), models).actor(actor));
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).unwrap_or_else(|fault| fault.object());
  (engine, got)
}

/// The value of a header of a request, by its name in lower case.
fn header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
  let head = request.split("\r\n\r\n").next().unwrap_or_default();
  head.lines().find_map(|line| {
    let (key, value) = line.split_once(':')?;
    key.eq_ignore_ascii_case(name).then(|| value.trim())
  })
}

#[test]
fn a_provider_of_the_network_is_asked_with_its_credential_and_streams_its_turn() {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let asked_of = served(listener, chatted());
  let listed = json!({"compat": {"rig": "chat", "env": ["FURB_TEST_KEY"], "api": format!("http://127.0.0.1:{port}/v1"),
    "models": {"m": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}],
      "limit": {"context": 1000}, "cost": {"input": 1e4, "output": 2e4},
      "modalities": {"input": ["text"], "output": ["text"]}}}}});
  let catalog =
    Catalog::of(&super::catalog::parsed(&listed.to_string(), None), None, None, |name| {
      (name == "FURB_TEST_KEY").then(|| "sekrit".to_owned())
    });
  let (models, actor) = catalog.roster(None, Some("m/medium")).expect("the model is offered");
  assert_eq!(actor.as_deref(), Some("compat:m/high"));
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
  let root = engine.root().to_owned();
  let parts = [["", "hm"], ["close(", ""], ["3)", ""]]
    .map(|[text, thinking]| ["rung1", root.as_str(), text, thinking].map(str::to_owned));
  assert_eq!(told.lock().expect("the parts told").clone(), parts);
}

#[test]
fn the_host_is_told_nothing_more_of_what_a_model_writes_once_its_reply_is_over() {
  // A host writes from a work of its own, which the end of the reply does not end, as JavaScript does: here once the
  // test says the reply is over.
  let (go, gone) = std::sync::mpsc::channel::<()>();
  let gone = Arc::new(Mutex::new(gone));
  let host: Hosted = Arc::new(move |_, told| {
    told("before", "");
    let gone = Arc::clone(&gone);
    thread::spawn(move || {
      let _ = gone.lock().expect("the signal of the test").recv();
      told("after", "");
    });
    std::future::pending().boxed()
  });
  let (writes, told) = heard();
  let at = yard("over");
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let mut engine = lived(Provider::new(at.display().to_string(), vec![model]).writes(writes));
  let id = asked(&mut engine);
  let parts = |told: &Arc<Mutex<Vec<[String; 4]>>>| told.lock().expect("the parts told").len();
  assert!(until(&mut engine, |_| parts(&told) == 1), "the model wrote a part");
  engine.cancel(&id).expect("the prompt is cancelled");
  let over = |engine: &mut Engine| {
    let root = engine.root().to_owned();
    let facts =
      engine.transcript(verbs::Transcript { on: on(&root), of: None }).expect("the transcript");
    facts.iter().any(|one| one.kind() == "done" && one.about().starts_with("reply"))
  };
  assert!(until(&mut engine, over), "the reply is over");
  // The ear hears the done of the reply in the pump that says it, and the pumps after it give it room.
  within(&mut engine, Duration::from_millis(100), |_| false);
  go.send(()).expect("the host writes again");
  assert!(
    !within(&mut engine, Duration::from_millis(500), |_| parts(&told) > 1),
    "nothing more is told"
  );
}

#[test]
fn a_bearer_of_anthropic_goes_as_its_authorization_and_no_key_of_an_api_goes_with_it() {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
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
  let events: String = events
    .iter()
    .map(|one| format!("event: {}\ndata: {one}\n\n", one["type"].as_str().unwrap_or_default()))
    .collect();
  let asked_of = served(listener, events);
  let listed = json!({"anthropic": {"rig": "anthropic", "env": ["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"],
    "bearer": ["ANTHROPIC_AUTH_TOKEN"], "api": format!("http://127.0.0.1:{port}"),
    "models": {"c": {"limit": {"context": 1000, "output": 100}, "modalities": {"input": ["text"], "output": ["text"]}}}}});
  let env = [("ANTHROPIC_AUTH_TOKEN", "token"), ("ANTHROPIC_API_KEY", "key")];
  let (_, got) = networked("bearer", &listed, &env);
  assert_eq!(got.as_ref().as_int(), Some(1));
  let request = asked_of.join().expect("the provider read the request");
  assert!(request.starts_with("POST /v1/messages "), "{request}");
  assert_eq!(
    (header(&request, "authorization"), header(&request, "x-api-key")),
    (Some("Bearer token"), None)
  );
}

#[test]
fn the_gateway_of_cloudflare_takes_the_credential_of_cloudflare_alone_at_the_address_of_the_account()
 {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let asked_of = served(listener, chatted());
  let api = format!(
    "http://127.0.0.1:{port}/v1/${{CLOUDFLARE_ACCOUNT_ID}}/${{CLOUDFLARE_GATEWAY_ID}}/compat"
  );
  let listed = json!({"cloudflare-ai-gateway": {"rig": "gateway", "env": ["CLOUDFLARE_API_KEY"], "api": api,
    "models": {"openai/gpt-5": {"limit": {"context": 1000}, "modalities": {"input": ["text"], "output": ["text"]},
      "provider": {"npm": "@ai-sdk/openai"}}}}});
  let env = [
    ("CLOUDFLARE_API_KEY", "cf"),
    ("CLOUDFLARE_ACCOUNT_ID", "acc"),
    ("CLOUDFLARE_GATEWAY_ID", "gw"),
  ];
  let (_, got) = networked("gateway", &listed, &env);
  assert_eq!(got.as_ref().as_int(), Some(3));
  let request = asked_of.join().expect("the provider read the request");
  assert!(request.starts_with("POST /v1/acc/gw/compat/chat/completions "), "{request}");
  let credentials = (header(&request, "cf-aig-authorization"), header(&request, "authorization"));
  assert_eq!(credentials, (Some("Bearer cf"), None));
}

#[test]
fn a_gateway_of_the_protocol_of_pi_ai_streams_the_turn_of_its_model_and_what_it_cost() {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
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
  let events: String = events.iter().map(|one| format!("data: {one}\n\n")).collect();
  let asked_of = served(listener, events);
  let listed = json!({"radius": {"rig": "pi", "env": ["RADIUS_API_KEY"], "api": format!("http://127.0.0.1:{port}/v1"),
    "models": {"balanced": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["high"]}],
      "limit": {"context": 1000, "output": 100}, "modalities": {"input": ["text"], "output": ["text"]}}}}});
  let (mut engine, got) = networked("pi", &listed, &[("RADIUS_API_KEY", "r")]);
  assert_eq!(got.as_ref().as_int(), Some(2));
  let request = asked_of.join().expect("the gateway read the request");
  assert!(request.starts_with("POST /v1/messages "), "{request}");
  assert_eq!(header(&request, "authorization"), Some("Bearer r"));
  let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default())
    .expect("a JSON body");
  assert_eq!((&body["model"], &body["options"]["reasoning"]), (&json!("balanced"), &json!("high")));
  assert_eq!(body["context"]["systemPrompt"], SYSTEM);
  assert!(body["options"]["sessionId"].is_string(), "the conversation of a chain is its session");
  let expected = "('assistant', 'close(2)', (14, 2, 4, 0, 0.25), [";
  assert!(said(&mut engine)[1].2.starts_with(expected), "{:?}", said(&mut engine)[1]);
}

#[test]
fn azure_is_asked_for_the_deployment_of_a_model_at_the_address_and_the_version_of_the_environment()
{
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let asked_of = served(listener, chatted());
  let listed = json!({"azure": {"rig": "azure", "env": ["AZURE_OPENAI_API_KEY"],
    "api": "https://${AZURE_RESOURCE_NAME}.openai.azure.com", "models": {"gpt-5": {"limit": {"context": 1000},
      "modalities": {"input": ["text"], "output": ["text"]}}}}});
  let base = format!("http://127.0.0.1:{port}/openai/v1");
  let env = [
    ("AZURE_OPENAI_API_KEY", "az"),
    ("AZURE_OPENAI_BASE_URL", base.as_str()),
    ("AZURE_OPENAI_DEPLOYMENT_NAME_MAP", "gpt-4=old, gpt-5=mine"),
    ("AZURE_OPENAI_API_VERSION", "2025-01-01"),
  ];
  let (_, got) = networked("azure", &listed, &env);
  assert_eq!(got.as_ref().as_int(), Some(3));
  let request = asked_of.join().expect("azure read the request");
  let asked = "POST /openai/deployments/mine/chat/completions?api-version=2025-01-01 ";
  assert!(request.starts_with(asked), "{request}");
  assert_eq!(header(&request, "api-key"), Some("az"));
}

/// One chunk of the chat completions of a model, with what it adds and why it ended, if it did.
fn chunk(delta: Value, finish: Value) -> String {
  let one = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen",
    "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]});
  format!("data: {one}\n\n")
}

/// A life whose root prompts local:qwen, the one model of a local server of the tests, which lists it with a window of
/// 1000 and streams these events for its one reply; the prompt, and the two requests the server read.
fn local(name: &str, events: String) -> (Engine, String, std::thread::JoinHandle<[String; 2]>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let listing =
    json!({"object": "list", "data": [{"id": "qwen", "object": "model", "meta": {"n_ctx": 1000}}]});
  let asked_of =
    thread::spawn(move || [listing.to_string(), events].map(|one| exchanged(&listener, &one)));
  let address = format!("http://127.0.0.1:{port}");
  let env = |name: &str| (name == "FURB_LOCAL_URL").then(|| address.clone());
  let catalog =
    Catalog::of(&super::catalog::parsed("{}", None), None, Server::named(env).map(Arc::new), env);
  let (models, actor) =
    catalog.roster(None, Some("local:qwen/high")).expect("the model is offered");
  assert_eq!(actor.as_deref(), Some("local:qwen"), "a model of a local server takes no effort");
  assert_eq!(models[0].priced(), Some([0.0; 4]), "a model of a local server costs nothing");
  let mut engine = lived(Provider::new(yard(name).display().to_string(), models).actor(actor));
  let id = asked(&mut engine);
  (engine, id, asked_of)
}

/// Whether a word that holds this text ran on the root.
fn ran(engine: &mut Engine, text: &str) -> bool {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None })
    .expect("the root has a transcript");
  let holds = |one: &Fact| {
    (0..4)
      .any(|at| one.word(at).and_then(|word| word.as_str()).is_some_and(|word| word.contains(text)))
  };
  facts.iter().any(|one| one.kind() == "run" && holds(one))
}

/// The word of the done of the first reply, once the reply is done.
fn replied(engine: &mut Engine) -> String {
  let done = |engine: &mut Engine| {
    let facts = said(engine).into_iter();
    facts
      .into_iter()
      .find(|(kind, about, _)| kind == "done" && about == "reply1")
      .map(|(_, _, word)| word)
  };
  assert!(until(engine, |engine| done(engine).is_some()), "the reply is done");
  done(engine).unwrap_or_default()
}

#[test]
fn a_model_of_a_local_server_is_asked_with_no_key_and_a_cap_and_the_thought_in_its_text_is_a_block_of_reasoning()
 {
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105,
      "prompt_tokens_details": {"cached_tokens": 40}}});
  let thought = [
    chunk(json!({"role": "assistant", "content": "<think>hm</think>\n"}), Value::Null),
    chunk(json!({"content": "close(3)"}), json!("stop")),
    format!("data: {usage}\n\ndata: [DONE]\n\n"),
  ];
  let (mut engine, id, asked_of) = local("local", thought.concat());
  let turn = replied(&mut engine);
  let reasoning =
    "{'type': 'reasoning', 'id': None, 'content': [{'type': 'text', 'content': {'text': 'hm'}}]}";
  let expected = format!(
    "('assistant', 'close(3)', (100, 5, 40, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
  );
  assert_eq!(turn, expected);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let [listed, request] = asked_of.join().expect("the server read the requests");
  assert!(listed.starts_with("GET /v1/models "), "{listed}");
  assert!(request.starts_with("POST /v1/chat/completions "), "{request}");
  assert_eq!(
    (header(&listed, "authorization"), header(&request, "authorization")),
    (None, None),
    "a server that takes no key is sent none"
  );
  let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default())
    .expect("a JSON body");
  let asked =
    (&body["model"], &body["max_tokens"], body.get("cache_prompt"), body.get("reasoning_effort"));
  assert_eq!(
    asked,
    (&json!("qwen"), &json!(250), None, None),
    "a model writes a quarter of its window at most"
  );
}

#[test]
fn a_turn_whose_stream_stops_before_its_end_is_refused_and_no_part_of_its_word_runs() {
  let part = chunk(json!({"role": "assistant", "content": "x = 1"}), Value::Null);
  let (mut engine, _, asked_of) = local("cut", part);
  let word = replied(&mut engine);
  assert!(word.contains("the stream of local:qwen stopped before its end"), "{word}");
  assert!(!ran(&mut engine, "x = 1"), "no part of the word runs");
  asked_of.join().expect("the server read the requests");
}

#[test]
fn a_turn_that_the_most_it_writes_ended_is_refused_and_no_part_of_its_word_runs() {
  let part = chunk(json!({"role": "assistant", "content": "x = 1"}), json!("length"));
  let (mut engine, _, asked_of) = local("length", format!("{part}data: [DONE]\n\n"));
  let word = replied(&mut engine);
  assert!(
    word.contains("the turn of local:qwen reached the most it writes before its end: it wrote"),
    "{word}"
  );
  assert!(
    word.contains(r#"0 characters of thought and 5 of text, which end with "x = 1""#),
    "{word}"
  );
  assert!(!ran(&mut engine, "x = 1"), "no part of the word runs");
  asked_of.join().expect("the server read the requests");
}

/// A life whose root prompts local:qwen of a local server of the tests whose most thought in a turn is two parts, which
/// lists it with a window of 1000, streams these events for the first request of its one reply, and those for the
/// request that asks it again; the prompt, and the three requests the server read.
fn rethinking(
  first: String,
  again: String,
) -> (Engine, String, std::thread::JoinHandle<[String; 3]>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let listing =
    json!({"object": "list", "data": [{"id": "qwen", "object": "model", "meta": {"n_ctx": 1000}}]});
  let asked_of = thread::spawn(move || {
    [listing.to_string(), first, again].map(|one| exchanged(&listener, &one))
  });
  let address = format!("http://127.0.0.1:{port}");
  let env = |name: &str| match name {
    "FURB_LOCAL_URL" => Some(address.clone()),
    "FURB_LOCAL_THOUGHT" => Some("2".to_owned()),
    _ => None,
  };
  let catalog =
    Catalog::of(&super::catalog::parsed("{}", None), None, Server::named(env).map(Arc::new), env);
  let (models, actor) = catalog.roster(None, Some("local:qwen")).expect("the model is offered");
  let mut engine =
    lived(Provider::new(yard("rethought").display().to_string(), models).actor(actor));
  let id = asked(&mut engine);
  (engine, id, asked_of)
}

#[test]
fn a_model_whose_thought_passes_its_most_is_asked_again_with_that_thought_and_no_thought_and_its_one_fence_goes()
 {
  let thought =
    ["a", "b", "c", "d"].map(|one| chunk(json!({"reasoning_content": one}), Value::Null));
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105,
      "prompt_tokens_details": {"cached_tokens": 0}}});
  let again = [
    chunk(json!({"role": "assistant", "content": "```python\nclose(3)\n```"}), json!("stop")),
    format!("data: {usage}\n\ndata: [DONE]\n\n"),
  ];
  let (mut engine, id, asked_of) = rethinking(thought.concat(), again.concat());
  let turn = replied(&mut engine);
  let reasoning =
    "{'type': 'reasoning', 'id': None, 'content': [{'type': 'text', 'content': {'text': 'abc'}}]}";
  let expected = format!(
    "('assistant', 'close(3)', (100, 8, 0, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
  );
  assert_eq!(
    turn, expected,
    "the turn holds the thought, counts its parts, and loses its one fence"
  );
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let [_, first, second] = asked_of.join().expect("the server read the requests");
  let body = |request: &str| -> Value {
    serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body")
  };
  let (first, second) = (body(&first), body(&second));
  assert_eq!(
    first.get("chat_template_kwargs"),
    None,
    "the first request thinks as the server says"
  );
  let last =
    second["messages"].as_array().and_then(|messages| messages.last()).cloned().unwrap_or_default();
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
fn a_fence_in_a_turn_of_the_first_request_stays_for_the_gate() {
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105}});
  let events = [
    chunk(json!({"role": "assistant", "content": "```python\nclose(3)\n```"}), json!("stop")),
    format!("data: {usage}\n\ndata: [DONE]\n\n"),
  ];
  let (mut engine, _, asked_of) = local("fenced", events.concat());
  let turn = replied(&mut engine);
  assert!(turn.starts_with(r"('assistant', '```python\nclose(3)\n```'"), "{turn}");
  assert!(!ran(&mut engine, "close(3)"), "the gate refuses the fenced word");
  drop(engine);
  let _ = asked_of.join();
}

#[test]
fn one_fence_that_wraps_the_whole_text_goes_and_any_other_text_stays() {
  assert_eq!(unfenced("```python\nclose(3)\n```"), Some("close(3)"));
  assert_eq!(unfenced("\n```\nx = 1\ny = 2\n```\n"), Some("x = 1\ny = 2"));
  assert_eq!(unfenced("Here:\n```python\nclose(3)\n```"), None);
  assert_eq!(unfenced("```python\nclose(3)\n```\nDone."), None);
  assert_eq!(unfenced("```python\nx = 1\n```\n```python\nclose(x)\n```"), None);
  assert_eq!(unfenced("```js\nclose(3)\n```"), None);
  assert_eq!(unfenced("close(3)"), None);
}
