//! A life that writes its image at its rest past a tail of facts, from which a later life goes on,
//! and the readers of a life whose log let go of the facts of its past.

#![expect(
  clippy::disallowed_methods,
  reason = "a test blocks on the runtime that tokio starts for it"
)]
#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use crate::yard;

use std::{
  collections::BTreeMap,
  fs,
  path::Path,
  process,
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
use tokio::time;

/// The events of a life, each under its place in the log.
type Logged = (Seq, Arc<Event>);

/// The questions that one run of a word asks, enough for a tail of facts. The journal tells their
/// keeps at the end of the run, so a read of the record holds thousands of facts ahead of the
/// place that it waits for.
const ASKED: usize = 2600;

/// A directory of the test, empty, under a name of its own.
fn place(name: &str) -> yard::Yard {
  yard::Yard::new(format!("furb-host-rebirth-{name}-{}", process::id()))
}

/// A life that keeps its record, with no skill, whose model closes each request with None.
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
    claude: Some(at.join("no-claude").display().to_string()),
    directory: at.display().to_string(),
    record: Some(at.join("record.jsonl").display().to_string()),
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    skills: Some(false),
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
    let next = time::timeout(Duration::from_secs(120), events.next()).await;
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

/// The representation of a name of the module of the root, or why it has none.
async fn named(life: &LifeHandle, name: &str) -> Result<String, String> {
  match life.send(Command::Inspect { name: name.to_owned(), on: None }).await {
    Ok(Reply::Inspected { repr, .. }) => Ok(repr),
    other => Err(format!("{other:?}")),
  }
}

#[tokio::test]
async fn a_life_that_rests_past_its_tail_writes_its_image_and_no_checkpoint_and_goes_on() {
  let at = place("rest");
  let (record, image) = (at.join("record.jsonl"), at.join("record.jsonl.image"));
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  ran(&life, "n = 1").await;
  let busy = ran(&life, &format!("for i in range({ASKED}):\n  chance()")).await;
  let log = until(&mut events, ran_out(&busy)).await;
  let start = time::Instant::now();
  while !image.is_file() {
    assert!(start.elapsed() < Duration::from_secs(120), "the life wrote no image at its rest");
    time::sleep(Duration::from_millis(50)).await;
  }
  let rest = fs::read(&image).expect("the image at rest");
  ran(&life, "n = 2").await;
  assert_eq!(named(&life, "n").await, Ok("2".to_owned()), "the life goes on past its image");
  assert!(!log.iter().any(|one| matches!(*one.1, Event::Reborn(_))), "a tail asks no rebirth");
  let kept = world::kept(&record).expect("the record reads");
  assert_eq!(checkpoint::segment(&kept).len(), kept.len(), "the life wrote no checkpoint");
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  while events.next().await.is_some() {}
  // A life that a kill ends leaves the image of its rest and the entries that the record took after
  // it: a later life goes on from that image and feeds them, and says no fact as it opens.
  fs::write(&image, rest).expect("the image at rest again");
  let lines = fs::read(&record).expect("the record");
  let again = Life::spawn(opening(&at)).expect("the life has a thread");
  assert_eq!(named(&again, "n").await, Ok("2".to_owned()), "a later life feeds the tail");
  // It opens from the image, and stands the root once, as each later life does.
  let now = fs::read(&record).expect("the record");
  let added = now.strip_prefix(lines.as_slice()).map(String::from_utf8_lossy);
  let stand = |line: &str| {
    serde_json::from_str::<serde_json::Value>(line)
      .is_ok_and(|entry| entry[0][1].as_str().is_some_and(|id| id.starts_with("stand")))
  };
  assert!(
    added.as_deref().is_some_and(|added| added.lines().all(stand)),
    "it opens from the image"
  );
  assert!(matches!(again.send(Command::End).await, Ok(Reply::Done)));
}

#[tokio::test]
async fn a_life_past_its_tail_writes_its_image_while_an_act_is_in_flight() {
  let at = place("rest-held");
  let image = at.join("record.jsonl.image");
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  ran(&life, "b = bash('sleep 600')").await;
  let busy = ran(&life, &format!("for i in range({ASKED}):\n  chance()")).await;
  until(&mut events, ran_out(&busy)).await;
  let start = time::Instant::now();
  while !image.is_file() {
    assert!(start.elapsed() < Duration::from_secs(120), "the command in flight held the image");
    time::sleep(Duration::from_millis(50)).await;
  }
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
}

#[tokio::test]
async fn a_life_that_opens_by_its_record_past_a_tail_writes_its_image() {
  let at = place("bare");
  let image = at.join("record.jsonl.image");
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  // A recall tells each entry of the record once, so the run asks twice the questions of a tail.
  let busy = ran(&life, &format!("for i in range({}):\n  chance()", 2 * ASKED)).await;
  until(&mut events, ran_out(&busy)).await;
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  while events.next().await.is_some() {}
  // With no image, the next open reads the past of the record.
  fs::remove_file(&image).expect("the image of the life");
  let again = Life::spawn(opening(&at)).expect("the life has a thread");
  let start = time::Instant::now();
  while !image.is_file() {
    assert!(
      start.elapsed() < Duration::from_secs(120),
      "the life that read its record wrote no image"
    );
    time::sleep(Duration::from_millis(50)).await;
  }
  assert!(matches!(again.send(Command::End).await, Ok(Reply::Done)));
}

/// A life of a record whose image is gone, so it opened by its record: its log told each fact of
/// its past, then let go of each, since the record holds it.
async fn recalled(name: &str) -> (yard::Yard, LifeHandle) {
  let at = place(name);
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  let busy = ran(&life, &format!("for i in range({ASKED}):\n  chance()")).await;
  until(&mut events, ran_out(&busy)).await;
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  while events.next().await.is_some() {}
  fs::remove_file(at.join("record.jsonl.image")).expect("the image of the life");
  let again = Life::spawn(opening(&at)).expect("the life has a thread");
  (at, again)
}

/// The facts of a log, each with its place.
fn facts(of: &[Logged]) -> Vec<Logged> {
  of.iter().filter(|one| !matches!(*one.1, Event::Act(_) | Event::Text { .. })).cloned().collect()
}

/// How many questions of the run of the test a log tells.
fn chances(of: &[Logged]) -> usize {
  of.iter().filter(|one| matches!(&*one.1, Event::Fact(fact) if fact[0] == "chance")).count()
}

#[tokio::test]
async fn a_reader_that_comes_late_reads_each_fact_that_the_log_let_go_as_a_reader_that_came_early()
{
  let (_at, life) = recalled("late").await;
  let mut events = Box::pin(life.events(0));
  let rung = ran(&life, "n = 1").await;
  let mut log = until(&mut events, ran_out(&rung)).await;
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  log.extend(until(&mut events, |event| matches!(event, Event::Ended(None))).await);
  let late: Vec<Logged> = life.events(0).collect().await;
  assert_eq!(facts(&late), facts(&log), "a reader that comes late reads the same facts");
  assert_eq!((chances(&late), chances(&log)), (ASKED, ASKED), "no reader loses a fact of the past");
  let last = |of: &[Logged]| -> BTreeMap<String, Logged> {
    let acts = of.iter().filter_map(|one| match &*one.1 {
      Event::Act(act) => Some((act.id.clone(), one.clone())),
      _ => None,
    });
    acts.collect()
  };
  assert_eq!(last(&late), last(&log), "and the last event of each act, which replaces the others");
}

#[tokio::test]
async fn a_reader_of_the_log_that_holds_no_past_hears_each_act_and_no_fact_that_the_log_let_go() {
  let (_at, life) = recalled("held").await;
  let mut events = Box::pin(life.events(0));
  let first = ran(&life, "x = 1").await;
  until(&mut events, ran_out(&first)).await;
  // A reader from the tip hears the next word first, and no event before it.
  let mut tipped = Box::pin(life.tipped().await.expect("the life gives its tip"));
  let rung = ran(&life, "y = 2").await;
  let heard = until(&mut tipped, |_| true).await;
  let seen: Vec<Logged> = until(&mut events, ran_out(&rung)).await;
  assert!(seen.iter().any(|one| one.0 == heard[0].0), "the reader from the tip starts at the tip");
  assert!(heard[0].0 > seen[0].0, "the reader from the tip hears no event before it");
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  until(&mut events, |event| matches!(event, Event::Ended(None))).await;
  let all: Vec<Logged> = life.events(0).collect().await;
  let held: Vec<Logged> = life.held(0).collect().await;
  assert_eq!((chances(&all), chances(&held)), (ASKED, 0), "the held log tells no fact of the past");
  let acts = |of: &[Logged]| -> Vec<Logged> {
    of.iter().filter(|one| !matches!(&*one.1, Event::Fact(_))).cloned().collect()
  };
  assert_eq!(acts(&held), acts(&all), "and each other event at its place");
}
