//! furb, driven as a process, as an operator or a client drives it: the commands and the TUI it
//! hands the terminal to.
//!
//! Every life runs on the real engine and on the real ears of the crate, and a claude command line that answers from
//! a script stands in for the models, so no test asks one. The fake answers each line of input with the next word the
//! test gave it, and it counts what it answered.

#![cfg(unix)]
#![expect(clippy::expect_used, reason = "debt: an expect where a type can hold the invariant")]
#![expect(
  clippy::let_underscore_must_use,
  reason = "debt: a let _ drops a value that must be used"
)]
#![expect(clippy::let_underscore_untyped, reason = "debt: a let _ hides the type of what it drops")]

#[path = "../../tests/common/pipes.rs"]
mod pipes;

use std::{
  fs,
  io::{BufRead, BufReader, Read, Write},
  net::TcpListener,
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  process::{Child, Command, Output, Stdio},
  sync::mpsc,
  thread,
  time::{Duration, Instant},
};

use serde_json::{Value, json};

/// The fake claude of the tests of the crate, which answers the Nth turn with the word of the file word.N beside it.
const FAKE: &str = include_str!("../../src/world/provider/claude/fake-claude.sh");

/// A word that counts the lines of the file of the yard.
const COUNT: &str = "close(len(read('a.txt').lines))";

/// The env var that keeps the folder of each yard, so that a failure can still be read there.
const KEEP: &str = "FURB_KEEP_YARDS";

/// A directory of one test, and the fake claude beside it. The folder goes with the yard, on a
/// pass and on a failure, unless the env var `FURB_KEEP_YARDS` is set.
struct Yard {
  base: PathBuf,
  at: PathBuf,
  fake: PathBuf,
}

impl Drop for Yard {
  fn drop(&mut self) {
    if std::env::var_os(KEEP).is_some() {
      eprintln!("{KEEP} keeps {}", self.base.display());
    } else {
      let _ = fs::remove_dir_all(&self.base);
    }
  }
}

impl Yard {
  fn new(name: &str) -> Yard {
    // A process that starts in the yard reads its directory as the system gives it, with no link in it: on macOS the
    // temporary directory is under /var, a link to /private/var. So the yard is that path too.
    let base = std::env::temp_dir().join(format!("furb-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("the root of the yard");
    let root = fs::canonicalize(&base).expect("the root of the yard, with no link");
    let (at, fake) = (root.join("work"), root.join("fake"));
    fs::create_dir_all(&at).expect("a yard of the test");
    fs::create_dir_all(&fake).expect("a home of the fake");
    // The life of a test names no session, since a name asks the fake claude out of the order
    // of its words.
    fs::create_dir_all(root.join("config")).expect("the config directory");
    fs::write(root.join("config").join("config.json"), r#"{"extensions": {"naming": false}}"#)
      .expect("the config");
    fs::write(at.join("a.txt"), "one\ntwo\nthree\n").expect("the file a word reads");
    program(&fake.join("claude"), FAKE);
    Yard { base, at, fake }
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

  /// furb with these words, in the yard, on the fake claude and on the config directory and the cache of the yard,
  /// and with no TUI, no local server and no most of thought that the machine names.
  fn furb(&self, words: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_furb"));
    command.args(words).current_dir(&self.at);
    command.env("FURB_CLAUDE_BIN", self.fake.join("claude")).env_remove("FURB_TUI");
    let names = ["FURB_LOCAL_URL", "FURB_LOCAL_KEY", "FURB_LOCAL_WINDOW", "FURB_LOCAL_TOKENS"];
    for name in names.into_iter().chain(["MAX_THINKING_TOKENS"]) {
      command.env_remove(name);
    }
    command.env("FURB_CONFIG_DIR", self.config()).env("XDG_CACHE_HOME", self.at.join("cache"));
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

/// A local server of models on a port of this machine, which lists one model, `m`, and answers each chat completion
/// with this word; and each request it reads, as it reads it. A test of the command line sees no helper of the tests
/// of the crate, so this fake is a second one.
fn served(word: &str) -> (u16, mpsc::Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let listing =
    json!({"object": "list", "data": [{"id": "m", "object": "model", "meta": {"n_ctx": 1000}}]});
  let chunk = |delta: Value, usage: Value| {
    let one = json!({"id": "c", "object": "chat.completion.chunk", "created": 0, "model": "m",
      "choices": [{"index": 0, "delta": delta, "finish_reason": usage.is_object().then_some("stop")}],
      "usage": usage});
    format!("data: {one}\n\n")
  };
  let usage = json!({"prompt_tokens": 100, "completion_tokens": 5, "total_tokens": 105});
  let turn =
    [chunk(json!({"role": "assistant", "content": word}), Value::Null), chunk(json!({}), usage)];
  let turn = format!("{}data: [DONE]\n\n", turn.concat());
  let (tells, told) = mpsc::channel();
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(stream) = stream else { return };
      let mut reader = BufReader::new(stream);
      let mut head = String::new();
      while !head.ends_with("\r\n\r\n") {
        if reader.read_line(&mut head).unwrap_or_default() == 0 {
          break;
        }
      }
      let length = head.lines().find_map(|line| {
        let length = line.to_ascii_lowercase().strip_prefix("content-length: ")?.trim().to_owned();
        length.parse::<usize>().ok()
      });
      let mut body = vec![0; length.unwrap_or_default()];
      let _ = reader.read_exact(&mut body);
      let listed = head.starts_with("GET /v1/models ");
      let _ = tells.send(format!("{head}{}", String::from_utf8_lossy(&body)));
      let (kind, answer) = if listed {
        ("application/json", listing.to_string())
      } else {
        ("text/event-stream", turn.clone())
      };
      let head = format!("HTTP/1.1 200 OK\r\ncontent-type: {kind}\r\nconnection: close\r\n\r\n");
      let _ = reader.into_inner().write_all(format!("{head}{answer}").as_bytes());
    }
  });
  (port, told)
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
  let off = r#"{"extensions": {"catchup": false, "memory": false, "skills": false, "compaction": false, "ouroboros": false, "web": false, "mcp": false}}"#;
  fs::write(yard.at.join(".furb/config.json"), off).expect("a config of the project");
  assert_eq!(printed(yard.kept(&["run", "k = 3"]), ""), "None\n");
  assert_eq!(printed(yard.kept(&["run", "debug(t\"{k=}\")\nclose(k + 1)"]), ""), "4\n");
  let kinds: Vec<String> = fs::read_to_string(yard.record())
    .expect("the record")
    .lines()
    .map(|line| serde_json::from_str::<Value>(line).expect("an entry")[0][0].to_string())
    .collect();
  let stands = kinds.iter().filter(|kind| *kind == "\"stand\"").count();
  assert_eq!(
    stands, 3,
    "each life that keeps keeps its stand, and the setup keeps the stand it tells: {kinds:?}"
  );
  let before = fs::read(yard.record()).expect("the record");
  assert_eq!(
    ran(yard.furb(&["turns"]), "").status.code(),
    Some(2),
    "turns reads a record it is given"
  );
  let elsewhere = yard.at.join("elsewhere");
  fs::create_dir_all(&elsewhere).expect("another directory, whose configs turn the extensions on");
  let mut turns = yard.furb(&["turns", "--record"]);
  turns.arg(yard.record()).arg("--cwd").arg(&elsewhere);
  let said = printed(turns, "");
  assert!(
    said.contains("#rung5 debugged\nrung5_debug = 'k=3'"),
    "the turns of a record run every word again: {said}"
  );
  assert!(!said.contains("remember()"), "an inspection enables no extension: {said}");
  assert_eq!(fs::read(yard.record()).expect("the record"), before, "an inspection keeps nothing");
}

#[test]
fn a_checkpoint_at_the_tip_of_a_record_carries_the_module_of_its_root_to_the_next_life() {
  let yard = Yard::new("checkpoint");
  assert_eq!(printed(yard.kept(&["run", "kept = [1, 2]\nclose(len(kept))"]), ""), "2\n");
  printed(yard.furb(&["checkpoint", "--record", yard.record().to_str().unwrap()]), "");
  let record = fs::read_to_string(yard.record()).unwrap();
  assert!(
    record.lines().last().unwrap().starts_with("[[\"checkpoint\",\"chain1\",\"store\","),
    "{record}"
  );
  assert_eq!(printed(yard.kept(&["run", "close(kept + [3])"]), ""), "[1, 2, 3]\n");
}

#[test]
fn a_word_run_on_a_record_that_holds_pending_work_asks_that_work_again_with_no_wake() {
  let yard = Yard::new("pending");
  // The fake claude gives no answer to a turn that holds WAIT, so the reply runs until the life ends.
  let said = printed(yard.kept(&["run", "prompt(str, 'WAIT', on=chain('w'))\nclose(1)"]), "");
  assert_eq!(said, "1\n");
  let output = ran(yard.kept(&["run", "close(2)"]), "");
  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout), "2\n");
  assert_eq!(String::from_utf8_lossy(&output.stderr), "", "the open names no work that waits");
  // The reply that waits is the one whose start the record holds and whose end it lacks.
  let record = fs::read_to_string(yard.record()).expect("the record");
  let named = |line: &str, kind: &str| {
    let rest = line.strip_prefix(&format!(r#"[["{kind}",""#))?;
    Some(rest.split('"').next().unwrap_or_default().to_owned())
  };
  let started: Vec<String> = record
    .lines()
    .filter(|line| line.contains(r#""provider"]"#))
    .filter_map(|line| named(line, "started"))
    .collect();
  let done: Vec<String> = record.lines().filter_map(|line| named(line, "done")).collect();
  let left: Vec<&String> = started.iter().filter(|id| !done.contains(id)).collect();
  assert_eq!(
    left.len(),
    2,
    "the second life asked again the reply that the first one left: {left:?}"
  );
  assert_eq!(left[0], left[1], "one reply, asked once in each life: {left:?}");
}

/// What a command did, or nothing when it still ran after this many seconds and was killed.
fn ran_within(mut command: Command, seconds: u64) -> Option<Output> {
  let piped = || Stdio::piped();
  let child =
    command.stdin(Stdio::null()).stdout(piped()).stderr(piped()).spawn().expect("furb starts");
  pipes::finished(child, Duration::from_secs(seconds))
}

#[test]
fn a_command_that_fills_both_pipes_is_read_to_its_end_within_the_bound() {
  let mut command = Command::new("sh");
  command.args(["-c", "head -c 300000 /dev/zero; head -c 300000 /dev/zero >&2"]);
  let output = ran_within(command, 30).expect("the command ends");
  assert_eq!((output.stdout.len(), output.stderr.len()), (300_000, 300_000));
}

#[test]
fn a_command_that_runs_past_the_bound_is_killed_and_gives_nothing() {
  let mut command = Command::new("sh");
  command.args(["-c", "exec sleep 30"]);
  let started = Instant::now();
  assert!(ran_within(command, 1).is_none());
  assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn a_record_is_printed_as_facts_each_after_the_number_of_its_line() {
  let yard = Yard::new("facts");
  fs::create_dir_all(yard.at.join(".furb")).expect("the folder of the project");
  let off = r#"{"extensions": {"catchup": false, "memory": false, "skills": false, "compaction": false, "ouroboros": false, "web": false, "mcp": false}}"#;
  fs::write(yard.at.join(".furb/config.json"), off).expect("a config of the project");
  assert_eq!(printed(yard.kept(&["run", "k = 3"]), ""), "None\n");
  let facts = |words: &[&str]| {
    let mut command = yard.furb(&["facts", "--record"]);
    command.arg(yard.record()).args(words);
    printed(command, "")
  };
  let all = facts(&[]);
  let lines = fs::read_to_string(yard.record()).expect("the record").lines().count();
  let numbers: Vec<String> =
    all.lines().map(|line| line.split(": ").next().unwrap_or_default().to_owned()).collect();
  let expected: Vec<String> = (1..=lines).map(|n| n.to_string()).collect();
  assert_eq!(numbers, expected, "one fact for each line of the record: {all}");
  assert!(all.starts_with("1: ['chain', 'chain1', 'operator', '', 'root', '']\n"), "{all}");
  let rung = facts(&["--act", "rung4"]);
  assert!(rung.ends_with(": ['rung', 'rung4', 'operator', 'chain1', 'k = 3', '', '']\n"), "{rung}");
  assert_eq!(rung.lines().count(), 1, "the facts about one act: {rung}");
  assert!(all.contains(&rung), "a fact keeps the number of its line: {all}");
  assert_eq!(facts(&["--chain", "chain1"]), all, "every fact of this record is on the root");
  assert_eq!(facts(&["--chain", "chain2"]), "", "no fact is on a chain the record does not hold");
  let mut missing = yard.furb(&["facts", "--record", "nope.jsonl"]);
  missing.current_dir(&yard.at);
  assert!(refused(missing, "").contains("No record at nope.jsonl."), "a record that is not there");
}
#[test]
fn a_life_runs_the_extensions_that_its_record_enables_and_those_that_the_configs_turn_on() {
  let yard = Yard::new("extensions");
  let skill = yard.at.join(".furb/skills/brew/SKILL.md");
  fs::create_dir_all(skill.parent().expect("a folder")).expect("the folder of a skill");
  fs::write(&skill, "---\nname: brew\ndescription: Make tea.\n---\n").expect("a skill");
  fs::write(yard.at.join("CLAUDE.md"), "Use two spaces.\n").expect("a memory file");
  let every = "catchup: catchup()\nmemory: await recall()\nskills: await listed()\ncompaction\nouroboros\nweb\nmcp: await mcp_index()\n";
  assert_eq!(printed(yard.furb(&["extensions"]), ""), every, "the official extensions are on");
  let path = printed(yard.kept(&["run", "close((await skill('brew')).path)"]), "");
  // The World writes a path under the home of the user with ~ in place of the home.
  let home = std::env::home_dir().filter(|home| home.parent().is_some());
  let shown = match home.as_deref().and_then(|home| skill.strip_prefix(home).ok()) {
    Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
    None => skill.display().to_string(),
  };
  assert_eq!(path, format!("{shown:?}\n").replace('"', "'"));
  let mut turns = yard.furb(&["turns", "--record"]);
  turns.arg(yard.record());
  let said = printed(turns, "");
  // The memory file comes after the reads of the folders that memory lists.
  assert!(
    said.contains("/CLAUDE.md'\nread") && said.contains("_text = 'Use two spaces.'"),
    "{said}"
  );
  fs::create_dir_all(yard.at.join(".furb")).expect("the folder of the project");
  fs::write(yard.at.join(".furb/config.json"), r#"{"extensions": {"memory": false}}"#)
    .expect("a config of the project");
  assert_eq!(
    printed(yard.furb(&["extensions"]), ""),
    "catchup: catchup()\nskills: await listed()\ncompaction\nouroboros\nweb\nmcp: await mcp_index()\n"
  );
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
  let nothing = ["prompt", "say nothing", "--to", "operator", "--shape", "None"];
  let output = ran(yard.furb(&nothing), "");
  assert_eq!(String::from_utf8_lossy(&output.stdout), "None\n", "a notice reads no line");
  assert_eq!(String::from_utf8_lossy(&output.stderr), "prompt1 tells: say nothing\n");
  let said =
    refused(yard.furb(&["prompt", "count", "--to", "operator", "--shape", "int"]), "many\n");
  assert!(
    said.ends_with("furb: Refused: \"many\" is no int: invalid digit found in string\n"),
    "{said}"
  );
  let said = refused(yard.furb(&["prompt", "count", "--to", "operator", "--shape", "int"]), "");
  assert!(
    said.ends_with("furb: Refused: the operator cannot be read: the input is over\n"),
    "{said}"
  );
  let pick = ["prompt", "pick one", "--to", "operator", "--shape", "Literal['a', 'b']"];
  let output = ran(yard.furb(&pick), "b\n");
  assert_eq!(
    String::from_utf8_lossy(&output.stdout),
    "'b'\n",
    "a Literal is answered by one of its values"
  );
  let said = refused(yard.furb(&pick), "c\n");
  assert!(said.ends_with("furb: Refused: \"c\" is none of Literal['a', 'b']\n"), "{said}");
  let said = refused(yard.furb(&["prompt", "a set", "--to", "operator", "--shape", "set"]), "");
  assert_eq!(
    said, "furb: Refused: the operator answers no set\n",
    "a shape the operator answers not is not shown"
  );
}

#[test]
fn a_prompt_of_the_operator_that_names_no_shape_wants_a_str() {
  let yard = Yard::new("no-shape");
  yard.words(&["close(None)", "close('done')"]);
  assert_eq!(printed(yard.furb(&["prompt", "work"]), ""), "'done'\n");
}

#[test]
fn a_model_answers_a_prompt_of_any_shape() {
  let yard = Yard::new("any-shape");
  yard.words(&["close(['a', 'b'])"]);
  assert_eq!(
    printed(yard.furb(&["prompt", "two words", "--shape", "list[str]"]), ""),
    "['a', 'b']\n"
  );
}

#[test]
fn a_word_that_a_model_wraps_in_a_fence_runs_less_the_fence_and_its_chain_is_told_once() {
  let yard = Yard::new("fence");
  yard.words(&["```python\nx = 1\nclose(x + 2)\n```"]);
  assert_eq!(printed(yard.kept(&["prompt", "three", "--shape", "int"]), ""), "3\n");
  assert_eq!(yard.answered(), 1);
  let record = fs::read_to_string(yard.record()).expect("the record");
  let told: Vec<&str> =
    record.lines().filter(|line| line.contains("wrapped its word in a fence")).collect();
  assert_eq!(
    told.len(),
    1,
    "{}",
    told.iter().map(|line| &line[..line.len().min(400)]).collect::<Vec<_>>().join("\n")
  );
  assert!(!record.contains("```"), "{record}");
}

#[test]
fn a_word_that_a_model_wraps_in_a_tool_call_runs_less_its_markup_and_the_next_turn_holds_none_of_it()
 {
  let yard = Yard::new("invoke");
  let call = "<invoke name=\"bash\">\n<parameter name=\"command\">ls</parameter>\n</invoke>";
  yard.words(&[call, "close(5)"]);
  assert_eq!(printed(yard.kept(&["prompt", "five", "--shape", "int"]), ""), "5\n");
  assert_eq!(yard.answered(), 2);
  let input = fs::read_to_string(yard.fake.join("in")).expect("the input of the fake");
  let next = input.lines().last().unwrap_or_default();
  // The refused word goes on the user side, so the turn holds the whole chain again: read only its last paragraphs.
  let tail = &next[next.rfind("wrapped its word").unwrap_or(0)..];
  assert!(tail.contains("refused") && next.contains("wrapped its word"), "{tail}");
  assert!(!tail.contains("invoke") && !tail.contains("parameter"), "{tail}");
}

#[test]
fn the_life_of_a_prompt_offers_the_model_of_the_actor_it_names() {
  for (name, roster) in [("to", &[][..]), ("to-named", &["--roster", "haiku"][..])] {
    let yard = Yard::new(name);
    yard.words(&["close('hi')"]);
    let words = [&["prompt", "hi", "--to", "sonnet/high", "--shape", "str"][..], roster].concat();
    assert_eq!(printed(yard.furb(&words), ""), "'hi'\n", "{name}");
  }
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
  assert!(
    said.starts_with(
      "[user] chain()\n[assistant] debug(t\"{standing()[4]=}\")\n[user] #rung1 debugged\n"
    ),
    "the setup of the root comes first, with the turn of its extensions: {said}"
  );
  assert!(said.contains("\n[assistant] read('"), "then the read of the doctrine: {said}");
  assert!(said.contains("\n\n#chain1\nchain1_label = 'root'\n"), "then the root itself: {said}");
  let paragraphs = "\n\n#prompt1\nprompt1_message = 'say a number'\nprompt1: Act[int] = Act('prompt1')\n\n#prompt1 closed by console\nprompt1_value = 7";
  assert!(said.contains(paragraphs), "the turns of a root as a model read them: {said}");
  assert!(said.contains(&format!("\n[assistant] {COUNT}\n")), "{said}");
}

#[test]
fn a_prompt_the_world_paused_ends_its_command_with_why_and_goes_on_when_it_is_taken_up() {
  let yard = Yard::new("paused");
  let counted = ["prompt", "count the lines", "--shape", "int"];
  // A missing program is a failure that passes, which the provider asks again for minutes: this one fails at once.
  let broken = yard.at.join("broken");
  program(&broken, "#!/bin/sh\necho 'claude is broken' >&2\nexit 1\n");
  let mut failing = yard.kept(&counted);
  failing.env("FURB_CLAUDE_BIN", &broken);
  let said = refused(failing, "");
  assert!(
    said.contains("furb: prompt1 is paused: claude-cli:opus/high answered nothing: "),
    "{said}"
  );
  assert!(said.contains("A wake from a client of the daemon makes it go on."), "{said}");
  yard.words(&[COUNT]);
  assert_eq!(printed(yard.kept(&counted), ""), "3\n", "the command wakes the prompt it takes up");
  assert_eq!(yard.answered(), 1);
}

#[test]
fn furb_hands_the_terminal_to_the_tui_with_the_words_it_takes() {
  let yard = Yard::new("tui");
  let tui = yard.at.join("tui");
  program(&tui, "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit 3\n");
  let mut command = yard.furb(&[
    "--demo",
    "--record",
    "r.jsonl",
    "--cwd",
    "there",
    "--model",
    "sonnet/high",
    "--roster",
    "n",
    "--",
    "more",
  ]);
  command.env("FURB_TUI", &tui);
  let output = ran(command, "");
  assert_eq!(output.status.code(), Some(3), "furb ends with the code of the TUI");
  let words = String::from_utf8_lossy(&output.stdout);
  let expected = "--demo\n--record\nr.jsonl\n--cwd\nthere\n--model\nclaude-cli:sonnet\n--effort\nhigh\n\
                  --roster\nn\nmore\n";
  assert_eq!(words, expected);
}

#[test]
fn a_prompt_to_a_model_of_a_local_server_is_answered_by_that_server() {
  let yard = Yard::new("local");
  let (port, asked) = served(COUNT);
  let mut command = yard.kept(&["prompt", "count", "--shape", "int", "--model", "local:m"]);
  command.env("FURB_LOCAL_URL", format!("http://127.0.0.1:{port}")).env("FURB_LOCAL_KEY", "k");
  assert_eq!(printed(command, ""), "3\n");
  let requests: Vec<String> = asked.try_iter().collect();
  let lines: Vec<&str> = requests.iter().filter_map(|one| one.lines().next()).collect();
  assert_eq!(lines, ["GET /v1/models HTTP/1.1", "POST /v1/chat/completions HTTP/1.1"]);
  let keyed = |one: &String| one.to_ascii_lowercase().contains("authorization: bearer k");
  assert!(requests.iter().all(keyed), "{requests:?}");
  let body: Value = serde_json::from_str(requests[1].split("\r\n\r\n").nth(1).unwrap_or_default())
    .expect("a JSON body");
  assert_eq!((&body["model"], &body["max_tokens"]), (&json!("m"), &json!(250)));
  assert_eq!(yard.answered(), 0, "no claude is asked");
}

/// The first line that a program wrote on its stdout, or a failure after a minute.
fn said_first(child: &mut Child) -> String {
  let out = child.stdout.take().expect("the program has a stdout");
  let (said, heard) = mpsc::channel();
  thread::spawn(move || {
    let mut line = String::new();
    let read = BufReader::new(out).read_line(&mut line).map(|_| line);
    said.send(read).ok();
  });
  let line = heard.recv_timeout(Duration::from_secs(60)).expect("the program says a line in time");
  line.expect("the stdout of the program reads")
}

/// The end of a program, which a minute bounds: past it, the program is killed and the test fails.
fn ended(mut child: Child) -> std::process::ExitStatus {
  let pid = child.id();
  let (said, heard) = mpsc::channel();
  thread::spawn(move || {
    said.send(child.wait()).ok();
  });
  let Ok(status) = heard.recv_timeout(Duration::from_secs(60)) else {
    Command::new("kill").arg("-9").arg(pid.to_string()).status().ok();
    panic!("the program {pid} did not end in time")
  };
  status.expect("the program is awaited")
}

#[test]
fn furb_host_serves_one_daemon_on_the_machine_and_an_upgrade_takes_its_place() {
  let yard = Yard::new("host");
  // The cache of the yard is too deep for the path of a socket, which macOS bounds at 104 bytes.
  let cache = std::env::temp_dir().join(format!("fc{}", std::process::id()));
  let host = |words: &[&str]| {
    let mut command = yard.furb(words);
    command.env("XDG_CACHE_HOME", &cache);
    command
  };
  let refused = |output: Output| {
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(!output.status.success() && said.contains("a daemon already listens"), "{said}");
  };
  let mut first = host(&["host"]).stdout(Stdio::piped()).spawn().expect("furb host runs");
  let said = said_first(&mut first);
  assert!(said.starts_with("furb host listens in "), "{said:?}");
  refused(host(&["host"]).output().expect("furb host runs"));
  let mut upgraded =
    host(&["host", "upgrade"]).stdout(Stdio::null()).spawn().expect("the upgrade runs");
  assert!(ended(first).success(), "the daemon that ran gives its place and ends");
  refused(host(&["host"]).output().expect("furb host runs"));
  upgraded.kill().expect("the new daemon ends");
  ended(upgraded);
  fs::remove_dir_all(&cache).ok();
}
