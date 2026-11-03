//! A life on its thread, as a client of the host holds it: its commands, its events in order, and
//! its end.

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
  Object,
  life::{Answer, Opening},
};
use furb_host::{Command, Event, Life, LifeError, LifeHandle, Reply, Seq, State};
use futures::{Stream, StreamExt};
use serde_json::json;

/// The events of a life, each under its place in the log.
type Logged = (Seq, Arc<Event>);

/// A directory of one test, empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-host-{name}-{}", std::process::id()));
  fs::remove_dir_all(&at).ok();
  if let Err(no) = fs::create_dir_all(&at) {
    panic!("the test has no directory {}: {no}", at.display());
  }
  at
}

/// A life with no record and no extension, whose model answers each request with the next word of a
/// script, and with close(None) once the script runs out.
fn scripted(at: &Path, words: &[&str]) -> Opening {
  let words: Vec<String> = words.iter().map(|one| (*one).to_owned()).collect();
  let next = AtomicUsize::new(0);
  let answer = Answer(Arc::new(move |_, _| {
    let at = next.fetch_add(1, Ordering::SeqCst);
    let word = words.get(at).cloned().unwrap_or_else(|| "close(None)".to_owned());
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string(word),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  Opening {
    directory: at.display().to_string(),
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
    let next = tokio::time::timeout(Duration::from_secs(60), events.next()).await;
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

/// Whether an event is the act of a name, done.
fn done(id: &str) -> impl Fn(&Event) -> bool {
  move |event| matches!(event, Event::Act(act) if act.id == id && matches!(act.state, State::Done(_)))
}

/// The act that a command made.
async fn made(life: &LifeHandle, command: Command) -> String {
  match life.send(command).await {
    Ok(Reply::Act(id)) => id,
    other => panic!("the command made no act: {other:?}"),
  }
}

/// A rung of a word on the root.
fn rung(word: &str) -> Command {
  Command::Rung { word: word.to_owned(), on: None }
}

#[tokio::test]
async fn a_prompt_is_told_as_its_facts_and_its_acts_in_order_with_no_gap_until_the_end() {
  let at = place("prompt");
  let life = Life::spawn(scripted(&at, &["close('ok')"])).unwrap();
  let mut events = Box::pin(life.events(0));
  let prompt =
    Command::Prompt { shape: "str".to_owned(), message: "Say ok.".to_owned(), to: None, on: None };
  let id = made(&life, prompt).await;
  let got = until(&mut events, done(&id)).await;
  let places: Vec<Seq> = got.iter().map(|one| one.0).collect();
  assert_eq!(places, (0..got.len() as Seq).collect::<Vec<_>>());
  let states: Vec<&State> = got
    .iter()
    .filter_map(|one| match &*one.1 {
      Event::Act(act) if act.id == id => Some(&act.state),
      _ => None,
    })
    .collect();
  assert_eq!(states, [&State::Asked, &State::Started, &State::Done(json!("ok"))]);
  let asked =
    got.iter().position(|one| matches!(&*one.1, Event::Fact(fact) if fact[1] == json!(id)));
  let act = got.iter().position(|one| matches!(&*one.1, Event::Act(act) if act.id == id));
  assert!(asked.unwrap() < act.unwrap());
  assert!(got.iter().any(|one| *one.1 == Event::Changed("chain1".to_owned())));
  assert_eq!(life.send(Command::End).await.unwrap(), Reply::Done);
  let rest = until(&mut events, |event| matches!(event, Event::Ended(_))).await;
  assert_eq!(*rest.last().unwrap().1, Event::Ended(None));
  assert_eq!(rest[0].0, got.len() as Seq);
  assert!(events.next().await.is_none());
  assert!(matches!(life.send(Command::Standing).await, Err(LifeError::Ended)));
  fs::remove_dir_all(&at).unwrap();
}

#[tokio::test]
async fn a_reader_that_stands_behind_the_life_reads_its_events_again_from_the_log() {
  let at = place("behind");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut behind = Box::pin(life.events(0));
  let mut ahead = Box::pin(life.events(0));
  let id = made(&life, rung("for n in range(300):\n  clock()")).await;
  let heard = until(&mut ahead, done(&id)).await;
  assert!(heard.len() > 600, "{} events", heard.len());
  assert_eq!(until(&mut behind, done(&id)).await, heard);
  let mut late = Box::pin(life.events(0));
  assert_eq!(until(&mut late, done(&id)).await, heard);
  let mut tail = Box::pin(life.events(heard.len() as Seq - 1));
  assert_eq!(until(&mut tail, done(&id)).await, heard[heard.len() - 1..]);
  fs::remove_dir_all(&at).unwrap();
}

#[tokio::test]
async fn a_query_is_a_command_that_the_life_answers() {
  let at = place("queries");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let id = made(&life, rung("x = 41 + 1")).await;
  until(&mut events, done(&id)).await;
  let inspected = life.send(Command::Inspect { name: "x".to_owned(), on: None }).await.unwrap();
  let x = Reply::Inspected { kind: "int".to_owned(), repr: "42".to_owned(), value: json!(42) };
  assert_eq!(inspected, x);
  let Reply::Value(program) = life.send(Command::Program { on: None }).await.unwrap() else {
    panic!("the program is a value")
  };
  assert_eq!(program[id.as_str()], json!("x = 41 + 1"));
  let of = Some("rung".to_owned());
  let Reply::Values(rungs) = life.send(Command::Transcript { on: None, of }).await.unwrap() else {
    panic!("the transcript is values")
  };
  assert!(rungs.iter().any(|one| one[1] == json!(id)));
  let Reply::Values(turns) = life.send(Command::Turns { on: None }).await.unwrap() else {
    panic!("the turns are values")
  };
  assert!(turns.iter().any(|one| one[1].as_str().is_some_and(|said| said.contains("x = 41 + 1"))));
  let Reply::Value(standing) = life.send(Command::Standing).await.unwrap() else {
    panic!("the standing is a value")
  };
  assert!(standing.is_array());
  let gate = Command::Gate { word: "BAD".to_owned(), on: None };
  let Reply::Value(found) = life.send(gate).await.unwrap() else {
    panic!("the gate gives a value")
  };
  assert_ne!(found, json!([]));
  let wrong = life.send(Command::Answer { id: id.clone(), line: "1".to_owned() }).await;
  assert!(
    matches!(wrong, Err(LifeError::Refused(why)) if why == format!("No prompt is named {id}."))
  );
  fs::remove_dir_all(&at).unwrap();
}

#[tokio::test]
async fn a_life_that_does_not_open_ends_with_its_fault() {
  let at = place("fault");
  let opening = Opening { record: Some(at.display().to_string()), ..scripted(&at, &[]) };
  let life = Life::spawn(opening).unwrap();
  let mut events = Box::pin(life.events(0));
  let got = until(&mut events, |event| matches!(event, Event::Ended(_))).await;
  assert!(matches!(&*got.last().unwrap().1, Event::Ended(Some(why)) if !why.is_empty()));
  assert!(events.next().await.is_none());
  assert!(matches!(life.send(Command::Standing).await, Err(LifeError::Ended)));
  fs::remove_dir_all(&at).unwrap();
}
