//! The claude command line as a model of rig, driven against a command line that answers from a script, which
//! `fake-claude.sh` says.

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use futures::StreamExt;
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
    let bin = at.join("claude");
    fs::write(&bin, FAKE).expect("the fake is written");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("the fake runs");
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

/// The word after a flag among the words of a process.
fn after(args: &[String], flag: &str) -> Option<String> {
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
  asked(&model, &messages, settings).unwrap();
  assert!(yard.heard()[2].contains("second") && !yard.heard()[2].contains("first"));
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
fn a_turn_that_makes_no_progress_fails_at_its_stall_and_ends_its_process() {
  let yard = Yard::new("stall");
  let model = yard.claude(300).completion_model("sonnet");
  let no = asked(&model, &[user("WAIT")], json!({"session": "stall"})).unwrap_err();
  assert!(no.to_string().contains("Claude made no progress for 0.3s."), "{no}");
  let pid = yard.pids().remove(0);
  assert!(gone(&pid), "the process of the turn ended");
}

#[test]
fn a_turn_that_is_dropped_ends_its_process() {
  let yard = Yard::new("dropped");
  let model = yard.claude(10_000).completion_model("sonnet");
  let turn = model.completion(request("ENGINE ONLY", &[user("WAIT")], json!({})));
  let waited =
    runtime().block_on(async { tokio::time::timeout(Duration::from_millis(300), turn).await });
  assert!(waited.is_err(), "the turn was still in flight");
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
