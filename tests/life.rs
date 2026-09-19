//! One life of the real engine, driven by the crate from end to end.
//!
//! Every other test of the crate holds one piece against a scripted double. This holds the whole of it against
//! the engine itself: a life is opened, an operator calls verbs, a model answers a prompt, the Kernel gates the
//! word it wrote and runs it, the World of this machine does what the word asks of the disk and of the shell,
//! and the record it kept opens a second life.
//!
//! `tests/sandbox.py` is the sandbox: one python namespace behind a pipe, which is what [`furb::Session`] says a
//! sandbox must be. It sandboxes nothing, and it is no part of the crate. What it is for is that the crate can
//! be held to the real engine today, and that a sandbox of monty has these tests waiting for it.

use std::{
  io::{BufRead, BufReader, Write},
  path::PathBuf,
  process::{Child, ChildStdin, Command, Stdio},
  thread,
  time::{Duration, Instant},
};

use furb::{
  Ears, Fact, Life, Live, Talks, Value, Voice,
  host::Gate,
  life::{Host, Session},
  verb::{Act, Bash, Cwd, Peek, Prompt, Read, Turns, Verb, Write as Writes},
};

/// The root of the repository, which holds the sandbox and the record of a life.
fn root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// One sandbox of the tests: a python namespace behind a pipe.
struct Far {
  /// The process the namespace lives in.
  child: Child,
  /// What it says, a line to a message.
  says: BufReader<std::process::ChildStdout>,
  /// What it hears.
  hears: ChildStdin,
}

impl Far {
  /// One sandbox, up, with the fixture running in it.
  fn open() -> Self {
    let python = root().join(".venv").join("bin").join("python3");
    let python = if python.is_file() { python } else { PathBuf::from("python3") };
    let mut child = Command::new(python)
      .arg(root().join("tests").join("sandbox.py"))
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .spawn()
      .expect("a python of this machine");
    let says = BufReader::new(child.stdout.take().expect("the stdout of the sandbox"));
    let hears = child.stdin.take().expect("the stdin of the sandbox");
    Far { child, says, hears }
  }

  /// One message to the sandbox.
  fn tells(&mut self, said: &serde_json::Value) {
    writeln!(self.hears, "{said}").expect("the sandbox hears");
    self.hears.flush().expect("the sandbox hears");
  }

  /// One message of the sandbox.
  fn heard(&mut self) -> serde_json::Value {
    let mut line = String::new();
    self.says.read_line(&mut line).expect("the sandbox says");
    serde_json::from_str(&line).unwrap_or_else(|_| panic!("the sandbox said {line:?}"))
  }
}

impl Drop for Far {
  fn drop(&mut self) {
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

impl Session for Far {
  fn run(&mut self, code: &str, host: &mut dyn Host) -> Result<Value, Value> {
    self.tells(&serde_json::json!({ "run": code }));
    loop {
      let said = self.heard();
      if let Some(held) = said.get("call").and_then(serde_json::Value::as_array) {
        let name = held[0].as_str().unwrap_or_default().to_owned();
        let got = host.called(&name, &Value::of_record(&held[1]));
        self.tells(&serde_json::json!({ "said": got.record() }));
        continue;
      }
      if let Some(held) = said.get("raised") {
        return Err(Value::of_record(held));
      }
      return Ok(Value::of_record(said.get("gave").unwrap_or(&serde_json::Value::Null)));
    }
  }
}

/// A gate of the tests, which refuses a word that holds BAD, as the harness of the suite does.
struct Strict;

impl Gate for Strict {
  fn gate(&mut self, word: &str, _ladder: &[String], _shape: &str) -> Vec<String> {
    if word.contains("BAD") { vec!["BAD in rung".to_owned()] } else { Vec::new() }
  }
}

/// A model of the tests, which answers every ask with the next word it was given.
struct Says {
  /// The words it answers with, in order.
  words: Vec<String>,
  /// What every ask it heard stood on, so a test reads what the model was shown.
  read: Vec<String>,
}

impl Talks for Says {
  fn asked(&mut self, rung: &str, _on: &str, _actor: &str, turns: &[furb::Turn], voice: &Voice) {
    self.read.push(turns.last().map(furb::Turn::rendered).unwrap_or_default());
    let word = if self.words.is_empty() { "close(None)".to_owned() } else { self.words.remove(0) };
    let turn = Value::Tuple(vec![
      Value::Str("assistant".to_owned()),
      Value::List(vec![Value::Str(word)]),
      Value::Tuple(vec![Value::Int(0), Value::Int(0), Value::Int(0), Value::Int(0), Value::Float(0.0)]),
      Value::List(vec![]),
    ]);
    voice.send(Fact::new("answer", rung, "world", vec![turn]));
  }

  fn shown(&mut self, about: &str, _shape: &str, _message: &str, voice: &Voice) {
    voice.close(about, Value::Str("the operator answered".to_owned()));
  }
}

/// One life of the real engine, on a yard of the disk, with the model answering these words.
fn life(yard: &str, words: &[&str]) -> (Life<Far, Live<Says>, Strict>, PathBuf) {
  let at = std::env::temp_dir().join(format!("furb-life-{yard}"));
  let _ = std::fs::remove_dir_all(&at);
  std::fs::create_dir_all(&at).expect("a yard of the test");
  let (voice, ears) = Ears::made();
  let roster = Value::Tuple(vec![
    Value::Tuple(vec![Value::Str("operator".to_owned()), Value::Tuple(vec![]), Value::Int(200_000)]),
    Value::Tuple(vec![
      Value::Str("m".to_owned()),
      Value::Tuple(vec![Value::Str("low".to_owned())]),
      Value::Int(200_000),
    ]),
  ]);
  let talks = Says { words: words.iter().map(|one| (*one).to_owned()).collect(), read: Vec::new() };
  let world = Live::new(&at, "m/low", roster, talks, voice).keeping(at.join("record.jsonl"));
  let held = Life::boot(Far::open(), world, Strict, ears, &[]).expect("a life of the real engine");
  (held, at)
}

/// What an act came to, once the host has had its moment to answer for a model, a person and a command.
fn came(held: &mut Life<Far, Live<Says>, Strict>, act: &Act) -> Value {
  let waited = Instant::now();
  while waited.elapsed() < Duration::from_secs(30) {
    if let Some(got) = held.came(act).expect("the life answers what an act came to") {
      return got;
    }
    thread::sleep(Duration::from_millis(10));
  }
  panic!("{act} never came to anything")
}

#[test]
fn a_life_of_the_real_engine_opens_on_its_root_and_answers_what_the_root_stands_on() {
  let (mut held, at) = life("open", &[]);
  assert_eq!(held.root(), "chain://operator.1");
  let root = held.root().to_owned();
  assert_eq!(held.calls(Cwd { on: &root }).unwrap(), at.display().to_string());
  assert_eq!(held.calls(Turns { on: &root }).unwrap().len(), 1);
}

#[test]
fn the_world_of_this_machine_serves_a_read_and_a_write_of_the_real_engine() {
  let (mut held, at) = life("disk", &[]);
  let root = held.root().to_owned();
  let text = furb::Text::new("a.txt", "one\ntwo\n");
  let got = held.calls(Writes { text, on: &root }).unwrap();
  assert_eq!(got.content, "one\ntwo\n");
  assert_eq!(std::fs::read_to_string(at.join("a.txt")).unwrap(), "one\ntwo\n");
  let got = held.calls(Read { path: "a.txt", on: &root, ..Read::default() }).unwrap();
  assert_eq!(got.lines(), ["one", "two"]);
  let no = held.calls(Read { path: "none.txt", on: &root, ..Read::default() }).unwrap_err();
  assert!(no.refused(), "{no}");
}

#[test]
fn a_model_answers_a_prompt_of_the_real_engine_and_the_kernel_runs_the_word_it_wrote() {
  let (mut held, _) = life("prompt", &["close(len(read('a.txt').lines))"]);
  let root = held.root().to_owned();
  let text = furb::Text::new("a.txt", "one\ntwo\nthree\n");
  held.calls(Writes { text, on: &root }).unwrap();
  let act = held.calls(Prompt { shape: "int", message: "count the lines", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(3));
  assert!(held.world().talks.read[0].contains("count the lines"), "{:?}", held.world().talks.read[0]);
}

#[test]
fn the_gate_of_the_host_refuses_a_word_and_the_engine_asks_the_model_again() {
  let (mut held, _) = life("gate", &["close(BAD)", "close(7)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "count", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(7));
  assert!(held.world().talks.read[1].contains("BAD in rung"), "{:?}", held.world().talks.read[1]);
}

#[test]
fn a_command_of_the_real_engine_runs_in_the_world_of_this_machine_and_says_what_it_came_to() {
  let (mut held, _) = life("bash", &[]);
  let root = held.root().to_owned();
  let act = held.calls(Bash { command: "echo hi; exit 3", on: &root, ..Bash::default() }).unwrap();
  let got = came(&mut held, &act);
  assert_eq!(got.field("code"), Some(&Value::Int(3)));
  let exit = furb::Exit::of(&got).expect("an exit of the command");
  assert_eq!(exit.stdout.content, "hi\n");
}

#[test]
fn the_operator_answers_a_prompt_of_the_operator_by_closing_it() {
  let (mut held, _) = life("operator", &[]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "str", message: "say a word", to: "operator", on: &root }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Str("the operator answered".to_owned()));
}

#[test]
fn a_second_life_on_the_record_the_world_kept_makes_the_same_acts_again() {
  let (mut held, at) = life("again", &["close(2)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "one plus one", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(2));
  drop(held);

  let kept = furb::record::read(&std::fs::read_to_string(at.join("record.jsonl")).unwrap()).expect("the record");
  assert!(kept.len() > 1, "the World kept {} entries", kept.len());
  let (voice, ears) = Ears::made();
  let talks = Says { words: Vec::new(), read: Vec::new() };
  let world = Live::new(&at, "m/low", Value::Tuple(vec![]), talks, voice);
  let mut again = Life::boot(Far::open(), world, Strict, ears, &kept).expect("a second life on the record");
  assert_eq!(again.root(), root);
  assert_eq!(again.calls(Peek { at: &act, on: &root }).unwrap(), Value::Int(2));
  assert!(again.world().talks.read.is_empty(), "a later life asks no model again");
}

#[test]
fn what_a_verb_gave_is_read_as_the_shape_the_contract_gives() {
  let (mut held, _) = life("shapes", &[]);
  let root = held.root().to_owned();
  assert!(held.calls(furb::verb::Clock { on: &root }).unwrap() > 1.0);
  let drew = held.calls(furb::verb::Chance { on: &root }).unwrap();
  assert!((0.0..1.0).contains(&drew), "{drew}");
  assert_eq!(held.calls(furb::verb::Gate { word: "close(BAD)", returns: "int", on: &root }).unwrap(), ["BAD in rung"]);
  assert_eq!(held.calls(furb::verb::Cd { path: "w", on: &root }).unwrap(), "w");
  assert_eq!(held.calls(Cwd { on: &root }).unwrap(), "w");
}

#[test]
fn what_the_engine_raised_reaches_the_host_as_the_exception_it_is() {
  let (mut held, _) = life("raised", &[]);
  let no = held.word("get('bash://operator.9')").unwrap_err();
  assert_eq!(no.name(), "KeyError");
  assert!(!no.refused(), "{no}");
  let got = <Prompt as Verb>::gave(&Value::Int(3)).unwrap_err();
  assert_eq!(got.name(), "");
}

#[test]
fn a_word_of_a_model_awaits_an_act_and_the_kernel_carries_the_run_forward() {
  let (mut held, _) = life("await", &["close((await bash('echo hi')).code)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "run it", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(0));
}

#[test]
fn a_word_that_raises_is_asked_again_and_the_model_reads_what_it_raised() {
  let (mut held, _) = life("raises", &["close(1 // 0)", "close(5)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "count", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(5));
  assert!(held.world().talks.read[1].contains("ZeroDivisionError"), "{:?}", held.world().talks.read[1]);
}

#[test]
fn what_a_word_tells_stands_in_the_turns_the_model_reads_next() {
  // A tell outside a run tells nothing, which the contract says, so the word of a rung says this one.
  let (mut held, _) = life("tell", &["tell('noted', ('by', 'the word'), body='go on')", "close(1)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "note it", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(1));
  assert!(
    held.world().talks.read[1].contains("<noted by=\"the word\">\ngo on\n</noted>"),
    "{:?}",
    held.world().talks.read[1]
  );
}

#[test]
fn a_cancel_of_the_operator_ends_an_act_and_what_it_came_to_says_so() {
  let (mut held, _) = life("cancel", &[]);
  let root = held.root().to_owned();
  let act = held.calls(furb::verb::Wait { seconds: 30.0, on: &root }).unwrap();
  assert_eq!(held.came(&act).unwrap(), None);
  held.calls(furb::verb::Cancel { id: &act }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Error { name: "CancelledError".to_owned(), args: vec![] });
}

#[test]
fn a_chain_of_the_operator_has_a_transcript_of_its_own() {
  let (mut held, _) = life("chain", &[]);
  let root = held.root().to_owned();
  let made = held.calls(furb::verb::Chain { label: "work", on: &root, ..furb::verb::Chain::default() }).unwrap();
  assert_ne!(made.0, root);
  assert_eq!(held.calls(Turns { on: &made }).unwrap().len(), 1);
  let said: Vec<String> =
    furb::Turn::every(&held.calls(Turns { on: &made }).unwrap()).iter().map(furb::Turn::rendered).collect();
  assert!(said[0].contains("label=\"work\""), "{said:?}");
}

#[test]
fn the_stdin_of_a_fed_command_takes_what_a_write_of_its_door_says() {
  let (mut held, _) = life("fed", &[]);
  let root = held.root().to_owned();
  let act = held.calls(Bash { command: "cat", fed: true, on: &root, ..Bash::default() }).unwrap();
  let door = format!("bash://{}/stdin", act.rsplit_once("://").map_or("", |(_, one)| one));
  held.calls(Writes { text: furb::Text::new(&door, "one\n"), on: &root }).unwrap();
  held.calls(Writes { text: furb::Text::new(&door, ""), on: &root }).unwrap();
  let got = came(&mut held, &act);
  assert_eq!(furb::Exit::of(&got).expect("an exit").stdout.content, "one\n");
}

#[test]
fn a_show_says_which_lines_of_a_text_a_read_tells() {
  // A read outside a run tells nothing, which the contract says, so the word of a rung says this one.
  let (mut held, _) = life("show", &["close(len(read('a.txt', span(2, 3)).lines))"]);
  let root = held.root().to_owned();
  let text = furb::Text::new("a.txt", "one\ntwo\nthree\nfour\n");
  held.calls(Writes { text, on: &root }).unwrap();
  let act = held.calls(Prompt { shape: "int", message: "read it", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(4));
  let said: Vec<String> =
    furb::Turn::every(&held.calls(Turns { on: &root }).unwrap()).iter().map(furb::Turn::rendered).collect();
  let whole = said.join("\n");
  // A read tells the lines the model has not seen, each by its number, so the show is read off those.
  assert!(whole.contains("2 two\n3 three"), "{whole}");
  assert!(!whole.contains("four"), "{whole}");
}
