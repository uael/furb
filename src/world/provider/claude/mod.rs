//! The claude command line as a completion model of rig: one process of `claude -p` holds one
//! conversation and stays up between its turns, and each turn writes to it only what it has not
//! heard.
//!
//! A conversation is keyed by its model, its system prompt and the conversation the request names,
//! which the provider names as a chain of its life, so a chain holds one conversation and at most
//! one process, whatever the effort. A process spends the effort it started at, so a new effort
//! ends it, and the next process resumes the same conversation. A process sets
//! `MAX_THINKING_TOKENS` to the most tokens of thought that its turns name, and inherits it from
//! the environment of furb when they name none, and a new most ends it as a new effort does. A
//! process that is gone leaves its conversation on the disk under its id, so a later turn resumes
//! it, and a new conversation that grows out of one the command line holds forks it. A subscription
//! pays for the turns, and no key is read.

use std::{
  env,
  path::{Path, PathBuf},
  sync::Arc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use jiff::Zoned;
use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
  message::Message,
  streaming::{RawStreamingChoice, StreamingCompletionResponse},
};
use serde_json::{Value, json};
use tokio::{process::Command, sync::mpsc};

use futures::StreamExt;

use super::{Model, ended, search, uuid};
use crate::ear::runtime;

mod accounts;
mod conversation;
mod discover;
mod keeper;
mod locate;
mod process;
mod protocol;
pub use accounts::{Account, Source, Switch};
use accounts::{Accounts, NO_TOKEN};
use conversation::{Conversation, Run};
use discover::Found;
pub use discover::refresh;
use keeper::Kept;
use locate::PROGRAM;
pub(super) use locate::located;
use process::Spend;

/// The name of the command line as a provider: each response of it says this name, and each of its
/// models is named by it and the alias of its family.
pub const CLAUDE: &str = "claude-cli";

/// The efforts the command line spends on a turn, from least to most.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// How long each wait of a turn may go with no progress, unless `FURB_CLAUDE_STALL` gives other
/// seconds: for the process to take the input of the turn, to write a line of its stream, and to
/// end once its stdout closed.
const STALL: Duration = Duration::from_mins(15);

/// How long a process waits for its program when the program is missing or does not run, unless
/// `FURB_CLAUDE_GONE` gives other seconds: the command line that updates itself replaces its
/// program, and its link points at nothing for a moment.
const GONE: Duration = Duration::from_secs(60);

/// The words that begin the failure of a turn whose program was missing or did not run, as while an
/// update of claude replaces it, which the provider takes for a failure that passes.
pub(super) const MISSING: &str = "No program to run";

/// The words that begin the failure of a turn whose account was parked, which the provider takes
/// for a failure that passes: the turn that asks again runs as another account.
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

/// The words that begin the failure of a turn that finds every account signed out.
const ALL_SIGNED_OUT: &str = "Every account of the claude command line is signed out.";

/// Whether a failure is a failed sign in of the command line, or that every account of it is signed
/// out. A sign in outside furb ends it, so the provider waits and asks again.
pub(super) fn signed_out(no: &CompletionError) -> bool {
  let CompletionError::ProviderError(said) = no else { return false };
  let said = said.to_lowercase();
  said.starts_with(&ALL_SIGNED_OUT.to_lowercase()) || signed_out_words(&said)
}

fn signed_out_words(said: &str) -> bool {
  SIGNED_OUT.iter().any(|one| said.contains(one))
}

/// The claude command line: its program, how long a turn may go with no progress, how long a
/// process waits for a program that is missing or does not run, and the conversations it holds.
///
/// It is cheap to clone, and every clone holds the same conversations, which one keeper owns and
/// lends to one turn at a time. When the last clone goes, the keeper ends, and every process it
/// started ends.
#[derive(Clone)]
pub struct Claude {
  program: Program,
  stall: Duration,
  gone: Duration,
  keeper: mpsc::UnboundedSender<Kept>,
}

impl Claude {
  /// The command line of [`Claude::with`] and no program, when `FURB_CLAUDE_BIN` names one or this
  /// machine holds one.
  pub fn found() -> Option<Claude> {
    located().map(|bin| Claude::with(Some(bin), None))
  }

  /// The command line at this program, and how long a turn may go with no progress. With no
  /// program, it is the one that `FURB_CLAUDE_BIN` names, or the one found on this machine; with no
  /// stall, `FURB_CLAUDE_STALL` gives it in seconds, or it is 900 seconds. A process waits for a
  /// program that is missing or does not run for the seconds that `FURB_CLAUDE_GONE` gives, or for
  /// 60 seconds.
  pub fn with(bin: Option<PathBuf>, stall: Option<Duration>) -> Claude {
    let bin = bin.or_else(located).unwrap_or_else(|| PROGRAM.into());
    Claude::with_command(bin, Vec::new(), stall)
  }

  /// The command line that this program starts with these words before every flag that furb gives
  /// it, as `cswap run 3` holds the program `cswap` and the words `run` and `3`. No shell reads the
  /// words: each reaches the program as it stands, with its spaces, once, for a turn and for a
  /// search alike. The stall is the one of [`Claude::with`].
  pub fn with_command(bin: PathBuf, args: Vec<String>, stall: Option<Duration>) -> Claude {
    Claude::with_accounts(Some(bin), args, Vec::new(), Switch::default(), stall)
  }

  /// The command line that runs as these accounts, in the order of the list. The program and the
  /// stall are those of [`Claude::with`], and the words come before each flag as
  /// [`Claude::with_command`] says. A turn that meets a usage limit or a failed sign in parks its
  /// account and ends as a failure that passes, so the turn that asks again runs as another
  /// account: a limit parks an account until its reset, and a failed sign in until a new command
  /// line stands, as after a reload. With one account or none, nothing is parked.
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
    runtime().spawn(keeper::kept(asked, Arc::clone(&accounts)));
    Claude { program: Program::with(bin, args, accounts), stall, gone: patience(), keeper }
  }

  /// Whether the command line is signed in, as `claude auth status` says. It counts as signed in
  /// unless that command says it is not, in its JSON or in its words, so a version with no such
  /// command, or one that does not end in ten seconds, passes, and a turn says what is wrong. With
  /// accounts, it is signed in when any account is, and an account whose token cannot be read is
  /// not.
  pub fn signed(&self) -> bool {
    (0..self.program.accounts.count()).any(|at| self.program.under(at).signed())
  }

  /// The same command line, whose process waits this long for a program that is missing or does not
  /// run.
  #[cfg(all(test, unix))]
  pub(crate) fn waiting(self, gone: Duration) -> Claude {
    Claude { gone, ..self }
  }

  /// One model of the command line, by its name, as a completion model of rig.
  pub fn completion_model(&self, name: impl Into<String>) -> Completion {
    Completion { claude: self.clone(), name: name.into() }
  }

  /// The models of the command line as the provider offers them, each named `claude-cli:` and its
  /// alias: each with the window and the efforts that the command line reports for it, the images
  /// of a turn, the conversation of a reply under the setting `session`, and the most tokens of
  /// thought of a turn under the setting `max_thinking_tokens`. The command line counts what it
  /// read of the cache apart from the rest of what it read.
  ///
  /// The command line says its models itself, with no model prompt, once for each version of its
  /// program: a process of its own asks it, and furb keeps the answer in its cache until the
  /// program changes or [`refresh`] is called. A command line that does not answer, as an older one
  /// does not, gives the fixed table of furb, within 20 seconds.
  pub fn models(&self) -> Vec<Model> {
    self.family(discover::models(&self.program).unwrap_or_else(discover::fallback))
  }

  /// The models that furb offers when it cannot ask the command line: the fixed table, which asks
  /// nothing.
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
        // The command line says no price, so the catalog gives the class of each of its models.
        let model = match super::catalog::family(CLAUDE, &one.alias) {
          Some(class) => model.classed(class),
          None => model,
        };
        one.efforts.iter().fold(model.images(), |model, effort| {
          model.effort(effort.as_str(), json!({"effort": effort}))
        })
      })
      .collect()
  }
}

/// The program that starts the command line: its file, and the words that come before each flag
/// that furb gives it, as `cswap run 3` holds the file `cswap` and the words `run` and `3`. It
/// starts a process with no shell, so each word reaches the program as it stands.
#[derive(Clone)]
pub(super) struct Program {
  bin: Arc<Path>,
  args: Arc<[String]>,
  accounts: Arc<Accounts>,
  /// The account that each command of this program runs as, or none for the first that is not
  /// parked.
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

  /// A command that holds the program and its words, and the credential of its account. It fails
  /// with the reason when the token of the account cannot be read.
  fn command(&self) -> Result<Command, String> {
    let mut command = Command::new(&*self.bin);
    command.args(&*self.args);
    let at = self.pin.unwrap_or_else(|| self.accounts.first_free(SystemTime::now()));
    self.accounts.apply(at, command.as_std_mut())?;
    Ok(command)
  }

  /// A command that holds the program and its words, then these flags, then the flags and the
  /// environment that furb gives the command line for a turn, a probe and a search alike: no source
  /// of settings, no server of another, no command of its own, no bundled skill and no traffic of
  /// no use. Each adds its own flags after them. It fails with the reason when the token of the
  /// account cannot be read.
  pub(super) fn base_command(&self, first: &[&str]) -> Result<Command, String> {
    let mut command = self.command()?;
    command.args(first);
    command.args(["--setting-sources", "", "--strict-mcp-config", "--disable-slash-commands"]);
    command.env("CLAUDE_CODE_DISABLE_BUNDLED_SKILLS", "1");
    command.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
    Ok(command)
  }

  /// Whether this program, as its account, is signed in. It waits for the command on the runtime of
  /// the ears, so it never runs on a thread of that runtime nor on the thread of a life, and the
  /// command is ended after ten seconds.
  #[expect(
    clippy::disallowed_methods,
    reason = "debt: a call that blocks: block_on, where the catalog asks whether it is signed in with no await"
  )]
  fn signed(&self) -> bool {
    let Ok(mut command) = self.command() else { return false };
    command
      .args(["auth", "status"])
      .stdin(std::process::Stdio::null())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::piped())
      .kill_on_drop(true);
    let asked =
      async move { tokio::time::timeout(Duration::from_secs(10), command.output()).await };
    let Ok(Ok(out)) = runtime().block_on(asked) else { return true };
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
/// The request names its conversation under the setting `session`, its effort under `effort`, and
/// the most tokens of thought of the turn under `max_thinking_tokens`. It takes no tool and no
/// document, since furb asks for completions alone.
#[derive(Clone)]
pub struct Completion {
  claude: Claude,
  name: String,
}

/// Where the parts of a streamed turn go as the command line writes them.
pub(super) type Deltas = mpsc::UnboundedSender<Result<RawStreamingChoice, CompletionError>>;

impl CompletionModel for Completion {
  async fn completion(
    &self,
    request: CompletionRequest,
  ) -> Result<CompletionResponse, CompletionError> {
    self.claude.turn(&self.name, request, None).await
  }

  #[expect(clippy::unused_async_trait_impl, reason = "debt: an async fn that awaits nothing")]
  async fn stream(
    &self,
    request: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let (claude, name) = (self.claude.clone(), self.name.clone());
    let (deltas, mut heard) = mpsc::unbounded_channel();
    let turn = async move {
      let got = claude.turn(&name, request, Some(&deltas)).await;
      // Rig makes the choice of a stream of the parts it streamed, so the raw record of its end
      // holds the blocks the command line settled whole, which stand over those parts.
      let got = got.map(|mut response| {
        response.raw["settled"] = serde_json::to_value(&response.choice).unwrap_or_default();
        RawStreamingChoice::FinalResponse(ended(CLAUDE, response))
      });
      drop(deltas.send(got));
      None
    };
    // The turn goes with the stream: it runs as the stream is read, and a stream that is dropped
    // ends it.
    let told = futures::stream::poll_fn(move |cx| heard.poll_recv(cx));
    let stream =
      futures::stream::select(told, futures::stream::once(turn).filter_map(async |one| one));
    Ok(StreamingCompletionResponse::stream(CLAUDE, Box::pin(stream)))
  }
}

impl Claude {
  /// One turn of a model: the conversation it continues, which the keeper lends it alone until the
  /// turn is over, and what it came to.
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
    let session = match settings.get("session").and_then(Value::as_str) {
      Some(session) => session.to_owned(),
      None => uuid()?,
    };
    let system = request.preamble.unwrap_or_default();
    let messages: Vec<Message> = request
      .chat_history
      .into_iter()
      .filter(|one| !matches!(one, Message::System { .. }))
      .collect();
    let key = json!([name, system, session]).to_string();
    let fresh = Box::new(Conversation::new(&name, &system, uuid()?));
    let mut lent = keeper::borrow(&self.keeper, key, fresh, messages.clone()).await?;
    let accounts = &self.program.accounts;
    let now = SystemTime::now();
    let held = lent.conversation();
    let at = match held.account.filter(|at| accounts.free(*at, now)) {
      Some(at) => at,
      None => {
        let Some(at) = accounts.pick(now) else { return Err(self.exhausted(now)) };
        if held.account.is_some() {
          // The conversation of a parked account stays in its config dir, so this one begins again.
          *held = Conversation::new(&held.name, &held.system, uuid()?);
        }
        held.account = Some(at);
        at
      }
    };
    held.store = accounts.store(Some(at)).map(Path::to_path_buf);
    let program = self.program.under(at);
    let run = Run { program: &program, stall: self.stall, gone: self.gone, deltas };
    let got = held.turn(&messages, spend, &run).await;
    got.map_err(|no| self.switched(at, no))
  }

  /// The failure of a turn that finds every account parked: the wait for the first reset of a
  /// limit, in the words of the command line, or that every account is signed out.
  fn exhausted(&self, now: SystemTime) -> CompletionError {
    match self.program.accounts.earliest(now) {
      Some(end) => failed(format!("Claude AI usage limit reached|{}", epoch(end))),
      None => failed(format!("{ALL_SIGNED_OUT} Sign one in, then run /reload.")),
    }
  }

  /// What a failed turn of an account comes to. A usage limit or a failed sign in parks the
  /// account, and the turn ends as a failure that passes when another account stands, so the
  /// provider asks again as that account. When none stands, it ends as the wait for the first
  /// reset, or as it was.
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

/// Why an account cannot serve: a usage limit, and the moment it ends if the message names one, or
/// a failed sign in.
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
  signed_out_words(&said).then_some(Refusal::SignedOut)
}

/// A time as the seconds since 1970, which the older message of a usage limit gives.
fn epoch(at: SystemTime) -> u64 {
  at.duration_since(UNIX_EPOCH).map_or(0, |one| one.as_secs())
}

/// A failure of the command line, in its own words.
fn failed(why: impl Into<String>) -> CompletionError {
  CompletionError::ProviderError(why.into())
}

/// How long a process waits for its program: the seconds that `FURB_CLAUDE_GONE` gives, or
/// [`GONE`].
fn patience() -> Duration {
  let seconds = env::var("FURB_CLAUDE_GONE").ok().and_then(|one| one.parse::<f64>().ok());
  seconds.and_then(|one| Duration::try_from_secs_f64(one).ok()).unwrap_or(GONE)
}

#[cfg(all(test, unix))]
#[path = "mod.test.rs"]
pub(super) mod test;
