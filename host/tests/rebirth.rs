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
  view::{Kind, Phase},
  world,
};
use furb_host::{Command, Event, Life, LifeHandle, Reply, Seq};
use futures::{Stream, StreamExt};

/// The events of a life, each under its place in the log.
type Logged = (Seq, Arc<Event>);

/// The most facts that a life says after its checkpoint before it writes the next one.
const TAIL: usize = 10_000;

fn main() {
  let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
  runtime.expect("the test has a runtime").block_on(async {
    a_life_goes_on_through_each_rebirth().await;
    a_rebirth_waits_for_each_word_that_runs().await;
    a_rebirth_waits_for_each_act_with_an_ear_and_cuts_nothing().await;
    a_life_with_the_default_skills_is_born_again_at_its_tail().await;
  });
  the_check_of_a_held_rebirth_fails_on_an_early_rebirth_and_passes_a_later_one();
}

/// How many rebirths the log tells from a place on: the interval where live work must hold the
/// rebirth, which the test marks before the workload that it checks.
fn rebirths_from(log: &[Logged], mark: usize) -> usize {
  log[mark..].iter().filter(|one| matches!(*one.1, Event::Reborn(_))).count()
}

/// The check of the interval fails when a rebirth comes inside it, early or late, and passes when
/// the only rebirth comes after it.
fn the_check_of_a_held_rebirth_fails_on_an_early_rebirth_and_passes_a_later_one() {
  let at = |seq: Seq, event: Event| -> Logged { (seq, Arc::new(event)) };
  let told = || Event::Notice("a fact".to_owned());
  let early = [at(0, told()), at(1, Event::Reborn(None)), at(2, told()), at(3, told())];
  assert_eq!(rebirths_from(&early, 1), 1, "an early rebirth inside the interval is seen");
  let late = [at(0, told()), at(1, told()), at(2, told()), at(3, Event::Reborn(None))];
  assert_eq!(rebirths_from(&late, 1), 1, "a rebirth at the end of the interval is seen");
  assert_eq!(rebirths_from(&late, 4), 0, "a rebirth after the interval is allowed");
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

/// A word that binds an act whose ear lives until a cancel ends it, which no checkpoint keeps.
const LASTING: &str = "def lasting(on=''):
  def ear(id):
    yield 'started', id
    while True:
      yield {id}
  return act('lasting', on, controlled(ear))
held = lasting()
held_id = str(held)";

/// The name that a value of the root binds, as a string.
async fn bound(life: &LifeHandle, name: &str) -> String {
  let got = named(life, name).await.unwrap_or_else(|no| panic!("the root binds no {name}: {no}"));
  got.trim_matches('\'').to_owned()
}

async fn a_rebirth_waits_for_each_word_that_runs() {
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

  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
}

async fn a_rebirth_waits_for_each_act_with_an_ear_and_cuts_nothing() {
  let at = place("ear");
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  let mut log = Vec::new();
  let lasting = ran(&life, LASTING).await;
  log.extend(until(&mut events, ran_out(&lasting)).await);
  let held = bound(&life, "held_id").await;
  ran(&life, "late = prompt(str, 'Name a fruit.', to=OPERATOR)").await;
  let late = bound(&life, "late").await;
  let stuck = ran(&life, "kept = await wait(3600, late)").await;
  let mark = log.len();
  let busy = ran(&life, "for i in range(6000):\n  chance()").await;
  log.extend(until(&mut events, ran_out(&busy)).await);
  assert_eq!(
    rebirths_from(&log, mark),
    0,
    "a word that runs and an act with an ear hold the rebirth past twice the tail"
  );
  let answered = life.send(Command::Answer { id: late.clone(), line: "pear".to_owned() }).await;
  assert!(matches!(answered, Ok(Reply::Done)), "the prompt stayed open: {answered:?}");
  log.extend(until(&mut events, ran_out(&stuck)).await);
  let ended = ran(&life, &format!("cancel('{held}')")).await;
  log.extend(until(&mut events, ran_out(&ended)).await);
  log.extend(until(&mut events, reborn(false)).await);
  assert_eq!(named(&life, "kept").await, Ok(format!("'{late}'")), "the word ended before it");
  assert!(
    !log.iter().any(|one| message(&one.1).is_some_and(|said| said.contains("born again"))),
    "a rebirth cuts nothing, so it tells the root nothing"
  );
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
}

/// A life with the default skills, whose life words hold acts that live until the life ends, is
/// born again at its tail: the seed makes each of those acts again, so none holds the rebirth.
async fn a_life_with_the_default_skills_is_born_again_at_its_tail() {
  let at = place("skills");
  let life =
    Life::spawn(Opening { extensions: None, ..opening(&at) }).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  ran(&life, "n = 1").await;
  ran(&life, "for i in range(2600):\n  chance()").await;
  until(&mut events, reborn(false)).await;
  assert_eq!(named(&life, "n").await, Ok("1".to_owned()), "the life goes on past its tail");
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
}
