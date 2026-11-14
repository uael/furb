//! A life that opens from the image beside its record: it stands where the life before it stood,
//! reads no fact of its record, and takes again each act that the life before it held in flight.

#![expect(
  clippy::disallowed_methods,
  reason = "a test blocks on the runtime that tokio starts for it"
)]
#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{
  fs, future,
  path::Path,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use furb::{
  Object,
  life::{Answer, Opening},
  view::{Act, Reach},
};
use furb_host::{Command, Event, Life, LifeHandle, Reply};
use futures::StreamExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::life::{Logged, birth, done, facts_of, kept, made, place, rung, until};

/// A life that keeps its record, whose model closes each request with a word, or answers none when
/// it is given no word, so each reply stays in flight.
fn answering(at: &Path, word: Option<&'static str>) -> Opening {
  let answer = Answer(Arc::new(move |_, _| {
    Box::pin(async move {
      let Some(word) = word else { return future::pending().await };
      let said =
        [Object::string("assistant"), Object::string(word), Object::none(), Object::none()];
      Ok(Object::tuple(said))
    })
  }));
  Opening { answer: Some(answer), ..kept(at) }
}

/// What a window asks a life for its first screen, and what the engine says of its root.
async fn views(life: &LifeHandle) -> Vec<Reply> {
  let asked = [
    Command::Desk { notices: Vec::new() },
    Command::Threads { on: None, at: None, read: None, site: None },
    Command::Rail { hidden: Vec::new(), at: None },
    Command::Ladder {
      on: None,
      at: None,
      opened: Vec::new(),
      owed: Vec::new(),
      taken: None,
      shown: None,
    },
    Command::Moments { on: None },
    Command::Queued { on: None },
    Command::Turns { on: None },
    Command::Standing,
  ];
  let mut got = Vec::new();
  for one in asked {
    got.push(life.send(one).await.expect("a view of the life"));
  }
  got
}

/// A fact that the operator says last, and the log of the life up to it, as a reader that attaches
/// then reads it from its first place.
async fn log_of(life: &LifeHandle) -> Vec<Logged> {
  let sent = life.send(Command::Seen { about: "the tip".to_owned() }).await;
  assert!(matches!(sent, Ok(Reply::Done)), "{sent:?}");
  let tip =
    |event: &Event| matches!(event, Event::Fact(fact) if fact[0] == "seen" && fact[1] == "the tip");
  until(&mut Box::pin(life.events(0)), tip).await
}

/// The end of a life, once its thread is gone, so the record and its image stand whole.
async fn ended(life: LifeHandle) {
  let mut events = Box::pin(life.events(0));
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  while events.next().await.is_some() {}
}

/// The name of the act that a name of the module of the root binds.
async fn act_of(life: &LifeHandle, name: &str) -> String {
  match life.send(Command::Inspect { name: name.to_owned(), on: None }).await {
    Ok(Reply::Inspected { repr, .. }) => {
      repr.split('\'').nth(1).expect("the repr of an act").to_owned()
    }
    other => panic!("{name} binds no act: {other:?}"),
  }
}

/// The facts of a record of a kind about an act.
fn recorded(at: &Path, kind: &str, about: &str) -> Vec<Value> {
  recorded_all(at, kind).into_iter().filter(|fact| fact[1] == about).collect()
}

/// The facts of a record of a kind.
fn recorded_all(at: &Path, kind: &str) -> Vec<Value> {
  let record = fs::read_to_string(at.join("record.jsonl")).expect("the record");
  let facts = record.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok());
  facts.map(|entry| entry[0].clone()).filter(|fact| fact[0] == kind).collect()
}

#[tokio::test]
async fn a_life_opens_from_its_image_where_the_life_before_it_stood() {
  let at = place("image-stood");
  let (record, image) = (at.join("record.jsonl"), at.join("record.jsonl.image"));
  // The first life ends with a checkpoint, so the log of the second lets go of its facts, which a
  // reader then reads from the record.
  birth(&at, "x = 1\nc = chain('c')", Vec::new()).await;
  let life = Life::spawn(answering(&at, Some("close('ok')"))).expect("the second life");
  let mut events = Box::pin(life.events(0));
  let prompt =
    Command::Prompt { shape: "str".to_owned(), message: "Say ok.".to_owned(), to: None, on: None };
  let id = made(&life, prompt).await;
  until(&mut events, done(&id)).await;
  let (stood, told) = (views(&life).await, log_of(&life).await);
  ended(life).await;
  assert!(image.is_file(), "a life that keeps its record ends with its image");
  let lines = fs::read(&record).expect("the record");
  let life = Life::spawn(answering(&at, Some("close('ok')"))).expect("the third life");
  assert_eq!(views(&life).await, stood, "the first screen of the life is the one it left");
  let tip = until(&mut Box::pin(life.events(0)), |event| *event == *told[told.len() - 1].1).await;
  assert_eq!(tip, told, "a reader reads each event of the life before at its place");
  let said = fs::read(&record).expect("the record");
  assert!(said == lines, "an open from an image says no fact");
  ended(life).await;
}

#[tokio::test]
async fn a_life_that_opens_from_its_image_takes_again_each_act_in_flight() {
  let at = place("image-flight");
  let life = Life::spawn(answering(&at, None)).expect("the first life");
  let mut events = Box::pin(life.events(0));
  let word = "b = bash('sleep 600')\np = prompt(str, 'Which one?', to=OPERATOR)\n\
              w = wait(3600, p)\nr = prompt(str, 'Say late.')";
  let id = made(&life, rung(word)).await;
  let mut log = until(&mut events, done(&id)).await;
  let (w, b) = (act_of(&life, "w").await, act_of(&life, "b").await);
  let (p, r) = (act_of(&life, "p").await, act_of(&life, "r").await);
  // Each act is in flight once its ear said that it started: the wait, the command, the prompt to
  // the operator, and the reply that the model of the second prompt owes.
  let started = |log: &[Logged], id: &str| {
    facts_of(log).iter().any(|fact| fact[0] == "started" && fact[1] == id)
  };
  let reply = |log: &[Logged]| {
    let asked = facts_of(log).into_iter().rfind(|fact| fact[0] == "reply");
    asked.and_then(|fact| fact[1].as_str().map(str::to_owned))
  };
  let flies = |log: &[Logged]| {
    [&w, &b, &p].iter().all(|id| started(log, id)) && reply(log).is_some_and(|id| started(log, &id))
  };
  while !flies(&log) {
    log.extend(until(&mut events, |event| matches!(event, Event::Fact(_))).await);
  }
  ended(life).await;
  assert_eq!(recorded(&at, "due", &w).len(), 1, "the wait has one deadline");
  let life = Life::spawn(answering(&at, Some("close('late')"))).expect("the second life");
  let mut events = Box::pin(life.events(0));
  // The command, whose process went with the life before, is closed.
  let mut log = until(&mut events, done(&b)).await;
  let closed = facts_of(&log).into_iter().rfind(|fact| fact[0] == "done" && fact[1] == b.as_str());
  let why = format!("{b} was interrupted: the life that ran it ended before it did");
  assert_eq!(closed.map(|fact| fact[3].clone()), Some(json!({ "is": "Refused", "args": [why] })));
  // The model of this life answers the reply that the model of the life before owed.
  if !log.iter().any(|one| done(&r)(&one.1)) {
    log.extend(until(&mut events, done(&r)).await);
  }
  let late = life.send(Command::Peek { id: r }).await;
  assert!(matches!(&late, Ok(Reply::Value(value)) if *value == json!("late")), "{late:?}");
  // The operator answers the prompt, which ends the wait on it: the ear of time took the wait
  // again with the deadline that it had.
  let answered = life.send(Command::Answer { id: p.clone(), line: "this".to_owned() }).await;
  assert!(matches!(answered, Ok(Reply::Done)), "{answered:?}");
  until(&mut events, done(&w)).await;
  let this = life.send(Command::Peek { id: p }).await;
  assert!(matches!(&this, Ok(Reply::Value(value)) if *value == json!("this")), "{this:?}");
  ended(life).await;
  assert_eq!(recorded(&at, "due", &w).len(), 1, "the wait keeps its one deadline");
  assert_eq!(recorded(&at, "started", &w).len(), 2, "each life took the wait one time");
}

#[tokio::test]
async fn a_life_on_an_image_that_its_record_passed_takes_again_the_acts_that_no_later_entry_ended()
{
  let at = place("image-tail");
  let image = at.join("record.jsonl.image");
  let life = Life::spawn(answering(&at, None)).expect("the first life");
  let mut events = Box::pin(life.events(0));
  let word = "p = prompt(str, 'Which one?', to=OPERATOR)\nw = wait(3600, p)\n\
              v = prompt(str, 'And then?', to=OPERATOR)";
  let id = made(&life, rung(word)).await;
  let mut log = until(&mut events, done(&id)).await;
  let (w, v, p) = (act_of(&life, "w").await, act_of(&life, "v").await, act_of(&life, "p").await);
  let started = |log: &[Logged], id: &str| {
    facts_of(log).iter().filter(|fact| fact[0] == "started" && fact[1] == id).count()
  };
  while [&w, &v, &p].iter().any(|id| started(&log, id) == 0) {
    log.extend(until(&mut events, |event| matches!(event, Event::Fact(_))).await);
  }
  ended(life).await;
  let first = fs::read(&image).expect("the image of the first life");
  // The second life goes on from the image: the operator answers the first prompt, which ends
  // the wait, and the second prompt stays in flight.
  let life = Life::spawn(answering(&at, None)).expect("the second life");
  let mut events = Box::pin(life.events(0));
  let answered = life.send(Command::Answer { id: p.clone(), line: "this".to_owned() }).await;
  assert!(matches!(answered, Ok(Reply::Done)), "{answered:?}");
  until(&mut events, done(&w)).await;
  ended(life).await;
  // The second life leaves no image, as a life that crashed: the image of the first one stands,
  // and the record holds the entries of the second life after it.
  fs::write(&image, first).expect("the image of the first life again");
  let life = Life::spawn(answering(&at, None)).expect("the third life");
  let mut events = Box::pin(life.events(0));
  let mut log = Vec::new();
  // The console of each life took the second prompt one time, and the third life tells the start
  // of the second life again from the record.
  while started(&log, &v) < 3 {
    log.extend(until(&mut events, |event| matches!(event, Event::Fact(_))).await);
  }
  let this = life.send(Command::Peek { id: p }).await;
  assert!(matches!(&this, Ok(Reply::Value(value)) if *value == json!("this")), "{this:?}");
  ended(life).await;
  assert_eq!(recorded(&at, "started", &w).len(), 2, "no ear takes an act that the record ended");
  assert_eq!(recorded(&at, "started", &v).len(), 3, "each life took the act that no entry ended");
  assert_eq!(recorded(&at, "due", &w).len(), 1, "the wait keeps its one deadline");
}

#[tokio::test]
async fn a_life_whose_image_does_not_load_opens_by_its_record_after_its_past() {
  // The key of the image still names the tip of the record, and its body is not the one it names.
  opens_by_its_record_after_its_past("image-torn", |image| {
    *image.last_mut().expect("a byte") ^= 1;
  })
  .await;
}

#[tokio::test]
async fn a_life_whose_image_holds_the_part_of_a_host_of_another_build_opens_by_its_record() {
  // The key and the body agree, and the part of the host names the files of another build.
  opens_by_its_record_after_its_past("image-host", |image| {
    let end = image.iter().position(|byte| *byte == b'\n').expect("the line of the key");
    let mut key: Value = serde_json::from_slice(&image[..end]).expect("the key");
    let at = |field: &str| usize::try_from(key[field].as_u64().expect(field)).expect(field);
    let host = end + 1 + at("session") + at("half");
    let named = b"{\"build\":\"";
    assert!(image[host..].starts_with(named), "the part of the host names its build first");
    let digest = host + named.len();
    image[digest..digest + 64].fill(b'0');
    key["body"] = json!(format!("{:x}", Sha256::digest(&image[end + 1..])));
    let line = serde_json::to_vec(&key).expect("the key");
    image[..line.len()].copy_from_slice(&line);
    image[line.len()..end].fill(b' ');
  })
  .await;
}

/// A life whose image the change makes one that does not load opens by its record: it stands as
/// the life before it stood, tells its past, then its birth, and adds to its record what an open
/// by the record adds, once. It ends with an image that agrees.
async fn opens_by_its_record_after_its_past(name: &str, change: impl FnOnce(&mut Vec<u8>)) {
  let at = place(name);
  let image = at.join("record.jsonl.image");
  birth(&at, "x = 1", Vec::new()).await;
  let life = Life::spawn(kept(&at)).expect("the second life");
  let stood = views(&life).await;
  ended(life).await;
  let births = || recorded_all(&at, "stand").len();
  // What an open by the record adds to the record, with no image beside it.
  fs::remove_file(&image).expect("the image of the second life");
  let before = births();
  ended(Life::spawn(kept(&at)).expect("the third life")).await;
  let by_record = births() - before;
  assert!(by_record > 0, "an open by the record stands each chain");
  let mut changed = fs::read(&image).expect("the image");
  change(&mut changed);
  fs::write(&image, changed).expect("the image that does not load");
  let before = births();
  let life = Life::spawn(kept(&at)).expect("the fourth life");
  assert_eq!(views(&life).await[..3], stood[..3], "the life stands on its record");
  assert_eq!(births() - before, by_record, "the life opens by its record once, and no image");
  // The life told its past, then its birth: the rung of the first life is done in its log.
  let log = log_of(&life).await;
  let past = log.iter().position(|one| matches!(*one.1, Event::Reborn(None)));
  let rung =
    log[..past.expect("a birth after the past")].iter().rev().find_map(|one| match &*one.1 {
      Event::Act(act) if act.words.first() == Some(&json!("x = 1")) => Some(Act::done(act)),
      _ => None,
    });
  assert_eq!(rung, Some(true), "the life tells its past before its birth");
  ended(life).await;
  let held = fs::read(&image).expect("the image of the life that opened by the record");
  assert!(held.starts_with(b"{\"build\":"), "the life ends with an image that agrees");
}

#[tokio::test]
async fn a_life_that_opens_from_its_image_shows_each_thread_as_the_life_before_it_did() {
  let at = place("image-page");
  // The model answers its first request with a word that waits on a second request, which it
  // never answers, so the line of the word runs and its page says since when.
  let asked = Arc::new(AtomicUsize::new(0));
  let answer = Answer(Arc::new(move |_, _| {
    let first = asked.fetch_add(1, Ordering::Relaxed) == 0;
    Box::pin(async move {
      if !first {
        return future::pending().await;
      }
      let word = Object::string("await wait(3600, prompt(str, 'Say late.'))");
      Ok(Object::tuple([Object::string("assistant"), word, Object::none(), Object::none()]))
    })
  }));
  let life = Life::spawn(Opening { answer: Some(answer), ..kept(&at) }).expect("the first life");
  let mut events = Box::pin(life.events(0));
  let prompt =
    Command::Prompt { shape: "str".to_owned(), message: "Wait.".to_owned(), to: None, on: None };
  let thread = made(&life, prompt).await;
  until(&mut events, |event| matches!(event, Event::Fact(fact) if fact[0] == "due")).await;
  let page = |thread: String| Command::Thread { thread, reach: Reach { before: None, limit: 20 } };
  let before = life.send(page(thread.clone())).await.expect("the page of the thread");
  ended(life).await;
  let life = Life::spawn(answering(&at, None)).expect("the second life");
  let after = life.send(page(thread)).await.expect("the page of the thread");
  assert_eq!(after, before, "the line runs since the second that the life before saw it begin");
  ended(life).await;
}
