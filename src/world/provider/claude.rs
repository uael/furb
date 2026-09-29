//! The claude command line as a completion model of rig: one process of `claude -p` holds one conversation and
//! stays up between its turns, and each turn writes to it only what it has not heard.
//!
//! A conversation is keyed by its model, its system prompt and the conversation the request names, which the provider
//! names as a chain of its life, so a chain holds one conversation and at most one process, whatever the effort. A
//! process spends the effort it started at, so a new effort ends it, and the next process resumes the same
//! conversation. A process that is gone leaves its conversation on the disk under its id, so a later turn resumes
//! it, and a new conversation that grows out of one the command line holds forks it. A subscription pays for the
//! turns, and no key is read.

use std::{
  collections::HashMap,
  env,
  path::{Path, PathBuf},
  process::Stdio,
  sync::{Arc, Mutex, Once, Weak},
  time::{Duration, Instant},
};

use rig_core::{
  completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse, FinishReason, Usage,
  },
  message::{AssistantContent, DocumentSourceKind, Message, MimeType, Reasoning, UserContent},
  streaming::{RawStreamingChoice, StreamingCompletionResponse},
};
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
  process::{Child, ChildStdin, Command},
  sync::mpsc,
};

use futures::StreamExt;

use super::{Model, ended, uuid};
use crate::ear::reactor;

/// The name of the command line as a provider: each response of it says this name, and each of its models is named
/// by it and the alias of its family.
pub const CLAUDE: &str = "claude-cli";

/// The efforts the command line spends on a turn, from least to most.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// The models of the command line, by the alias of their family, and the window of each, the one a life stands on
/// when its host names none first.
const FAMILY: [(&str, u64); 4] =
  [("opus", 1_000_000), ("sonnet", 1_000_000), ("haiku", 200_000), ("fable", 1_000_000)];

/// How many conversations keep a process at most.
const WARM: usize = 8;

/// How many conversations the command line holds at most.
const HELD: usize = 64;

/// How long a conversation with no turn keeps its process.
const IDLE: Duration = Duration::from_secs(300);

/// How long a turn may go with no progress, unless `FURB_CLAUDE_STALL` gives other seconds.
const STALL: Duration = Duration::from_secs(900);

/// The longest line of the stream that the command line may write.
const LINE: usize = 32 * 1024 * 1024;

/// How many characters of what a process wrote on its stderr a failure keeps.
const TAIL: usize = 2000;

/// The error of the system that says a program is open to be written, which is ETXTBSY.
const BUSY: i32 = 26;

/// The claude command line: its program, how long a turn may go with no progress, and the conversations it holds.
///
/// It is cheap to clone, and every clone holds the same conversations. When the last clone goes, every process it
/// started ends.
#[derive(Clone)]
pub struct Claude {
  held: Arc<Held>,
}

struct Held {
  bin: PathBuf,
  stall: Duration,
  conversations: Mutex<HashMap<String, Arc<tokio::sync::Mutex<Conversation>>>>,
  /// The sweep of the processes that sat idle, begun at the first turn.
  swept: Once,
}

impl Claude {
  /// The command line of [`Claude::with`] and no program, when `FURB_CLAUDE_BIN` names one or this machine holds one.
  pub fn found() -> Option<Claude> {
    located().map(|bin| Claude::with(Some(bin), None))
  }

  /// The command line at this program, and how long a turn may go with no progress. With no program, it is the one
  /// that `FURB_CLAUDE_BIN` names, or the one found on this machine; with no stall, `FURB_CLAUDE_STALL` gives it in
  /// seconds, or it is 900 seconds.
  pub fn with(bin: Option<PathBuf>, stall: Option<Duration>) -> Claude {
    let bin = bin.or_else(located).unwrap_or_else(|| PROGRAM.into());
    let stall = stall.unwrap_or_else(|| {
      let seconds = env::var("FURB_CLAUDE_STALL").ok().and_then(|one| one.parse::<f64>().ok());
      seconds.and_then(|one| Duration::try_from_secs_f64(one).ok()).unwrap_or(STALL)
    });
    let held = Arc::new(Held { bin, stall, conversations: Mutex::default(), swept: Once::new() });
    Claude { held }
  }

  /// One model of the command line, by its name, as a completion model of rig.
  pub fn completion_model(&self, name: impl Into<String>) -> Completion {
    Completion { claude: self.clone(), name: name.into() }
  }

  /// The models of the command line as the provider offers them, each named `claude-cli:` and its alias: each with
  /// its window, every effort, and the images of a turn, and the conversation of a reply under the setting
  /// `session`. The command line counts what it read of the cache apart from the rest of what it read.
  pub fn models(&self) -> Vec<Model> {
    FAMILY
      .iter()
      .map(|(alias, window)| {
        let named = format!("{CLAUDE}:{alias}");
        let mut model =
          Model::new(named, *window, self.completion_model(*alias)).conversation("session");
        model.apart = true;
        EFFORTS
          .iter()
          .fold(model.images(), |model, effort| model.effort(*effort, json!({"effort": effort})))
      })
      .collect()
  }
}

/// One model of the claude command line, as a completion model of rig.
///
/// The request names its conversation under the setting `session`, and its effort under `effort`. It takes no tool
/// and no document, since furb asks for completions alone.
#[derive(Clone)]
pub struct Completion {
  claude: Claude,
  name: String,
}

/// Where the parts of a streamed turn go as the command line writes them.
type Deltas = mpsc::UnboundedSender<Result<RawStreamingChoice, CompletionError>>;

impl Completion {
  /// The command line, whose processes that sat idle end at the next sweep, which a turn does and a sweep of its own
  /// does too while no turn comes; that sweep begins at the first turn, and ends when the last clone goes.
  fn held(&self) -> Arc<Held> {
    let held = Arc::clone(&self.claude.held);
    held.swept.call_once(|| {
      let weak = Arc::downgrade(&held);
      reactor().spawn(async move {
        let mut ticks = tokio::time::interval(IDLE);
        ticks.tick().await;
        loop {
          ticks.tick().await;
          let Some(held) = Weak::upgrade(&weak) else { return };
          held.sweep();
        }
      });
    });
    held
  }
}

impl CompletionModel for Completion {
  async fn completion(
    &self,
    request: CompletionRequest,
  ) -> Result<CompletionResponse, CompletionError> {
    self.held().turn(&self.name, request, None).await
  }

  async fn stream(
    &self,
    request: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let (held, name) = (self.held(), self.name.clone());
    let (deltas, mut heard) = mpsc::unbounded_channel();
    let turn = async move {
      let got = held.turn(&name, request, Some(&deltas)).await;
      let _ =
        deltas.send(got.map(|response| RawStreamingChoice::FinalResponse(ended(CLAUDE, response))));
      None
    };
    // The turn goes with the stream: it runs as the stream is read, and a stream that is dropped ends it.
    let told = futures::stream::poll_fn(move |cx| heard.poll_recv(cx));
    let stream =
      futures::stream::select(told, futures::stream::once(turn).filter_map(async |one| one));
    Ok(StreamingCompletionResponse::stream(CLAUDE, Box::pin(stream)))
  }
}

impl Held {
  /// One turn of a model: the conversation it continues, which answers one turn at a time, and what it came to.
  async fn turn(
    &self,
    name: &str,
    request: CompletionRequest,
    deltas: Option<&Deltas>,
  ) -> Result<CompletionResponse, CompletionError> {
    let name = request.model.clone().unwrap_or_else(|| name.to_owned());
    if !request.tools.is_empty() || !request.documents.is_empty() {
      return Err(failed(format!(
        "{name} answers a completion alone, with no tool and no document."
      )));
    }
    let settings = request.additional_params.unwrap_or_default();
    let effort = settings.get("effort").and_then(Value::as_str).map(str::to_owned);
    if let Some(effort) = effort.as_deref().filter(|one| !EFFORTS.contains(one)) {
      let efforts = EFFORTS.join(", ");
      return Err(failed(format!("{name} spends one of {efforts} on a turn, never {effort}.")));
    }
    let session = settings.get("session").and_then(Value::as_str).map_or_else(uuid, str::to_owned);
    let system = request.preamble.unwrap_or_default();
    let messages: Vec<Message> = request
      .chat_history
      .into_iter()
      .filter(|one| !matches!(one, Message::System { .. }))
      .collect();
    let key = json!([name, system, session]).to_string();
    let conversation = self.conversation(&key, &name, &system, &messages);
    let got =
      conversation.lock().await.turn(&messages, effort, &self.bin, self.stall, deltas).await;
    self.sweep();
    got
  }

  /// The conversation of a key: the one the command line holds, or a new one, which forks the idle conversation it
  /// grows out of when there is one, so it takes all that conversation holds.
  fn conversation(
    &self,
    key: &str,
    name: &str,
    system: &str,
    messages: &[Message],
  ) -> Arc<tokio::sync::Mutex<Conversation>> {
    let mut held = self.conversations.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(one) = held.get(key) {
      return Arc::clone(one);
    }
    let incoming: Vec<String> = messages.iter().map(canonical).collect();
    let mut made = Conversation::new(name, system);
    let donor = held
      .values()
      .filter_map(|one| one.try_lock().ok())
      .filter(|one| {
        let chain = one.chain.len();
        chain > 0
          && one.name == name
          && one.system == system
          && chain < incoming.len()
          && incoming[..chain] == one.chain[..]
          && messages[chain..].iter().all(|one| !matches!(one, Message::Assistant { .. }))
      })
      .max_by_key(|one| one.chain.len())
      .map(|one| (one.id.clone(), one.chain.clone()));
    if let Some((parent, chain)) = donor {
      made.parent = Some(parent);
      made.chain = chain;
    }
    let made = Arc::new(tokio::sync::Mutex::new(made));
    held.insert(key.to_owned(), Arc::clone(&made));
    made
  }

  /// What no turn needs goes: the process of a conversation that sat idle too long, or that stands past the warm
  /// ones, the least used first, and a conversation with no process past the held ones.
  fn sweep(&self) {
    let mut held = self.conversations.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut idle: Vec<_> = held
      .iter()
      .filter_map(|(key, one)| one.try_lock().ok().map(|one| (key.clone(), one)))
      .collect();
    idle.sort_by_key(|(_, one)| one.used);
    let mut warm = held.len() - idle.len();
    warm += idle.iter().filter(|(_, one)| one.process.is_some()).count();
    let mut gone = Vec::new();
    for (key, one) in &mut idle {
      if one.process.is_some() && (warm > WARM || one.used.elapsed() > IDLE) {
        one.process = None;
        warm -= 1;
      }
      if one.process.is_none() && held.len() - gone.len() > HELD {
        gone.push(key.clone());
      }
    }
    drop(idle);
    for key in gone {
      held.remove(&key);
    }
  }
}

/// One conversation of the command line: what it is fixed at, its id, where it stands, and its process.
struct Conversation {
  name: String,
  system: String,
  /// The id of the conversation, which the command line keeps it under.
  id: String,
  /// The conversation that the first process of this one forks.
  parent: Option<String>,
  /// Whether a process of this conversation stood, so the next resumes it.
  resumed: bool,
  /// The messages the command line heard, as their meaning alone, the reply of the last turn among them.
  chain: Vec<String>,
  /// The last message the command line heard while no reply follows it: the message of a turn that came to no
  /// reply, which the next request may give again grown at its end.
  asked: Option<Message>,
  /// The messages of the last request, which a request that follows it continues.
  requested: Vec<String>,
  /// Whether the command line heard the last message given again, whole or grown, and came to no reply again, so
  /// the next time begins anew.
  again: bool,
  process: Option<Process>,
  used: Instant,
}

impl Conversation {
  fn new(name: &str, system: &str) -> Conversation {
    Conversation {
      name: name.to_owned(),
      system: system.to_owned(),
      id: uuid(),
      parent: None,
      resumed: false,
      chain: Vec::new(),
      asked: None,
      requested: Vec::new(),
      again: false,
      process: None,
      used: Instant::now(),
    }
  }

  /// One turn at an effort: the messages that the command line has not heard, written as one line, and the reply it
  /// streams.
  ///
  /// A request that continues neither the reply of the last turn nor the last request begins a new conversation. A
  /// request that gives again a turn that came to no reply sends its last message again, since a turn that failed is
  /// asked again whole, or, when that message grew at its end by what was told since, what it gained alone, as a new
  /// message of the same conversation, which keeps its cache. It does so once: when the command line heard that and
  /// came to no reply again, the next time begins a new conversation.
  async fn turn(
    &mut self,
    messages: &[Message],
    effort: Option<String>,
    bin: &Path,
    stall: Duration,
    deltas: Option<&Deltas>,
  ) -> Result<CompletionResponse, CompletionError> {
    let incoming: Vec<String> = messages.iter().map(canonical).collect();
    let last = incoming.len().saturating_sub(1);
    let grown = match (&self.asked, messages.last()) {
      (Some(asked), Some(now))
        if self.chain.len() == incoming.len() && incoming[..last] == self.chain[..last] =>
      {
        gained(asked, now)
      }
      _ => None,
    };
    let continues = |held: &[String]| incoming.starts_with(held);
    let start = if grown.is_some() {
      Some(last)
    } else if continues(&self.chain) {
      Some(self.chain.len())
    } else {
      continues(&self.requested).then_some(self.requested.len())
    };
    let again = grown.is_some() || start.is_some_and(|at| at > 0 && at == incoming.len());
    let anew = start.is_none() || again && self.again;
    let delta = match (start, &grown) {
      (Some(_), Some(one)) if !anew => std::slice::from_ref(one),
      (Some(at), None) if !anew => &messages[if again { last } else { at }..],
      _ => {
        *self = Conversation::new(&self.name, &self.system);
        messages
      }
    };
    let asked = messages.last().cloned();
    let again = again && !anew;
    let mut flight =
      Flight { conversation: self, incoming, asked, again, heard: false, reply: None };
    let got = flight.run(delta, effort, bin, stall, deltas).await;
    if let Ok(response) = &got {
      let reply = Message::Assistant { id: None, content: response.choice.clone() };
      flight.reply = Some(canonical(&reply));
    }
    got
  }

  /// A process of the conversation at an effort: a new one, a resumed one, or the fork of its parent.
  async fn start(
    &mut self,
    bin: &Path,
    effort: Option<String>,
  ) -> Result<Process, CompletionError> {
    let mut command = Command::new(bin);
    command.args(["-p", "--input-format", "stream-json", "--output-format", "stream-json"]);
    command.args(["--verbose", "--include-partial-messages", "--model", &self.name]);
    command.args(["--system-prompt", &self.system, "--setting-sources", ""]);
    // A completion needs no tool: no tool of the command line, no server of another, and no command of its own.
    command.args(["--strict-mcp-config", "--tools", "", "--disable-slash-commands"]);
    match (&self.parent, self.resumed) {
      (_, true) => command.args(["--resume", &self.id]),
      (Some(parent), false) => command.args(["--resume", parent, "--fork-session"]),
      (None, false) => command.args(["--session-id", &self.id]),
    };
    if let Some(effort) = &effort {
      command.args(["--effort", effort]);
    }
    // The command line serves completions alone, so its skills and its traffic of no use go.
    command.env("CLAUDE_CODE_DISABLE_BUNDLED_SKILLS", "1");
    command.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    // A program that a process holds open to write it is busy, as a new program is for a moment while a fork of
    // another thread holds it, so it starts at a later try.
    let mut tries = 0;
    let mut child = loop {
      match command.spawn() {
        Err(no) if no.raw_os_error() == Some(BUSY) && tries < 50 => tries += 1,
        got => break got,
      }
      tokio::time::sleep(Duration::from_millis(10)).await;
    }
    .map_err(|no| failed(format!("{} did not start: {no}", bin.display())))?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
      (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
      return Err(failed("claude started with no pipes"));
    };
    self.resumed = true;
    let hurt = Arc::new(Mutex::new(String::new()));
    let tailing = Some(tokio::spawn(tailed(stderr, Arc::clone(&hurt))));
    let (said, lines) = mpsc::unbounded_channel();
    tokio::spawn(listened(stdout, said));
    Ok(Process { effort, child, stdin, lines, hurt, tailing, paid: 0.0 })
  }
}

/// What the stdout of a process says: an event of the stream, a line that is none, or its end, with why when the
/// end is no exit.
enum Line {
  Event(Value),
  Odd(String),
  Over(Option<String>),
}

/// One process of the command line: the effort it spends, the child, its stdin, what its stdout says, the tail of its
/// stderr, and the running cost it said last, since it says the cost of all its turns.
struct Process {
  /// The effort of each turn of the process, which it started at.
  effort: Option<String>,
  child: Child,
  stdin: ChildStdin,
  lines: mpsc::UnboundedReceiver<Line>,
  hurt: Arc<Mutex<String>>,
  /// The read of its stderr, which an exit waits for, so the failure says all the process wrote.
  tailing: Option<tokio::task::JoinHandle<()>>,
  paid: f64,
}

impl Process {
  /// Whether the process still stands: what an earlier turn left unread goes, and an end among it says no.
  fn stands(&mut self) -> bool {
    while let Ok(line) = self.lines.try_recv() {
      if let Line::Over(_) = line {
        return false;
      }
    }
    matches!(self.child.try_wait(), Ok(None))
  }

  /// The last of what the process wrote on its stderr, after a colon, or nothing.
  fn tail(&self) -> String {
    let hurt = self.hurt.lock().map(|one| one.trim().to_owned()).unwrap_or_default();
    if hurt.is_empty() { hurt } else { format!(": {hurt}") }
  }
}

/// Every line of the stdout of a process, read as it comes, until its end.
async fn listened(stdout: impl AsyncRead + Unpin, said: mpsc::UnboundedSender<Line>) {
  let mut reader = BufReader::new(stdout);
  let mut read = Vec::new();
  loop {
    read.clear();
    let limit = u64::try_from(LINE).unwrap_or(u64::MAX) + 1;
    let line = match (&mut reader).take(limit).read_until(b'\n', &mut read).await {
      Ok(0) => Line::Over(None),
      Ok(_) if read.len() > LINE => {
        Line::Over(Some(format!("Claude wrote a line over {LINE} bytes.")))
      }
      Ok(_) => match serde_json::from_slice::<Value>(read.trim_ascii()) {
        Ok(event) => Line::Event(event),
        Err(_) if read.trim_ascii().is_empty() => continue,
        Err(_) => Line::Odd(String::from_utf8_lossy(read.trim_ascii()).into_owned()),
      },
      Err(no) => Line::Over(Some(format!("The stdout of claude failed: {no}"))),
    };
    let over = matches!(line, Line::Over(_));
    if said.send(line).is_err() || over {
      return;
    }
  }
}

/// The last characters of the stderr of a process, kept for the failure that comes next.
async fn tailed(mut stderr: impl AsyncRead + Unpin, hurt: Arc<Mutex<String>>) {
  let mut chunk = vec![0u8; 4096];
  while let Ok(n) = stderr.read(&mut chunk).await
    && n > 0
  {
    let Ok(mut held) = hurt.lock() else { return };
    held.push_str(&String::from_utf8_lossy(&chunk[..n]));
    let cut = held.len() - last(&held).len();
    held.drain(..cut);
  }
}

/// The last characters of a text that a failure keeps.
fn last(text: &str) -> &str {
  let cut = text.char_indices().rev().nth(TAIL - 1).map_or(0, |(at, _)| at);
  &text[cut..]
}

/// A turn in flight. However it ends, the conversation holds the request, what the command line heard of it, and
/// the reply once the turn came to one; a turn that came to no reply ends its process, whose stream is then at no
/// boundary a later turn can trust.
struct Flight<'a> {
  conversation: &'a mut Conversation,
  incoming: Vec<String>,
  /// The last message of the request.
  asked: Option<Message>,
  /// Whether the turn gives again the last message of a turn that came to no reply, whole or grown.
  again: bool,
  /// Whether the command line heard the line of the turn.
  heard: bool,
  reply: Option<String>,
}

impl Drop for Flight<'_> {
  fn drop(&mut self) {
    let held = &mut *self.conversation;
    held.requested = self.incoming.clone();
    if self.heard {
      held.chain = self.incoming.clone();
      held.asked = self.asked.take();
      held.again = self.again;
    }
    match self.reply.take() {
      Some(reply) => {
        held.chain.push(reply);
        held.asked = None;
        held.again = false;
      }
      None => held.process = None,
    }
    held.used = Instant::now();
  }
}

impl Flight<'_> {
  async fn run(
    &mut self,
    delta: &[Message],
    effort: Option<String>,
    bin: &Path,
    stall: Duration,
    deltas: Option<&Deltas>,
  ) -> Result<CompletionResponse, CompletionError> {
    let line = input(delta)?;
    let held = &mut *self.conversation;
    // A process spends the effort it started at, so a turn at another effort ends it.
    if held.process.as_mut().is_some_and(|one| one.effort != effort || !one.stands()) {
      held.process = None;
    }
    if held.process.is_none() {
      held.process = Some(held.start(bin, effort).await?);
    }
    let Conversation { process: Some(process), id, name, .. } = held else {
      return Err(failed("claude is not running"));
    };
    let wrote = async {
      process.stdin.write_all(line.as_bytes()).await?;
      process.stdin.flush().await
    };
    if let Err(no) = wrote.await {
      return Err(failed(format!("claude took no input: {no}{}", process.tail())));
    }
    self.heard = true;
    let mut reply = Reply { deltas, ..Reply::default() };
    let mut odd = String::new();
    let mut due = tokio::time::Instant::now() + stall;
    loop {
      let Ok(heard) = tokio::time::timeout_at(due, process.lines.recv()).await else {
        let odd = if odd.is_empty() { odd } else { format!(" The last line it wrote was {odd}") };
        let seconds = stall.as_secs_f64();
        return Err(failed(format!("Claude made no progress for {seconds}s.{odd}")));
      };
      let event = match heard {
        Some(Line::Event(event)) => event,
        Some(Line::Odd(text)) => {
          last(&text).clone_into(&mut odd);
          continue;
        }
        Some(Line::Over(Some(why))) => return Err(failed(why)),
        Some(Line::Over(None)) | None => {
          let code = process.child.wait().await.ok().and_then(|status| status.code());
          if let Some(tailing) = process.tailing.take() {
            let _ = tokio::time::timeout(Duration::from_secs(1), tailing).await;
          }
          let code = code.map_or_else(|| "a signal".to_owned(), |code| code.to_string());
          return Err(failed(format!("Claude exited ({code}){}", process.tail())));
        }
      };
      let kind = event.get("type").and_then(Value::as_str).unwrap_or_default();
      if matches!(kind, "stream_event" | "assistant" | "result") {
        due = tokio::time::Instant::now() + stall;
      }
      match kind {
        "stream_event" => reply.streamed(event.get("event").unwrap_or(&Value::Null)),
        "assistant" => reply.settled(&event["message"]["content"]),
        "result" => {
          if let Some(session) = event.get("session_id").and_then(Value::as_str) {
            session.clone_into(id);
          }
          if event.get("is_error").and_then(Value::as_bool).unwrap_or_default() {
            let said = event.get("result").and_then(Value::as_str).filter(|one| !one.is_empty());
            return Err(failed(said.unwrap_or("Claude reported an error.")));
          }
          return Ok(reply.response(&event, name, id, process));
        }
        _ => {}
      }
    }
  }
}

/// One block of a reply as the command line streams it: a text, or a thought and its signature.
enum Block {
  Text(String),
  Thinking(String, Option<String>),
}

impl Block {
  fn content(&self) -> AssistantContent {
    match self {
      Block::Text(text) => AssistantContent::text(text.clone()),
      Block::Thinking(text, signature) => {
        AssistantContent::Reasoning(Reasoning::new_with_signature(text, signature.clone()))
      }
    }
  }
}

/// The reply of a turn as it is written: its blocks, and where the stream puts each.
#[derive(Default)]
struct Reply<'a> {
  blocks: Vec<Block>,
  /// The place in the reply of each block the current message streams, by its index in the stream. A block of a kind
  /// this does not read has none, and nothing streamed means the command line says each message whole.
  streamed: Option<HashMap<u64, usize>>,
  /// How many blocks of the current message the command line has settled, which is the index of the next one.
  settled: u64,
  deltas: Option<&'a Deltas>,
}

impl Reply<'_> {
  /// One part of the stream, told to whoever streams the turn.
  fn told(&self, part: RawStreamingChoice) {
    if let Some(deltas) = self.deltas {
      let _ = deltas.send(Ok(part));
    }
  }

  /// A block that begins, which a stream opens.
  fn opened(&mut self, block: Block) {
    let id = self.blocks.len().to_string();
    self.told(match block {
      Block::Text(_) => RawStreamingChoice::TextStart { id: id.into(), additional_params: None },
      Block::Thinking(..) => {
        RawStreamingChoice::ReasoningStart { id: id.into(), provider_id: None }
      }
    });
    self.blocks.push(block);
  }

  /// What the block at a place gained, which a stream tells as it comes.
  fn grew(&mut self, at: usize, text: &str) {
    let id = at.to_string();
    match self.blocks.get_mut(at) {
      Some(Block::Text(held)) => {
        held.push_str(text);
        self.told(RawStreamingChoice::Message(text.to_owned()));
      }
      Some(Block::Thinking(held, _)) => {
        held.push_str(text);
        let reasoning = text.to_owned();
        self.told(RawStreamingChoice::ReasoningDelta {
          id: id.into(),
          provider_id: None,
          reasoning,
        });
      }
      None => {}
    }
  }

  /// The block at a place is whole, which a stream closes.
  fn closed(&self, at: usize) {
    let id = at.to_string().into();
    match self.blocks.get(at) {
      Some(Block::Text(_)) => self.told(RawStreamingChoice::TextEnd { id }),
      Some(block @ Block::Thinking(..)) => {
        let AssistantContent::Reasoning(reasoning) = block.content() else { return };
        let reasoning = Some(reasoning);
        self.told(RawStreamingChoice::ReasoningEnd {
          id,
          reasoning,
          signature: None,
          wire_sent: true,
        });
      }
      None => {}
    }
  }

  /// One event of a message as it streams.
  fn streamed(&mut self, update: &Value) {
    let kind = update.get("type").and_then(Value::as_str).unwrap_or_default();
    if kind == "message_start" {
      self.streamed = Some(HashMap::new());
      self.settled = 0;
    }
    let index = update.get("index").and_then(Value::as_u64).unwrap_or_default();
    let at = self.streamed.get_or_insert_default().get(&index).copied();
    match kind {
      "content_block_start" => {
        let block = &update["content_block"];
        let text =
          |key: &str| block.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
        let block = match block.get("type").and_then(Value::as_str) {
          Some("text") => Block::Text(text("text")),
          Some("thinking") => Block::Thinking(text("thinking"), None),
          _ => return,
        };
        let place = self.blocks.len();
        self.streamed.get_or_insert_default().insert(index, place);
        self.opened(block);
      }
      "content_block_delta" => {
        let Some(at) = at else { return };
        let delta = &update["delta"];
        let text = |key: &str| delta.get(key).and_then(Value::as_str).unwrap_or_default();
        let block = self.blocks.get(at);
        match delta.get("type").and_then(Value::as_str) {
          Some("text_delta") if matches!(block, Some(Block::Text(_))) => {
            self.grew(at, text("text"))
          }
          Some("thinking_delta") if matches!(block, Some(Block::Thinking(..))) => {
            self.grew(at, text("thinking"));
          }
          Some("signature_delta") => {
            if let Some(Block::Thinking(_, signature)) = self.blocks.get_mut(at) {
              signature.get_or_insert_default().push_str(text("signature"));
            }
          }
          _ => {}
        }
      }
      "content_block_stop" => {
        if let Some(at) = at {
          self.closed(at);
        }
      }
      _ => {}
    }
  }

  /// The blocks of a message that the command line says whole. It says each block again, once it is whole, in the
  /// order it streamed them: the whole block stands over the one streamed at its place, and never twice. A message
  /// that did not stream is told as it comes.
  fn settled(&mut self, content: &Value) {
    let blocks = content.as_array().map(Vec::as_slice).unwrap_or_default();
    for one in blocks {
      let text = |key: &str| one.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
      let signature = one.get("signature").and_then(Value::as_str).map(str::to_owned);
      let block = match one.get("type").and_then(Value::as_str) {
        Some("text") => Some(Block::Text(text("text"))),
        Some("thinking") => Some(Block::Thinking(text("thinking"), signature)),
        _ => None,
      };
      match &self.streamed {
        Some(streamed) => {
          let at = streamed.get(&self.settled).copied();
          self.settled += 1;
          if let (Some(at), Some(block)) = (at, block) {
            self.blocks[at] = block;
          }
        }
        None => {
          let Some(block) = block else { continue };
          let whole = match &block {
            Block::Text(text) | Block::Thinking(text, _) => text.clone(),
          };
          let at = self.blocks.len();
          self.opened(match block {
            Block::Text(_) => Block::Text(String::new()),
            Block::Thinking(_, signature) => Block::Thinking(String::new(), signature),
          });
          self.grew(at, &whole);
          self.closed(at);
        }
      }
    }
  }

  /// The response the result of a turn ends the reply with: its blocks, or the text of the result when the reply
  /// holds none, what the turn read and wrote, and its part of the cost the process says of all its turns.
  fn response(
    mut self,
    result: &Value,
    name: &str,
    id: &str,
    process: &mut Process,
  ) -> CompletionResponse {
    if self.blocks.is_empty()
      && let Some(text) = result.get("result").and_then(Value::as_str).filter(|one| !one.is_empty())
    {
      self.blocks.push(Block::Text(text.to_owned()));
    }
    let spent = &result["usage"];
    let count = |key: &str| spent.get(key).and_then(Value::as_u64).unwrap_or_default();
    let mut usage = Usage::new();
    usage.input_tokens = count("input_tokens");
    usage.output_tokens = count("output_tokens");
    usage.cached_input_tokens = count("cache_read_input_tokens");
    usage.cache_creation_input_tokens = count("cache_creation_input_tokens");
    usage.total_tokens = usage.input_tokens
      + usage.output_tokens
      + usage.cached_input_tokens
      + usage.cache_creation_input_tokens;
    let total = result.get("total_cost_usd").and_then(Value::as_f64).unwrap_or(process.paid);
    let cost = (total - process.paid).max(0.0);
    process.paid = total;
    let finish = match result.get("stop_reason").and_then(Value::as_str) {
      Some("max_tokens") => FinishReason::Length,
      Some("refusal") => FinishReason::ContentFilter,
      _ => FinishReason::Stop,
    };
    let choice = self.blocks.iter().map(Block::content).collect();
    CompletionResponse::new(choice, usage, CLAUDE)
      .with_finish_reason(finish)
      .with_model(name)
      .with_response_id(id)
      .with_raw(json!({"cost": cost}))
  }
}

/// The one line of input that holds every message the command line has not heard: an earlier turn framed as such,
/// and the last message of the user as itself.
fn input(messages: &[Message]) -> Result<String, CompletionError> {
  let mut content = Vec::new();
  for (at, message) in messages.iter().enumerate() {
    let (you, blocks): (bool, Vec<Value>) = match message {
      Message::System { .. } => continue,
      Message::User { content } => (false, content.iter().map(said).collect::<Result<_, _>>()?),
      Message::Assistant { content, .. } => {
        let blocks = content.iter().map(|one| match one {
          AssistantContent::Text(text) => Ok(Some(json!({"type": "text", "text": text.text}))),
          AssistantContent::Reasoning(_) => Ok(None),
          _ => Err(failed("The claude command line takes no tool call and no image of its own.")),
        });
        (true, blocks.collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect())
      }
    };
    if you || at + 1 < messages.len() {
      let who = if you { "you" } else { "user" };
      content.push(json!({"type": "text", "text": format!("[earlier turn: {who}]\n")}));
    }
    content.extend(blocks);
  }
  if content.is_empty() {
    return Err(failed("There is no new message for Claude."));
  }
  let line = json!({"type": "user", "message": {"role": "user", "content": content}});
  Ok(format!("{line}\n"))
}

/// What the last message of a request gained over the message the command line heard at its place, as a message of
/// its own: the text after the text it heard, and the blocks after the blocks it heard. It is none when the message
/// changed other than at its end, or gained nothing.
fn gained(heard: &Message, now: &Message) -> Option<Message> {
  let (Message::User { content: heard }, Message::User { content: now }) = (heard, now) else {
    return None;
  };
  let (Some(UserContent::Text(was)), Some(UserContent::Text(is))) = (heard.first(), now.first())
  else {
    return None;
  };
  let text = is.text.strip_prefix(was.text.as_str())?.trim_start();
  let blocks = now[1..].strip_prefix(&heard[1..])?;
  let text = (!text.is_empty()).then(|| UserContent::text(text));
  let content: Vec<UserContent> = text.into_iter().chain(blocks.iter().cloned()).collect();
  (!content.is_empty()).then_some(Message::User { content })
}

/// One block of a message of the user, as the input of the command line holds it.
fn said(block: &UserContent) -> Result<Value, CompletionError> {
  match block {
    UserContent::Text(text) => Ok(json!({"type": "text", "text": text.text})),
    UserContent::Image(image) => {
      let DocumentSourceKind::Base64(data) = &image.data else {
        return Err(failed("The claude command line takes an image as base64 alone."));
      };
      let media = image.media_type.as_ref().map(MimeType::to_mime_type);
      let source = json!({"type": "base64", "media_type": media, "data": data});
      Ok(json!({"type": "image", "source": source}))
    }
    _ => Err(failed(
      "The claude command line takes a text or an image from the user, and no tool result.",
    )),
  }
}

/// A message as its meaning alone, so two requests that say the same thing agree. A thought counts by its place
/// alone, since a reply that a record gives back may hold a thought that was emptied.
fn canonical(message: &Message) -> String {
  let (role, blocks): (&str, Vec<Value>) = match message {
    Message::System { content } => ("system", vec![json!(["text", content])]),
    Message::User { content } => {
      let blocks = content.iter().map(|one| match one {
        UserContent::Text(text) => json!(["text", text.text]),
        one => serde_json::to_value(one).unwrap_or_default(),
      });
      ("user", blocks.collect())
    }
    Message::Assistant { content, .. } => {
      let blocks = content.iter().map(|one| match one {
        AssistantContent::Text(text) => json!(["text", text.text]),
        AssistantContent::Reasoning(_) => json!(["thinking"]),
        one => serde_json::to_value(one).unwrap_or_default(),
      });
      ("assistant", blocks.collect())
    }
  };
  json!([role, blocks]).to_string()
}

/// A failure of the command line, in its own words.
fn failed(why: impl Into<String>) -> CompletionError {
  CompletionError::ProviderError(why.into())
}

/// The name of the program of the command line.
const PROGRAM: &str = if cfg!(windows) { "claude.exe" } else { "claude" };

/// The command line of this machine: the one `FURB_CLAUDE_BIN` names, or the first program named claude on the
/// PATH, in the local programs of the home, or, on macOS, in the newest version of Claude Desktop.
pub(super) fn located() -> Option<PathBuf> {
  if let Some(bin) = env::var_os("FURB_CLAUDE_BIN").filter(|one| !one.is_empty()) {
    return Some(bin.into());
  }
  let mut folders: Vec<PathBuf> =
    env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect();
  if let Some(home) = env::home_dir() {
    folders.push(home.join(".local/bin"));
    if cfg!(target_os = "macos") {
      let desktop = home.join("Library/Application Support/Claude/claude-code");
      let mut versions: Vec<PathBuf> = std::fs::read_dir(&desktop)
        .map(|found| found.filter_map(|one| one.ok().map(|one| one.path())).collect())
        .unwrap_or_default();
      versions.sort_by_key(|one| std::cmp::Reverse(numbered(one)));
      folders.extend(versions.iter().map(|one| one.join("claude.app/Contents/MacOS")));
    }
  }
  crate::world::program("claude", folders)
}

/// The numbers of a version, in order, which sort as a version does.
fn numbered(version: &Path) -> Vec<u64> {
  let name = version.file_name().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
  name.split(|one: char| !one.is_ascii_digit()).filter_map(|one| one.parse().ok()).collect()
}

#[cfg(all(test, unix))]
#[path = "claude.test.rs"]
pub(super) mod test;
