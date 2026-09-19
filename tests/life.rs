//! One life of the real engine, driven by the crate from end to end.
//!
//! Every other test of the crate holds one piece against a scripted double. This holds the whole of it against
//! the engine itself: a life is opened, an operator calls verbs, a model answers a prompt, the Kernel gates the
//! word it wrote and runs it, a World does what the word asks of the disk and of the shell, and the record it
//! kept opens a second life.
//!
//! Two doubles carry it, and neither is part of the crate. `tests/sandbox.py` is a sandbox: one python namespace
//! behind a pipe, which is what [`furb::Session`] says a sandbox must be, and which sandboxes nothing. [`Yard`]
//! is a World of this machine, small enough to read: what a life may touch is the host's to decide, so the crate
//! ships the trait and every host writes one of these.
//!
//! What they are for is that the crate is held to the real engine today, and that a sandbox of monty has these
//! tests waiting for it: they run against any `Session`, so the same ones point at monty with no change.

use std::{
  collections::HashMap,
  fs,
  io::{BufRead, BufReader, Read, Write},
  path::PathBuf,
  process::{Child, ChildStdin, Command, Stdio},
  sync::{Arc, Mutex},
  thread,
  time::{Duration, Instant},
};

use furb::{
  Ears, Entry, Fact, Life, Reply, Value, Voice, World,
  host::{Gate, WORLD},
  life::{Host, Session},
  verb::{Act, Bash, Cwd, Peek, Prompt, Read as Reads, Turns, Verb, Write as Writes},
};

/// The root of the repository, which holds the sandbox the tests open.
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
        // What the sandbox hands over is plain, and the mark of it is the boundary's to read, not this one's.
        let got = host.called(&name, &Value::of_data(&held[1]));
        self.tells(&serde_json::json!({ "said": got.record() }));
        continue;
      }
      if let Some(held) = said.get("raised") {
        return Err(Value::of_data(held));
      }
      return Ok(Value::of_data(said.get("gave").unwrap_or(&serde_json::Value::Null)));
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

/// What the World is waiting for the engine to answer, and the fact it answers once it holds it.
enum Asking {
  /// A read or a write, waiting for where the paths of its chain resolve.
  Path(Fact),
  /// A command, waiting for whether its stderr flows into its stdout.
  Merged(Fact),
  /// A command that knows its streams, waiting for where it runs.
  Where(Fact),
}

/// A World of this machine, for the tests: a directory, a shell, and a model that answers by a script.
///
/// It is no part of the crate. What a life may touch is the host's to decide, so the crate ships the trait and
/// this is one host, written small so that a reader of the tests can hold the whole of it.
struct Yard {
  /// Where the chains of the life stand.
  at: PathBuf,
  /// The actors it offers.
  roster: Value,
  /// The file it keeps the record in, and nothing when it keeps none.
  record: Option<PathBuf>,
  /// How it says what it did not answer where it was asked.
  voice: Voice,
  /// The words its model answers with, in order.
  words: Vec<String>,
  /// What every ask it heard showed the model, so a test reads what the model was told.
  read: Vec<String>,
  /// The acts it starts, kept from when it hears them until it is told to start them.
  acts: HashMap<String, Fact>,
  /// The commands that stand, by the act that made each one.
  running: HashMap<String, Arc<Mutex<Child>>>,
  /// What it is waiting for the engine to answer.
  asking: Option<Asking>,
}

impl Yard {
  /// A World on a yard of the disk, with the model answering these words.
  fn new(at: PathBuf, voice: Voice, words: &[&str]) -> Self {
    let roster = Value::Tuple(vec![
      Value::Tuple(vec![Value::Str("operator".to_owned()), Value::Tuple(vec![]), Value::Int(200_000)]),
      Value::Tuple(vec![
        Value::Str("m".to_owned()),
        Value::Tuple(vec![Value::Str("low".to_owned())]),
        Value::Int(200_000),
      ]),
    ]);
    Yard {
      record: Some(at.join("record.jsonl")),
      at,
      roster,
      voice,
      words: words.iter().map(|one| (*one).to_owned()).collect(),
      read: Vec::new(),
      acts: HashMap::new(),
      running: HashMap::new(),
      asking: None,
    }
  }

  /// The act a start names, done, once the chain has said where its paths resolve.
  fn started(&mut self, act: &Fact, here: &str) {
    let words = act.words();
    let about = act.about().to_owned();
    match act.kind() {
      "bash" => {
        let command = words.get(1).and_then(Value::as_str).unwrap_or_default().to_owned();
        let fed = words.get(2) == Some(&Value::Bool(true));
        self.runs(&about, here, &command, fed);
      }
      "wait" => {
        let (voice, held) = (self.voice.clone(), seconds(words.get(1)));
        thread::spawn(move || {
          thread::sleep(held);
          voice.send(Fact::new("done", &about, WORLD, vec![Value::None]));
        });
      }
      _ => self.voice.close(&about, Value::Str("the operator answered".to_owned())),
    }
  }

  /// One command, up, which says each of its streams as it reads it and its code when it ends.
  fn runs(&mut self, about: &str, here: &str, command: &str, fed: bool) {
    let mut child = Command::new("sh")
      .arg("-c")
      .arg(command)
      .current_dir(self.at.join(here))
      .stdin(if fed { Stdio::piped() } else { Stdio::null() })
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .expect("a shell of this machine");
    let mut streams = Vec::new();
    let doors: Vec<(&str, Box<dyn Read + Send>)> = [
      child.stdout.take().map(|one| ("stdout", Box::new(one) as Box<dyn Read + Send>)),
      child.stderr.take().map(|one| ("stderr", Box::new(one) as Box<dyn Read + Send>)),
    ]
    .into_iter()
    .flatten()
    .collect();
    for (stream, mut door) in doors {
      let (voice, about, stream) = (self.voice.clone(), about.to_owned(), stream.to_owned());
      streams.push(thread::spawn(move || {
        let mut said = String::new();
        let _ = door.read_to_string(&mut said);
        if !said.is_empty() {
          voice.send(Fact::new("out", &about, WORLD, vec![Value::Str(said), Value::Str(stream)]));
        }
      }));
    }
    let child = Arc::new(Mutex::new(child));
    self.running.insert(about.to_owned(), Arc::clone(&child));
    let (voice, about) = (self.voice.clone(), about.to_owned());
    thread::spawn(move || {
      let code = loop {
        match child.lock().ok().and_then(|mut one| one.try_wait().ok().flatten()) {
          Some(held) => break held.code().map_or(Value::None, |one| Value::Int(i64::from(one))),
          None => thread::sleep(Duration::from_millis(5)),
        }
      };
      for one in streams {
        let _ = one.join();
      }
      voice.send(Fact::new("exited", &about, WORLD, vec![code]));
    });
  }

  /// One entry of the record onto its file, as the python World writes it.
  fn keep(&self, entry: &Value) -> Option<()> {
    let (at, held) = (self.record.as_ref()?, entry.as_entries()?);
    let made = Entry {
      before: held.first().and_then(Value::as_str).unwrap_or_default().to_owned(),
      fact: Fact(held.get(1).and_then(Value::as_entries).unwrap_or_default().to_vec()),
      answer: held.get(2).cloned(),
    };
    let mut file = fs::OpenOptions::new().create(true).append(true).open(at).ok()?;
    writeln!(file, "{}", made.line()).ok()
  }
}

impl World for Yard {
  fn hears(&mut self, fact: &Fact) -> Reply {
    let words = fact.words();
    if fact.question() && ["bash", "wait", "prompt"].contains(&fact.kind()) {
      self.acts.insert(fact.about().to_owned(), fact.clone());
    }
    match fact.kind() {
      "stand" => {
        let standing = Value::Tuple(vec![
          self.roster.clone(),
          Value::Str(self.at.display().to_string()),
          Value::Str("m/low".to_owned()),
        ]);
        Reply::say(Fact::new("done", fact.about(), WORLD, vec![standing]))
      }
      "clock" | "chance" => Reply::say(Fact::new("done", fact.about(), WORLD, vec![Value::Float(0.5)])),
      "read" | "write" => {
        // Where a path resolves is the chain's to say, so it is asked before the disk is touched.
        self.asking = Some(Asking::Path(fact.clone()));
        Reply::ask("cwd", fact.on().unwrap_or_default())
      }
      "start" => {
        let Some(act) = self.acts.get(fact.about()).cloned() else { return Reply::Nothing };
        if act.kind() != "bash" {
          self.started(&act, "");
          return Reply::Nothing;
        }
        let on = act.on().unwrap_or_default().to_owned();
        self.asking = Some(Asking::Merged(act));
        Reply::Ask { kind: "merged".to_owned(), on, words: vec![Value::Str(fact.about().to_owned())] }
      }
      "ask" => {
        self.read.push(text(words.get(2).unwrap_or(&Value::None)));
        let word = if self.words.is_empty() { "close(None)".to_owned() } else { self.words.remove(0) };
        let turn = Value::Tuple(vec![
          Value::Str("assistant".to_owned()),
          Value::List(vec![Value::Str(word)]),
          Value::None,
          Value::List(vec![]),
        ]);
        self.voice.send(Fact::new("answer", fact.about(), WORLD, vec![turn]));
        Reply::Nothing
      }
      "feed" => {
        if let Some(held) = self.running.get(fact.about())
          && let Ok(mut child) = held.lock()
        {
          match words.first().and_then(Value::as_str) {
            Some(said) => {
              if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(said.as_bytes());
              }
            }
            None => {
              child.stdin.take();
            }
          }
        }
        Reply::Nothing
      }
      "cancel" | "close" => {
        let held: Vec<String> = self.running.keys().cloned().collect();
        let said: Vec<Fact> = held
          .into_iter()
          .filter_map(|one| {
            let child = self.running.remove(&one)?;
            let _ = child.lock().map(|mut held| held.kill());
            Some(Fact::new("exited", &one, WORLD, vec![Value::None]))
          })
          .collect();
        if said.is_empty() { Reply::Nothing } else { Reply::Say(said) }
      }
      "exited" => {
        self.running.remove(fact.about());
        Reply::Nothing
      }
      "keep" => {
        if let Some(entry) = words.first() {
          self.keep(entry);
        }
        Reply::Nothing
      }
      _ => Reply::Nothing,
    }
  }

  fn answered(&mut self, got: &Value) -> Reply {
    let Some(asking) = self.asking.take() else { return Reply::Nothing };
    let here = got.as_str().unwrap_or_default().to_owned();
    match asking {
      Asking::Path(held) if held.kind() == "read" => {
        let path = held.words().get(1).and_then(Value::as_str).unwrap_or_default().to_owned();
        let at = self.at.join(&here).join(&path);
        let said = match fs::read_to_string(&at) {
          Ok(one) => Value::text(at.display().to_string(), one),
          Err(_) => Value::refused(format!("no file at {}", at.display())),
        };
        Reply::say(Fact::new("done", held.about(), WORLD, vec![said]))
      }
      Asking::Path(held) => {
        let text = held.words().get(1).cloned().unwrap_or(Value::None);
        let path = text.field("path").and_then(Value::as_str).unwrap_or_default();
        let content = text.field("content").and_then(Value::as_str).unwrap_or_default();
        let at = self.at.join(&here).join(path);
        if let Some(over) = at.parent() {
          fs::create_dir_all(over).expect("a directory of the test");
        }
        fs::write(&at, content).expect("a file of the test");
        Reply::say(Fact::new("done", held.about(), WORLD, vec![Value::text(at.display().to_string(), content)]))
      }
      Asking::Merged(held) => {
        let on = held.on().unwrap_or_default().to_owned();
        self.asking = Some(Asking::Where(held));
        Reply::ask("cwd", on)
      }
      Asking::Where(held) => {
        self.started(&held, &here);
        Reply::Nothing
      }
    }
  }
}

/// The seconds a word of an act names, which the engine says as a number of either kind.
fn seconds(word: Option<&Value>) -> Duration {
  match word {
    Some(Value::Float(held)) => Duration::from_secs_f64(held.max(0.0)),
    Some(Value::Int(held)) => Duration::from_secs(u64::try_from(*held).unwrap_or(0)),
    _ => Duration::ZERO,
  }
}

/// Every text a value holds, one after the other, which is enough for a test to read what a model was told.
fn text(got: &Value) -> String {
  match got {
    Value::Str(held) => held.clone(),
    Value::List(held) | Value::Tuple(held) => held.iter().map(text).collect::<Vec<String>>().join(" "),
    Value::Map(held) | Value::Shape { fields: held, .. } => {
      held.iter().map(|(_, one)| text(one)).collect::<Vec<String>>().join(" ")
    }
    Value::Error { name, args } => format!("{name} {}", args.iter().map(text).collect::<Vec<String>>().join(" ")),
    _ => String::new(),
  }
}

/// One life of the real engine, on a yard of the disk, with the model answering these words.
fn life(yard: &str, words: &[&str]) -> (Life<Far, Yard, Strict>, PathBuf, Voice) {
  let at = std::env::temp_dir().join(format!("furb-life-{yard}"));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).expect("a yard of the test");
  let (voice, ears) = Ears::made();
  let world = Yard::new(at.clone(), voice.clone(), words);
  let held = Life::boot(Far::open(), world, Strict, ears, &[]).expect("a life of the real engine");
  (held, at, voice)
}

/// A second life on what a World kept of the first.
fn again(at: PathBuf, kept: &[Entry]) -> Result<Life<Far, Yard, Strict>, furb::Refusal> {
  let (voice, ears) = Ears::made();
  let mut world = Yard::new(at, voice, &[]);
  world.record = None;
  Life::boot(Far::open(), world, Strict, ears, kept)
}

/// What an act came to, once the host has had its moment to answer for a model, a person and a command.
fn came(held: &mut Life<Far, Yard, Strict>, act: &Act) -> Value {
  let waited = Instant::now();
  while waited.elapsed() < Duration::from_secs(30) {
    if let Some(got) = held.came(act).expect("the life answers what an act came to") {
      return got;
    }
    held.waits(Duration::from_millis(20));
  }
  panic!("{act} never came to anything")
}

/// What the turns of a chain say, as one text, which is what the model of that chain reads.
fn told(held: &mut Life<Far, Yard, Strict>, on: &str) -> String {
  let got = held.calls(Turns { on }).expect("the turns of the chain");
  got.iter().map(text).collect::<Vec<String>>().join("\n")
}

#[test]
fn a_life_of_the_real_engine_opens_on_its_root_and_answers_what_the_root_stands_on() {
  let (mut held, at, _) = life("open", &[]);
  assert_eq!(held.root(), "chain://operator.1");
  let root = held.root().to_owned();
  assert_eq!(held.calls(Cwd { on: &root }).unwrap(), at.display().to_string());
  assert_eq!(held.calls(Turns { on: &root }).unwrap().len(), 1);
}

#[test]
fn a_world_serves_a_read_and_a_write_of_the_real_engine() {
  let (mut held, at, _) = life("disk", &[]);
  let root = held.root().to_owned();
  let text = furb::Text::new("a.txt", "one\ntwo\n");
  let got = held.calls(Writes { text, on: &root }).unwrap();
  assert_eq!(got.content, "one\ntwo\n");
  assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "one\ntwo\n");
  let got = held.calls(Reads { path: "a.txt", on: &root, ..Reads::default() }).unwrap();
  assert_eq!(got.lines(), ["one", "two"]);
  let no = held.calls(Reads { path: "none.txt", on: &root, ..Reads::default() }).unwrap_err();
  assert!(no.refused(), "{no}");
}

#[test]
fn a_model_answers_a_prompt_of_the_real_engine_and_the_kernel_runs_the_word_it_wrote() {
  let (mut held, _, _) = life("prompt", &["close(len(read('a.txt').lines))"]);
  let root = held.root().to_owned();
  let text = furb::Text::new("a.txt", "one\ntwo\nthree\n");
  held.calls(Writes { text, on: &root }).unwrap();
  let act = held.calls(Prompt { shape: "int", message: "count the lines", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(3));
  assert!(held.world().read[0].contains("count the lines"), "{:?}", held.world().read[0]);
}

#[test]
fn the_gate_of_the_host_refuses_a_word_and_the_engine_asks_the_model_again() {
  let (mut held, _, _) = life("gate", &["close(BAD)", "close(7)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "count", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(7));
  assert!(held.world().read[1].contains("BAD in rung"), "{:?}", held.world().read[1]);
}

#[test]
fn a_command_of_the_real_engine_runs_in_the_world_and_says_what_it_came_to() {
  let (mut held, _, _) = life("bash", &[]);
  let root = held.root().to_owned();
  let act = held.calls(Bash { command: "echo hi; exit 3", on: &root, ..Bash::default() }).unwrap();
  let got = came(&mut held, &act);
  assert_eq!(got.field("code"), Some(&Value::Int(3)));
  assert_eq!(furb::Exit::of(&got).expect("an exit of the command").stdout.content, "hi\n");
}

#[test]
fn the_operator_answers_a_prompt_of_the_operator_by_closing_it() {
  let (mut held, _, _) = life("operator", &[]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "str", message: "say a word", to: "operator", on: &root }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Str("the operator answered".to_owned()));
}

#[test]
fn a_second_life_on_the_record_the_world_kept_makes_the_same_acts_again() {
  let (mut held, at, _) = life("again", &["close(2)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "one plus one", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(2));
  drop(held);

  let kept = furb::record::read(&fs::read_to_string(at.join("record.jsonl")).unwrap()).expect("the record");
  assert!(kept.len() > 1, "the World kept {} entries", kept.len());
  let mut held = again(at, &kept).expect("a second life on the record");
  assert_eq!(held.root(), root);
  assert_eq!(held.calls(Peek { at: &act, on: &root }).unwrap(), Value::Int(2));
  assert!(held.world().read.is_empty(), "a later life asks no model again");
}

#[test]
fn what_a_verb_gave_is_read_as_the_shape_the_contract_gives() {
  let (mut held, _, _) = life("shapes", &[]);
  let root = held.root().to_owned();
  assert_eq!(held.calls(furb::verb::Clock { on: &root }).unwrap(), 0.5);
  assert_eq!(held.calls(furb::verb::Gate { word: "close(BAD)", returns: "int", on: &root }).unwrap(), ["BAD in rung"]);
  assert_eq!(held.calls(furb::verb::Cd { path: "w", on: &root }).unwrap(), "w");
  assert_eq!(held.calls(Cwd { on: &root }).unwrap(), "w");
}

#[test]
fn what_the_engine_raised_reaches_the_host_as_the_exception_it_is() {
  let (mut held, _, _) = life("raised", &[]);
  let no = held.word("get('bash://operator.9')").unwrap_err();
  assert_eq!(no.name(), "KeyError");
  assert!(!no.refused(), "{no}");
  assert_eq!(<Prompt as Verb>::gave(&Value::Int(3)).unwrap_err().name(), "");
}

#[test]
fn a_word_of_a_model_awaits_an_act_and_the_kernel_carries_the_run_forward() {
  let (mut held, _, _) = life("await", &["close((await bash('echo hi')).code)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "run it", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(0));
}

#[test]
fn a_word_that_raises_is_asked_again_and_the_model_reads_what_it_raised() {
  let (mut held, _, _) = life("raises", &["close(1 // 0)", "close(5)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "count", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(5));
  assert!(held.world().read[1].contains("ZeroDivisionError"), "{:?}", held.world().read[1]);
}

#[test]
fn what_a_word_tells_stands_in_the_turns_the_model_reads_next() {
  // A tell outside a run tells nothing, which the contract says, so the word of a rung says this one.
  let (mut held, _, _) = life("tell", &["tell('noted', ('by', 'the word'), body='go on')", "close(1)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "note it", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(1));
  let said = &held.world().read[1];
  assert!(said.contains("noted") && said.contains("go on"), "{said}");
}

#[test]
fn a_cancel_of_the_operator_ends_an_act_and_what_it_came_to_says_so() {
  let (mut held, _, _) = life("cancel", &[]);
  let root = held.root().to_owned();
  let act = held.calls(furb::verb::Wait { seconds: 30.0, on: &root }).unwrap();
  assert_eq!(held.came(&act).unwrap(), None);
  held.calls(furb::verb::Cancel { id: &act }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Error { name: "CancelledError".to_owned(), args: vec![] });
}

#[test]
fn a_chain_of_the_operator_has_a_transcript_of_its_own() {
  let (mut held, _, _) = life("chain", &[]);
  let root = held.root().to_owned();
  let made = held.calls(furb::verb::Chain { label: "work", on: &root, ..furb::verb::Chain::default() }).unwrap();
  assert_ne!(made.0, root);
  assert!(told(&mut held, &made).contains("work"), "a chain tells the label it was opened with");
}

#[test]
fn the_stdin_of_a_fed_command_takes_what_a_write_of_its_door_says() {
  let (mut held, _, _) = life("fed", &[]);
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
  let (mut held, _, _) = life("show", &["close(len(read('a.txt', span(2, 3)).lines))"]);
  let root = held.root().to_owned();
  held.calls(Writes { text: furb::Text::new("a.txt", "one\ntwo\nthree\nfour\n"), on: &root }).unwrap();
  let act = held.calls(Prompt { shape: "int", message: "read it", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(4));
  let whole = told(&mut held, &root);
  assert!(whole.contains("2 two\n3 three"), "{whole}");
  assert!(!whole.contains("four"), "{whole}");
}

#[test]
fn a_chain_a_host_pauses_makes_no_rung_until_the_operator_wakes_it() {
  let (mut held, _, voice) = life("pause", &["close(9)"]);
  let root = held.root().to_owned();
  voice.pause(&root);
  held.heard().unwrap();
  let act = held.calls(Prompt { shape: "int", message: "count", on: &root, ..Prompt::default() }).unwrap();
  for _ in 0..20 {
    assert_eq!(held.came(&act).unwrap(), None);
    thread::sleep(Duration::from_millis(5));
  }
  assert!(held.world().read.is_empty(), "a paused chain asks no model");
  held.calls(furb::verb::Wake { id: &root }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(9));
}

#[test]
fn a_record_that_holds_an_act_the_life_makes_otherwise_is_a_drift() {
  let (mut held, at, _) = life("drift", &["close((await bash('echo hi')).code)"]);
  let root = held.root().to_owned();
  let act = held.calls(Prompt { shape: "int", message: "run it", on: &root, ..Prompt::default() }).unwrap();
  assert_eq!(came(&mut held, &act), Value::Int(0));
  drop(held);

  // The command of the act alone, and not the word that made it, so the word makes an act the record denies.
  let lines = fs::read_to_string(at.join("record.jsonl")).unwrap().replace("\"echo hi\"", "\"echo bye\"");
  let kept = furb::record::read(&lines).expect("a record of one life");
  match again(at, &kept) {
    Err(no) => assert_eq!(no.name(), "Drift", "{no}"),
    Ok(_) => panic!("a record that holds an act the life makes otherwise is a drift"),
  }
}

#[test]
fn a_chain_with_a_source_keeps_the_acts_its_filter_names_and_no_others() {
  let (mut held, _, _) = life("source", &[]);
  let root = held.root().to_owned();
  let command = held.calls(Bash { command: "echo one", on: &root, ..Bash::default() }).unwrap();
  came(&mut held, &command);
  let waiting = held.calls(furb::verb::Wait { seconds: 0.0, on: &root }).unwrap();
  came(&mut held, &waiting);

  let twin = held
    .calls(furb::verb::Chain {
      label: "twin",
      source: &root,
      filter: Some(furb::Filter::take([command.0.clone()])),
      on: &root,
    })
    .unwrap();
  let whole = told(&mut held, &twin);
  assert!(whole.contains(&command.0), "the twin keeps the act its filter names: {whole}");
  assert!(!whole.contains(&waiting.0), "and keeps no other: {whole}");
}
