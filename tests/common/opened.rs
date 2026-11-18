//! A life that a host opens on a record in a yard, on a model whose replies a script gives, and the
//! records that it leaves. The suite of the crate and the tests of the bound of a turn beside the
//! life share it: each includes it as a module, and gives it `answered`, which the script calls at
//! each answer, and `Yard`.
#![expect(clippy::string_slice, reason = "a panic is how a test fails")]
#![allow(dead_code, reason = "each side takes the part of the harness that it needs")]

use std::{
  collections::{HashSet, VecDeque},
  env, fs,
  path::{Path, PathBuf},
  process,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering::Relaxed},
  },
};

use futures::executor;
use sha2::{Digest, Sha256};

use furb::life::{Answer, Opening};
use furb::{Engine, Object, skill::enabled, verbs, world};

use super::{Yard, answered};

/// A directory of its own for one test, and empty.
pub(crate) fn yard(name: &str) -> Yard {
  static MADE: AtomicUsize = AtomicUsize::new(0);
  let n = MADE.fetch_add(1, Relaxed);
  Yard::new(format!("furb-life-{name}-{}-{n}", process::id()))
}

/// A life in a directory of the tests, on its record and on the configs of that directory, whose
/// provider offers the operator alone.
pub(crate) fn opening(at: &Path, keeps: bool) -> Opening {
  let place = |name: &str| Some(at.join(name).display().to_string());
  Opening {
    // A life asks the claude command line of the machine for its models, which takes seconds: a
    // test names none.
    claude: Some(at.join("no-claude").display().to_string()),
    directory: at.display().to_string(),
    record: place("record.jsonl"),
    keeps: Some(keeps),
    config: place("config"),
    roster: Some(Vec::new().into()),
    ..Opening::default()
  }
}

/// The names of the extensions that a life runs.
pub(crate) fn names(engine: &mut Engine) -> Vec<String> {
  let root = engine
    .transcript(verbs::Transcript { on: Some("chain1".to_owned()), of: None, at: None })
    .unwrap();
  enabled(&root).into_iter().map(|one| one.name).collect()
}

/// What a word that a rung runs on the root closes with.
pub(crate) fn closed(engine: &mut Engine, word: &str) -> Object {
  let word = Some(word.to_owned());
  let rung =
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
  executor::block_on(rung).unwrap()
}

/// The fact made of a build of an engine, with no version, the schema of the record and the release
/// of this build, as a line of the record holds it.
pub(crate) fn made_of(engine: &str) -> String {
  format!(
    "[\"made\",\"chain1\",\"store\",\"{engine}\",null,\"{}\",\"{}\"]",
    world::RECORD_SCHEMA,
    env!("CARGO_PKG_VERSION")
  )
}

/// The fact made of an engine, as a record holds it, with the texts of that engine, of its Kernel
/// and of its checkpointer.
pub(crate) fn made_with(engine: &str, held: &[&str]) -> String {
  let texts = serde_json::to_string(held).unwrap();
  format!("{},{}]", made_of(engine).trim_end_matches(']'), &texts[1..texts.len() - 1])
}

/// The fact made of this build, as its record holds it.
pub(crate) fn ours() -> String {
  made_with(&world::engine(), &[furb::ENGINE, furb::KERNEL, furb::CHECKPOINTER])
}

/// The record of the life of a test at a directory, as if another build had made its last segment:
/// its last fact made of this build names this engine with one more line, so the next life that
/// keeps the record crosses there, and each later open carries the life across that place. Each
/// call names another engine, so the record crosses at each. The line is a comment, so the engine
/// that a model reads is the same: a fact that a life keeps after a crossing, as a turn that tells
/// a chain the lines of the engine that differ, holds the texts that the facts made name, and this
/// change of a fact made after the fact must not change it.
pub(crate) fn crossing(at: &Path) {
  static ENGINES: AtomicUsize = AtomicUsize::new(0);
  let record = at.join("record.jsonl");
  let text = fs::read_to_string(&record).unwrap();
  let (ours, n) = (ours(), ENGINES.fetch_add(1, Relaxed));
  let place = text.rfind(&ours).expect("the record names this build");
  let made = elsewhere(&format!("# crossed {n}")).0;
  fs::write(&record, format!("{}{made}{}", &text[..place], &text[place + ours.len()..])).unwrap();
}

/// The fact made of the engine of this build with one more line, as a line of the record holds it,
/// with its digest.
pub(crate) fn elsewhere(line: &str) -> (String, String) {
  let source = format!("{}\n{line}\n", furb::ENGINE);
  named(&[source.as_str(), furb::KERNEL, furb::CHECKPOINTER])
}

/// The fact made of an engine, a Kernel and a checkpointer, as a line of the record holds it, with
/// the digest that names them.
pub(crate) fn named(held: &[&str; 3]) -> (String, String) {
  let mut hash = Sha256::new();
  for one in held {
    hash.update(one.as_bytes());
    hash.update([0]);
  }
  let named = format!("{:x}", hash.finalize());
  (made_with(&named, held), named)
}

/// The records of the promise: those of each folder of `tests/corpus` of the major of this build,
/// which each later build of the major opens, as `tests/corpus/README.md` says. A record whose
/// bytes a folder before it holds already comes once, as `next` holds those of the last major
/// until the coming release changes them.
pub(crate) fn corpus() -> Vec<PathBuf> {
  let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("corpus");
  let folders =
    fs::read_dir(corpus).unwrap().map(|one| one.unwrap().path()).filter(|one| one.is_dir());
  let mut records: Vec<PathBuf> = folders
    .flat_map(|folder| fs::read_dir(folder).unwrap().map(|one| one.unwrap().path()))
    .filter(|one| one.extension().is_some_and(|named| named == "jsonl"))
    .filter(|one| world::unopened(one).is_none())
    .collect();
  records.sort();
  let mut held = HashSet::new();
  records.retain(|one| held.insert(fs::read(one).unwrap()));
  records
}

/// A life of a test whose model answers each request with the next word of a script, and with
/// close(None) once the script runs out. A test that cuts its first word alone sets its bound with
/// [`cut_first`].
pub(crate) fn scripted(at: &Path, words: &[&str]) -> Opening {
  let said: VecDeque<String> = words.iter().map(|one| (*one).to_owned()).collect();
  let said = Arc::new(Mutex::new((said, 0)));
  let answer = Answer(Arc::new(move |_, _| {
    let mut said = said.lock().unwrap();
    said.1 += 1;
    answered(said.1);
    let word = said.0.pop_front().unwrap_or_else(|| "close(None)".to_owned());
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string(word),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  Opening {
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    answer: Some(answer),
    ..opening(at, true)
  }
}

/// The words of the model of the life of `worked`: the root writes, reads, runs a command, waits,
/// reads the clock and chance, and asks a chain of its own, whose model answers in the second word.
pub(crate) const WORK: [&str; 2] = [
  "write(Text(\"notes.md\", \"one\\ntwo\\n\"))\nnotes = read(\"notes.md\")\nsaid = await bash(\"echo hi\")\nawait \
   wait(0.01, __name__)\nhelper = chain(\"helper\")\ngot = await prompt(int, \"Give 2.\", on=helper)\nclose(got + \
   len(notes.lines) + int(clock() > 0) + int(0 <= chance() < 1))",
  "close(2)",
];

/// The record of the life of `WORK` in a directory of its own: a first life works, and a second
/// life on the record takes one prompt more, so the record holds two openings. Each official
/// extension runs on it.
pub(crate) fn worked(name: &str) -> Yard {
  let at = yard(name);
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

/// The record of the life of `WORK`, which never crossed, replayed in a life that only
/// inspects it.
pub(crate) fn replayed(by: &str) -> Engine {
  let at = yard(&format!("shown-{by}"));
  fs::copy(worked(&format!("shown-{by}-made")), at.join("record.jsonl")).unwrap();
  Opening { inspecting: Some(true), ..opening(&at, false) }.boot::<String>([]).unwrap().0
}

/// The entries of the last segment of the record of a test: those from its last crossing on, or
/// each of them when it never crossed.
pub(crate) fn segment(at: &Path) -> Vec<Object> {
  let record = at.join("record.jsonl");
  let from = world::crossings(&record).unwrap().last().copied().unwrap_or(0);
  world::kept(&record).unwrap().split_off(from)
}

/// The word of the first life of a record whose module, chain and open prompt a later life holds.
pub(crate) const WAITED: &str = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose('late')";

/// A life on a record that holds a value, a definition, a chain and a prompt open on it, whose
/// record then crosses, and the name of that prompt.
pub(crate) fn crossed(at: &Path) -> String {
  let mut first = scripted(at, &[WAITED]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nSIZE = 6\ndef f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\n\
              await wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  crossing(at);
  waiting
}

/// The record at a place, with each line that holds a piece changed by a function.
pub(crate) fn changed(at: &Path, piece: &str, change: impl Fn(&str) -> String) {
  let path = at.join("record.jsonl");
  let text = fs::read_to_string(&path).unwrap();
  assert!(text.contains(piece), "the record holds {piece}");
  let lines: Vec<String> = text
    .lines()
    .map(|line| if line.contains(piece) { change(line) } else { line.to_owned() })
    .collect();
  fs::write(&path, lines.join("\n") + "\n").unwrap();
}

/// A life that a fault stopped inside a run that a bound cut keeps the end of that run with no
/// place, since the keep of the run itself waited behind the first step of the Kernel. A life that
/// inspects the record and one that keeps it reach the exact state of its tip, as the life held it
/// at the stop: each fact of the record, each turn and each name, with the end of the cut run.
/// A prompt of the operator on the root.
pub(crate) fn prompted(message: &str) -> verbs::Prompt {
  verbs::Prompt {
    message: Some(message.to_owned()),
    on: Some("chain1".to_owned()),
    ..Default::default()
  }
}
