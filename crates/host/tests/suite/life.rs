//! A life on its thread, as a client of the host holds it: its commands, its events in order, and
//! its end.
#![expect(clippy::indexing_slicing, reason = "a panic is how a test fails")]
#![expect(clippy::string_slice, reason = "a panic is how a test fails")]
#![expect(clippy::panic, reason = "a panic is how a test fails")]
#![expect(clippy::unreachable, reason = "a panic is how a test fails")]
#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

use crate::yard;

use std::{
  collections::BTreeMap,
  env,
  fmt::Write as _,
  fs,
  path::{MAIN_SEPARATOR, Path, PathBuf},
  process,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc::channel,
  },
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use furb::{
  Object,
  life::{Answer, Opening},
  view,
  view::{Act, Kind, Line, Mark, Phase, Reach, Retrying, RunState, State},
  world::{
    self, SchemaError, Scratch,
    config::{Change, Layer},
  },
};
use furb_host::{Command, Door, Event, Life, LifeError, LifeHandle, Reply, Seq};
use futures::{Stream, StreamExt, future};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::{task, time};

/// The events of a life, each under its place in the log.
pub(crate) type Logged = (Seq, Arc<Event>);

/// A directory of one test, empty.
pub(crate) fn place(name: &str) -> yard::Yard {
  yard::Yard::new(format!("furb-host-{name}-{}", process::id()))
}

/// A path as the World writes it for a model: `~` in place of the home of the user that it stands
/// under, as the temporary folder of Windows does, and the path as it stands otherwise.
pub(crate) fn told(path: &Path) -> String {
  let home = env::home_dir().filter(|home| home.parent().is_some());
  match home.and_then(|home| path.strip_prefix(home).ok().map(Path::to_path_buf)) {
    Some(rest) => format!("~{MAIN_SEPARATOR}{}", rest.display()),
    None => path.display().to_string(),
  }
}

/// A life with no record and no skill, whose model answers each request with the next word of a
/// script, and with close(None) once the script runs out.
pub(crate) fn scripted(at: &Path, words: &[&str]) -> Opening {
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
    claude: Some(at.join("no-claude").display().to_string()),
    directory: at.display().to_string(),
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    skills: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}

/// The events of a stream, up to the first that the test waits for. A bound of time fails a test
/// that hangs.
pub(crate) async fn until(
  events: &mut (impl Stream<Item = Logged> + Unpin),
  wanted: impl Fn(&Event) -> bool,
) -> Vec<Logged> {
  let mut got = Vec::new();
  loop {
    let next = time::timeout(Duration::from_secs(60), events.next()).await;
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
pub(crate) fn done(id: &str) -> impl Fn(&Event) -> bool {
  move |event| matches!(event, Event::Act(act) if *act.id == *id && matches!(act.phase, Phase::Done(_)))
}

/// Whether an event is the act of a name, in a state.
fn stands(id: String, state: State) -> impl Fn(&Event) -> bool {
  move |event| matches!(event, Event::Act(act) if *act.id == *id && act.state == state)
}

/// The act that a command made.
pub(crate) async fn made(life: &LifeHandle, command: Command) -> String {
  match life.send(command).await {
    Ok(Reply::Act(id)) => id,
    other => panic!("the command made no act: {other:?}"),
  }
}

/// A rung of a word on the root.
pub(crate) fn rung(word: &str) -> Command {
  Command::Rung { word: word.to_owned(), on: None }
}

/// The act of a kind that a rung of a door made, once its fact is told.
pub(crate) async fn under(
  events: &mut (impl Stream<Item = Logged> + Unpin),
  rung: &str,
  kind: &str,
) -> String {
  let made = |event: &Event| matches!(event, Event::Fact(f) if f[0] == kind && f[2] == rung);
  let got = until(events, made).await;
  match got.last().map(|one| &*one.1) {
    Some(Event::Fact(fact)) if let Some(name) = fact[1].as_str() => name.to_owned(),
    other => panic!("no fact of a {kind}: {other:?}"),
  }
}

#[tokio::test]
async fn a_prompt_is_told_as_its_facts_and_its_acts_in_order_until_the_end() {
  let at = place("prompt");
  let life = Life::spawn(scripted(&at, &["close('ok')"])).unwrap();
  let mut events = Box::pin(life.events(0));
  let prompt =
    Command::Prompt { shape: "str".to_owned(), message: "Say ok.".to_owned(), to: None, on: None };
  let id = made(&life, prompt).await;
  let got = until(&mut events, done(&id)).await;
  let places: Vec<Seq> = got.iter().map(|one| one.0).collect();
  assert_eq!(places[0], 0);
  assert!(places.windows(2).all(|two| two[0] < two[1]), "{places:?}");
  let phases: Vec<&Phase> = got
    .iter()
    .filter_map(|one| match &*one.1 {
      Event::Act(act) if *act.id == *id => Some(&act.phase),
      _ => None,
    })
    .collect();
  assert_eq!(phases, [&Phase::Asked, &Phase::Started, &Phase::Done(json!("ok"))]);
  let asked =
    got.iter().position(|one| matches!(&*one.1, Event::Fact(fact) if fact[1] == json!(id)));
  let act = got.iter().position(|one| matches!(&*one.1, Event::Act(act) if *act.id == *id));
  assert!(asked.unwrap() < act.unwrap());
  assert!(got.iter().any(|one| *one.1 == Event::Changed("chain1".to_owned())));
  assert_eq!(life.send(Command::End).await.unwrap(), Reply::Done);
  let rest = until(&mut events, |event| matches!(event, Event::Ended(_))).await;
  assert_eq!(*rest.last().unwrap().1, Event::Ended(None));
  assert_eq!(rest[0].0, places[places.len() - 1] + 1);
  assert!(events.next().await.is_none());
  assert!(matches!(life.send(Command::Standing).await, Err(LifeError::Ended)));
}

#[tokio::test]
async fn a_reader_that_stands_behind_the_life_reads_its_events_again_from_the_log() {
  let at = place("behind");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut behind = Box::pin(life.events(0));
  let mut ahead = Box::pin(life.events(0));
  let id = made(&life, rung("for n in range(300):\n  chance()")).await;
  let heard = until(&mut ahead, done(&id)).await;
  assert!(heard.len() > 600, "{} events", heard.len());
  assert_eq!(until(&mut behind, done(&id)).await, heard);
  let mut late = Box::pin(life.events(0));
  assert_eq!(until(&mut late, done(&id)).await, heard);
  let last = heard.last().map_or(0, |one| one.0);
  let mut tail = Box::pin(life.events(last));
  assert_eq!(until(&mut tail, done(&id)).await, heard[heard.len() - 1..]);
}

/// A fact that a command said while the reply of a command at the tip waited in the inbox stands
/// once: after the place where the reply was given, so in the events and not in the reply.
#[tokio::test]
async fn a_fact_said_while_a_reply_at_the_tip_waits_is_in_its_events_and_not_in_the_reply() {
  let at = place("after");
  // The model holds the thread of the life from its first request until the test drops `hold`.
  let (hold, gate) = channel::<()>();
  let (began, entered) = channel::<()>();
  let (gate, began) = (Mutex::new(gate), Mutex::new(began));
  let inner = scripted(&at, &[]).answer.unwrap();
  let answer = Answer(Arc::new(move |kind, message| {
    began.lock().unwrap().send(()).ok();
    gate.lock().unwrap().recv().ok();
    (inner.0)(kind, message)
  }));
  let opening = Opening { answer: Some(answer), ..scripted(&at, &[]) };
  let life = Life::spawn(opening).unwrap();
  made(&life, rung("prompt(None, 'go', on=chain('side'))")).await;
  let began = task::spawn_blocking(move || entered.recv_timeout(Duration::from_secs(60)));
  assert!(began.await.unwrap().is_ok(), "the model of the life holds its thread");
  let of = Some("rung".to_owned());
  let mut asked = Box::pin(life.after(Command::Transcript { on: None, of }));
  assert!(futures::poll!(&mut asked).is_pending(), "the thread holds the reply at the tip");
  let mut later = Box::pin(life.send(rung("y = 2")));
  assert!(futures::poll!(&mut later).is_pending(), "the thread holds the rung after it");
  drop(hold);
  let (replied, events) = asked.await.expect("the life gives the reply at its tip");
  let Ok(Reply::Act(id)) = later.await else { panic!("the rung is made") };
  let Ok(Reply::Values(rungs)) = replied else { panic!("the transcript is given") };
  assert!(!rungs.iter().any(|one| one[1] == json!(id)), "the reply holds no later rung: {rungs:?}");
  let mut events = Box::pin(events);
  let rung =
    |event: &Event| matches!(event, Event::Fact(fact) if fact[0] == "rung" && fact[1] == json!(id));
  until(&mut events, rung).await;
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
  let wrong = life.send(Command::Answer { id: id.clone(), line: "1".to_owned() }).await;
  assert!(
    matches!(wrong, Err(LifeError::Refused(why)) if why == format!("No prompt is named {id}."))
  );
}

#[tokio::test]
async fn the_name_of_the_session_is_the_last_that_the_operator_said_on_the_root_as_the_life_folds_it()
 {
  let at = place("named");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  assert_eq!(life.name(), None);
  for name in ["First", "Mine"] {
    let word =
      format!("act(\"rename_session\", \"\", lambda id: (yield \"done\", id, \"{name}\"))");
    let id = made(&life, rung(&word)).await;
    until(&mut events, done(&id)).await;
  }
  assert_eq!(life.name().as_deref(), Some("Mine"));
  let id =
    made(&life, rung("act('rename_session', 'chain2', lambda id: (yield 'done', id, 'Off'))"))
      .await;
  until(&mut events, done(&id)).await;
  assert_eq!(life.name().as_deref(), Some("Mine"));
}

#[tokio::test]
async fn the_configs_of_a_life_are_read_and_changed_through_its_commands_and_a_change_stands_it() {
  let at = place("configs");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let key = |names: &[&str]| names.iter().map(|one| (*one).to_owned()).collect::<Vec<_>>();
  let thinking =
    Change { key: key(&["max_thinking_tokens"]), value: Some(json!(0)), layer: Some(Layer::Local) };
  let Reply::Act(stand) = life.send(Command::Configure(thinking)).await.unwrap() else {
    panic!("a change gives the act of the stand that follows it")
  };
  assert!(stand.starts_with("stand"), "{stand}");
  // The life says the key and the value that wins for it, which each kit hears.
  let said = |event: &Event| matches!(event, Event::Fact(fact) if fact[0] == "configured");
  let told = until(&mut events, said).await;
  let Event::Fact(fact) = &*told.last().unwrap().1 else { unreachable!() };
  assert_eq!((&fact[3], &fact[4]), (&json!(["max_thinking_tokens"]), &json!(0)), "{fact}");
  let Reply::Values(settings) = life.send(Command::Settings).await.unwrap() else {
    panic!("the settings are values")
  };
  let thinking = settings.iter().find(|one| one["key"] == json!(["max_thinking_tokens"])).unwrap();
  assert_eq!((&thinking["value"], &thinking["layer"]), (&json!(0), &json!("local")));
  assert_eq!(thinking["property"]["type"], json!("integer"), "the schema names the key");
  let wrong = Change {
    key: key(&["providers", "nothing"]),
    value: Some(json!(true)),
    layer: Some(Layer::Project),
  };
  let refused = life.send(Command::Configure(wrong)).await;
  assert!(
    matches!(&refused, Err(LifeError::Refused(why)) if why.contains("nothing")),
    "{refused:?}"
  );
  assert!(!at.join(".furb/config.json").exists(), "a refused change leaves no config");
  fs::remove_dir_all(&at).unwrap();
}

#[tokio::test]
async fn the_skills_of_a_life_say_how_each_plays_and_a_change_that_turns_one_on_plays_it() {
  let at = place("plays");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let web = async || {
    let Reply::Skills(all) = life.send(Command::Plays).await.unwrap() else {
      panic!("the skills are skills")
    };
    let one = all.into_iter().find(|one| one.name == "web").unwrap();
    (one.plays, one.on, one.from, one.turned_by)
  };
  let furb = || "furb".to_owned();
  assert_eq!(web().await, (false, false, furb(), json!(true)));
  let turned = |on: bool| Change {
    key: vec!["skills".to_owned(), "web".to_owned()],
    value: Some(json!(on)),
    layer: None,
  };
  life.send(Command::Configure(turned(true))).await.unwrap();
  // The change plays it at once, and a name of `skills` takes the local config.
  assert_eq!(web().await, (true, true, furb(), json!(true)));
  let local = fs::read_to_string(at.join(".furb/config.local.json")).unwrap();
  assert_eq!(
    serde_json::from_str::<serde_json::Value>(&local).unwrap(),
    json!({"skills": {"web": true}})
  );
  // One turned off plays on until the life ends.
  life.send(Command::Configure(turned(false))).await.unwrap();
  assert_eq!(web().await, (true, false, furb(), json!(true)));
  fs::remove_dir_all(&at).unwrap();
}

#[tokio::test]
async fn a_life_that_does_not_open_ends_with_its_fault() {
  let at = place("fault");
  let opening = Opening { record: Some(at.display().to_string()), ..scripted(&at, &[]) };
  let life = Life::spawn(opening).unwrap();
  let mut events = Box::pin(life.events(0));
  // A window that attaches at the tip, past each place of the log, hears the end too.
  let mut tip = Box::pin(life.events(Seq::MAX));
  let got = until(&mut events, |event| matches!(event, Event::Ended(_))).await;
  let Event::Ended(Some(fault)) = &*got.last().unwrap().1 else { panic!("the end has a fault") };
  assert_ne!(fault, "");
  assert!(events.next().await.is_none());
  let heard = time::timeout(
    Duration::from_secs(10),
    until(&mut tip, |event| matches!(event, Event::Ended(_))),
  );
  let heard = heard.await.expect("a reader at the tip hears the end of the life");
  assert!(matches!(&*heard.last().unwrap().1, Event::Ended(Some(why)) if why == fault));
  // A command that reaches the life after its end hears the fault that ended it.
  let refused = life.send(Command::Standing).await;
  assert!(matches!(&refused, Err(LifeError::Refused(why)) if why == fault), "{refused:?}");
}
/// A record from before the stable cut opens in no life: the life tells each of its facts and
/// ends with why, and a window reads its threads from the fold of those facts, read only, while
/// each other verb hears why; nothing writes on the record.
#[tokio::test]
async fn a_record_from_before_the_stable_cut_shows_its_threads_to_a_window_and_says_why() {
  let at = place("before-cut");
  let record = at.join("record.jsonl");
  let before = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/replay/speaker.jsonl");
  fs::copy(before, &record).unwrap();
  let text = fs::read_to_string(&record).unwrap();
  let opening = Opening { record: Some(record.display().to_string()), ..scripted(&at, &[]) };
  let life = Life::spawn(opening).unwrap();
  let mut events = Box::pin(life.events(0));
  let got = until(&mut events, |event| matches!(event, Event::Ended(_))).await;
  let Event::Ended(Some(why)) = &*got.last().unwrap().1 else { panic!("the end says why") };
  assert_eq!(*why, SchemaError::BeforeCut { of: "record" }.to_string());
  let told = got.iter().filter(|one| matches!(&*one.1, Event::Fact(_))).count();
  let facts = world::retold(&record).unwrap().facts().unwrap().len();
  assert_eq!(told, facts, "the life tells each fact of the record");
  let threads = life.send(Command::Threads { on: None, at: None, read: None, site: None }).await;
  let Ok(Reply::Threads(threads)) = threads else { panic!("the threads: {threads:?}") };
  let prompts: Vec<&str> = threads.threads.iter().map(|one| one.thread.prompt.as_str()).collect();
  assert_eq!(prompts, ["prompt1", "prompt3"], "a window reads the threads of the record");
  let reach = Reach { before: None, limit: 10 };
  let page = life.send(Command::Thread { thread: "prompt1".to_owned(), reach, at: None }).await;
  let Ok(Reply::Page(page)) = page else { panic!("a page of the thread: {page:?}") };
  let lines: Vec<&str> = page.exchanges[0]
    .block
    .iter()
    .flat_map(|one| &one.lines)
    .map(|one| one.text.as_str())
    .collect();
  assert_eq!(lines[0], "b = await bash('echo a')", "a window reads the word of the thread");
  let refused = life.send(Command::Standing).await;
  assert!(matches!(&refused, Err(LifeError::Refused(no)) if no == why), "{refused:?}");
  assert_eq!(fs::read_to_string(&record).unwrap(), text, "nothing writes on the record");
}

#[tokio::test]
async fn each_act_is_told_with_its_meaning_and_each_chain_with_the_state_of_its_work() {
  let at = place("meaning");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let raised = made(&life, rung("BAD")).await;
  let got = until(&mut events, stands(raised.clone(), State::Failed)).await;
  let Some(Event::Act(failed)) = got.last().map(|one| &*one.1) else { panic!("an act") };
  let why = RunState::Failed("NameError: name 'BAD' is not defined".to_owned());
  assert_eq!(failed.run, Some(why));
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
  let chain = |one: &&Logged| matches!(&*one.1, Event::Act(act) if *act.id == *"chain1");
  let Some(Event::Act(root)) = held.iter().rfind(chain).map(|one| &*one.1) else {
    panic!("an act")
  };
  assert_eq!(root.state, State::Paused);
}
#[tokio::test]
async fn the_verbs_of_the_operator_act_on_the_life_as_a_word_of_his_would() {
  let at = place("operator");
  // A grant loads the official extension that defines it, so this life runs the skills.
  let life = Life::spawn(Opening { skills: Some(true), ..scripted(&at, &[]) }).unwrap();
  let mut events = Box::pin(life.events(0));
  let text = |path: &str| path.to_owned();
  let door = |door: Door| rung(&door.word());
  let write =
    made(&life, door(Door::Write { path: text("note.txt"), content: text("hi\n") })).await;
  until(&mut events, done(&write)).await;
  let read = made(&life, door(Door::Read { path: text("note.txt") })).await;
  let read = under(&mut events, &read, "read").await;
  until(&mut events, done(&read)).await;
  let Reply::Value(got) = life.send(Command::Peek { id: read }).await.unwrap() else {
    panic!("a read comes to a value")
  };
  assert!(got.to_string().contains("hi\\n"), "{got}");
  let cat = Door::Bash { command: text("cat note.txt"), stdin: Some(String::new()) };
  let bash = made(&life, door(cat)).await;
  let bash = under(&mut events, &bash, "bash").await;
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
  let grant = made(&life, door(Door::Grant { usd: Some(2.0), share: None })).await;
  until(&mut events, done(&grant)).await;
  let other = Kind::Other("grant".to_owned());
  until(&mut events, |event| matches!(event, Event::Act(act) if act.kind == other)).await;
  let stand = made(&life, door(Door::Stand)).await;
  let stand = under(&mut events, &stand, "stand").await;
  until(&mut events, done(&stand)).await;
  // The scratch directory that a command of the life made goes as the thread of the life ends.
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

#[tokio::test]
async fn each_door_of_a_person_runs_as_his_word_that_the_transcript_shows() {
  let at = place("doors");
  fs::create_dir_all(at.join("work")).unwrap();
  let life = Life::spawn(scripted(&at, &["x = 1\nclose('one')"])).unwrap();
  let mut events = Box::pin(life.events(0));
  let text = |one: &str| one.to_owned();
  let asked = Command::Prompt { shape: text("str"), message: text("Set x."), to: None, on: None };
  let prompt = made(&life, asked).await;
  until(&mut events, done(&prompt)).await;
  let doors = [
    Door::Chain { label: text("bench") },
    Door::Fork { label: text("rewind"), source: text("chain1"), kept: Some(vec![prompt.clone()]) },
    Door::Cd { path: text("work") },
    Door::Actor { actor: text("claude-cli:opus/low") },
    Door::Write { path: prompt.clone(), content: text("x = 2\nclose('two')") },
  ];
  let mut ran = Vec::new();
  for door in &doors {
    let rung = made(&life, rung(&door.word())).await;
    until(&mut events, done(&rung)).await;
    ran.push(rung);
  }
  let Reply::Values(facts) = life.send(Command::Transcript { on: None, of: None }).await.unwrap()
  else {
    panic!("the transcript is values")
  };
  // Each door is a rung of his word on the chain, as the rung of a word of a model is.
  for (rung, door) in ran.iter().zip(&doors) {
    let told =
      facts.iter().any(|fact| fact[0] == "rung" && fact[1] == **rung && fact[4] == door.word());
    assert!(told, "{rung} tells {} in {facts:?}", door.word());
  }
  let chains: Vec<&serde_json::Value> = facts.iter().filter(|fact| fact[0] == "chain").collect();
  let labels: Vec<&serde_json::Value> = chains.iter().map(|fact| &fact[4]).collect();
  assert_eq!(labels, [&json!("bench"), &json!("rewind")]);
  assert_eq!(chains[1][5], json!("chain1"), "the rewind stands on the chain, which stays");
  let cwd = life.send(Command::Cwd { on: None }).await.unwrap();
  assert_eq!(cwd, Reply::Value(json!(told(&at.join("work")))));
  let inspect = |name: &str| Command::Inspect { name: text(name), on: None };
  let Reply::Inspected { repr, .. } = life.send(inspect("actor")).await.unwrap() else {
    panic!("a name is inspected")
  };
  assert_eq!(repr, "'claude-cli:opus/low'");
  // The write of the door of the prompt runs its program again from the word that differs.
  let Reply::Inspected { repr, .. } = life.send(inspect("x")).await.unwrap() else {
    panic!("a name is inspected")
  };
  assert_eq!(repr, "2");
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
  let Reply::Desk(desk) = life.send(Command::Desk { notices: vec![] }).await.unwrap() else {
    panic!("the desk is a desk")
  };
  let desk = serde_json::to_value(desk).unwrap();
  assert_eq!(desk["questions"], json!(1), "{desk}");
  assert_eq!(desk["asks"][0]["id"], json!(asked));
  assert_eq!(desk["asks"][0]["choices"], json!(["yes", "no"]));
  let threads = Command::Threads { on: None, at: None, read: None, site: None };
  let Reply::Threads(now) = life.send(threads).await.unwrap() else { panic!("the threads") };
  assert_eq!(now.state, State::Asks, "{now:?}");
  let Reply::Moments(moments) = life.send(Command::Moments { on: None }).await.unwrap() else {
    panic!("the moments")
  };
  let first = *moments.first().expect("a moment");
  let past = Command::Threads { on: None, at: Some(first), read: None, site: None };
  let Reply::Threads(back) = life.send(past).await.unwrap() else { panic!("the threads") };
  assert_ne!(back.state, State::Asks, "the chain asked nothing at its first moment: {back:?}");
  let Reply::Acts(then) = life.send(Command::AsOf { at: first }).await.unwrap() else {
    panic!("the acts then")
  };
  assert!(then.iter().all(|act| act.began <= first), "{then:?}");
  assert!(!then.iter().any(|act| *act.id == *asked), "{then:?}");
  life.send(Command::Answer { id: asked, line: "yes".to_owned() }).await.unwrap();
  until(&mut events, stands("chain1".to_owned(), State::Done)).await;
  let Reply::Desk(desk) = life.send(Command::Desk { notices: vec![] }).await.unwrap() else {
    panic!("the desk is a desk")
  };
  assert_eq!(serde_json::to_value(desk).unwrap()["questions"], json!(0));
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
async fn a_follow_up_on_a_thread_waits_on_its_chain_and_goes_as_a_message_of_the_thread() {
  let at = place("queue-thread");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let prompt = Command::Prompt {
    shape: "str".to_owned(),
    message: "Which base?".to_owned(),
    to: Some("operator".to_owned()),
    on: None,
  };
  let thread = made(&life, prompt).await;
  until(&mut events, stands("chain1".to_owned(), State::Asks)).await;
  let queue = Command::Queue {
    entry: "t1".to_owned(),
    shape: "None".to_owned(),
    message: "Then rebase.".to_owned(),
    on: Some(thread.clone()),
  };
  assert_eq!(life.send(queue).await.unwrap(), Reply::Done);
  // The follow-up waits on the chain of the thread, and names the thread.
  let told = until(&mut events, waits(&["t1"])).await;
  let Event::Queued { waiting, .. } = &*told.last().unwrap().1 else { unreachable!() };
  assert_eq!(waiting[0].thread.as_deref(), Some(thread.as_str()));
  life.send(Command::Answer { id: thread.clone(), line: "main".to_owned() }).await.unwrap();
  until(&mut events, waits(&[])).await;
  let sent = went(&life, "t1").await;
  let Reply::Value(act) = life.send(Command::Get { id: sent.clone() }).await.unwrap() else {
    panic!("the act of the follow-up is a value")
  };
  assert_eq!(act[3], json!(thread), "the message stands on the thread: {act}");
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
  // A later life on the record makes no change again, by its image or by the record that it
  // replays: it writes no file.
  for image in [true, false] {
    if !image {
      fs::remove_file(format!("{record}.image")).unwrap();
    }
    let again = Life::spawn(opening()).unwrap();
    let mut events = Box::pin(again.events(0));
    let id = made(&again, rung("x = 1")).await;
    until(&mut events, done(&id)).await;
    assert_eq!(again.send(Command::End).await.unwrap(), Reply::Done);
    assert_eq!(fs::read_to_string(&journal).unwrap(), lines);
  }
  // The scratch directory of the record, with the file that the life wrote, stands in the cache.
  let scratch = Scratch::of(Path::new(&record)).unwrap().unwrap();
  fs::remove_dir_all(scratch.path()).unwrap();
}

#[tokio::test]
async fn a_journal_that_the_disk_refuses_is_told_once_as_it_fails_and_once_as_it_recovers() {
  let at = place("lost");
  let notices = |got: &[Logged], says: &str| {
    got.iter().filter(|one| matches!(&*one.1, Event::Notice(text) if text.contains(says))).count()
  };
  let notice = |says: &'static str| move |event: &Event| matches!(event, Event::Notice(text) if text.contains(says));
  let write = |name: &str| rung(&format!("write(Text('{name}', 'x\\n'))"));
  // A folder in the place of the journal makes the disk refuse each change.
  let refused = |name: &str| {
    let record = at.join(name).display().to_string();
    let journal = PathBuf::from(format!("{record}.changes.jsonl"));
    fs::create_dir(&journal).unwrap();
    (Life::spawn(Opening { record: Some(record), ..scripted(&at, &[]) }).unwrap(), journal)
  };
  let lost = |end: &Result<Reply, LifeError>, journal: &Path, count: usize| {
    let says = format!("the journal of file changes at {} lost {count} of them", journal.display());
    matches!(end, Err(LifeError::Refused(why)) if why.starts_with(&format!("{says}, the first for: ")))
  };
  let (life, journal) = refused("a.jsonl");
  let mut events = Box::pin(life.events(0));
  let id = made(&life, write("a.txt")).await;
  let mut got = until(&mut events, done(&id)).await;
  let id = made(&life, write("b.txt")).await;
  got.extend(until(&mut events, done(&id)).await);
  // The end replies once the journal took each change, so it says each change that it lost.
  let end = life.send(Command::End).await;
  assert!(lost(&end, &journal, 2), "{end:?}");
  got.extend(until(&mut events, |event| matches!(event, Event::Ended(_))).await);
  assert_eq!(notices(&got, "took no change"), 1, "the life hears the first fault alone");
  let (life, journal) = refused("b.jsonl");
  let mut events = Box::pin(life.events(0));
  made(&life, write("c.txt")).await;
  until(&mut events, notice("took no change")).await;
  fs::remove_dir(&journal).unwrap();
  made(&life, write("d.txt")).await;
  let got = until(&mut events, notice("takes changes again")).await;
  assert_eq!(notices(&got, "took no change"), 0, "the life hears the fault once");
  let end = life.send(Command::End).await;
  assert!(lost(&end, &journal, 1), "{end:?}");
  let kept = fs::read_to_string(&journal).unwrap();
  assert_eq!(kept.lines().count(), 1, "the journal keeps the change after its recovery: {kept}");
}

#[tokio::test]
async fn a_verb_of_the_operator_gives_the_words_of_its_act() {
  let at = place("words");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let door = |door: Door| rung(&door.word());
  let given = Door::Bash { command: "cat".to_owned(), stdin: Some("hi\n".to_owned()) };
  let bash = made(&life, door(given)).await;
  let bash = under(&mut events, &bash, "bash").await;
  until(&mut events, done(&bash)).await;
  let Reply::Value(exit) = life.send(Command::Peek { id: bash }).await.unwrap() else {
    panic!("a command comes to a value")
  };
  assert!(exit.to_string().contains("hi"), "{exit}");
  let fed = made(&life, door(Door::Bash { command: "cat".to_owned(), stdin: None })).await;
  let fed = under(&mut events, &fed, "bash").await;
  let stdin = Door::Write { path: format!("{fed}/stdin"), content: "fed\n".to_owned() };
  life.send(door(stdin)).await.unwrap();
  let close = Door::Write { path: format!("{fed}/stdin"), content: String::new() };
  life.send(door(close)).await.unwrap();
  until(&mut events, done(&fed)).await;
  let Reply::Value(exit) = life.send(Command::Peek { id: fed }).await.unwrap() else {
    panic!("a command comes to a value")
  };
  assert!(exit.to_string().contains("fed"), "{exit}");
  let bound = made(&life, rung("x = 7")).await;
  until(&mut events, done(&bound)).await;
  let copy = Door::Fork { label: "copy".to_owned(), source: "chain1".to_owned(), kept: None };
  let fork = made(&life, door(copy)).await;
  let fork = under(&mut events, &fork, "chain").await;
  let Reply::Value(program) = life.send(Command::Program { on: Some(fork) }).await.unwrap() else {
    panic!("the program is a value")
  };
  assert_eq!(program[bound.as_str()], json!("x = 7"));
  let why = Some("for lunch".to_owned());
  life.send(Command::Pause { id: "chain1".to_owned(), why }).await.unwrap();
  let paused = |event: &Event| {
    matches!(event, Event::Act(act) if *act.id == *"chain1"
      && act.pause.as_ref().is_some_and(|one| one.why == Some(json!("for lunch"))))
  };
  until(&mut events, paused).await;
  // The scratch directory that a command of the life made goes as the thread of the life ends.
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

/// A life that keeps its record at a place, with no skill, whose model closes each request.
pub(crate) fn kept(at: &Path) -> Opening {
  Opening { record: Some(at.join("record.jsonl").display().to_string()), ..scripted(at, &[]) }
}

/// The facts of a log, in order, but each that says the record kept an entry, which the log of a
/// record holds as its place and no reader of a later life hears.
pub(crate) fn facts_of(log: &[Logged]) -> Vec<serde_json::Value> {
  let facts = log.iter().filter_map(|one| match &*one.1 {
    Event::Fact(fact) if fact[0] != "keep" => Some(fact.clone()),
    _ => None,
  });
  facts.collect()
}

/// The record of a life that ended, as if another build had made its last segment: its last fact
/// made of this build names the engine of this build with a line of comment more, so the next life
/// that keeps it crosses there and each later open carries the life across that place. The image
/// no longer names the tip of the record, so the next life opens by the record. The `opening`
/// names the record.
pub(crate) async fn crossed(opening: Opening) {
  static ENGINES: AtomicUsize = AtomicUsize::new(0);
  let Some(record) = opening.record else { panic!("the life keeps no record") };
  let Ok(text) = fs::read_to_string(&record) else { panic!("the record reads") };
  let head = format!("[[\"made\",\"chain1\",\"store\",\"{}\"", world::engine());
  let Some(at) = text.rfind(&head) else { panic!("the record names this build") };
  let end = text[at..].find('\n').map_or(text.len(), |end| at + end);
  let Ok(mut entry) = serde_json::from_str::<serde_json::Value>(&text[at..end]) else {
    panic!("the fact made parses")
  };
  let Some(fact) = entry[0].as_array_mut() else { panic!("an entry holds its fact first") };
  let texts: Vec<String> =
    (7..10).map(|at| fact[at].as_str().unwrap_or_default().to_owned()).collect();
  // Each crossing names an engine of its own, as a record crosses only to texts that differ.
  let n = ENGINES.fetch_add(1, Ordering::Relaxed);
  let engine = format!("{}# Another engine, {n}.\n", texts[0]);
  let mut hash = Sha256::new();
  for one in [engine.as_str(), &texts[1], &texts[2]] {
    hash.update(one.as_bytes());
    hash.update([0]);
  }
  fact[3] = json!(format!("{:x}", hash.finalize()));
  fact[7] = json!(engine);
  let written = fs::write(&record, format!("{}{entry}{}", &text[..at], &text[end..]));
  assert!(written.is_ok(), "the record takes the fact made of another engine");
}

/// The facts of one birth of a life on a record: a word on its root, then each command, then its
/// end, and the crossing of the record, across which the next birth opens.
pub(crate) async fn birth(at: &Path, word: &str, commands: Vec<Command>) -> Vec<serde_json::Value> {
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
  crossed(kept(at)).await;
  facts_of(&log)
}

/// That a later log starts with each fact of an earlier one, each as many times, with each fact
/// that one holds more times than the other when they part.
fn retells(later: &[serde_json::Value], earlier: &[serde_json::Value], what: &str) {
  let counted = |facts: &[serde_json::Value]| {
    let mut counts = BTreeMap::<String, i64>::new();
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
async fn a_life_across_its_crossings_tells_each_repeated_fact_of_its_history_in_order() {
  let at = place("repeated");
  let (pause, wake) = (
    || Command::Pause { id: "chain2".to_owned(), why: Some("hold".to_owned()) },
    || Command::Wake { id: "chain2".to_owned() },
  );
  let word = "w = chain('w')\npause(w, 'hold'); wake(w); pause(w, 'hold'); wake(w)";
  // A birth tells the past that its replay says again under the speaker journal, and the past
  // before a crossing under the speaker that the record holds: a fact is the same fact under
  // either. The fact made of each birth names the engine that the test wrote in its place.
  let unsaid = |facts: Vec<serde_json::Value>| -> Vec<serde_json::Value> {
    let unsaid = facts.into_iter().filter(|one| one[0] != "made").map(|mut one| {
      one.as_array_mut().filter(|one| one.len() > 2).map(|one| one.remove(2));
      one
    });
    unsaid.collect()
  };
  let first = unsaid(birth(&at, word, vec![pause(), wake()]).await);
  let second = unsaid(birth(&at, "z = 0", vec![pause(), wake()]).await);
  let third = unsaid(birth(&at, "y = 1", vec![pause(), wake()]).await);
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
  // repeats an earlier one, as it was told, and no fact that only a carry restates.
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

#[tokio::test]
async fn a_life_across_a_crossing_tells_the_acts_of_its_history_each_at_the_place_of_its_fact() {
  let at = place("history");
  birth(&at, "x = 1", vec![]).await;
  let Ok(life) = Life::spawn(kept(&at)) else { panic!("the life has no thread") };
  let mut events = Box::pin(life.events(0));
  let id = made(&life, rung("y = 2")).await;
  let mut log = until(&mut events, done(&id)).await;
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  log.extend(until(&mut events, |event| matches!(event, Event::Ended(_))).await);
  // The place of an act is the count of the facts that the log told up to its question, but each
  // fact that takes no place in time, as a whole replay of the record says it.
  let (mut facts, mut acts) = (Vec::new(), Vec::new());
  for one in &log {
    match &*one.1 {
      Event::Fact(fact) if view::timed(fact) => facts.push(fact.clone()),
      Event::Act(act) => acts.push((**act).clone()),
      _ => {}
    }
  }
  let misplaced: Vec<String> = acts
    .iter()
    .filter(|act| {
      let at = usize::try_from(act.began).ok().and_then(|one| one.checked_sub(1));
      at.and_then(|one| facts.get(one)).is_none_or(|fact| fact[1] != act.id.as_ref())
    })
    .map(|act| format!("{} at {}", act.id, act.began))
    .collect();
  assert!(misplaced.is_empty(), "these acts stand at no fact of theirs: {misplaced:?}");
  let earlier = acts.iter().rev().find(|act| act.words.first() == Some(&json!("x = 1")));
  let done = earlier.is_some_and(|act| act.kind == Kind::Rung && Act::done(act));
  assert!(done, "the rung of the first birth is told done at last: {earlier:?}");
}

#[tokio::test]
async fn a_reader_that_attaches_late_reads_each_line_that_a_running_command_printed_so_far() {
  let at = place("outsofar");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut first = Box::pin(life.events(0));
  let command = "for i in $(seq 1 3000); do echo line$i; done; sleep 30".to_owned();
  let id = made(&life, rung(&Door::Bash { command, stdin: Some(String::new()) }.word())).await;
  let id = under(&mut first, &id, "bash").await;
  let last = |event: &Event| matches!(event, Event::Fact(f) if f[0] == "out" && f[3].as_str().is_some_and(|t| t.contains("line3000")));
  until(&mut first, last).await;
  let mut late = Box::pin(life.events(0));
  let got = until(&mut late, last).await;
  let printed: String = got
    .iter()
    .filter_map(|(_, event)| match &**event {
      Event::Fact(f) if f[0] == "out" && f[1] == id.as_str() && f[4] == "stdout" => f[3].as_str(),
      _ => None,
    })
    .collect();
  let mut wanted = String::new();
  for n in 1..=3000 {
    writeln!(wanted, "line{n}").unwrap();
  }
  assert_eq!(
    printed, wanted,
    "a reader at 0 reads the whole output so far, in the facts of the log"
  );
  let running = got.iter().any(
    |(_, e)| matches!(&**e, Event::Act(a) if *a.id == *id && !matches!(a.phase, Phase::Done(_))),
  );
  assert!(running, "the command still runs while the reader reads");
}
#[tokio::test]
async fn a_change_of_the_roster_offers_its_models_at_the_next_stand_of_the_life() {
  let at = place("roster");
  let life = Life::spawn(scripted(&at, &[])).unwrap();
  let mut events = Box::pin(life.events(0));
  let offered = async || {
    let Reply::Value(standing) = life.send(Command::Standing).await.unwrap() else {
      panic!("the standing is a value")
    };
    let names = standing[0].as_array().cloned().unwrap_or_default();
    names.iter().filter_map(|one| one[0].as_str().map(str::to_owned)).collect::<Vec<_>>()
  };
  assert!(!offered().await.contains(&"claude-cli:sonnet".to_owned()));
  let roster = vec!["claude-cli:opus".to_owned(), "claude-cli:sonnet".to_owned()];
  let stand = made(&life, Command::Roster(roster)).await;
  until(&mut events, done(&stand)).await;
  assert!(offered().await.contains(&"claude-cli:sonnet".to_owned()));
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

#[tokio::test]
async fn a_retry_of_a_model_shows_on_the_line_that_it_writes_and_its_failure_once_when_the_attempts_end()
 {
  let at = place("retry");
  let calls = AtomicUsize::new(0);
  let answer = Answer(Arc::new(move |_, _| {
    let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
    let why = if first { "the provider is overloaded" } else { "the request is malformed" };
    Box::pin(async move { Err(why.to_owned()) })
  }));
  let life = Life::spawn(Opening { answer: Some(answer), ..scripted(&at, &[]) }).unwrap();
  let mut events = Box::pin(life.events(0));
  let shape = "None".to_owned();
  let message = "Ground the branch.".to_owned();
  let now = || SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
  let before = now();
  let prompt = made(&life, Command::Prompt { shape, message, to: None, on: None }).await;
  let page = || async {
    let reach = Reach { before: None, limit: 10 };
    let Reply::Page(page) =
      life.send(Command::Thread { thread: prompt.clone(), reach, at: None }).await.unwrap()
    else {
      panic!("the page of a thread")
    };
    let block = page.exchanges[0].block.clone().expect("a block that answers the message");
    block.lines
  };
  let got = until(&mut events, |event| matches!(event, Event::Retried { .. })).await;
  let Some(Event::Retried { retry, .. }) = got.last().map(|one| &*one.1) else { unreachable!() };
  // The first retry waits four seconds, from a moment between the two reads of the clock.
  assert!((before + 4..=now() + 5).contains(&retry.at), "{before} {}", retry.at);
  let why = "the provider is overloaded".to_owned();
  let retries = Retrying { attempt: 2, most: 10, wait: 4, at: retry.at, why };
  let waits = Line {
    number: 1,
    text: String::new(),
    mark: Mark::Retries(retries),
    output: None,
    made: Vec::new(),
    writes: Vec::new(),
    awaits: Vec::new(),
    pictures: Vec::new(),
  };
  assert_eq!(page().await, [waits]);
  // The model is asked again once the call fails for good, and that call fails too, so the
  // provider pauses the chain: its thread shows the failure once.
  until(&mut events, stands("chain1".to_owned(), State::Paused)).await;
  let replied =
    |event: &Event| matches!(event, Event::Act(act) if act.kind == Kind::Reply && act.done());
  until(&mut events, replied).await;
  let why = "opus/high answered nothing: the request is malformed";
  let why = why.to_owned();
  let failed = Line {
    number: 1,
    text: String::new(),
    mark: Mark::Failed { why },
    output: None,
    made: Vec::new(),
    writes: Vec::new(),
    awaits: Vec::new(),
    pictures: Vec::new(),
  };
  assert_eq!(page().await, [failed]);
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

#[tokio::test]
async fn a_lone_wake_of_a_chain_that_the_provider_paused_asks_its_model_again_and_the_life_goes_on()
{
  let at = place("lone-wake");
  let calls = Arc::new(AtomicUsize::new(0));
  let counted = Arc::clone(&calls);
  let answer = Answer(Arc::new(move |_, _| {
    counted.fetch_add(1, Ordering::SeqCst);
    Box::pin(async move { Err("the request is malformed".to_owned()) })
  }));
  let life = Life::spawn(Opening { answer: Some(answer), ..scripted(&at, &[]) }).unwrap();
  let mut events = Box::pin(life.events(0));
  let shape = "None".to_owned();
  let message = "Ground the branch.".to_owned();
  made(&life, Command::Prompt { shape, message, to: None, on: None }).await;
  until(&mut events, stands("chain1".to_owned(), State::Paused)).await;
  let asked = calls.load(Ordering::SeqCst);
  // The wake alone, with no message before it, as any client of the daemon can send it.
  assert!(matches!(life.send(Command::Wake { id: "chain1".to_owned() }).await, Ok(Reply::Done)));
  let replies =
    |event: &Event| matches!(event, Event::Act(act) if act.kind == Kind::Reply && !act.done());
  until(&mut events, replies).await;
  until(&mut events, stands("chain1".to_owned(), State::Paused)).await;
  assert!(calls.load(Ordering::SeqCst) > asked, "the wake asks the model again");
  assert!(matches!(life.send(Command::Standing).await, Ok(Reply::Value(_))), "the life goes on");
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}

/// A life that keeps its record at a place, whose model works for ever on each message that says
/// so, and closes each other request.
fn working(at: &Path) -> Opening {
  let answer = Answer(Arc::new(move |request, _| {
    let works = request.to_string().contains("(works)");
    Box::pin(async move {
      if works {
        future::pending::<()>().await;
      }
      let word = ["assistant", "close(None)"].map(Object::string);
      Ok(Object::tuple([word[0].clone(), word[1].clone(), Object::none(), Object::none()]))
    })
  }));
  Opening { answer: Some(answer), ..kept(at) }
}

#[tokio::test]
async fn a_message_that_the_operator_takes_back_while_its_chain_works_on_another_ends_cancelled() {
  let at = place("take-back");
  let life = Life::spawn(working(&at)).unwrap();
  let mut events = Box::pin(life.events(0));
  let said = |message: &str, on: Option<&str>| Command::Prompt {
    shape: "str".to_owned(),
    message: message.to_owned(),
    to: None,
    on: on.map(str::to_owned),
  };
  let first = made(&life, said("Read the boot (works)", None)).await;
  let replies =
    |event: &Event| matches!(event, Event::Act(act) if act.kind == Kind::Reply && !act.done());
  until(&mut events, replies).await;
  // The rung of the second message waits behind the rung of the first, whose model works.
  let second = made(&life, said("Then the replay (works)", Some(&first))).await;
  life.send(Command::Cancel { id: second.clone(), why: None }).await.unwrap();
  let got = until(&mut events, done(&second)).await;
  let Some(Event::Act(ended)) = got.last().map(|one| &*one.1) else { panic!("an act") };
  assert!(ended.cancelled(), "{ended:?}");
  life.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
  // A later life on the record holds the message as cancelled, and the first as it was.
  let later = Life::spawn(working(&at)).unwrap();
  let mut events = Box::pin(later.events(0));
  let got = until(&mut events, done(&second)).await;
  let Some(Event::Act(ended)) = got.last().map(|one| &*one.1) else { panic!("an act") };
  assert!(ended.cancelled(), "{ended:?}");
  let first_done = got.iter().any(|one| done(&first)(&one.1));
  assert!(!first_done, "the first message still runs");
  later.send(Command::End).await.unwrap();
  while events.next().await.is_some() {}
}
