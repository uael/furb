//! The opening of a life: what it enables, what it keeps, and the ears it hears, on the record that the store keeps.

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
  value::entry,
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
  assert_eq!(names(&mut first), ["memory", "skills", "compaction", "ouroboros", "web"]);
  drop(first);
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("config.json"), r#"{"extensions": {"skills": false}}"#).unwrap();
  let (mut later, record) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut later),
    ["memory", "skills", "compaction", "ouroboros", "web"],
    "the record wins over the configs"
  );
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert!(!kinds(&kept[record.len()..]).contains(&"enable".to_owned()), "it enabled nothing new");
  let fresh = Some(at.join("fresh.jsonl").display().to_string());
  let (mut fresh, _) = Opening { record: fresh, ..opening(&at, true) }.boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut fresh),
    ["memory", "compaction", "ouroboros", "web"],
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
    ["provider", "extensions", "memory", "skills", "web", "files", "bash", "time", "store", "log"]
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
  let rung =
    first.iter().position(|one| one.starts_with("['rung', 'rung1', 'operator', 'chain1', 'x = 1"));
  let run = first.iter().position(|one| one.starts_with("['run', 'run1', "));
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
  let told =
    facts[one..].iter().filter(|one| one.as_ref().py_repr().contains("the memory")).count();
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
fn a_life_whose_outside_works_keeps_no_snapshot_of_that_work() {
  let at = place("works");
  let open =
    || Opening { extensions: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap().0;
  let mut first = open();
  let wait = first.wait(&[], verbs::Wait { seconds: Some(60.0), on: Some("chain1".to_owned()) });
  drop(wait.unwrap());
  drop(first);
  let second = open();
  assert!(
    !second.restored(),
    "the snapshot that the life kept before the wait covers fewer entries than the record"
  );
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

#[test]
fn a_life_on_a_record_of_another_version_is_refused_before_any_replay_and_says_both_versions_and_what_to_do()
 {
  let at = place("other-version");
  let other = world::VERSION + 1;
  let (record, text) = made_elsewhere(
    &at,
    &format!("[\"made\",\"chain1\",\"store\",\"{}\",{other}]", world::engine()),
  );
  let open = || Opening { extensions: Some(false), ..opening(&at, true) };
  for life in [open(), Opening { inspecting: Some(true), ..open() }] {
    let refused = life.boot::<String>([]).map(|_| ()).unwrap_err();
    assert_eq!(
      refused.to_string(),
      format!(
        "Drift: the record {} was made by a build of version {other}, and this build is of version {}, so a replay \
         would drift. Open the record with the build of version {other}, which stands at {} once a build of that \
         version ran on this machine, and which script/builds.sh {other} makes there. Or start a new life.",
        record.display(),
        world::VERSION,
        world::build(other).unwrap().display()
      )
    );
  }
  assert_eq!(fs::read_to_string(&record).unwrap(), text, "it keeps nothing");
}

#[test]
fn a_life_on_a_record_of_another_engine_before_the_versions_is_refused_before_any_replay_and_says_both_engines()
 {
  let at = place("other");
  let (record, text) =
    made_elsewhere(&at, &format!("[\"made\",\"chain1\",\"store\",\"{}\"]", "0".repeat(64)));
  let open = || Opening { extensions: Some(false), ..opening(&at, true) };
  for life in [open(), Opening { inspecting: Some(true), ..open() }] {
    let refused = life.boot::<String>([]).map(|_| ()).unwrap_err();
    assert_eq!(
      refused.to_string(),
      format!(
        "Drift: the record {} was made by the engine 000000000000, and this build runs the engine {:.12}, so a \
         replay would drift. Open the record with the build that made it, or start a new life.",
        record.display(),
        world::engine()
      )
    );
  }
  assert_eq!(fs::read_to_string(&record).unwrap(), text, "it keeps nothing");
}

/// The corpus that the gate of the versions replays: the records in `tests/corpus`, which `tests/corpus/README.md`
/// says.
fn corpus() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("corpus")
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

/// The words of the model of the life of the corpus: the root writes, reads, runs a command, waits, reads the
/// clock and chance, and asks a chain of its own, whose model answers in the second word.
const WORK: [&str; 2] = [
  "write(Text(\"notes.md\", \"one\\ntwo\\n\"))\nnotes = read(\"notes.md\")\nsaid = await bash(\"echo hi\")\nawait \
   wait(0.01)\nhelper = chain(\"helper\")\ngot = await prompt(int, \"Give 2.\", on=helper)\nclose(got + \
   len(notes.lines) + int(clock() > 0) + int(0 <= chance() < 1))",
  "close(2)",
];

#[test]
#[ignore = "it makes the corpus again, which a person runs after a change that adds one to the version"]
fn make_the_corpus() {
  let at = place("corpus");
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
  fs::create_dir_all(corpus()).unwrap();
  fs::copy(at.join("record.jsonl"), corpus().join("work.jsonl")).unwrap();
}

#[test]
fn each_record_of_the_corpus_names_this_version_and_replays_on_this_build_with_no_drift() {
  let mut records: Vec<PathBuf> = fs::read_dir(corpus())
    .unwrap()
    .map(|one| one.unwrap().path())
    .filter(|one| one.extension().is_some_and(|named| named == "jsonl"))
    .collect();
  records.sort();
  assert!(!records.is_empty(), "the corpus holds no record");
  for one in records {
    let name = one.file_stem().unwrap().to_string_lossy().into_owned();
    let at = place(&format!("corpus-{name}"));
    fs::copy(&one, at.join("record.jsonl")).unwrap();
    let kept = world::kept(at.join("record.jsonl")).unwrap();
    assert_eq!(
      world::version(&kept),
      Some(world::VERSION),
      "{name} is a record of another version: make the corpus again, as tests/corpus/README.md says"
    );
    let inspects = Opening { inspecting: Some(true), ..opening(&at, false) };
    if let Err(drift) = inspects.boot::<String>([]) {
      panic!(
        "{name} drifts on this build: add one to world::VERSION and make the corpus again, as tests/corpus/README.md says. {drift}"
      );
    }
  }
}

#[test]
#[ignore = "a measure, which a person runs in release"]
fn measure_how_long_a_life_opens_from_its_snapshot_and_by_a_replay() {
  let at = place("measure");
  let open = |snapshot: bool| {
    let opening =
      Opening { extensions: Some(false), snapshot: Some(snapshot), ..opening(&at, true) };
    opening.boot::<String>([]).unwrap().0
  };
  let n: usize = std::env::var("RUNGS").ok().and_then(|one| one.parse().ok()).unwrap_or(200);
  let mut engine = open(true);
  for i in 0..n {
    closed(&mut engine, &format!("x{i} = [{i}] * 50\nclose(len(x{i}))"));
  }
  drop(engine);
  let entries = world::kept(at.join("record.jsonl")).unwrap().len();
  let size = fs::metadata(at.join("record.jsonl.snapshot")).unwrap().len();
  let t = std::time::Instant::now();
  let restored = open(true);
  let from = t.elapsed();
  assert!(restored.restored());
  drop(restored);
  let t = std::time::Instant::now();
  drop(open(false));
  eprintln!(
    "rungs {n}, entries {entries}, snapshot {size} bytes: from the snapshot {from:?}, by a replay {:?}",
    t.elapsed()
  );
}

#[test]
fn a_life_goes_on_from_a_checkpoint_with_the_names_the_module_and_the_open_prompt_of_each_chain() {
  let at = place("checkpoint");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nran = await bash('echo one')\nqueue = [ran]\nCAP = 6\ndef f():\n  return CAP\n\
              waiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  // The checkpoint is written as the command line writes it: in a life that only replays the record.
  let (mut inspected, kept) =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap();
  let written = crate::checkpoint::written(&mut inspected, &kept).unwrap();
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
fn a_word_that_ran_at_a_checkpoint_runs_again_and_asks_again_the_prompt_that_it_awaited() {
  let at = place("checkpoint-asked");
  let work = "got = await prompt(str, 'Ask.', on=chain('asked'))\nclose(got)";
  let mut first =
    scripted(&at, &[work, "await wait(3600)\nclose('late')"]).boot::<String>([]).unwrap().0;
  let word =
    "waiting = prompt(str, 'Wait.', on=chain('work'))\nawait wait(0.5)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  let (mut inspected, kept) =
    Opening { inspecting: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap();
  let written = crate::checkpoint::written(&mut inspected, &kept).unwrap();
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
  let written =
    crate::checkpoint::written(&mut first, &world::kept(at.join("record.jsonl")).unwrap()).unwrap();
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
#[cfg(unix)]
fn a_record_of_an_older_version_whose_build_writes_no_checkpoint_is_refused_with_why() {
  let at = place("unupgraded");
  let path = at.join("record.jsonl");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  closed(&mut first, "close(1)");
  drop(first);
  let text = fs::read_to_string(&path).unwrap();
  let ours = format!("\"{}\",{}]", world::engine(), world::VERSION);
  fs::write(
    &path,
    text.replace(&ours, &format!("\"{}\",{}]", world::engine(), world::VERSION - 1)),
  )
  .unwrap();
  let program = at.join("old-furb");
  fs::write(&program, "#!/bin/sh\necho no such command >&2\nexit 2\n").unwrap();
  #[cfg(unix)]
  fs::set_permissions(&program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
  let refused =
    super::upgraded(&path, &at.display().to_string(), |_| Some(program.clone())).unwrap_err();
  assert_eq!(
    refused.to_string(),
    format!(
      "Refused: the build of version {} at {} wrote no checkpoint of {}: no such command",
      world::VERSION - 1,
      program.display(),
      path.display()
    )
  );
  assert!(
    super::upgraded(&path, "", |_| None).is_ok(),
    "a record whose build the store lacks stays as it is"
  );
}
