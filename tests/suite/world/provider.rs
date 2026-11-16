//! The provider of models in a life of the real engine, through what the crate gives a host:
//! models of rig that answer from a script or fail, a model that never answers, and the claude
//! command line of the tests. The laws that no door reaches stand beside the module, in
//! `src/world/provider.test.rs`, each family with its reason.

#![cfg_attr(
  unix,
  expect(
    clippy::format_collect,
    reason = "debt: a format! for each item, collected into a String"
  )
)]
#![expect(
  clippy::let_underscore_must_use,
  reason = "debt: a let _ drops a value that must be used"
)]
#![expect(clippy::let_underscore_untyped, reason = "debt: a let _ hides the type of what it drops")]
#![expect(
  clippy::unnecessary_wraps,
  reason = "debt: a Result or an Option that is always Ok or Some"
)]

use std::{
  collections::HashMap,
  fs,
  path::{MAIN_SEPARATOR, Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use futures::{
  FutureExt, StreamExt,
  channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
};
use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse, Usage},
  message::{AssistantContent, Message, Reasoning, UserContent},
  streaming::{RawStreamingChoice, StreamFinal, StreamingCompletionResponse, UnknownPayload},
  test_utils::{MockCompletionModel, MockError, MockStreamEvent},
};
use serde_json::{Value, json};

use jiff::{SignedDuration, Zoned};

use furb::skill::Places;
use furb::world::{
  Hosted, Members, Model, Provider, Retries, Retry, Searches, Writes, catalog, images,
};
use furb::{Act, Ear, Engine, Fact, Object, Text, fact, life::Opening, verbs, world};

#[cfg(unix)]
use crate::{claude, drive::on_reactor};
use crate::{
  drive::{block_on, park, waker},
  opened,
  yard::Yard,
};
#[cfg(unix)]
use furb::value::entry;
#[cfg(unix)]
use furb::{SYSTEM, world::Thinking};
use futures::stream;
use rig_core::http_client;
use rig_core::message;
use std::env;
use std::future as std_future;
use std::process;
use std::sync::mpsc;
use std::time;
use tokio::time as tokio_time;

/// A yard of one test, empty.
pub(super) fn yard(name: &str) -> Yard {
  Yard::new(format!("furb-provider-{name}-{}", process::id()))
}

/// A life of the real engine on a provider in a yard of its own, and the ears of the World.
fn lived(provider: Provider) -> Engine {
  seeing(provider, None)
}

/// A life of the real engine on a provider in a yard of its own, and the ears of the World, whose
/// files keep each image that a read gives in a gallery.
fn seeing(provider: Provider, gallery: Option<PathBuf>) -> Engine {
  let scratch = world::Scratch::own();
  // The config directory of the user is a place that no one writes, never the one of the machine.
  let config = env::temp_dir().join(format!("furb-provider-user-{}", process::id()));
  let places = Places::under(config);
  let ears: [(&str, Box<dyn Ear>); 4] = [
    ("provider", provider.places(places.clone()).ear()),
    ("files", world::files(scratch.clone(), places.clone(), gallery)),
    ("bash", world::bash(scratch, places.home)),
    ("time", world::time()),
  ];
  Engine::boot(Vec::<Object>::new(), ears).expect("the life opens")
}

/// A life of the real engine on the provider of these models and the ears of the World, in a yard
/// of its own.
fn life(name: &str, models: Vec<Model>) -> (Engine, Yard) {
  let at = yard(name);
  (lived(Provider::new(at.display().to_string(), models)), at)
}

/// The model of the tests, which answers from its script, with two efforts.
fn scripted(model: &(impl CompletionModel + Clone + 'static)) -> Model {
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

pub(super) fn on(root: &str) -> Option<String> {
  Some(root.to_owned())
}

/// A prompt on the root that wants an int.
pub(super) fn asked(engine: &mut Engine) -> String {
  let root = engine.root().to_owned();
  let with =
    verbs::Prompt { message: Some("count".to_owned()), on: on(&root), ..Default::default() };
  engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned()
}

/// The facts of the root that the provider said about replies and chains: each kind, what it is
/// about, and the representation of its first word. A reply and a rung are named by their order
/// among those facts, as `reply1` for the first reply that the provider took, since the setup of a
/// chain crafts replies and rungs of its own before them.
pub(super) fn said(engine: &mut Engine) -> Vec<(String, String, String)> {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
    .expect("the root has a transcript");
  let mut names: HashMap<String, String> = HashMap::new();
  let mut counts: HashMap<&str, usize> = HashMap::new();
  facts
    .iter()
    .filter(|one| one.by() == "provider" && !one.about().starts_with("stand"))
    .map(|one| {
      let word = one.word(0).map(|one| one.py_repr()).unwrap_or_default();
      let about = one.about().to_owned();
      let kind = ["reply", "rung"].into_iter().find(|kind| fact::named(&about, kind));
      let about = match kind {
        Some(kind) => names
          .entry(about)
          .or_insert_with(|| {
            let count = counts.entry(kind).or_default();
            *count += 1;
            format!("{kind}{count}")
          })
          .clone(),
        None => about,
      };
      (one.kind().to_owned(), about, word)
    })
    .collect()
}

/// The id of the first reply that the provider took, and the rung that asked it.
pub(super) fn taken(engine: &mut Engine) -> (String, String) {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
    .expect("the root has a transcript");
  let id = facts
    .iter()
    .find(|one| {
      one.by() == "provider" && one.kind() == "started" && fact::named(one.about(), "reply")
    })
    .map(|one| one.about().to_owned())
    .expect("the provider took a reply");
  let rung = facts
    .iter()
    .find(|one| one.kind() == "reply" && one.about() == id)
    .map(|one| one.by().to_owned());
  (id, rung.expect("a rung asked the reply"))
}

/// The turns of a chain after its setup, which ends at the turn `stand()` that the setup crafts.
fn chat(history: &[Message]) -> &[Message] {
  let stands = |one: &Message| match one {
    Message::Assistant { content, .. } => content.iter().any(
      |block| matches!(block, AssistantContent::Text(text) if text.text.starts_with("stand()")),
    ),
    Message::User { .. } | Message::System { .. } => false,
  };
  &history[history.iter().position(stands).map_or(0, |at| at + 1)..]
}

/// The engine driven until a condition of it holds, or ten seconds are up.
pub(super) fn until(engine: &mut Engine, holds: impl Fn(&mut Engine) -> bool) -> bool {
  within(engine, Duration::from_secs(10), holds)
}

/// The engine driven until a condition of it holds, or a time is up.
fn within(engine: &mut Engine, time: Duration, holds: impl Fn(&mut Engine) -> bool) -> bool {
  let waker = waker();
  let end = Instant::now() + time;
  while Instant::now() < end {
    engine.pump(&waker).expect("the engine is driven");
    if holds(engine) {
      return true;
    }
    park(Duration::from_millis(20));
  }
  false
}

#[test]
fn the_provider_answers_a_stand_with_its_roster_its_directory_its_default_actor_and_its_doctrine() {
  let (mut engine, at) = life("stand", vec![scripted(&MockCompletionModel::new([]))]);
  let standing = engine.standing().expect("the life stands");
  let doctrine = catalog::doctrine(&at).expect("a doctrine stands");
  // The default actor is the first model, at the level of an actor that names none. A path under
  // the home shows from `~`, and Python shows each backslash of a path of Windows twice.
  let shown = |one: &Path| {
    let home = env::home_dir().filter(|home| home.parent().is_some());
    let rest = home.as_deref().and_then(|home| one.strip_prefix(home).ok());
    let tilded = rest.map_or_else(
      || one.display().to_string(),
      |rest| format!("~{MAIN_SEPARATOR}{}", rest.display()),
    );
    tilded.replace('\\', "\\\\")
  };
  let expected = format!(
    "[[['m', ['low', 'high'], 1000, 'dear'], ['operator', [], 200000, 'cheap']], '{}', 'm/high', '{}']",
    shown(&at),
    shown(&doctrine)
  );
  assert_eq!(standing.py_repr(), expected);
}

#[test]
fn a_standing_that_gives_skills_ends_with_the_absolute_home_of_the_user() {
  let at = yard("home");
  let memory = ("memory".to_owned(), "memory.py".to_owned(), String::new(), false);
  let model = scripted(&MockCompletionModel::new([]));
  let provider = Provider::new(at.display().to_string(), vec![model]).advertises(vec![memory]);
  let mut engine = lived(provider);
  let standing = engine.standing().expect("the life stands").py_repr();
  // A path under the home shows as ~, so the standing is where a chain learns the folders above
  // the home. Python shows each backslash of a path of Windows twice.
  let home =
    env::home_dir().expect("the user has a home").display().to_string().replace('\\', "\\\\");
  assert!(standing.ends_with(&format!(", '{home}']")), "{standing}");
}

#[test]
fn the_operator_entry_lists_its_members_and_a_prompt_asks_one_of_them_as_operator_and_its_login() {
  // A word reads the window that the roster offers to `operator/ana`, as a prompt with that `to`
  // finds its actor, and -1 when the roster offers none.
  let model = streaming(&[("close(offered(standing()[0], 'operator/ana') or -1)", Usage::new())]);
  let at = yard("members");
  let provider = Provider::new(at.display().to_string(), vec![scripted(&model)])
    .members(Members::new(vec!["ana".to_owned(), "bo".to_owned()]));
  let mut engine = lived(provider);
  let standing = engine.standing().expect("the life stands").py_repr();
  assert!(standing.contains("['operator', ['ana', 'bo'], 200000, 'cheap']"), "{standing}");
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(200_000), "operator/ana names the operator entry");
}

#[test]
fn the_entry_of_each_model_ends_at_its_window_then_its_class() {
  let qwen = Model::new("local:qwen3", 1000, MockCompletionModel::new([]));
  let (mut engine, _) = life("doctrine", vec![qwen, scripted(&MockCompletionModel::new([]))]);
  let standing = engine.standing().expect("the life stands");
  let expected = "[[['local:qwen3', [], 1000, 'dear'], ['m', ['low', 'high'], 1000, 'dear'], ['operator', [], 200000, 'cheap']]";
  assert!(standing.py_repr().starts_with(expected), "{}", standing.py_repr());
}

#[test]
fn an_assistant_turn_goes_again_as_the_blocks_its_provider_gave() {
  let thought = Reasoning::new_with_signature("hmm", Some("sig".into()));
  // The stream says the id of the thought, which the turn keeps and the provider reads again.
  let thought = AssistantContent::Reasoning(Reasoning { id: Some("r".into()), ..thought });
  let first = [thought, AssistantContent::text("x = 1\ndebug(t'{x}')")];
  let reasoning =
    message::ReasoningContent::Text { text: "hmm".into(), signature: Some("sig".into()) };
  let turns = [
    vec![
      MockStreamEvent::Reasoning { id: "r".into(), content: reasoning },
      MockStreamEvent::text("x = 1\ndebug(t'{x}')"),
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
  let assistant = chat(&requests[1].chat_history).iter().find_map(|one| match one {
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
  // reply2 is the reply of the stand that the root plays on itself, which its crafted rung answers.
  let expected = [
    ("started", "reply1", String::new()),
    ("done", "reply1", refused("boom")),
    ("started", "reply2", String::new()),
    (
      "tell",
      root.as_str(),
      format!(
        "[\"#{root} paused by provider\\n{root}_value = 'm/high answered nothing: ProviderError: boom again'\"]"
      ),
    ),
    ("pause", root.as_str(), "'m/high answered nothing: ProviderError: boom again'".into()),
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
      tokio_time::sleep(Duration::from_millis(1)).await;
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
    // The turn between the two refusals makes an act, so it is no barren step of its thread.
    turn("x = 1\nstand()"),
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
  fn completion(
    &self,
    _: CompletionRequest,
  ) -> impl Future<Output = Result<CompletionResponse, CompletionError>> {
    std_future::ready(Err(CompletionError::ProviderError("the provider streams".into())))
  }

  async fn stream(
    &self,
    _: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let _flag = Flag(Arc::clone(&self.dropped));
    self.asked.store(true, Ordering::SeqCst);
    std_future::pending().await
  }
}

#[test]
fn a_cancel_over_a_reply_ends_the_call_of_its_model_and_the_ear_says_nothing_more_of_it() {
  let model = Silent::default();
  let (mut engine, _) = life("cancel", vec![Model::new("m", 1000, model.clone())]);
  let id = asked(&mut engine);
  assert!(until(&mut engine, |_| model.asked.load(Ordering::SeqCst)), "the model is asked");
  engine.cancel(&id, verbs::Cancel::default()).expect("the prompt is cancelled");
  let (reply, _) = taken(&mut engine);
  let no = block_on(Act::<Object>::of(&mut engine, &reply)).expect_err("the reply ends");
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
  breaks: Option<fn() -> CompletionError>,
  bill: Option<Value>,
}

impl Halting {
  fn writes(parts: &[&'static str]) -> Self {
    Self { parts: parts.to_vec(), dropped: Arc::default(), breaks: None, bill: None }
  }

  /// A model that tells this bill of its request before it writes these parts of its text, as the
  /// claude command line tells it: a part of its stream that holds the bill under `bill`.
  fn bills(bill: Value, parts: &[&'static str]) -> Self {
    Self { bill: Some(bill), ..Self::writes(parts) }
  }

  /// A model that writes these parts of its text, and whose stream then fails, in place of waiting.
  fn breaks(parts: &[&'static str]) -> Self {
    Self {
      breaks: Some(|| CompletionError::ProviderError("the stream broke".into())),
      ..Self::writes(parts)
    }
  }
}

/// A model whose first call is this halting one, and whose later calls this mock answers.
#[derive(Clone)]
struct First {
  halting: Halting,
  then: MockCompletionModel,
  called: Arc<AtomicBool>,
}

impl CompletionModel for First {
  fn completion(
    &self,
    request: CompletionRequest,
  ) -> impl Future<Output = Result<CompletionResponse, CompletionError>> {
    self.then.completion(request)
  }

  async fn stream(
    &self,
    request: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    if self.called.swap(true, Ordering::SeqCst) {
      self.then.stream(request).await
    } else {
      self.halting.stream(request).await
    }
  }
}

/// The turns that the provider said each reply was done with, in order.
fn turns_done(engine: &mut Engine) -> Vec<String> {
  let dones = said(engine).into_iter().filter(|(kind, about, word)| {
    kind == "done" && about.starts_with("reply") && word.starts_with("('assistant'")
  });
  dones.map(|(_, _, word)| word).collect()
}

impl CompletionModel for Halting {
  fn completion(
    &self,
    _: CompletionRequest,
  ) -> impl Future<Output = Result<CompletionResponse, CompletionError>> {
    std_future::ready(Err(CompletionError::ProviderError("the provider streams".into())))
  }

  fn stream(
    &self,
    _: CompletionRequest,
  ) -> impl Future<Output = Result<StreamingCompletionResponse, CompletionError>> {
    let flag = Flag(Arc::clone(&self.dropped));
    let billed = self
      .bill
      .clone()
      .map(|bill| Ok(RawStreamingChoice::Unknown(UnknownPayload::new(json!({ "bill": bill })))));
    let wrote =
      self.parts.clone().into_iter().map(|part| Ok(RawStreamingChoice::Message(part.to_owned())));
    let said = stream::iter(billed.into_iter().chain(wrote));
    if let Some(broke) = self.breaks {
      let broke = stream::iter([Err(broke())]);
      return std_future::ready(Ok(StreamingCompletionResponse::stream(
        "halting",
        Box::pin(said.chain(broke)),
      )));
    }
    let held = stream::once(async move {
      let _flag = flag;
      std_future::pending::<()>().await;
    })
    .filter_map(|()| std_future::ready(None::<Result<RawStreamingChoice, CompletionError>>));
    std_future::ready(Ok(StreamingCompletionResponse::stream(
      "halting",
      Box::pin(said.chain(held)),
    )))
  }
}

/// Whether the root holds a tell that holds each of these texts.
fn told_of(engine: &mut Engine, texts: &[&str]) -> bool {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
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
fn a_reply_ends_at_the_first_statement_that_cannot_be_python_and_the_run_of_its_word_raises() {
  let model = Halting::writes(&["Here is the code:\n", "foo()\n", "bar()\n"]);
  let (mut engine, _) = life("stops", vec![Model::new("m", 1000, model.clone())]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("Here is the code:"));
  assert!(
    until(&mut engine, |_| model.dropped.load(Ordering::SeqCst)),
    "the call of the model ended before it wrote the rest"
  );
  assert!(
    until(&mut engine, |engine| told_of(engine, &["raised", "SyntaxError"])),
    "the run of the word as far as the statement raised"
  );
  assert!(!ran(&mut engine, "foo"), "no later statement ran");
}

#[test]
fn a_reply_that_ends_before_its_stream_pays_what_its_request_read_as_far_as_its_stream_told() {
  let bill = json!({"input": 3000, "output": 7, "read": 0, "wrote": 2000, "dollars": 0.25});
  let model = Halting::bills(bill, &["Here is the code:\n", "foo()\n"]);
  let (mut engine, _) = life("billed", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  let turn = "('assistant', 'Here is the code:', (3000, 7, 0, 2000, 0.25), [{'type': 'text', 'text': 'Here is the code:'}])";
  assert!(until(&mut engine, |engine| !turns_done(engine).is_empty()), "the reply is done");
  assert_eq!(turns_done(&mut engine), [turn]);
}

#[test]
fn the_next_turn_of_a_chain_pays_what_a_reply_that_a_cancel_ended_read() {
  let bill = json!({"input": 3000, "output": 7, "read": 0, "wrote": 2000, "dollars": 0.25});
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let halting = Halting::bills(bill, &["x = 1\n", "y = (\n"]);
  let model = First { halting, then: streaming(&[("close(3)", usage)]), called: Arc::default() };
  let (mut engine, _) = life("gone", vec![Model::new("m", 1000, model)]);
  let id = asked(&mut engine);
  assert!(until(&mut engine, |engine| wrote(engine, "x = 1\n")), "a part of the word ran");
  engine.cancel(&id, verbs::Cancel::default()).expect("the prompt is cancelled");
  asked(&mut engine);
  let turn =
    "('assistant', 'close(3)', (10, 5, 0, 0, 0.25), [{'type': 'text', 'text': 'close(3)'}])";
  assert!(until(&mut engine, |engine| !turns_done(engine).is_empty()), "the next reply is done");
  assert_eq!(turns_done(&mut engine), [turn]);
}

#[test]
fn a_reply_whose_call_failed_after_a_part_of_its_word_ran_pays_the_request_of_that_call() {
  let bill = json!({"input": 3000, "output": 7, "read": 0, "wrote": 2000, "dollars": 0.25});
  let model = Halting { bill: Some(bill), ..Halting::breaks(&["x = 3\n", "y = x"]) };
  let (mut engine, _) = life("failed", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  let turn =
    "('assistant', 'x = 3', (3000, 7, 0, 2000, 0.25), [{'type': 'text', 'text': 'x = 3'}])";
  assert!(until(&mut engine, |engine| !turns_done(engine).is_empty()), "the reply is done");
  assert_eq!(turns_done(&mut engine), [turn]);
}

#[test]
fn a_reply_ends_after_a_fault_that_follows_a_quote_at_the_line_of_the_fault() {
  // The quote is whole python, so it runs, and the reply ends with it, which is the word that ran.
  let parts = ["<s:note>\n", "Hello\n", "</s:note>\n", "Here is the code:\n", "close(1)\n"];
  let model = Halting::writes(&parts);
  let (mut engine, _) = life("quotes", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("<s:note>\nHello\n</s:note>"));
  assert!(
    until(&mut engine, |engine| told_of(engine, &["is the word that ran", "line 4"])),
    "the chain is told the fault at its line"
  );
}

#[test]
fn a_reply_ends_after_the_first_statement_whose_fault_no_later_statement_mends() {
  // The fault stands first, so no part of the word ran before it.
  let parts = ["y = (\n", "  nothing_binds_this)\n", "x = 1\n", "close(x)\n"];
  let model = Halting::writes(&parts);
  let (mut engine, _) = life("settled", vec![Model::new("m", 1000, model.clone())]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("y = (\n  nothing_binds_this)"));
  assert!(
    until(&mut engine, |_| model.dropped.load(Ordering::SeqCst)),
    "the call of the model ended before it wrote the rest"
  );
  assert!(
    until(&mut engine, |engine| told_of(engine, &["raised", "nothing_binds_this"])),
    "the run of the word as far as the statement raised"
  );
}

#[test]
fn a_name_that_a_later_statement_binds_does_not_end_a_reply_in_a_function() {
  let parts = ["def f():\n", "  return later()\n", "def later():\n", "  return 4\n", "close(f())"];
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let events: Vec<_> = parts
    .iter()
    .map(|part| MockStreamEvent::text(*part))
    .chain([MockStreamEvent::FinalResponse(StreamFinal::new("mock", usage))])
    .collect();
  let model = MockCompletionModel::from_stream_turns([events]);
  let (mut engine, _) = life("later", vec![scripted(&model)]);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(4));
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
    "z = 1 + \\\n",
    "  2\n",
    "try:\n",
    "  w = 3\n",
    "except ValueError:\n",
    "  w = 0\n",
    "def f(a,\n",
    "      b):\n",
    "  return a + b\n",
    "close(x[0] + len(t) + y + len(note) + z + w + f(1, 2))",
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
  assert_eq!(got.as_ref().as_int(), Some(3 + 18 + 1 + 6 + 3 + 3 + 3));
}

#[test]
fn a_close_mark_inside_a_line_of_a_quote_does_not_end_the_reply_at_its_open_mark() {
  let parts =
    ["<s:old>\n", "x = \"</s:old>\" in text\n", "more text\n", "</s:old>\n", "close(len(old))"];
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let events: Vec<_> = parts
    .iter()
    .map(|part| MockStreamEvent::text(*part))
    .chain([MockStreamEvent::FinalResponse(StreamFinal::new("mock", usage))])
    .collect();
  let model = MockCompletionModel::from_stream_turns([events]);
  let (mut engine, _) = life("inline", vec![scripted(&model)]);
  let id = asked(&mut engine);
  assert!(
    until(&mut engine, |engine| ran(engine, "len(old)")),
    "the whole word ran, past the close mark inside a line of the quote"
  );
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(33));
}

#[test]
fn a_triple_quote_whose_paragraph_starts_at_the_left_edge_runs_once_its_word_is_whole() {
  // A chunk ends at the blank line inside the triple quote, and the paragraph after it starts at
  // the left edge: the string is not whole until its close.
  let first = "close(\"\"\"My take: a stronger control model, which costs less.\n\n";
  let parts = [first, "What I like: the paragraph at the left edge.\n", "end\"\"\")"];
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let events: Vec<_> = parts
    .iter()
    .map(|part| MockStreamEvent::text(*part))
    .chain([MockStreamEvent::FinalResponse(StreamFinal::new("mock", usage))])
    .collect();
  let model = MockCompletionModel::from_stream_turns([events]);
  let at = yard("triple");
  let mut engine = lived(Provider::new(at.display().to_string(), vec![scripted(&model)]));
  let root = engine.root().to_owned();
  let with = verbs::Prompt { message: Some("say".to_owned()), on: on(&root), ..Default::default() };
  let id = engine.prompt(Object::string("str"), with).expect("a prompt is made").id().to_owned();
  assert!(
    until(&mut engine, |engine| ran(engine, "What I like")),
    "the whole word ran, past the paragraph at the left edge inside the triple quote"
  );
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  let whole = "My take: a stronger control model, which costs less.\n\nWhat I like: the paragraph at \
               the left edge.\nend";
  assert_eq!(got.as_ref().as_str(), Some(whole));
  let parts = said(&mut engine).into_iter().filter(|one| one.0 == "writing").count();
  assert_eq!(parts, 0, "no part of the word ran while the model wrote it");
}

#[test]
fn a_quote_that_the_model_writes_in_small_chunks_holds_until_its_close_mark_and_binds_whole() {
  // The close mark comes in a later chunk than the open mark, and no chunk ends at the end of a
  // line.
  let parts = ["<s:q", ">\nHel", "lo\nwor", "ld\n</s", ":q>\nclo", "se(len(", "q))"];
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let events: Vec<_> = parts
    .iter()
    .map(|part| MockStreamEvent::text(*part))
    .chain([MockStreamEvent::FinalResponse(StreamFinal::new("mock", usage))])
    .collect();
  let model = MockCompletionModel::from_stream_turns([events]);
  let (mut engine, _) = life("chunked", vec![scripted(&model)]);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(12), "the quote bound whole before close(len(q)) ran");
  assert!(!told_of(&mut engine, &["has no close mark"]), "no finding of an open quote");
}

#[test]
fn a_quote_on_the_line_of_a_fault_is_cut_at_its_close_mark_not_its_open_mark() {
  // The quote stands where the body of the block must, so the statement that cannot be python is
  // on the line of its open mark, and its close mark comes in a later chunk: the turn holds the
  // whole quote, so no raise says that the quote has no close mark.
  let parts = ["if True:\n<s:q", ">\nHel", "lo\nwor", "ld\n</s", ":q>\nclo", "se(len(", "q))"];
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let events: Vec<_> = parts
    .iter()
    .map(|part| MockStreamEvent::text(*part))
    .chain([MockStreamEvent::FinalResponse(StreamFinal::new("mock", usage))])
    .collect();
  let model = MockCompletionModel::from_stream_turns([events]);
  let (mut engine, _) = life("fault-quote", vec![scripted(&model)]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("if True:\n<s:q>\nHello\nworld\n</s:q>"));
  assert!(!told_of(&mut engine, &["has no close mark"]), "the quote that closes is not told open");
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
  use claude::Yard;
  let yard = Yard::new("chain");
  let claude = yard.claude(10_000);
  let (mut engine, _) = life("claude", on_reactor(claude.models()));
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
  assert_eq!(fs::read_to_string(prompt).expect("the file of the system prompt"), SYSTEM);
  let heard = yard.heard();
  assert_eq!(heard.len(), 2);
  assert!(heard[1].contains("second task") && !heard[1].contains("first task"), "{}", heard[1]);
  // The ids of chains repeat in every life, and a life on the same command line keeps a
  // conversation of its own.
  let (mut other, _) = life("claude-other", on_reactor(claude.models()));
  let with =
    verbs::Prompt { message: Some("first task".into()), on: on(&root), ..Default::default() };
  block_on(other.prompt(Object::string("str"), with).expect("a prompt is made"))
    .expect("an answer");
  // The first process of a conversation starts it, or resumes the turns of the setup of its chain
  // that furb wrote.
  let session = |pid: &str| {
    let args = yard.args(pid);
    claude::after(&args, "--session-id").or_else(|| claude::after(&args, "--resume"))
  };
  let both = yard.pids();
  assert_eq!(both.len(), 2);
  assert_ne!(session(&both[0]), session(&both[1]));
  drop((engine, other, claude));
  assert!(claude::gone(&pids[0]), "the process ends with the last hold of the command line");
}

#[cfg(unix)]
#[test]
fn the_claude_command_line_is_told_the_most_thought_of_the_provider_unless_the_environment_gave_it()
{
  use claude::Yard;
  let fake = Yard::new("most");
  let claude = fake.claude(10_000);
  let inherited = Thinking { most: 4096, inherited: true };
  for (name, most) in [("most-told", Thinking::default()), ("most-inherited", inherited)] {
    let provider = Provider::new(yard(name).display().to_string(), on_reactor(claude.models()));
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
  let given = env::var("MAX_THINKING_TOKENS").unwrap_or_else(|_| "unset".to_owned());
  assert_eq!(thinking, ["32768".to_owned(), given], "a most that the environment gave goes untold");
}

#[cfg(unix)]
#[test]
fn the_word_of_the_claude_command_line_is_the_text_it_settled_over_the_parts_it_streamed() {
  use claude::Yard;
  let yard = Yard::new("gap");
  let claude = yard.claude(10_000);
  let (mut engine, _) = life("claude-gap", on_reactor(claude.models()));
  let root = engine.root().to_owned();
  // The command line streams the text of its reply less its first part, and then says the text
  // whole.
  let with =
    verbs::Prompt { message: Some("GAP task".into()), on: on(&root), ..Default::default() };
  let act = engine.prompt(Object::string("str"), with).expect("a prompt is made");
  assert_eq!(block_on(act).map(|got| got.py_repr()).expect("an answer"), "'reply 1'");
  assert_eq!(yard.heard().len(), 1, "the first reply answered the prompt");
}

/// Whom a test tells what a model writes, and what it was told, each as its rung, its chain, its
/// text and its thought.
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
  let requests = model.requests();
  let [Message::User { content }] = chat(&requests[0].chat_history) else {
    panic!("one user turn")
  };
  let named = images::reference("a.png", &kept.uri);
  let parts: Vec<_> = content.iter().collect();
  let place = parts.iter().position(|one| matches!(one, UserContent::Image(_)));
  let place = place.expect("the image goes to the model");
  assert!(matches!(parts[place - 1], UserContent::Text(text) if text.text.ends_with(&named)));
  assert!(matches!(parts.get(place + 1), Some(UserContent::Text(_))), "the text after it follows");
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

/// An image attached in the images of a yard, whose file is then lost, and its uri.
fn lost(at: &Path) -> String {
  fs::write(at.join("a.png"), b"\x89PNG\r\n\x1a\nimage").expect("the image is written");
  let kept = images::attach(&at.join("images"), &at.join("a.png")).expect("the image is attached");
  let file = kept.uri.strip_prefix("furb-image://").expect("the uri of an image");
  fs::remove_file(at.join("images").join(file)).expect("the image is lost");
  kept.uri
}

#[test]
fn a_turn_that_names_an_image_whose_file_is_lost_gets_its_answer_and_the_life_tells_the_loss_once()
{
  for name in ["claude-cli:opus", "codex:gpt-6"] {
    let at = yard("images-lost");
    let uri = lost(&at);
    let message = format!("look at {}", images::reference("a.png", &uri));
    let model = streaming(&[("close(1)", Usage::new()), ("close(2)", Usage::new())]);
    let held = Model::new(name, 1000, model.clone()).effort("high", json!({})).images();
    let mut engine =
      lived(Provider::new(at.display().to_string(), vec![held]).images(at.join("images")));
    for want in ["1", "2"] {
      let root = engine.root().to_owned();
      let with =
        verbs::Prompt { message: Some(message.clone()), on: on(&root), ..Default::default() };
      let id =
        engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned();
      let told =
        |engine: &mut Engine| said(engine).iter().any(|one| one.2.contains("lost an image"));
      assert!(
        want == "2" || until(&mut engine, told),
        "{name} tells the loss: {:?}",
        said(&mut engine)
      );
      let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("an answer");
      assert_eq!(got.py_repr(), want, "{name}");
    }
    let first = &model.requests()[0].chat_history;
    let [Message::User { content }] = chat(first) else { panic!("one user turn: {first:?}") };
    assert!(content.iter().all(|one| matches!(one, UserContent::Text(_))), "no image: {name}");
    assert!(format!("{content:?}").contains(&uri), "the text names the image: {name}");
    let tells: Vec<_> = said(&mut engine).into_iter().filter(|one| one.0 == "tell").collect();
    let lost = tells.iter().filter(|one| one.2.contains("lost an image") && one.2.contains(&uri));
    assert_eq!(lost.count(), 1, "{name}: {tells:?}");
  }
}

#[cfg(unix)]
#[test]
fn the_claude_command_line_hears_a_turn_that_names_a_lost_image_as_its_text_alone() {
  use claude::Yard;
  let fake = Yard::new("images-lost");
  let at = yard("claude-images-lost");
  let uri = lost(&at);
  let provider = Provider::new(at.display().to_string(), on_reactor(fake.claude(10_000).models()))
    .images(at.join("images"));
  let mut engine = seeing(provider, Some(at.join("images")));
  let root = engine.root().to_owned();
  let message = format!("look at {}", images::reference("a.png", &uri));
  let with = verbs::Prompt { message: Some(message), on: on(&root), ..Default::default() };
  let act = engine.prompt(Object::string("str"), with).expect("a prompt is made");
  block_on(act).expect("an answer");
  let heard = fake.heard();
  assert!(heard[0].contains("![a.png]"), "the text names the image: {}", heard[0]);
  assert!(!heard[0].contains(r#""data":"#), "no image goes: {}", heard[0]);
  let lost = said(&mut engine).into_iter().filter(|one| one.2.contains("lost an image")).count();
  assert_eq!(lost, 1);
}

#[cfg(unix)]
#[test]
fn the_claude_command_line_sees_each_image_that_a_prompt_names_and_each_image_that_a_word_reads() {
  use base64::{Engine as _, engine::general_purpose::STANDARD};
  use claude::Yard;
  let fake = Yard::new("images");
  let at = yard("claude-images");
  let (named, read) = (b"\x89PNG\r\n\x1a\nnamed".as_slice(), b"\x89PNG\r\n\x1a\nread".as_slice());
  fs::write(at.join("named.png"), named).expect("the image is written");
  fs::write(at.join("read.png"), read).expect("the image is written");
  let kept =
    images::attach(&at.join("images"), &at.join("named.png")).expect("the image is attached");
  // The first turn reads a PNG, and the claude of the test answers the second with a close.
  let word = format!("read({:?})", at.join("read.png").display().to_string());
  fs::write(fake.at.join("word.1"), serde_json::to_string(&word).expect("a word of json"))
    .expect("the word is written");
  let provider = Provider::new(at.display().to_string(), on_reactor(fake.claude(10_000).models()))
    .images(at.join("images"));
  let mut engine = seeing(provider, Some(at.join("images")));
  let root = engine.root().to_owned();
  let message = format!("look at {}", images::reference("named.png", &kept.uri));
  let with = verbs::Prompt { message: Some(message), on: on(&root), ..Default::default() };
  let act = engine.prompt(Object::string("str"), with).expect("a prompt is made");
  // The claude of the test says that the turn that it answers holds an image.
  assert_eq!(block_on(act).map(|got| got.py_repr()).expect("an answer"), "'an image, reply 2'");
  let heard = fake.heard();
  let image = |bytes: &[u8]| format!(r#""data":"{}""#, STANDARD.encode(bytes));
  assert_eq!(heard.len(), 2, "{heard:?}");
  // The image stands after the text that names it and before the text that follows, not at the end
  // of the turn.
  let place = |text: &str| heard[0].find(text);
  let (name, data) = (place("![named.png]"), place(&image(named)));
  let after = heard[0].rfind("prompt1: Act[");
  assert!(
    matches!((name, data, after), (Some(a), Some(b), Some(c)) if a < b && b < c),
    "the image after its name and before the text that follows it: {}",
    heard[0]
  );
  assert!(heard[0].contains(&image(named)), "the image of the prompt: {}", heard[0]);
  assert!(heard[1].contains(&image(read)), "the image that the word read: {}", heard[1]);
  assert!(!heard[1].contains(&image(named)), "the command line hears an image once: {}", heard[1]);
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
  let last = request["messages"]
    .as_array()
    .and_then(|messages| messages.last())
    .cloned()
    .unwrap_or_default();
  assert_eq!(last["role"], "user");
  let texts = last["content"].as_array().cloned().unwrap_or_default();
  let texts = texts.iter().filter_map(|one| one["text"].as_str());
  assert!(
    texts.into_iter().any(|text| text.contains("#prompt1\nprompt1_message = 'count'\n")),
    "{last}"
  );
  let expected = "('assistant', 'close(7)', (10, 2, 0, 0, 1.0), None)";
  assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), expected.into()));
  assert_eq!(told.lock().expect("the parts told").len(), 1);
}

/// A model whose first calls fail with these failures, one call each, whose later calls answer
/// `close(7)`, and the moment of each call.
#[derive(Clone)]
struct Failing {
  failures: Arc<Mutex<Vec<CompletionError>>>,
  then: MockCompletionModel,
  moments: Arc<Mutex<Vec<Instant>>>,
}

impl Failing {
  fn new(failures: Vec<CompletionError>) -> Self {
    let then = streaming(&[("close(7)", Usage::new())]);
    Self { failures: Arc::new(Mutex::new(failures)), then, moments: Arc::default() }
  }

  fn calls(&self) -> usize {
    self.moments.lock().expect("the calls").len()
  }
}

impl CompletionModel for Failing {
  fn completion(
    &self,
    request: CompletionRequest,
  ) -> impl Future<Output = Result<CompletionResponse, CompletionError>> {
    self.then.completion(request)
  }

  async fn stream(
    &self,
    request: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    self.moments.lock().expect("the calls").push(Instant::now());
    let failed = {
      let mut failures = self.failures.lock().expect("the failures");
      (!failures.is_empty()).then(|| failures.remove(0))
    };
    match failed {
      Some(no) => Err(no),
      None => self.then.stream(request).await,
    }
  }
}

/// The 429 of a usage limit of a subscription that ends at this second since the epoch, as a
/// provider of the network words it.
fn usage_limit(resets_at: u64) -> CompletionError {
  let error = json!({"type": "usage_limit_reached", "resets_at": resets_at});
  too_many(&json!({"error": error}).to_string(), &[])
}

/// The second since the epoch that is this many seconds from now, or before it.
fn from_now(seconds: i64) -> u64 {
  let now =
    time::SystemTime::now().duration_since(time::UNIX_EPOCH).expect("a time after the epoch");
  now.as_secs().saturating_add_signed(seconds)
}

/// What the first call of a reply that fails so comes to: the retry that the host is told before
/// its wait, or the word of the done of the reply that the failure ends.
fn came_to(no: CompletionError) -> Result<Retry, String> {
  let (retries, told) = retried();
  let model = scripted(&Failing::new(vec![no]));
  let mut engine =
    lived(Provider::new(yard("came-to").display().to_string(), vec![model]).retries(retries));
  asked(&mut engine);
  let ended = |engine: &mut Engine| {
    !told.lock().expect("the retries told").is_empty()
      || said(engine).iter().any(|(kind, ..)| kind == "done")
  };
  assert!(until(&mut engine, ended), "{:?}", said(&mut engine));
  let first = told.lock().expect("the retries told").first().map(|one| one.2.clone());
  first.ok_or_else(|| {
    said(&mut engine).into_iter().find(|(kind, ..)| kind == "done").expect("a done").2
  })
}

/// Why the provider asks a model again after a call that fails with this text, if it does.
fn passes(why: &str) -> Option<String> {
  came_to(CompletionError::ProviderError(why.to_owned())).ok().map(|retry| retry.why)
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

#[test]
fn cut_turns_and_transient_provider_errors_are_recoverable() {
  // A transient failure is made again in the same reply.
  assert_eq!(passes("connection reset by peer").as_deref(), Some("connection reset"));
  assert_eq!(passes("503 Service Unavailable").as_deref(), Some("server error 503"));
  // A second refusal in a row pauses the chain unless the model can correct it in another turn.
  let pauses = |no: fn() -> CompletionError| {
    let model = Failing::new(vec![no(), no()]);
    let (mut engine, _) = life("recoverable", vec![scripted(&model)]);
    asked(&mut engine);
    let answered = |engine: &mut Engine| {
      said(engine).iter().any(|(kind, about, word)| {
        kind == "pause" || (kind == "done" && about == "reply3" && word.starts_with("('assistant'"))
      })
    };
    assert!(until(&mut engine, answered), "{:?}", said(&mut engine));
    said(&mut engine).iter().any(|(kind, ..)| kind == "pause")
  };
  let cut =
    || CompletionError::ResponseError("the turn reached the most it writes before its end".into());
  assert!(!pauses(cut), "a cut turn is recoverable");
  let refusals: [fn() -> CompletionError; 5] = [
    || CompletionError::ProviderError("the model does not accept images".into()),
    || CompletionError::ResponseError("invalid response format".into()),
    || CompletionError::ResponseError("prompt exceeds the model context window".into()),
    || CompletionError::ProviderError("400 prompt is too long".into()),
    || {
      CompletionError::ProviderError("Prompt is too long \u{b7} automatic compaction failed".into())
    },
  ];
  for no in refusals {
    assert!(pauses(no), "{}", no());
  }
}

#[test]
fn a_refusal_of_a_prompt_past_the_window_of_its_model_overflows() {
  let done = |why: &str| came_to(CompletionError::ProviderError(why.into())).unwrap_err();
  let ended = |kind: &str, why: &str| {
    format!("{kind}(args=('m/high answered nothing: ProviderError: {why}'))")
  };
  for why in [
    "prompt exceeds the model context window",
    "400 prompt is too long",
    "Prompt is too long \u{b7} automatic compaction failed",
    "400 context_length_exceeded: maximum context length is 8192 tokens",
  ] {
    assert_eq!(done(why), ended("Overflow", why));
  }
  let why = "the model does not accept images";
  assert_eq!(done(why), ended("Refused", why));
}

#[test]
fn only_a_failure_of_the_network_or_of_the_side_of_the_provider_passes() {
  // A claude command line that an update took away passes too, which
  // a_claude_that_an_update_took_away_is_asked_again_after_its_backoff_until_it_comes_back proves
  // beside the provider.
  assert_eq!(
    passes("error sending request: connection reset").as_deref(),
    Some("connection reset")
  );
  assert_eq!(passes("the request timed out").as_deref(), Some("timed out"));
  assert_eq!(passes("429 Too Many Requests").as_deref(), Some("rate limited"));
  assert_eq!(passes("Invalid status code 502 Bad Gateway").as_deref(), Some("server error 502"));
  assert_eq!(passes("unavailable"), None);
  assert_eq!(passes("boom"), None);
  assert_eq!(passes("400 Bad Request: unsupported parameter"), None);
  assert_eq!(passes("could not resolve host").as_deref(), Some("network error"));
  assert_eq!(passes("No model is m/x."), None);
  let said = "No program to run: claude did not start: No such file or directory (os error 2)";
  assert_eq!(passes(said), None, "the text alone is no missing program");
  assert_eq!(passes("Claude exited (1): Error: 'claude' was not found on PATH."), None);
  let stopped = CompletionError::ResponseError("the stream stopped, 503".into());
  assert!(came_to(stopped).is_err(), "a failure of the turn does not pass");
}

#[test]
fn a_claude_command_line_that_cannot_reach_its_server_fails_on_the_network_and_passes() {
  let lost = "API Error: Can't reach the API server, check your internet or DNS (ENOTFOUND)";
  assert_eq!(passes(lost).as_deref(), Some("network error"));
  assert_eq!(passes("getaddrinfo EAI_AGAIN api.anthropic.com").as_deref(), Some("network error"));
}

#[test]
fn the_host_is_told_nothing_more_of_what_a_model_writes_once_its_reply_is_over() {
  // A host writes from a work of its own, which the end of the reply does not end, as JavaScript
  // does: here once the test says the reply is over.
  let (go, gone) = mpsc::channel::<()>();
  let gone = Arc::new(Mutex::new(gone));
  let host: Hosted = Arc::new(move |_, told| {
    told("before", "");
    let gone = Arc::clone(&gone);
    thread::spawn(move || {
      let _ = gone.lock().expect("the signal of the test").recv();
      told("after", "");
    });
    std_future::pending().boxed()
  });
  let (writes, told) = heard();
  let at = yard("over");
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let mut engine = lived(Provider::new(at.display().to_string(), vec![model]).writes(writes));
  let id = asked(&mut engine);
  let parts = |told: &Arc<Mutex<Vec<[String; 4]>>>| told.lock().expect("the parts told").len();
  assert!(until(&mut engine, |_| parts(&told) == 1), "the model wrote a part");
  engine.cancel(&id, verbs::Cancel::default()).expect("the prompt is cancelled");
  let over = |engine: &mut Engine| {
    let root = engine.root().to_owned();
    let facts = engine
      .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
      .expect("the transcript");
    facts.iter().any(|one| one.kind() == "done" && one.about().starts_with("reply"))
  };
  assert!(until(&mut engine, over), "the reply is over");
  // The ear hears the done of the reply in the pump that says it, and the pumps after it give it
  // room.
  within(&mut engine, Duration::from_millis(100), |_| false);
  go.send(()).expect("the host writes again");
  assert!(
    !within(&mut engine, Duration::from_millis(500), |_| parts(&told) > 1),
    "nothing more is told"
  );
}

/// Whether a word that holds this text ran on the root.
fn ran(engine: &mut Engine, text: &str) -> bool {
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
fn a_word_that_ends_with_a_line_of_three_backticks_runs_without_it_and_the_chain_is_told_first() {
  // The fence is a line at the left edge, so the statements before it ran as the model wrote it.
  let model = streaming(&[("x = 3\ny = x\n```\n", Usage::new())]);
  let (mut engine, _) = life("unclosed", vec![scripted(&model)]);
  asked(&mut engine);
  replied(&mut engine);
  assert!(until(&mut engine, |engine| ran(engine, "y = x")), "the word runs");
  let facts = said(&mut engine);
  let kinds: Vec<_> =
    facts.iter().map(|(kind, about, _)| (kind.as_str(), about.as_str())).take(4).collect();
  assert_eq!(
    kinds,
    [("started", "reply1"), ("writing", "rung1"), ("tell", "rung1"), ("done", "reply1")]
  );
  assert!(facts[2].2.contains("the provider dropped that line"), "{}", facts[2].2);
  assert!(facts[3].2.starts_with("('assistant', 'x = 3\\ny = x', "), "{}", facts[3].2);
  assert!(facts[3].2.ends_with("[{'type': 'text', 'text': 'x = 3\\ny = x'}])"), "{}", facts[3].2);
}

/// The parts of the word of the root that the provider said as writings, each with how many
/// characters of the word stand before it.
fn writings(engine: &mut Engine) -> Vec<(String, i64)> {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
    .expect("the root has a transcript");
  let part = |one: &Fact| {
    let text = one.word(0).and_then(|word| word.as_str().map(str::to_owned));
    text.zip(one.word(1).and_then(|word| word.as_int()))
  };
  facts.iter().filter(|one| one.kind() == "writing").filter_map(part).collect()
}

/// The word of the turn that the first reply of the root was done with, once it is done.
fn turn(engine: &mut Engine) -> String {
  replied(engine);
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
    .expect("the root has a transcript");
  let done = facts.iter().find(|one| {
    one.by() == "provider" && one.kind() == "done" && fact::named(one.about(), "reply")
  });
  let items = done.and_then(|one| one.word(0)).and_then(|word| word.items());
  items.and_then(|items| items.get(1)?.as_str().map(str::to_owned)).expect("a turn has a word")
}

/// A life whose model streams this text as its reply in one part, and the word of the turn that
/// the reply was done with.
fn turned(name: &str, text: &str) -> (Engine, String) {
  let model = streaming(&[(text, Usage::new())]);
  let (mut engine, _) = life(name, vec![scripted(&model)]);
  asked(&mut engine);
  let word = turn(&mut engine);
  (engine, word)
}

/// Each text that a model streams as its reply comes to the turn beside it, and the chain of the
/// first text is told this of what the provider dropped.
fn turns(name: &str, rows: &[(&str, &str)], told: &str) {
  for (at, (text, word)) in rows.iter().enumerate() {
    let (mut engine, got) = turned(&format!("{name}{at}"), text);
    assert_eq!((text, got.as_str()), (text, *word));
    assert!(at > 0 || told_of(&mut engine, &[told]), "{text:?} is told {told:?}");
  }
}

#[test]
fn the_statements_of_a_word_run_in_parts_that_end_where_a_line_at_the_left_edge_starts() {
  let search = "s = await act('search', '', None, 'q', 'm')\n";
  let parts = ["x = 1\n", search, "if x:\n  y", " = 2\nelsewhere", " = 3\n", "w = 4\n"];
  let model = Model::new("m", 1000, Halting::writes(&parts)).searches(found_by_native());
  let (mut engine, _) = life("parts", vec![model]);
  asked(&mut engine);
  let given = ["x = 1\n", search, "if x:\n  y = 2\n", "elsewhere = 3\n"];
  assert!(
    until(&mut engine, |engine| given.iter().all(|part| wrote(engine, part))),
    "{:?}",
    writings(&mut engine)
  );
  let starts: Vec<_> = writings(&mut engine).into_iter().map(|(part, _)| part).collect();
  assert_eq!(starts, given, "a name that starts with else opens no clause of the if before it");
}

#[test]
fn a_part_counts_the_characters_of_the_word_before_it_as_python_counts_them() {
  let model = Halting::writes(&["é = 1\n", "\n", "y = 2\n", "z"]);
  let (mut engine, _) = life("counted", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert!(until(&mut engine, |engine| writings(engine).len() == 2), "{:?}", writings(&mut engine));
  assert_eq!(writings(&mut engine), [("é = 1\n\n".to_owned(), 0), ("y = 2\n".to_owned(), 7)]);
}

#[test]
fn a_whole_word_whose_end_more_text_could_mend_is_the_word_that_ran_and_its_chain_is_told_the_line()
{
  let (mut engine, word) = turned("mendable", "x = 1\ny = (2,\n");
  assert_eq!(word, "x = 1");
  let told = ["is the word that ran", "line 2: the statement there cannot be python"];
  assert!(until(&mut engine, |engine| told_of(engine, &told)));
}

#[test]
fn a_text_loses_the_marks_of_a_fence_around_it_and_keeps_all_else() {
  let quoted = "```python\n<s:doc>\n```sh\nls\n```\n</s:doc>\nclose(doc)\n```";
  let rows = [
    ("```python\nclose(3)\n```", "close(3)"),
    ("```\nclose(3)", "close(3)"),
    (quoted, "<s:doc>\n```sh\nls\n```\n</s:doc>\nclose(doc)"),
    ("close(3)\n```", "close(3)"),
    ("x = 1\n```\n", "x = 1"),
    ("```js and more\nclose(3)", "```js and more"),
  ];
  turns("fenced", &rows, "wrapped its word in a fence");
}

#[test]
fn a_text_loses_the_markup_of_a_tool_call_around_it_and_keeps_all_else() {
  let call =
    "<invoke name=\"python\">\n<parameter name=\"code\">\nclose(3)\n</parameter>\n</invoke>";
  let inline = "<function_calls>\n<invoke name=\"python\">\n<parameter name=\"code\">x = 1\nclose(x)</parameter>\n</invoke>\n</function_calls>";
  let rows = [
    (call, "close(3)"),
    (inline, "x = 1\nclose(x)"),
    ("<invoke name=\"bash\">", ""),
    ("x = 1\n<invoke name=\"a\">", "x = 1"),
    ("<s:invoke>\nhi\n</s:invoke>\nclose(invoke)", "<s:invoke>\nhi\n</s:invoke>\nclose(invoke)"),
  ];
  turns("called", &rows, "in the markup of a tool call");
}

#[test]
fn the_close_of_a_tag_of_a_tool_call_alone_after_a_word_of_python_goes() {
  let rows = [
    ("x = 1\nclose(x)\n</parameter>", "x = 1\nclose(x)"),
    ("x = 1\n</invoke>\n</function_calls>", "x = 1"),
    ("<s:doc>\nhi\n</s:doc>\nclose(doc)\n</parameter>", "<s:doc>\nhi\n</s:doc>\nclose(doc)"),
    ("x = \"\"\"\n</parameter>\n\"\"\"", "x = \"\"\"\n</parameter>\n\"\"\""),
    ("<s:doc>\n</parameter>\n</s:doc>\nclose(doc)", "<s:doc>\n</parameter>\n</s:doc>\nclose(doc)"),
    ("x = (\n</parameter>", "x = (\n</parameter>"),
  ];
  turns("closed", &rows, "in the markup of a tool call");
}

#[test]
fn the_markup_of_a_tool_call_of_qwen_goes_as_that_of_a_tool_call_of_claude_does() {
  let call = "<tool_call>\n<function=write>\n<parameter=content>\nx = 1\nclose(x)\n</parameter>\n</function>\n</tool_call>";
  let rows = [
    (call, "x = 1\nclose(x)"),
    ("<function=write><parameter=content>x = 1", "x = 1"),
    ("<function=write>\n<parameter=content>x = 1\nclose(x)</function>", "x = 1\nclose(x)"),
    ("x = 1\n<function=write>", "x = 1"),
  ];
  turns("qwen", &rows, "the text of a file is a quote");
}

#[test]
fn a_break_of_html_alone_goes_before_the_word() {
  let rows = [
    ("<br>", ""),
    ("<br/>\n```\nclose(1)", "close(1)"),
    ("<BR />\nclose(1)", "close(1)"),
    ("close(1)\n<br>", "close(1)"),
  ];
  turns("broke", &rows, "in a break of HTML");
}

#[test]
fn a_first_line_that_starts_with_a_lone_equal_sign_binds_an_underscore() {
  let rows = [
    ("= 3\nclose(_)", "_ = 3\nclose(_)"),
    ("```python\n= read(\"a\")\n```", "_ = read(\"a\")"),
    ("x == 1", "x == 1"),
    ("== 1", "== 1"),
    ("x = 1\n= 2", "x = 1"),
  ];
  turns("unnamed", &rows, "a binding with no name");
}

#[test]
fn a_fence_or_the_markup_of_a_tool_call_around_a_word_goes_as_each_of_its_lines_ends() {
  let call = "<invoke name=\"python\">\n<parameter name=\"code\">x = 1\ny = 2\nclose(y)</parameter>\n</invoke>";
  for (at, text) in ["```python\nx = 1\ny = 2\nclose(y)\n```", call].into_iter().enumerate() {
    let (mut engine, word) = turned(&format!("wrapped{at}"), text);
    assert_eq!(word, "x = 1\ny = 2\nclose(y)");
    assert_eq!(writings(&mut engine), [("x = 1\ny = 2\n".to_owned(), 0)], "{text:?}");
  }
}

#[test]
fn the_mark_that_ends_a_fence_or_a_tool_call_starts_no_statement() {
  for (at, end) in ["```", "```\n", "</parameter>", "</invoke>\n"].into_iter().enumerate() {
    let (mut engine, word) = turned(&format!("ends{at}"), &format!("x = 1\nclose(x)\n{end}"));
    assert_eq!(word, "x = 1\nclose(x)");
    assert_eq!(writings(&mut engine), [("x = 1\n".to_owned(), 0)], "{end:?}");
  }
}

#[test]
fn a_line_of_three_backticks_at_the_end_of_a_word_stays_while_a_quote_stands_open_before_it() {
  let rows: Vec<Value> =
    serde_json::from_str(include_str!("../../quotes.json")).expect("the table of quotes");
  for (at, row) in rows.iter().enumerate() {
    let word = row["word"].as_str().expect("a word");
    let text = format!("{word}\n```");
    let (_, got) = turned(&format!("quoted{at}"), &text);
    let open = row["open"].as_bool().expect("whether a quote stands open");
    assert_eq!(got, if open { text.as_str() } else { word.trim_end() }, "{word:?}");
  }
}

#[test]
fn a_mark_whose_name_is_no_python_name_opens_no_quote_so_a_reply_ends_at_its_line() {
  let (mut engine, word) = turned("dashed", "<s:old-text>\nHello\n```");
  assert_eq!(word, "<s:old-text>");
  assert!(until(&mut engine, |engine| told_of(engine, &["raised", "SyntaxError"])));
}

#[test]
fn a_model_asked_again_writes_its_word_from_its_start() {
  let usage = Usage { input_tokens: 10, output_tokens: 5, total_tokens: 15, ..Default::default() };
  let halting =
    Halting { breaks: Some(|| usage_limit(from_now(-3600))), ..Halting::writes(&["x = (\n"]) };
  let model =
    First { halting, then: streaming(&[("z = 3\nclose(z)", usage)]), called: Arc::default() };
  let (mut engine, _) = life("again", vec![scripted(&model)]);
  let id = asked(&mut engine);
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(3))
  };
  assert!(until(&mut engine, done), "{:?}", said(&mut engine));
  assert_eq!(writings(&mut engine), [("z = 3\n".to_owned(), 0)]);
}

/// A model whose one stream says each part that the test sends it, and ends when the test drops
/// its sender.
#[derive(Clone)]
struct Paced(Arc<Mutex<Option<UnboundedReceiver<RawStreamingChoice>>>>);

impl Paced {
  fn new() -> (Self, UnboundedSender<RawStreamingChoice>) {
    let (send, said) = unbounded();
    (Self(Arc::new(Mutex::new(Some(said)))), send)
  }
}

impl CompletionModel for Paced {
  fn completion(
    &self,
    _: CompletionRequest,
  ) -> impl Future<Output = Result<CompletionResponse, CompletionError>> {
    std_future::ready(Err(CompletionError::ProviderError("the provider streams".into())))
  }

  fn stream(
    &self,
    _: CompletionRequest,
  ) -> impl Future<Output = Result<StreamingCompletionResponse, CompletionError>> {
    let said = self.0.lock().expect("the parts").take().expect("the model streams once");
    std_future::ready(Ok(StreamingCompletionResponse::stream("paced", Box::pin(said.map(Ok)))))
  }
}

#[test]
fn a_word_whose_first_part_a_pause_held_runs_whole_with_its_turn_and_in_no_part() {
  let (model, send) = Paced::new();
  let (writes, told) = heard();
  let provider = Provider::new(yard("held").display().to_string(), vec![scripted(&model)]);
  let mut engine = lived(provider.writes(writes));
  let id = asked(&mut engine);
  let started = |engine: &mut Engine| said(engine).iter().any(|(kind, ..)| kind == "started");
  assert!(until(&mut engine, started), "the provider took the reply");
  let root = engine.root().to_owned();
  engine.pause(&root, verbs::Pause::default()).expect("the chain pauses");
  let part = |text: &str| RawStreamingChoice::Message(text.to_owned());
  send.unbounded_send(part("x = 1\ny")).expect("the model writes");
  send.unbounded_send(part(" = 2\n")).expect("the model writes");
  let parts = |told: &Arc<Mutex<Vec<[String; 4]>>>| told.lock().expect("the parts told").len();
  assert!(until(&mut engine, |_| parts(&told) == 2), "the ear heard each part in turn");
  engine.wake(&root).expect("the chain wakes");
  send.unbounded_send(part("z = 3\nclose(z)")).expect("the model writes");
  let end = RawStreamingChoice::FinalResponse(StreamFinal::new("paced", Usage::new()));
  send.unbounded_send(end).expect("the model ends");
  drop(send);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  assert_eq!(writings(&mut engine), []);
}

#[test]
fn a_call_that_the_usage_limit_refuses_waits_for_its_reset_and_the_chain_is_told_until_when() {
  let failing = Failing::new(vec![usage_limit(from_now(1))]);
  let (retries, told) = retried();
  let model = scripted(&failing);
  let provider = Provider::new(yard("limit").display().to_string(), vec![model]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7));
  assert_eq!(failing.calls(), 2);
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
  assert!(
    note.contains(&format!("rung(\"actor = \\'<model>\\'\", on=\"{root}\") and wake(\"{root}\")")),
    "{note}"
  );
}

/// The 429 of a provider, with this body and these headers.
fn too_many(body: &str, headers: &[(&'static str, &str)]) -> CompletionError {
  let response = http_client::Response::builder().status(429).body(()).expect("a response");
  let mut held = http_client::HeaderMap::new();
  for (name, value) in headers {
    held.insert(*name, http_client::HeaderValue::from_str(value).expect("a header value"));
  }
  CompletionError::HttpError(http_client::Error::InvalidStatusCodeWithDetails {
    status: response.status(),
    body: body.to_owned(),
    headers: Box::new(held),
  })
}

/// A call that a 429 with this body refuses goes again after the reset that the body names, and
/// the chain is told that reset once.
fn waits_for_the_reset_of(name: &str, body: &Value) {
  let failing = Failing::new(vec![too_many(&body.to_string(), &[])]);
  let (retries, told) = retried();
  let provider =
    Provider::new(yard(name).display().to_string(), vec![scripted(&failing)]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7));
  let moments = failing.moments.lock().expect("the calls").clone();
  assert_eq!(moments.len(), 2);
  assert!(moments[1] - moments[0] >= Duration::from_secs(1), "{:?}", moments[1] - moments[0]);
  let heard = told.lock().expect("the retries told").clone();
  assert_eq!(heard.len(), 1);
  assert!(heard[0].2.why.starts_with("usage limit until "), "{}", heard[0].2.why);
  assert!(heard[0].2.wait <= Duration::from_secs(4), "{:?}", heard[0].2.wait);
}

#[test]
fn a_429_of_a_usage_limit_that_gives_resets_in_seconds_is_asked_again_after_that_reset() {
  let error = json!({"type": "usage_limit_reached", "resets_in_seconds": 1});
  waits_for_the_reset_of("limit-in-seconds", &json!({"error": error}));
}

#[test]
fn a_429_of_a_usage_limit_that_gives_resets_at_alone_is_asked_again_after_that_reset() {
  let now =
    time::SystemTime::now().duration_since(time::UNIX_EPOCH).expect("a time after the epoch");
  let error = json!({"type": "usage_limit_reached", "resets_at": now.as_secs() + 2});
  waits_for_the_reset_of("limit-at", &json!({"error": error}));
}

#[test]
fn a_429_is_a_usage_limit_only_when_its_body_says_so_and_a_number_past_the_range_gives_no_end() {
  let why = |no: CompletionError| came_to(no).expect("a 429 is asked again").why;
  let plain = too_many(r#"{"error": {"type": "rate_limit_error"}}"#, &[("retry-after", "20")]);
  assert_eq!(why(plain), "rate limited");
  let limit = r#"{"error": {"type": "usage_limit_reached"}}"#;
  // The end of the limit is twenty seconds after the 429, which a minute of the clock may part.
  let after = |zoned: Zoned| {
    let at = zoned.checked_add(SignedDuration::from_secs(20)).expect("a later moment");
    at.strftime("usage limit until %Y-%m-%d %H:%M (%Q)").to_string()
  };
  let before = after(Zoned::now());
  let got = why(too_many(limit, &[("retry-after", "20")]));
  assert!([before, after(Zoned::now())].contains(&got), "{got}");
  assert_eq!(why(too_many(limit, &[])), "usage limit");
  let huge = json!({"error": {"type": "usage_limit_reached", "resets_at": u64::MAX}});
  assert_eq!(why(too_many(&huge.to_string(), &[])), "usage limit");
}

#[test]
fn a_stand_ends_the_wait_of_a_usage_limit_and_asks_the_model_again_at_once() {
  let failing = Failing::new(vec![usage_limit(from_now(86_400))]);
  let (retries, told) = retried();
  let model = scripted(&failing);
  let provider =
    Provider::new(yard("limit-now").display().to_string(), vec![model]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let heard = Arc::clone(&told);
  assert!(until(&mut engine, move |_| heard.lock().expect("the retries told").len() == 1));
  assert!(told.lock().expect("the retries told")[0].2.wait > Duration::from_secs(80_000));
  engine.stand(verbs::Stand { on: Some("chain1".to_owned()) }).expect("the life stands");
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(7))
  };
  assert!(until(&mut engine, done), "the reply waits no more for the reset of the limit");
  assert_eq!(failing.calls(), 2);
}

#[test]
fn a_stand_that_meets_the_same_usage_limit_again_tells_the_chain_nothing_more() {
  let failing = Failing::new(vec![usage_limit(from_now(86_400)), usage_limit(from_now(86_400))]);
  let (retries, told) = retried();
  let model = scripted(&failing);
  let provider =
    Provider::new(yard("limit-twice").display().to_string(), vec![model]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  for waits in 1..=2 {
    let heard = Arc::clone(&told);
    assert!(until(&mut engine, move |_| heard.lock().expect("the retries told").len() == waits));
    engine.stand(verbs::Stand { on: Some("chain1".to_owned()) }).expect("the life stands");
  }
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(7))
  };
  assert!(until(&mut engine, done), "the reply asks the model again after the second stand");
  assert_eq!(failing.calls(), 3);
  let notes = said(&mut engine)
    .into_iter()
    .filter(|(kind, _, note)| kind == "tell" && note.contains("waits on"))
    .count();
  assert_eq!(notes, 1);
}

#[test]
fn a_limit_that_stands_after_its_reset_is_asked_again_soon_and_the_chain_is_told_once() {
  let failing = Failing::new(vec![usage_limit(from_now(-3600)), usage_limit(from_now(-3600))]);
  let (retries, told) = retried();
  let model = scripted(&failing);
  let provider =
    Provider::new(yard("limit-twice").display().to_string(), vec![model]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(7));
  assert_eq!(failing.calls(), 3);
  assert_eq!(told.lock().expect("the retries told").len(), 2);
  let kinds: Vec<_> = said(&mut engine).into_iter().map(|(kind, ..)| kind).collect();
  assert_eq!(kinds, ["started", "tell", "done"]);
}

#[cfg(unix)]
#[test]
fn a_claude_command_line_at_its_usage_limit_is_asked_again_at_the_reset_and_answers_with_no_pause()
{
  use claude::Yard;
  let yard = Yard::new("limit");
  let claude = yard.claude(10_000);
  let (mut engine, _) = life("claude-limit", on_reactor(claude.models()));
  let root = engine.root().to_owned();
  let with =
    verbs::Prompt { message: Some("LIMIT task".into()), on: on(&root), ..Default::default() };
  let act = engine.prompt(Object::string("str"), with).expect("a prompt is made");
  assert_eq!(block_on(act).expect("the prompt is answered").py_repr(), "'reply 1'");
  let kinds: Vec<_> = said(&mut engine).into_iter().map(|(kind, ..)| kind).collect();
  assert_eq!(kinds, ["started", "tell", "done"]);
  assert_eq!(yard.heard().len(), 2, "the reply asked the command line again after the reset");
}

/// Whether the Kernel says that a writing of this part of a word ran with nothing raised.
fn wrote(engine: &mut Engine, part: &str) -> bool {
  let root = engine.root().to_owned();
  let facts = engine
    .transcript(verbs::Transcript { on: on(&root), of: None, at: None })
    .expect("the root has a transcript");
  let place = |one: &Fact| (one.about().to_owned(), one.word(1).and_then(|word| word.as_int()));
  let made: Vec<_> = facts
    .iter()
    .filter(|one| {
      one.kind() == "writing" && one.word(0).and_then(|word| word.as_str()) == Some(part)
    })
    .map(place)
    .collect();
  facts.iter().any(|one| {
    let ran = (one.about().to_owned(), one.word(0).and_then(|word| word.as_int()));
    one.kind() == "ran"
      && made.contains(&ran)
      && one.word(1).is_some_and(|word| word.type_name() == "NoneType")
  })
}

#[test]
fn a_statement_that_a_model_finished_runs_while_the_model_writes_the_rest_of_its_word() {
  let model = Halting::writes(&["x = 3\n", "y = x\n"]);
  let (mut engine, _) = life("early", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert!(
    until(&mut engine, |engine| wrote(engine, "x = 3\n")),
    "the statement ran as its model wrote"
  );
  assert!(
    !wrote(&mut engine, "y = x\n"),
    "a statement that may go on runs once a line after it starts"
  );
  assert!(!ran(&mut engine, "x = 3"), "the word has no run while its model writes it");
}

#[test]
fn a_statement_that_raised_ends_the_reply_with_the_word_that_ran_whose_run_is_done_with_the_raise()
{
  let model = Halting::writes(&["x = int(\"a\")\n", "close(5)\n"]);
  let (mut engine, _) = life("raised", vec![Model::new("m", 1000, model.clone())]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("x = int(\"a\")"));
  assert!(
    until(&mut engine, |_| model.dropped.load(Ordering::SeqCst)),
    "the call of the model ended before it wrote the rest"
  );
  assert!(
    until(&mut engine, |engine| told_of(engine, &["raised", "ValueError"])),
    "the run of the word that ran is done with the raise"
  );
  assert!(!ran(&mut engine, "close(5)"), "no statement after the raise runs");
}

#[test]
fn once_a_part_ran_a_fault_after_it_ends_the_reply_with_the_word_that_ran_and_the_chain_is_told_the_fault()
 {
  let model = Halting::writes(&["x = 3\n", "y = = 2\n", "z = 1\n"]);
  let (mut engine, _) = life("after", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("x = 3"));
  assert!(
    until(&mut engine, |engine| told_of(
      engine,
      &["is the word that ran", "line 2: the statement there cannot be python"]
    )),
    "the chain is told the fault of the statement after the word that ran"
  );
  assert!(until(&mut engine, |engine| ran(engine, "x = 3")), "the word that ran runs");
}

#[test]
fn the_note_of_a_fault_after_the_word_that_ran_is_python_that_binds_its_why_under_the_rung() {
  let model = Halting::writes(&["x = 3\n", "y = = 2\n", "z = 1\n"]);
  let (mut engine, _) = life("noted", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("x = 3"));
  assert!(until(&mut engine, |engine| told_of(
    engine,
    &["is the word that ran", "_why = 'line 2: the statement there cannot be python'"]
  )));
  let next = |engine: &mut Engine| {
    said(engine).iter().any(|(kind, about, _)| kind == "done" && about == "reply2")
  };
  assert!(until(&mut engine, next), "the chain asks its model again after the note");
  let root = engine.root().to_owned();
  let facts = engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
  let raised: Vec<String> = facts
    .iter()
    .filter(|one| one.kind() == "done" && one.about().starts_with("run"))
    .map(|one| one.word(0).map(|word| word.py_repr()).unwrap_or_default())
    .filter(|word| word.contains("SyntaxError"))
    .collect();
  assert!(raised.is_empty(), "the turn that holds the note runs as python: {raised:?}");
}

#[test]
fn the_note_of_a_fault_after_the_word_that_ran_shows_the_last_lines_that_ran() {
  let model = Halting::writes(&["x = 3\n", "y = x\n", "Now I know.\n", "z = 1\n"]);
  let (mut engine, _) = life("stopped", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("x = 3\ny = x"));
  assert!(
    until(&mut engine, |engine| told_of(
      engine,
      &[
        "each act, close and name of that part stands",
        "line 1: x = 3",
        "line 2: y = x",
        "no later statement runs"
      ]
    )),
    "the chain is told the last lines that ran"
  );
}

#[test]
fn a_turn_that_is_not_python_after_the_part_that_ran_is_the_word_that_ran() {
  let model = streaming(&[("x = 3\ny = x\nThat is all, the count is done.", Usage::new())]);
  let (mut engine, _) = life("markup", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("x = 3\ny = x"));
  assert!(
    until(&mut engine, |engine| told_of(
      engine,
      &["line 2: y = x", "line 3: the statement there cannot be python"]
    )),
    "the chain is told the line of the turn that cannot be python"
  );
}

#[test]
fn a_reply_whose_call_ended_after_a_part_of_its_word_ran_is_the_word_that_ran() {
  let model = Halting::breaks(&["x = 3\n", "y = x"]);
  let (mut engine, _) = life("ended", vec![Model::new("m", 1000, model)]);
  asked(&mut engine);
  assert_eq!(replied(&mut engine), cut("x = 3"));
  assert!(
    until(&mut engine, |engine| told_of(
      engine,
      &["the call of its model ended after a part of it ran"]
    )),
    "the chain is told why its turn is the word that ran"
  );
}

#[test]
fn a_status_is_the_one_of_an_http_error_or_the_one_that_heads_or_follows_status_code() {
  assert_eq!(passes("503 Service Unavailable").as_deref(), Some("server error 503"));
  assert_eq!(
    passes("Invalid status code 504 with message: x").as_deref(),
    Some("server error 504")
  );
  assert_eq!(passes("the file /tmp/500/a was not found"), None);
  assert_eq!(passes("No model listens on 127.0.0.1:5029"), None);
  let response = http_client::Response::builder().status(502).body(());
  let status = http_client::Error::InvalidStatusCode(response.expect("a response").status());
  let retry = came_to(CompletionError::HttpError(status)).expect("a status of the server passes");
  assert_eq!(retry.why, "server error 502");
}

/// The parts of each user turn of a request, as `text` and `image`.
fn parts(history: &[Message]) -> Vec<Vec<&'static str>> {
  let kind = |part: &UserContent| match part {
    UserContent::Text(_) => "text",
    UserContent::Image(_) => "image",
    _ => "other",
  };
  let users = history.iter().filter_map(|one| match one {
    Message::User { content } => Some(content.iter().map(kind).collect()),
    Message::Assistant { .. } | Message::System { .. } => None,
  });
  users.collect()
}

/// A life of a model that sees images, on a gallery of the yard that holds one image, and the name
/// by which a message shows that image.
fn imaged(name: &str, model: &MockCompletionModel) -> (Engine, Yard, String) {
  let at = yard(name);
  fs::write(at.join("a.png"), b"\x89PNG\r\n\x1a\nimage").expect("the image is written");
  let gallery = at.join("images");
  let kept = images::attach(&gallery, &at.join("a.png")).expect("the image is attached");
  let models = vec![scripted(model).images()];
  let provider = Provider::new(at.display().to_string(), models).images(&gallery);
  (lived(provider), at, images::reference("a.png", &kept.uri))
}

/// A prompt on the root of a life with this message, which wants an int.
fn messaged(engine: &mut Engine, message: String) -> String {
  let root = engine.root().to_owned();
  let with = verbs::Prompt { message: Some(message), on: on(&root), ..Default::default() };
  engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned()
}

/// How many images the user turns of a request hold.
fn pictured(request: &CompletionRequest) -> usize {
  parts(chat(&request.chat_history)).concat().iter().filter(|one| **one == "image").count()
}

#[test]
fn a_read_of_an_image_hands_it_to_the_model_and_a_text_that_names_an_image_hands_none() {
  let at = yard("read-images");
  fs::write(at.join("a.png"), b"\x89PNG\r\n\x1a\nimage").expect("the image is written");
  let gallery = at.join("images");
  let kept = images::attach(&gallery, &at.join("a.png")).expect("the image is attached");
  fs::write(at.join("a.md"), format!("see {}", images::reference("a.png", &kept.uri)))
    .expect("the text is written");
  let read = |name: &str| format!("read({:?})", at.join(name).display().to_string());
  let (text, image) = (read("a.md"), read("a.png"));
  let model =
    streaming(&[(&text, Usage::new()), (&image, Usage::new()), ("close(1)", Usage::new())]);
  let provider =
    Provider::new(at.display().to_string(), vec![scripted(&model).images()]).images(&gallery);
  let mut engine = seeing(provider, Some(gallery));
  let id = asked(&mut engine);
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  let requests = model.requests();
  assert_eq!((pictured(&requests[1]), pictured(&requests[2])), (0, 1));
}

#[test]
fn an_image_that_a_message_names_twice_comes_once_and_a_life_with_no_gallery_hands_none() {
  let model = streaming(&[("close(1)", Usage::new())]);
  let (mut engine, at, named) = imaged("images-twice", &model);
  let id = messaged(&mut engine, format!("look at {named} and {named}"));
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  let request = model.requests().remove(0);
  let [.., Message::User { content }] = chat(&request.chat_history) else { panic!("a user turn") };
  let parts: Vec<_> = content.iter().collect();
  let place = parts.iter().position(|one| matches!(one, UserContent::Image(_))).expect("an image");
  assert!(
    matches!(parts[place - 1], UserContent::Text(text) if text.text.ends_with(&format!("look at {named}")))
  );
  assert_eq!(pictured(&request), 1, "the image comes after its first name alone");
  let blind = streaming(&[("close(1)", Usage::new())]);
  let provider = Provider::new(at.display().to_string(), vec![scripted(&blind).images()]);
  let mut engine = lived(provider);
  let id = messaged(&mut engine, format!("look at {named}"));
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(pictured(&blind.requests()[0]), 0, "a life with no gallery hands no image");
}

#[test]
fn an_image_whose_file_fails_to_load_for_another_cause_than_its_loss_fails_the_reply() {
  let model = streaming(&[("close(1)", Usage::new())]);
  let (mut engine, at, named) = imaged("images-broken", &model);
  let file = fs::read_dir(at.join("images")).expect("the gallery").next().expect("an image");
  let file = file.expect("an entry").path();
  fs::remove_file(&file).expect("the image goes");
  fs::create_dir(&file).expect("a folder stands where the image stood");
  messaged(&mut engine, format!("look at {named}"));
  assert!(until(&mut engine, |engine| said(engine).len() >= 2), "{:?}", said(&mut engine));
  let (kind, about, word) = said(&mut engine).remove(1);
  assert_eq!((kind.as_str(), about.as_str()), ("done", "reply1"));
  assert!(word.starts_with("Refused("), "{word}");
  assert_eq!(model.request_count(), 0, "the model is asked nothing");
}

#[test]
fn each_image_comes_once_in_a_request_after_the_text_that_names_it_as_a_fork_gives_its_origin_again()
 {
  let model = streaming(&[("close(1)", Usage::new()), ("close(2)", Usage::new())]);
  let (mut engine, _at, named) = imaged("images-once", &model);
  let root = engine.root().to_owned();
  let with = verbs::Prompt {
    message: Some(format!("look at {named}")),
    on: on(&root),
    ..Default::default()
  };
  let id = engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned();
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  let fork =
    verbs::Chain { label: Some("side".to_owned()), source: on(&root), ..Default::default() };
  let side = engine.chain(fork).expect("the fork is made").id().to_owned();
  counted(&mut engine, &side, 2);
  let requests = model.requests();
  let images = |at: usize| {
    parts(chat(&requests[at].chat_history)).concat().iter().filter(|one| **one == "image").count()
  };
  assert_eq!((images(0), images(1)), (1, 1), "{:?}", parts(&requests[1].chat_history));
}

#[test]
fn a_message_with_an_image_in_the_middle_goes_as_text_image_text_in_the_order_of_the_message() {
  let model = streaming(&[("close(1)", Usage::new())]);
  let (mut engine, _at, named) = imaged("images-middle", &model);
  let root = engine.root().to_owned();
  let message = Some(format!("before {named} after"));
  let with = verbs::Prompt { message, on: on(&root), ..Default::default() };
  let id = engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned();
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  let request = model.requests().remove(0);
  let [.., Message::User { content }] = chat(&request.chat_history) else { panic!("a user turn") };
  let parts: Vec<_> = content.iter().collect();
  let [.., UserContent::Text(before), UserContent::Image(_), UserContent::Text(after)] = &parts[..]
  else {
    panic!("text, image, text: {parts:?}")
  };
  assert!(before.text.ends_with(&format!("before {named}")), "{}", before.text);
  assert!(after.text.starts_with(" after"), "{}", after.text);
  let opened = |one: &&UserContent| matches!(one, UserContent::Text(text) if text.text.starts_with("#chain1\n"));
  assert!(parts.iter().any(opened), "the open of the chain stays a block of its own: {parts:?}");
}

#[test]
fn a_turn_of_a_model_with_no_usage_and_no_blocks_goes_as_its_python_in_the_role_of_the_model() {
  let model = streaming(&[("close(3)", Usage::new())]);
  let (mut engine, _) = life("crafted", vec![scripted(&model)]);
  let id = asked(&mut engine);
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  // The setup of a chain crafts the turns of the model that call its verbs, with no usage and no
  // blocks.
  let request = model.requests().remove(0);
  let crafted: Vec<_> = request
    .chat_history
    .iter()
    .filter_map(|one| match one {
      Message::Assistant { content, .. } => Some(content.clone().into_iter().collect::<Vec<_>>()),
      Message::User { .. } | Message::System { .. } => None,
    })
    .collect();
  let stands = |one: &Vec<AssistantContent>| matches!(&one[..], [AssistantContent::Text(text)] if text.text.starts_with("stand()\n"));
  assert!(crafted.iter().any(stands), "{crafted:?}");
  assert!(crafted.iter().all(|one| matches!(&one[..], [AssistantContent::Text(_)])), "{crafted:?}");
}

#[test]
fn the_open_of_a_chain_after_its_setup_goes_as_a_block_of_its_own() {
  let model = streaming(&[("close(3)", Usage::new())]);
  let (mut engine, _) = life("opened", vec![scripted(&model)]);
  let id = asked(&mut engine);
  block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  let request = model.requests().remove(0);
  let after = chat(&request.chat_history);
  let Some(Message::User { content }) = after.first() else { panic!("a user turn: {after:?}") };
  let texts: Vec<String> = content
    .iter()
    .filter_map(|one| match one {
      UserContent::Text(text) => Some(text.text.clone()),
      _ => None,
    })
    .collect();
  assert!(texts.len() > 1, "{texts:?}");
  assert!(texts.iter().any(|one| one.starts_with("#chain1\n")), "{texts:?}");
}

#[test]
fn a_stand_over_a_paused_chain_asks_no_model_until_the_wake() {
  let failing = Failing::new(vec![usage_limit(from_now(86_400))]);
  let (retries, told) = retried();
  let model = scripted(&failing);
  let at = yard("pause-stand").display().to_string();
  let mut engine = lived(Provider::new(at, vec![model]).retries(retries));
  let id = asked(&mut engine);
  let heard = Arc::clone(&told);
  assert!(until(&mut engine, move |_| heard.lock().expect("the retries told").len() == 1));
  let root = engine.root().to_owned();
  engine.pause(&root, verbs::Pause::default()).expect("the chain pauses");
  engine.stand(verbs::Stand { on: Some(root.clone()) }).expect("the life stands");
  let early = within(&mut engine, Duration::from_millis(500), |_| failing.calls() > 1);
  assert!(!early, "a stand asks no model for a paused reply");
  engine.wake(&root).expect("the chain wakes");
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(7))
  };
  assert!(until(&mut engine, done), "the wake asks the model again");
  assert_eq!(failing.calls(), 2);
}

#[test]
fn the_config_of_a_project_gives_a_model_another_class_in_the_roster() {
  let at = yard("classes");
  fs::create_dir_all(at.join(".furb")).unwrap();
  let config = r#"{"classes": {"p:m": "cheap", "p:n": "free"}}"#;
  fs::write(at.join(".furb").join("config.json"), config).unwrap();
  let models = vec![
    Model::new("p:m", 1000, MockCompletionModel::new([])),
    Model::new("p:n", 1000, MockCompletionModel::new([])),
  ];
  let mut engine = lived(Provider::new(at.display().to_string(), models));
  let standing = engine.standing().expect("the life stands").py_repr();
  // A word that names no class leaves the class of the catalog.
  assert!(
    standing.starts_with(
      "[[['p:m', [], 1000, 'cheap'], ['p:n', [], 1000, 'dear'], ['operator', [], 200000, 'cheap']]"
    ),
    "{standing}"
  );
}

/// The native search of the tests, which answers with its query.
fn found_by_native() -> Searches {
  Arc::new(|query| Box::pin(std_future::ready(Ok(format!("Found {query}.")))))
}

/// What a word that a rung runs on the root of a life closes with, or why it raised.
pub(super) fn closed(engine: &mut Engine, word: &str) -> Result<Object, String> {
  let root = engine.root().to_owned();
  let rung = verbs::Rung { word: Some(word.to_owned()), on: Some(root), ..Default::default() };
  block_on(engine.rung(rung).expect("a rung is made")).map_err(|no| no.to_string())
}

#[test]
fn a_search_is_answered_with_the_native_search_of_its_actor_as_a_text_at_the_path_of_its_query() {
  let model = streaming(&[]);
  let model = scripted(&model).searches(found_by_native());
  let mut engine = lived(Provider::new(yard("searched").display().to_string(), vec![model]));
  let got = closed(&mut engine, "close(await act('search', '', None, 'rust', 'm'))").unwrap();
  assert_eq!(Text::of(got.as_ref()).unwrap(), Text::new("rust", "Found rust."));
}

#[test]
fn a_provider_that_searches_no_more_refuses_a_search() {
  let at = opened::yard("inspected");
  let inspecting = Opening { inspecting: Some(true), ..opened::opening(&at, false) };
  let mut engine = inspecting.boot::<String>([]).expect("the life opens").0;
  let no = closed(&mut engine, "close(await act('search', '', None, 'rust', 'm'))").unwrap_err();
  assert!(no.contains("a life that inspects searches nothing"), "{no}");
}

/// A life on the claude command line of the tests, as one account whose config dir is `dir` in
/// the yard, and that yard.
#[cfg(unix)]
fn on_claude(name: &str) -> (Engine, claude::Yard) {
  use claude::Yard;
  use furb::world::claude::{Account, Source, Switch};
  let yard = Yard::new(name);
  let dir = yard.at.join("dir");
  let accounts = vec![Account { name: "a".to_owned(), source: Source::ConfigDir(dir) }];
  let claude = yard.claude_as(accounts, Switch::Failover);
  let (engine, _) = life(&format!("claude-{name}"), on_reactor(claude.models()));
  (engine, yard)
}

/// The entries of the conversation that the last process of the command line resumed.
#[cfg(unix)]
fn resumed(yard: &claude::Yard) -> Vec<Value> {
  let pids = yard.pids();
  let id = claude::after(&yard.args(pids.last().expect("a process")), "--resume")
    .expect("the last process resumes a conversation");
  let projects = fs::read_dir(yard.at.join("dir/projects")).expect("the projects of the account");
  let projects: Vec<_> = projects.flatten().collect();
  let file = projects[0].path().join(format!("{id}.jsonl"));
  let text = fs::read_to_string(file).expect("the conversation");
  text.lines().map(|one| serde_json::from_str(one).expect("an entry is json")).collect()
}

/// The answer of a prompt of the root that says this.
#[cfg(unix)]
fn answer(engine: &mut Engine, said: &str) -> Object {
  let root = engine.root().to_owned();
  let with = verbs::Prompt { message: Some(said.into()), on: on(&root), ..Default::default() };
  block_on(engine.prompt(Object::string("str"), with).expect("a prompt is made"))
    .expect("an answer")
}

/// A compaction as the skill compaction makes it: the chain jumps to a chain that keeps each of its
/// acts but those of the last exchange whose message says aside, and whose first rung tells it the
/// summary. Its turns are then no turns of the conversation before, so its next turn begins a new
/// one, which furb writes.
#[cfg(unix)]
const CUT: &str = r#"facts = [x for x in transcript(scope(acting())) if question(x)]
rungs = [at for at, x in enumerate(facts) if x[0] == 'rung']
start = max(at for at, x in enumerate(facts) if x[0] == 'prompt' and 'aside' in repr(x))
end = next((at for at in range(start + 1, rungs[-1]) if facts[at][0] == 'prompt'), rungs[-1])
jump([x[1] for x in facts[start:end]], "<s:summary>\nThe chain asked first.</s:summary>\nsummary")"#;

/// The signed thought that the command line of the tests writes for a line that holds THINK.
#[cfg(unix)]
fn signed() -> Value {
  json!({"type": "thinking", "thinking": "hmm", "signature": "sig"})
}

#[cfg(unix)]
#[test]
fn a_signed_thought_of_a_reply_after_a_compaction_reaches_the_next_conversation_whole() {
  let (mut engine, yard) = on_claude("compacted");
  answer(&mut engine, "aside");
  assert_eq!(answer(&mut engine, "THINK first").py_repr(), "'reply 2'");
  closed(&mut engine, CUT).expect("the cut runs");
  answer(&mut engine, "second");
  let entries = resumed(&yard);
  let stand =
    entries.iter().position(|one| one["message"]["content"].to_string().contains("stand()"));
  let after = &entries[stand.map_or(0, |at| at + 1)..];
  let said =
    after.iter().find(|one| one["type"] == "assistant").map(|one| &one["message"]["content"]);
  // The first turn of the model is the reply before the cut; the chain wrote the turns after it.
  assert_eq!(said, Some(&json!([signed(), {"type": "text", "text": "close(\"reply 2\")"}])));
}

#[cfg(unix)]
#[test]
fn a_signed_thought_written_after_a_compaction_reaches_the_next_turn_whole() {
  let (mut engine, yard) = on_claude("thought-after");
  answer(&mut engine, "aside");
  closed(&mut engine, CUT).expect("the cut runs");
  answer(&mut engine, "DEEP after the cut");
  answer(&mut engine, "aside again");
  // A process of the command line keeps its own turns, so a turn reads the thought from the
  // conversation that furb writes only when it begins a new one: a second compaction makes it.
  closed(&mut engine, CUT).expect("the second cut runs");
  answer(&mut engine, "third");
  let thought: String = (1..=4000).map(|at| format!("step {at} of a long thought. ")).collect();
  let signature: String = (1..=512).map(|at| format!("{:x}", at % 16)).collect();
  let deep = [
    json!({"type": "thinking", "thinking": thought, "signature": signature}),
    json!({"type": "thinking", "thinking": "", "signature": format!("omitted-{signature}")}),
    json!({"type": "redacted_thinking", "data": "opaque"}),
  ];
  let held = resumed(&yard)
    .iter()
    .filter(|one| one["type"] == "assistant")
    .filter(|one| {
      one["message"]["content"].as_array().is_some_and(|blocks| blocks.starts_with(&deep))
    })
    .count();
  assert_eq!(
    held, 1,
    "the conversation of the third turn holds the long signed thought, the one with no text \
     and the redacted one written after the first cut, whole and once"
  );
}

#[cfg(unix)]
#[test]
fn a_ceiling_on_a_chain_pauses_it_at_a_reply_that_a_named_account_served() {
  use furb::skill;
  let (mut engine, _yard) = on_claude("ceiling");
  let root = engine.root().to_owned();
  // The engine holds no grant: the rung plays the word of the official extension, then makes one.
  let kit =
    skill::official().into_iter().find(|one| one.name == "grant").expect("the extension grant");
  let word = Some(format!("{}\ngrant(usd=0.005)", kit.word));
  engine.rung(verbs::Rung { word, on: on(&root), ..Default::default() }).expect("a grant is made");
  // The answer of the prompt never comes: the ceiling pauses the chain at the first reply.
  let with = verbs::Prompt { message: Some("first".into()), on: on(&root), ..Default::default() };
  drop(engine.prompt(Object::string("str"), with).expect("a prompt is made"));
  let pauses = |engine: &mut Engine| {
    let of = verbs::Transcript { on: Some(root.clone()), of: Some("pause".to_owned()), at: None };
    engine.transcript(of).unwrap().iter().filter(|fact| fact.about() == root).count()
  };
  assert!(until(&mut engine, |engine| pauses(engine) == 1), "the ceiling paused the chain");
}

#[cfg(unix)]
#[test]
fn a_reply_of_a_named_account_has_numbers_alone_in_its_usage_and_a_fact_that_names_the_account() {
  let (mut engine, _yard) = on_claude("named");
  answer(&mut engine, "first");
  let root = engine.root().to_owned();
  let facts = |engine: &mut Engine, of: &str| {
    let of = verbs::Transcript { on: Some(root.clone()), of: Some(of.to_owned()), at: None };
    engine.transcript(of).unwrap()
  };
  let named = facts(&mut engine, "account");
  assert_eq!(named.len(), 1);
  assert_eq!(
    named[0].word(0).and_then(|one| one.as_str().map(str::to_owned)).as_deref(),
    Some("a")
  );
  let (reply, _) = taken(&mut engine);
  let turn = engine.peek(&reply, verbs::Peek::default()).unwrap();
  let usage = entry(&turn.as_ref(), 2).and_then(|one| one.items()).unwrap();
  assert_eq!(usage.len(), 5, "the usage holds the five counts");
  assert!(usage.iter().all(|one| one.as_str().is_none()), "the usage holds numbers alone");
}

#[test]
fn a_model_of_rig_reads_the_thought_of_each_earlier_turn_whole_and_one_that_forgets_reads_none() {
  // A model that forgets is a model of a local server, whose earlier turns
  // a_local_model_reads_its_earlier_turn_with_no_thought proves beside the provider.
  let thought = MockStreamEvent::reasoning("a long thought");
  let ended = || MockStreamEvent::FinalResponse(StreamFinal::new("mock", Usage::new()));
  let model = MockCompletionModel::from_stream_turns([
    vec![thought, MockStreamEvent::text("close(1)"), ended()],
    vec![MockStreamEvent::text("close(2)"), ended()],
  ]);
  let (mut engine, _) = life("thought", vec![scripted(&model)]);
  let root = engine.root().to_owned();
  counted(&mut engine, &root, 1);
  counted(&mut engine, &root, 2);
  let request = model.requests().remove(1);
  let turns: Vec<_> = chat(&request.chat_history)
    .iter()
    .filter_map(|one| match one {
      Message::Assistant { content, .. } => Some(content.clone().into_iter().collect::<Vec<_>>()),
      Message::User { .. } | Message::System { .. } => None,
    })
    .collect();
  let [AssistantContent::Reasoning(read), AssistantContent::Text(word)] = &turns[0][..] else {
    panic!("the thought, then the word: {turns:?}")
  };
  assert_eq!((read.display_text(), word.text.as_str()), ("a long thought".to_owned(), "close(1)"));
}

/// A prompt on a chain that the model answers with this value, once it is answered.
pub(super) fn counted(engine: &mut Engine, on: &str, value: i64) -> String {
  let with = verbs::Prompt {
    message: Some("count".to_owned()),
    on: Some(on.to_owned()),
    ..Default::default()
  };
  let id = engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned();
  let got = block_on(Act::<Object>::of(engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(value));
  id
}
