//! The claude command line as a completion model of rig: one process of `claude -p` holds one conversation and
//! stays up between its turns, and each turn writes to it only what it has not heard.
//!
//! A conversation is keyed by its model, its system prompt and the conversation the request names, which the provider
//! names as a chain of its life, so a chain holds one conversation and at most one process, whatever the effort. A
//! process spends the effort it started at, so a new effort ends it, and the next process resumes the same
//! conversation. A process sets `MAX_THINKING_TOKENS` to the most tokens of thought that its turns name, and
//! inherits it from the environment of furb when they name none, and a new most ends it as a new effort does. A
//! process that is gone leaves its conversation on the disk under its id, so a later turn resumes
//! it, and a new conversation that grows out of one the command line holds forks it. A subscription pays for the
//! turns, and no key is read.

use std::{
  collections::{HashMap, VecDeque},
  env,
  path::{Path, PathBuf},
  process::Stdio,
  sync::Arc,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use jiff::Zoned;
use rig_core::{
  completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse, FinishReason, Usage,
  },
  message::{AssistantContent, DocumentSourceKind, Message, MimeType, Reasoning, UserContent},
  streaming::{RawStreamingChoice, StreamingCompletionResponse},
};
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
  sync::{mpsc, oneshot},
};

use futures::{FutureExt, StreamExt};
use sha2::{Digest, Sha256};

use super::{Model, ended, search, uuid};
use crate::ear::reactor;

mod accounts;
mod discover;
pub use accounts::{Account, Source, Switch};
use accounts::{Accounts, NO_TOKEN};
use discover::Found;
pub use discover::refresh;

/// The name of the command line as a provider: each response of it says this name, and each of its models is named
/// by it and the alias of its family.
pub const CLAUDE: &str = "claude-cli";

/// The efforts the command line spends on a turn, from least to most.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// How many conversations keep a process at most.
const WARM: usize = 8;

/// How many conversations the command line holds at most.
const HELD: usize = 64;

/// How long a conversation with no turn keeps its process.
const IDLE: Duration = Duration::from_secs(300);

/// How long each wait of a turn may go with no progress, unless `FURB_CLAUDE_STALL` gives other seconds: for the process
/// to take the input of the turn, to write a line of its stream, and to end once its stdout closed.
const STALL: Duration = Duration::from_secs(900);

/// How long a turn reads what a process that ended still holds on its stdout and its stderr, since a process that it
/// started may hold them open.
const GRACE: Duration = Duration::from_secs(1);

/// The longest line of the stream that the command line may write.
const LINE: usize = 32 * 1024 * 1024;

/// How many characters of what a process wrote on its stderr a failure keeps.
const TAIL: usize = 2000;

/// The error of the system that says a program is open to be written, which is ETXTBSY.
const BUSY: i32 = 26;

/// How long a process waits for its program when the program is missing or does not run, unless
/// `FURB_CLAUDE_GONE` gives other seconds: the command line that updates itself replaces its program, and its link
/// points at nothing for a moment.
const GONE: Duration = Duration::from_secs(60);

/// The first pause before a process tries its program again, which doubles at each try up to [`PAUSE`].
const FIRST: Duration = Duration::from_millis(100);

/// The longest pause before a process tries its program again.
const PAUSE: Duration = Duration::from_secs(5);

/// What the command line says when it holds no conversation under the id that a process resumes, as when its file
/// is gone.
const LOST: &str = "No conversation found with session ID";

/// The words that begin the failure of a turn whose program was missing or did not run, as while an update of claude
/// replaces it, which the provider takes for a failure that passes.
pub(super) const MISSING: &str = "No program to run";

/// The words that begin the failure of a turn whose account was parked, which the provider takes for a failure that
/// passes: the turn that asks again runs as another account.
pub(super) const SWITCHED: &str = "Switched account";

/// What a failed sign in of the command line says, in lower case.
const SIGNED_OUT: [&str; 6] = [
  "please run /login",
  "not logged in",
  "invalid api key",
  "oauth token has expired",
  "authentication_error",
  NO_TOKEN,
];

/// The claude command line: its program, how long a turn may go with no progress, how long a process waits for a
/// program that is missing or does not run, and the conversations it holds.
///
/// It is cheap to clone, and every clone holds the same conversations, which one keeper owns and lends to one turn at
/// a time. When the last clone goes, the keeper ends, and every process it started ends.
#[derive(Clone)]
pub struct Claude {
  program: Program,
  stall: Duration,
  gone: Duration,
  keeper: mpsc::UnboundedSender<Kept>,
}

impl Claude {
  /// The command line of [`Claude::with`] and no program, when `FURB_CLAUDE_BIN` names one or this machine holds one.
  pub fn found() -> Option<Claude> {
    located().map(|bin| Claude::with(Some(bin), None))
  }

  /// The command line at this program, and how long a turn may go with no progress. With no program, it is the one
  /// that `FURB_CLAUDE_BIN` names, or the one found on this machine; with no stall, `FURB_CLAUDE_STALL` gives it in
  /// seconds, or it is 900 seconds. A process waits for a program that is missing or does not run for the seconds
  /// that `FURB_CLAUDE_GONE` gives, or for 60 seconds.
  pub fn with(bin: Option<PathBuf>, stall: Option<Duration>) -> Claude {
    let bin = bin.or_else(located).unwrap_or_else(|| PROGRAM.into());
    Claude::with_command(bin, Vec::new(), stall)
  }

  /// The command line that this program starts with these words before every flag that furb gives it, as `cswap run
  /// 3` holds the program `cswap` and the words `run` and `3`. No shell reads the words: each reaches the program as
  /// it stands, with its spaces, once, for a turn and for a search alike. The stall is the one of [`Claude::with`].
  pub fn with_command(bin: PathBuf, args: Vec<String>, stall: Option<Duration>) -> Claude {
    Claude::with_accounts(Some(bin), args, Vec::new(), Switch::default(), stall)
  }

  /// The command line that runs as these accounts, in the order of the list. The program and the stall are those of
  /// [`Claude::with`], and the words come before each flag as [`Claude::with_command`] says. A turn that meets a usage
  /// limit or a failed sign in parks its account and ends as a failure that passes, so the turn that asks again runs
  /// as another account: a limit parks an account until its reset, and a failed sign in until a new command line
  /// stands, as after a reload. With one account or none, nothing is parked.
  pub fn with_accounts(
    bin: Option<PathBuf>,
    args: Vec<String>,
    accounts: Vec<Account>,
    switch: Switch,
    stall: Option<Duration>,
  ) -> Claude {
    let bin = bin.or_else(located).unwrap_or_else(|| PROGRAM.into());
    let stall = stall.unwrap_or_else(|| {
      let seconds = env::var("FURB_CLAUDE_STALL").ok().and_then(|one| one.parse::<f64>().ok());
      seconds.and_then(|one| Duration::try_from_secs_f64(one).ok()).unwrap_or(STALL)
    });
    let accounts = Arc::new(Accounts::new(accounts, switch));
    let (keeper, asked) = mpsc::unbounded_channel();
    reactor().spawn(kept(asked, Arc::clone(&accounts)));
    Claude { program: Program::with(bin, args, accounts), stall, gone: patience(), keeper }
  }

  /// Whether the command line is signed in, as `claude auth status` says. It counts as signed in unless that command
  /// says it is not, in its JSON or in its words, so a version with no such command, or one that does not end in ten
  /// seconds, passes, and a turn says what is wrong. With accounts, it is signed in when any account is, and an
  /// account whose token cannot be read is not.
  pub fn signed(&self) -> bool {
    (0..self.program.accounts.count()).any(|at| self.program.under(at).signed())
  }

  /// The same command line, whose process waits this long for a program that is missing or does not run.
  #[cfg(all(test, unix))]
  pub(crate) fn waiting(self, gone: Duration) -> Claude {
    Claude { gone, ..self }
  }

  /// One model of the command line, by its name, as a completion model of rig.
  pub fn completion_model(&self, name: impl Into<String>) -> Completion {
    Completion { claude: self.clone(), name: name.into() }
  }

  /// The models of the command line as the provider offers them, each named `claude-cli:` and its alias: each with
  /// the window and the efforts that the command line reports for it, the images of a turn, the conversation of a
  /// reply under the setting `session`, and the most tokens of thought of a turn under the setting
  /// `max_thinking_tokens`. The command line counts what it read of the cache apart from the rest of what it read.
  ///
  /// The command line says its models itself, with no model prompt, once for each version of its program: a process
  /// of its own asks it, and furb keeps the answer in its cache until the program changes or [`refresh`] is called.
  /// A command line that does not answer, as an older one does not, gives the fixed table of furb, within 20 seconds.
  pub fn models(&self) -> Vec<Model> {
    self.family(discover::models(&self.program).unwrap_or_else(discover::fallback))
  }

  /// The models that furb offers when it cannot ask the command line: the fixed table, which asks nothing.
  pub fn fixed(&self) -> Vec<Model> {
    self.family(discover::fallback())
  }

  fn family(&self, found: Vec<Found>) -> Vec<Model> {
    found
      .into_iter()
      .map(|one| {
        let named = format!("{CLAUDE}:{}", one.alias);
        let model = Model::new(named, one.window, self.completion_model(one.alias.as_str()))
          .conversation("session");
        let mut model = model.searches(search::claude(self.program.clone(), one.alias.clone()));
        model.apart = true;
        model.thought = Some(super::Thought::Told);
        one.efforts.iter().fold(model.images(), |model, effort| {
          model.effort(effort.as_str(), json!({"effort": effort}))
        })
      })
      .collect()
  }
}

/// The program that starts the command line: its file, and the words that come before each flag that furb gives it,
/// as `cswap run 3` holds the file `cswap` and the words `run` and `3`. It starts a process with no shell, so each
/// word reaches the program as it stands.
#[derive(Clone)]
pub(super) struct Program {
  bin: Arc<Path>,
  args: Arc<[String]>,
  accounts: Arc<Accounts>,
  /// The account that each command of this program runs as, or none for the first that is not parked.
  pin: Option<usize>,
}

impl Program {
  #[cfg(all(test, unix))]
  pub(super) fn new(bin: PathBuf, args: Vec<String>) -> Program {
    Program::with(bin, args, Arc::new(Accounts::new(Vec::new(), Switch::default())))
  }

  fn with(bin: PathBuf, args: Vec<String>, accounts: Arc<Accounts>) -> Program {
    Program { bin: bin.into(), args: args.into(), accounts, pin: None }
  }

  /// The same program, whose commands all run as this account.
  fn under(&self, at: usize) -> Program {
    Program { pin: Some(at), ..self.clone() }
  }

  /// A command that holds the program and its words, to which a turn or a search adds its own flags, and the
  /// credential of its account. It fails with the reason when the token of the account cannot be read.
  pub(super) fn command(&self) -> Result<Command, String> {
    let mut command = Command::new(&*self.bin);
    command.args(&*self.args);
    let at = self.pin.unwrap_or_else(|| self.accounts.first_free(SystemTime::now()));
    self.accounts.apply(at, command.as_std_mut())?;
    Ok(command)
  }

  /// Whether this program, as its account, is signed in.
  fn signed(&self) -> bool {
    let Ok(mut tokio) = self.command() else { return false };
    let Ok(mut child) = tokio
      .as_std_mut()
      .args(["auth", "status"])
      .stdin(std::process::Stdio::null())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::piped())
      .spawn()
    else {
      return true;
    };
    let until = std::time::Instant::now() + Duration::from_secs(10);
    while matches!(child.try_wait(), Ok(None)) {
      if std::time::Instant::now() >= until {
        let _ = child.kill();
        let _ = child.wait();
        return true;
      }
      std::thread::sleep(Duration::from_millis(20));
    }
    let Ok(out) = child.wait_with_output() else { return true };
    let said = String::from_utf8_lossy(&out.stdout);
    let logged =
      serde_json::from_str::<Value>(&said).ok().and_then(|one| one["loggedIn"].as_bool());
    let words = format!("{said}{}", String::from_utf8_lossy(&out.stderr)).to_lowercase();
    logged.unwrap_or(!words.contains("not logged in"))
  }

  /// The file of the program, as a failure to start it names it.
  pub(super) fn name(&self) -> std::path::Display<'_> {
    self.bin.display()
  }
}

/// One model of the claude command line, as a completion model of rig.
///
/// The request names its conversation under the setting `session`, its effort under `effort`, and the most tokens of
/// thought of the turn under `max_thinking_tokens`. It takes no tool
/// and no document, since furb asks for completions alone.
#[derive(Clone)]
pub struct Completion {
  claude: Claude,
  name: String,
}

/// Where the parts of a streamed turn go as the command line writes them.
type Deltas = mpsc::UnboundedSender<Result<RawStreamingChoice, CompletionError>>;

impl CompletionModel for Completion {
  async fn completion(
    &self,
    request: CompletionRequest,
  ) -> Result<CompletionResponse, CompletionError> {
    self.claude.turn(&self.name, request, None).await
  }

  async fn stream(
    &self,
    request: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let (claude, name) = (self.claude.clone(), self.name.clone());
    let (deltas, mut heard) = mpsc::unbounded_channel();
    let turn = async move {
      let got = claude.turn(&name, request, Some(&deltas)).await;
      // Rig makes the choice of a stream of the parts it streamed, so the raw record of its end holds the blocks the
      // command line settled whole, which stand over those parts.
      let got = got.map(|mut response| {
        response.raw["settled"] = serde_json::to_value(&response.choice).unwrap_or_default();
        RawStreamingChoice::FinalResponse(ended(CLAUDE, response))
      });
      let _ = deltas.send(got);
      None
    };
    // The turn goes with the stream: it runs as the stream is read, and a stream that is dropped ends it.
    let told = futures::stream::poll_fn(move |cx| heard.poll_recv(cx));
    let stream =
      futures::stream::select(told, futures::stream::once(turn).filter_map(async |one| one));
    Ok(StreamingCompletionResponse::stream(CLAUDE, Box::pin(stream)))
  }
}

impl Claude {
  /// One turn of a model: the conversation it continues, which the keeper lends it alone until the turn is over, and
  /// what it came to.
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
    if let Some(effort) =
      effort.as_deref().filter(|one| !discover::takes(&self.program, &name, one))
    {
      let efforts = EFFORTS.join(", ");
      return Err(failed(format!("{name} spends one of {efforts} on a turn, never {effort}.")));
    }
    let thinking = settings.get(super::KEY_THINKING).and_then(Value::as_u64);
    let spend = Spend { effort, thinking };
    let session = settings.get("session").and_then(Value::as_str).map_or_else(uuid, str::to_owned);
    let system = request.preamble.unwrap_or_default();
    let messages: Vec<Message> = request
      .chat_history
      .into_iter()
      .filter(|one| !matches!(one, Message::System { .. }))
      .collect();
    let key = json!([name, system, session]).to_string();
    let (lent, borrowed) = oneshot::channel();
    let asked = Kept::Lend { key: key.clone(), name, system, messages: messages.clone(), lent };
    let gone = || failed("the keeper of the conversations of claude is gone");
    self.keeper.send(asked).map_err(|_| gone())?;
    let conversation = borrowed.await.map_err(|_| gone())?;
    let mut lent = Lent { key, conversation: Some(conversation), keeper: self.keeper.clone() };
    let accounts = &self.program.accounts;
    let now = SystemTime::now();
    let held = lent.conversation();
    let at = match held.account.filter(|at| accounts.free(*at, now)) {
      Some(at) => at,
      None => {
        let Some(at) = accounts.pick(now) else { return Err(self.exhausted(now)) };
        if held.account.is_some() {
          // The conversation of a parked account stays in its config dir, so this one begins again.
          *held = Conversation::new(&held.name, &held.system);
        }
        held.account = Some(at);
        at
      }
    };
    held.store = accounts.store(Some(at)).map(Path::to_path_buf);
    let program = self.program.under(at);
    let got = held.turn(&messages, spend, &program, self.stall, self.gone, deltas).await;
    got.map_err(|no| self.switched(at, no))
  }

  /// The failure of a turn that finds every account parked: the wait for the first reset of a limit, in the words of
  /// the command line, or that every account is signed out.
  fn exhausted(&self, now: SystemTime) -> CompletionError {
    match self.program.accounts.earliest(now) {
      Some(end) => failed(format!("Claude AI usage limit reached|{}", epoch(end))),
      None => failed(
        "Every account of the claude command line is signed out. Sign one in, then run /reload.",
      ),
    }
  }

  /// What a failed turn of an account comes to. A usage limit or a failed sign in parks the account, and the turn
  /// ends as a failure that passes when another account stands, so the provider asks again as that account. When none
  /// stands, it ends as the wait for the first reset, or as it was.
  fn switched(&self, at: usize, no: CompletionError) -> CompletionError {
    let accounts = &self.program.accounts;
    if !accounts.switchable() {
      return no;
    }
    let Some(why) = refusal(&no) else { return no };
    let now = SystemTime::now();
    let (until, reason) = match why {
      Refusal::Limit(end) => (Some(end.unwrap_or(now + super::LIMIT_POLL)), "its usage limit"),
      Refusal::SignedOut => (None, "its sign in failed"),
    };
    accounts.park(at, until);
    match (accounts.peek(now), accounts.earliest(now)) {
      (Some(next), _) => failed(format!(
        "{SWITCHED} from {} to {}: {reason}",
        accounts.name(at),
        accounts.name(next)
      )),
      (None, Some(_)) => self.exhausted(now),
      (None, None) => no,
    }
  }
}

/// Why an account cannot serve: a usage limit, and the moment it ends if the message names one, or a failed sign in.
enum Refusal {
  Limit(Option<SystemTime>),
  SignedOut,
}

fn refusal(no: &CompletionError) -> Option<Refusal> {
  if let Some(reset) = super::limit(no, &Zoned::now()) {
    let end = reset
      .and_then(|at| u64::try_from(at.timestamp().as_second()).ok())
      .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds));
    return Some(Refusal::Limit(end));
  }
  let CompletionError::ProviderError(said) = no else { return None };
  let said = said.to_lowercase();
  SIGNED_OUT.iter().any(|one| said.contains(one)).then_some(Refusal::SignedOut)
}

/// A time as the seconds since 1970, which the older message of a usage limit gives.
fn epoch(at: SystemTime) -> u64 {
  at.duration_since(UNIX_EPOCH).map_or(0, |one| one.as_secs())
}

/// What the keeper of the conversations is asked: to lend the conversation of a key to a turn, or to take it back.
enum Kept {
  Lend {
    key: String,
    name: String,
    system: String,
    messages: Vec<Message>,
    lent: oneshot::Sender<Box<Conversation>>,
  },
  Back {
    key: String,
    conversation: Box<Conversation>,
  },
}

/// The conversation of a key as the keeper holds it: at hand, or lent to a turn, with the turns that wait for it.
enum Slot {
  Idle(Box<Conversation>),
  Lent(VecDeque<oneshot::Sender<Box<Conversation>>>),
}

/// A conversation lent to a turn, which goes back to the keeper however the turn ends.
struct Lent {
  key: String,
  conversation: Option<Box<Conversation>>,
  keeper: mpsc::UnboundedSender<Kept>,
}

impl Lent {
  fn conversation(&mut self) -> &mut Conversation {
    self.conversation.as_mut().expect("a lent conversation is held until it goes back")
  }
}

impl Drop for Lent {
  fn drop(&mut self) {
    if let Some(conversation) = self.conversation.take() {
      let key = std::mem::take(&mut self.key);
      let _ = self.keeper.send(Kept::Back { key, conversation });
    }
  }
}

/// The keeper of the conversations: it lends each to one turn at a time, and the next turn of a conversation lent
/// waits for it; it takes each back, and then sweeps; and it sweeps once in each span that a process may sit idle.
/// It ends when the last clone of the command line goes, and every conversation goes with it.
async fn kept(mut asked: mpsc::UnboundedReceiver<Kept>, accounts: Arc<Accounts>) {
  let mut held: HashMap<String, Slot> = HashMap::new();
  let mut ticks = tokio::time::interval(IDLE);
  ticks.tick().await;
  loop {
    tokio::select! {
      one = asked.recv() => match one {
        None => return,
        Some(Kept::Lend { key, name, system, messages, lent }) => {
          lend(&mut held, &accounts, key, &name, &system, &messages, lent);
        }
        Some(Kept::Back { key, conversation }) => {
          back(&mut held, key, conversation);
          swept(&mut held);
        }
      },
      _ = ticks.tick() => swept(&mut held),
    }
  }
}

/// The conversation of a key, lent to a turn: the one the keeper holds, or a new one, which forks the idle
/// conversation it grows out of when there is one, so it takes all that conversation holds. A turn of a conversation
/// lent waits for it. A new conversation takes the account that the policy gives, and forks only a conversation that
/// stands in the same config dir.
fn lend(
  held: &mut HashMap<String, Slot>,
  accounts: &Accounts,
  key: String,
  name: &str,
  system: &str,
  messages: &[Message],
  lent: oneshot::Sender<Box<Conversation>>,
) {
  let conversation = match held.remove(&key) {
    Some(Slot::Idle(conversation)) => conversation,
    Some(Slot::Lent(mut waiting)) => {
      waiting.push_back(lent);
      held.insert(key, Slot::Lent(waiting));
      return;
    }
    None => {
      let incoming: Vec<String> = messages.iter().map(canonical).collect();
      let mut made = Conversation::new(name, system);
      made.account = accounts.pick(SystemTime::now());
      let store = accounts.store(made.account);
      let donor = held
        .values()
        .filter_map(|one| match one {
          Slot::Idle(one) => Some(one),
          Slot::Lent(_) => None,
        })
        .filter(|one| {
          let chain = one.chain.len();
          chain > 0
            && one.name == name
            && one.system == system
            && accounts.store(one.account) == store
            && chain < incoming.len()
            && incoming[..chain] == one.chain[..]
            && messages[chain..].iter().all(|one| !matches!(one, Message::Assistant { .. }))
        })
        .max_by_key(|one| one.chain.len());
      if let Some(donor) = donor {
        made.parent = Some(donor.id.clone());
        made.chain = donor.chain.clone();
      }
      Box::new(made)
    }
  };
  match lent.send(conversation) {
    Ok(()) => held.insert(key, Slot::Lent(VecDeque::new())),
    // The turn went before it was lent the conversation, which stays at hand.
    Err(conversation) => held.insert(key, Slot::Idle(conversation)),
  };
}

/// A conversation taken back: lent to the next turn that waits for it and still stands, or kept at hand.
fn back(held: &mut HashMap<String, Slot>, key: String, mut conversation: Box<Conversation>) {
  let mut waiting = match held.remove(&key) {
    Some(Slot::Lent(waiting)) => waiting,
    _ => VecDeque::new(),
  };
  while let Some(next) = waiting.pop_front() {
    match next.send(conversation) {
      Ok(()) => {
        held.insert(key, Slot::Lent(waiting));
        return;
      }
      Err(again) => conversation = again,
    }
  }
  held.insert(key, Slot::Idle(conversation));
}

/// What no turn needs goes: the process of a conversation that sat idle too long, or that stands past the warm ones,
/// the least used first, and a conversation with no process past the held ones. A conversation lent stands warm.
fn swept(held: &mut HashMap<String, Slot>) {
  let count = held.len();
  let mut idle: Vec<_> = held
    .iter_mut()
    .filter_map(|(key, one)| match one {
      Slot::Idle(one) => Some((key.clone(), one)),
      Slot::Lent(_) => None,
    })
    .collect();
  idle.sort_by_key(|(_, one)| one.used);
  let mut warm = count - idle.len();
  warm += idle.iter().filter(|(_, one)| one.process.is_some()).count();
  let mut gone = Vec::new();
  for (key, one) in &mut idle {
    if one.process.is_some() && (warm > WARM || one.used.elapsed() > IDLE) {
      one.process = None;
      warm -= 1;
    }
    if one.process.is_none() && count - gone.len() > HELD {
      gone.push(key.clone());
    }
  }
  drop(idle);
  for key in gone {
    held.remove(&key);
  }
}

/// The directory that each process runs in: a folder of furb, where the command line finds the memory of no project,
/// and keeps the conversations of furb under one name.
fn home() -> Result<PathBuf, CompletionError> {
  let folder = env::temp_dir().join("furb-claude");
  std::fs::create_dir_all(&folder)
    .and_then(|()| std::fs::canonicalize(&folder))
    .map_err(|no| failed(format!("{} was not made: {no}", folder.display())))
}

/// The file that holds a system prompt, which the command line reads by `--system-prompt-file`, so the command line
/// of a process stays short and `ps` shows it whole. The file stands in the temporary directory under the digest of
/// the prompt, so every process of one prompt reads the one file, and a new file lands whole by a rename.
fn prompt_file(system: &str) -> Result<PathBuf, CompletionError> {
  let folder = env::temp_dir().join("furb-claude");
  let file = folder.join(format!("{:x}.md", Sha256::digest(system.as_bytes())));
  if !file.is_file() {
    std::fs::create_dir_all(&folder)
      .map_err(|no| failed(format!("{} was not made: {no}", folder.display())))?;
    let part = folder.join(format!("{}.part", uuid()));
    std::fs::write(&part, system)
      .and_then(|()| std::fs::rename(&part, &file))
      .map_err(|no| failed(format!("{} was not written: {no}", file.display())))?;
  }
  Ok(file)
}

/// One conversation of the command line: what it is fixed at, its id, where it stands, and its process.
struct Conversation {
  name: String,
  system: String,
  /// The id of the conversation, which the command line keeps it under.
  id: String,
  /// The conversation that the first process of this one forks.
  parent: Option<String>,
  /// The account that the processes of this conversation run as, which its config dir keeps the conversation for.
  account: Option<usize>,
  /// Whether a process of this conversation stood, so the next resumes it.
  resumed: bool,
  /// The config dir of the account of this conversation, none for the default one: the command line keeps its
  /// conversations there, and a first process finds there the turns that it resumes.
  store: Option<PathBuf>,
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
  /// Whether a block of this conversation holds the mark of the cache that ends the setup of its chain.
  marked: bool,
}

impl Conversation {
  fn new(name: &str, system: &str) -> Conversation {
    Conversation {
      name: name.to_owned(),
      system: system.to_owned(),
      id: uuid(),
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
  fn renew(&mut self) {
    let (account, store) = (self.account, self.store.take());
    *self = Conversation::new(&self.name, &self.system);
    self.account = account;
    self.store = store;
  }

  /// Write the turns before the last message as a conversation of the command line under the id of this one, with the
  /// role of each, which the first process of this one resumes. It says whether it wrote a turn.
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
    for message in past {
      let (kind, blocks): (&str, Vec<Value>) = match message {
        Message::System { .. } => continue,
        Message::User { content } => {
          let mut blocks: Vec<Value> = content.iter().map(said).collect::<Result<_, _>>()?;
          cached(&mut blocks, &mut self.marked);
          ("user", blocks)
        }
        Message::Assistant { content, .. } => {
          let blocks = content.iter().filter_map(|one| match one {
            AssistantContent::Text(text) => Some(json!({"type": "text", "text": text.text})),
            _ => None,
          });
          ("assistant", blocks.collect())
        }
      };
      if blocks.is_empty() {
        continue;
      }
      let id = uuid();
      let message = json!({"role": kind, "content": blocks});
      let entry = json!({"type": kind, "uuid": id, "parentUuid": parent, "sessionId": self.id, "cwd": cwd,
        "isSidechain": false, "userType": "external", "timestamp": now, "message": message});
      lines.push_str(&format!("{entry}\n"));
      parent = json!(id);
    }
    if lines.is_empty() {
      return Ok(false);
    }
    let folder = store.join("projects").join(name);
    let file = folder.join(format!("{}.jsonl", self.id));
    let part = folder.join(format!("{}.part", uuid()));
    std::fs::create_dir_all(&folder)
      .and_then(|()| std::fs::write(&part, lines))
      .and_then(|()| std::fs::rename(&part, &file))
      .map_err(|no| failed(format!("{} was not written: {no}", file.display())))?;
    self.resumed = true;
    Ok(true)
  }

  /// One turn at what it spends: the messages that the command line has not heard, written as one line, and the reply it
  /// streams.
  ///
  /// A request that continues neither the reply of the last turn nor the last request begins a new conversation. A
  /// request that gives again a turn that came to no reply sends its last message again, since a turn that failed is
  /// asked again whole, or, when that message grew at its end by what was told since, what it gained alone, as a new
  /// message of the same conversation, which keeps its cache. It does so once: when the command line heard that and
  /// came to no reply again, the next time begins a new conversation.
  ///
  /// When the command line holds no conversation under the id that a process resumes, as when its file is gone, the
  /// turn rebuilds it once: a new conversation that resumes the messages of the request. When that fails too, the
  /// failure names the lost conversation and the failed rebuild.
  async fn turn(
    &mut self,
    messages: &[Message],
    spend: Spend,
    program: &Program,
    stall: Duration,
    gone: Duration,
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
    let mut delta = match (start, &grown) {
      (Some(_), Some(one)) if !anew => std::slice::from_ref(one),
      (Some(at), None) if !anew => &messages[if again { last } else { at }..],
      _ => {
        self.renew();
        messages
      }
    };
    let asked = messages.last().cloned();
    let mut again = again && !anew;
    let mut lost = None;
    loop {
      let mut flight = Flight {
        conversation: &mut *self,
        incoming: incoming.clone(),
        asked: asked.clone(),
        again,
        heard: false,
        reply: None,
        restored: None,
      };
      let got = flight.run(delta, spend.clone(), program, stall, gone, deltas).await;
      if let Ok(response) = &got {
        let reply = Message::Assistant { id: None, content: response.choice.clone() };
        flight.reply = Some(canonical(&reply));
      }
      drop(flight);
      match (got, &lost) {
        (Err(no), None) if no.to_string().contains(LOST) => {
          lost = Some(self.id.clone());
          self.renew();
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

  /// A process of the conversation at what it spends: a new one, a resumed one, or the fork of its parent. When its program
  /// is missing or does not run, it tries again after a pause that grows, until the program starts or [`GONE`]
  /// passed.
  async fn start(
    &mut self,
    program: &Program,
    spend: Spend,
    gone: Duration,
  ) -> Result<Process, CompletionError> {
    let mut command = program.command().map_err(failed)?;
    command.args(["-p", "--input-format", "stream-json", "--output-format", "stream-json"]);
    command.args(["--verbose", "--include-partial-messages", "--model", &self.name]);
    let prompt = prompt_file(&self.system)?;
    command.arg("--system-prompt-file").arg(&prompt).args(["--setting-sources", ""]);
    command.current_dir(home()?);
    // A completion needs no tool: no tool of the command line, no server of another, and no command of its own.
    command.args(["--strict-mcp-config", "--tools", "", "--disable-slash-commands"]);
    match (&self.parent, self.resumed) {
      (_, true) => command.args(["--resume", &self.id]),
      (Some(parent), false) => command.args(["--resume", parent, "--fork-session"]),
      (None, false) => command.args(["--session-id", &self.id]),
    };
    if let Some(effort) = &spend.effort {
      command.args(["--effort", effort]);
    }
    if let Some(most) = spend.thinking {
      command.env(super::ENV_THINKING, most.to_string());
    }
    // The command line serves completions alone, so its skills and its traffic of no use go.
    command.env("CLAUDE_CODE_DISABLE_BUNDLED_SKILLS", "1");
    command.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
    // Else the command line adds to a message of the user the memory of the project and of the user, and the file of
    // each @path that the message holds, as the results of a tool.
    command.env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1");
    command.env("CLAUDE_CODE_DISABLE_FILE_MENTIONS", "1");
    command.env("CLAUDE_CODE_DISABLE_ATTACHMENTS", "1");
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    // A program that a process holds open to write it is busy, as a new program is for a moment while a fork of
    // another thread holds it, so it starts at a later try.
    let mut tries = 0;
    let due = Instant::now() + gone;
    let mut pause = FIRST;
    let mut child = loop {
      let wait = match command.spawn() {
        Err(no) if no.raw_os_error() == Some(BUSY) && tries < 50 => {
          tries += 1;
          Duration::from_millis(10)
        }
        Err(no)
          if matches!(
            no.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
          ) && Instant::now() < due =>
        {
          let wait = pause.min(due.saturating_duration_since(Instant::now()));
          pause = (pause * 2).min(PAUSE);
          wait
        }
        got => break got,
      };
      tokio::time::sleep(wait).await;
    }
    .map_err(|no| {
      let absent =
        matches!(no.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied);
      let missing = if absent { format!("{MISSING}: ") } else { String::new() };
      failed(format!("{missing}{} did not start: {no}", program.name()))
    })?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
      (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
      return Err(failed("claude started with no pipes"));
    };
    self.resumed = true;
    let out = Out { reader: BufReader::new(stdout), read: Vec::new() };
    let tail = Tail { stderr: Some(stderr), kept: String::new() };
    Ok(Process { spend, child, stdin, out, tail, paid: 0.0 })
  }
}

/// What the stdout of a process says: an event of the stream, a line that is none, or its end, with why when the
/// end is no exit.
enum Line {
  Event(Value),
  Odd(String),
  Over(Option<String>),
}

/// What a process of the command line spends on each of its turns, which it started at: its effort, and the most
/// tokens of thought that it sets as `MAX_THINKING_TOKENS` when its turns name one.
#[derive(Clone, PartialEq)]
struct Spend {
  effort: Option<String>,
  thinking: Option<u64>,
}

/// One process of the command line: what it spends, the child, its stdin, its stdout, the tail of its stderr,
/// and the running cost it said last, since it says the cost of all its turns.
struct Process {
  /// What each turn of the process spends, which it started at.
  spend: Spend,
  child: Child,
  stdin: ChildStdin,
  out: Out,
  tail: Tail,
  paid: f64,
}

impl Process {
  /// Whether the process still stands: what an earlier turn left unread goes, and an end among it says no.
  fn stands(&mut self) -> bool {
    while let Some(line) = self.out.line().now_or_never() {
      if let Line::Over(_) = line {
        return false;
      }
    }
    matches!(self.child.try_wait(), Ok(None))
  }
}

/// The stdout of a process, read a line at a time, and the part of a line read so far, which stays when a read is
/// dropped before its line is whole.
struct Out {
  reader: BufReader<ChildStdout>,
  read: Vec<u8>,
}

impl Out {
  /// The next line of the stdout, as it comes.
  async fn line(&mut self) -> Line {
    loop {
      let limit = u64::try_from((LINE + 1).saturating_sub(self.read.len())).unwrap_or(u64::MAX);
      let got = (&mut self.reader).take(limit).read_until(b'\n', &mut self.read).await;
      return match got {
        Ok(0) if self.read.is_empty() => Line::Over(None),
        Ok(_) if self.read.len() > LINE => {
          Line::Over(Some(format!("Claude wrote a line over {LINE} bytes.")))
        }
        Ok(_) => {
          let read = std::mem::take(&mut self.read);
          match serde_json::from_slice::<Value>(read.trim_ascii()) {
            Ok(event) => Line::Event(event),
            Err(_) if read.trim_ascii().is_empty() => continue,
            Err(_) => Line::Odd(String::from_utf8_lossy(read.trim_ascii()).into_owned()),
          }
        }
        Err(no) => Line::Over(Some(format!("The stdout of claude failed: {no}"))),
      };
    }
  }
}

/// The stderr of a process, until its end, and the last characters it wrote, kept for the failure that comes next.
struct Tail {
  stderr: Option<ChildStderr>,
  kept: String,
}

impl Tail {
  /// What the stderr writes next, kept, until its end.
  async fn heard(&mut self) {
    let Some(stderr) = self.stderr.as_mut() else { return std::future::pending().await };
    let mut chunk = [0u8; 4096];
    match stderr.read(&mut chunk).await {
      Ok(n) if n > 0 => {
        self.kept.push_str(&String::from_utf8_lossy(&chunk[..n]));
        let cut = self.kept.len() - last(&self.kept).len();
        self.kept.drain(..cut);
      }
      _ => self.stderr = None,
    }
  }

  /// What a work of the process comes to, with its stderr read meanwhile, so the process never stops for it.
  async fn beside<T>(&mut self, work: impl Future<Output = T>) -> T {
    let mut work = std::pin::pin!(work);
    loop {
      tokio::select! {
        got = &mut work => return got,
        () = self.heard() => {}
      }
    }
  }

  /// All that the stderr writes until its end.
  async fn drained(&mut self) {
    while self.stderr.is_some() {
      self.heard().await;
    }
  }

  /// The last of what the process wrote on its stderr, after a colon, or nothing.
  fn said(&self) -> String {
    let kept = self.kept.trim();
    if kept.is_empty() { String::new() } else { format!(": {kept}") }
  }
}

/// The last characters of a text that a failure keeps.
fn last(text: &str) -> &str {
  let cut = text.char_indices().rev().nth(TAIL - 1).map_or(0, |(at, _)| at);
  &text[cut..]
}

/// The failure of a turn that waited its stall with no progress, with the last line it read that was no event.
fn stalled(stall: Duration, odd: &str) -> CompletionError {
  let seconds = stall.as_secs_f64();
  let odd =
    if odd.is_empty() { String::new() } else { format!(" The last line it wrote was {odd}") };
  failed(format!("Claude made no progress for {seconds}s.{odd}"))
}

/// The failure of a turn whose process ended, with its code or by a signal, and all that it wrote on its stderr,
/// which the stderr gives until its end or for a grace; and whether the process found no program to run: it wrote no
/// line of its stream, and its stderr says so, as a prefix of the command line says while an update replaces claude.
async fn exited(
  code: Option<i32>,
  tail: &mut Tail,
  program: &Program,
  spoke: bool,
) -> (CompletionError, bool) {
  let _ = tokio::time::timeout(GRACE, tail.drained()).await;
  let code = code.map_or_else(|| "a signal".to_owned(), |code| code.to_string());
  if !spoke && absent(&tail.kept) {
    let said = tail.said();
    return (failed(format!("{MISSING}: {} exited ({code}){said}", program.name())), true);
  }
  (failed(format!("Claude exited ({code}){}", tail.said())), false)
}

/// Whether what a process wrote on its stderr says that it found no program.
fn absent(said: &str) -> bool {
  let said = said.to_lowercase();
  said.contains("not found") || said.contains("no such file or directory")
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
  /// Whether the conversation was resumed before the turn, which it is again when the process of the turn found no
  /// program to run, since the command line then holds nothing of the turn.
  restored: Option<bool>,
}

impl Drop for Flight<'_> {
  fn drop(&mut self) {
    let held = &mut *self.conversation;
    held.requested = self.incoming.clone();
    if let Some(resumed) = self.restored {
      held.resumed = resumed;
    }
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
    spend: Spend,
    program: &Program,
    stall: Duration,
    gone: Duration,
    deltas: Option<&Deltas>,
  ) -> Result<CompletionResponse, CompletionError> {
    let held = &mut *self.conversation;
    let before = held.resumed;
    // The command line takes no turn of the model on its input: it drops one, or reads it as text of the user. So a
    // first process resumes the turns before the last message from a conversation that furb writes.
    let delta = match delta.split_last() {
      Some((last, past)) if !held.resumed && held.parent.is_none() && held.seed(past)? => {
        std::slice::from_ref(last)
      }
      _ => delta,
    };
    let line = input(delta, &mut held.marked)?;
    // A process spends what it started at, so a turn at another effort or another most of thought ends it.
    if held.process.as_mut().is_some_and(|one| one.spend != spend || !one.stands()) {
      held.process = None;
    }
    if held.process.is_none() {
      held.process = Some(held.start(program, spend, gone).await?);
    }
    let Conversation { process: Some(process), id, name, .. } = held else {
      return Err(failed("claude is not running"));
    };
    // The stall bounds each wait of the turn: for the process to take its input, to write a line, and to end once its
    // stdout closed.
    let mut due = tokio::time::Instant::now() + stall;
    let Process { stdin, tail, .. } = process;
    let wrote = async {
      stdin.write_all(line.as_bytes()).await?;
      stdin.flush().await
    };
    match tokio::time::timeout_at(due, tail.beside(wrote)).await {
      Ok(Ok(())) => {}
      Ok(Err(no)) => {
        // A process that took no input ended, and the failure says all that it wrote on its stderr.
        let _ = tokio::time::timeout(GRACE, process.tail.drained()).await;
        let said = process.tail.said();
        if absent(&process.tail.kept) {
          self.restored = Some(before);
          return Err(failed(format!("{MISSING}: {} took no input: {no}{said}", program.name())));
        }
        return Err(failed(format!("claude took no input: {no}{said}")));
      }
      Err(_) => return Err(stalled(stall, "")),
    }
    self.heard = true;
    let mut reply = Reply { deltas, ..Reply::default() };
    let mut odd = String::new();
    // Whether the process wrote a line, as a process that found no program to run writes none.
    let mut spoke = false;
    // The code of a process that ended. A process that it started may hold its stdout open, so the turn sees the end
    // of the process itself, and then reads what its stdout holds for a grace.
    let mut ended = None;
    due = tokio::time::Instant::now() + stall;
    let code = loop {
      let heard = tokio::select! {
        line = process.tail.beside(process.out.line()) => line,
        status = process.child.wait(), if ended.is_none() => {
          ended = Some(status.ok().and_then(|one| one.code()));
          due = tokio::time::Instant::now() + GRACE;
          continue;
        }
        () = tokio::time::sleep_until(due) => {
          let Some(code) = ended else { return Err(stalled(stall, &odd)) };
          break code;
        }
      };
      let event = match heard {
        Line::Event(event) => event,
        Line::Odd(text) => {
          spoke = true;
          last(&text).clone_into(&mut odd);
          continue;
        }
        Line::Over(Some(why)) => return Err(failed(why)),
        Line::Over(None) => {
          let code = match ended {
            Some(code) => code,
            None => {
              let waited = process.tail.beside(process.child.wait());
              match tokio::time::timeout_at(due, waited).await {
                Ok(status) => status.ok().and_then(|one| one.code()),
                Err(_) => return Err(stalled(stall, &odd)),
              }
            }
          };
          break code;
        }
      };
      spoke = true;
      let kind = event.get("type").and_then(Value::as_str).unwrap_or_default();
      if matches!(kind, "stream_event" | "assistant" | "result") {
        due = tokio::time::Instant::now() + stall;
      }
      match kind {
        "stream_event" => reply.streamed(event.get("event").unwrap_or(&Value::Null)),
        "assistant" => {
          if let Some(spent) = event["message"].get("usage") {
            reply.spent = Some(spent.clone());
          }
          reply.settled(&event["message"]["content"]);
        }
        "result" => {
          if let Some(session) = event.get("session_id").and_then(Value::as_str) {
            session.clone_into(id);
          }
          if event.get("is_error").and_then(Value::as_bool).unwrap_or_default() {
            let said = event.get("result").and_then(Value::as_str).filter(|one| !one.is_empty());
            return Err(failed(said.map_or_else(|| unexplained(&event), str::to_owned)));
          }
          return Ok(reply.response(&event, name, id, process));
        }
        _ => {}
      }
    };
    let (no, missing) = exited(code, &mut process.tail, program, spoke).await;
    if missing {
      self.heard = false;
      self.restored = Some(before);
    }
    Err(no)
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
  /// The usage of the last message the command line settled, which is the one request of the turn whose answer the
  /// reply holds.
  spent: Option<Value>,
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
  /// holds none, what its last request read and wrote, and its part of the cost the process says of all its turns.
  /// The usage of the result is the sum of every request of the turn, and the command line makes many when the model
  /// calls a tool, which it refuses, or stops at its limit of output, so only the last request says what the answer
  /// read of the window. A result that no message with a usage came before gives its own usage.
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
    let spent = self.spent.take().unwrap_or_else(|| result["usage"].clone());
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

/// The blocks of a message with the block before the first open of a chain in a conversation marked for the cache, so
/// the setup that every chain with no source shares ends an entry of the cache, which the first turn of every later
/// chain reads. The mark lives an hour, as the marks of the command line do, since a mark of an hour must come before
/// one of minutes, and a conversation marks one block at most, as the command line takes four marks in all.
fn cached(blocks: &mut [Value], marked: &mut bool) {
  let opens = |one: &Value| {
    one["text"]
      .as_str()
      .is_some_and(|text| crate::world::provider::banner(&format!("\n\n{text}")) == Some(2))
  };
  if let Some(at) = (1..blocks.len()).find(|&at| !*marked && opens(&blocks[at])) {
    blocks[at - 1]["cache_control"] = json!({"type": "ephemeral", "ttl": "1h"});
    *marked = true;
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

/// What an error result tells when its text is empty: its subtype, its status and its errors, so that a login error
/// shows apart from a limit.
fn unexplained(event: &Value) -> String {
  let mut facts = Vec::new();
  if let Some(subtype) = event.get("subtype").and_then(Value::as_str).filter(|one| !one.is_empty())
  {
    facts.push(format!("subtype {subtype}"));
  }
  if let Some(status) = event.get("api_error_status").filter(|one| !one.is_null()) {
    facts.push(format!("api_error_status {status}"));
  }
  let errors = match event.get("errors") {
    Some(Value::Array(all)) => all
      .iter()
      .map(|one| one.as_str().map_or_else(|| one.to_string(), str::to_owned))
      .collect::<Vec<_>>()
      .join("; "),
    _ => String::new(),
  };
  let head = if facts.is_empty() { String::new() } else { format!(" ({})", facts.join(", ")) };
  let tail = if errors.is_empty() { String::new() } else { format!(": {errors}") };
  format!("Claude reported an error{head}{tail}.")
}

/// The name of the program of the command line.
const PROGRAM: &str = if cfg!(windows) { "claude.exe" } else { "claude" };

/// How long a process waits for its program: the seconds that `FURB_CLAUDE_GONE` gives, or [`GONE`].
fn patience() -> Duration {
  let seconds = env::var("FURB_CLAUDE_GONE").ok().and_then(|one| one.parse::<f64>().ok());
  seconds.and_then(|one| Duration::try_from_secs_f64(one).ok()).unwrap_or(GONE)
}

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
#[path = "mod.test.rs"]
pub(super) mod test;
