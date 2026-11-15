//! The laws of the provider that no door of the crate reaches, each family for its reason:
//! - the backoff of a call, which no host sets, and whose own wait is four seconds that double;
//! - the moment that a usage limit of the claude command line ends, which reads the clock and the
//!   zone of the machine;
//! - the id of a conversation, which reads the random bytes of the machine;
//! - a failed sign in, a switched account and a refresh that waits, which only the claude command
//!   line makes, from its words or as it starts, and whose fake runs on unix alone;
//! - the parts of a reply that the door shows only mixed with the others: the cleaners of a word,
//!   the end of a thought that the provider cut, and the share of thought at each effort, of which
//!   a model of a local server takes only some levels;
//! - the turns that only a record of an earlier build holds: a word that its gate refused, and
//!   marks;
//! - the order of versions, which only the search of the claude command line and the update of
//!   the catalog read, on the machine.
//!
//! The provider in a life through what the crate gives a host stands in
//! `tests/suite/world/provider.rs`, and the models that the catalog makes of the configs and of the
//! environment of a process in `tests/suite/world/catalog.rs`.

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
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use futures::FutureExt;
use rig_core::{
  completion::CompletionError,
  message::Message,
  test_utils::{MockCompletionModel, MockError, MockStreamEvent},
};
use serde_json::{Value, json};

use jiff::{Zoned, civil};

use super::{
  Hosted, Model, Provider, Retries,
  answer::{unclosed, unfenced, uuid_from},
  images,
  messages::{messages, tail},
  retry::{Backoff, Retry, Transient, passing, resets},
  thinking::{Thinking, budgeted},
};
use crate::{Act, Ear, Engine, Object, fact, verbs, world};

#[cfg(unix)]
use crate::testing::on_reactor;
use crate::testing::{Yard, block_on, park, waker};
use std::env;
use std::process;
use std::sync::mpsc;
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
fn the_end_of_a_thought_that_the_provider_cut_counts_characters() {
  assert_eq!(tail("abcdé", 2), "dé");
  assert_eq!(tail("ab", 5), "ab");
}
