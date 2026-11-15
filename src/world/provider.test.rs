//! The laws of the provider that no door of the crate reaches, each family for its reason:
//! - the backoff of a call, which no host sets, and whose own wait is four seconds that double;
//! - the moment that a usage limit of the claude command line ends, which reads the clock and the
//!   zone of the machine;
//! - the id of a conversation, which reads the random bytes of the machine;
//! - the models of the network and of a local server, with their classes, prices and thought,
//!   which the catalog makes from the environment of the process and the cache of the machine;
//! - the most thought of a turn, which reads the environment of the process;
//! - the refresh of a stand, which an `Opening` sets on the catalog of the machine;
//! - a failed sign in and a switched account, which only the claude command line types from its
//!   words, and whose fake runs on unix alone;
//! - a word that ends with a fence, whose law the other cleaners of a reply hide at the door;
//! - the turns that only a record of an earlier build holds: a word that its gate refused, and
//!   marks;
//! - the order of versions, which only the search of the claude command line and the update of
//!   the catalog read, on the machine.
//!
//! The provider in a life through what the crate gives a host stands in
//! `tests/suite/world/provider.rs`.

#![expect(
  clippy::format_collect,
  reason = "debt: a format! for each item, collected into a String"
)]
#![expect(
  clippy::needless_pass_by_value,
  reason = "debt: an argument taken by value and only borrowed"
)]
#![expect(
  clippy::redundant_closure_for_method_calls,
  reason = "debt: a closure where a method path serves"
)]
#![expect(
  clippy::unnecessary_wraps,
  reason = "debt: a Result or an Option that is always Ok or Some"
)]

use std::{
  collections::HashMap,
  fs,
  io::{BufRead, BufReader, Read, Write},
  net::TcpListener,
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use futures::FutureExt;
use rig_core::{
  completion::{CompletionError, Usage},
  message::{AssistantContent, Message},
  streaming::StreamFinal,
  test_utils::{MockCompletionModel, MockError, MockStreamEvent},
};
use serde_json::{Value, json};

use jiff::{Zoned, civil};

use super::{
  Hosted, Model, Provider, Retries, Writes,
  answer::{unclosed, unfenced, uuid_from},
  catalog::Catalog,
  images,
  local::Server,
  messages::{messages, tail},
  model::{CHEAP, Class, standing},
  retry::{Backoff, Retry, Transient, passing, resets},
  thinking::{Thinking, budgeted, thinking},
};
use crate::{Act, Ear, Engine, Fact, Object, SYSTEM, Text, fact, verbs, world};

#[cfg(unix)]
use crate::testing::on_reactor;
use crate::testing::{Yard, block_on, park, waker};
use std::env;
use std::process;
use std::sync::mpsc;
use std::time;
use tokio::task;

/// A yard of one test, empty.
fn yard(name: &str) -> Yard {
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
  let ears: [(&str, Box<dyn Ear>); 4] = [
    ("provider", provider.config(config.clone()).ear()),
    ("files", world::files(scratch.clone(), config, gallery)),
    ("bash", world::bash(scratch)),
    ("time", world::time()),
  ];
  Engine::boot(Vec::<Object>::new(), ears).expect("the life opens")
}

/// A life of the real engine on the record that a store keeps at a path, with the ears of the
/// World, as a later life opens on what an earlier one kept.
fn stored(provider: Provider, record: &Path) -> Engine {
  let (kept, store) = world::store(record).expect("the record opens");
  let scratch = world::Scratch::own();
  let config = env::temp_dir().join(format!("furb-provider-user-{}", process::id()));
  let ears: [(&str, Box<dyn Ear>); 5] = [
    ("provider", provider.config(config.clone()).ear()),
    ("files", world::files(scratch.clone(), config, None)),
    ("bash", world::bash(scratch)),
    ("time", world::time()),
    ("store", store),
  ];
  Engine::boot(kept, ears).expect("the life opens")
}

/// A life of the real engine on the provider of these models and the ears of the World, in a yard
/// of its own.
fn life(name: &str, models: Vec<Model>) -> (Engine, Yard) {
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

/// The facts of the root that the provider said about replies and chains: each kind, what it is
/// about, and the representation of its first word. A reply and a rung are named by their order
/// among those facts, as `reply1` for the first reply that the provider took, since the setup of a
/// chain crafts replies and rungs of its own before them.
fn said(engine: &mut Engine) -> Vec<(String, String, String)> {
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
fn taken(engine: &mut Engine) -> (String, String) {
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

/// The messages of a request of the network after the setup of its chain, as [`chat`] says.
fn chat_of(messages: &[Value]) -> &[Value] {
  let stands = |one: &Value| {
    one["role"] == "assistant"
      && serde_json::to_string(&one["content"]).is_ok_and(|text| text.contains("stand()"))
  };
  &messages[messages.iter().position(stands).map_or(0, |at| at + 1)..]
}

/// The engine driven until a condition of it holds, or ten seconds are up.
fn until(engine: &mut Engine, holds: impl Fn(&mut Engine) -> bool) -> bool {
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
fn a_reply_comes_to_the_turn_of_the_model_with_its_usage_its_dollars_and_its_blocks() {
  let mut usage = Usage::new();
  (usage.input_tokens, usage.output_tokens, usage.total_tokens) = (33, 5, 38);
  (usage.cached_input_tokens, usage.cache_creation_input_tokens) = (20, 3);
  let model = streaming(&[("close(3)", usage)]);
  let (mut engine, _) = life("turn", vec![scripted(&model).price(1e4, 2e4, 1e3, 5e3)]);
  let id = asked(&mut engine);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  // The price counts each token once: ten read fresh, five written, twenty read from the cache and
  // three written to it.
  let expected =
    "('assistant', 'close(3)', (33, 5, 20, 3, 0.235), [{'type': 'text', 'text': 'close(3)'}])";
  assert_eq!(said(&mut engine)[1], ("done".into(), "reply1".into(), expected.into()));
  let request = model.requests().remove(0);
  assert_eq!(request.preamble.as_deref(), Some(SYSTEM));
  assert_eq!(request.additional_params, Some(json!({"thinking": 8192})));
  let [Message::User { content }] = chat(&request.chat_history) else {
    panic!("one user turn: {:?}", request.chat_history)
  };
  assert!(format!("{content:?}").contains("count"), "the turn holds the message of the prompt");
}

#[test]
fn the_provider_wakes_a_chain_that_it_paused_after_its_backoff_and_leaves_it_paused_after_its_bound()
 {
  let failed = || vec![MockStreamEvent::Error(MockError::provider("boom"))];
  let model = MockCompletionModel::from_stream_turns((0..12).map(|_| failed()));
  let backoff = Backoff { most: 1, first: Duration::from_millis(5) };
  let provider = Provider::new(yard("woken").display().to_string(), vec![scripted(&model)]);
  let mut engine = lived(provider.backoff(backoff));
  let root = engine.root().to_owned();
  asked(&mut engine);
  let woken = super::hearer::WOKEN as usize;
  let wakes = |engine: &mut Engine| {
    said(engine).iter().filter(|(kind, about, _)| kind == "wake" && *about == root).count()
  };
  assert!(until(&mut engine, |engine| wakes(engine) == woken), "{:?}", said(&mut engine));
  let more = within(&mut engine, Duration::from_millis(300), |engine| wakes(engine) > woken);
  assert!(!more, "the provider leaves the chain paused after its bound");
  assert_eq!(model.request_count(), 2 + woken, "each wake asks the model once more");
}

#[test]
fn a_life_that_opens_on_a_record_with_a_pause_of_the_provider_wakes_the_chain_under_the_same_bound()
{
  let failed = || vec![MockStreamEvent::Error(MockError::provider("boom"))];
  let at = yard("reborn");
  let record = at.join("record.jsonl");
  let provider = |model: &MockCompletionModel, first: Duration| {
    Provider::new(at.display().to_string(), vec![scripted(model)])
      .backoff(Backoff { most: 1, first })
  };
  let before = MockCompletionModel::from_stream_turns((0..2).map(|_| failed()));
  let mut engine = stored(provider(&before, Duration::from_secs(3600)), &record);
  let root = engine.root().to_owned();
  asked(&mut engine);
  let count = |engine: &mut Engine, kind: &str| {
    said(engine).iter().filter(|(one, about, _)| *one == kind && *about == root).count()
  };
  assert!(until(&mut engine, |engine| count(engine, "pause") == 1), "{:?}", said(&mut engine));
  drop(engine);
  let after = MockCompletionModel::from_stream_turns((0..12).map(|_| failed()));
  let mut engine = stored(provider(&after, Duration::from_millis(5)), &record);
  let woken = super::hearer::WOKEN as usize;
  assert!(until(&mut engine, |engine| count(engine, "wake") == woken), "{:?}", said(&mut engine));
  let more =
    within(&mut engine, Duration::from_millis(300), |engine| count(engine, "wake") > woken);
  assert!(!more, "the provider leaves the chain paused after its bound");
}

#[cfg(unix)]
#[test]
fn a_claude_that_an_update_took_away_is_asked_again_after_its_backoff_until_it_comes_back() {
  use super::claude::{Claude, test::Yard};
  let fake = Yard::new("away");
  let claude = Claude::with_command(fake.prefix(), Vec::new(), Some(Duration::from_secs(10)));
  let (retries, told) = retried();
  let backoff = Backoff { most: 10, first: Duration::from_millis(100) };
  let provider = Provider::new(yard("away").display().to_string(), on_reactor(claude.models()))
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

/// A function of the host that fails with these reasons, one call each, and then answers
/// `close(7)`, and how many times it was called.
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

/// The same model, whose failures read as the claude command line words them: a usage limit or a
/// failed sign in in the text that a function of the host fails with is the typed failure that the
/// claude provider gives.
fn spoken(mut model: Model) -> Model {
  let inner = Arc::clone(&model.answers);
  model.answers = Arc::new(move |asked| {
    let got = inner(asked);
    Box::pin(async move { got.await.map_err(super::claude::worded) })
  });
  model
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
  let (rung, root) = (taken(&mut engine).1, engine.root().to_owned());
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
fn an_exhausted_recoverable_provider_error_gives_the_model_another_turn_without_pausing() {
  for (label, why) in [
    ("transient", "connection reset"),
    ("cut", "the turn reached the most it writes before its end"),
  ] {
    let reasons = [why, why];
    let (host, calls) = failing(&reasons);
    let model = scripted(&MockCompletionModel::default()).hosted(host);
    let backoff = Backoff { most: 1, first: Duration::from_millis(1) };
    let provider = Provider::new(yard(label).display().to_string(), vec![model]).backoff(backoff);
    let mut engine = lived(provider);
    asked(&mut engine);
    assert!(
      until(&mut engine, |engine| calls.load(Ordering::SeqCst) >= 3
        && said(engine).iter().any(|(kind, about, _)| kind == "started" && about == "reply2")),
      "{label}: {:?}",
      said(&mut engine)
    );
    let facts = said(&mut engine);
    assert!(!facts.iter().any(|(kind, _, _)| kind == "pause"), "{label}: {facts:?}");
    assert!(calls.load(Ordering::SeqCst) >= 3, "{label}: {:?}", calls.load(Ordering::SeqCst));
  }
}

#[test]
fn a_model_whose_prompt_is_always_too_long_pauses_the_chain_with_why_after_two_replies() {
  let why = "Prompt is too long · automatic compaction failed";
  let (host, calls) = failing(&[why; 50]);
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let backoff = Backoff { most: 1, first: Duration::from_millis(1) };
  let provider =
    Provider::new(yard("too-long").display().to_string(), vec![model]).backoff(backoff);
  let mut engine = lived(provider);
  let root = engine.root().to_owned();
  asked(&mut engine);
  let paused = |engine: &mut Engine| {
    said(engine).into_iter().find(|(kind, about, _)| kind == "pause" && *about == root)
  };
  assert!(until(&mut engine, |engine| paused(engine).is_some()), "{:?}", said(&mut engine));
  let (_, _, told) = paused(&mut engine).expect("the chain paused");
  assert!(told.contains(why), "the pause says why: {told}");
  let first =
    said(&mut engine).into_iter().find(|(kind, about, _)| kind == "done" && about == "reply1");
  let (_, _, refused) = first.expect("the first reply is done");
  assert!(refused.starts_with("Overflow("), "the refusal is an Overflow: {refused}");
  let bound = 2 + super::hearer::WOKEN as usize;
  assert!(until(&mut engine, |_| calls.load(Ordering::SeqCst) >= bound), "each wake asks once");
  let more =
    within(&mut engine, Duration::from_millis(300), |_| calls.load(Ordering::SeqCst) > bound);
  assert!(!more, "the paused chain asks the model no more: {}", calls.load(Ordering::SeqCst));
}

/// A provider on a port of this machine, which reads one request and streams these events. It gives
/// the request it read.
pub(super) fn served(listener: TcpListener, events: String) -> thread::JoinHandle<String> {
  thread::spawn(move || exchanged(&listener, &events))
}

/// One request that a provider on a port of this machine reads, and the events it streams in
/// answer; and the request it read.
fn exchanged(listener: &TcpListener, events: &str) -> String {
  // The provider waits for its request as long as the engine takes to send it, which a loaded
  // machine stretches: a bound of time here refused a late request, and the engine asked again for
  // ever. A test whose provider is never asked ends at the slow timeout of the runner.
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

/// A life whose root prompts the one model of a provider of the network, which the catalog makes of
/// this listing with this environment; and what the prompt came to.
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
  let (_, rung) = taken(&mut engine);
  let parts = [["", "hm"], ["close(", ""], ["3)", ""]]
    .map(|[text, thinking]| [rung.as_str(), root.as_str(), text, thinking].map(str::to_owned));
  assert_eq!(told.lock().expect("the parts told").clone(), parts);
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

/// A life whose root prompts local:qwen, the one model of a local server of the tests, which lists
/// it with a window of 1000 and streams these events for its one reply; the prompt, and the two
/// requests the server read.
fn local(name: &str, events: String) -> (Engine, String, thread::JoinHandle<[String; 2]>) {
  thinking_local(name, events, Thinking::default())
}

/// The life of [`local`], whose model thinks at most so many tokens in one turn.
fn thinking_local(
  name: &str,
  events: String,
  most: Thinking,
) -> (Engine, String, thread::JoinHandle<[String; 2]>) {
  serving(name, most, move |listener| exchanged(listener, &events))
}

/// The life of [`local`], whose server streams the reply as it likes, with what it read of the
/// request.
fn serving(
  name: &str,
  most: Thinking,
  serve: impl FnOnce(&TcpListener) -> String + Send + 'static,
) -> (Engine, String, thread::JoinHandle<[String; 2]>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let listing =
    json!({"object": "list", "data": [{"id": "qwen", "object": "model", "meta": {"n_ctx": 1000}}]});
  let asked_of =
    thread::spawn(move || [exchanged(&listener, &listing.to_string()), serve(&listener)]);
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

/// A life whose root prompts local:qwen of a local server of the tests whose most thought in a turn
/// is two parts, which lists it with a window of 1000, streams these events for the first request
/// of its one reply, and those for the request that asks it again; the prompt, and the three
/// requests the server read.
fn rethinking(first: String, again: String) -> (Engine, String, thread::JoinHandle<Vec<String>>) {
  rethinking_at("local:qwen/high", vec![first, again])
}

/// A life whose root prompts this actor of local:qwen of a local server of the tests whose most
/// thought in a turn is two parts at the effort `high` and 16 at another, which lists it with a
/// window of 1000 and streams these events for each request in turn; the prompt, and the requests
/// the server read, the listing first.
fn rethinking_at(
  actor: &str,
  answers: Vec<String>,
) -> (Engine, String, thread::JoinHandle<Vec<String>>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let listing =
    json!({"object": "list", "data": [{"id": "qwen", "object": "model", "meta": {"n_ctx": 1000}}]});
  let asked_of = thread::spawn(move || {
    [vec![listing.to_string()], answers]
      .concat()
      .iter()
      .map(|one| exchanged(&listener, one))
      .collect()
  });
  let address = format!("http://127.0.0.1:{port}");
  let env = |name: &str| (name == "FURB_LOCAL_URL").then(|| address.clone());
  let catalog =
    Catalog::of(&super::catalog::parsed("{}", None), None, Server::named(env).map(Arc::new), env);
  let most = if actor.ends_with("/high") { 2 } else { 16 };
  let (models, actor) = catalog.roster(None, Some(actor)).expect("the model is offered");
  let provider = Provider::new(yard("rethought").display().to_string(), models).actor(actor);
  let mut engine = lived(provider.thinking(Thinking { most, inherited: false }));
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
    "{'type': 'reasoning', 'id': 'cut', 'content': [{'type': 'text', 'content': {'text': 'abc'}}]}";
  let expected = format!(
    "('assistant', 'close(3)', (100, 8, 0, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
  );
  assert_eq!(
    turn, expected,
    "the turn holds the thought, counts its parts, and loses its one fence"
  );
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let got = asked_of.join().expect("the server read the requests");
  let (first, second) = (&got[1], &got[2]);
  let body = |request: &str| -> Value {
    serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body")
  };
  let (first, second) = (body(first), body(second));
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
    chunk(json!({"content": "x = 1\ndebug(t'{x}')"}), json!("stop")),
    ended.clone(),
  ];
  let again = [chunk(json!({"role": "assistant", "content": "close(x)"}), json!("stop")), ended];
  let (mut engine, id, asked_of) = rethinking(first.concat(), again.concat());
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(1));
  let got = asked_of.join().expect("the server read the requests");
  let second = &got[2];
  let body: Value =
    serde_json::from_str(second.split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body");
  let earlier = body["messages"]
    .as_array()
    .and_then(|messages| chat_of(messages).iter().find(|one| one["role"] == "assistant"))
    .cloned()
    .unwrap_or_default();
  assert_eq!(
    (&earlier["content"], earlier.get("reasoning_content")),
    (&json!([{"type": "text", "text": "x = 1\ndebug(t'{x}')"}]), None),
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
    "{'type': 'reasoning', 'id': 'cut', 'content': [{'type': 'text', 'content': {'text': 'ab'}}]}";
  let expected = format!(
    "('assistant', 'close(3)', (100, 7, 0, 0, 0.0), [{reasoning}, {{'type': 'text', 'text': 'close(3)'}}])"
  );
  assert_eq!(turn, expected, "the turn holds the thought and the word, and counts what both wrote");
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let got = asked_of.join().expect("the server read the requests");
  let second = &got[2];
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
fn a_turn_of_a_model_loses_its_marks_and_a_turn_of_marks_alone_goes_with_the_user_turns_around_it_joined()
 {
  let turn = |role: &str, text: &str| {
    Object::tuple([Object::string(role), Object::string(text), Object::none(), Object::none()])
  };
  let call = "<invoke name=\"python\">\n<parameter name=\"code\">close(1)</parameter>\n</invoke>";
  let turns = Object::list([
    turn("user", "ask"),
    turn("assistant", "```python"),
    turn("user", "#rung2 refused"),
    turn("assistant", call),
    turn("user", "next"),
  ]);
  let model = scripted(&MockCompletionModel::new([]));
  let got = messages(turns.as_ref(), &model, &mut images::Images::new(None)).unwrap();
  let said: Vec<Value> = got
    .iter()
    .map(|one| match one {
      Message::User { content } => json!(["user", format!("{content:?}")]),
      Message::Assistant { content, .. } => json!(["assistant", format!("{content:?}")]),
      Message::System { .. } => json!(["system"]),
    })
    .collect();
  assert_eq!(said.len(), 3, "{said:?}");
  let text = |at: usize| said[at][1].as_str().unwrap_or_default().to_owned();
  assert!(text(0).contains("ask\\n\\n#rung2 refused"), "{said:?}");
  assert!(
    text(1).contains("close(1)") && !text(1).contains("invoke") && !text(1).contains("parameter"),
    "{said:?}"
  );
  assert_eq!(said[2][0], "user");
  assert!(text(2).contains("next"), "{said:?}");
}

#[test]
fn a_line_of_three_backticks_alone_at_the_end_goes_unless_a_fence_or_a_quote_opens_before_it() {
  assert_eq!(unclosed("close(3)\n```").as_deref(), Some("close(3)"));
  assert_eq!(unclosed("x = 1\nclose(x)\n  ```  \n\n").as_deref(), Some("x = 1\nclose(x)"));
  assert_eq!(
    unclosed("```python\nclose(3)\n```"),
    None,
    "a fence that opens the word keeps its close"
  );
  assert_eq!(unclosed("<s:doc>\nrun:\n```"), None, "the text of an open quote keeps its fence");
  assert_eq!(
    unclosed("<s:doc>\n```sh\nls\n```\n</s:doc>\nclose(doc)\n```").as_deref(),
    Some("<s:doc>\n```sh\nls\n```\n</s:doc>\nclose(doc)"),
    "the fences of a closed quote stay"
  );
  assert_eq!(unclosed("close(3)\n```\nDone."), None, "a fence before prose stays");
  assert_eq!(unclosed("close(3)\n```python"), None);
  assert_eq!(unclosed("close(3)"), None);
  assert_eq!(unclosed("```"), None);
  assert_eq!(unclosed("\n```"), None);
}

#[test]
fn a_call_that_a_failed_sign_in_refuses_waits_for_a_stand_and_the_chain_is_told_once() {
  let (host, calls) = failing(&["Invalid API key \u{b7} Please run /login"]);
  let (retries, told) = retried();
  let model = spoken(scripted(&MockCompletionModel::default()).hosted(host));
  let provider =
    Provider::new(yard("signed-out").display().to_string(), vec![model]).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let heard = Arc::clone(&told);
  assert!(until(&mut engine, move |_| heard.lock().expect("the retries told").len() == 1));
  let first = told.lock().expect("the retries told")[0].2.clone();
  assert_eq!(
    (first.why.as_str(), first.wait),
    ("signed out, sign in again", Duration::from_secs(60))
  );
  engine.stand(verbs::Stand { on: Some("chain1".to_owned()) }).expect("the life stands");
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(7))
  };
  assert!(until(&mut engine, done), "the reply waits no more for the sign in");
  assert_eq!(calls.load(Ordering::SeqCst), 2);
  let root = engine.root().to_owned();
  let notes: Vec<_> = said(&mut engine)
    .into_iter()
    .filter(|(kind, _, note)| kind == "tell" && note.contains("waits on"))
    .collect();
  assert_eq!(notes.len(), 1);
  assert!(notes[0].2.contains(&format!("#{root} waits on m/high: signed out, sign in again")));
}

#[test]
fn a_stand_on_new_settings_asks_a_reply_that_waits_for_a_usage_limit_again_on_the_new_command_line()
{
  let later = time::SystemTime::now()
    .duration_since(time::UNIX_EPOCH)
    .expect("a time after the epoch")
    .as_secs()
    + 86_400;
  let limit = format!("Claude AI usage limit reached|{later}");
  let (old, before) = failing(&[&limit, &limit, &limit]);
  let (new, after) = failing(&[]);
  let fresh = scripted(&MockCompletionModel::default()).hosted(new);
  let (retries, told) = retried();
  let model = spoken(scripted(&MockCompletionModel::default()).hosted(old));
  let stale = model.clone();
  let switched = Arc::new(AtomicBool::new(false));
  let flipped = Arc::clone(&switched);
  let provider = Provider::new(yard("limit-switch").display().to_string(), vec![model])
    .retries(retries)
    .refreshing(Arc::new(move || {
      let models = if flipped.load(Ordering::SeqCst) { &fresh } else { &stale };
      let got = Ok((vec![models.clone()], None, Thinking::default()));
      Box::pin(async move { got })
    }));
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let heard = Arc::clone(&told);
  assert!(until(&mut engine, move |_| !heard.lock().expect("the retries told").is_empty()));
  switched.store(true, Ordering::SeqCst);
  engine.stand(verbs::Stand { on: Some("chain1".to_owned()) }).expect("the life stands");
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(7))
  };
  assert!(until(&mut engine, done), "the reply asks the new command line");
  assert_eq!((before.load(Ordering::SeqCst), after.load(Ordering::SeqCst)), (1, 1));
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
fn a_usage_limit_reads_its_clock_and_its_month_as_jiff_reads_them() {
  let said = |time: &str| format!("You've hit your limit \u{b7} resets {time} (Europe/Paris)");
  let now = paris(10, 23, 16, 19);
  // Noon and midnight, with the minutes or with none, in any case.
  assert_eq!(resets(&said("12pm"), &now), Some(paris(10, 24, 12, 0)));
  assert_eq!(resets(&said("12PM"), &paris(10, 23, 11, 0)), Some(paris(10, 23, 12, 0)));
  assert_eq!(resets(&said("12:30am"), &now), Some(paris(10, 24, 0, 30)));
  assert_eq!(resets(&said("3pm"), &now), Some(paris(10, 24, 15, 0)));
  assert_eq!(resets(&said("3:45Pm"), &now), Some(paris(10, 24, 15, 45)));
  // A month whole, or short, with a day of one digit or of two.
  assert_eq!(resets(&said("October 28, 3pm"), &now), Some(paris(10, 28, 15, 0)));
  assert_eq!(resets(&said("Nov 2 at 9am"), &now), Some(paris(11, 2, 9, 0)));
  // A clock or a date that is none.
  for none in
    ["13pm", "0am", "5:61pm", "5:10", "Feb 30, 3pm", "Oct 32, 3pm", "Foo 3, 3pm", "Oct, 3pm"]
  {
    assert_eq!(resets(&said(none), &now), None, "{none}");
  }
}

#[test]
fn a_usage_limit_at_a_time_that_a_change_of_the_clock_skips_or_repeats_ends_at_the_day_that_it_names()
 {
  let said = "You've hit your limit \u{b7} resets 2:30am (Europe/Paris)";
  let moment = |month, day, hour, minute| {
    civil::date(2026, month, day).at(hour, minute, 0, 0).in_tz("Europe/Paris").expect("a moment")
  };
  // The clock of Paris goes from 2:00 to 3:00 on 29 March, so 2:30 stands at 3:30.
  let spring = resets(said, &paris(3, 28, 23, 0)).expect("a moment after now");
  assert_eq!(spring, moment(3, 29, 2, 30));
  assert_eq!(spring.hour(), 3);
  // It goes back from 3:00 to 2:00 on 25 October, so 2:30 stands twice, and the first one is taken.
  let autumn = resets(said, &paris(10, 24, 23, 0)).expect("a moment after now");
  assert_eq!(autumn, moment(10, 25, 2, 30));
  assert_eq!(autumn.offset().seconds(), 2 * 3600);
  // A time with no day, said after the change, is the next day's.
  let five = "You've hit your limit \u{b7} resets 5pm (Europe/Paris)";
  assert_eq!(resets(five, &paris(10, 25, 18, 0)), Some(paris(10, 26, 17, 0)));
}

#[test]
fn a_usage_limit_whose_message_names_no_moment_has_none() {
  let now = paris(10, 23, 16, 19);
  assert_eq!(resets("You've hit your limit \u{b7} resets soon", &now), None);
  assert_eq!(resets("You've hit your limit \u{b7} resets 5pm (Mars/Base)", &now), None);
  assert_eq!(resets("You've hit your limit \u{b7} resets Mon 5pm", &now), None);
}

#[test]
fn a_switched_account_is_a_failure_that_passes_in_its_own_words_and_no_wait_for_a_limit() {
  let why = super::claude::Error::Switched("from perso to team: its usage limit".to_owned());
  let no = CompletionError::RequestError(Box::new(why));
  let said = "Switched account from perso to team: its usage limit";
  assert_eq!(passing(&no), Some(Transient::Switched(said.to_owned())));
  assert!(!matches!(super::claude::error(&no), Some(super::claude::Error::Limit { .. })));
}

#[test]
fn a_turn_of_the_model_that_the_gate_refused_goes_on_the_user_side_quoted_under_its_finding() {
  let turn = |role: &str, text: &str| {
    Object::tuple([Object::string(role), Object::string(text), Object::none(), Object::none()])
  };
  let finding = "#rung2 refused\nrung2_findings = 'line 1: BAD is not defined'";
  let turns = Object::list([
    turn("user", "ask"),
    turn("assistant", "x = BAD"),
    turn("user", &format!("{finding}\n\n#rung3 advance on prompt1")),
    turn("assistant", "close(1)"),
  ]);
  let model = scripted(&MockCompletionModel::new([]));
  let got = messages(turns.as_ref(), &model, &mut images::Images::new(None)).unwrap();
  let said: Vec<Value> = got
    .iter()
    .map(|one| match one {
      Message::User { content } => json!(["user", format!("{content:?}")]),
      Message::Assistant { content, .. } => json!(["assistant", format!("{content:?}")]),
      Message::System { .. } => json!(["system"]),
    })
    .collect();
  assert_eq!(said.len(), 2, "the refused word is no turn of the model: {said:?}");
  let text = said[0][1].as_str().unwrap_or_default();
  let quoted = "ask\\n\\n#rung2 refused\\nrung2_findings = 'line 1: BAD is not defined'\\n<s:rung2_word>\\nx = BAD\\n</s:rung2_word>\\n\\n#rung3 advance";
  assert!(text.contains(quoted), "{said:?}");
  assert_eq!(said[1][0], "assistant");
}

#[test]
fn the_local_config_of_the_project_gives_the_most_thought_over_its_config() {
  let at = yard("thinking-local");
  fs::create_dir_all(at.join(".furb")).expect("the folder of the config");
  fs::write(at.join(".furb").join("config.json"), r#"{"max_thinking_tokens": 0}"#)
    .expect("the config");
  let local = at.join(".furb").join("config.local.json");
  fs::write(&local, r#"{"max_thinking_tokens": 2048}"#).expect("the local config");
  assert_eq!(
    thinking(None, &at),
    Thinking { most: 2048, inherited: false },
    "the local config wins"
  );
  fs::write(&local, r#"{"max_thinking_tokens": "much"}"#).expect("the local config");
  assert_eq!(
    thinking(None, &at),
    Thinking { most: 0, inherited: false },
    "a local value that is no count is as unsaid"
  );
}

#[test]
fn a_pause_over_the_chain_holds_the_next_call_of_a_reply_that_waits_for_a_retry_until_the_wake() {
  let (host, calls) = failing(&["Connection reset by peer (os error 54)"]);
  let (retries, told) = retried();
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let backoff = Backoff { most: 10, first: Duration::from_millis(200) };
  let at = yard("pause-retry").display().to_string();
  let provider = Provider::new(at, vec![model]).backoff(backoff).retries(retries);
  let mut engine = lived(provider);
  let id = asked(&mut engine);
  let heard = Arc::clone(&told);
  assert!(until(&mut engine, move |_| heard.lock().expect("the retries told").len() == 1));
  let root = engine.root().to_owned();
  engine.pause(&root, verbs::Pause::default()).expect("the chain pauses");
  let early = within(&mut engine, Duration::from_millis(800), |_| calls.load(Ordering::SeqCst) > 1);
  assert!(!early, "a paused reply asks its model no more");
  engine.wake(&root).expect("the chain wakes");
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(7))
  };
  assert!(until(&mut engine, done), "the wake lets the call go on");
  assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn a_model_whose_thought_the_provider_cuts_thinks_an_eighth_of_the_most_at_low_and_a_quarter_at_medium()
 {
  assert_eq!(budgeted(32_768, Some("low")), 4096);
  assert_eq!(budgeted(32_768, Some("minimal")), 4096);
  assert_eq!(budgeted(32_768, Some("medium")), 8192);
  assert_eq!(budgeted(32_768, Some("high")), 32_768);
  assert_eq!(budgeted(32_768, Some("max")), 32_768);
  assert_eq!(budgeted(32_768, None), 32_768, "a model with no effort thinks to the most");
  assert_eq!(budgeted(2, Some("low")), 1, "a share is one token at least");
}

#[test]
fn a_model_at_medium_is_cut_at_a_quarter_of_the_most_thought() {
  let thought =
    ["a", "b", "c", "d", "e", "f"].map(|one| chunk(json!({"reasoning_content": one}), Value::Null));
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105}});
  let again = [
    chunk(json!({"role": "assistant", "content": "close(3)"}), json!("stop")),
    format!("data: {usage}\n\ndata: [DONE]\n\n"),
  ];
  let (mut engine, id, asked_of) =
    rethinking_at("local:qwen/medium", vec![thought.concat(), again.concat()]);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let got = asked_of.join().expect("the server read the requests");
  let second: Value =
    serde_json::from_str(got[2].split("\r\n\r\n").nth(1).unwrap_or_default()).expect("a JSON body");
  let last =
    second["messages"].as_array().and_then(|messages| messages.last()).cloned().unwrap_or_default();
  assert_eq!(
    (&last["reasoning_content"], &second["chat_template_kwargs"]),
    (&json!("abcde"), &json!({"enable_thinking": false})),
    "a most of 16 at medium cuts the thought after its fifth part"
  );
}

#[test]
fn a_local_model_reads_the_end_of_a_thought_that_the_provider_cut_the_same_in_each_later_turn() {
  let usage = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "qwen", "choices": [],
    "usage": {"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105}});
  let ended = format!("data: {usage}\n\ndata: [DONE]\n\n");
  let word = |text: &str| {
    [chunk(json!({"role": "assistant", "content": text}), json!("stop")), ended.clone()].concat()
  };
  let thought =
    ["a", "b", "c", "d"].map(|one| chunk(json!({"reasoning_content": one}), Value::Null));
  let answers = vec![
    thought.concat(),
    word("x = 1\ndebug(t'{x}')"),
    word("y = 2\ndebug(t'{y}')"),
    word("close(x + y)"),
  ];
  let (mut engine, id, asked_of) = rethinking_at("local:qwen/high", answers);
  let got = block_on(Act::<Object>::of(&mut engine, &id)).expect("the prompt is answered");
  assert_eq!(got.as_ref().as_int(), Some(3));
  let got = asked_of.join().expect("the server read the requests");
  let thoughts = |request: &str| -> Vec<Value> {
    let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default())
      .expect("a JSON body");
    let messages = body["messages"].as_array().cloned().unwrap_or_default();
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
  assert_eq!(tail("abcdé", 2), "dé", "the end of a thought counts characters");
  assert_eq!(tail("ab", 5), "ab");
}

#[test]
fn a_stand_that_reads_the_settings_slowly_holds_no_chain_while_it_reads() {
  let (answer, login) = mpsc::channel::<()>();
  let login = Arc::new(Mutex::new(login));
  let (host, _) = failing(&[]);
  let model = scripted(&MockCompletionModel::default()).hosted(host);
  let fresh = model.clone();
  let provider = Provider::new(yard("slow-stand").display().to_string(), vec![model]).refreshing(
    Arc::new(move || {
      let (login, fresh) = (Arc::clone(&login), fresh.clone());
      Box::pin(async move {
        // A login of a command line that answers only once the test lets it.
        let waited = task::spawn_blocking(move || {
          login.lock().expect("the login").recv().expect("the login answers");
        });
        waited.await.expect("the login is waited for");
        Ok((vec![fresh], None, Thinking::default()))
      })
    }),
  );
  let mut engine = lived(provider);
  let root = engine.root().to_owned();
  let stand = engine.stand(verbs::Stand { on: Some(root) }).expect("the life stands");
  let stand = stand.id().to_owned();
  let other = verbs::Chain { label: Some("other".to_owned()), ..Default::default() };
  let other = engine.chain(other).expect("a chain is made").id().to_owned();
  let with =
    verbs::Prompt { message: Some("count".to_owned()), on: on(&other), ..Default::default() };
  let id = engine.prompt(Object::string("int"), with).expect("a prompt is made").id().to_owned();
  let done = |engine: &mut Engine| {
    engine.peek(&id, verbs::Peek::default()).is_ok_and(|one| one.as_ref().as_int() == Some(7))
  };
  assert!(until(&mut engine, done), "the other chain goes on while the stand reads");
  let stood = move |engine: &mut Engine| {
    engine
      .peek(&stand, verbs::Peek::default())
      .is_ok_and(|one| one.as_ref().type_name() != "NoneType")
  };
  assert!(!stood(&mut engine), "the stand still reads the settings");
  answer.send(()).expect("the login is waited for");
  assert!(until(&mut engine, stood), "the stand is done once the login answers");
}

#[test]
fn a_model_takes_the_class_of_the_catalog_else_the_one_of_its_price_else_dear() {
  let catalog = Catalog::of(&indexmap::IndexMap::new(), None, None, |_| None);
  let class = |name: &str| catalog.find(name).map(|one| one.class());
  assert_eq!(class("claude-cli:haiku"), Some(Class::Cheap));
  assert_eq!(class("claude-cli:opus"), Some(Class::Dear));
  assert_eq!(super::catalog::family("codex", "gpt-5-mini"), Some(Class::Cheap));
  assert_eq!(super::catalog::family("codex", "gpt-5"), Some(Class::Dear));
  assert_eq!(super::catalog::family("codex", "ghost"), None);
  let model = || Model::new("p:m", 1000, MockCompletionModel::new([]));
  let priced = |input: f64| model().price(input, 1.0, 0.0, 0.0).class();
  assert_eq!([priced(0.25), priced(CHEAP), priced(3.0)], [Class::Cheap, Class::Cheap, Class::Dear]);
  assert_eq!(model().class(), Class::Dear);
  assert_eq!(model().price(0.1, 0.1, 0.0, 0.0).classed(Class::Dear).class(), Class::Dear);
}

#[test]
fn the_configs_of_the_user_and_of_the_project_give_the_classes_and_the_list_of_automatic_work() {
  let at = yard("configured");
  let (user, project) = (at.join("user"), at.join("project"));
  fs::create_dir_all(project.join(".furb")).unwrap();
  fs::create_dir_all(&user).unwrap();
  let mine = r#"{"classes": {"claude-cli:haiku": "dear", "claude-cli:opus": "cheap"},
    "automatic": ["claude-cli:sonnet", "ghost", "claude-cli:opus"]}"#;
  fs::write(user.join("config.json"), mine).unwrap();
  fs::write(
    project.join(".furb").join("config.json"),
    r#"{"classes": {"claude-cli:opus": "dear"}}"#,
  )
  .unwrap();
  let catalog = Catalog::of(&indexmap::IndexMap::new(), None, None, |_| None);
  let models: Vec<Model> =
    ["claude-cli:haiku", "claude-cli:opus", "claude-cli:sonnet", "claude-cli:fable"]
      .iter()
      .map(|name| catalog.find(name).expect("the catalog holds the claude command line"))
      .collect();
  let roster = |directory: &Path| {
    let given = standing(&models, &directory.display().to_string(), "a", None, &[], &user);
    let names = given.py_repr();
    names
      .split("], ['")
      .map(|one| one.rsplit("], ").next().unwrap_or_default().to_owned())
      .collect::<Vec<_>>()
  };
  // The catalog gives haiku the class cheap and the others dear, the user turns haiku dear and opus
  // cheap, and the project turns opus dear again; the list of the user ranks sonnet first and opus
  // after it.
  assert_eq!(
    roster(&project)[..4],
    ["200000, 'dear'", "1000000, 'dear', 2", "1000000, 'dear', 0", "1000000, 'dear'"]
      .map(str::to_owned)
  );
  assert_eq!(
    roster(&at)[..4],
    ["200000, 'dear'", "1000000, 'cheap', 2", "1000000, 'dear', 0", "1000000, 'dear'"]
      .map(str::to_owned)
  );
  fs::remove_dir_all(&user).unwrap();
  assert_eq!(roster(&at)[..2], ["200000, 'cheap'", "1000000, 'dear'"].map(str::to_owned));
}

#[test]
fn a_machine_with_no_random_bytes_fails_the_call_that_needs_an_id() {
  let got = uuid_from(|_| Err(getrandom::Error::UNSUPPORTED));
  let Err(CompletionError::ProviderError(why)) = got else {
    panic!("a machine with no random bytes gave an id")
  };
  assert!(why.starts_with("the machine gave no random bytes"), "{why}");
}

#[test]
fn an_id_is_a_uuid_of_version_4_and_variant_1() {
  let zeros = uuid_from(|bytes| {
    bytes.fill(0);
    Ok(())
  });
  assert_eq!(zeros.unwrap(), "00000000-0000-4000-8000-000000000000");
  let ones = uuid_from(|bytes| {
    bytes.fill(0xff);
    Ok(())
  });
  assert_eq!(ones.unwrap(), "ffffffff-ffff-4fff-bfff-ffffffffffff");
}

#[test]
fn a_version_is_compared_by_its_numbers() {
  assert!(super::numbers("1.10.0") > super::numbers("1.4.0"));
  assert_eq!(super::numbers("1.4.0"), vec![1, 4, 0]);
  assert_eq!(super::numbers("claude-2.1.119"), vec![2, 1, 119]);
}

#[test]
fn a_refresh_that_reads_no_settings_refuses_the_stand_with_its_cause() {
  let model = scripted(&MockCompletionModel::default());
  let provider = Provider::new(yard("unrefreshed").display().to_string(), vec![model]).refreshing(
    Arc::new(|| {
      let no = super::catalog::Unknown::Name { name: "gone".to_owned() };
      Box::pin(async move { Err(super::Unrefreshed::Roster(no)) })
    }),
  );
  let mut engine = lived(provider);
  let root = engine.root().to_owned();
  let stand = engine.stand(verbs::Stand { on: Some(root) }).expect("the life stands");
  let stand = stand.id().to_owned();
  let refused = |engine: &mut Engine| {
    engine.peek(&stand, verbs::Peek::default()).is_ok_and(|one| {
      one.py_repr() == "Refused(args=('No model is gone. Name one as provider:id.'))"
    })
  };
  assert!(until(&mut engine, refused), "the stand says why the refresh read nothing");
}

/// A provider whose roster grows by a second model once the flag is set, as when a stand reads new
/// settings.
fn widening(model: &MockCompletionModel, wide: &Arc<AtomicBool>) -> Provider {
  let (first, wider) =
    (scripted(model), vec![scripted(model), Model::new("n", 1000, model.clone())]);
  let flipped = Arc::clone(wide);
  Provider::new(yard("prefix").display().to_string(), vec![first.clone()]).refreshing(Arc::new(
    move || {
      let models = if flipped.load(Ordering::SeqCst) { wider.clone() } else { vec![first.clone()] };
      Box::pin(async move { Ok((models, None, Thinking::default())) })
    },
  ))
}

/// A prompt on a chain that the model answers with this value, once it is answered.
fn counted(engine: &mut Engine, on: &str, value: i64) -> String {
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

/// The messages of each request that a model read, as JSON.
fn histories(model: &MockCompletionModel) -> Vec<Vec<Value>> {
  let json = |one: &Message| serde_json::to_value(one).expect("a message is json");
  model.requests().iter().map(|one| one.chat_history.iter().map(json).collect()).collect()
}

#[test]
fn each_request_of_a_chain_starts_with_the_request_before_it_across_a_stand_an_edit_and_a_fork() {
  let words = ["x = 1\ndebug(t'{x}')", "close(1)", "close(2)", "close(3)", "close(4)", "close(5)"];
  let model = streaming(&words.map(|one| (one, Usage::new())));
  let wide = Arc::new(AtomicBool::new(false));
  let mut engine = lived(widening(&model, &wide));
  let root = engine.root().to_owned();
  let first = counted(&mut engine, &root, 1);
  wide.store(true, Ordering::SeqCst);
  let stand =
    engine.stand(verbs::Stand { on: Some(root.clone()) }).expect("the life stands").id().to_owned();
  block_on(Act::<Object>::of(&mut engine, &stand)).expect("the stand is answered");
  counted(&mut engine, &root, 2);
  let door = Text::new(first, "x = 4\ndebug(t'{x}')");
  engine.write(&door, verbs::Write { on: Some(root.clone()) }).expect("the door is written");
  counted(&mut engine, &root, 3);
  let fork = verbs::Chain {
    label: Some("side".to_owned()),
    source: Some(root.clone()),
    ..Default::default()
  };
  let side = engine.chain(fork).expect("the fork is made").id().to_owned();
  counted(&mut engine, &side, 4);
  counted(&mut engine, &root, 5);
  let requests = histories(&model);
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
