//! A life that its thread opens again from its checkpoint: past half of the memory bound of its
//! sandbox, past that bound, and past the tail of facts that a checkpoint bounds. The memory of the
//! sandboxes is a count of the whole process, so this test runs in a process of its own, with no
//! harness, and sets the bound before it starts a thread.

#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that it starts for itself"
)]
#![expect(clippy::expect_used, reason = "a test with no harness fails by a panic")]

use std::{
  fs,
  path::{Path, PathBuf},
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use furb::{
  Object, checkpoint,
  life::{Answer, Opening},
  world,
};
use furb_host::{Command, Event, Life, LifeHandle, Reply, Seq, view::Phase};
use futures::{Stream, StreamExt};
use serde_json::Value;

/// The events of a life, each under its place in the log.
type Logged = (Seq, Arc<Event>);

/// The most facts that a life says after its checkpoint before it writes the next one.
const TAIL: usize = 10_000;

fn main() {
  // SAFETY: no other thread runs yet, so none reads the environment while it changes.
  unsafe { std::env::set_var("FURB_SANDBOX_MEMORY", "256") };
  let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
  runtime.expect("the test has a runtime").block_on(a_life_goes_on_through_each_rebirth());
}

/// A directory of the test, empty.
fn place() -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-host-rebirth-{}", std::process::id()));
  fs::remove_dir_all(&at).ok();
  fs::create_dir_all(&at).expect("the test has a directory");
  at
}

/// A life that keeps its record, with no extension, whose model closes each request with None.
fn opening(at: &Path) -> Opening {
  let asked = AtomicUsize::new(0);
  let answer = Answer(Arc::new(move |_, _| {
    asked.fetch_add(1, Ordering::SeqCst);
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string("close(None)"),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  Opening {
    directory: at.display().to_string(),
    record: Some(at.join("record.jsonl").display().to_string()),
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()]),
    extensions: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}

/// The events of a stream, up to the first that the test waits for. A bound of time fails a test
/// that hangs.
async fn until(
  events: &mut (impl Stream<Item = Logged> + Unpin),
  wanted: impl Fn(&Event) -> bool,
) -> Vec<Logged> {
  let mut got = Vec::new();
  loop {
    let next = tokio::time::timeout(Duration::from_secs(120), events.next()).await;
    let Ok(Some(logged)) = next else {
      let last: Vec<String> =
        got.iter().rev().take(20).map(|one| format!("{:.300}", format!("{one:?}"))).collect();
      panic!("the life told no event in time, after {}", last.join("\n"))
    };
    let found = wanted(&logged.1);
    got.push(logged);
    if found {
      return got;
    }
  }
}

/// A word that runs on the root as a turn of the operator.
async fn ran(life: &LifeHandle, word: &str) {
  let made = life.send(Command::Rung { word: word.to_owned(), on: None }).await;
  assert!(matches!(made, Ok(Reply::Act(_))), "{word} ran: {made:?}");
}

/// The representation of a name of the module of the root, or why it has none.
async fn named(life: &LifeHandle, name: &str) -> Result<String, String> {
  match life.send(Command::Inspect { name: name.to_owned(), on: None }).await {
    Ok(Reply::Inspected { repr, .. }) => Ok(repr),
    other => Err(format!("{other:?}")),
  }
}

/// The text of each turn of a chain.
async fn turns(life: &LifeHandle, on: &str) -> String {
  match life.send(Command::Turns { on: Some(on.to_owned()) }).await {
    Ok(Reply::Values(turns)) => Value::from(turns).to_string(),
    other => panic!("the chain gave no turns: {other:?}"),
  }
}

/// Whether an event is a rebirth after a fault, or after none, as the test wants it.
fn reborn(fault: bool) -> impl Fn(&Event) -> bool {
  move |event| matches!(event, Event::Reborn(why) if why.is_some() == fault)
}

/// The facts that a log tells twice.
fn twice(log: &[Logged]) -> Vec<String> {
  let mut seen = std::collections::HashSet::new();
  let facts = log.iter().filter_map(|one| match &*one.1 {
    Event::Fact(fact) => Some(fact.to_string()),
    _ => None,
  });
  facts.filter(|fact| !seen.insert(fact.clone())).collect()
}

async fn a_life_goes_on_through_each_rebirth() {
  let at = place();
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  ran(&life, "n = 1\nwork = chain('work')").await;
  let work = named(&life, "work").await.expect("the root binds work");
  let work = work.trim_matches('\'').to_owned();
  let asked = Command::Prompt {
    shape: "str".to_owned(),
    message: "Name a color.".to_owned(),
    to: Some("operator".to_owned()),
    on: Some(work.clone()),
  };
  let Ok(Reply::Act(asked)) = life.send(asked).await else { panic!("the prompt was not made") };
  let mut log =
    until(&mut events, |event| matches!(event, Event::Act(act) if act.id == asked)).await;

  ran(&life, "big = b'.' * (150 << 20)").await;
  log.extend(until(&mut events, reborn(false)).await);
  assert!(furb::sandbox_held() < furb::sandbox_bound() / 2, "the rebirth let go of the memory");
  assert_eq!(named(&life, "n").await, Ok("1".to_owned()), "the module stands");
  assert!(named(&life, "big").await.is_err(), "a value that a checkpoint cannot hold is lost");

  ran(&life, "m = 2").await;
  let cut = life.send(Command::Rung { word: "boom = 'a' * (1 << 30)".to_owned(), on: None }).await;
  assert!(format!("{cut:?}").contains("MemoryError"), "the fault cuts the command: {cut:?}");
  log.extend(until(&mut events, reborn(true)).await);
  let fault = log.iter().find_map(|one| match &*one.1 {
    Event::Reborn(Some(fault)) => Some(fault.clone()),
    _ => None,
  });
  assert!(fault.is_some_and(|one| one.contains("MemoryError")), "the rebirth names the fault");
  assert_eq!(named(&life, "n").await, Ok("1".to_owned()), "the module of the checkpoint stands");
  assert!(named(&life, "boom").await.is_err(), "the word that the fault cut runs no more");
  assert!(turns(&life, "chain1").await.contains("m = 2"), "the root keeps the turns of its tail");

  let answer = Command::Answer { id: asked.clone(), line: "red".to_owned() };
  let answered = life.send(answer).await;
  assert!(matches!(answered, Ok(Reply::Done)), "the prompt stayed open: {answered:?}");
  log.extend(until(&mut events, |event| {
    matches!(event, Event::Act(act) if act.id == asked && act.phase == Phase::Done("red".into()))
  }).await);
  ran(&life, "for i in range(2600):\n  chance()").await;
  log.extend(until(&mut events, reborn(false)).await);
  assert_eq!(named(&life, "n").await, Ok("1".to_owned()), "the life goes on past its tail");

  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  log.extend(until(&mut events, |event| matches!(event, Event::Ended(None))).await);
  let late: Vec<Logged> = life.events(0).collect().await;
  assert_eq!(late, log, "a reader that comes late reads the same log");
  let rebirths = log.iter().filter(|one| matches!(*one.1, Event::Reborn(_))).count();
  assert_eq!(rebirths, 3, "each reader heard each rebirth");
  assert_eq!(twice(&log), Vec::<String>::new(), "no fact is told twice");

  let kept = world::kept(at.join("record.jsonl")).expect("the record reads");
  assert!(checkpoint::segment(&kept).len() <= TAIL, "a later open replays a short tail");
  let again = Life::spawn(opening(&at)).expect("the life has a thread");
  assert_eq!(named(&again, "n").await, Ok("1".to_owned()), "a later life goes on");
  assert!(matches!(again.send(Command::End).await, Ok(Reply::Done)));
}
