//! The claude command line as a model of rig, driven against a command line that answers from a script, which
//! `fake-claude.sh` says.

use std::{
  fs,
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::OnceLock,
  time::Duration,
};

use futures::{
  StreamExt,
  future::{Either, select},
};
use rig_core::{
  completion::{CompletionModel, CompletionRequest},
  message::{AssistantContent, Message, Reasoning, Text, UserContent},
  streaming::StreamedAssistantContent,
};
use serde_json::{Value, json};

use super::{Claude, Completion};
use crate::world::provider::runtime;

/// The fake command line, which the tests of the command line of furb run too.
const FAKE: &str = include_str!("fake-claude.sh");

/// A yard of the test that holds the fake command line and what it wrote.
pub(crate) struct Yard {
  pub at: PathBuf,
}

impl Yard {
  pub fn new(name: &str) -> Yard {
    let at = std::env::temp_dir().join(format!("furb-claude-{name}"));
    let _ = fs::remove_dir_all(&at);
    fs::create_dir_all(&at).expect("a yard of the test");
    fs::hard_link(ran(), at.join("claude")).expect("the fake is linked");
    Yard { at }
  }

  /// The fake as the command line, with a stall of these milliseconds.
  pub fn claude(&self, stall: u64) -> Claude {
    Claude::with(Some(self.at.join("claude")), Some(Duration::from_millis(stall)))
  }

  /// The id of each process the fake was, in the order they started.
  pub fn pids(&self) -> Vec<String> {
    let pids = fs::read_to_string(self.at.join("pids")).unwrap_or_default();
    pids.lines().map(str::to_owned).collect()
  }

  /// The words a process was started with.
  pub fn args(&self, pid: &str) -> Vec<String> {
    let args = fs::read_to_string(self.at.join(format!("args.{pid}"))).unwrap_or_default();
    args.split_terminator('\0').map(str::to_owned).collect()
  }

  /// Each line of input the fake read.
  pub fn heard(&self) -> Vec<String> {
    let heard = fs::read_to_string(self.at.join("in")).unwrap_or_default();
    heard.lines().map(str::to_owned).collect()
  }
}

/// The fake, written once for all the tests of this process, after it ran once. The system examines a new program
/// the first time it runs it. On macOS this takes more than a tenth of a second when the machine is idle, and much
/// more when many tests start new programs at the same time. The result holds for the file, so each yard links to
/// this file, and no clock of a test includes that examination.
fn ran() -> &'static Path {
  static RAN: OnceLock<PathBuf> = OnceLock::new();
  RAN.get_or_init(|| {
    let at = std::env::temp_dir().join("furb-claude-fake");
    let _ = fs::remove_dir_all(&at);
    fs::create_dir_all(&at).expect("the folder of the fake");
    let bin = at.join("claude");
    fs::write(&bin, FAKE).expect("the fake is written");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("the fake runs");
    // A new program is busy for a moment while the fork of another thread holds it open to write. The crate starts
    // its programs in the same way.
    let mut tries = 0;
    let status = loop {
      match Command::new(&bin).stdin(Stdio::null()).stdout(Stdio::null()).status() {
        Err(no) if no.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 50 => tries += 1,
        got => break got,
      }
      std::thread::sleep(Duration::from_millis(10));
    };
    assert!(status.expect("the fake starts").success(), "the fake ran once");
    bin
  })
}

/// The word after a flag among the words of a process.
pub(crate) fn after(args: &[String], flag: &str) -> Option<String> {
  args.iter().position(|one| one == flag).and_then(|at| args.get(at + 1).cloned())
}

/// A message of the user.
fn user(text: &str) -> Message {
  Message::User { content: vec![UserContent::Text(Text::new(text))] }
}

/// A request of these messages, with a system prompt and settings.
fn request(system: &str, messages: &[Message], settings: Value) -> CompletionRequest {
  CompletionRequest {
    model: None,
    preamble: Some(system.to_owned()),
    chat_history: messages.to_vec(),
    documents: Vec::new(),
    tools: Vec::new(),
    temperature: None,
    max_tokens: None,
    tool_choice: None,
    additional_params: Some(settings),
    output_schema: None,
    record_telemetry_content: false,
  }
}

/// One whole turn of a model.
fn asked(
  model: &Completion,
  messages: &[Message],
  settings: Value,
) -> Result<rig_core::completion::CompletionResponse, rig_core::completion::CompletionError> {
  runtime().block_on(model.completion(request("ENGINE ONLY", messages, settings)))
}

#[test]
fn a_turn_starts_the_command_line_for_a_pure_completion_and_says_its_blocks_and_its_usage() {
  let yard = Yard::new("flags");
  let model = yard.claude(10_000).completion_model("sonnet");
  let got =
    asked(&model, &[user("first")], json!({"effort": "xhigh", "session": "chain"})).unwrap();
  assert_eq!(got.choice, [AssistantContent::text("close(\"reply 1\")")]);
  let spent = got.usage;
  let counts = (spent.input_tokens, spent.output_tokens, spent.cached_input_tokens);
  assert_eq!(counts, (10, 5, 20));
  assert_eq!((spent.cache_creation_input_tokens, spent.total_tokens), (3, 38));
  assert_eq!(got.raw, json!({"cost": 0.01}));
  let pids = yard.pids();
  assert_eq!(pids.len(), 1);
  let args = yard.args(&pids[0]);
  assert_eq!(args[..5], ["-p", "--input-format", "stream-json", "--output-format", "stream-json"]);
  for flag in
    ["--verbose", "--include-partial-messages", "--strict-mcp-config", "--disable-slash-commands"]
  {
    assert!(args.iter().any(|one| one == flag), "{flag} in {args:?}");
  }
  assert_eq!(after(&args, "--model").as_deref(), Some("sonnet"));
  assert_eq!(after(&args, "--system-prompt").as_deref(), Some("ENGINE ONLY"));
  assert_eq!(after(&args, "--effort").as_deref(), Some("xhigh"));
  assert_eq!(after(&args, "--tools").as_deref(), Some(""));
  assert_eq!(after(&args, "--setting-sources").as_deref(), Some(""));
  assert_eq!(after(&args, "--session-id").map(|one| one.len()), Some(36));
  let environment = fs::read_to_string(format!("/proc/{}/environ", pids[0])).unwrap_or_default();
  if !environment.is_empty() {
    assert!(environment.contains("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1"));
  }
}

#[test]
fn the_next_turn_of_a_conversation_writes_to_its_process_only_what_it_has_not_heard() {
  let yard = Yard::new("next");
  let model = yard.claude(10_000).completion_model("sonnet");
  let settings = json!({"session": "chain"});
  let mut messages = vec![user("first")];
  let one = asked(&model, &messages, settings.clone()).unwrap();
  messages.push(Message::Assistant { id: None, content: one.choice });
  messages.push(user("second"));
  let two = asked(&model, &messages, settings.clone()).unwrap();
  assert_eq!(two.choice, [AssistantContent::text("close(\"reply 2\")")]);
  let cost = two.raw["cost"].as_f64().unwrap_or_default();
  assert!((cost - 0.01).abs() < 1e-9, "each turn pays its own part: {cost}");
  assert_eq!(yard.pids().len(), 1, "the conversation kept its process");
  let heard = yard.heard();
  assert_eq!(heard.len(), 2);
  assert!(heard[1].contains("second") && !heard[1].contains("first"), "{}", heard[1]);
  // The same request again is a turn that failed and is asked again: its last message goes once more.
  asked(&model, &messages, settings.clone()).unwrap();
  assert!(yard.heard()[2].contains("second") && !yard.heard()[2].contains("first"));
  // A request that continues nothing the conversation heard begins a new one.
  asked(&model, &[user("other")], settings).unwrap();
  let pids = yard.pids();
  let session = |pid: &str| after(&yard.args(pid), "--session-id");
  assert_eq!(pids.len(), 2);
  assert_ne!(session(&pids[0]), session(&pids[1]));
}

#[test]
fn a_turn_asked_again_after_it_failed_resumes_its_conversation_once_and_then_begins_anew() {
  let yard = Yard::new("again");
  let model = yard.claude(10_000).completion_model("sonnet");
  for _ in 0..3 {
    asked(&model, &[user("FAIL")], json!({"session": "chain"})).unwrap_err();
  }
  let pids = yard.pids();
  let args: Vec<_> = pids.iter().map(|pid| yard.args(pid)).collect();
  let first = after(&args[0], "--session-id");
  assert_eq!(after(&args[1], "--resume"), first, "the second try resumes the conversation");
  assert!(after(&args[2], "--session-id").is_some_and(|one| Some(&one) != first.as_ref()));
}

#[test]
fn a_turn_given_again_with_its_last_message_grown_after_it_failed_sends_what_it_gained_on_its_conversation()
 {
  let yard = Yard::new("grown");
  let model = yard.claude(10_000).completion_model("sonnet");
  let settings = json!({"session": "chain"});
  let one = asked(&model, &[user("first")], settings.clone()).unwrap();
  let answer = Message::Assistant { id: None, content: one.choice };
  asked(&model, &[user("first"), answer.clone(), user("second FAIL")], settings.clone())
    .unwrap_err();
  let grown = [user("first"), answer, user("second FAIL\n\n# third")];
  let got = asked(&model, &grown, settings).unwrap();
  assert_eq!(got.choice, [AssistantContent::text("close(\"reply 1\")")]);
  let pids = yard.pids();
  assert_eq!(pids.len(), 2);
  let first = after(&yard.args(&pids[0]), "--session-id");
  assert_eq!(after(&yard.args(&pids[1]), "--resume"), first, "the same conversation goes on");
  let line: Value = serde_json::from_str(yard.heard().last().unwrap()).unwrap();
  assert_eq!(line["message"]["content"], json!([{"type": "text", "text": "# third"}]));
}

#[test]
fn a_message_that_the_command_line_did_not_hear_goes_whole_at_the_next_turn() {
  let yard = Yard::new("unheard");
  let model = yard.claude(10_000).completion_model("sonnet");
  let settings = json!({"session": "chain"});
  let one = asked(&model, &[user("first")], settings.clone()).unwrap();
  let answer = Message::Assistant { id: None, content: one.choice };
  let turn =
    |last: &str| asked(&model, &[user("first"), answer.clone(), user(last)], settings.clone());
  turn("second FAIL").unwrap_err();
  let bin = yard.at.join("claude");
  fs::remove_file(&bin).unwrap();
  turn("second FAIL\n\nthird").unwrap_err();
  fs::write(&bin, FAKE).unwrap();
  fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
  turn("second FAIL\n\nthird\n\nfourth").unwrap();
  let pids = yard.pids();
  assert_eq!(pids.len(), 2);
  let first = after(&yard.args(&pids[0]), "--session-id");
  assert_eq!(after(&yard.args(&pids[1]), "--resume"), first, "the same conversation goes on");
  let line: Value = serde_json::from_str(yard.heard().last().unwrap()).unwrap();
  assert_eq!(line["message"]["content"], json!([{"type": "text", "text": "third\n\nfourth"}]));
}

#[test]
fn a_turn_given_again_whole_after_its_grown_message_did_not_go_sends_only_what_the_message_gained()
{
  let yard = Yard::new("unsent");
  let model = yard.claude(10_000).completion_model("sonnet");
  let settings = json!({"session": "chain"});
  let one = asked(&model, &[user("first")], settings.clone()).unwrap();
  let answer = Message::Assistant { id: None, content: one.choice };
  let turn =
    |last: &str| asked(&model, &[user("first"), answer.clone(), user(last)], settings.clone());
  turn("second FAIL").unwrap_err();
  let bin = yard.at.join("claude");
  fs::remove_file(&bin).unwrap();
  turn("second FAIL\n\nthird").unwrap_err();
  fs::write(&bin, FAKE).unwrap();
  fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
  turn("second FAIL\n\nthird").unwrap();
  let pids = yard.pids();
  assert_eq!(pids.len(), 2);
  let first = after(&yard.args(&pids[0]), "--session-id");
  assert_eq!(after(&yard.args(&pids[1]), "--resume"), first, "the same conversation goes on");
  let line: Value = serde_json::from_str(yard.heard().last().unwrap()).unwrap();
  assert_eq!(line["message"]["content"], json!([{"type": "text", "text": "third"}]));
}

#[test]
fn a_turn_given_again_grown_that_comes_to_no_reply_again_begins_a_new_conversation_at_the_next_turn()
 {
  let yard = Yard::new("regrown");
  let model = yard.claude(10_000).completion_model("sonnet");
  let settings = json!({"session": "chain"});
  let one = asked(&model, &[user("first")], settings.clone()).unwrap();
  let answer = Message::Assistant { id: None, content: one.choice };
  let turn =
    |last: &str| asked(&model, &[user("first"), answer.clone(), user(last)], settings.clone());
  turn("second FAIL").unwrap_err();
  turn("second FAIL\n\nthird FAIL").unwrap_err();
  turn("second FAIL\n\nthird FAIL\n\nfourth").unwrap_err();
  let pids = yard.pids();
  assert_eq!(pids.len(), 3);
  let first = after(&yard.args(&pids[0]), "--session-id");
  assert_eq!(after(&yard.args(&pids[1]), "--resume"), first, "the grown message goes once");
  let anew = after(&yard.args(&pids[2]), "--session-id");
  assert!(anew.is_some() && anew != first, "the next time, a new conversation begins");
  let heard = yard.heard();
  assert!(heard[2].contains("third FAIL") && !heard[2].contains("second"), "{}", heard[2]);
  assert!(heard[3].contains("first") && heard[3].contains("fourth"), "{}", heard[3]);
}

#[test]
fn past_the_warm_processes_the_least_used_one_ends_and_past_the_held_conversations_the_least_used_one_goes()
 {
  let yard = Yard::new("pool");
  let model = yard.claude(10_000).completion_model("sonnet");
  let turn = |at: usize, messages: &[Message]| {
    asked(&model, messages, json!({"session": format!("s{at}")})).expect("a turn")
  };
  let reply = turn(0, &[user("first")]);
  (1..=8).for_each(|at| drop(turn(at, &[user("first")])));
  let first = yard.pids().remove(0);
  assert!(gone(&first), "past eight warm processes the least used one ends");
  (9..=64).for_each(|at| drop(turn(at, &[user("first")])));
  let answer = Message::Assistant { id: None, content: reply.choice };
  turn(0, &[user("first"), answer, user("second")]);
  let again = yard.args(yard.pids().last().expect("a process"));
  let id = after(&yard.args(&first), "--session-id");
  assert_ne!(after(&again, "--resume"), id, "past 64 held conversations the least used one went");
}

#[test]
fn a_new_effort_ends_the_process_of_the_conversation_and_the_next_process_resumes_it_at_that_effort()
 {
  let yard = Yard::new("efforts");
  let model = yard.claude(10_000).completion_model("sonnet");
  let turn = |messages: &[Message], effort: &str| {
    asked(&model, messages, json!({"effort": effort, "session": "chain"})).unwrap()
  };
  let mut messages = vec![user("first")];
  let one = turn(&messages, "high");
  messages.extend([Message::Assistant { id: None, content: one.choice }, user("second")]);
  let two = turn(&messages, "low");
  messages.extend([Message::Assistant { id: None, content: two.choice }, user("third")]);
  turn(&messages, "high");
  let pids = yard.pids();
  assert_eq!(pids.len(), 3);
  let first = after(&yard.args(&pids[0]), "--session-id");
  for (pid, effort) in pids[1..].iter().zip(["low", "high"]) {
    let args = yard.args(pid);
    assert_eq!(after(&args, "--resume"), first, "the same conversation goes on");
    assert!(!args.iter().any(|one| one == "--fork-session"), "{args:?}");
    assert_eq!(after(&args, "--effort").as_deref(), Some(effort));
  }
  assert!(gone(&pids[0]) && gone(&pids[1]), "one process of the chain stands at a time");
  let heard = yard.heard();
  assert!(heard[2].contains("third") && !heard[2].contains("second"), "{}", heard[2]);
}

#[test]
fn an_effort_that_the_command_line_does_not_take_is_refused_before_any_process_starts() {
  let yard = Yard::new("effort");
  let model = yard.claude(10_000).completion_model("sonnet");
  let no = asked(&model, &[user("first")], json!({"effort": "huge"})).unwrap_err();
  let why = "sonnet spends one of low, medium, high, xhigh, max on a turn, never huge.";
  assert!(no.to_string().contains(why), "{no}");
  assert!(yard.pids().is_empty());
}

#[test]
fn a_result_is_the_text_of_a_reply_that_streamed_no_block_and_fails_the_turn_when_it_says_an_error()
{
  let yard = Yard::new("result");
  let model = yard.claude(10_000).completion_model("sonnet");
  let got = asked(&model, &[user("BARE")], json!({})).unwrap();
  assert_eq!(got.choice, [AssistantContent::text("close(1)")]);
  let no = asked(&model, &[user("ERROR")], json!({})).unwrap_err();
  assert!(no.to_string().contains("the model is overloaded"), "{no}");
}

#[test]
fn a_line_longer_than_a_reader_takes_fails_its_turn() {
  let yard = Yard::new("long");
  let model = yard.claude(10_000).completion_model("sonnet");
  let no = asked(&model, &[user("LONG")], json!({})).unwrap_err();
  assert!(no.to_string().contains("Claude wrote a line over 33554432 bytes."), "{no}");
}

#[test]
fn a_conversation_that_grows_out_of_another_forks_it() {
  let yard = Yard::new("fork");
  let model = yard.claude(10_000).completion_model("sonnet");
  let mut messages = vec![user("first")];
  let one = asked(&model, &messages, json!({"session": "parent"})).unwrap();
  messages.push(Message::Assistant { id: None, content: one.choice });
  messages.push(user("second"));
  asked(&model, &messages, json!({"session": "child"})).unwrap();
  let pids = yard.pids();
  let parent = after(&yard.args(&pids[0]), "--session-id").unwrap_or_default();
  let args = yard.args(&pids[1]);
  assert_eq!(after(&args, "--resume"), Some(parent));
  assert!(args.iter().any(|one| one == "--fork-session"));
  assert!(!yard.heard()[1].contains("first"), "the fork holds what its parent heard");
}

#[test]
fn a_thought_streams_as_it_comes_and_settles_once_at_its_place() {
  let yard = Yard::new("thought");
  let model = yard.claude(10_000).completion_model("sonnet");
  let (texts, choice) = runtime()
    .block_on(async {
      let mut stream = model.stream(request("ENGINE ONLY", &[user("THINK")], json!({}))).await?;
      let mut texts = Vec::new();
      while let Some(one) = stream.next().await {
        if let StreamedAssistantContent::Text(text) = one? {
          texts.push(text.text);
        }
      }
      Ok::<_, rig_core::completion::CompletionError>((texts, stream.choice))
    })
    .unwrap();
  assert_eq!(texts, ["close(", "\"reply 1\")"]);
  let thought = Reasoning::new_with_signature("hmm", Some("sig".to_owned()));
  let expected =
    [AssistantContent::Reasoning(thought), AssistantContent::text("close(\"reply 1\")")];
  assert_eq!(choice, expected);
  let whole = asked(&model, &[user("THINK")], json!({})).unwrap();
  assert_eq!(whole.choice, expected);
}

#[test]
fn a_process_that_exits_fails_its_turn_with_the_tail_of_its_stderr() {
  let yard = Yard::new("exits");
  let model = yard.claude(10_000).completion_model("sonnet");
  let no = asked(&model, &[user("FAIL")], json!({})).unwrap_err();
  assert!(no.to_string().contains("Claude exited (2): deliberate failure"), "{no}");
}

#[test]
fn a_turn_that_makes_no_progress_fails_at_its_stall_with_the_last_odd_line_and_ends_its_process() {
  let yard = Yard::new("stall");
  let model = yard.claude(300).completion_model("sonnet");
  let no = asked(&model, &[user("ODD")], json!({"session": "stall"})).unwrap_err();
  let why = "Claude made no progress for 0.3s. The last line it wrote was no line of json";
  assert!(no.to_string().contains(why), "{no}");
  let pid = yard.pids().remove(0);
  assert!(gone(&pid), "the process of the turn ended");
}

#[test]
fn a_turn_that_is_dropped_ends_its_process() {
  let yard = Yard::new("dropped");
  let model = yard.claude(10_000).completion_model("sonnet");
  let turn = model.completion(request("ENGINE ONLY", &[user("WAIT")], json!({})));
  // The turn drops when the fake heard its line, after it wrote its id, however long it took to start.
  let heard = async {
    while !yard.heard().iter().any(|one| one.contains("WAIT")) {
      tokio::time::sleep(Duration::from_millis(10)).await;
    }
  };
  let raced = runtime().block_on(async {
    tokio::time::timeout(Duration::from_secs(10), select(Box::pin(turn), Box::pin(heard))).await
  });
  let Ok(Either::Right(((), turn))) = raced else {
    panic!("the turn was still in flight when the fake heard its line")
  };
  drop(turn);
  let pid = yard.pids().remove(0);
  assert!(gone(&pid), "the process of the dropped turn ended");
}

/// Whether the process of an id is gone, within a time, as a process ends a moment after it is killed. A process
/// that ended and that nothing reaped yet is gone too.
pub(crate) fn gone(pid: &str) -> bool {
  let Ok(pid) = pid.parse::<libc::pid_t>() else { return false };
  for _ in 0..100 {
    // SAFETY: a signal of zero sends nothing, and says whether the process is there.
    let there = unsafe { libc::kill(pid, 0) } == 0;
    let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    if !there || status.contains("State:\tZ") {
      return true;
    }
    std::thread::sleep(Duration::from_millis(20));
  }
  false
}
