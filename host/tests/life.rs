//! A life on its thread, as a client of the host holds it: its commands, its events in order, and
//! its end.

#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

#[path = "../../tests/common/yard.rs"]
mod yard;

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
  world::config::{Change, Layer},
};
use furb_host::{
  Command, Event, Life, LifeError, LifeHandle, Reply, Seq,
  view::{Kind, Phase, RunState, State},
};
use futures::{Stream, StreamExt};
use serde_json::json;

/// The events of a life, each under its place in the log.
type Logged = (Seq, Arc<Event>);

/// A directory of one test, empty.
fn place(name: &str) -> yard::Yard {
  yard::Yard::new(format!("furb-host-{name}-{}", std::process::id()))
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
  move |event| matches!(event, Event::Act(act) if act.id == id && matches!(act.phase, Phase::Done(_)))
}

/// Whether an event is the act of a name, in a state.
fn stands(id: String, state: State) -> impl Fn(&Event) -> bool {
  move |event| matches!(event, Event::Act(act) if act.id == id && act.state == state)
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
  let phases: Vec<&Phase> = got
    .iter()
    .filter_map(|one| match &*one.1 {
      Event::Act(act) if act.id == id => Some(&act.phase),
      _ => None,
    })
    .collect();
  assert_eq!(phases, [&Phase::Asked, &Phase::Started, &Phase::Done(json!("ok"))]);
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
}

#[tokio::test]
async fn the_configs_of_a_life_are_read_and_changed_through_its_commands_and_a_change_stands_it() {
  let at = place("configs");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let key = |names: &[&str]| names.iter().map(|one| (*one).to_owned()).collect::<Vec<_>>();
  let thinking =
    Change { key: key(&["max_thinking_tokens"]), value: Some(json!(0)), layer: Layer::Local };
  let Reply::Act(stand) = life.send(Command::Configure(thinking)).await.unwrap() else {
    panic!("a change gives the act of the stand that follows it")
  };
  assert!(stand.starts_with("stand"), "{stand}");
  let Reply::Values(settings) = life.send(Command::Settings).await.unwrap() else {
    panic!("the settings are values")
  };
  let thinking = settings.iter().find(|one| one["key"] == json!(["max_thinking_tokens"])).unwrap();
  assert_eq!((&thinking["value"], &thinking["layer"]), (&json!(0), &json!("local")));
  assert_eq!(thinking["property"]["type"], json!("integer"), "the schema names the key");
  let wrong =
    Change { key: key(&["providers", "nothing"]), value: Some(json!(true)), layer: Layer::Project };
  let refused = life.send(Command::Configure(wrong)).await;
  assert!(
    matches!(&refused, Err(LifeError::Refused(why)) if why.contains("nothing")),
    "{refused:?}"
  );
  assert!(!at.join(".furb/config.json").exists(), "a refused change leaves no config");
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
}
#[tokio::test]
async fn each_act_is_told_with_its_meaning_and_each_chain_with_the_state_of_its_work() {
  let at = place("meaning");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let refused = made(&life, rung("BAD")).await;
  let got = until(&mut events, stands(refused.clone(), State::Failed)).await;
  let Some(Event::Act(failed)) = got.last().map(|one| &*one.1) else { panic!("an act") };
  assert!(
    matches!(&failed.run, Some(RunState::Refused(found)) if found.contains("BAD")),
    "{:?}",
    failed.run
  );
  let prompt = Command::Prompt {
    shape: "str".to_owned(),
    message: "Which base?".to_owned(),
    to: Some("operator".to_owned()),
    on: None,
  };
  let asked = made(&life, prompt).await;
  until(&mut events, stands(asked.clone(), State::Asks)).await;
  until(&mut events, stands("chain1".to_owned(), State::Asks)).await;
  life.send(Command::Answer { id: asked.clone(), line: "main".to_owned() }).await.unwrap();
  until(&mut events, stands(asked, State::Done)).await;
  until(&mut events, stands("chain1".to_owned(), State::Done)).await;
  made(&life, rung("await wait(60, __name__)")).await;
  until(&mut events, |event| matches!(event, Event::Act(act) if act.kind == Kind::Wait)).await;
  life.send(Command::Pause { id: "chain1".to_owned(), why: None }).await.unwrap();
  let held = until(
    &mut events,
    |event| matches!(event, Event::Act(act) if act.kind == Kind::Wait && act.state == State::Paused),
  )
  .await;
  let Some(Event::Act(wait)) = held.last().map(|one| &*one.1) else { panic!("an act") };
  assert_eq!(wait.pause.as_ref().map(|one| one.by.as_str()), Some("operator"));
  let chain = |one: &&Logged| matches!(&*one.1, Event::Act(act) if act.id == "chain1");
  let Some(Event::Act(root)) = held.iter().rfind(chain).map(|one| &*one.1) else {
    panic!("an act")
  };
  assert_eq!(root.state, State::Paused);
}
#[tokio::test]
async fn the_verbs_of_the_operator_act_on_the_life_as_a_word_of_his_would() {
  let at = place("operator");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let text = |path: &str| path.to_owned();
  let write = Command::Write { path: text("note.txt"), content: text("hi\n"), on: None };
  assert_eq!(life.send(write).await.unwrap(), Reply::Done);
  let read = life.send(Command::Read { path: text("note.txt"), on: None }).await.unwrap();
  assert!(matches!(&read, Reply::Value(got) if got["content"] == json!("hi\n")), "{read:?}");
  let bash =
    made(&life, Command::Bash { command: text("cat note.txt"), stdin: None, on: None }).await;
  until(&mut events, done(&bash)).await;
  let Reply::Value(exit) = life.send(Command::Peek { id: bash }).await.unwrap() else {
    panic!("a command comes to a value")
  };
  assert!(exit.to_string().contains("hi"), "{exit}");
  let bound = made(&life, rung("x = 1")).await;
  until(&mut events, done(&bound)).await;
  let Reply::Values(names) = life.send(Command::Names { on: None }).await.unwrap() else {
    panic!("the names are values")
  };
  assert!(names.contains(&json!("x")), "{names:?}");
  let grant = Command::Grant { usd: Some(2.0), share: None, on: None };
  assert!(made(&life, grant).await.starts_with("grant"));
  let stand = made(&life, Command::Stand).await;
  until(&mut events, done(&stand)).await;
  // The scratch directory that a command of the life made goes as the thread of the life ends.
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

#[tokio::test]
async fn the_operator_closes_an_act_and_says_what_he_saw() {
  let at = place("operator-record");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let text = |path: &str| path.to_owned();
  let asked = |shape: &str| Command::Prompt {
    shape: shape.to_owned(),
    message: text("How much?"),
    to: Some(text("operator")),
    on: None,
  };
  let float = made(&life, asked("float")).await;
  life.send(Command::Close { id: float.clone(), value: text("2.0") }).await.unwrap();
  let got = until(&mut events, done(&float)).await;
  let Some(Event::Act(closed)) = got.last().map(|one| &*one.1) else { panic!("an act") };
  assert_eq!(closed.value().map(serde_json::Value::to_string), Some(text("2.0")));
  let declined = made(&life, asked("str")).await;
  let refusal = text(r#"{"is": "Refused", "args": ["Not now."]}"#);
  life.send(Command::Close { id: declined.clone(), value: refusal }).await.unwrap();
  let got = until(&mut events, done(&declined)).await;
  let Some(Event::Act(closed)) = got.last().map(|one| &*one.1) else { panic!("an act") };
  assert!(
    closed.failed() && closed.value().is_some_and(|why| why.to_string().contains("Not now."))
  );
  assert_eq!(life.send(Command::Seen { about: text("chain1") }).await.unwrap(), Reply::Done);
  let of = |kind: &str| Command::Transcript { on: None, of: Some(text(kind)) };
  let Reply::Values(seen) = life.send(of("seen")).await.unwrap() else { panic!("values") };
  assert_eq!(
    seen.iter().map(|fact| (&fact[1], &fact[2])).collect::<Vec<_>>(),
    [(&json!("chain1"), &json!("operator"))]
  );
}

#[tokio::test]
async fn the_life_gives_the_meaning_of_the_acts_that_it_told() {
  let at = place("meaning");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let prompt = Command::Prompt {
    shape: "bool".to_owned(),
    message: "Merge it?".to_owned(),
    to: Some("operator".to_owned()),
    on: None,
  };
  let asked = made(&life, prompt).await;
  until(&mut events, stands("chain1".to_owned(), State::Asks)).await;
  let Reply::Value(desk) = life.send(Command::Desk { notices: vec![] }).await.unwrap() else {
    panic!("the desk is a value")
  };
  assert_eq!(desk["questions"], json!(1), "{desk}");
  assert_eq!(desk["asks"][0]["id"], json!(asked));
  assert_eq!(desk["asks"][0]["choices"], json!(["yes", "no"]));
  let threads = Command::Threads { on: None, at: None };
  let Reply::Value(now) = life.send(threads).await.unwrap() else { panic!("a value") };
  assert_eq!(now["state"], json!("asks"), "{now}");
  let Reply::Values(moments) = life.send(Command::Moments).await.unwrap() else {
    panic!("the moments are values")
  };
  let first = moments.first().and_then(serde_json::Value::as_u64).expect("a moment");
  let Reply::Values(then) = life.send(Command::AsOf { at: first }).await.unwrap() else {
    panic!("the acts then are values")
  };
  assert!(then.iter().all(|act| act["began"].as_u64() <= Some(first)), "{then:?}");
  assert!(!then.iter().any(|act| act["id"] == json!(asked)), "{then:?}");
  life.send(Command::Answer { id: asked, line: "yes".to_owned() }).await.unwrap();
  until(&mut events, stands("chain1".to_owned(), State::Done)).await;
  let Reply::Value(desk) = life.send(Command::Desk { notices: vec![] }).await.unwrap() else {
    panic!("the desk is a value")
  };
  assert_eq!(desk["questions"], json!(0), "{desk}");
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

/// A follow-up of an entry on the root, with a message that names it.
fn follow(entry: &str) -> Command {
  let message = format!("Then {entry}.");
  Command::Queue { entry: entry.to_owned(), shape: "None".to_owned(), message, on: None }
}

/// Whether an event tells the follow-ups that wait on the root, by their entries.
fn waits(entries: &'static [&'static str]) -> impl Fn(&Event) -> bool {
  move |event| {
    matches!(event, Event::Queued { chain, waiting } if chain == "chain1"
      && waiting.iter().map(|one| one.entry.as_str()).eq(entries.iter().copied()))
  }
}

/// The facts of the queue of the operator on the root.
async fn queue_facts(life: &LifeHandle) -> Vec<serde_json::Value> {
  let of = Command::Transcript { on: None, of: Some("queue".to_owned()) };
  match life.send(of).await {
    Ok(Reply::Values(facts)) => facts,
    other => panic!("the transcript gave no values: {other:?}"),
  }
}

/// The steps of the facts of the queue of the operator on the root, each with its entry.
async fn steps(life: &LifeHandle) -> Vec<(String, String)> {
  let facts = queue_facts(life).await;
  let text = |fact: &serde_json::Value, at: usize| fact[at].as_str().unwrap_or_default().to_owned();
  facts.iter().map(|fact| (text(fact, 3), text(fact, 4))).collect()
}

/// The prompt that a follow-up went as, which its fact `sent` names.
async fn went(life: &LifeHandle, entry: &str) -> String {
  let facts = queue_facts(life).await;
  let sent = facts.iter().find(|fact| fact[3] == json!("sent") && fact[4] == json!(entry));
  match sent.and_then(|fact| fact[5].as_str()) {
    Some(act) => act.to_owned(),
    None => panic!("the follow-up {entry} went as no prompt: {facts:?}"),
  }
}

#[tokio::test]
async fn a_follow_up_waits_while_its_chain_asks_the_operator_and_goes_once_it_rests() {
  let at = place("queue-asks");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let prompt = Command::Prompt {
    shape: "str".to_owned(),
    message: "Which base?".to_owned(),
    to: Some("operator".to_owned()),
    on: None,
  };
  let asked = made(&life, prompt).await;
  until(&mut events, stands("chain1".to_owned(), State::Asks)).await;
  assert_eq!(life.send(follow("q1")).await.unwrap(), Reply::Done);
  assert_eq!(life.send(follow("q1")).await.unwrap(), Reply::Done);
  until(&mut events, waits(&["q1"])).await;
  let Reply::Values(waiting) = life.send(Command::Queued { on: None }).await.unwrap() else {
    panic!("values")
  };
  assert_eq!(waiting.len(), 1);
  assert_eq!(waiting[0]["message"], json!("Then q1."));
  life.send(Command::Answer { id: asked, line: "main".to_owned() }).await.unwrap();
  until(&mut events, waits(&[])).await;
  let sent = went(&life, "q1").await;
  until(&mut events, done(&sent)).await;
  let pairs = |all: &[&str]| all.iter().map(|step| ((*step).to_owned(), "q1".to_owned())).collect();
  let expected: Vec<(String, String)> = pairs(&["add", "begin", "sent"]);
  assert_eq!(steps(&life).await, expected);
  let refused = life.send(Command::Unqueue { entry: "q1".to_owned() }).await;
  assert!(matches!(refused, Err(LifeError::Refused(_))), "{refused:?}");
}

#[tokio::test]
async fn a_pause_holds_the_queue_and_the_operator_takes_a_follow_up_out_of_it() {
  let at = place("queue-pause");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  life.send(Command::Pause { id: "chain1".to_owned(), why: None }).await.unwrap();
  life.send(follow("q1")).await.unwrap();
  life.send(follow("q2")).await.unwrap();
  until(&mut events, waits(&["q1", "q2"])).await;
  life.send(Command::Unqueue { entry: "q1".to_owned() }).await.unwrap();
  until(&mut events, waits(&["q2"])).await;
  life.send(Command::Wake { id: "chain1".to_owned() }).await.unwrap();
  until(&mut events, waits(&[])).await;
  let sent = went(&life, "q2").await;
  until(&mut events, done(&sent)).await;
  let kinds: Vec<String> = steps(&life).await.into_iter().map(|(step, _)| step).collect();
  assert_eq!(kinds, ["add", "add", "drop", "begin", "sent"]);
}

#[tokio::test]
async fn a_reopen_of_the_record_finds_the_queue_and_sends_it_once_the_chain_rests() {
  let at = place("queue-reopen");
  let record = at.join("a.jsonl").display().to_string();
  let opening = || Opening { record: Some(record.clone()), ..scripted(&at, &[]) };
  let life = Life::spawn(opening()).unwrap();
  let mut events = Box::pin(life.events(0));
  life.send(Command::Pause { id: "chain1".to_owned(), why: None }).await.unwrap();
  life.send(follow("q1")).await.unwrap();
  until(&mut events, waits(&["q1"])).await;
  life.send(Command::End).await.unwrap();
  drop(events);
  let life = Life::spawn(opening()).unwrap();
  let mut events = Box::pin(life.events(0));
  until(&mut events, waits(&["q1"])).await;
  let Reply::Values(waiting) = life.send(Command::Queued { on: None }).await.unwrap() else {
    panic!("values")
  };
  assert_eq!(waiting.iter().map(|one| one["entry"].clone()).collect::<Vec<_>>(), [json!("q1")]);
  life.send(Command::Wake { id: "chain1".to_owned() }).await.unwrap();
  until(&mut events, waits(&[])).await;
  let sent = went(&life, "q1").await;
  until(&mut events, done(&sent)).await;
}
#[tokio::test]
async fn each_write_keeps_its_change_in_the_journal_beside_the_record_once_the_life_ends() {
  let at = place("journal");
  fs::write(at.join("a.txt"), "old\n").unwrap();
  fs::write(at.join("c.bin"), [0xff, 0xfe, 0x00]).unwrap();
  let record = at.join("a.jsonl").display().to_string();
  let opening = || Opening { record: Some(record.clone()), ..scripted(&at, &[]) };
  let life = Life::spawn(opening()).unwrap();
  let mut events = Box::pin(life.events(0));
  let word = "write(Text('a.txt', 'new\\n'))\nwrite(Text('b.txt', 'made\\n'))\n\
    write(Text('scratch://n', 'x'))\nwrite(Text('c.bin', 'text\\n'))";
  let id = made(&life, rung(word)).await;
  until(&mut events, done(&id)).await;
  // The end replies once the disk holds each change, so the test reads the journal at once.
  assert_eq!(life.send(Command::End).await.unwrap(), Reply::Done);
  let journal = PathBuf::from(format!("{record}.changes.jsonl"));
  let lines = fs::read_to_string(&journal).unwrap();
  let changes: Vec<serde_json::Value> =
    lines.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
  let file = |name: &str| at.join(name).display().to_string();
  assert_eq!(changes.len(), 3, "{lines}");
  assert_eq!(changes[0]["path"], json!(file("a.txt")));
  assert_eq!((&changes[0]["before"], &changes[0]["after"]), (&json!("old\n"), &json!("new\n")));
  assert!(changes[0]["act"].as_str().is_some_and(|act| act.starts_with("write")));
  assert_eq!(changes[0].get("made"), None);
  assert_eq!(changes[1]["path"], json!(file("b.txt")));
  assert_eq!((&changes[1]["before"], &changes[1]["made"]), (&json!(""), &json!(true)));
  // A file that cannot be read as text gives no empty text that an undo would put back.
  assert_eq!(changes[2]["path"], json!(file("c.bin")));
  assert_eq!((changes[2].get("made"), &changes[2]["after"]), (None, &json!("text\n")));
  assert!(changes[2]["unread"].as_str().is_some_and(|why| !why.is_empty()), "{lines}");
  drop(events);
  // A later life on the record makes no change again: it writes no file.
  let again = Life::spawn(opening()).unwrap();
  let mut events = Box::pin(again.events(0));
  let id = made(&again, rung("x = 1")).await;
  until(&mut events, done(&id)).await;
  assert_eq!(again.send(Command::End).await.unwrap(), Reply::Done);
  assert_eq!(fs::read_to_string(&journal).unwrap(), lines);
}
#[tokio::test]
async fn a_verb_of_the_operator_gives_the_words_of_its_act() {
  let at = place("words");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let given = Some(Some("hi\n".to_owned()));
  let bash = made(&life, Command::Bash { command: "cat".to_owned(), stdin: given, on: None }).await;
  until(&mut events, done(&bash)).await;
  let Reply::Value(exit) = life.send(Command::Peek { id: bash }).await.unwrap() else {
    panic!("a command comes to a value")
  };
  assert!(exit.to_string().contains("hi"), "{exit}");
  let fed =
    made(&life, Command::Bash { command: "cat".to_owned(), stdin: Some(None), on: None }).await;
  let stdin =
    Command::Write { path: format!("{fed}/stdin"), content: "fed\n".to_owned(), on: None };
  life.send(stdin).await.unwrap();
  let close = Command::Write { path: format!("{fed}/stdin"), content: String::new(), on: None };
  life.send(close).await.unwrap();
  until(&mut events, done(&fed)).await;
  let Reply::Value(exit) = life.send(Command::Peek { id: fed }).await.unwrap() else {
    panic!("a command comes to a value")
  };
  assert!(exit.to_string().contains("fed"), "{exit}");
  let bound = made(&life, rung("x = 7")).await;
  until(&mut events, done(&bound)).await;
  let label = Some("copy".to_owned());
  let fork =
    made(&life, Command::Chain { label, source: Some("chain1".to_owned()), on: None }).await;
  let Reply::Value(program) = life.send(Command::Program { on: Some(fork) }).await.unwrap() else {
    panic!("the program is a value")
  };
  assert_eq!(program[bound.as_str()], json!("x = 7"));
  let why = Some("for lunch".to_owned());
  life.send(Command::Pause { id: "chain1".to_owned(), why }).await.unwrap();
  let paused = |event: &Event| {
    matches!(event, Event::Act(act) if act.id == "chain1"
      && act.pause.as_ref().is_some_and(|one| one.why == Some(json!("for lunch"))))
  };
  until(&mut events, paused).await;
  // The scratch directory that a command of the life made goes as the thread of the life ends.
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

/// A life that keeps its record at a place, with no extension, whose model closes each request.
fn kept(at: &Path) -> Opening {
  Opening { record: Some(at.join("record.jsonl").display().to_string()), ..scripted(at, &[]) }
}

/// The facts of a log, in order, but each that says the record kept an entry, which the log of a
/// record holds as its place and no reader of a later life hears.
fn facts_of(log: &[Logged]) -> Vec<serde_json::Value> {
  let facts = log.iter().filter_map(|one| match &*one.1 {
    Event::Fact(fact) if fact[0] != "keep" => Some(fact.clone()),
    _ => None,
  });
  facts.collect()
}

/// The facts of one birth of a life on a record: a word on its root, then each command, then its
/// end, which keeps its checkpoint.
async fn birth(at: &Path, word: &str, commands: Vec<Command>) -> Vec<serde_json::Value> {
  let Ok(life) = Life::spawn(kept(at)) else { panic!("the life has no thread") };
  let mut events = Box::pin(life.events(0));
  let id = made(&life, rung(word)).await;
  let mut log = until(&mut events, done(&id)).await;
  for command in commands {
    let sent = life.send(command).await;
    assert!(matches!(sent, Ok(Reply::Done)), "{sent:?}");
  }
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  log.extend(until(&mut events, |event| matches!(event, Event::Ended(_))).await);
  facts_of(&log)
}

/// That a later log starts with each fact of an earlier one, each as many times, with each fact
/// that one holds more times than the other when they part.
fn retells(later: &[serde_json::Value], earlier: &[serde_json::Value], what: &str) {
  let counted = |facts: &[serde_json::Value]| {
    let mut counts = std::collections::BTreeMap::<String, i64>::new();
    for one in facts {
      *counts.entry(one.to_string()).or_default() += 1;
    }
    counts
  };
  let (mut parted, theirs) = (counted(earlier), counted(&later[..earlier.len().min(later.len())]));
  for (fact, n) in theirs {
    *parted.entry(fact).or_default() -= n;
  }
  parted.retain(|_, n| *n != 0);
  let parted: Vec<String> = parted.iter().map(|(fact, n)| format!("{n:+} {fact:.200}")).collect();
  assert!(parted.is_empty(), "{what}, but for these facts, earlier less later: {parted:#?}");
}

#[tokio::test]
async fn a_life_on_its_checkpoints_tells_each_repeated_fact_of_its_history_in_order() {
  let at = place("repeated");
  let (pause, wake) = (
    || Command::Pause { id: "chain2".to_owned(), why: Some("hold".to_owned()) },
    || Command::Wake { id: "chain2".to_owned() },
  );
  let word = "w = chain('w')\npause(w, 'hold'); wake(w); pause(w, 'hold'); wake(w)";
  let first = birth(&at, word, vec![pause(), wake()]).await;
  let second = birth(&at, "z = 0", vec![pause(), wake()]).await;
  let third = birth(&at, "y = 1", vec![pause(), wake()]).await;
  let controls = |facts: &[serde_json::Value]| -> Vec<(String, String)> {
    let controls = facts.iter().filter(|one| {
      one[1] == "chain2" && ["pause", "wake"].contains(&one[0].as_str().unwrap_or_default())
    });
    controls.map(|one| (one[0].to_string(), one[2].to_string())).collect()
  };
  let pauses =
    |facts: &[serde_json::Value]| controls(facts).iter().filter(|one| one.0 == "\"pause\"").count();
  assert_eq!(pauses(&first), 3, "{:?}", controls(&first));
  // Each later birth tells first the whole history of the births before it, with each fact that
  // repeats an earlier one, as it was told, and no fact that only a checkpoint restates.
  retells(&second, &first, "the second birth retells the first whole");
  assert_eq!(third[..first.len()], second[..first.len()], "each birth retells the first alike");
  let times = |facts: &[serde_json::Value], one: &serde_json::Value| {
    facts.iter().filter(|other| *other == one).count()
  };
  // The second and the third births do alike, so each adds as many occurrences of each fact of
  // the first: a restatement told again would add more, and a repeat dropped fewer.
  let added =
    |later: &[serde_json::Value], earlier: &[serde_json::Value], one: &serde_json::Value| {
      i64::try_from(times(later, one)).unwrap() - i64::try_from(times(earlier, one)).unwrap()
    };
  let again: Vec<String> = first
    .iter()
    .filter(|one| added(&third, &second, one) != added(&second, &first, one))
    .map(|one| format!("{:.200}", one.to_string()))
    .collect();
  assert!(again.is_empty(), "the births add these facts unlike: {again:#?}");
  assert_eq!(pauses(&second), 4, "{:?}", controls(&second));
  assert_eq!(pauses(&third), 5, "{:?}", controls(&third));
  let (later, earlier) = (controls(&third), controls(&second));
  assert_eq!(later[..earlier.len()], earlier[..], "the pauses and wakes keep their order");
}
