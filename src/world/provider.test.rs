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

use futures::{FutureExt, StreamExt};
use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse, Usage},
  message::{AssistantContent, Message, Reasoning, UserContent},
  streaming::{RawStreamingChoice, StreamFinal, StreamingCompletionResponse},
  test_utils::{MockCompletionModel, MockError, MockStreamEvent},
};
use serde_json::{Value, json};

use jiff::{Zoned, civil};

use super::{
  Backoff, Hosted, Model, Provider, Retries, Retry, Thinking, Writes, catalog::Catalog, images,
  limit, local::Server, passing, resets, thinking, unfenced,
};
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
    ("bash", world::bash(scratch, &[])),
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
    "[[['m', ['low', 'high'], 1000, {}], ['operator', [], 200000]], '{}', 'm/high']",
    Object::string(super::catalog::DOCTRINE).py_repr(),
    at.display().to_string().replace('\\', "\\\\")
  );
  assert_eq!(standing.py_repr(), expected);
}

#[test]
fn the_entry_of_each_model_holds_its_doctrine_after_its_window() {
  let qwen = Model::new("local:qwen3", 1000, MockCompletionModel::new([]));
  let (mut engine, _) = life("doctrine", vec![qwen, scripted(&MockCompletionModel::new([]))]);
  let standing = engine.standing().expect("the life stands");
  let taught = Object::string(super::catalog::DOCTRINE).py_repr();
  let expected = format!(
    "[[['local:qwen3', [], 1000, {taught}], ['m', ['low', 'high'], 1000, {taught}], ['operator', [], 200000]]"
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

/// A model that writes these parts of its text and then waits, and says when its call is dropped.
#[derive(Clone)]
struct Halting {
  parts: Vec<&'static str>,
  dropped: Arc<AtomicBool>,
}

impl Halting {
  fn writes(parts: &[&'static str]) -> Halting {
    Halting { parts: parts.to_vec(), dropped: Arc::default() }
  }
}

impl CompletionModel for Halting {
  async fn completion(&self, _: CompletionRequest) -> Result<CompletionResponse, CompletionError> {
    Err(CompletionError::ProviderError("the provider streams".into()))
  }

  async fn stream(
    &self,
    _: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let flag = Flag(Arc::clone(&self.dropped));
    let said = futures::stream::iter(
      self.parts.clone().into_iter().map(|part| Ok(RawStreamingChoice::Message(part.to_owned()))),
    );
    let held = futures::stream::once(async move {
      let _flag = flag;
      std::future::pending::<()>().await;
    })
    .filter_map(|()| std::future::ready(None::<Result<RawStreamingChoice, CompletionError>>));
    Ok(StreamingCompletionResponse::stream("halting", Box::pin(said.chain(held))))
  }
}

/// Whether the root holds a tell that holds each of these texts.
fn told_of(engine: &mut Engine, texts: &[&str]) -> bool {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None })
    .expect("the root has a transcript");
  facts.iter().any(|one| {
    let all = one.word(0).map(|word| word.py_repr()).unwrap_or_default();
    one.kind() == "tell" && texts.iter().all(|text| all.contains(text))
  })
}

/// The turn of a reply that ended at a word as far as this text, which counted nothing.
fn cut(text: &str) -> String {
  let text = text.replace('\n', "\\n");
  format!("('assistant', '{text}', (0, 0, 0, 0, 0.0), [{{'type': 'text', 'text': '{text}'}}])")
}

#[test]
fn a_reply_ends_at_the_first_statement_that_cannot_be_python_and_the_gate_refuses_its_word() {
  let model = Halting::writes(&["```python\n", "foo()\n", "bar()\n"]);
  let (mut engine, _) = life("stops", vec![Model::new("m", 1000, model.clone())]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("```python"));
  assert!(
    until(&mut engine, |_| model.dropped.load(Ordering::SeqCst)),
    "the call of the model ended before it wrote the rest"
  );
  assert!(
    until(&mut engine, |engine| told_of(engine, &["refused", "line 1"])),
    "the gate refused the word as far as the statement"
  );
  assert!(!ran(&mut engine, "foo"), "no word ran");
}

#[test]
fn a_reply_ends_after_a_fault_that_follows_a_quote_at_the_line_of_the_fault() {
  let parts = ["<s:note>\n", "Hello\n", "</s:note>\n", "Here is the code:\n", "close(1)\n"];
  let model = Halting::writes(&parts);
  let (mut engine, _) = life("quotes", vec![Model::new("m", 1000, model.clone())]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("<s:note>\nHello\n</s:note>\nHere is the code:"));
}

#[test]
fn a_word_whose_statements_stand_over_many_lines_is_not_ended_while_the_model_writes_it() {
  let parts = [
    "x = (\n",
    "  3,\n",
    ")\n",
    "t = \'\'\'\n",
    "text at the edge\n",
    "\'\'\'\n",
    "<s:note>\n",
    "Hello\n",
    "</s:note>\n",
    "if x:\n",
    "  y = 1\n",
    "else:\n",
    "  y = 2\n",
    "close(x[0] + len(t) + y + len(note))",
  ];
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let events: Vec<_> = parts
    .iter()
    .map(|part| MockStreamEvent::text(*part))
    .chain([MockStreamEvent::FinalResponse(StreamFinal::new("mock", usage))])
    .collect();
  let model = MockCompletionModel::from_stream_turns([events]);
  let (mut engine, _) = life("whole", vec![scripted(&model)]);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3 + 18 + 1 + 6));
}

#[test]
fn what_a_function_of_the_host_tells_is_no_word_that_ends_its_reply() {
  let host: Hosted = Arc::new(|_, told| {
    async move {
      told("```python\nthe work goes on\n", "");
      told("it goes on\n", "");
      Ok(Object::tuple([
        Object::string("assistant"),
        Object::string("close(2)"),
        Object::tuple([
          Object::int(0),
          Object::int(0),
          Object::int(0),
          Object::int(0),
          Object::float(0.0),
        ]),
        Object::list([]),
      ]))
    }
    .boxed()
  });
  let at = yard("hosted");
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let mut engine = lived(Provider::new(at.display().to_string(), vec![model]));
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(2));
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
fn a_claude_that_an_update_took_away_is_asked_again_after_its_backoff_until_it_comes_back() {
  use super::claude::{Claude, test::Yard};
  let fake = Yard::new("away");
  let claude = Claude::with_command(fake.prefix(), Vec::new(), Some(Duration::from_secs(10)));
  let (retries, told) = retried();
  let backoff = Backoff { most: 10, first: Duration::from_millis(100) };
  let provider = Provider::new(yard("away").display().to_string(), claude.models())
    .backoff(backoff)
    .retries(retries);
  let mut engine = lived(provider);
  let root = engine.root().to_owned();
  fake.away();
  let with = verbs::Prompt { message: Some("task".into()), on: on(&root), ..Default::default() };
  let id = engine.prompt(Object::string("str"), with).expect("a prompt is made").id().to_owned();
  let waited = until(&mut engine, |_| told.lock().expect("the retries told").len() >= 2);
  assert!(waited, "the call is made again while the program is away");
  fake.back();
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.py_repr(), "'reply 1'");
  let retries = told.lock().expect("the retries told").clone();
  assert!(retries.iter().all(|one| one.2.why == "the program is missing"), "{retries:?}");
  assert_eq!(retries[..2].iter().map(|one| one.2.attempt).collect::<Vec<_>>(), [2, 3]);
}

#[cfg(unix)]
#[test]
fn the_claude_command_line_is_told_the_most_thought_of_the_provider_unless_the_environment_gave_it()
{
  use super::claude::test::Yard;
  let fake = Yard::new("most");
  let claude = fake.claude(10_000);
  let inherited = Thinking { most: 4096, inherited: true };
  for (name, most) in [("most-told", Thinking::default()), ("most-inherited", inherited)] {
    let provider = Provider::new(yard(name).display().to_string(), claude.models());
    let mut engine = lived(provider.thinking(most).actor(Some("claude-cli:opus/high".into())));
    let root = engine.root().to_owned();
    let with = verbs::Prompt { message: Some("task".into()), on: on(&root), ..Default::default() };
    block_on(engine.prompt(Object::string("str"), with).expect("a prompt is made"))
      .expect("an answer");
  }
  let thinking: Vec<String> = fake
    .pids()
    .iter()
    .map(|pid| fs::read_to_string(fake.at.join(format!("thinking.{pid}"))).unwrap_or_default())
    .collect();
  let given = std::env::var("MAX_THINKING_TOKENS").unwrap_or_else(|_| "unset".to_owned());
  assert_eq!(thinking, ["32768".to_owned(), given], "a most that the environment gave goes untold");
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

/// A function of the host that fails with these reasons, one call each, and then answers `close(7)`, and how many
/// times it was called.
fn failing(reasons: &[&str]) -> (Hosted, Arc<AtomicUsize>) {
  let calls = Arc::new(AtomicUsize::new(0));
  let (counted, reasons) =
    (Arc::clone(&calls), reasons.iter().map(|one| one.to_string()).collect::<Vec<_>>());
  let host: Hosted = Arc::new(move |_, told| {
    let call = counted.fetch_add(1, Ordering::SeqCst);
    if let Some(why) = reasons.get(call).cloned() {
      told("clo", "");
      return async move { Err(why) }.boxed();
    }
    told("close(7)", "");
    let usage = Object::list([10, 2, 0, 0, 1].map(Object::int));
    let turn = [Object::string("assistant"), Object::string("close(7)"), usage, Object::none()];
    async move { Ok(Object::list(turn)) }.boxed()
  });
  (host, calls)
}

/// The retries that a test was told, each as its rung, its chain and the retry.
type Told = Arc<Mutex<Vec<(String, String, Retry)>>>;

/// Whom a test tells each retry, and what it was told.
fn retried() -> (Retries, Told) {
  let heard: Told = Arc::default();
  let kept = Arc::clone(&heard);
  let retries: Retries = Arc::new(move |rung, chain, retry| {
    kept.lock().expect("the retries told").push((rung.to_owned(), chain.to_owned(), retry.clone()));
  });
  (retries, heard)
}

/// The rung that the first reply of the root answers.
fn replying(engine: &mut Engine) -> String {
  let root = engine.root().to_owned();
  let facts =
    engine.transcript(verbs::Transcript { on: on(&root), of: None }).expect("a transcript");
  facts.iter().find(|one| one.kind() == "reply").map(|one| one.by().to_owned()).expect("a reply")
}

#[test]
fn a_call_that_fails_twice_for_a_cause_that_passes_is_made_again_after_its_backoff_and_the_host_is_told_each_retry()
 {
  let (host, calls) =
    failing(&["Connection reset by peer (os error 54)", "API Error: 529 Overloaded"]);
  let (retries, told) = retried();
  let at = yard("retried");
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let backoff = Backoff { most: 10, first: Duration::from_millis(5) };
  let provider =
    Provider::new(at.display().to_string(), vec![model]).backoff(backoff).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7));
  assert_eq!(calls.load(Ordering::SeqCst), 3);
  let (rung, root) = (replying(&mut engine), engine.root().to_owned());
  let retry = |attempt, millis, why: &str| {
    let wait = Duration::from_millis(millis);
    (rung.clone(), root.clone(), Retry { attempt, most: 10, wait, why: why.to_owned() })
  };
  let expected = vec![retry(2, 5, "connection reset"), retry(3, 10, "the provider is overloaded")];
  assert_eq!(*told.lock().expect("the retries told"), expected);
  let kinds: Vec<_> = said(&mut engine).into_iter().map(|(kind, about, _)| (kind, about)).collect();
  assert_eq!(
    kinds,
    [("started", "reply1"), ("done", "reply1")].map(|(k, a)| (k.to_owned(), a.to_owned()))
  );
}

#[test]
fn a_call_whose_attempts_are_spent_is_refused_with_its_last_failure() {
  let (host, calls) = failing(&["503 Service Unavailable"; 3]);
  let (retries, told) = retried();
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let backoff = Backoff { most: 3, first: Duration::from_millis(1) };
  let provider = Provider::new(yard("spent").display().to_string(), vec![model])
    .backoff(backoff)
    .retries(retries);
  let mut engine = lived(provider);
  asked(&mut engine);
  assert!(until(&mut engine, |engine| said(engine).len() >= 2), "{:?}", said(&mut engine));
  // The chain asks again after the refusal, and the function answers that later call.
  assert!(calls.load(Ordering::SeqCst) >= 3);
  let attempts: Vec<_> =
    told.lock().expect("the retries told").iter().map(|one| one.2.attempt).take(2).collect();
  assert_eq!(attempts, [2, 3]);
  let refused = "Refused(args=('m/high answered nothing: ProviderError: 503 Service Unavailable'))";
  assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), refused.into()));
}

#[test]
fn only_a_failure_of_the_network_or_of_the_side_of_the_provider_passes() {
  let provider = |why: &str| passing(&CompletionError::ProviderError(why.to_owned()));
  assert_eq!(
    provider("error sending request: connection reset").as_deref(),
    Some("connection reset")
  );
  assert_eq!(provider("the request timed out").as_deref(), Some("timed out"));
  assert_eq!(provider("429 Too Many Requests").as_deref(), Some("rate limited"));
  assert_eq!(provider("Invalid status code 502 Bad Gateway").as_deref(), Some("server error 502"));
  assert_eq!(provider("unavailable"), None);
  assert_eq!(provider("boom"), None);
  assert_eq!(provider("No model is m/x."), None);
  assert_eq!(
    provider("No program to run: claude did not start: No such file or directory (os error 2)")
      .as_deref(),
    Some("the program is missing")
  );
  assert_eq!(provider("Claude exited (1): Error: 'claude' was not found on PATH."), None);
  assert_eq!(passing(&CompletionError::ResponseError("the stream stopped, 503".into())), None);
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
  thinking_local(name, events, Thinking::default())
}

/// The life of [`local`], whose model thinks at most so many tokens in one turn.
fn thinking_local(
  name: &str,
  events: String,
  most: Thinking,
) -> (Engine, String, std::thread::JoinHandle<[String; 2]>) {
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
  assert_eq!(
    actor.as_deref(),
    Some("local:qwen/high"),
    "a model of a local server takes the effort that its actor names"
  );
  assert_eq!(models[0].priced(), Some([0.0; 4]), "a model of a local server costs nothing");
  let provider = Provider::new(yard(name).display().to_string(), models).actor(actor);
  let mut engine = lived(provider.thinking(most));
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
    (&json!("qwen"), &json!(250), None, Some(&json!("high"))),
    "a model writes a quarter of its window at most, and thinks at the effort of its actor"
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
  let env = |name: &str| (name == "FURB_LOCAL_URL").then(|| address.clone());
  let catalog =
    Catalog::of(&super::catalog::parsed("{}", None), None, Server::named(env).map(Arc::new), env);
  let (models, actor) = catalog.roster(None, Some("local:qwen")).expect("the model is offered");
  let provider = Provider::new(yard("rethought").display().to_string(), models).actor(actor);
  let mut engine = lived(provider.thinking(Thinking { most: 2, inherited: false }));
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
fn the_most_thought_of_a_turn_is_that_of_the_environment_else_that_of_the_config_else_32768() {
  let at = yard("thinking");
  assert_eq!(thinking(None, &at), Thinking { most: 32_768, inherited: false });
  fs::create_dir_all(at.join(".furb")).expect("the folder of the config");
  fs::write(at.join(".furb").join("config.json"), r#"{"max_thinking_tokens": 0}"#)
    .expect("the config");
  assert_eq!(thinking(None, &at), Thinking { most: 0, inherited: false }, "the config gives it");
  assert_eq!(
    thinking(Some("4096"), &at),
    Thinking { most: 4096, inherited: true },
    "the environment wins over the config"
  );
  assert_eq!(
    thinking(Some("much"), &at),
    Thinking { most: 0, inherited: true },
    "a value that is no count is as unsaid, and claude inherits it"
  );
}

#[test]
fn a_model_of_a_local_server_with_a_most_thought_of_zero_is_asked_with_its_thought_off_from_the_first_request()
 {
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105}});
  let events = [
    chunk(json!({"role": "assistant", "content": "close(3)"}), json!("stop")),
    format!("data: {usage}\n\ndata: [DONE]\n\n"),
  ];
  let none = Thinking { most: 0, inherited: false };
  let (mut engine, id, asked_of) = thinking_local("unthought", events.concat(), none);
  replied(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let [_, first] = asked_of.join().expect("the server read the requests");
  let first: Value =
    serde_json::from_str(first.split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body");
  assert_eq!(
    (&first["chat_template_kwargs"], first.get("reasoning_effort")),
    (&json!({"enable_thinking": false}), None),
    "the first request turns the thought off"
  );
}

#[test]
fn a_local_model_reads_its_earlier_turn_with_no_thought() {
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105}});
  let ended = format!("data: {usage}\n\ndata: [DONE]\n\n");
  let first = [
    chunk(json!({"role": "assistant", "reasoning_content": "a"}), Value::Null),
    chunk(json!({"content": "x = 1"}), json!("stop")),
    ended.clone(),
  ];
  let again = [chunk(json!({"role": "assistant", "content": "close(x)"}), json!("stop")), ended];
  let (mut engine, id, asked_of) = rethinking(first.concat(), again.concat());
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(1));
  let [_, _, second] = asked_of.join().expect("the server read the requests");
  let body: Value =
    serde_json::from_str(second.split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body");
  let earlier = body["messages"]
    .as_array()
    .and_then(|messages| messages.iter().find(|one| one["role"] == "assistant"))
    .cloned()
    .unwrap_or_default();
  assert_eq!(
    (&earlier["content"], earlier.get("reasoning_content")),
    (&json!([{"type": "text", "text": "x = 1"}]), None),
    "the earlier turn goes with its word and no thought"
  );
}

#[test]
fn a_turn_of_a_local_model_loses_the_fence_that_opens_it_with_no_closing_fence() {
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105}});
  let events = [
    chunk(json!({"role": "assistant", "content": "```python\nclose(3)\n"}), json!("stop")),
    format!("data: {usage}\n\ndata: [DONE]\n\n"),
  ];
  let (mut engine, id, asked_of) = local("fenced", events.concat());
  let turn = replied(&mut engine);
  assert!(
    turn.starts_with(
      "('assistant', 'close(3)', (100, 5, 0, 0, 0.0), [{'type': 'text', 'text': 'close(3)'}]"
    ),
    "{turn}"
  );
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  asked_of.join().expect("the server read the requests");
}

#[test]
fn a_turn_with_a_thought_and_no_word_is_asked_again_with_that_thought_and_no_thought() {
  let thought = ["a", "b"].map(|one| chunk(json!({"reasoning_content": one}), Value::Null));
  let first_usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen",
    "choices": [], "usage": {"prompt_tokens": 100, "completion_tokens": 2, "total_tokens": 102}});
  let first = [
    thought.concat(),
    chunk(json!({"content": ""}), json!("stop")),
    format!("data: {first_usage}\n\ndata: [DONE]\n\n"),
  ];
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105,
      "prompt_tokens_details": {"cached_tokens": 0}}});
  let again = [
    chunk(json!({"role": "assistant", "content": "close(3)"}), json!("stop")),
    format!("data: {usage}\n\ndata: [DONE]\n\n"),
  ];
  let (mut engine, id, asked_of) = rethinking(first.concat(), again.concat());
  let turn = replied(&mut engine);
  let reasoning =
    "{'type': 'reasoning', 'id': None, 'content': [{'type': 'text', 'content': {'text': 'ab'}}]}";
  let expected = format!(
    "('assistant', 'close(3)', (100, 7, 0, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
  );
  assert_eq!(turn, expected, "the turn holds the thought and the word, and counts what both wrote");
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let [_, _, second] = asked_of.join().expect("the server read the requests");
  let second: Value =
    serde_json::from_str(second.split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body");
  let last =
    second["messages"].as_array().and_then(|messages| messages.last()).cloned().unwrap_or_default();
  assert_eq!(
    (&last["role"], &last["reasoning_content"], &second["chat_template_kwargs"]),
    (&json!("assistant"), &json!("ab"), &json!({"enable_thinking": false})),
    "the model is asked again with its thought and no more thought"
  );
}

#[test]
fn the_fence_that_opens_a_text_goes_with_its_closing_fence_at_its_end_and_any_other_text_stays() {
  assert_eq!(unfenced("```python\nclose(3)\n```").as_deref(), Some("close(3)"));
  assert_eq!(unfenced("\n```\nx = 1\ny = 2\n```\n").as_deref(), Some("x = 1\ny = 2"));
  assert_eq!(
    unfenced("```python\nclose(3)\n").as_deref(),
    Some("close(3)"),
    "a fence with no close goes"
  );
  assert_eq!(
    unfenced("```python\nx = '```'").as_deref(),
    Some("x = '```'"),
    "backticks inside a line stay"
  );
  assert_eq!(unfenced("```python\nclose(3)\n```\nDone.").as_deref(), Some("close(3)\n```\nDone."));
  assert_eq!(
    unfenced("```python\nx = 1\n```\n```python\nclose(x)\n```").as_deref(),
    Some("x = 1\n```\n```python\nclose(x)")
  );
  assert_eq!(unfenced("Here:\n```python\nclose(3)\n```").as_deref(), None);
  assert_eq!(unfenced("```js\nclose(3)\n```").as_deref(), None);
  assert_eq!(unfenced("close(3)").as_deref(), None);
  assert_eq!(
    unfenced("<\nread(\"a\")\n</\n<\nclose(1)\n</").as_deref(),
    Some("read(\"a\")\nclose(1)"),
    "a bare tag that wraps each part of a word goes, each time"
  );
  assert_eq!(unfenced("<answer>\nclose(3)\n</answer>").as_deref(), Some("close(3)"));
  assert_eq!(unfenced("<s:x>\nhi\n</s:x>\nclose(x)"), None, "the mark of a quote is no tag");
  assert_eq!(
    unfenced("<answer>\nx = '<b>'\n<b>\nclose(x)").as_deref(),
    Some("x = '<b>'\n<b>\nclose(x)"),
    "a line of another tag stays"
  );
}

#[test]
fn a_call_that_the_usage_limit_refuses_waits_for_its_reset_and_the_chain_is_told_until_when() {
  let soon = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .expect("a time after the epoch")
    .as_secs()
    + 1;
  let (host, calls) = failing(&[&format!("Claude AI usage limit reached|{soon}")]);
  let (retries, told) = retried();
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let provider = Provider::new(yard("limit").display().to_string(), vec![model]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7));
  assert_eq!(calls.load(Ordering::SeqCst), 2);
  let heard = told.lock().expect("the retries told").clone();
  assert_eq!(heard.len(), 1);
  assert_eq!((heard[0].2.attempt, heard[0].2.most), (2, 10));
  assert!(heard[0].2.why.starts_with("usage limit until "), "{}", heard[0].2.why);
  assert!(heard[0].2.wait <= Duration::from_secs(3), "{:?}", heard[0].2.wait);
  let root = engine.root().to_owned();
  let facts = said(&mut engine);
  let kinds: Vec<_> =
    facts.iter().map(|(kind, about, _)| (kind.as_str(), about.as_str())).collect();
  assert_eq!(kinds, [("started", "reply1"), ("tell", root.as_str()), ("done", "reply1")]);
  let note = &facts[1].2;
  assert!(note.contains(&format!("#{root} waits on m/high: usage limit until ")), "{note}");
}

/// A moment in Paris.
fn paris(month: i8, day: i8, hour: i8, minute: i8) -> Zoned {
  civil::date(2026, month, day)
    .at(hour, minute, 0, 0)
    .in_tz("Europe/Paris")
    .expect("a moment in Paris")
}

#[test]
fn a_usage_limit_ends_at_the_next_time_its_message_names_in_its_zone() {
  let said = "You've hit your session limit \u{b7} resets 5:10pm (Europe/Paris)";
  assert_eq!(resets(said, &paris(10, 23, 16, 19)), Some(paris(10, 23, 17, 10)));
  assert_eq!(resets(said, &paris(10, 23, 17, 11)), Some(paris(10, 24, 17, 10)));
  let noon = "You've hit your limit \u{b7} resets 12am (Europe/Paris)";
  assert_eq!(resets(noon, &paris(10, 23, 16, 19)), Some(paris(10, 24, 0, 0)));
}

#[test]
fn a_usage_limit_ends_at_the_day_its_message_names() {
  let said = "You've hit your weekly limit \u{b7} resets Oct 28, 3pm (Europe/Paris)";
  assert_eq!(resets(said, &paris(10, 23, 16, 19)), Some(paris(10, 28, 15, 0)));
  let past = "You've hit your weekly limit \u{b7} resets Oct 2 at 3pm (Europe/Paris)";
  let next = civil::date(2027, 10, 2).at(15, 0, 0, 0).in_tz("Europe/Paris").expect("a moment");
  assert_eq!(resets(past, &paris(10, 23, 16, 19)), Some(next));
}

#[test]
fn a_usage_limit_ends_at_the_second_its_older_message_names() {
  let said = "Claude AI usage limit reached|1792768200";
  let at = resets(said, &paris(10, 23, 16, 19)).expect("a moment");
  assert_eq!(at.timestamp().as_second(), 1_792_768_200);
}

#[test]
fn a_usage_limit_whose_message_names_no_moment_has_none() {
  let now = paris(10, 23, 16, 19);
  assert_eq!(resets("You've hit your limit \u{b7} resets soon", &now), None);
  assert_eq!(resets("You've hit your limit \u{b7} resets 5pm (Mars/Base)", &now), None);
  assert_eq!(resets("You've hit your limit \u{b7} resets Mon 5pm", &now), None);
}

#[test]
fn only_a_usage_limit_of_the_claude_command_line_is_a_limit_that_waits_for_its_reset() {
  let now = paris(10, 23, 16, 19);
  let failed = |why: &str| CompletionError::ProviderError(why.to_owned());
  let said = failed("You've hit your session limit \u{b7} resets 5:10pm (Europe/Paris)");
  assert_eq!(limit(&said, &now), Some(Some(paris(10, 23, 17, 10))));
  assert_eq!(limit(&failed("You've hit your limit \u{b7} resets soon"), &now), Some(None));
  assert_eq!(limit(&failed("429 rate limit exceeded"), &now), None);
  assert_eq!(limit(&failed("boom"), &now), None);
}

#[test]
fn a_limit_that_stands_after_its_reset_is_asked_again_soon_and_the_chain_is_told_once() {
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .expect("a time after the epoch")
    .as_secs();
  let past = format!("Claude AI usage limit reached|{}", now - 3600);
  let (host, calls) = failing(&[&past, &past]);
  let (retries, told) = retried();
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let provider =
    Provider::new(yard("limit-twice").display().to_string(), vec![model]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7));
  assert_eq!(calls.load(Ordering::SeqCst), 3);
  assert_eq!(told.lock().expect("the retries told").len(), 2);
  let kinds: Vec<_> = said(&mut engine).into_iter().map(|(kind, ..)| kind).collect();
  assert_eq!(kinds, ["started", "tell", "done"]);
}

#[cfg(unix)]
#[test]
fn a_claude_command_line_at_its_usage_limit_is_asked_again_at_the_reset_and_answers_with_no_pause()
{
  use super::claude::test::Yard;
  let yard = Yard::new("limit");
  let claude = yard.claude(10_000);
  let (mut engine, _) = life("claude-limit", claude.models());
  let root = engine.root().to_owned();
  let with =
    verbs::Prompt { message: Some("LIMIT task".into()), on: on(&root), ..Default::default() };
  let act = engine.prompt(Object::string("str"), with).expect("a prompt is made");
  assert_eq!(block_on(act).expect("the prompt is answered").py_repr(), "'reply 1'");
  let kinds: Vec<_> = said(&mut engine).into_iter().map(|(kind, ..)| kind).collect();
  assert_eq!(kinds, ["started", "tell", "done"]);
  assert_eq!(yard.heard().len(), 2, "the reply asked the command line again after the reset");
}
