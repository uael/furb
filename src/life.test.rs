//! The opening of a life: what it enables, what it keeps, and the ears it hears, on the record that the store keeps.

#![expect(
  clippy::clone_on_ref_ptr,
  reason = "debt: a clone of an Arc or an Rc that hides its kind"
)]
#![expect(
  clippy::disallowed_methods,
  reason = "debt: a call that blocks: sleep, block_on or try_wait"
)]
#![expect(
  clippy::let_underscore_must_use,
  reason = "debt: a let _ drops a value that must be used"
)]
#![expect(clippy::let_underscore_untyped, reason = "debt: a let _ hides the type of what it drops")]
#![expect(clippy::similar_names, reason = "debt: names that differ by one letter")]
#![expect(clippy::single_char_pattern, reason = "debt: a string of one char as a pattern")]

use std::{
  cell::RefCell,
  collections::VecDeque,
  fs,
  path::{Path, PathBuf},
  rc::Rc,
  sync::{Arc, Mutex},
};

use super::{Answer, Opening};
use crate::{
  Ear, Engine, Fact, Object, Text,
  ear::{ear, hear, hearing, say},
  extension::enabled,
  value::{ObjectRef, entry},
  verbs, world,
};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-life-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
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
  assert_eq!(
    names(&mut first),
    ["catchup", "memory", "skills", "compaction", "naming", "ouroboros", "web", "mcp"]
  );
  drop(first);
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("config.json"), r#"{"extensions": {"skills": false}}"#).unwrap();
  let (mut later, record) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut later),
    ["catchup", "memory", "skills", "compaction", "naming", "ouroboros", "web", "mcp"],
    "the record wins over the configs"
  );
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert!(!kinds(&kept[record.len()..]).contains(&"enable".to_owned()), "it enabled nothing new");
  let fresh = Some(at.join("fresh.jsonl").display().to_string());
  let (mut fresh, _) = Opening { record: fresh, ..opening(&at, true) }.boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut fresh),
    ["catchup", "memory", "compaction", "naming", "ouroboros", "web", "mcp"],
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
    rung.and_then(|at| first[at].split("'").nth(3).map(str::to_owned)).unwrap_or_default();
  let run = first
    .iter()
    .position(|one| one.starts_with("['run', ") && one.contains(&format!(", '{named}', 'x = 1")));
  assert!(
    rung.is_some_and(|rung| run.is_some_and(|run| rung < run)),
    "the log holds the rung and its run"
  );
  assert!(
    !world::kept(&record).unwrap().iter().any(|one| one.as_ref().py_repr().contains("'run1'"))
  );
  drop(open());
  let again = shown(world::logged(&record).unwrap().expect("a later life writes the log again"));
  // A fact that the journal says again is its own, so a later life says each fact again with its kind and its name.
  let head = |one: &String| one.splitn(3, ", ").take(2).collect::<Vec<_>>().join(", ");
  let heads = |facts: &[String]| facts.iter().map(head).collect::<Vec<_>>();
  assert_eq!(
    heads(&again[..first.len()]),
    heads(&first),
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
    let opening = Opening { extensions: Some(false), snapshot: Some(false), ..opening(&at, true) };
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

/// The kind and the name of each fact of the log of a record, after the facts that a number of them leave out.
fn heads(record: &Path, after: usize) -> Vec<String> {
  let facts = world::logged(record).unwrap().expect("the log agrees with the record");
  let head =
    |one: &Object| one.as_ref().py_repr().splitn(3, ", ").take(2).collect::<Vec<_>>().join(", ");
  facts.iter().skip(after).map(head).collect()
}

#[test]
fn a_later_life_opens_from_the_snapshot_that_agrees_with_its_record_and_goes_on_as_a_replay_goes_on()
 {
  for extensions in [false, true] {
    goes_on(&place(&format!("snapshot-{extensions}")), extensions);
  }
}

/// A life that keeps a snapshot, and two later lives on its record, one from the snapshot and one that replays.
fn goes_on(at: &Path, extensions: bool) {
  let (record, again) = (at.join("record.jsonl"), at.join("again.jsonl"));
  for (name, text) in [("a.txt", "a\n"), ("b.txt", "b\n"), ("CLAUDE.md", "the memory\n")] {
    fs::write(at.join(name), text).unwrap();
  }
  let open = |record: &Path, snapshot: bool| {
    let record = Some(record.display().to_string());
    let opening = Opening {
      extensions: Some(extensions),
      snapshot: Some(snapshot),
      record,
      ..opening(at, true)
    };
    opening.boot::<String>([]).unwrap().0
  };
  let mut first = open(&record, true);
  assert!(!first.restored(), "a new record has no snapshot");
  assert_eq!(closed(&mut first, "x = 41\nread('a.txt')\nclose(x)").as_ref().as_int(), Some(41));
  drop(first);
  // A later life says each fact of the first life again, or holds it from the snapshot, before its own.
  let one = heads(&record, 0).len();
  for end in ["", ".log", ".snapshot"] {
    fs::copy(format!("{}{end}", record.display()), format!("{}{end}", again.display())).unwrap();
  }
  let mut restored = open(&record, true);
  let mut replayed = open(&again, false);
  assert!(restored.restored(), "the snapshot agrees with the record");
  assert!(!replayed.restored(), "a life that takes no snapshot replays");
  let word = "read('b.txt')\nclose(x + 1)";
  assert_eq!(
    closed(&mut restored, word).as_ref().as_int(),
    Some(42),
    "the module holds what the word bound"
  );
  assert_eq!(closed(&mut replayed, word).as_ref().as_int(), Some(42));
  drop((restored, replayed));
  let (restored, replayed) = (heads(&record, one), heads(&again, one));
  assert_eq!(restored, replayed, "both lives go on with the same facts");
  let facts = world::logged(&record).unwrap().unwrap();
  // The catch-up of a stand reads each file again, so only a tell counts as what a chain is told.
  let told = facts[one..]
    .iter()
    .filter(|one| {
      one.as_ref().py_repr().starts_with("['tell'") && one.as_ref().py_repr().contains("the memory")
    })
    .count();
  let first =
    facts[..one].iter().filter(|one| one.as_ref().py_repr().contains("the memory")).count();
  assert!(!extensions || first > 0, "the first life told the memory");
  assert_eq!(told, 0, "the memory ear heard the past, so it tells no chain again what it holds");
}

/// An ear of the host that waits for a chain by its name, and keeps the kind and the act of each fact it hears
/// that is no act.
fn listening(chain: &'static str, heard: Rc<RefCell<Vec<String>>>) -> Box<dyn Ear> {
  hearing(&[chain], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if !a.question() {
        heard.borrow_mut().push(format!("{} {}", a.kind(), a.about()));
      }
    }
  })
}

#[test]
fn an_ear_that_a_life_from_its_snapshot_tells_the_past_hears_what_it_hears_in_a_replay() {
  let at = place("hearing");
  let (record, again) = (at.join("record.jsonl"), at.join("again.jsonl"));
  let open = |record: &Path, snapshot: bool, heard: &Rc<RefCell<Vec<String>>>| {
    let record = Some(record.display().to_string());
    let opening = Opening { snapshot: Some(snapshot), record, ..opening(&at, true) };
    opening.boot([("listening", listening("chain2", heard.clone()))]).unwrap().0
  };
  let mut first = open(&record, true, &Rc::default());
  // A message on the thread of a prompt stands on that prompt, and the chain of that prompt names it alone.
  let word = "side = chain('side')\nasked = prompt(None, 'ask', to=OPERATOR, on=side)\n\
    more = prompt(None, 'more', on=asked)\nclose(None, more)\nclose(None, asked)\nclose(1)";
  assert_eq!(closed(&mut first, word).as_ref().as_int(), Some(1));
  drop(first);
  for end in ["", ".log", ".snapshot"] {
    fs::copy(format!("{}{end}", record.display()), format!("{}{end}", again.display())).unwrap();
  }
  let (told, replayed) = (Rc::default(), Rc::default());
  let restored = open(&record, true, &told);
  let replay = open(&again, false, &replayed);
  assert!(restored.restored() && !replay.restored());
  assert!(replayed.borrow().iter().any(|one| one == "tell prompt2"), "{:?}", replayed.borrow());
  assert_eq!(*told.borrow(), *replayed.borrow());
}

#[test]
fn a_snapshot_that_its_record_outgrew_is_dropped_and_the_life_replays() {
  let at = place("outgrew");
  let record = at.join("record.jsonl");
  let open =
    || Opening { extensions: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap().0;
  let mut first = open();
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  let snapshot = fs::read(format!("{}.snapshot", record.display())).unwrap();
  let mut second = open();
  assert!(second.restored());
  closed(&mut second, "close(x)");
  drop(second);
  fs::write(format!("{}.snapshot", record.display()), snapshot).unwrap();
  let mut third = open();
  assert!(!third.restored(), "the record holds entries after those that the snapshot covers");
  assert_eq!(closed(&mut third, "close(x + 1)").as_ref().as_int(), Some(2));
}

#[test]
fn a_life_whose_outside_works_keeps_a_snapshot_and_a_later_life_holds_that_work_as_a_replay_holds_it()
 {
  let at = place("works");
  let (record, again) = (at.join("record.jsonl"), at.join("again.jsonl"));
  let open = |record: &Path, snapshot: bool| {
    let record = Some(record.display().to_string());
    let opening =
      Opening { extensions: Some(false), snapshot: Some(snapshot), record, ..opening(&at, true) };
    opening.boot::<String>([]).unwrap().0
  };
  let mut first = open(&record, true);
  let on = Some("chain1".to_owned());
  drop(first.wait(&[], verbs::Wait { seconds: Some(60.0), on: on.clone() }).unwrap());
  drop(first.bash("sleep 30", verbs::Bash { on, ..Default::default() }).unwrap());
  drop(first);
  let one = heads(&record, 0).len();
  for end in ["", ".log", ".snapshot"] {
    fs::copy(format!("{}{end}", record.display()), format!("{}{end}", again.display())).unwrap();
  }
  let mut restored = open(&record, true);
  let mut replayed = open(&again, false);
  assert!(restored.restored(), "the snapshot covers the work that ran when the life ended");
  let left = restored.pending().unwrap();
  assert_eq!(
    left,
    [("wait1", "wait"), ("bash1", "bash")].map(|(a, b)| (a.to_owned(), b.to_owned()))
  );
  assert_eq!(
    left,
    replayed.pending().unwrap(),
    "the work that ran waits for a wake, as in a replay"
  );
  for engine in [&mut restored, &mut replayed] {
    engine.wake("chain1").unwrap();
    assert!(engine.pending().unwrap().is_empty(), "the wake put that work to the outside again");
  }
  drop((restored, replayed));
  // The life from the snapshot stands again at its tip, where the root reads the clock, and each ear hears the wake
  // in its own order.
  let acts = |record: &Path| {
    let mut facts = heads(record, one);
    facts.retain(|one| {
      !one.contains("'stand") && !one.contains("'clock") && !one.starts_with("['tell', 'chain1'")
    });
    facts.sort();
    facts
  };
  let (restored, replayed) = (acts(&record), acts(&again));
  assert!(restored.iter().any(|one| one.starts_with("['started', 'wait1'")), "{restored:?}");
  assert_eq!(restored, replayed, "both lives go on with the same facts");
}

#[test]
fn a_snapshot_of_another_build_or_on_another_log_is_not_taken_and_the_life_replays() {
  let at = place("another");
  let record = at.join("record.jsonl");
  let open =
    || Opening { extensions: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap().0;
  let mut first = open();
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  let snapshot = PathBuf::from(format!("{}.snapshot", record.display()));
  let held = fs::read(&snapshot).unwrap();
  let line = held.iter().position(|one| *one == b'\n').unwrap();
  let head = String::from_utf8(held[..line].to_vec()).unwrap();
  let build = head.split("\"build\":\"").nth(1).unwrap().split('"').next().unwrap().to_owned();
  let other =
    [head.replace(&build, &"0".repeat(build.len())).into_bytes(), held[line..].to_vec()].concat();
  fs::write(&snapshot, other).unwrap();
  let second = open();
  assert!(!second.restored(), "a snapshot of another build is not taken");
  drop(second);
  assert!(open().restored(), "the life that replayed kept a snapshot of this build");
  let log = format!("{}.log", record.display());
  let text = fs::read_to_string(&log).unwrap();
  fs::write(&log, text.replacen("\"chain1\"", "\"chain1\" ", 1)).unwrap();
  assert!(!open().restored(), "a snapshot of another log is not taken");
}

/// The build that each fact made of a record names, in its order: its engine, and its version when it names one.
fn builds(record: &Path) -> Vec<(String, Option<i64>)> {
  let kept = world::kept(record).unwrap();
  let fact = |one: &Object| entry(&one.as_ref(), 0).and_then(Fact::of).unwrap();
  let named = |one: Fact| {
    let engine = one.word(0).and_then(|word| word.as_str().map(str::to_owned))?;
    Some((engine, one.word(1).and_then(|word| word.as_int())))
  };
  kept.iter().map(fact).filter(|one| one.kind() == world::MADE).filter_map(named).collect()
}

#[test]
fn a_record_names_the_engine_and_the_version_that_made_it_once_in_a_file_of_lines_and_in_a_database()
 {
  for name in ["record.jsonl", "record.db"] {
    let at = place(&format!("made-{name}"));
    let record = at.join(name);
    let open = || {
      let path = Some(record.display().to_string());
      Opening { record: path, extensions: Some(false), ..opening(&at, true) }.boot::<String>([])
    };
    drop(open().unwrap());
    let made = vec![(world::engine(), Some(world::VERSION))];
    assert_eq!(builds(&record), made);
    drop(open().unwrap());
    assert_eq!(builds(&record), made, "a later life names it no more");
  }
}

#[test]
fn a_record_that_names_no_engine_replays_as_before_and_names_none() {
  let at = place("unnamed");
  let record = at.join("record.jsonl");
  let open = || Opening { extensions: Some(false), snapshot: Some(false), ..opening(&at, true) };
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
  assert_eq!(builds(&record), Vec::new(), "this engine did not make it");
}

/// A record of one rung that a life of a test kept, and the text of the record with its fact made changed.
fn made_elsewhere(at: &Path, made: &str) -> (PathBuf, String) {
  let record = at.join("record.jsonl");
  let mut first = Opening { extensions: Some(false), snapshot: Some(false), ..opening(at, true) }
    .boot::<String>([])
    .unwrap()
    .0;
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  let ours = format!("[\"made\",\"chain1\",\"store\",\"{}\",{}]", world::engine(), world::VERSION);
  let text = fs::read_to_string(&record).unwrap();
  assert!(text.contains(&ours), "the record names this build");
  let text = text.replace(&ours, made);
  fs::write(&record, &text).unwrap();
  (record, text)
}

#[test]
fn a_record_of_another_engine_of_the_same_version_replays_and_goes_on() {
  let at = place("same-version");
  let made = format!("[\"made\",\"chain1\",\"store\",\"{}\",{}]", "0".repeat(64), world::VERSION);
  let (record, _) = made_elsewhere(&at, &made);
  let open = || Opening { extensions: Some(false), snapshot: Some(false), ..opening(&at, true) };
  let mut later = open().boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut later, "close(x + 1)").as_ref().as_int(), Some(2));
  drop(later);
  assert_eq!(
    builds(&record),
    [("0".repeat(64), Some(world::VERSION))],
    "the build that made it stays named"
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
    snapshot: Some(false),
    ..opening(&at, true)
  }
  .boot::<String>([])
  .unwrap();
  assert_eq!(
    asked(),
    1,
    "the opening asked the models, and the stand of the boot holds what it found"
  );
  stood(&mut engine).unwrap();
  assert_eq!(asked(), 2, "a later stand asks the command line again");
}

#[test]
fn a_life_opens_when_its_actor_and_a_roster_name_left_the_catalog_and_tells_each_chain_once() {
  let at = place("catalog-left");
  let (mut engine, _) = Opening {
    extensions: Some(false),
    snapshot: Some(false),
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
    snapshot: Some(false),
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
   wait(0.01)\nhelper = chain(\"helper\")\ngot = await prompt(int, \"Give 2.\", on=helper)\nclose(got + \
   len(notes.lines) + int(clock() > 0) + int(0 <= chance() < 1))",
  "close(2)",
];

/// The record of the life of `WORK` in a directory of its own: a first life works, and a second
/// life on the record takes one prompt more, so the record holds two openings. Each official
/// extension runs on it.
fn worked(name: &str) -> PathBuf {
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
  at.join("record.jsonl")
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
  let config = Some(format!("{}-config", at.display()));
  let refused =
    Opening { config, ..opening(&at, true) }.boot::<String>([]).map(|_| ()).unwrap_err();
  assert!(refused.to_string().starts_with("Drift: "), "{refused}");
  let moved = format!("stand()\\ncd('{place}-elsewhere')");
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
fn each_record_of_the_replays_names_this_version_and_replays_on_this_build_with_no_drift() {
  let mut records: Vec<PathBuf> = fs::read_dir(replays())
    .unwrap()
    .map(|one| one.unwrap().path())
    .filter(|one| one.extension().is_some_and(|named| named == "jsonl"))
    .collect();
  records.sort();
  assert!(!records.is_empty(), "tests/replay holds no record");
  for one in records {
    let name = one.file_stem().unwrap().to_string_lossy().into_owned();
    let at = place(&format!("replay-{name}"));
    fs::copy(&one, at.join("record.jsonl")).unwrap();
    let kept = world::kept(at.join("record.jsonl")).unwrap();
    assert_eq!(
      world::version(&kept),
      Some(world::VERSION),
      "{name} is a record of another version: make it again, as tests/replay/README.md says"
    );
    let inspects = Opening { inspecting: Some(true), ..opening(&at, false) };
    if let Err(drift) = inspects.boot::<String>([]) {
      panic!(
        "{name} drifts on this build: add one to world::VERSION and make it again, as tests/replay/README.md says. {drift}"
      );
    }
  }
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

/// Each record of `tests/replay`, and the record of the life of `WORK`, replayed in a life that
/// only inspects it, by its name.
fn replayed(by: &str) -> Vec<(String, Engine)> {
  let mut records: Vec<PathBuf> = fs::read_dir(replays())
    .unwrap()
    .map(|one| one.unwrap().path())
    .filter(|one| one.extension().is_some_and(|named| named == "jsonl"))
    .collect();
  records.sort();
  let work = worked(&format!("shown-{by}-made")).with_file_name("work.jsonl");
  fs::rename(work.with_file_name("record.jsonl"), &work).unwrap();
  records.push(work);
  let open = |one: PathBuf| {
    let name = one.file_stem().unwrap().to_string_lossy().into_owned();
    let at = place(&format!("shown-{by}-{name}"));
    fs::copy(&one, at.join("record.jsonl")).unwrap();
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
    Some(8),
    "each official extension runs on the life of work"
  );
  let unseeded = engine.word(UNSEEDED, vec![]).unwrap();
  assert_eq!(unseeded.as_ref().items().unwrap().len(), 0, "{}", unseeded.as_ref().py_repr());
}

#[test]
#[ignore = "a measure, which a person runs in release"]
fn measure_how_long_a_life_opens_from_its_snapshot_and_by_a_replay() {
  let open = |at: &Path, snapshot: bool| {
    let opening = Opening { snapshot: Some(snapshot), ..scripted(at, &[]) };
    opening.boot::<String>([]).unwrap().0
  };
  let number = |name: &str, or: usize| -> usize {
    std::env::var(name).ok().and_then(|one| one.parse().ok()).unwrap_or(or)
  };
  let (n, size, tail) = (number("RUNGS", 200), number("CHARS", 0), number("TAIL", 20));
  let word = |name: &str, i: usize| match size {
    0 => format!("{name}{i} = [{i}] * 50\nclose(len({name}{i}))"),
    // A rung that closes with a large text weighs on the record, the log and the snapshot, as a
    // command does.
    _ => format!("{name}{i} = '{i} ' * {}\nclose({name}{i})", size / 2),
  };
  // One record that a life of n rungs made, and one where that life ended with its checkpoint and a
  // later life added a tail of TAIL rungs: each later life opens from the snapshot that the last
  // life kept as it ended, from the checkpoint and the tail, or by a replay of the whole record.
  let (whole, cut) = (place("measure"), place("measure-checkpoint"));
  for at in [&whole, &cut] {
    let mut engine = open(at, true);
    for i in 0..n {
      closed(&mut engine, &word("x", i));
    }
    if *at == cut {
      engine.checkpoint().unwrap();
      engine = open(at, true);
      for i in 0..tail {
        closed(&mut engine, &word("y", i));
      }
    }
    // A life that works always holds work of the outside that runs, as a wait.
    let wait = verbs::Wait { seconds: Some(3600.0), on: Some("chain1".to_owned()) };
    drop(engine.wait(&[], wait).unwrap());
    drop(engine);
  }
  let timed = |at: &Path, snapshot: bool| {
    let t = std::time::Instant::now();
    let engine = open(at, snapshot);
    let spent = t.elapsed();
    assert_eq!(engine.restored(), snapshot);
    spent
  };
  for at in [&whole, &cut] {
    let kept = world::kept(at.join("record.jsonl")).unwrap();
    let after = kept.len() - crate::checkpoint::last(&kept).map_or(0, |one| one + 1);
    let bytes = fs::metadata(at.join("record.jsonl")).unwrap().len();
    // A life that a checkpoint cut keeps no snapshot, so that record opens from its checkpoint
    // alone.
    let snapshot = fs::metadata(at.join("record.jsonl.snapshot")).map(|one| one.len()).ok();
    let from = snapshot.map(|_| timed(at, true));
    eprintln!(
      "rungs {n}, tail {tail}, entries {}, {after} to replay, record {bytes} bytes, snapshot {snapshot:?} bytes: \
       from the snapshot {from:?}, with no snapshot {:?}",
      kept.len(),
      timed(at, false)
    );
  }
}

#[test]
fn a_life_goes_on_from_a_checkpoint_with_the_names_the_module_and_the_open_prompt_of_each_chain() {
  let at = place("checkpoint");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nran = await bash('echo one')\nqueue = [ran]\nSIZE = 6\ndef f():\n  return SIZE\n\
              waiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  // The checkpoint is written as the command line writes it: in a life that only replays the record.
  let mut inspected =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let written = crate::checkpoint::written(&mut inspected).unwrap();
  drop(inspected);
  crate::checkpoint::keep(&at.join("record.jsonl"), written).unwrap();
  let mut later = scripted(&at, &["close('again')"]).boot::<String>([]).unwrap().0;
  let word = format!(
    "wake('chain2')\nclose([str(ran), queue, f(), get('{waiting}')[:2], await waiting, str(chain('more')), str(bash('echo two'))])"
  );
  let exit = "Exit(code=0, stdout=Text(path='bash1/stdout', content='one\\n'), stderr=Text(path='bash1/stderr', content=''))";
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[{exit:?}, [{exit}], 6, ('prompt', '{waiting}'), 'round 2', 'chain3', 'bash2']"),
    "each name, value and answer of the past stands, the word that ran runs again from its top on the module that \
     the checkpoint carries and closes the open prompt with no new turn of the model, and a new act takes the next name"
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
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0)\nclose('late')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nshell = bash('echo one')\nawait shell\nqueue = [shell]\nSIZE = 6\n\
              def f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  first.checkpoint().unwrap();
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let fact = entry(&kept[crate::checkpoint::last(&kept).unwrap()].as_ref(), 0).and_then(Fact::of);
  let data = crate::wire::outward(fact.unwrap().word(0).unwrap());
  assert_eq!(data["schema"], serde_json::json!(crate::checkpoint::SCHEMA));
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
  let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
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
fn a_checkpoint_of_a_later_schema_is_refused_with_both_schemas() {
  let at = place("checkpoint-schema");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  closed(&mut first, "kept = 1\nclose(kept)");
  first.checkpoint().unwrap();
  let text = fs::read_to_string(at.join("record.jsonl")).unwrap();
  let schema = format!("\"schema\":{}", crate::checkpoint::SCHEMA);
  assert!(text.contains(&schema), "the checkpoint names its schema");
  let later = format!("\"schema\":{}", crate::checkpoint::SCHEMA + 1);
  fs::write(at.join("record.jsonl"), text.replace(&schema, &later)).unwrap();
  let refused = scripted(&at, &[]).boot::<String>([]).unwrap_err();
  assert_eq!(
    refused.message(),
    format!(
      "the checkpoint of the record holds the schema {}, and this build reads the schema {} at most: open the \
       record with a later build",
      crate::checkpoint::SCHEMA + 1,
      crate::checkpoint::SCHEMA
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
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  closed(&mut first, "await bash('echo kept')\nclose(None)");
  let word = "work = chain('work')\nSIZE = 6\ndef f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\n\
              await wait(0.3)\nclose(str(waiting))";
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
  let (mut later, booted) = scripted(&at, &[]).boot::<String>([]).unwrap();
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
    Some("\"sleep 60\" was interrupted: the life that ran it ended before it did"),
    "the command ends as the ear of commands ends one that a life left"
  );
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
fn a_word_that_ran_at_a_checkpoint_runs_again_and_asks_again_the_prompt_that_it_awaited() {
  let at = place("checkpoint-asked");
  let work = "got = await prompt(str, 'Ask.', on=chain('asked'))\nclose(got)";
  let mut first =
    scripted(&at, &[work, "await wait(3600)\nclose('late')"]).boot::<String>([]).unwrap().0;
  let word =
    "waiting = prompt(str, 'Wait.', on=chain('work'))\nawait wait(0.5)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  let mut inspected =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let written = crate::checkpoint::written(&mut inspected).unwrap();
  drop(inspected);
  crate::checkpoint::keep(&at.join("record.jsonl"), written).unwrap();
  let mut later = scripted(&at, &["close('fresh')"]).boot::<String>([]).unwrap().0;
  let word = format!(
    "wake('chain2')\ngot = await Act('{waiting}')\nclose([got, [x[1] for c in ['chain1', 'chain2', 'chain3', 'chain4'] for x in transcript(c, 'prompt') if not over(x[1])], str(chain('more'))])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    "['fresh', [], 'chain5']",
    "the word asks its prompt again on a new chain, and the prompt that it awaited at the checkpoint is not carried"
  );
}

#[test]
fn a_record_of_another_version_opens_on_this_build_from_its_last_checkpoint() {
  let at = place("upgrade");
  let path = at.join("record.jsonl");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut first, "kept = 1\nclose(kept)").as_ref().as_int(), Some(1));
  let written = crate::checkpoint::written(&mut first).unwrap();
  drop(first);
  crate::checkpoint::keep(&path, written).unwrap();
  let made =
    |engine: &str, version: i64| format!("[\"made\",\"chain1\",\"store\",\"{engine}\",{version}]");
  let text = fs::read_to_string(&path).unwrap();
  let ours = made(&world::engine(), world::VERSION);
  assert!(text.contains(&ours), "{text}");
  fs::write(&path, text.replace(&ours, &made(&"0".repeat(64), world::VERSION + 1))).unwrap();
  let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut later, "close(kept + 1)").as_ref().as_int(),
    Some(2),
    "only the build after the checkpoint counts"
  );
  assert_eq!(
    closed(
      &mut later,
      "close([x[3][0] for x in transcript('chain1', 'tell') if 'rai' + 'sed = ' in x[3][0]])"
    )
    .as_ref()
    .py_repr(),
    "[]",
    "the carried turns hold no answer of an act that the checkpoint does not carry, so no word of an extension raises"
  );
}

#[test]
fn a_record_that_another_engine_went_on_with_after_a_checkpoint_opens_with_each_chain_its_turns_and_its_open_prompts()
 {
  let at = place("crossed");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nqueue = [1]\nSIZE = 6\ndef f():\n  return SIZE\n\
              waiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  first.checkpoint().unwrap();
  let mut tail = scripted(&at, &["await wait(3600)\nclose('late')"]).boot::<String>([]).unwrap().0;
  let word = "after = f() + 1\nqueue.append(after)\nmore = chain('more')\nasked = prompt(str, 'Ask.', on=more)\n\
              shell = bash('echo tail')\nawait shell\nawait wait(0.3)\nclose(str(asked))";
  let asked = closed(&mut tail, word).as_ref().as_str().unwrap().to_owned();
  let turns = "close([len(turns('chain1')), len(turns('chain3'))])";
  let counted = |life: &mut Engine| -> Vec<i64> {
    let got = closed(life, turns);
    got.as_ref().items().unwrap().iter().filter_map(ObjectRef::as_int).collect()
  };
  let told = counted(&mut tail);
  drop(tail);
  // The facts after the checkpoint name another engine, as a build of another engine names them.
  let path = at.join("record.jsonl");
  let text = fs::read_to_string(&path).unwrap();
  let made =
    |engine: &str| format!("[\"made\",\"chain1\",\"store\",\"{engine}\",{}]", world::VERSION);
  let cut = text
    .rfind("[[\"checkpoint\"")
    .and_then(|at| text[at..].find('\n').map(|end| at + end))
    .unwrap();
  let theirs = text[cut..].replacen(&made(&world::engine()), &made(&"0".repeat(64)), 1);
  let text = format!("{}{theirs}", &text[..cut]);
  fs::write(&path, &text).unwrap();
  drop(Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap());
  assert_eq!(
    fs::read_to_string(&path).unwrap(),
    text,
    "a life that inspects the record writes nothing"
  );
  let (mut later, booted) = scripted(&at, &["close('three')"]).boot::<String>([]).unwrap();
  assert_eq!(
    booted.len(),
    1,
    "the life boots on a checkpoint that carries the tail, and replays no fact of it"
  );
  let now = counted(&mut later);
  assert!(
    now.iter().zip(&told).all(|(now, told)| now >= told),
    "each chain keeps its turns: {now:?} {told:?}"
  );
  let word = format!(
    "wake('chain3')\nclose([f(), 'after' in globals(), 'queue' in globals(), get('{waiting}')[:2], \
     over('{waiting}'), peek(bash1), await Act('{asked}'), str(chain('last'))])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!(
      "[6, False, False, ('prompt', '{waiting}'), False, Exit(code=0, stdout=Text(path='bash1/stdout', \
       content='tail\\n'), stderr=Text(path='bash1/stderr', content='')), 'three', 'chain4']"
    ),
    "each definition of the checkpoint, each open prompt and each act that a tell names goes on, a name that the tail \
     changed is lost, and a new act takes the next name"
  );
  let word = "close([x[3][0] for x in transcript('chain1') if x[0] == 'tell' and x[3][0].startswith('#') and 'It lost' in x[3][0]][-1])";
  let lost = closed(&mut later, word).as_ref().as_str().unwrap().to_owned();
  eprintln!("LOST {}", lost.replace('\n', " | "));
  for name in ["after", "queue", "more", "asked", "shell"] {
    assert!(lost.contains(&format!("'{name}'")), "the chain is told that it lost {name}: {lost}");
  }
  drop(later);
  let kept = world::kept(&path).unwrap();
  let named: Vec<Option<String>> = crate::checkpoint::segment(&kept)
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(|a| a.kind() == world::MADE)
    .map(|a| a.word(0).and_then(|one| one.as_str().map(str::to_owned)))
    .collect();
  assert_eq!(named, [Some(world::engine())], "the life after the carry names this engine once");
  let mut again = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut again, "close(f())").as_ref().as_int(), Some(6));
  drop(again);
  let kept = world::kept(&path).unwrap();
  let checkpoints = kept.iter().filter(|one| {
    entry(&one.as_ref(), 0)
      .and_then(Fact::of)
      .is_some_and(|a| a.kind() == crate::checkpoint::CHECKPOINT)
  });
  assert_eq!(
    checkpoints.count(),
    2,
    "a later life of this engine replays the tail that it made, and carries nothing"
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
