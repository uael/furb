//! The ear of the kits, in a life of the real engine, with kits whose process is a script of the
//! shell that speaks the frames of a kit on its stdin and its stdout.

use std::{
  env, fs, future,
  path::{Path, PathBuf},
  process,
  time::{Duration, Instant},
};

use futures::stream;

use super::kits;
#[cfg(unix)]
use crate::testing::pause;
use crate::{
  Ear, Engine, Fact, Object,
  ear::{Co, Next, ear, hear, say},
  skill::Places,
  testing::block_on,
  verbs,
};

/// The ear that answers a stand with the operator alone, in the directory of the tests.
fn standing() -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() == "stand" && a.question() {
        let operator = Object::list([Object::string("operator"), Object::list([]), Object::int(1)]);
        let here = Object::string(env::temp_dir().display().to_string());
        let standing = Object::list([Object::list([operator]), here, Object::string("operator")]);
        say(&mut co, Fact::says("done", a.about(), [standing]));
      }
    }
  })
}

/// A new folder of a test.
fn place(name: &str) -> PathBuf {
  let at = env::temp_dir().join(format!("furb-kits-{name}-{}", process::id()));
  drop(fs::remove_dir_all(&at));
  fs::create_dir_all(&at).unwrap();
  at
}

/// The kit `note` in a folder, which answers the acts `note_add`, whose process is the module of
/// Python given, and the path of that module.
fn noted(at: &Path, module: &str) -> PathBuf {
  let manifest = "---\nname: note\ndescription: Notes.\nmetadata:\n  furb-process: kit.py\n  \
                  furb-acts: note_add\n---\n";
  fs::write(at.join("SKILL.md"), manifest).unwrap();
  let program = at.join("kit.py");
  fs::write(&program, module).unwrap();
  program
}

/// A kit that says its hello and its attach, marks its start in the file `started` of its folder,
/// and closes each act `note_add` with the text `added`.
const ADDS: &str = r#"import json, os, subprocess, sys
child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(600)"])
open("started", "a").write(f"{os.getpid()} {child.pid}\n")
def say(verb):
  sys.stdout.write(json.dumps(verb) + "\n")
  sys.stdout.flush()
say({"id": 0, "verb": "hello", "protocol": 44, "kind": "skill", "name": "note"})
say({"id": 1, "verb": "attach", "life": "", "from": 0})
n = 2
for line in sys.stdin:
  frame = json.loads(line)
  fact = frame.get("event", {}).get("data") or [""]
  if fact[0] == "note_add":
    say({"id": n, "verb": "close", "life": "", "act": fact[1], "value": json.dumps("added")})
    n += 1
"#;

/// A life on the ear of the kits, with the kit `note` of a folder enabled.
fn life(at: &Path, program: &Path) -> Engine {
  let places = Places { config: at.join("user"), home: None };
  let ears = [("stand", standing()), ("kits", kits(String::new(), at.to_owned(), None, places))];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  enable(&mut engine, program);
  engine
}

/// The fact enable of the kit `note` whose process is a program.
fn enable(engine: &mut Engine, program: &Path) {
  let words = ["note", "", "", "", "", "", &program.display().to_string()].map(Object::string);
  engine.say("enable", "chain1", words.into()).unwrap();
}

/// The pids that each process of the kit wrote in its file `started` as it started: its own and
/// the one of the process that it started.
fn started(at: &Path) -> Vec<Vec<String>> {
  let read = fs::read_to_string(at.join("started")).unwrap_or_default();
  read.lines().map(|line| line.split(' ').map(str::to_owned).collect()).collect()
}

/// Whether the process of a pid is gone, within a bound that a loaded machine does not reach.
#[cfg(unix)]
fn gone(pid: &str) -> bool {
  (0..200).any(|_| {
    let mut alive = process::Command::new("kill");
    let gone = !alive.args(["-0", pid]).stderr(process::Stdio::null()).status().unwrap().success();
    if !gone {
      pause(Duration::from_millis(25));
    }
    gone
  })
}

/// What a word that a rung runs on the root closes with, or why it raised.
fn closed(engine: &mut Engine, word: &str) -> Result<Object, String> {
  let root = engine.root().to_owned();
  let rung = verbs::Rung { word: Some(word.to_owned()), on: Some(root), ..Default::default() };
  block_on(engine.rung(rung).unwrap()).map_err(|no| no.to_string())
}

#[test]
fn a_kit_starts_at_the_first_act_of_its_kinds_and_its_process_answers_each_one() {
  let at = place("adds");
  let program = noted(&at, ADDS);
  let mut engine = life(&at, &program);
  assert!(!at.join("started").exists(), "a kit starts at its first act, not at its enable");
  for _ in 0..2 {
    let got = closed(&mut engine, "close(await act('note_add', '', None, 'milk'))").unwrap();
    assert_eq!(got.as_ref().as_str(), Some("added"));
  }
  assert_eq!(started(&at).len(), 1);
}

/// A kit whose process closes each act `note_add` with the value of its key `folder`, which it
/// reads by the verb config.
const READS: &str = r#"import json, sys
def say(verb):
  sys.stdout.write(json.dumps(verb) + "\n")
  sys.stdout.flush()
say({"id": 0, "verb": "hello", "protocol": 44, "kind": "skill", "name": "note"})
say({"id": 1, "verb": "attach", "life": "", "from": 0})
act = None
for line in sys.stdin:
  frame = json.loads(line)
  fact = frame.get("event", {}).get("data") or [""]
  if fact[0] == "note_add":
    act = fact[1]
    say({"id": 2, "verb": "config", "life": "", "config": {"action": "get"}})
  if frame.get("id") == 2 and frame.get("frame") == "reply":
    keys = {tuple(one["key"]): one["value"] for one in frame["answer"]["data"]}
    say({"id": 3, "verb": "close", "life": "", "act": act, "value": json.dumps(keys["note", "folder"])})
"#;

#[test]
fn a_kit_reads_the_value_of_each_of_its_keys_by_the_verb_config() {
  let at = place("reads");
  let program = noted(&at, READS);
  let manifest = fs::read_to_string(at.join("SKILL.md")).unwrap();
  fs::write(
    at.join("SKILL.md"),
    manifest.replace("furb-acts", "furb-config: keys.json\n  furb-acts"),
  )
  .unwrap();
  let keys = r#"{"properties": {"folder": {"type": "string", "default": "notes"}}}"#;
  fs::write(at.join("keys.json"), keys).unwrap();
  fs::create_dir_all(at.join(".furb")).unwrap();
  let config = r#"{"skills": {"note": "kit.py"}, "note": {"folder": "journal"}}"#;
  fs::write(at.join(".furb/config.json"), config).unwrap();
  let mut engine = life(&at, &program);
  let got = closed(&mut engine, "close(await act('note_add', '', None, 'milk'))").unwrap();
  assert_eq!(got.as_ref().as_str(), Some("journal"));
}

#[cfg(unix)]
#[test]
fn a_new_enable_ends_the_process_of_its_kit_and_the_end_of_the_life_ends_each_process() {
  let at = place("ends-all");
  let program = noted(&at, ADDS);
  let mut engine = life(&at, &program);
  closed(&mut engine, "close(await act('note_add', '', None, 'milk'))").unwrap();
  enable(&mut engine, &program);
  closed(&mut engine, "close(await act('note_add', '', None, 'milk'))").unwrap();
  let pids = started(&at);
  assert_eq!(pids.len(), 2);
  assert!(pids[0].iter().all(|pid| gone(pid)), "a new enable ends the process and its children");
  drop(engine);
  assert!(
    pids[1].iter().all(|pid| gone(pid)),
    "the end of the life ends the process and its children"
  );
}

#[test]
fn a_process_that_ends_before_its_life_refuses_the_act_that_it_held_with_why() {
  let at = place("ends");
  let program = noted(&at, "import sys\nsys.stderr.write('boom\\n')\nsys.exit(3)\n");
  let mut engine = life(&at, &program);
  let no = closed(&mut engine, "close(await act('note_add', '', None, 'milk'))").unwrap_err();
  let why = format!("The process {} of the kit note ended with ", program.display());
  assert!(no.contains(&why), "{no}");
  assert!(no.contains("The last lines of its stderr:\nboom"), "{no}");
}

/// The acts that a run awaits, so a hop is a part of the difference of two runs: the acts of a
/// kit and the acts of an ear of this process, which each answer later through the same facts.
const HOPS: u32 = 10;

/// The least time of each of `runs` runs of a step.
fn least(runs: usize, mut step: impl FnMut()) -> Duration {
  (0..runs)
    .map(|_| {
      let at = Instant::now();
      step();
      at.elapsed()
    })
    .min()
    .unwrap()
}

/// The least program of the system: `true`, or an exit of `cmd` on Windows.
fn least_program() -> process::Command {
  #[cfg(unix)]
  let program = process::Command::new("true");
  #[cfg(windows)]
  let program = {
    let mut program = process::Command::new("cmd");
    program.args(["/c", "exit"]);
    program
  };
  program
}

/// The ear that answers each act `note_later` in this process as a kit answers its acts: it says
/// that it started, and its work says the done.
fn later() -> Box<dyn Ear> {
  ear(move |mut co: Co<String>| async move {
    loop {
      match co.next().await {
        Next::Heard(a) if a.kind() == "note_later" && a.question() => {
          say(&mut co, Fact::says("started", a.about(), []));
          co.work(stream::once(future::ready(a.about().to_owned())));
        }
        Next::Worked(id) => say(&mut co, Fact::says("done", &id, [Object::string("added")])),
        _ => {}
      }
    }
  })
}

#[test]
fn the_hop_to_the_process_of_a_kit_costs_less_than_the_start_of_the_least_program() {
  let at = place("speed");
  let program = noted(&at, ADDS);
  let places = Places { config: at.join("user"), home: None };
  let kits = kits(String::new(), at, None, places);
  let ears = [("stand", standing()), ("later", later()), ("kits", kits)];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  let words = ["note", "", "", "", "", "", &program.display().to_string()].map(Object::string);
  engine.say("enable", "chain1", words.into()).unwrap();
  let mut ran = |kind: &str| {
    let word =
      format!("for _ in range({HOPS}):\n  await act('{kind}', '', None, 'milk')\nclose(None)");
    least(10, || drop(closed(&mut engine, &word).unwrap()))
  };
  ran("note_add");
  let (kit, here) = (ran("note_add"), ran("note_later"));
  let start = least(10, || {
    least_program().status().unwrap();
  });
  let hop = kit.saturating_sub(here) / HOPS;
  assert!(
    hop < start,
    "the hop took {hop:?} ({kit:?} less {here:?}, over {HOPS}), and the start of the least program {start:?}"
  );
}
