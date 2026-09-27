//! The provider of models in a life of the real engine: models of rig that answer from a script, a model that never
//! answers, and the claude command line of the tests.

use std::{
  fs,
  future::Future,
  path::PathBuf,
  pin::pin,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  task::{Context, Poll, Wake, Waker},
  thread,
  time::{Duration, Instant},
};

use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse, Usage},
  message::{AssistantContent, Message, Reasoning},
  streaming::StreamingCompletionResponse,
  test_utils::{MockCompletionModel, MockTurn},
};
use serde_json::json;

use super::Model;
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

/// A life of the real engine on the provider of these models and the ears of the World, in a yard of its own.
fn life(yard: &str, models: Vec<Model>) -> (Engine, PathBuf) {
  let at = std::env::temp_dir().join(format!("furb-provider-{yard}"));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).expect("a yard of the test");
  let directory = at.display().to_string();
  let ears: [(&str, Box<dyn Ear>); 4] = [
    ("provider", world::provider(directory, models, None)),
    ("files", world::files()),
    ("bash", world::bash()),
    ("time", world::time()),
  ];
  (Engine::boot(Vec::<Object>::new(), ears).expect("the life opens"), at)
}

/// The model of the tests, which answers from its script, with two efforts.
fn scripted(model: &MockCompletionModel) -> Model {
  Model::new("m", 1000, model.clone())
    .effort("low", json!({"thinking": 1024}))
    .effort("high", json!({"thinking": 8192}))
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
  // Python shows each backslash of a path of Windows twice.
  let expected = format!(
    "[[['m', ['low', 'high'], 1000], ['operator', [], 200000]], '{}', 'm/low']",
    at.display().to_string().replace('\\', "\\\\")
  );
  assert_eq!(standing.py_repr(), expected);
}

#[test]
fn a_reply_comes_to_the_turn_of_the_model_with_its_usage_and_its_blocks() {
  let mut usage = Usage::new();
  (usage.input_tokens, usage.output_tokens, usage.total_tokens) = (10, 5, 38);
  (usage.cached_input_tokens, usage.cache_creation_input_tokens) = (20, 3);
  let turn = MockTurn::text("close(3)").with_usage(usage).with_raw(json!({"cost": 0.25}));
  let model = MockCompletionModel::new([turn]);
  let (mut engine, _) = life("turn", vec![scripted(&model)]);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let expected =
    "('assistant', 'close(3)', (33, 5, 20, 3, 0.25), [{'type': 'text', 'text': 'close(3)'}])";
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
  let thought =
    AssistantContent::Reasoning(Reasoning::new_with_signature("hmm", Some("sig".into())));
  let first = [thought, AssistantContent::text("x = 1")];
  let turns = [MockTurn::from_contents(first.clone()), MockTurn::text("close(x)")];
  let model = MockCompletionModel::new(turns);
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
  let model = MockCompletionModel::new([MockTurn::error("boom"), MockTurn::error("boom again")]);
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
    let _flag = Flag(Arc::clone(&self.dropped));
    self.asked.store(true, Ordering::SeqCst);
    std::future::pending().await
  }

  async fn stream(
    &self,
    _: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    Err(CompletionError::ProviderError("no stream".into()))
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
  assert_eq!((after("--model"), after("--effort")), (Some("fable"), Some("low")));
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
