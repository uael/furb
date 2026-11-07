//! One conversation of the claude command line: what it is fixed at, where it stands, the turns
//! that furb writes for its first process to resume, and each turn, which writes to its process
//! only what the command line has not heard and settles what that turn came to.

use std::{
  env,
  path::PathBuf,
  time::{Duration, Instant},
};

use rig_core::{
  completion::{CompletionError, CompletionResponse},
  message::{
    AssistantContent, DocumentSourceKind, Message, MimeType, ReasoningContent, UserContent,
  },
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::{
  Deltas, Error, Program, discover, error, failed,
  process::{GRACE, Line, Process, Spend, absent, exited, last, stalled},
  protocol::{Event, Finished, Reply, unexplained},
  refused, worded,
};
use crate::world::provider;
use crate::world::{
  disk::{self, Access},
  provider::{ENV_THINKING, uuid},
};
use std::fs;
use std::mem;
use std::slice;
use tokio::process;
use tokio::time;

/// The flag by which the command line records its system prompt once for a conversation.
const SNAPSHOT: &str = "--system-prompt-snapshot";

/// The directory that each process runs in: a folder of furb, where the command line finds the
/// memory of no project, and keeps the conversations of furb under one name.
fn home() -> Result<PathBuf, CompletionError> {
  let folder = env::temp_dir().join("furb-claude");
  fs::create_dir_all(&folder)
    .and_then(|()| fs::canonicalize(&folder))
    .map_err(|no| failed(format!("{} was not made: {no}", folder.display())))
}

/// The file that holds a system prompt, which the command line reads by `--system-prompt-file`, so
/// the command line of a process stays short and `ps` shows it whole. The file stands in the
/// temporary directory under the digest of the prompt, so every process of one prompt reads the one
/// file, and a new file lands whole by a rename.
fn prompt_file(system: &str) -> Result<PathBuf, CompletionError> {
  let folder = env::temp_dir().join("furb-claude");
  let file = folder.join(format!("{:x}.md", Sha256::digest(system.as_bytes())));
  if !file.is_file() {
    disk::write(&file, system.as_bytes(), Access::Shared).map_err(|no| failed(no.to_string()))?;
  }
  Ok(file)
}

/// One conversation of the command line: what it is fixed at, its id, where it stands, and its
/// process.
pub(super) struct Conversation {
  pub(super) name: String,
  pub(super) system: String,
  /// The id of the conversation, which the command line keeps it under.
  pub(super) id: String,
  /// The conversation that the first process of this one forks.
  pub(super) parent: Option<String>,
  /// The account that the processes of this conversation run as, which its config dir keeps the
  /// conversation for.
  pub(super) account: Option<usize>,
  /// Whether a process of this conversation stood, so the next resumes it.
  resumed: bool,
  /// The config dir of the account of this conversation, none for the default one: the command line
  /// keeps its conversations there, and a first process finds there the turns that it resumes.
  pub(super) store: Option<PathBuf>,
  /// The messages the command line heard, as their meaning alone, the reply of the last turn among
  /// them.
  pub(super) chain: Vec<String>,
  /// The last message the command line heard while no reply follows it: the message of a turn that
  /// came to no reply, which the next request may give again grown at its end.
  asked: Option<Message>,
  /// The messages of the last request, which a request that follows it continues.
  requested: Vec<String>,
  /// Whether the command line heard the last message given again, whole or grown, and came to no
  /// reply again, so the next time begins anew.
  again: bool,
  pub(super) process: Option<Process>,
  pub(super) used: Instant,
  /// Whether a block of this conversation holds the mark of the cache that ends the setup of its
  /// chain, as a fork holds each block of the conversation it forks.
  pub(super) marked: bool,
}

impl Conversation {
  pub(super) fn new(name: &str, system: &str, id: String) -> Self {
    Self {
      name: name.to_owned(),
      system: system.to_owned(),
      id,
      account: None,
      parent: None,
      resumed: false,
      store: None,
      chain: Vec::new(),
      asked: None,
      requested: Vec::new(),
      again: false,
      process: None,
      used: Instant::now(),
      marked: false,
    }
  }

  /// A new conversation under a new id, on the same account and in the same config dir.
  fn renew(&mut self) -> Result<(), CompletionError> {
    let id = uuid()?;
    let (account, store) = (self.account, self.store.take());
    *self = Self::new(&self.name, &self.system, id);
    self.account = account;
    self.store = store;
    Ok(())
  }

  /// Write the turns before the last message as a conversation of the command line under the id of
  /// this one, with the role of each, which the first process of this one resumes. It says whether
  /// it wrote a turn.
  fn seed(&mut self, past: &[Message]) -> Result<bool, CompletionError> {
    let at = home()?;
    let store = match (&self.store, env::var_os("CLAUDE_CONFIG_DIR")) {
      (Some(dir), _) => dir.clone(),
      (None, Some(dir)) => PathBuf::from(dir),
      (None, None) => env::home_dir()
        .ok_or_else(|| failed("no home holds the config dir of claude"))?
        .join(".claude"),
    };
    let cwd = at.to_string_lossy();
    let name: String =
      cwd.chars().map(|one| if one.is_ascii_alphanumeric() { one } else { '-' }).collect();
    let now = jiff::Timestamp::now().to_string();
    let mut lines = String::new();
    let mut parent = Value::Null;
    // The last message of the user in the past ends the setup that every chain with no source
    // shares, since the turn after it tells the chain where it stands, so its last block takes the
    // mark of the cache.
    let shared = past.iter().rposition(|one| matches!(one, Message::User { .. }));
    for (at, message) in past.iter().enumerate() {
      let (kind, blocks): (&str, Vec<Value>) = match message {
        Message::System { .. } => continue,
        Message::User { content } => {
          let mut blocks: Vec<Value> = content.iter().map(said).collect::<Result<_, _>>()?;
          if Some(at) == shared
            && !self.marked
            && let Some(last) = blocks.last_mut()
          {
            last["cache_control"] = json!({"type": "ephemeral", "ttl": lasting()});
            self.marked = true;
          }
          ("user", blocks)
        }
        Message::Assistant { content, .. } => {
          ("assistant", content.iter().flat_map(resumed).collect())
        }
      };
      if blocks.is_empty() {
        continue;
      }
      let id = uuid()?;
      let message = json!({"role": kind, "content": blocks});
      let entry = json!({"type": kind, "uuid": id, "parentUuid": parent, "sessionId": self.id, "cwd": cwd,
        "isSidechain": false, "userType": "external", "timestamp": now, "message": message});
      lines.push_str(&entry.to_string());
      lines.push('\n');
      parent = json!(id);
    }
    if lines.is_empty() {
      return Ok(false);
    }
    let folder = store.join("projects").join(name);
    let file = folder.join(format!("{}.jsonl", self.id));
    disk::write(&file, lines.as_bytes(), Access::Shared).map_err(|no| failed(no.to_string()))?;
    self.resumed = true;
    Ok(true)
  }

  /// One turn at what it spends: the messages that the command line has not heard, written as one
  /// line, and the reply it streams.
  ///
  /// A request that continues neither the reply of the last turn nor the last request begins a new
  /// conversation. A request that gives again a turn that came to no reply sends its last message
  /// again, since a turn that failed is asked again whole, or, when that message grew at its end by
  /// what was told since, what it gained alone, as a new message of the same conversation, which
  /// keeps its cache. It does so once: when the command line heard that and came to no reply again,
  /// the next time begins a new conversation.
  ///
  /// When the command line holds no conversation under the id that a process resumes, as when its
  /// file is gone, the turn rebuilds it once: a new conversation that resumes the messages of the
  /// request. When that fails too, the failure names the lost conversation and the failed rebuild.
  pub(super) async fn turn(
    &mut self,
    messages: &[Message],
    spend: Spend,
    run: &Run<'_>,
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
    let mut delta = match (start, &grown) {
      (Some(_), Some(one)) if !anew => slice::from_ref(one),
      (Some(at), None) if !anew => &messages[if again { last } else { at }..],
      _ => {
        self.renew()?;
        messages
      }
    };
    let asked = messages.last();
    let mut again = again && !anew;
    let mut vanished = None;
    loop {
      let mut outcome =
        Outcome { incoming: &incoming, asked, again, unheard: None, restored: None, replied: None };
      let got = self.flight(delta, &spend, run, &mut outcome).await.map_err(worded);
      let got = got.map(|(response, process)| {
        let reply = Message::Assistant { id: None, content: response.choice.clone() };
        outcome.replied = Some((canonical(&reply), process));
        response
      });
      self.settle(outcome);
      match (got, &vanished) {
        (Err(no), None) if matches!(error(&no), Some(Error::Lost(_))) => {
          vanished = Some(self.id.clone());
          self.renew()?;
          (delta, again) = (messages, false);
        }
        (Err(no), Some(old)) => {
          let n = messages.len();
          return Err(failed(format!(
            "Claude lost the conversation {old}, and the new one that furb rebuilt from the {n} messages of the \
             chain failed too: {no}"
          )));
        }
        (got, _) => return got,
      }
    }
  }

  /// A process of the conversation at what it spends: a new one, a resumed one, or the fork of its
  /// parent.
  async fn start(
    &mut self,
    program: &Program,
    spend: &Spend,
    gone: Duration,
  ) -> Result<Process, CompletionError> {
    let print = ["-p", "--input-format", "stream-json", "--output-format", "stream-json"];
    let mut command = program.base_command(&print, true).map_err(|why| refused(why.into()))?;
    command.args(["--verbose", "--include-partial-messages", "--model", &self.name]);
    let prompt = prompt_file(&self.system)?;
    command.arg("--system-prompt-file").arg(&prompt);
    // Else a resumed or forked conversation sends the system prompt that its first request
    // recorded, and not this one. A command line that does not list the flag would refuse it, so it
    // gets none.
    if discover::lists(program, SNAPSHOT).await {
      command.args([SNAPSHOT, "off"]);
    }
    command.current_dir(home()?);
    // A completion needs no tool of the command line, as it needs no server of another and no
    // command of its own.
    command.args(["--tools", ""]);
    match (&self.parent, self.resumed) {
      (_, true) => command.args(["--resume", &self.id]),
      (Some(parent), false) => command.args(["--resume", parent, "--fork-session"]),
      (None, false) => command.args(["--session-id", &self.id]),
    };
    if let Some(effort) = &spend.effort {
      command.args(["--effort", effort]);
    }
    if let Some(most) = spend.thinking {
      command.env(ENV_THINKING, most.to_string());
    }
    // Else the command line adds to a message of the user the memory of the project and of the
    // user, and the file of each @path that the message holds, as the results of a tool.
    command.env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1");
    command.env("CLAUDE_CODE_DISABLE_FILE_MENTIONS", "1");
    command.env("CLAUDE_CODE_DISABLE_ATTACHMENTS", "1");
    // Else the command line adds the CLAUDE.md files of the project and of the user to the context.
    command.env("CLAUDE_CODE_DISABLE_CLAUDE_MDS", "1");
    pinned(&mut command);
    let process = Process::spawn(command, program, spend.clone(), gone).await?;
    self.resumed = true;
    Ok(process)
  }

  /// What a turn leaves: the conversation holds its request, what the command line heard of it, and
  /// its reply and the process that wrote it once the turn came to one. A turn that came to no
  /// reply leaves no process, since its stream stands at no boundary that a later turn can trust.
  /// What the command line heard is held from the moment it heard it, by [`Conversation::heard`],
  /// so a turn that is dropped before it settles leaves it too.
  fn settle(&mut self, outcome: Outcome<'_>) {
    self.requested = outcome.incoming.to_vec();
    if let Some(resumed) = outcome.restored {
      self.resumed = resumed;
    }
    if let Some((reply, process)) = outcome.replied {
      self.chain.push(reply);
      self.asked = None;
      self.again = false;
      self.process = Some(process);
    }
    self.used = Instant::now();
  }

  /// What the command line heard of a turn whose line it took: the conversation holds it at once,
  /// so that a turn dropped before it settles leaves it, and the next turn sends only what it
  /// gained. It gives what the conversation held before, which it holds again when the process
  /// found no program to run.
  fn heard(&mut self, outcome: &Outcome<'_>) -> Heard {
    self.requested = outcome.incoming.to_vec();
    Heard {
      chain: mem::replace(&mut self.chain, outcome.incoming.to_vec()),
      asked: mem::replace(&mut self.asked, outcome.asked.cloned()),
      again: mem::replace(&mut self.again, outcome.again),
    }
  }

  /// One flight of a turn: the line of the messages that the command line has not heard, written to
  /// a process at what the turn spends, and the reply that it streams, with that process. The
  /// process stands apart from the conversation until the turn settles, so a turn that is dropped
  /// drops its process, and keeps what the command line heard.
  async fn flight(
    &mut self,
    delta: &[Message],
    spend: &Spend,
    run: &Run<'_>,
    outcome: &mut Outcome<'_>,
  ) -> Result<(CompletionResponse, Process), CompletionError> {
    let held = self.process.take();
    let before = self.resumed;
    // The command line takes no turn of the model on its input: it drops one, or reads it as text
    // of the user. So a first process resumes the turns before the last message from a conversation
    // that furb writes.
    let delta = match delta.split_last() {
      Some((last, past)) if !self.resumed && self.parent.is_none() && self.seed(past)? => {
        slice::from_ref(last)
      }
      _ => delta,
    };
    let line = input(delta, &mut self.marked)?;
    // A process spends what it started at, so a turn at another effort or another most of thought
    // ends it.
    let kept = held.and_then(|mut one| (one.spend == *spend && one.stands()).then_some(one));
    let mut process = match kept {
      Some(one) => one,
      None => self.start(run.program, spend, run.gone).await?,
    };
    // The stall bounds each wait of the turn: for the process to take its input, to write a line,
    // and to end once its stdout closed.
    let due = time::Instant::now() + run.stall;
    let Process { stdin, tail, .. } = &mut process;
    let wrote = async {
      stdin.write_all(line.as_bytes()).await?;
      stdin.flush().await
    };
    match time::timeout_at(due, tail.beside(wrote)).await {
      Ok(Ok(())) => {}
      Ok(Err(no)) => {
        // A process that took no input ended, and the failure says all that it wrote on its stderr.
        time::timeout(GRACE, process.tail.drained()).await.ok();
        let said = process.tail.said();
        if absent(&process.tail.kept) {
          outcome.restored = Some(before);
          let name = run.program.name();
          return Err(refused(Error::Missing(format!("{name} took no input: {no}{said}"))));
        }
        return Err(failed(format!("claude took no input: {no}{said}")));
      }
      Err(_) => return Err(stalled(run.stall, "")),
    }
    let was = self.heard(outcome);
    outcome.unheard = Some(was);
    self.listened(process, run, outcome, before).await
  }

  /// The code of a process that closed its stdout but did not end yet, once it ends before the turn
  /// is due.
  async fn ended_by(
    process: &mut Process,
    due: time::Instant,
    stall: Duration,
    odd: &str,
  ) -> Result<Option<i32>, CompletionError> {
    let waited = process.tail.beside(process.child.wait());
    match time::timeout_at(due, waited).await {
      Ok(status) => Ok(status.ok().and_then(|one| one.code())),
      Err(_) => Err(stalled(stall, odd)),
    }
  }

  /// The response that the result of a turn ends the reply with, with the process that paid for it,
  /// or the failure that the result says.
  fn concluded(
    &mut self,
    result: &Finished,
    reply: Reply,
    mut process: Process,
  ) -> Result<(CompletionResponse, Process), CompletionError> {
    if let Some(session) = &result.session_id {
      session.clone_into(&mut self.id);
    }
    if result.is_error {
      let said = result.result.as_deref().filter(|one| !one.is_empty());
      return Err(failed(said.map_or_else(|| unexplained(result), str::to_owned)));
    }
    let response = reply.response(result, &self.name, &self.id, &mut process.paid);
    Ok((response, process))
  }

  /// The reply that a process streams for the line that it heard, with that process. A process that
  /// found no program to run heard nothing, and the conversation is resumed as it was before the
  /// turn.
  async fn listened(
    &mut self,
    mut process: Process,
    run: &Run<'_>,
    outcome: &mut Outcome<'_>,
    before: bool,
  ) -> Result<(CompletionResponse, Process), CompletionError> {
    let stall = run.stall;
    let mut reply = Reply::new(run.deltas);
    let mut odd = String::new();
    // Whether the process wrote a line, as a process that found no program to run writes none.
    let mut spoke = false;
    // The code of a process that ended. A process that it started may hold its stdout open, so the
    // turn sees the end of the process itself, and then reads what its stdout holds for a grace.
    let mut ended = None;
    let mut due = time::Instant::now() + stall;
    let code = loop {
      let heard = tokio::select! {
        line = process.tail.beside(process.out.line()) => line,
        status = process.child.wait(), if ended.is_none() => {
          ended = Some(status.ok().and_then(|one| one.code()));
          due = time::Instant::now() + GRACE;
          continue;
        }
        () = time::sleep_until(due) => {
          let Some(code) = ended else { return Err(stalled(stall, &odd)) };
          break code;
        }
      };
      let event = match heard {
        Line::Event(event) => *event,
        Line::Odd(text) => {
          spoke = true;
          last(&text).clone_into(&mut odd);
          continue;
        }
        Line::Over(Some(why)) => return Err(failed(why)),
        Line::Over(None) => {
          break if let Some(code) = ended {
            code
          } else {
            Self::ended_by(&mut process, due, stall, &odd).await?
          };
        }
      };
      spoke = true;
      if !matches!(event, Event::Other) {
        due = time::Instant::now() + stall;
      }
      match event {
        Event::Stream { event } => reply.streamed(&event),
        Event::Assistant { message } => {
          reply.spent = message.usage.or_else(|| reply.spent.take());
          reply.settled(&message.content);
        }
        Event::Result(result) => return self.concluded(&result, reply, process),
        Event::ControlResponse { .. } | Event::Other => {}
      }
    };
    let (no, missing) = exited(code, &mut process.tail, run.program, spoke).await;
    if missing {
      if let Some(was) = outcome.unheard.take() {
        (self.chain, self.asked, self.again) = (was.chain, was.asked, was.again);
      }
      outcome.restored = Some(before);
    }
    Err(no)
  }
}

/// What a turn runs with: the program as the account of the turn, how long each wait of the turn
/// may go with no progress, how long a process waits for a program that is missing or does not run,
/// and where the parts of the reply go as the command line writes them.
pub(super) struct Run<'a> {
  pub(super) program: &'a Program,
  pub(super) stall: Duration,
  pub(super) gone: Duration,
  pub(super) deltas: Option<&'a Deltas>,
}

/// What one flight of a turn came to, which [`Conversation::settle`] applies however the flight
/// ends.
struct Outcome<'a> {
  /// The messages of the request, as their meaning alone.
  incoming: &'a [String],
  /// The last message of the request.
  asked: Option<&'a Message>,
  /// Whether the turn gives again the last message of a turn that came to no reply, whole or grown.
  again: bool,
  /// What the conversation held before the command line heard the line of the turn, once it heard
  /// it.
  unheard: Option<Heard>,
  /// Whether the conversation was resumed before the turn, which it is again when the process of
  /// the turn found no program to run, since the command line then holds nothing of the turn.
  restored: Option<bool>,
  /// The reply of the turn, as its meaning alone, and the process that wrote it.
  replied: Option<(String, Process)>,
}

/// What a conversation held of what the command line heard: the messages, the last message that
/// came to no reply, and whether it was given again.
struct Heard {
  chain: Vec<String>,
  asked: Option<Message>,
  again: bool,
}

/// The one line of input that holds every message the command line has not heard: an earlier turn
/// framed as such, and the last message of the user as itself.
fn input(messages: &[Message], marked: &mut bool) -> Result<String, CompletionError> {
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
  cached(&mut content, marked);
  // The client composed the turn, so the command line delivers its text as written: no file
  // mention, no slash command and no attachment pass at the start of the turn.
  let line = json!({"type": "user", "message": {"role": "user", "content": content}, "client_composed": true});
  Ok(format!("{line}\n"))
}

/// What the last message of a request gained over the message the command line heard at its place,
/// as a message of its own: the text after the text it heard, and the blocks after the blocks it
/// heard. It is none when the message changed other than at its end, or gained nothing.
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

/// The life of each mark of the cache in a request of the command line: an hour, or five minutes
/// when `FORCE_PROMPT_CACHING_5M` forces it on the command line, which no other setting overrides.
/// The API refuses a request that puts a mark of an hour after one of minutes, and the command line
/// marks its system prompt before the messages and the last message after them, so the mark that a
/// conversation writes lives as long as the marks of the command line in each of its requests.
pub(super) fn lasting() -> &'static str {
  if env::var_os("FORCE_PROMPT_CACHING_5M").is_some_and(|one| !one.is_empty()) {
    "5m"
  } else {
    "1h"
  }
}

/// The command line pinned to the life of [`lasting`] in each of its requests. Unpinned, it marks
/// for an hour the turns of a subscription within its limits alone, and for five minutes its
/// compaction, its turns past the limits and every turn on a key of the API, so a mark of an hour
/// in a message made its compaction fail.
fn pinned(command: &mut process::Command) {
  command.env("CLAUDE_CODE_PROMPT_CACHE_TTL", lasting());
  command.env("CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL", lasting());
  command.env_remove("CLAUDE_CODE_MESSAGES_PROMPT_CACHE_TTL");
}

/// The blocks of a message with the block before the first open of a chain in a conversation marked
/// for the cache, so the setup that every chain with no source shares ends an entry of the cache,
/// which the first turn of every later chain reads. The mark lives as long as the marks of the
/// command line, as [`lasting`] says, and a conversation marks one block at most, as the command
/// line takes four marks in all.
pub(super) fn cached(blocks: &mut [Value], marked: &mut bool) {
  let opens = |one: &Value| {
    one["text"].as_str().is_some_and(|text| provider::banner(&format!("\n\n{text}")) == Some(2))
  };
  if let Some(at) = (1..blocks.len()).find(|&at| !*marked && opens(&blocks[at])) {
    blocks[at - 1]["cache_control"] = json!({"type": "ephemeral", "ttl": lasting()});
    *marked = true;
  }
}

/// One block of a turn of the model as a conversation of the command line holds it: its text, or
/// each part of its thought that the model signed, whole with its signature, so the next reply
/// reads the thought of the turn before it. A thought with no signature, as one that the provider
/// cut, and one that a record emptied go not at all, since the API takes no thought that its
/// signature does not prove.
fn resumed(block: &AssistantContent) -> Vec<Value> {
  let signed = |one: &ReasoningContent| match one {
    ReasoningContent::Text { text, signature: Some(signature) } if !text.is_empty() => {
      Some(json!({"type": "thinking", "thinking": text, "signature": signature}))
    }
    ReasoningContent::Redacted { data } => Some(json!({"type": "redacted_thinking", "data": data})),
    _ => None,
  };
  match block {
    AssistantContent::Text(text) => vec![json!({"type": "text", "text": text.text})],
    AssistantContent::Reasoning(thought) => thought.content.iter().filter_map(signed).collect(),
    _ => Vec::new(),
  }
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

/// A message as its meaning alone, so two requests that say the same thing agree. A thought counts
/// by its place alone, since a reply that a record gives back may hold a thought that was emptied.
pub(super) fn canonical(message: &Message) -> String {
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
