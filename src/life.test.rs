//! The opening of a life: what it enables, what it keeps, and the ears it hears, on the record that the store keeps.

#![expect(clippy::disallowed_methods, reason = "a test blocks on its runtime and fails by a panic")]

use std::{
  collections::VecDeque,
  fs,
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
};

use super::{Answer, Opening};
use crate::testing::Yard;
use crate::{
  Act, Ear, Engine, Fact, Object, Text,
  ear::{ear, hear, say},
  extension::enabled,
  value::entry,
  verbs, world,
};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> Yard {
  static MADE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
  let n = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  Yard::new(format!("furb-life-{name}-{}-{n}", std::process::id()))
}

/// A life in a directory of the tests, on its record and on the configs of that directory, whose provider offers
/// the operator alone.
fn opening(at: &Path, keeps: bool) -> Opening {
  let place = |name: &str| Some(at.join(name).display().to_string());
  Opening {
    directory: at.display().to_string(),
    record: place("record.jsonl"),
    keeps: Some(keeps),
    config: place("config"),
    roster: Some(Vec::new()),
    ..Opening::default()
  }
}

/// The names of the extensions that a life runs.
fn names(engine: &mut Engine) -> Vec<String> {
  let root =
    engine.transcript(verbs::Transcript { on: Some("chain1".to_owned()), of: None }).unwrap();
  enabled(&root).into_iter().map(|one| one.name).collect()
}

/// The kind of each fact of a record.
fn kinds(record: &[Object]) -> Vec<String> {
  let fact = |one: &Object| entry(&one.as_ref(), 0).and_then(Fact::of).unwrap().kind().to_owned();
  record.iter().map(fact).collect()
}

#[test]
fn a_life_enables_what_the_configs_turn_on_and_a_later_life_runs_what_its_record_enables() {
  let at = place("configs");
  let (mut first, _) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(names(&mut first), ["catchup", "memory", "skills", "compaction", "naming"]);
  drop(first);
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("config.json"), r#"{"extensions": {"skills": false}}"#).unwrap();
  let (mut later, record) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut later),
    ["catchup", "memory", "skills", "compaction", "naming"],
    "the record wins over the configs"
  );
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert!(!kinds(&kept[record.len()..]).contains(&"enable".to_owned()), "it enabled nothing new");
  let fresh = Some(at.join("fresh.jsonl").display().to_string());
  let (mut fresh, _) = Opening { record: fresh, ..opening(&at, true) }.boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut fresh),
    ["catchup", "memory", "compaction", "naming"],
    "a life on a new record enables what the configs turn on"
  );
}

#[test]
fn a_life_that_inspects_its_record_keeps_nothing_enables_nothing_new_and_its_ears_that_do_work_are_silent()
 {
  let at = place("inspects");
  fs::write(at.join("a.txt"), "one\n").unwrap();
  drop(Opening { extensions: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap());
  let before = fs::read_to_string(at.join("record.jsonl")).unwrap();
  let (mut engine, _) =
    Opening { inspecting: Some(true), ..opening(&at, true) }.boot::<String>([]).unwrap();
  let read = verbs::Read { on: Some("chain1".to_owned()), ..Default::default() };
  assert_eq!(engine.read("a.txt", read).unwrap_err().message(), "nothing takes read");
  assert_eq!(names(&mut engine), Vec::<String>::new(), "it enables nothing the record does not");
  drop(engine);
  assert_eq!(fs::read_to_string(at.join("record.jsonl")).unwrap(), before);
}

#[test]
fn the_ears_of_the_crate_come_after_the_ears_of_the_host_each_under_the_name_every_host_hears_it_by()
 {
  let at = place("ears");
  let (_, ears) = opening(&at, true).parts().unwrap();
  let names: Vec<&str> = ears.iter().map(|(name, _)| name.as_str()).collect();
  assert_eq!(
    names,
    ["provider", "extensions", "skills", "mcp", "fetch", "files", "bash", "time", "store", "log"]
  );
  drop(ears);
  let (mut engine, _) = opening(&at, true).boot([("mine", mine())]).unwrap();
  let read = verbs::Read { on: Some(engine.root().to_owned()), ..Default::default() };
  assert_eq!(engine.read("mine://a", read).unwrap(), Text::new("mine://a", "mine\n"));
}

/// An ear of the host that takes a read of a path of its own, which the files refuse.
fn mine() -> Box<dyn Ear> {
  ear(|mut co| async move {
    loop {
      let a = hear(&mut co).await;
      let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      if a.question() && a.kind() == "read" && path.starts_with("mine://") {
        say(&mut co, Fact::says("done", a.about(), [Text::new(path, "mine\n").object()]));
      }
    }
  })
}

#[test]
fn a_life_that_keeps_a_record_writes_a_new_file_and_a_new_file_in_a_new_folder_to_its_scratch_directory()
 {
  let at = place("scratch");
  let (mut engine, _) =
    Opening { extensions: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap();
  let root = engine.root().to_owned();
  let on = || Some(root.clone());
  let write = verbs::Write { on: on() };
  let plan = engine.write(&Text::new("scratch://plan.md", "plan\n"), write).unwrap();
  assert_eq!(plan, Text::new("scratch://plan.md", "plan\n"));
  let write = verbs::Write { on: on() };
  let note = engine.write(&Text::new("scratch://notes/new.md", "note\n"), write).unwrap();
  assert_eq!(note, Text::new("scratch://notes/new.md", "note\n"));
  let read = verbs::Read { on: on(), ..Default::default() };
  assert_eq!(engine.read("scratch://", read).unwrap(), Text::new("scratch://", "notes/\nplan.md"));
}

#[test]
fn a_life_that_keeps_a_record_writes_the_log_of_every_fact_beside_it_and_a_later_life_writes_it_again_from_its_start()
 {
  let at = place("log");
  let record = at.join("record.jsonl");
  let open =
    || Opening { extensions: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap().0;
  let mut engine = open();
  let word = Some("x = 1\nclose(x)".to_owned());
  let rung =
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
  assert_eq!(futures::executor::block_on(rung).unwrap().as_ref().as_int(), Some(1));
  drop(engine);
  let shown =
    |facts: Vec<Object>| facts.iter().map(|one| one.as_ref().py_repr()).collect::<Vec<_>>();
  let first = shown(world::logged(&record).unwrap().expect("the log agrees with the record"));
  let rung = first.iter().position(|one| one.contains("'operator', 'chain1', 'x = 1"));
  let named =
    rung.and_then(|at| first[at].split('\'').nth(3).map(str::to_owned)).unwrap_or_default();
  let run = first
    .iter()
    .position(|one| one.starts_with("['run', ") && one.contains(&format!(", '{named}', 'x = 1")));
  assert!(
    rung.is_some_and(|rung| run.is_some_and(|run| rung < run)),
    "the log holds the rung and its run"
  );
  assert!(
    world::kept(&record).unwrap().iter().any(|one| one.as_ref().py_repr().contains("'run1'")),
    "the record keeps the question of the run that the chain made"
  );
  drop(open());
  let again = shown(world::logged(&record).unwrap().expect("a later life writes the log again"));
  // A fact that the journal says again is its own, so a later life says each fact again with its kind and its name.
  let head = |one: &String| one.splitn(3, ", ").take(2).collect::<Vec<_>>().join(", ");
  // The journal says the fact teach again only once the bus is still, so a replay logs it later
  // than the first life did.
  let heads = |facts: &[String]| {
    facts.iter().map(head).filter(|one| !one.starts_with("['teach'")).collect::<Vec<_>>()
  };
  assert!(again.iter().any(|one| one.starts_with("['teach', 'chain1'")), "{again:?}");
  let mine = heads(&first);
  assert_eq!(
    heads(&again)[..mine.len()],
    mine[..],
    "a later life says the log again before its own facts"
  );
  let mut kept = fs::OpenOptions::new().append(true).open(&record).unwrap();
  std::io::Write::write_all(&mut kept, b"[[\"stand\",\"stand9\",\"operator\",\"chain1\"]]\n")
    .unwrap();
  assert!(
    world::logged(&record).unwrap().is_none(),
    "a log that its record outgrew does not agree"
  );
}

#[test]
fn a_later_life_writes_in_its_log_each_act_of_its_record_that_an_ear_of_the_outside_made_and_does_not_make_again()
 {
  let at = place("owed");
  let record = at.join("record.jsonl");
  let open = || {
    let opening = Opening { extensions: Some(false), ..opening(&at, true) };
    opening.boot::<String>([]).unwrap().0
  };
  let mut engine = open();
  assert_eq!(
    closed(&mut engine, "await bash('true', show=HIDDEN)\nclose(1)").as_ref().as_int(),
    Some(1)
  );
  drop(engine);
  let asked = |facts: &[Object]| {
    facts.iter().any(|one| one.as_ref().py_repr().contains("['merged', 'merged1', 'bash'"))
  };
  assert!(
    asked(&world::kept(&record).unwrap()),
    "the record keeps the question that the ear of commands asked"
  );
  let mut engine = open();
  assert_eq!(closed(&mut engine, "close(2)").as_ref().as_int(), Some(2));
  drop(engine);
  let facts =
    world::logged(&record).unwrap().expect("the log of the later life agrees with the record");
  assert!(asked(&facts), "the log of the later life holds the question that no ear asked again");
}

/// What a word that a rung runs on the root closes with.
fn closed(engine: &mut Engine, word: &str) -> Object {
  let word = Some(word.to_owned());
  let rung =
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
  futures::executor::block_on(rung).unwrap()
}

/// The standing that a stand on the root gives, once the World answered it, or why it refused it.
fn stood(engine: &mut Engine) -> Result<Object, crate::Fault> {
  let stand = engine.stand(verbs::Stand { on: Some("chain1".to_owned()) })?;
  futures::executor::block_on(stand)
}

#[test]
fn a_life_that_opens_on_a_record_that_a_stop_cut_asks_the_reply_again_and_ends_the_command_with_no_wake()
 {
  let at = place("cut");
  let (asked, heard) = futures::channel::mpsc::unbounded::<()>();
  let asked = Mutex::new(asked);
  // The first model writes a part of its turn, and the life stops while it writes the rest.
  let streams = Answer(Arc::new(move |_, told| {
    told("x = ", "");
    drop(asked.lock().unwrap().unbounded_send(()));
    Box::pin(std::future::pending())
  }));
  let first = Opening {
    roster: Some(vec!["claude-cli:opus".to_owned()]),
    answer: Some(streams),
    extensions: Some(false),
    ..opening(&at, true)
  };
  let mut first = first.boot::<String>([]).unwrap().0;
  let on = Some("chain1".to_owned());
  drop(first.bash("sleep 30", verbs::Bash { on: on.clone(), ..Default::default() }).unwrap());
  let message = Some("go".to_owned());
  let prompt = verbs::Prompt { message, on, ..Default::default() };
  let prompt = first.prompt(Object::string("str"), prompt).unwrap().id().to_owned();
  {
    let mut heard = heard;
    let streamed = futures::StreamExt::next(&mut heard);
    let act = Act::<Object>::of(&mut first, &prompt);
    futures::executor::block_on(futures::future::select(act, streamed));
  }
  drop(first);
  let mut later = Opening { extensions: Some(false), ..scripted(&at, &["close('again')"]) }
    .boot::<String>([])
    .unwrap()
    .0;
  assert!(
    later.pending().unwrap().is_empty(),
    "the open put the work that the stop cut to the outside"
  );
  let got = futures::executor::block_on(Act::<Object>::of(&mut later, &prompt)).unwrap();
  assert_eq!(got.as_ref().as_str(), Some("again"), "the reply that the stop cut is asked again");
  let command = later.word("repr(peek('bash1'))", vec![]).unwrap();
  let command = command.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(
    command.contains("interrupted"),
    "the command that the stop cut ends as interrupted: {command}"
  );
  let running =
    "[x[1] for x in transcript(ROOT) if x[0] == 'started' and x[1] != ROOT and not over(x[1])]";
  let running = later.word(running, vec![]).unwrap().as_ref().py_repr();
  assert_eq!(running, "[]", "no act stays running with no process");
}

#[test]
fn a_word_that_polls_the_clock_ends_at_the_bound_of_a_turn_and_two_readings_in_a_row_still_run() {
  crate::sand::BOUND.set(std::time::Duration::from_secs(2));
  let at = place("clock-spin");
  let poll = "t0 = clock()\nwhile clock() - t0 < 60:\n  pass\nclose('polled')";
  let twice = "a = clock()\nb = clock()\nclose(b >= a)";
  let mut life = Opening { extensions: Some(false), ..scripted(&at, &[poll, twice]) }
    .boot::<String>([])
    .unwrap()
    .0;
  let asked = verbs::Prompt {
    message: Some("go".to_owned()),
    on: Some("chain1".to_owned()),
    ..Default::default()
  };
  let started = std::time::Instant::now();
  let prompt = life.prompt(Object::string("object"), asked).unwrap().id().to_owned();
  let got = futures::executor::block_on(Act::<Object>::of(&mut life, &prompt)).unwrap();
  let took = started.elapsed();
  assert!(life.stopped().is_none(), "the cut of the poll leaves the life whole");
  assert_eq!(got.as_ref().as_bool(), Some(true), "two readings in a row run");
  let raised = life
    .word("repr([x[3] for x in transcript('chain1', 'tell') if 'raised' in x[3][0]][:1])", vec![])
    .unwrap();
  let raised = raised.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(
    raised.contains("past the bound of a turn"),
    "the poll ends at the bound of a turn: {raised}"
  );
  let clocks = life.word("len(transcript('chain1', 'clock'))", vec![]).unwrap();
  eprintln!("the poll made {:?} acts of clock in {took:?}", clocks.as_ref().as_int());
  assert!(
    took < std::time::Duration::from_secs(20),
    "the poll was cut near 2 s, not at 60: {took:?}"
  );
  drop(life);
  let again = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert!(again.raised().is_none(), "a later life replays the poll and its refusal with no drift");
}

#[test]
fn a_life_whose_open_act_a_verb_of_no_words_made_ends_with_a_checkpoint_in_its_record() {
  let at = place("ends-kept");
  let mut life =
    Opening { extensions: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  // A verb with no words makes an act that makes an open prompt, as name_session makes the
  // prompt of its chain.
  let word = "with site.set(act('named', ROOT, None)):\n  named = prompt(str, 'Name?', to=OPERATOR)\nclose(1)";
  assert_eq!(closed(&mut life, word).as_ref().as_int(), Some(1));
  drop(life);
  let mut inspected =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let written =
    crate::checkpoint::written(&mut inspected).expect("the life gathers its checkpoint");
  drop(inspected);
  crate::checkpoint::keep(&at.join("record.jsonl"), written).unwrap();
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert!(crate::checkpoint::last(&kept).is_some(), "the record holds a checkpoint entry");
}

#[test]
fn a_life_that_a_fault_of_the_sandbox_stopped_opens_again_and_goes_on() {
  crate::sand::BOUND.set(std::time::Duration::from_millis(100));
  let at = place("faulted");
  let cutter = "try:\n  while True:\n    pass\nfinally:\n  while True:\n    pass";
  let mut first =
    Opening { extensions: Some(false), ..scripted(&at, &[cutter]) }.boot::<String>([]).unwrap().0;
  let on = Some("chain1".to_owned());
  let message = Some("go".to_owned());
  let asked = verbs::Prompt { message, on: on.clone(), ..Default::default() };
  let prompt = first.prompt(Object::string("str"), asked).unwrap().id().to_owned();
  drop(futures::executor::block_on(Act::<Object>::of(&mut first, &prompt)));
  assert!(first.stopped().is_some(), "the second excess of the turn stops the life");
  drop(first);
  let mut later =
    Opening { extensions: Some(false), ..scripted(&at, &["close('again')", "close('on')"]) }
      .boot::<String>([])
      .expect("a life that a fault stopped opens again")
      .0;
  assert!(later.stopped().is_none(), "the open does not run the word into the same stop");
  let message = Some("go on".to_owned());
  let asked = verbs::Prompt { message, on, ..Default::default() };
  let next = later.prompt(Object::string("str"), asked).unwrap().id().to_owned();
  let got = futures::executor::block_on(Act::<Object>::of(&mut later, &next)).unwrap();
  assert_eq!(got.as_ref().as_str(), Some("on"), "the life goes on");
  let state = format!(
    "repr([peek('{prompt}'), peek([x[1] for x in transcript('chain1', 'rung') if x[2] == '{prompt}'][0])])"
  );
  let ended = later.word(&state, vec![]).unwrap();
  let ended = ended.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(
    ended.starts_with("['again', Refused(") && ended.contains("was interrupted"),
    "the rung that stopped the life ends as interrupted, and its prompt asks its model again: {ended}"
  );
  drop(later);
  let mut again =
    Opening { extensions: Some(false), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  assert!(again.stopped().is_none(), "a second open replays the end of the rung");
  let replayed = again.word(&state, vec![]).unwrap();
  assert_eq!(
    replayed.as_ref().as_str().unwrap_or_default(),
    ended,
    "the replay holds the same state"
  );
}

/// The engine that each fact made of a record names, in its order.
fn builds(record: &Path) -> Vec<String> {
  let kept = world::kept(record).unwrap();
  let fact = |one: &Object| entry(&one.as_ref(), 0).and_then(Fact::of).unwrap();
  let named = |one: Fact| one.word(0).and_then(|word| word.as_str().map(str::to_owned));
  kept.iter().map(fact).filter(|one| one.kind() == world::MADE).filter_map(named).collect()
}

#[test]
fn a_record_names_the_engine_that_made_it_once_in_a_file_of_lines_and_in_a_database() {
  for name in ["record.jsonl", "record.db"] {
    let at = place(&format!("made-{name}"));
    let record = at.join(name);
    let open = || {
      let path = Some(record.display().to_string());
      Opening { record: path, extensions: Some(false), ..opening(&at, true) }.boot::<String>([])
    };
    drop(open().unwrap());
    let made = vec![world::engine()];
    assert_eq!(builds(&record), made);
    let kept = world::kept(&record).unwrap();
    assert_eq!(
      world::release(&kept).as_deref(),
      Some(env!("CARGO_PKG_VERSION")),
      "it names the release"
    );
    let schema = kept
      .iter()
      .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
      .find(|one| one.kind() == world::MADE)
      .and_then(|one| one.word(2).and_then(|word| word.as_str().map(str::to_owned)));
    assert_eq!(schema, Some(world::RECORD_SCHEMA.to_string()), "it names the schema of the record");
    drop(open().unwrap());
    assert_eq!(builds(&record), made, "a later life names it no more");
  }
}

#[test]
fn a_record_that_names_no_engine_replays_as_before_and_names_none() {
  let at = place("unnamed");
  let record = at.join("record.jsonl");
  let open = || Opening { extensions: Some(false), ..opening(&at, true) };
  let mut first = open().boot::<String>([]).unwrap().0;
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  let text = fs::read_to_string(&record).unwrap();
  let old: String = text
    .lines()
    .filter(|one| !one.contains("[\"made\""))
    .map(|one| one.to_owned() + "\n")
    .collect();
  fs::write(&record, &old).unwrap();
  let mut later = open().boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut later, "close(x + 1)").as_ref().as_int(), Some(2));
  drop(later);
  assert_eq!(builds(&record), Vec::<String>::new(), "this engine did not make it");
}

/// The fact made of a build of an engine, with no version, the schema of the record and the release
/// of this build, as a line of the record holds it.
fn made_of(engine: &str) -> String {
  format!(
    "[\"made\",\"chain1\",\"store\",\"{engine}\",null,\"{}\",\"{}\"]",
    world::RECORD_SCHEMA,
    env!("CARGO_PKG_VERSION")
  )
}

/// The fact made of an engine, as a record holds it, with the texts of that engine and of its
/// Kernel.
fn made_with(engine: &str, source: &str, kernel: &str) -> String {
  let texts = serde_json::to_string(&[source, kernel]).unwrap();
  format!("{},{}]", made_of(engine).trim_end_matches(']'), &texts[1..texts.len() - 1])
}

/// The fact made of this build, as its record holds it.
fn ours() -> String {
  made_with(&world::engine(), crate::ENGINE, crate::KERNEL)
}

/// A record of one rung that a life of a test kept, and the text of the record with its fact made changed.
fn made_elsewhere(at: &Path, made: &str) -> (PathBuf, String) {
  let record = at.join("record.jsonl");
  let mut first =
    Opening { extensions: Some(false), ..opening(at, true) }.boot::<String>([]).unwrap().0;
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  let ours = ours();
  let text = fs::read_to_string(&record).unwrap();
  assert!(text.contains(&ours), "the record names this build");
  let text = text.replace(&ours, made);
  fs::write(&record, &text).unwrap();
  (record, text)
}

#[test]
fn a_record_of_another_engine_that_holds_no_texts_replays_and_goes_on() {
  let at = place("same-version");
  let made = made_of(&"0".repeat(64));
  let (record, _) = made_elsewhere(&at, &made);
  let open = || Opening { extensions: Some(false), ..opening(&at, true) };
  let mut later = open().boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut later, "close(x + 1)").as_ref().as_int(), Some(2));
  drop(later);
  assert_eq!(builds(&record), ["0".repeat(64)], "the build that made it stays named");
}

#[test]
fn a_record_of_another_engine_replays_on_the_texts_that_it_holds_and_goes_on_with_them() {
  use sha2::{Digest, Sha256};
  let at = place("texts");
  let source = format!("{}\ncrossed = True\n", crate::ENGINE);
  let mut hash = Sha256::new();
  for one in [source.as_str(), crate::KERNEL] {
    hash.update(one.as_bytes());
    hash.update([0]);
  }
  let named = format!("{:x}", hash.finalize());
  let (record, text) = made_elsewhere(&at, &made_with(&named, &source, crate::KERNEL));
  let open = || Opening { extensions: Some(false), ..opening(&at, true) };
  let mut later = open().boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut later, "close(crossed and x + 1)").as_ref().as_int(),
    Some(2),
    "the life replays and goes on on the engine that its record holds"
  );
  drop(later);
  assert_eq!(builds(&record), std::slice::from_ref(&named), "the record names it alone");
  let mut again = open().boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut again, "close(x + 2)").as_ref().as_int(),
    Some(3),
    "it replays its tail on it"
  );
  drop(again);
  fs::write(&record, text.replace(&named, &"0".repeat(64))).unwrap();
  let refused = open().boot::<String>([]).err().map(|one| format!("{one:?}")).unwrap_or_default();
  assert!(
    refused.contains("another engine"),
    "texts of another engine refuse the record: {refused}"
  );
}

/// The records that each build of this version replays word by word: the records in `tests/replay`,
/// which `tests/replay/README.md` says. The records of the promise of a release are in
/// `tests/corpus`, which `tests/promise.rs` opens.
fn replays() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("replay")
}

/// A life of a test whose model answers each request with the next word of a script, and with close(None) once the
/// script runs out.
fn scripted(at: &Path, words: &[&str]) -> Opening {
  let said: VecDeque<String> = words.iter().map(|one| (*one).to_owned()).collect();
  let said = Arc::new(Mutex::new(said));
  let answer = Answer(Arc::new(move |_, _| {
    let word = said.lock().unwrap().pop_front().unwrap_or_else(|| "close(None)".to_owned());
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string(word),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  Opening {
    roster: Some(vec!["claude-cli:opus".to_owned()]),
    answer: Some(answer),
    ..opening(at, true)
  }
}

#[cfg(unix)]
#[test]
fn the_boot_of_a_life_asks_the_claude_command_line_its_models_once_and_a_later_stand_again() {
  let at = place("discovery");
  let bin = at.join("claude");
  let fake = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/world/provider/claude/fake-claude.sh");
  // A program written now has a stamp of its own, so no discovery of an earlier run in the cache of furb answers it.
  fs::write(&bin, fs::read(fake).unwrap()).unwrap();
  fs::set_permissions(&bin, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
  fs::write(at.join("models"), "nova 100 low\n").unwrap();
  let asked = || fs::read_to_string(at.join("discoveries")).unwrap_or_default().lines().count();
  let (mut engine, _) = Opening {
    claude: Some(bin.display().to_string()),
    roster: Some(vec!["claude-cli:nova".to_owned()]),
    extensions: Some(false),
    ..opening(&at, true)
  }
  .boot::<String>([])
  .unwrap();
  // This test failed once in 20 full runs, with no log that said why. Each check says what the
  // fake saw, so the next failure names its cause.
  let saw = || {
    let heard = |name: &str| fs::read_to_string(at.join(name)).unwrap_or_else(|no| no.to_string());
    format!("discoveries: {:?}, model: {:?}", heard("discoveries"), heard("model"))
  };
  assert_eq!(
    asked(),
    1,
    "the opening asked the models, and the stand of the boot holds what it found; {}",
    saw()
  );
  let started = std::time::Instant::now();
  let stand = stood(&mut engine);
  let took = started.elapsed();
  assert!(stand.is_ok(), "the stand took {took:?} and failed: {stand:?}; {}", saw());
  assert_eq!(asked(), 2, "a later stand asks the command line again, in {took:?}; {}", saw());
}

#[test]
fn a_life_opens_when_its_actor_and_a_roster_name_left_the_catalog_and_tells_each_chain_once() {
  let at = place("catalog-left");
  let (mut engine, _) = Opening {
    extensions: Some(false),
    roster: Some(vec!["claude-cli:opus".to_owned(), "claude-cli:gone".to_owned()]),
    actor: Some("claude-cli:vanished/high".to_owned()),
    ..scripted(&at, &[])
  }
  .boot::<String>([])
  .unwrap();
  let roster = "close([a[0] for a in standing()[0]])";
  assert_eq!(closed(&mut engine, roster).py_repr(), "['claude-cli:opus', 'operator']");
  let actor = "close(standing()[2])";
  assert_eq!(closed(&mut engine, actor).py_repr(), "'claude-cli:opus/high'");
  let left = r#"close(len([1 for x in transcript('chain1') if x[0] == 'tell' and x[2] == 'provider' and "_left = ['claude-cli:gone', 'claude-cli:vanished/high']" in x[3][0]]))"#;
  assert_eq!(closed(&mut engine, left).as_ref().as_int(), Some(1));
  stood(&mut engine).unwrap();
  assert_eq!(closed(&mut engine, left).as_ref().as_int(), Some(1), "a later stand tells nothing");
  let refused = "try:\n  await prompt(int, 'which', to='claude-cli:gone')\nexcept Refused as no:\n  close(str(no))";
  assert_eq!(closed(&mut engine, refused).py_repr(), "'claude-cli:gone no actor'");
}

#[test]
fn a_life_that_names_no_actor_and_no_roster_goes_on_with_the_ones_its_record_stood_on_last() {
  let at = place("stood-again");
  let both = "close((standing()[2], sorted(a[0] for a in standing()[0])))";
  let given = Opening {
    extensions: Some(false),
    roster: Some(vec!["claude-cli:opus".to_owned(), "claude-cli:sonnet".to_owned()]),
    actor: Some("claude-cli:sonnet/low".to_owned()),
    ..scripted(&at, &[])
  };
  let (mut first, _) = given.boot::<String>([]).unwrap();
  let had = closed(&mut first, both).py_repr();
  assert_eq!(
    had,
    "('claude-cli:sonnet/low', ['claude-cli:opus', 'claude-cli:sonnet', 'operator'])"
  );
  drop(first);
  let unsaid = Opening { extensions: Some(false), roster: None, actor: None, ..scripted(&at, &[]) };
  let (mut later, _) = unsaid.boot::<String>([]).unwrap();
  assert_eq!(
    closed(&mut later, both).py_repr(),
    had,
    "the open named neither, so the record rules"
  );
}

#[test]
fn a_stand_reloads_provider_settings_and_a_refused_reload_keeps_the_life() {
  let at = place("provider-settings");
  fs::create_dir_all(at.join("config")).unwrap();
  fs::create_dir_all(at.join(".furb")).unwrap();
  fs::write(
    at.join("config/config.json"),
    r#"{"providers":{"bench":{"url":"http://127.0.0.1:9/v1","models":{"one":{"window":65536}}}}}"#,
  )
  .unwrap();
  let path = at.join(".furb/config.json");
  fs::write(&path, r#"{"providers":{"claude-cli":false,"local":false}}"#).unwrap();
  let (mut engine, _) = Opening {
    extensions: Some(false),
    roster: None,
    actor: Some("bench:one".to_owned()),
    ..scripted(&at, &["close(23)", "close(29)"])
  }
  .boot::<String>([])
  .unwrap();
  let names = "close([a[0] for a in standing()[0] if str(a[0]).startswith('bench:')])";
  assert_eq!(closed(&mut engine, names).py_repr(), "['bench:one']");
  assert_eq!(
    closed(&mut engine, "marker = 17\nclose(await prompt(int, to='bench:one'))").as_ref().as_int(),
    Some(23)
  );
  fs::write(&path, r#"{"providers":{"bench":{"models":{"two":{"window":131072,"images":true}}}}}"#)
    .unwrap();
  stood(&mut engine).unwrap();
  assert_eq!(closed(&mut engine, names).py_repr(), "['bench:two']");
  assert_eq!(
    closed(&mut engine, "close(marker + await prompt(int, to='bench:two'))").as_ref().as_int(),
    Some(46)
  );
  fs::write(&path, r#"{"providers":{"bench":{"models":{"bad":{"window":0}}}}}"#).unwrap();
  assert!(stood(&mut engine).is_err());
  assert_eq!(closed(&mut engine, names).py_repr(), "['bench:two']");
  assert_eq!(closed(&mut engine, "close(marker)").as_ref().as_int(), Some(17));
  fs::write(&path, r#"{"providers":{"bench":false}}"#).unwrap();
  stood(&mut engine).unwrap();
  assert_eq!(closed(&mut engine, names).py_repr(), "[]");
  drop(engine);
  fs::remove_dir_all(at).unwrap();
}

/// The words of the model of the life of `worked`: the root writes, reads, runs a command, waits,
/// reads the clock and chance, and asks a chain of its own, whose model answers in the second word.
const WORK: [&str; 2] = [
  "write(Text(\"notes.md\", \"one\\ntwo\\n\"))\nnotes = read(\"notes.md\")\nsaid = await bash(\"echo hi\")\nawait \
   wait(0.01, __name__)\nhelper = chain(\"helper\")\ngot = await prompt(int, \"Give 2.\", on=helper)\nclose(got + \
   len(notes.lines) + int(clock() > 0) + int(0 <= chance() < 1))",
  "close(2)",
];

/// The record of the life of `WORK` in a directory of its own: a first life works, and a second
/// life on the record takes one prompt more, so the record holds two openings. Each official
/// extension runs on it.
fn worked(name: &str) -> Yard {
  let at = place(name);
  let mut first = scripted(&at, &WORK).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut first, "close(await prompt(int, 'Work in the directory.'))").as_ref().as_int(),
    Some(6)
  );
  drop(first);
  let mut later = scripted(&at, &["close('done')"]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut later, "close(await prompt(str, 'Say done.'))").as_ref().as_str(),
    Some("done")
  );
  drop(later);
  at.within("record.jsonl")
}

#[test]
fn a_life_whose_replay_drifts_keeps_nothing_beside_its_record_but_the_lease() {
  let at = place("drifts");
  let record = at.join("record.jsonl");
  // The place where the first stand stood, changed, makes the replay drift at the word that the stand told.
  let mut lines: Vec<String> =
    fs::read_to_string(worked("drifts-work")).unwrap().lines().map(str::to_owned).collect();
  let stood = lines
    .iter()
    .position(|line| line.starts_with(r#"[["done","stand1","#))
    .expect("the record holds the first stand");
  let mut entry: serde_json::Value = serde_json::from_str(&lines[stood]).unwrap();
  let place = entry[0][3][1].as_str().expect("the stand names its place").to_owned();
  entry[0][3][1] = serde_json::Value::from(format!("{place}-elsewhere"));
  lines[stood] = entry.to_string();
  // The last line is whole and not ended, which a store mends only at its first keep.
  fs::write(&record, lines.join("\n")).unwrap();
  fs::write(at.join("record.jsonl.log"), "{\"entries\": 0}\n").unwrap();
  let held = |name: &str| fs::read(at.join(name)).unwrap();
  let (text, log) = (held("record.jsonl"), held("record.jsonl.log"));
  // The config directory of the user stands apart, since a life writes there the copy of each word.
  let apart = Yard::unmade(format!("furb-life-drifts-config-{}", std::process::id()));
  let config = Some(apart.display().to_string());
  let refused =
    Opening { config, ..opening(&at, true) }.boot::<String>([]).map(|_| ()).unwrap_err();
  assert!(refused.to_string().starts_with("Drift: "), "{refused}");
  // The word shows the place as python writes it, and the message as Rust debugs it: four marks for each one.
  let shown = place.replace('\\', "\\\\\\\\");
  let moved = format!("stand()\\ncd('{shown}-elsewhere')");
  assert!(
    refused.to_string().contains(&moved),
    "the drift is at the word that the stand told: {refused}"
  );
  let mut beside: Vec<String> = fs::read_dir(&at)
    .unwrap()
    .map(|one| one.unwrap().file_name().to_string_lossy().into_owned())
    .collect();
  beside.sort();
  assert_eq!(beside, ["record.jsonl", "record.jsonl.lock", "record.jsonl.log"]);
  assert_eq!(held("record.jsonl"), text, "the record stays as it was");
  assert_eq!(held("record.jsonl.log"), log, "the log stays as it was");
}

#[test]
fn each_record_of_the_replays_replays_on_its_own_engine_with_no_drift() {
  let mut records: Vec<PathBuf> = fs::read_dir(replays())
    .unwrap()
    .map(|one| one.unwrap().path())
    .filter(|one| one.extension().is_some_and(|named| named == "jsonl" || named == "zst"))
    .collect();
  records.sort();
  assert!(!records.is_empty(), "tests/replay holds no record");
  for one in records {
    let name = one.file_stem().unwrap().to_string_lossy().trim_end_matches(".jsonl").to_owned();
    let at = place(&format!("replay-{name}"));
    laid(&one, &at.join("record.jsonl"));
    let kept = world::kept(at.join("record.jsonl")).unwrap();
    let made = kept
      .iter()
      .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
      .find(|a| a.kind() == world::MADE);
    assert!(
      made.is_some_and(|a| a.word(5).is_some()),
      "{name} holds no texts of its engine: give them with script/engines.py, as tests/replay/README.md says"
    );
    let inspects = Opening { inspecting: Some(true), ..opening(&at, false) };
    if let Err(drift) = inspects.clone().boot::<String>([]) {
      panic!("{name} drifts on its own engine, a defect of this build: {drift}");
    }
    if crate::checkpoint::last(&kept).is_some()
      && let Err(drift) = (Opening { whole: Some(true), ..inspects }).boot::<String>([])
    {
      panic!("{name} drifts from its first entry, born again at each checkpoint: {drift}");
    }
  }
}

#[test]
fn a_record_of_an_engine_that_an_adapter_names_drifts_on_that_engine_without_the_adapter() {
  use sha2::{Digest, Sha256};
  let at = place("unadapted");
  laid(&replays().join("ranks.jsonl.zst"), &at.join("record.jsonl"));
  let mut lines: Vec<String> =
    fs::read_to_string(at.join("record.jsonl")).unwrap().lines().map(str::to_owned).collect();
  let made = lines.iter().position(|one| one.starts_with(r#"[["made""#)).unwrap();
  let mut entry: serde_json::Value = serde_json::from_str(&lines[made]).unwrap();
  let fact = entry[0].as_array_mut().unwrap();
  // The same engine with one more line at its end, which no adapter names.
  let engine = format!("{}\n", fact[7].as_str().unwrap());
  let mut hash = Sha256::new();
  for one in [engine.as_str(), fact[8].as_str().unwrap()] {
    hash.update(one.as_bytes());
    hash.update([0]);
  }
  fact[3] = format!("{:x}", hash.finalize()).into();
  fact[7] = engine.into();
  lines[made] = entry.to_string();
  fs::write(at.join("record.jsonl"), lines.join("\n") + "\n").unwrap();
  let drift =
    Opening { inspecting: Some(true), ..opening(&at, false) }.boot::<String>([]).unwrap_err();
  assert!(format!("{drift:?}").contains("rung3230 drifts"), "{drift:?}");
}

/// Each run of each chain whose word no turn of that chain shows, as a python expression in the names of the engine.
const UNSHOWN: &str = "[(c, x[1], x[4]) for c in [f'chain{n}' for n in range(1, 99) if get(f'chain{n}')] \
                       for x in transcript(c, 'run') if x[3] == c \
                       and not any(x[5] in unquoted(t[1]) for t in turns(on=c))]";

/// Each extension whose file the root does not show as the turn `rung(read('<file>', HIDDEN))` and then the turn
/// of its code alone, or whose definitions the module of the root lacks.
const UNSEEDED: &str = "[e[3] for e in transcript('chain1', 'enable') if e[4].strip() and (not any(t[:2] == \
                        ('assistant', f'rung(read({e[7]!r}, HIDDEN).content)') and turns(on='chain1')[n + 1][:2] == \
                        ('user', e[4]) for n, t in enumerate(turns(on='chain1'))) or not all(m in \
                        module('chain1') for m in re.findall(r'^(?:def|class) (\\w+)', e[4], re.M)))]";

/// A record of the replays, written whole at `to`: one that is too large to keep plain is kept
/// by zstd with a long window, and read through it.
fn laid(from: &Path, to: &Path) {
  if from.extension().is_some_and(|named| named == "zst") {
    let mut plain = zstd::Decoder::new(fs::File::open(from).unwrap()).unwrap();
    plain.window_log_max(27).unwrap();
    std::io::copy(&mut plain, &mut fs::File::create(to).unwrap()).unwrap();
  } else {
    fs::copy(from, to).unwrap();
  }
}

/// Each record of `tests/replay`, and the record of the life of `WORK`, replayed in a life that
/// only inspects it, by its name.
fn replayed(by: &str) -> Vec<(String, Engine)> {
  let mut records: Vec<PathBuf> = fs::read_dir(replays())
    .unwrap()
    .map(|one| one.unwrap().path())
    .filter(|one| one.extension().is_some_and(|named| named == "jsonl" || named == "zst"))
    .collect();
  records.sort();
  let made = worked(&format!("shown-{by}-made"));
  let work = made.with_file_name("work.jsonl");
  fs::rename(work.with_file_name("record.jsonl"), &work).unwrap();
  records.push(work);
  let open = |one: PathBuf| {
    let name = one.file_stem().unwrap().to_string_lossy().trim_end_matches(".jsonl").to_owned();
    let at = place(&format!("shown-{by}-{name}"));
    laid(&one, &at.join("record.jsonl"));
    let inspects = Opening { inspecting: Some(true), ..opening(&at, false) };
    (name, inspects.boot::<String>([]).unwrap().0)
  };
  records.into_iter().map(open).collect()
}

#[test]
fn each_run_of_a_chain_of_the_replays_has_a_turn_that_shows_its_word() {
  for (name, mut engine) in replayed("unshown") {
    let unshown = engine.word(UNSHOWN, vec![]).unwrap();
    assert_eq!(
      unshown.as_ref().items().unwrap().len(),
      0,
      "{name}: {}",
      unshown.as_ref().py_repr()
    );
  }
}

#[test]
fn a_replay_of_work_shows_each_file_of_an_extension_as_the_rung_of_its_read_then_its_code_alone() {
  let (_, mut engine) = replayed("seeded").into_iter().find(|one| one.0 == "work").unwrap();
  let enables = engine.word("len(transcript('chain1', 'enable'))", vec![]).unwrap();
  assert_eq!(
    enables.as_ref().as_int(),
    Some(5),
    "each official extension that plays by default runs on the life of work"
  );
  let unseeded = engine.word(UNSEEDED, vec![]).unwrap();
  assert_eq!(unseeded.as_ref().items().unwrap().len(), 0, "{}", unseeded.as_ref().py_repr());
}

#[test]
#[ignore = "a measure, which a person runs in release"]
fn measure_how_long_a_life_opens_from_its_checkpoint_and_by_a_replay() {
  let open = |at: &Path| scripted(at, &[]).boot::<String>([]).unwrap().0;
  let number = |name: &str, or: usize| -> usize {
    std::env::var(name).ok().and_then(|one| one.parse().ok()).unwrap_or(or)
  };
  let (n, size, tail) = (number("RUNGS", 200), number("CHARS", 0), number("TAIL", 20));
  let word = |name: &str, i: usize| match size {
    0 => format!("{name}{i} = [{i}] * 50\nclose(len({name}{i}))"),
    // A rung that closes with a large text weighs on the record, the log and the checkpoint, as a
    // command does.
    _ => format!("{name}{i} = '{i} ' * {}\nclose({name}{i})", size / 2),
  };
  // One record that a life of n rungs made, and one where that life ended with its checkpoint and a
  // later life added a tail of TAIL rungs: a later life replays the first whole, and opens on the
  // second from the checkpoint and the tail.
  let (whole, cut) = (place("measure"), place("measure-checkpoint"));
  for at in [&whole, &cut] {
    let mut engine = open(at);
    for i in 0..n {
      closed(&mut engine, &word("x", i));
    }
    if **at == *cut {
      engine.checkpoint().unwrap();
      engine = open(at);
      for i in 0..tail {
        closed(&mut engine, &word("y", i));
      }
    }
    // A life that works always holds work of the outside that runs, as a wait.
    let wait = verbs::Wait { seconds: Some(3600.0), on: Some("chain1".to_owned()) };
    drop(engine.wait(&["chain1"], wait).unwrap());
    drop(engine);
  }
  for at in [&whole, &cut] {
    let kept = world::kept(at.join("record.jsonl")).unwrap();
    let after = kept.len() - crate::checkpoint::last(&kept).map_or(0, |one| one + 1);
    let bytes = fs::metadata(at.join("record.jsonl")).unwrap().len();
    let t = std::time::Instant::now();
    let engine = open(at);
    let spent = t.elapsed();
    drop(engine);
    eprintln!(
      "rungs {n}, tail {tail}, entries {}, {after} to replay, record {bytes} bytes: open {spent:?}",
      kept.len()
    );
  }
}

#[test]
fn a_life_goes_on_from_a_checkpoint_with_the_names_the_module_and_the_open_prompt_of_each_chain() {
  let at = place("checkpoint");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nran = await bash('echo one')\nqueue = [ran]\nSIZE = 6\ndef f():\n  return SIZE\n\
              waiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  // The checkpoint is written as the command line writes it: in a life that only replays the record.
  let mut inspected =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let written = crate::checkpoint::written(&mut inspected).unwrap();
  drop(inspected);
  crate::checkpoint::keep(&at.join("record.jsonl"), written).unwrap();
  let mut later =
    scripted(&at, &["rounds += 1\nclose(f'round {rounds}')"]).boot::<String>([]).unwrap().0;
  let word = format!(
    "wake('chain2')\nclose([str(ran), queue, f(), get('{waiting}')[:2], await waiting, str(chain('more')), str(bash('echo two'))])"
  );
  let exit = "Exit(code=0, stdout=Text(path='bash1/stdout', content='one\\n'), stderr=Text(path='bash1/stderr', content=''))";
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[{exit:?}, [{exit}], 6, ('prompt', '{waiting}'), 'round 2', 'chain3', 'bash2']"),
    "each name, value and answer of the past stands, the model starts the cut word again in a turn on the module \
     that the checkpoint carries and closes the open prompt, and a new act takes the next name"
  );
  assert_eq!(
    closed(&mut later, "close([x[1] for x in transcript('chain1', 'merged')])").as_ref().py_repr(),
    "['merged2']",
    "a name that an ear of the outside took in the past stays taken"
  );
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let made = |one: &Object| {
    entry(&one.as_ref(), 0).and_then(Fact::of).is_some_and(|a| a.kind() == world::MADE)
  };
  assert_eq!(
    crate::checkpoint::segment(&kept).iter().filter(|one| made(one)).count(),
    1,
    "the build that goes on names itself once"
  );
  let mut again = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut again, "close([f(), str(chain('last'))])").as_ref().py_repr(),
    "[6, 'chain4']",
    "a later life replays the segment after the checkpoint"
  );
}

#[test]
fn a_checkpoint_holds_data_alone_and_a_later_engine_seeds_a_life_from_it_with_no_act_run_again() {
  let at = place("checkpoint-data");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose('late')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nshell = bash('echo one')\nawait shell\nqueue = [shell]\nSIZE = 6\n\
              def f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  first.checkpoint().unwrap();
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let fact = entry(&kept[crate::checkpoint::last(&kept).unwrap()].as_ref(), 0).and_then(Fact::of);
  let data = crate::wire::outward(fact.unwrap().word(0).unwrap());
  assert_eq!(data["schema"], serde_json::json!(world::CHECKPOINT_SCHEMA.to_string()));
  assert!(
    data.get("version").is_none(),
    "the checkpoint names its schema and no version of the replay"
  );
  assert!(data.get("entries").is_none(), "the checkpoint holds data and no entry");
  let chains: Vec<&str> =
    data["chains"].as_array().unwrap().iter().filter_map(|one| one["id"].as_str()).collect();
  assert_eq!(chains, ["chain1", "chain2"]);
  let root = &data["chains"][0];
  assert_eq!(root["values"]["queue"], serde_json::json!("[Act('bash1')]"));
  assert!(root["definitions"].as_str().unwrap().contains("def f():"));
  let acts: Vec<(&str, bool)> = data["acts"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| (one["id"].as_str().unwrap(), one.get("value").is_some()))
    .collect();
  assert!(acts.contains(&("bash1", true)) && acts.contains(&(waiting.as_str(), false)), "{acts:?}");
  assert!(
    !data.to_string().contains("say('prefix'"),
    "the checkpoint holds no word that the crate crafts"
  );
  let mut later = scripted(&at, &["close('late')"]).boot::<String>([]).unwrap().0;
  let word = format!(
    "wake('chain2')\nclose([await queue[0], f(), get('{waiting}')[:2], await Act('{waiting}'), str(bash('echo two'))])"
  );
  let exit = "Exit(code=0, stdout=Text(path='bash1/stdout', content='one\\n'), stderr=Text(path='bash1/stderr', content=''))";
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[{exit}, 6, ('prompt', '{waiting}'), 'late', 'bash2']"),
    "each value, definition, open prompt and act that a value names stands, and a new act takes the next name"
  );
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let tail = crate::checkpoint::segment(&kept);
  let again = tail
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(|a| a.kind() == "started" && a.about() == "bash1");
  assert_eq!(again.count(), 0, "no carried act runs again");
}

/// The words of the seed of the root that the lives after the last checkpoint of a record gave the
/// gate.
fn seeds(at: &Path) -> usize {
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let seed = |a: &Fact| {
    a.kind() == "gate"
      && a.word(1).and_then(|one| one.as_str()) == Some("debug(t\"{standing()[4]=}\")")
  };
  crate::checkpoint::segment(&kept)
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(seed)
    .count()
}

#[test]
fn a_checkpoint_of_a_later_life_of_a_record_opens_with_no_drift_and_seeds_the_root_once() {
  let at = place("checkpoint-relife");
  let open =
    || Opening { extensions: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let mut first = open();
  assert_eq!(closed(&mut first, "n = 1\nclose(n)").as_ref().as_int(), Some(1));
  assert_eq!(seeds(&at), 1, "the first life of a record seeds the root once");
  first.checkpoint().unwrap();
  for n in 2..=4 {
    let mut later = open();
    let word = "n = n + 1\nclose([n, len([x for x in transcript('chain1', 'setup')])])";
    assert_eq!(
      closed(&mut later, word).as_ref().py_repr(),
      format!("[{n}, 1]"),
      "each later life goes on from the checkpoint of the life before it, and holds the one setup of the root"
    );
    assert_eq!(seeds(&at), 0, "a life that goes on from a checkpoint runs no seed of the root");
    later.checkpoint().unwrap();
  }
}

#[test]
fn a_large_value_and_an_exit_come_back_from_the_checkpoint_at_each_later_open_and_no_rung_of_the_seed_raises()
 {
  let at = place("checkpoint-large");
  let open = || scripted(&at, &[]).boot::<String>([]).unwrap().0;
  let mut first = open();
  let word = "big = 'x' * LARGEST\nran = Exit(0, Text('o', 'y' * LARGEST), Text('e', ''))\nsmall = 1\nclose(small)";
  assert_eq!(closed(&mut first, word).as_ref().as_int(), Some(1));
  first.checkpoint().unwrap();
  for _ in 0..2 {
    let mut later = open();
    let read = "close([len(big), len(ran.stdout.content), small, repr(raised)])";
    assert_eq!(
      closed(&mut later, read).as_ref().py_repr(),
      "[524288, 524288, 1, 'None']",
      "the definitions make no value again, so each comes back from the checkpoint"
    );
    later.checkpoint().unwrap();
  }
}

/// The word of the first life of a record whose module, chain and open prompt a later life holds.
const WAITED: &str = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose('late')";

/// A life on a record that holds a value, a definition, a chain and a prompt open on it, which ends
/// with its checkpoint, and the name of that prompt.
fn checkpointed(at: &Path) -> String {
  let mut first = scripted(at, &[WAITED]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nSIZE = 6\ndef f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\n\
              await wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  first.checkpoint().unwrap();
  waiting
}

/// A later life on the record of [`checkpointed`] holds each chain, the turns of the root, the
/// module and the open prompt on its chain.
fn reopened(at: &Path, waiting: &str, why: &str) {
  let mut later = scripted(at, &[]).boot::<String>([]).unwrap().0;
  let word = format!(
    "close([f(), get('chain2')[4], len(turns(on='chain1')) > 1, get('{waiting}')[3:5], \
     isinstance(peek('{waiting}'), str) or not over('{waiting}')])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    "[6, 'work', True, ('chain2', 'str'), True]",
    "{why}"
  );
}

/// The record at a place, with each line that holds a piece changed by a function.
fn changed(at: &Path, piece: &str, change: impl Fn(&str) -> String) {
  let path = at.join("record.jsonl");
  let text = fs::read_to_string(&path).unwrap();
  assert!(text.contains(piece), "the record holds {piece}");
  let lines: Vec<String> = text
    .lines()
    .map(|line| if line.contains(piece) { change(line) } else { line.to_owned() })
    .collect();
  fs::write(&path, lines.join("\n") + "\n").unwrap();
}

#[test]
fn a_checkpoint_of_a_later_minor_of_its_schema_with_fields_more_opens_with_each_chain_and_its_prompts()
 {
  let at = place("checkpoint-minor");
  let waiting = checkpointed(&at);
  let ours = format!("\"schema\":\"{}\"", world::CHECKPOINT_SCHEMA);
  let later =
    format!("\"schema\":\"{}.{}\",\"later\":{{\"seen\":1}}", world::CHECKPOINT_SCHEMA.major, 9);
  changed(&at, "[[\"checkpoint\",", |line| {
    line.replace(&ours, &later).replace("\"id\":\"", "\"later\":1,\"id\":\"")
  });
  let text = fs::read_to_string(at.join("record.jsonl")).unwrap();
  assert!(text.contains("\"later\":1,\"id\":\"chain2\""), "a chain holds a field more");
  reopened(
    &at,
    &waiting,
    "a later minor of the schema opens, and each field that this build does not know is skipped",
  );
}

#[test]
fn a_record_whose_tail_holds_a_kind_of_fact_that_this_build_does_not_know_opens_with_each_chain_and_its_prompts()
 {
  let at = place("record-kind");
  let waiting = checkpointed(&at);
  reopened(
    &at,
    &waiting,
    "the first life after the checkpoint goes on from it and asks the open prompt again",
  );
  let later = format!(
    "[[\"made\",\"chain1\",\"store\",\"{}\",null,\"{}.9\",\"9.9.9\"]]",
    "0".repeat(64),
    world::RECORD_SCHEMA.major
  );
  let unknown = [
    later.as_str(),
    "[[\"glimpse\",\"chain1\",\"store\",{\"seen\":1}]]",
    "[[\"glimpse\",\"glimpse1\",\"store\",\"chain1\",\"later\"]]",
    "[[\"done\",\"glimpse1\",\"store\",\"seen\"]]",
  ];
  let path = at.join("record.jsonl");
  let text = fs::read_to_string(&path).unwrap();
  fs::write(&path, format!("{text}{}\n", unknown.join("\n"))).unwrap();
  reopened(
    &at,
    &waiting,
    "a later build of the major made the tail, and this build skips the kind that it does not know",
  );
}

#[test]
fn a_record_of_another_major_of_its_schema_is_refused_with_both_schemas_and_keeps_nothing() {
  let at = place("record-major");
  checkpointed(&at);
  let ours = format!(",\"{}\",", world::RECORD_SCHEMA);
  let theirs = format!(",\"{}.0\",", world::RECORD_SCHEMA.major + 1);
  changed(&at, "[[\"made\",", |line| line.replace(&ours, &theirs));
  let text = fs::read_to_string(at.join("record.jsonl")).unwrap();
  let refused = scripted(&at, &[]).boot::<String>([]).unwrap_err();
  assert_eq!(
    refused.message(),
    format!(
      "the record holds the schema {}.0, of another major than the schema {} that this build writes: open it with a \
       build that writes the major {}, or start a new life",
      world::RECORD_SCHEMA.major + 1,
      world::RECORD_SCHEMA,
      world::RECORD_SCHEMA.major + 1
    )
  );
  assert_eq!(fs::read_to_string(at.join("record.jsonl")).unwrap(), text, "it keeps nothing");
}

#[test]
fn a_checkpoint_of_another_major_of_its_schema_is_refused_with_both_schemas() {
  let at = place("checkpoint-schema");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  closed(&mut first, "kept = 1\nclose(kept)");
  first.checkpoint().unwrap();
  let ours = format!("\"schema\":\"{}\"", world::CHECKPOINT_SCHEMA);
  let theirs = format!("\"schema\":\"{}.0\"", world::CHECKPOINT_SCHEMA.major + 1);
  changed(&at, &ours, |line| line.replace(&ours, &theirs));
  let refused = scripted(&at, &[]).boot::<String>([]).unwrap_err();
  assert_eq!(
    refused.message(),
    format!(
      "the checkpoint holds the schema {}.0, of another major than the schema {} that this build writes: open it \
       with a build that writes the major {}, or start a new life",
      world::CHECKPOINT_SCHEMA.major + 1,
      world::CHECKPOINT_SCHEMA,
      world::CHECKPOINT_SCHEMA.major + 1
    )
  );
}

#[test]
fn a_checkpoint_of_the_form_before_its_schema_that_a_later_life_wrote_opens_and_seeds_no_root_again()
 {
  let at = place("checkpoint-1034-relife");
  fs::copy(replays().join("handover").join("relife.jsonl"), at.join("record.jsonl")).unwrap();
  for n in 3..=4 {
    let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
    let word = format!("queue.append({n})\nclose([f(), queue])");
    assert_eq!(
      closed(&mut later, &word).as_ref().py_repr(),
      format!("[6, {:?}]", (1..=n).collect::<Vec<_>>()),
      "a life that a later life of that form handed over goes on with its module"
    );
    assert_eq!(
      seeds(&at),
      0,
      "a life that goes on from a checkpoint of that form runs no seed of the root"
    );
    later.checkpoint().unwrap();
  }
}

#[test]
fn a_checkpoint_of_the_form_before_its_schema_opens_on_this_build() {
  let at = place("checkpoint-1034");
  fs::copy(replays().join("handover").join("checkpoint.jsonl"), at.join("record.jsonl")).unwrap();
  let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut later, "close([f(), len(queue), get(waiting)[:2], over(waiting)])")
      .as_ref()
      .py_repr(),
    "[6, 1, ('prompt', 'prompt1'), False]",
    "a life that a build of that form handed over goes on with its module and its open prompt"
  );
  assert_eq!(
    seeds(&at),
    0,
    "a life that goes on from a checkpoint of that form runs no seed of the root"
  );
}

#[test]
fn a_later_life_of_the_engine_that_wrote_a_checkpoint_boots_on_it_and_on_the_tail_alone() {
  let at = place("checkpoint-live");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  closed(&mut first, "await bash('echo kept')\nclose(None)");
  let word = "work = chain('work')\nSIZE = 6\ndef f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\n\
              await wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  let turns = "close(len(turns('chain2')))";
  let told = closed(&mut first, turns).as_ref().as_int();
  first.checkpoint().unwrap();
  let mut second = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut second, "after = f() + 1\nclose(after)").as_ref().as_int(), Some(7));
  drop(second);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let at_checkpoint = crate::checkpoint::last(&kept).expect("the life kept its checkpoint");
  let ours = format!("\"engine\":\"{}\"", world::engine());
  assert!(
    fs::read_to_string(at.join("record.jsonl")).unwrap().contains(&ours),
    "the engine of this build wrote the checkpoint"
  );
  let tail = &kept[at_checkpoint + 1..];
  let carried = crate::checkpoint::booted(kept[..=at_checkpoint].to_vec()).len();
  let again = "rounds += 1\nclose(f'round {rounds}')";
  let (mut later, booted) = scripted(&at, &[again]).boot::<String>([]).unwrap();
  assert_eq!(
    booted.len(),
    carried + tail.len(),
    "a later life of the same engine boots on the checkpoint and the tail alone"
  );
  assert_eq!(&booted[carried..], tail);
  assert!(closed(&mut later, turns).as_ref().as_int() >= told, "each chain keeps its turns");
  let word = format!(
    "wake('chain2')\nclose([f(), after, get('{waiting}')[:2], peek('bash1').stdout.content, await Act('{waiting}')])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[6, 7, ('prompt', '{waiting}'), 'kept\\n', 'round 2']"),
    "the module of each chain and its open prompt go on, with what the tail added, and a command that no value names \
     keeps its outcome"
  );
}

#[test]
fn a_command_that_ran_at_a_checkpoint_ends_in_the_life_that_goes_on_from_it_as_interrupted() {
  let at = place("checkpoint-running");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  let with = verbs::Bash { on: Some("chain1".to_owned()), ..Default::default() };
  drop(first.bash("sleep 60", with).unwrap());
  first.checkpoint().unwrap();
  let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert!(later.pending().unwrap().is_empty(), "the checkpoint left nothing to wake");
  assert_eq!(
    closed(&mut later, "close(str(peek('bash1')))").as_ref().as_str(),
    Some("bash1 was interrupted: the life that ran it ended before it did"),
    "the command ends as the ear of commands ends one that a life left"
  );
}

#[test]
fn a_word_that_awaits_a_command_at_a_checkpoint_ends_as_cut_and_its_model_starts_it_again_in_a_turn()
 {
  let at = place("checkpoint-cut-word");
  let work = "await bash('sleep 60')\nclose('late')";
  let mut first = scripted(&at, &[work]).boot::<String>([]).unwrap().0;
  let asked = "waiting = prompt(str, 'Wait.', on=chain('work'))\nawait wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, asked).as_ref().as_str().unwrap().to_owned();
  let open = "close([x[1] for x in transcript('chain2', 'run') if not over(x[1])])";
  let runs = closed(&mut first, open).as_ref().py_repr();
  let run = runs.trim_start_matches("['").trim_end_matches("']").to_owned();
  assert!(
    run.starts_with("run") && !run.contains(','),
    "the word of chain2 awaits its command: {runs}"
  );
  first.checkpoint().unwrap();
  let mut later = scripted(&at, &["close('again')"]).boot::<String>([]).unwrap().0;
  let again = format!(
    "wake('chain2')\nclose([await Act('{waiting}'), str(peek('bash1')), \
     any(\"it cut ['{run}', 'bash1']\" in t[1] for t in turns('chain2'))])"
  );
  assert_eq!(
    closed(&mut later, &again).as_ref().py_repr(),
    "['again', 'bash1 was interrupted: the life that ran it ended before it did', True]",
    "the command ends as interrupted, the chain is told that the checkpoint cut the word, and the model of the \
     chain starts the word again in a turn that it reads"
  );
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let ran = crate::checkpoint::segment(&kept)
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(|a| {
      (a.kind() == "started" && a.about() == "bash1")
        || (a.kind() == "rung" && a.word(1).and_then(|one| one.as_str()) == Some(work))
    })
    .count();
  assert_eq!(ran, 0, "no word that ran at the checkpoint runs again, nor its command");
}

#[test]
fn a_life_that_keeps_no_record_refuses_a_checkpoint() {
  let at = place("checkpoint-unkept");
  let life = Opening { keeps: Some(false), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  assert_eq!(
    life.checkpoint().unwrap_err().message(),
    "a life that keeps no record keeps no checkpoint"
  );
}

#[test]
fn the_log_of_a_life_after_a_checkpoint_holds_the_facts_of_the_lives_before_it_and_agrees_with_its_record()
 {
  let at = place("checkpoint-log");
  let record = at.join("record.jsonl");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  let before = "before = 'said before'\nclose(before)";
  closed(&mut first, before);
  drop(first);
  let mut inspected =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let written = crate::checkpoint::written(&mut inspected).unwrap();
  drop(inspected);
  crate::checkpoint::keep(&record, written).unwrap();
  for word in ["close('said after')", "close('said last')"] {
    let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
    closed(&mut later, word);
    drop(later);
    let facts = world::logged(&record).unwrap().expect("the log agrees with its record");
    let words: Vec<String> = facts
      .iter()
      .filter_map(|one| {
        let items = one.as_ref().items()?;
        (items.first()?.as_str()? == "rung")
          .then(|| items.get(4)?.as_str().map(str::to_owned))
          .flatten()
      })
      .collect();
    for said in [before, word] {
      assert_eq!(words.iter().filter(|one| *one == said).count(), 1, "{said}");
    }
  }
}

#[test]
fn a_chain_that_only_a_value_names_goes_on_from_a_checkpoint_with_its_own_module() {
  let at = place("checkpoint-named");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  closed(&mut first, "kids = [chain('kid')]\nawait rung('n = 5', on=kids[0])\nclose(None)");
  drop(first);
  let mut inspected =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let written = crate::checkpoint::written(&mut inspected).unwrap();
  drop(inspected);
  crate::checkpoint::keep(&at.join("record.jsonl"), written).unwrap();
  let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut later, "close([kids, module(kids[0]).get('n')])").as_ref().py_repr(),
    "[['chain2'], 5]",
    "the checkpoint carries a chain that a value of a module names, with the values of its own module"
  );
}

#[test]
fn a_word_that_ran_at_a_checkpoint_is_cut_with_the_prompt_that_it_awaited() {
  let at = place("checkpoint-asked");
  let work = "got = await prompt(str, 'Ask.', on=chain('asked'))\nclose(got)";
  let mut first = scripted(&at, &[work, "await wait(3600, __name__)\nclose('late')"])
    .boot::<String>([])
    .unwrap()
    .0;
  let again = "waiting = prompt(str, 'Wait.', on=chain('work'))\nawait wait(0.5, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, again).as_ref().as_str().unwrap().to_owned();
  drop(first);
  let mut inspected =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let written = crate::checkpoint::written(&mut inspected).unwrap();
  drop(inspected);
  crate::checkpoint::keep(&at.join("record.jsonl"), written).unwrap();
  let mut later = scripted(&at, &["close('fresh')"]).boot::<String>([]).unwrap().0;
  let again = format!(
    "wake('chain2')\ngot = await Act('{waiting}')\nclose([got, [x[1] for c in ['chain1', 'chain2', 'chain3', 'chain4'] for x in transcript(c, 'prompt') if not over(x[1])], str(chain('more'))])"
  );
  assert_eq!(
    closed(&mut later, &again).as_ref().py_repr(),
    "['fresh', [], 'chain4']",
    "the model starts the cut word again in a new turn, no word runs again to make a chain, and the prompt that the \
     cut word awaited at the checkpoint is not carried"
  );
}

#[test]
fn a_record_that_another_engine_went_on_with_after_a_checkpoint_replays_its_tail_to_the_exact_state_at_its_tip()
 {
  let at = place("crossed");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nqueue = [1]\nSIZE = 6\ndef f():\n  return SIZE\n\
              waiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  closed(&mut first, word);
  first.checkpoint().unwrap();
  let mut tail =
    scripted(&at, &["await wait(3600, __name__)\nclose('late')"]).boot::<String>([]).unwrap().0;
  let word = "after = f() + 1\nqueue.append(after)\nmore = chain('more')\nasked = prompt(str, 'Ask.', on=more)\n\
              shell = bash('echo tail')\nawait shell\nawait wait(0.3, __name__)\nclose(str(asked))";
  let asked = closed(&mut tail, word).as_ref().as_str().unwrap().to_owned();
  drop(tail);
  // The facts after the checkpoint name another engine, as a build of another engine names them.
  let path = at.join("record.jsonl");
  let text = fs::read_to_string(&path).unwrap();
  let cut = text
    .rfind("[[\"checkpoint\"")
    .and_then(|at| text[at..].find('\n').map(|end| at + end))
    .unwrap();
  let theirs = text[cut..].replacen(&ours(), &made_of(&"0".repeat(64)), 1);
  fs::write(&path, format!("{}{theirs}", &text[..cut])).unwrap();
  let (mut later, booted) = scripted(&at, &["close('three')"]).boot::<String>([]).unwrap();
  assert!(booted.len() > 1, "the life replays the tail, word by word");
  let word = format!(
    "close([f(), after, queue, more, shell, peek(shell).stdout.content, get('{asked}')[:2], str(chain('last'))])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[6, 7, [1, 7], 'chain3', 'bash1', 'tail\\n', ('prompt', '{asked}'), 'chain4']"),
    "each name that the tail bound holds its value, each act that the tail made goes on, and a new act takes the next \
     name"
  );
  drop(later);
  let kept = world::kept(&path).unwrap();
  let checkpoints = kept.iter().filter(|one| {
    entry(&one.as_ref(), 0)
      .and_then(Fact::of)
      .is_some_and(|a| a.kind() == crate::checkpoint::CHECKPOINT)
  });
  assert_eq!(checkpoints.count(), 1, "the life writes no checkpoint that carries the tail");
  fs::remove_file(format!("{}.log", path.display())).unwrap();
  let told = world::retold(&path).unwrap();
  assert!(!told.logged, "the reader reads the record alone");
  let facts: Vec<String> = told.facts().unwrap().iter().map(|one| one.as_ref().py_repr()).collect();
  let heads = |kind: &str, of: &str| -> Vec<&String> {
    facts.iter().filter(|one| one.starts_with(&format!("['{kind}', '{of}', "))).collect()
  };
  assert!(
    heads("chain", "chain3").iter().any(|one| one.contains("'more'")),
    "the reader shows the chain that a word of the tail made, with its label: {facts:?}"
  );
  assert!(
    heads("prompt", &asked).len() == 1 && heads("done", &asked).is_empty(),
    "the reader shows the prompt that the word made on it, open: {facts:?}"
  );
}

#[test]
fn a_life_reads_the_classes_of_the_config_directory_that_it_opens_on_and_none_of_the_machine() {
  let at = place("classes");
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config/config.json"), r#"{"classes": {"claude-cli:opus": "cheap"}}"#).unwrap();
  let opening = Opening {
    roster: Some(vec!["claude-cli:opus".to_owned()]),
    extensions: Some(false),
    ..opening(&at, false)
  };
  let (mut engine, _) = opening.boot::<String>([]).unwrap();
  let standing = engine.standing().unwrap().py_repr();
  // The catalog gives opus the class dear, so only the config of this directory makes it cheap.
  assert!(standing.starts_with("[[['claude-cli:opus', ["), "{standing}");
  assert!(standing.contains("1000000, 'cheap']"), "{standing}");
}

/// A word that the gate refuses, since it gives a str to a name of type int, and that runs to its
/// close.
const MISTYPED: &str = "x: int = 'one'\nclose(x)";

/// What a word that a rung runs on the root closes with, or why it was refused.
fn rung(engine: &mut Engine, word: &str) -> Result<Object, crate::Fault> {
  let word = Some(word.to_owned());
  let rung =
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() })?;
  futures::executor::block_on(rung)
}

/// Each fact of the root, as python shows it.
fn facts(engine: &mut Engine) -> Vec<String> {
  let root =
    engine.transcript(verbs::Transcript { on: Some("chain1".to_owned()), of: None }).unwrap();
  root.iter().map(|one| one.0.as_ref().py_repr()).collect()
}

#[test]
fn a_config_that_turns_the_gate_off_runs_a_word_that_the_gate_refuses_and_its_replay_with_the_gate_on_holds_the_same_facts()
 {
  let on = place("gate-on");
  let open = |at: &Path, keeps: bool| Opening { extensions: Some(false), ..opening(at, keeps) };
  let (mut engine, _) = open(&on, false).boot::<String>([]).unwrap();
  let refused = rung(&mut engine, MISTYPED).unwrap_err();
  assert!(refused.to_string().contains("int"), "the gate on refuses the word: {refused}");
  drop(engine);
  let off = place("gate-off");
  fs::create_dir_all(off.join(".furb")).unwrap();
  fs::write(off.join(".furb/config.json"), r#"{"gate": false}"#).unwrap();
  let (mut engine, _) = open(&off, true).boot::<String>([]).unwrap();
  let ran = rung(&mut engine, MISTYPED).unwrap();
  assert_eq!(ran.as_ref().as_str(), Some("one"), "the gate off lets the word run");
  let lived = facts(&mut engine);
  let gated = lived.iter().find(|one| one.starts_with("('gate', ") && one.contains("x: int"));
  let gated = gated.expect("the record holds the gate of the word");
  let name = gated.split('\'').nth(3).unwrap_or_default();
  let answer = format!("('done', '{name}', 'gate', [])");
  assert!(lived.contains(&answer), "the gate off answers the word with no finding: {gated}");
  drop(engine);
  // A config of the project that turns the gate on again: the replay gives the answer that the
  // record holds.
  fs::write(off.join(".furb/config.local.json"), r#"{"gate": true}"#).unwrap();
  let (mut engine, _) = open(&off, false).boot::<String>([]).unwrap();
  assert!(engine.raised().is_none(), "the replay does not drift: {:?}", engine.raised());
  let replayed = facts(&mut engine);
  let mut left = replayed;
  let missing: Vec<_> = lived
    .iter()
    .filter(|one| match left.iter().position(|other| other == *one) {
      Some(at) => {
        left.remove(at);
        false
      }
      None => true,
    })
    .map(|one| one.chars().take(300).collect::<String>())
    .collect();
  assert!(missing.is_empty(), "the replay holds each fact of the life: {missing:#?}");
  assert_eq!(rung(&mut engine, "close(1)").unwrap().as_ref().as_int(), Some(1));
  assert!(rung(&mut engine, MISTYPED).is_err(), "the gate on refuses the word again in this life");
}

#[test]
fn the_gate_is_on_unless_the_last_config_that_names_it_says_false() {
  let at = place("gate-configs");
  let (user, furb) = (at.join("user"), at.join(".furb"));
  fs::create_dir_all(&user).unwrap();
  fs::create_dir_all(&furb).unwrap();
  let gated = || crate::gate::configured(&user, &at);
  assert!(gated(), "no config names the gate");
  fs::write(user.join("config.json"), r#"{"gate": false}"#).unwrap();
  assert!(!gated(), "the config of the user turns it off");
  fs::write(furb.join("config.json"), r#"{"gate": "no"}"#).unwrap();
  assert!(
    gated(),
    "configs that do not read, as with a value of the key that is no bool, leave it on"
  );
  fs::write(furb.join("config.local.json"), r#"{"gate": true}"#).unwrap();
  assert!(gated(), "the local config of the project wins");
}
