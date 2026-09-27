//! furb, driven as a process, as an operator or a client drives it: the commands, the JSON-RPC, and the TUI it hands
//! the terminal to.
//!
//! Every life runs on the real engine and on the real ears of the crate, and a claude command line that answers from
//! a script stands in for the models, so no test asks one. The fake answers each line of input with the next word the
//! test gave it, and with `close(None)` once the words run out, and it counts what it answered.

#![cfg(unix)]

use std::{
  fs,
  io::{BufRead, BufReader, Write},
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  process::{Child, ChildStdin, Command, Output, Stdio},
  sync::mpsc,
  thread,
  time::{Duration, Instant},
};

use serde_json::{Value, json};

/// The fake claude: each turn is one message that the command line says whole, and the result of the turn.
const FAKE: &str = r#"#!/bin/sh
here=$(dirname "$0")
while IFS= read -r line; do
  n=$(( $(cat "$here/count" 2>/dev/null || echo 0) + 1 ))
  echo "$n" > "$here/count"
  word=$(cat "$here/word.$n" 2>/dev/null || printf '"close(None)"')
  printf '{"type":"assistant","message":{"content":[{"type":"text","text":%s}]}}\n' "$word"
  printf '{"type":"result","session_id":"fake","total_cost_usd":0,"usage":{"input_tokens":10,"output_tokens":5}}\n'
done
"#;

/// A word that counts the lines of the file of the yard.
const COUNT: &str = "close(len(read('a.txt').lines))";

/// A directory of one test, and the fake claude beside it.
struct Yard {
  at: PathBuf,
  fake: PathBuf,
}

impl Yard {
  fn new(name: &str) -> Yard {
    let root = std::env::temp_dir().join(format!("furb-cli-{name}"));
    let _ = fs::remove_dir_all(&root);
    let (at, fake) = (root.join("work"), root.join("fake"));
    fs::create_dir_all(&at).expect("a yard of the test");
    fs::create_dir_all(&fake).expect("a home of the fake");
    fs::write(at.join("a.txt"), "one\ntwo\nthree\n").expect("the file a word reads");
    program(&fake.join("claude"), FAKE);
    Yard { at, fake }
  }

  /// The words the fake answers with, in order.
  fn words(&self, words: &[&str]) {
    for (at, word) in words.iter().enumerate() {
      let said = serde_json::to_string(word).expect("a word as JSON");
      fs::write(self.fake.join(format!("word.{}", at + 1)), said).expect("the word is written");
    }
  }

  /// How many turns the fake answered.
  fn answered(&self) -> usize {
    let count = fs::read_to_string(self.fake.join("count")).unwrap_or_default();
    count.trim().parse().unwrap_or_default()
  }

  fn record(&self) -> PathBuf {
    self.at.join("record.jsonl")
  }

  /// furb with these words, in the yard, on the fake claude and on the config directory of the yard, and with no
  /// TUI that the machine names.
  fn furb(&self, words: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_furb"));
    command.args(words).current_dir(&self.at);
    command.env("FURB_CLAUDE_BIN", self.fake.join("claude")).env_remove("FURB_TUI");
    command.env("FURB_CONFIG_DIR", self.config());
    command
  }

  /// The config directory of the user, in the yard.
  fn config(&self) -> PathBuf {
    self.fake.with_file_name("config")
  }

  /// furb with these words, on the record of the yard.
  fn kept(&self, words: &[&str]) -> Command {
    let mut command = self.furb(words);
    command.arg("--record").arg(self.record());
    command
  }
}

/// A program of the test, written and made to run.
fn program(at: &Path, script: &str) {
  fs::write(at, script).expect("the program is written");
  fs::set_permissions(at, fs::Permissions::from_mode(0o755)).expect("the program runs");
}

/// What a command did, given this input.
fn ran(mut command: Command, input: &str) -> Output {
  let piped = || Stdio::piped();
  let mut child =
    command.stdin(piped()).stdout(piped()).stderr(piped()).spawn().expect("furb starts");
  let mut stdin = child.stdin.take().expect("the stdin of furb");
  stdin.write_all(input.as_bytes()).expect("furb reads its input");
  drop(stdin);
  child.wait_with_output().expect("furb ends")
}

/// What a command printed on stdout, once it ended well.
fn printed(command: Command, input: &str) -> String {
  let output = ran(command, input);
  let err = String::from_utf8_lossy(&output.stderr);
  assert!(output.status.success(), "furb failed: {err}");
  String::from_utf8_lossy(&output.stdout).into_owned()
}

/// What a command said on stderr, once it failed.
fn refused(command: Command, input: &str) -> String {
  let output = ran(command, input);
  assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
  String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_command_line_takes_four_commands_of_the_operator() {
  let yard = Yard::new("commands");
  let help = printed(yard.furb(&["--help"]), "");
  for command in ["prompt", "turns", "run", "extensions"] {
    assert!(help.contains(&format!("  {command} ")), "{help}");
  }
  let output = ran(yard.furb(&["prompt", "count", "--shape", "nothing"]), "");
  assert_eq!(output.status.code(), Some(2));
  let said = String::from_utf8_lossy(&output.stderr);
  assert!(said.contains("[possible values: str, None, bool, int, float, list, dict]"), "{said}");
  assert_eq!(
    ran(yard.furb(&["turns"]), "").status.code(),
    Some(2),
    "turns reads a record it is given"
  );
}

#[test]
fn a_word_its_caller_wrote_runs_on_the_root() {
  let yard = Yard::new("run");
  assert_eq!(printed(yard.furb(&["run", "k = 1\nclose(k + 1)"]), ""), "2\n");
  assert_eq!(printed(yard.furb(&["run", "k = 1"]), ""), "None\n");
  assert!(!yard.record().exists(), "a life given no record keeps none");
}

#[test]
fn a_life_is_opened_on_the_record_it_is_given_and_resumed_from_it() {
  let yard = Yard::new("resumed");
  fs::create_dir_all(yard.at.join(".furb")).expect("the folder of the project");
  let off = r#"{"extensions": {"memory": false, "skills": false}}"#;
  fs::write(yard.at.join(".furb/config.json"), off).expect("a config of the project");
  assert_eq!(printed(yard.kept(&["run", "k = 3"]), ""), "None\n");
  assert_eq!(printed(yard.kept(&["run", "close(k + 1)"]), ""), "4\n");
  let kinds: Vec<String> = fs::read_to_string(yard.record())
    .expect("the record")
    .lines()
    .map(|line| serde_json::from_str::<Value>(line).expect("an entry")[0][0].to_string())
    .collect();
  let stands = kinds.iter().filter(|kind| *kind == "\"stand\"").count();
  assert_eq!(stands, 2, "each life that keeps keeps its stand: {kinds:?}");
  let before = fs::read(yard.record()).expect("the record");
  let elsewhere = yard.at.join("elsewhere");
  fs::create_dir_all(&elsewhere).expect("another directory, whose configs turn the extensions on");
  let mut turns = yard.furb(&["turns", "--record"]);
  turns.arg(yard.record()).arg("--cwd").arg(&elsewhere);
  let said = printed(turns, "");
  assert!(said.contains("#rung2 closed 4"), "the turns of a record run every word again: {said}");
  assert!(!said.contains("remember()"), "an inspection enables no extension: {said}");
  assert_eq!(fs::read(yard.record()).expect("the record"), before, "an inspection keeps nothing");
}

#[test]
fn a_life_runs_the_extensions_that_its_record_enables_and_those_that_the_configs_turn_on() {
  let yard = Yard::new("extensions");
  let skill = yard.at.join(".furb/skills/brew/SKILL.md");
  fs::create_dir_all(skill.parent().expect("a folder")).expect("the folder of a skill");
  fs::write(&skill, "---\nname: brew\ndescription: Make tea.\n---\n").expect("a skill");
  fs::write(yard.at.join("CLAUDE.md"), "Use two spaces.\n").expect("a memory file");
  let every = "memory: remember()\nskills: skills()\n";
  assert_eq!(printed(yard.furb(&["extensions"]), ""), every, "the official extensions are on");
  let path = printed(yard.kept(&["run", "close(skill('brew').path)"]), "");
  assert_eq!(path, format!("{:?}\n", skill.display().to_string()).replace('"', "'"));
  let mut turns = yard.furb(&["turns", "--record"]);
  turns.arg(yard.record());
  let said = printed(turns, "");
  assert!(said.contains("#memory ") && said.contains("# 1 Use two spaces."), "{said}");
  fs::create_dir_all(yard.at.join(".furb")).expect("the folder of the project");
  fs::write(yard.at.join(".furb/config.json"), r#"{"extensions": {"memory": false}}"#)
    .expect("a config of the project");
  assert_eq!(printed(yard.furb(&["extensions"]), ""), "skills: skills()\n");
  assert_eq!(printed(yard.kept(&["extensions"]), ""), every, "a life runs what its record enables");
}

#[test]
fn a_prompt_of_the_operator_is_answered_at_the_terminal() {
  let yard = Yard::new("terminal");
  let output =
    ran(yard.furb(&["prompt", "say a word", "--to", "operator", "--shape", "str"]), "a word\n");
  assert_eq!(String::from_utf8_lossy(&output.stdout), "'a word'\n");
  let asked = String::from_utf8_lossy(&output.stderr);
  assert_eq!(asked, "prompt1 wants a str: say a word\n> ");
  assert_eq!(printed(yard.furb(&["prompt", "say nothing", "--to", "operator"]), "\n"), "None\n");
  let said =
    refused(yard.furb(&["prompt", "count", "--to", "operator", "--shape", "int"]), "many\n");
  assert!(said.ends_with("furb: Refused: \"many\" is no int\n"), "{said}");
  let said = refused(yard.furb(&["prompt", "count", "--to", "operator", "--shape", "int"]), "");
  assert!(
    said.ends_with("furb: Refused: the operator cannot be read: the input is over\n"),
    "{said}"
  );
}

#[test]
fn the_life_of_a_prompt_offers_the_model_of_the_actor_it_names() {
  let yard = Yard::new("to");
  yard.words(&["close('hi')"]);
  assert_eq!(
    printed(yard.furb(&["prompt", "hi", "--to", "sonnet/high", "--shape", "str"]), ""),
    "'hi'\n"
  );
}

#[test]
fn a_prompt_the_record_already_holds_is_taken_up_and_never_asked_again() {
  let yard = Yard::new("again");
  let asked = ["prompt", "say a number", "--to", "operator", "--shape", "int"];
  assert_eq!(printed(yard.kept(&asked), "7\n"), "7\n");
  assert_eq!(
    printed(yard.kept(&asked), ""),
    "7\n",
    "the record answers, and the operator is asked nothing"
  );
  yard.words(&[COUNT]);
  let counted = ["prompt", "count the lines", "--shape", "int"];
  assert_eq!(printed(yard.kept(&counted), ""), "3\n");
  assert_eq!(printed(yard.kept(&counted), ""), "3\n");
  assert_eq!(yard.answered(), 1, "a later life asks no model for what its record holds");
  let mut turns = yard.furb(&["turns", "--record"]);
  turns.arg(yard.record());
  let said = printed(turns, "");
  assert!(said.starts_with("[user] #chain1 root\n"), "{said}");
  let paragraphs =
    "\n\n#prompt1 say a number\nprompt1: Act[int] = Act('prompt1')\n\n#prompt1 closed 7";
  assert!(said.contains(paragraphs), "the turns of a root as a model read them: {said}");
  assert!(said.contains(&format!("\n[assistant] {COUNT}\n")), "{said}");
}

#[test]
fn a_prompt_the_world_paused_ends_its_command_with_why_and_goes_on_when_it_is_taken_up() {
  let yard = Yard::new("paused");
  let counted = ["prompt", "count the lines", "--shape", "int"];
  let mut missing = yard.kept(&counted);
  missing.env("FURB_CLAUDE_BIN", yard.at.join("missing"));
  let said = refused(missing, "");
  assert!(
    said.contains("furb: prompt1 is paused: claude-cli:opus/low answered nothing: "),
    "{said}"
  );
  assert!(said.contains("A wake from the TUI or from `furb --mode rpc` makes it go on."), "{said}");
  yard.words(&[COUNT]);
  assert_eq!(printed(yard.kept(&counted), ""), "3\n", "the command wakes the prompt it takes up");
  assert_eq!(yard.answered(), 1);
}

/// A client of `furb --mode rpc`: its line into the stdin of furb, and each record furb writes, as it comes.
struct Client {
  child: Child,
  stdin: Option<ChildStdin>,
  records: mpsc::Receiver<Value>,
  /// The records read and not yet looked for.
  held: Vec<Value>,
}

impl Client {
  /// furb serving a life on the record of the yard.
  fn new(yard: &Yard) -> Client {
    Client::with(yard, &[])
  }

  /// furb serving a life on the record of the yard, with more words.
  fn with(yard: &Yard, words: &[&str]) -> Client {
    let mut child = yard
      .kept(&[&["--mode", "rpc"], words].concat())
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .spawn()
      .expect("furb starts");
    let stdin = child.stdin.take();
    let stdout = BufReader::new(child.stdout.take().expect("the stdout of furb"));
    let (sends, records) = mpsc::channel();
    thread::spawn(move || {
      for line in stdout.lines().map_while(Result::ok) {
        let record = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line} is no JSON"));
        if sends.send(record).is_err() {
          return;
        }
      }
    });
    Client { child, stdin, records, held: Vec::new() }
  }

  /// One line into the stdin of furb.
  fn line(&mut self, line: &str) {
    let stdin = self.stdin.as_mut().expect("the stdin of furb");
    writeln!(stdin, "{line}").expect("furb reads a command");
  }

  /// The response to a command, sent under this id.
  fn asked(&mut self, id: &str, mut command: Value) -> Value {
    command["id"] = json!(id);
    self.line(&command.to_string());
    self.until(|one| one["type"] == "response" && one["id"] == id)
  }

  /// The data of the response to a command that succeeded.
  fn data(&mut self, id: &str, command: Value) -> Value {
    let response = self.asked(id, command);
    assert_eq!(response["success"], true, "{response}");
    response["data"].clone()
  }

  /// The event that an act is done.
  fn done(&mut self, act: &str) -> Value {
    self.until(|one| one["type"] == "done" && one["act"] == act)
  }

  /// The first record that matches, taken, or a failure when none comes within a minute.
  fn until(&mut self, what: impl Fn(&Value) -> bool) -> Value {
    if let Some(at) = self.held.iter().position(&what) {
      return self.held.remove(at);
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
      let left = deadline.saturating_duration_since(Instant::now());
      let Ok(one) = self.records.recv_timeout(left) else {
        panic!("no record came that matches, after {:#?}", self.held);
      };
      if what(&one) {
        return one;
      }
      self.held.push(one);
    }
  }

  /// Whether no record that matches comes for a while.
  fn quiet(&mut self, what: impl Fn(&Value) -> bool, time: Duration) -> bool {
    let deadline = Instant::now() + time;
    while let Ok(one) =
      self.records.recv_timeout(deadline.saturating_duration_since(Instant::now()))
    {
      self.held.push(one);
    }
    !self.held.iter().any(what)
  }

  /// The end of the life: stdin ends, and furb with it.
  fn ended(mut self) -> bool {
    drop(self.stdin.take());
    self.child.wait().expect("furb ends").success()
  }
}

impl Drop for Client {
  fn drop(&mut self) {
    let _ = self.child.kill();
  }
}

#[test]
fn a_prompt_of_the_client_is_answered_by_a_model_and_its_done_is_sent() {
  let yard = Yard::new("rpc-prompt");
  yard.words(&[COUNT]);
  let mut client = Client::new(&yard);
  let asked = json!({"type": "prompt", "message": "count the lines", "shape": "int"});
  let response = client.asked("1", asked);
  assert_eq!(
    response,
    json!({"id": "1", "type": "response", "command": "prompt", "success": true, "data": {"act": "prompt1"}})
  );
  assert_eq!(client.done("prompt1"), json!({"type": "done", "act": "prompt1", "value": 3}));
  let fact = client.until(|one| one["type"] == "fact" && one["fact"][0] == "prompt");
  assert_eq!(
    fact["fact"],
    json!(["prompt", "prompt1", "operator", "chain1", "int", "count the lines", ""])
  );
  let peeked = client.data("2", json!({"type": "peek", "act": "prompt1"}));
  assert_eq!(peeked, json!({"done": true, "value": 3}));
  let turns = client.data("3", json!({"type": "turns"}));
  let turns = turns["turns"].as_array().expect("the turns");
  assert_eq!((&turns[1][0], &turns[1][1]), (&json!("assistant"), &json!(COUNT)));
  let facts = client.data("4", json!({"type": "transcript", "on": "chain1"}));
  assert!(facts["facts"].as_array().expect("the facts").contains(&fact["fact"]), "{facts}");
  assert!(client.ended());
  assert_eq!(yard.answered(), 1);
}

#[test]
fn a_prompt_to_the_operator_is_sent_and_the_close_of_the_client_answers_it() {
  let yard = Yard::new("rpc-operator");
  yard.words(&["close(await prompt(int, 'how many?', 'operator'))"]);
  let mut client = Client::new(&yard);
  client.data("1", json!({"type": "prompt", "message": "ask me", "shape": "int"}));
  let asked = client.until(|one| one["type"] == "prompt");
  let prompt = json!({"type": "prompt", "act": "prompt2", "on": "chain1", "shape": "int", "message": "how many?"});
  assert_eq!(asked, prompt);
  let state = client.data("2", json!({"type": "state"}));
  assert_eq!(
    state["prompts"],
    json!([{"act": "prompt2", "on": "chain1", "shape": "int", "message": "how many?"}])
  );
  assert_eq!(
    (&state["root"], &state["paused"], &state["acts"]),
    (&json!("chain1"), &json!(false), &json!(["prompt1"]))
  );
  assert_eq!(state["standing"][2], "claude-cli:opus/low");
  let wrong = client.asked("3", json!({"type": "close", "act": "prompt2", "value": "seven"}));
  assert_eq!(wrong["success"], false, "a close of the wrong shape is refused: {wrong}");
  client.data("4", json!({"type": "close", "act": "prompt2", "value": 7}));
  assert_eq!(client.done("prompt1")["value"], 7);
  assert_eq!(client.data("5", json!({"type": "state"}))["prompts"], json!([]));
  let float = json!({"type": "prompt", "message": "a float", "shape": "float", "to": "operator"});
  let float = client.data("6", float)["act"].as_str().expect("the act").to_owned();
  client.until(|one| one["type"] == "prompt" && one["act"] == float);
  client.data("7", json!({"type": "close", "act": float, "value": 2}));
  let done = client.done(&float);
  assert!(
    done["value"].is_f64() && done["value"] == 2.0,
    "a whole number closes a prompt of a float: {done}"
  );
  let never = json!({"type": "prompt", "message": "never", "shape": "str", "to": "operator"});
  let never = client.data("8", never)["act"].as_str().expect("the act").to_owned();
  client.data("9", json!({"type": "cancel", "act": never}));
  let raised =
    json!({"type": "done", "act": never, "raised": {"is": "CancelledError", "args": []}});
  assert_eq!(client.done(&never), raised);
  let no = json!({"type": "prompt", "message": "no", "shape": "str", "to": "operator"});
  let no = client.data("10", no)["act"].as_str().expect("the act").to_owned();
  let refusal = json!({"is": "Refused", "args": ["no"]});
  client.data("11", json!({"type": "close", "act": no, "value": refusal}));
  assert_eq!(
    client.done(&no)["raised"],
    refusal,
    "an exception closes a prompt with that exception"
  );
}

#[test]
fn a_life_offers_the_model_of_its_default_actor_and_the_models_it_names_and_no_more() {
  let yard = Yard::new("rpc-roster");
  let names = |client: &mut Client| {
    let state = client.data("1", json!({"type": "state"}));
    let roster = state["standing"][0].as_array().cloned().unwrap_or_default();
    let names: Vec<String> =
      roster.iter().filter_map(|one| one[0].as_str().map(str::to_owned)).collect();
    (names, state["standing"][2].clone())
  };
  let mut first = Client::new(&yard);
  let (roster, actor) = names(&mut first);
  assert!(first.ended(), "the record is free for the next life");
  assert_eq!(
    (roster, actor),
    (vec!["claude-cli:opus".to_owned(), "operator".to_owned()], json!("claude-cli:opus/low"))
  );
  let mut client = Client::with(&yard, &["--model", "sonnet/high", "--roster", "haiku"]);
  let (roster, actor) = names(&mut client);
  assert_eq!(roster, ["claude-cli:sonnet", "claude-cli:haiku", "operator"]);
  assert_eq!(actor, "claude-cli:sonnet/high");
  let refused =
    client.data("2", json!({"type": "prompt", "message": "hi", "shape": "str", "to": "opus"}));
  let prompt = refused["act"].as_str().expect("the act").to_owned();
  assert_eq!(client.done(&prompt)["raised"]["is"], "Refused", "the roster holds no opus");
}

#[test]
fn a_word_of_the_client_runs_as_a_rung_and_a_pause_holds_a_chain_until_its_wake() {
  let yard = Yard::new("rpc-rung");
  yard.words(&["close(1)"]);
  let mut client = Client::new(&yard);
  assert_eq!(
    client.data("1", json!({"type": "rung", "word": "k = 1\nclose(k + 1)"})),
    json!({"act": "rung3"}),
    "the two rungs of the extensions come first"
  );
  assert_eq!(client.done("rung3")["value"], 2);
  client.data("2", json!({"type": "pause", "act": "chain1"}));
  let state = client.data("3", json!({"type": "state"}));
  assert_eq!(
    (&state["paused"], &state["extensions"]),
    (&json!(true), &json!(["memory", "skills"]))
  );
  client.data("4", json!({"type": "prompt", "message": "one", "shape": "int"}));
  assert!(
    client.quiet(|one| one["type"] == "done", Duration::from_secs(1)),
    "a paused chain asks no model"
  );
  assert_eq!(yard.answered(), 0);
  client.data("5", json!({"type": "wake", "act": "chain1"}));
  assert_eq!(client.done("prompt1")["value"], 1);
  assert_eq!(client.data("6", json!({"type": "state"}))["paused"], false);
}

#[test]
fn a_command_that_does_nothing_says_why() {
  let yard = Yard::new("rpc-refused");
  let mut client = Client::new(&yard);
  client.line("not json");
  let parse = client.until(|one| one["command"] == "parse");
  assert_eq!((&parse["type"], &parse["success"]), (&json!("response"), &json!(false)));
  let unknown = client.asked("1", json!({"type": "nothing"}));
  assert_eq!(unknown["error"], "Unknown command: nothing");
  assert_eq!(client.asked("2", json!({"type": "close"}))["error"], "The command needs act.");
  assert_eq!(
    client.asked("3", json!({"type": "peek", "act": "bash9"}))["error"],
    "No act is named bash9."
  );
  assert_eq!(client.asked("4", json!({"type": "rung", "word": 1}))["error"], "word is no string.");
  assert_eq!(client.asked("5", json!({"type": "prompt", "to": "nobody"}))["success"], true);
  let done = client.done("prompt1");
  assert_eq!(
    done["raised"]["is"], "Refused",
    "a prompt to no actor of the roster is refused: {done}"
  );
}

#[test]
fn the_life_ends_with_stdin_and_a_later_life_resumes_its_record() {
  let yard = Yard::new("rpc-later");
  yard.words(&[COUNT]);
  let mut client = Client::new(&yard);
  client.data("1", json!({"type": "prompt", "message": "count the lines", "shape": "int"}));
  client.done("prompt1");
  assert!(client.ended(), "furb ends well when its stdin ends");
  let mut later = Client::new(&yard);
  assert_eq!(
    later.data("1", json!({"type": "peek", "act": "prompt1"})),
    json!({"done": true, "value": 3})
  );
  let record = later.data("2", json!({"type": "state"}))["record"].clone();
  assert_eq!(record, json!(yard.record().display().to_string()));
  assert!(later.ended());
  assert_eq!(yard.answered(), 1);
}

#[test]
fn furb_hands_the_terminal_to_the_tui_with_the_words_it_takes() {
  let yard = Yard::new("tui");
  let tui = yard.at.join("tui");
  program(&tui, "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit 3\n");
  let mut command = yard.furb(&[
    "--demo", "--record", "r.jsonl", "--cwd", "there", "--model", "m", "--roster", "n", "--",
    "--effort", "high",
  ]);
  command.env("FURB_TUI", &tui);
  let output = ran(command, "");
  assert_eq!(output.status.code(), Some(3), "furb ends with the code of the TUI");
  let words = String::from_utf8_lossy(&output.stdout);
  let expected =
    "--demo\n--record\nr.jsonl\n--cwd\nthere\n--model\nm\n--roster\nn\n--effort\nhigh\n";
  assert_eq!(words, expected);
}
