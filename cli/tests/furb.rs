//! furb, driven as a process, as an operator drives it: the commands, and the TUI it hands the terminal to.
//!
//! Every life runs on the real engine and on the real ears of the crate, and a claude command line that answers from
//! a script stands in for the models, so no test asks one. The fake answers each line of input with the next word the
//! test gave it, and with `close(None)` once the words run out, and it counts what it answered.

#![cfg(unix)]

use std::{
  fs,
  io::Write,
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  process::{Command, Output, Stdio},
};

use serde_json::Value;

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

  /// furb with these words, in the yard, on the fake claude, and with no TUI that the machine names.
  fn furb(&self, words: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_furb"));
    command.args(words).current_dir(&self.at);
    command.env("FURB_CLAUDE_BIN", self.fake.join("claude")).env_remove("FURB_TUI");
    command
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
fn the_command_line_takes_three_commands_of_the_operator() {
  let yard = Yard::new("commands");
  let help = printed(yard.furb(&["--help"]), "");
  for command in ["prompt", "turns", "run"] {
    assert!(help.contains(&format!("  {command} ")), "{help}");
  }
  let output = ran(yard.furb(&["prompt", "count", "--shape", "nothing"]), "");
  assert_eq!(output.status.code(), Some(2));
  let said = String::from_utf8_lossy(&output.stderr);
  assert!(said.contains("[possible values: none, str, int, float, bool, list]"), "{said}");
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
  fs::create_dir_all(&elsewhere).expect("another directory");
  let mut turns = yard.furb(&["turns", "--record"]);
  turns.arg(yard.record()).arg("--cwd").arg(&elsewhere);
  let said = printed(turns, "");
  assert!(said.contains("#rung2 closed 4"), "the turns of a record run every word again: {said}");
  assert_eq!(
    fs::read(yard.record()).expect("the record"),
    before,
    "a life that only reads keeps nothing"
  );
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
  assert!(said.contains("furb: prompt1 is paused: opus/low answered nothing: "), "{said}");
  assert!(said.contains("A wake from the TUI makes it go on."), "{said}");
  yard.words(&[COUNT]);
  assert_eq!(printed(yard.kept(&counted), ""), "3\n", "the command wakes the prompt it takes up");
  assert_eq!(yard.answered(), 1);
}

#[test]
fn furb_hands_the_terminal_to_the_tui_with_the_words_it_takes() {
  let yard = Yard::new("tui");
  let tui = yard.at.join("tui");
  program(&tui, "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit 3\n");
  let mut command =
    yard.furb(&["--demo", "--record", "r.jsonl", "--cwd", "there", "--", "--model", "m"]);
  command.env("FURB_TUI", &tui);
  let output = ran(command, "");
  assert_eq!(output.status.code(), Some(3), "furb ends with the code of the TUI");
  let words = String::from_utf8_lossy(&output.stdout);
  assert_eq!(words, "--demo\n--record\nr.jsonl\n--cwd\nthere\n--model\nm\n");
}
