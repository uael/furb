//! The kits of a life that a host opens, whose process is a module of Python that speaks the
//! frames of a kit on its stdin and its stdout.

use std::{
  fs, future,
  path::{Path, PathBuf},
  process,
  time::{Duration, Instant},
};

use futures::stream;

#[cfg(unix)]
use crate::drive::pause;
use crate::{drive::block_on, opened::opening, yard::Yard};
use furb::{
  Ear, Engine, Fact, Object,
  ear::{Co, Next, ear, say},
  verbs,
};

/// A project of its own for one test, whose config turns on the kit `note` of its folder
/// `.furb/skills/note`, and the folder of the kit.
fn place(name: &str) -> (Yard, PathBuf) {
  let at = Yard::new(format!("furb-kits-{name}-{}", process::id()));
  let folder = at.join(".furb").join("skills").join("note");
  fs::create_dir_all(&folder).unwrap_or_else(|no| panic!("{no}"));
  let config = r#"{"skills": {"note": "skills/note"}}"#;
  fs::write(at.join(".furb").join("config.json"), config).unwrap_or_else(|no| panic!("{no}"));
  (at, folder)
}

/// The kit `note` in a folder, which answers the acts `note_add`, with a word, whose process is
/// the module of Python given, and the path of that module.
fn noted(folder: &Path, module: &str) -> PathBuf {
  let manifest = "---\nname: note\ndescription: Notes.\nmetadata:\n  furb-word: note.py\n  \
                  furb-process: kit.py\n  furb-acts: note_add\n---\n";
  let program = folder.join("kit.py");
  let written = fs::write(folder.join("SKILL.md"), manifest)
    .and_then(|()| fs::write(folder.join("note.py"), "note_kit = 1\n"))
    .and_then(|()| fs::write(&program, module));
  written.unwrap_or_else(|no| panic!("the kit was not written: {no}"));
  program
}

/// A kit that says its hello and its attach, marks its start in the file `started` beside it, and
/// closes each act `note_add` with the text `added`.
const ADDS: &str = r#"import json, os, subprocess, sys
child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(600)"])
started = os.path.join(os.path.dirname(os.path.abspath(__file__)), "started")
open(started, "a").write(f"{os.getpid()} {child.pid}\n")
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

/// A life that a host opens on a project, with the ears of the host given.
fn life(at: &Path, host: impl IntoIterator<Item = (&'static str, Box<dyn Ear>)>) -> Engine {
  opening(at, true).boot(host).unwrap_or_else(|no| panic!("the life did not open: {no}")).0
}

/// The fact enable of the kit `note` whose process is a program.
fn enable(engine: &mut Engine, program: &Path) {
  let words = ["note", "", "", "", "", "", &program.display().to_string()].map(Object::string);
  engine.say("enable", "chain1", words.into()).unwrap_or_else(|no| panic!("{no}"));
}

/// The pids that each process of the kit wrote in its file `started` as it started: its own and
/// the one of the process that it started.
fn started(folder: &Path) -> Vec<Vec<String>> {
  let read = fs::read_to_string(folder.join("started")).unwrap_or_default();
  read.lines().map(|line| line.split(' ').map(str::to_owned).collect()).collect()
}

/// Whether the process of a pid is gone, within a bound that a loaded machine does not reach.
#[cfg(unix)]
fn gone(pid: &str) -> bool {
  (0..200).any(|_| {
    let mut alive = process::Command::new("kill");
    let status = alive.args(["-0", pid]).stderr(process::Stdio::null()).status();
    let gone = !status.unwrap_or_else(|no| panic!("{no}")).success();
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
  let run = engine.rung(rung).unwrap_or_else(|no| panic!("{no}"));
  block_on(run).map_err(|no| no.to_string())
}

/// The word that awaits one act `note_add` and closes with its value.
const ADD: &str = "close(await act('note_add', '', None, 'milk'))";

#[test]
fn a_kit_starts_at_the_first_act_of_its_kinds_and_its_process_answers_each_one() {
  let (at, folder) = place("adds");
  noted(&folder, ADDS);
  let mut engine = life(&at, []);
  assert!(!folder.join("started").exists(), "a kit starts at its first act, not at its enable");
  for _ in 0..2 {
    let got = closed(&mut engine, ADD).unwrap_or_else(|no| panic!("{no}"));
    assert_eq!(got.as_ref().as_str(), Some("added"));
  }
  assert_eq!(started(&folder).len(), 1);
}

#[test]
fn the_kit_of_a_fact_enable_is_the_process_that_it_names_with_the_acts_of_the_manifest_above_it() {
  let (at, folder) = place("named");
  let program = noted(&folder, ADDS);
  fs::write(at.join(".furb").join("config.json"), "{}").unwrap_or_else(|no| panic!("{no}"));
  let mut engine = life(&at, []);
  let words = ["other", "", "", "", "", "", &program.display().to_string()].map(Object::string);
  engine.say("enable", "chain1", words.into()).unwrap_or_else(|no| panic!("{no}"));
  let got = closed(&mut engine, ADD).unwrap_or_else(|no| panic!("{no}"));
  assert_eq!(got.as_ref().as_str(), Some("added"));
}

#[test]
fn a_manifest_whose_process_leaves_its_folder_or_that_names_none_gives_no_kit() {
  for (name, process) in [("out", "  furb-process: ../kit.py\n"), ("bare", "")] {
    let (at, folder) = place(name);
    noted(&folder, ADDS);
    let manifest = format!(
      "---\nname: note\ndescription: Notes.\nmetadata:\n  furb-word: note.py\n{process}  \
       furb-acts: note_add\n---\n"
    );
    fs::write(folder.join("SKILL.md"), manifest).unwrap_or_else(|no| panic!("{no}"));
    fs::copy(folder.join("kit.py"), at.join(".furb").join("skills").join("kit.py"))
      .unwrap_or_else(|no| panic!("{no}"));
    let mut engine = life(&at, []);
    let no = closed(&mut engine, ADD).expect_err("no process answers the act");
    assert!(no.contains("note_add"), "{name}: {no}");
    let outside = at.join(".furb").join("skills");
    assert!(started(&folder).is_empty() && started(&outside).is_empty(), "{name}: no process ran");
  }
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
  let (at, folder) = place("reads");
  noted(&folder, READS);
  let manifest = fs::read_to_string(folder.join("SKILL.md")).unwrap_or_else(|no| panic!("{no}"));
  let keys = r#"{"properties": {"folder": {"type": "string", "default": "notes"}}}"#;
  let config = r#"{"skills": {"note": "skills/note"}, "note": {"folder": "journal"}}"#;
  let written = fs::write(
    folder.join("SKILL.md"),
    manifest.replace("furb-acts", "furb-config: keys.json\n  furb-acts"),
  )
  .and_then(|()| fs::write(folder.join("keys.json"), keys))
  .and_then(|()| fs::write(at.join(".furb").join("config.json"), config));
  written.unwrap_or_else(|no| panic!("{no}"));
  let mut engine = life(&at, []);
  let got = closed(&mut engine, ADD).unwrap_or_else(|no| panic!("{no}"));
  assert_eq!(got.as_ref().as_str(), Some("journal"));
}

#[cfg(unix)]
#[test]
fn a_new_enable_ends_the_process_of_its_kit_and_the_end_of_the_life_ends_each_process() {
  let (at, folder) = place("ends-all");
  let program = noted(&folder, ADDS);
  let mut engine = life(&at, []);
  closed(&mut engine, ADD).unwrap_or_else(|no| panic!("{no}"));
  enable(&mut engine, &program);
  closed(&mut engine, ADD).unwrap_or_else(|no| panic!("{no}"));
  let pids = started(&folder);
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
  let (at, folder) = place("ends");
  let program = noted(&folder, "import sys\nsys.stderr.write('boom\\n')\nsys.exit(3)\n");
  let mut engine = life(&at, []);
  let no = closed(&mut engine, ADD).expect_err("the act is refused");
  let why = format!("The process {} of the kit note ended with ", program.display());
  assert!(no.contains(&why), "{no}");
  assert!(no.contains("The last lines of its stderr:\nboom"), "{no}");
}

/// The acts that a run awaits, so a hop is a part of the difference of two runs: the acts of a
/// kit and the acts of an ear of this process, which each answer later through the same facts.
const HOPS: u32 = 10;

/// The least time of each of `runs` runs of a step.
fn least(runs: usize, mut step: impl FnMut()) -> Duration {
  let times = (0..runs).map(|_| {
    let at = Instant::now();
    step();
    at.elapsed()
  });
  times.min().unwrap_or_default()
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
  let (at, folder) = place("speed");
  noted(&folder, ADDS);
  let mut engine = life(&at, [("later", later())]);
  let mut ran = |kind: &str| {
    let word =
      format!("for _ in range({HOPS}):\n  await act('{kind}', '', None, 'milk')\nclose(None)");
    least(10, || drop(closed(&mut engine, &word).unwrap_or_else(|no| panic!("{no}"))))
  };
  ran("note_add");
  let (kit, here) = (ran("note_add"), ran("note_later"));
  let start = least(10, || {
    least_program().status().unwrap_or_else(|no| panic!("{no}"));
  });
  let hop = kit.saturating_sub(here) / HOPS;
  assert!(
    hop < start,
    "the hop took {hop:?} ({kit:?} less {here:?}, over {HOPS}), and the start of the least program {start:?}"
  );
}
