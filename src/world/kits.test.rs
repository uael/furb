//! The law of speed of the ear of the kits, in a life of the real engine whose ear of this process
//! stands beside the kits, so the two acts that it compares pass the same ears. No door places an
//! ear of a host there: a life that a host opens puts the ears of the host first. The rest of the
//! kits stands in the suite of the crate, through a life that a host opens.

use std::{
  env, fs, future,
  path::{Path, PathBuf},
  process,
  time::{Duration, Instant},
};

use futures::stream;

use super::kits;
use crate::{
  Ear, Engine, Fact, Object,
  ear::{Co, Next, ear, hear, say},
  skill::Places,
  testing::{Yard, block_on},
  verbs,
};

/// A new folder of a test.
fn place(name: &str) -> Yard {
  Yard::new(format!("furb-kits-{name}-{}", process::id()))
}

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

/// What a word that a rung runs on the root closes with, or why it raised.
fn closed(engine: &mut Engine, word: &str) -> Result<Object, String> {
  let root = engine.root().to_owned();
  let rung = verbs::Rung { word: Some(word.to_owned()), on: Some(root), ..Default::default() };
  block_on(engine.rung(rung).unwrap()).map_err(|no| no.to_string())
}

/// The runs of one awaited act of each kind: the least of each is the run that no other work of
/// the machine cut, so a hop is the difference of the least run of an act of a kit and the least
/// run of an act of an ear of this process, which each answer later through the same facts. A run
/// of ten acts was cut far more often than a run of one, and the gates on a loaded runner of Linux
/// read a hop of 671 µs where the hop costs about 40 µs.
const RUNS: usize = 50;

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
  let kits = kits(String::new(), at.to_path_buf(), None, places);
  let ears = [("stand", standing()), ("later", later()), ("kits", kits)];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  let words = ["note", "", "", "", "", "", &program.display().to_string()].map(Object::string);
  engine.say("enable", "chain1", words.into()).unwrap();
  let mut ran = |kind: &str| {
    let word = format!("await act('{kind}', '', None, 'milk')\nclose(None)");
    least(RUNS, || drop(closed(&mut engine, &word).unwrap()))
  };
  ran("note_add");
  let (kit, here) = (ran("note_add"), ran("note_later"));
  let start = least(RUNS, || {
    least_program().status().unwrap();
  });
  let hop = kit.saturating_sub(here);
  assert!(
    hop < start,
    "the hop took {hop:?} ({kit:?} less {here:?}), and the start of the least program {start:?}"
  );
}
