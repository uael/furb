//! A life that its thread opens again from its checkpoint, past the tail of facts that a checkpoint
//! bounds. The test runs in a process of its own, with no harness.

#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that it starts for itself"
)]
#![expect(clippy::expect_used, reason = "a test with no harness fails by a panic")]

#[path = "../../tests/common/yard.rs"]
mod yard;

use std::{
  path::Path,
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
use furb_host::{
  Command, Event, Life, LifeHandle, Reply, Seq,
  view::{Kind, Phase},
};
use futures::{Stream, StreamExt};

/// The events of a life, each under its place in the log.
type Logged = (Seq, Arc<Event>);

/// The most facts that a life says after its checkpoint before it writes the next one.
const TAIL: usize = 10_000;

fn main() {
  let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
  runtime.expect("the test has a runtime").block_on(async {
    a_life_goes_on_through_each_rebirth().await;
    a_rebirth_waits_for_each_word_up_to_its_bound_and_names_what_it_cuts().await;
  });
}

/// A directory of the test, empty, under a name of its own.
fn place(name: &str) -> yard::Yard {
  yard::Yard::new(format!("furb-host-rebirth-{name}-{}", std::process::id()))
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

/// A word that runs on the root as a turn of the operator: the name of its rung.
async fn ran(life: &LifeHandle, word: &str) -> String {
  match life.send(Command::Rung { word: word.to_owned(), on: None }).await {
    Ok(Reply::Act(rung)) => rung,
    made => panic!("{word} ran: {made:?}"),
  }
}

/// Whether an event is the run of the word of a rung, once it is done.
fn ran_out(rung: &str) -> impl Fn(&Event) -> bool {
  move |event| {
    matches!(event, Event::Act(act) if act.kind == Kind::Run
      && matches!(act.phase, Phase::Done(_))
      && act.words.first().and_then(|word| word.as_str()) == Some(rung))
  }
}

/// The message of a prompt, when an event is a prompt.
fn message(event: &Event) -> Option<String> {
  match event {
    Event::Act(act) if act.kind == Kind::Prompt => {
      act.words.get(1).and_then(|word| word.as_str()).map(str::to_owned)
    }
    _ => None,
  }
}

/// The representation of a name of the module of the root, or why it has none.
async fn named(life: &LifeHandle, name: &str) -> Result<String, String> {
  match life.send(Command::Inspect { name: name.to_owned(), on: None }).await {
    Ok(Reply::Inspected { repr, .. }) => Ok(repr),
    other => Err(format!("{other:?}")),
  }
}

/// Whether an event is a rebirth after a fault, or after none, as the test wants it.
fn reborn(fault: bool) -> impl Fn(&Event) -> bool {
  move |event| matches!(event, Event::Reborn(why) if why.is_some() == fault)
}

/// The facts of the opening of each rebirth that the log tells twice: those that the opening says
/// before the stand of its boot, which restate its checkpoint. A fact after that stand is one of
/// the life, though it equals an earlier one.
fn restated(log: &[Logged]) -> Vec<String> {
  let fact = |one: &Logged| match &*one.1 {
    Event::Fact(fact) => Some(fact.to_string()),
    _ => None,
  };
  let facts: Vec<String> = log.iter().filter_map(fact).collect();
  let mut opening = false;
  let mut restated = Vec::new();
  for one in log {
    match &*one.1 {
      Event::Reborn(_) => opening = true,
      Event::Fact(said) if opening && said[0] == "stand" && said[2] == "operator" => {
        opening = false;
      }
      Event::Fact(said) if opening => {
        let said = said.to_string();
        if facts.iter().filter(|other| **other == said).count() > 1 {
          restated.push(said);
        }
      }
      _ => {}
    }
  }
  restated
}

async fn a_life_goes_on_through_each_rebirth() {
  let at = place("tail");
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
  assert_eq!(rebirths, 1, "each reader heard each rebirth");
  assert_eq!(
    restated(&log),
    Vec::<String>::new(),
    "no fact that a checkpoint restates is told twice"
  );

  let kept = world::kept(at.join("record.jsonl")).expect("the record reads");
  assert!(checkpoint::segment(&kept).len() <= TAIL, "a later open replays a short tail");
  let again = Life::spawn(opening(&at)).expect("the life has a thread");
  assert_eq!(named(&again, "n").await, Ok("1".to_owned()), "a later life goes on");
  assert!(matches!(again.send(Command::End).await, Ok(Reply::Done)));
}

/// A word that binds an act whose ear lives until its life ends, which no checkpoint keeps.
const LASTING: &str = "def lasting(on=''):
  def ear(id):
    yield 'started', id
    while True:
      yield {id}
  return act('lasting', on, ear)
held = lasting()
held_id = str(held)";

/// The name that a value of the root binds, as a string.
async fn bound(life: &LifeHandle, name: &str) -> String {
  let got = named(life, name).await.unwrap_or_else(|no| panic!("the root binds no {name}: {no}"));
  got.trim_matches('\'').to_owned()
}

async fn a_rebirth_waits_for_each_word_up_to_its_bound_and_names_what_it_cuts() {
  let at = place("bound");
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  ran(&life, "asked = prompt(str, 'Name a color.', to=OPERATOR)").await;
  let asked = bound(&life, "asked").await;
  let waits = ran(&life, "got = await wait(3600, asked)").await;
  let busy = ran(&life, "for i in range(2600):\n  chance()").await;
  let mut log = until(&mut events, ran_out(&busy)).await;
  assert!(
    !log.iter().any(|one| matches!(*one.1, Event::Reborn(_))),
    "a word that awaits a wait holds the rebirth past the tail"
  );
  let answered = life.send(Command::Answer { id: asked.clone(), line: "red".to_owned() }).await;
  assert!(matches!(answered, Ok(Reply::Done)), "the prompt stayed open: {answered:?}");
  log.extend(until(&mut events, ran_out(&waits)).await);
  log.extend(until(&mut events, reborn(false)).await);
  assert_eq!(
    named(&life, "got").await,
    Ok(format!("'{asked}'")),
    "the word ended before the rebirth"
  );

  ran(&life, LASTING).await;
  let held = bound(&life, "held_id").await;
  ran(&life, "late = prompt(str, 'Name a fruit.', to=OPERATOR)").await;
  let late = bound(&life, "late").await;
  let stuck = ran(&life, "await wait(3600, late)").await;
  ran(&life, "for i in range(6000):\n  chance()").await;
  log.extend(until(&mut events, reborn(false)).await);
  let told =
    until(&mut events, |event| message(event).is_some_and(|one| one.contains("born again")));
  let told = message(&told.await.last().expect("the root was told").1).expect("a prompt");
  assert!(
    told.contains(&format!("{held} on ")),
    "the rebirth names the act with an ear that it cut: {told}"
  );
  assert!(
    told.contains(&format!("(the word of {stuck})")),
    "the rebirth names the word that it cut: {told}"
  );
  assert!(!told.contains(&late), "the rebirth names no prompt that the checkpoint keeps: {told}");
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
}
