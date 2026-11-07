//! furb, the command line: the TUI, and one command of the operator on one life.
//!
//! With no command, furb hands the terminal to the TUI. `prompt`, `turns`, `run` and
//! `skills` each open one life and print what it came to, and `facts` prints a record as
//! it is. Every life runs on the engine of the crate, on the ears of the World that the
//! crate writes, the extensions among them, and on the provider of the model of its default
//! actor and of the models that `--roster` names, or of every model that the
//! catalog of the crate offers when `--roster` names none.

mod console;
mod error;
mod host;
mod life;
mod tui;

use std::{
  collections::HashMap,
  fmt::Display,
  io::{self, Write},
  path::{self, Path, PathBuf},
  process::ExitCode,
  time::Duration,
};

use clap::{Args, Parser, Subcommand};
use furb::{checkpoint, life::Opening, value::entry, verbs, world};
use furb_host::GRACE;
use tokio::runtime;
use tokio::task;

use crate::{error::CliError, life::Life};

/// What the command line takes, whose help says the description of the package.
#[derive(Parser)]
#[command(name = "furb", version, about, args_conflicts_with_subcommands = true)]
struct Furb {
  /// Open the TUI on its demo session, which asks no model.
  #[arg(long)]
  demo: bool,
  #[command(flatten)]
  place: Place,
  #[command(flatten)]
  stand: Stand,
  /// More words for the TUI, after --, such as --effort high.
  #[arg(last = true, value_name = "TUI WORDS")]
  more: Vec<String>,
  #[command(subcommand)]
  command: Option<Command>,
}

/// The record a life keeps and resumes from, and the directory its chains start in.
#[derive(Args)]
struct Place {
  /// The record to keep, and to resume from: a database of SQLite when its name ends in `.sqlite`, `.sqlite3` or
  /// `.db`, and else a file of lines.
  #[arg(long)]
  record: Option<PathBuf>,
  /// The directory the chains of the life start in, the current directory when unsaid.
  #[arg(long)]
  cwd: Option<PathBuf>,
}

/// What a life stands on: the actor a prompt goes to when it names none, and the models it offers beside the model
/// of that actor. A life that names no model in its roster offers every model that the catalog offers, and a life
/// that names some offers those and no more, since the standing of every chain tells the roster.
#[derive(Args, Default)]
struct Stand {
  /// The actor a prompt goes to when it names none, as provider:model/effort; when unsaid, a model of the claude
  /// command line, then of the network, then of the local server that `FURB_LOCAL_URL` names.
  #[arg(long)]
  model: Option<String>,
  /// A model the life offers beside the model of that actor, as provider:model; say it again for each model. When
  /// unsaid, the life offers every model of the catalog.
  #[arg(long)]
  roster: Vec<String>,
}

/// One command of the operator, on one life.
#[derive(Subcommand)]
enum Command {
  /// Prompt an actor on the root chain and print what the prompt gave.
  Prompt {
    /// The message, which the actor reads.
    message: String,
    /// The actor, as provider:model/effort, or as an id that one model alone holds, which for a model of the local
    /// server holds a colon; the default actor of the chain
    /// when unsaid.
    #[arg(long, default_value = "", hide_default_value = true)]
    to: String,
    /// The shape of the response, as python names it. A model answers any shape, and the console refuses a shape
    /// that the operator does not answer.
    #[arg(long, default_value = life::SHAPE)]
    shape: String,
    #[command(flatten)]
    place: Place,
    #[command(flatten)]
    stand: Stand,
  },
  /// Print the turns of the root of a life made again from its record, which only inspects it.
  Turns {
    /// The record to read.
    #[arg(long)]
    record: PathBuf,
    /// The directory the chains of the life start in, the current directory when unsaid.
    #[arg(long)]
    cwd: Option<PathBuf>,
    #[command(flatten)]
    stand: Stand,
  },
  /// Print each fact of a record after the number of its line, as python shows it, which opens no life and reads
  /// the record as it is.
  Facts {
    /// The record to read.
    #[arg(long)]
    record: PathBuf,
    /// Print only the facts about this act, as rung3 or bash1.
    #[arg(long)]
    act: Option<String>,
    /// Print only the facts on this chain, as chain1: the acts on it and the facts about them.
    #[arg(long)]
    chain: Option<String>,
  },
  /// Run a word the caller wrote on the root chain and print what it gave. It wakes no work that an earlier life
  /// left pending, and names that work on stderr.
  Run {
    /// The python of the word.
    word: String,
    #[command(flatten)]
    place: Place,
    #[command(flatten)]
    stand: Stand,
  },
  /// Write at the tip of a record the checkpoint of its life, from which a life of any later build goes on: each chain
  /// that lives, with its turns, its module and its open prompts. It replays the record and asks no model.
  Checkpoint {
    /// The record to write the checkpoint at.
    #[arg(long)]
    record: PathBuf,
    /// The directory the chains of the life start in, the current directory when unsaid.
    #[arg(long)]
    cwd: Option<PathBuf>,
  },
  /// Print where the models of this machine came from: for models.dev and for pi-ai, the version, the time of the last
  /// good fetch, and the error of the last failed one. A running life uses a new catalog at its next stand.
  Catalog {
    /// Fetch both sources now. A source that is offline or gives invalid data leaves the last valid catalog in place.
    #[arg(long)]
    update: bool,
  },
  /// Serve the daemon of this machine, which holds each life by its record while a window or a pin
  /// holds it, and refuses to start where a daemon serves already.
  Host {
    /// How many seconds a life that no window and no pin holds lives before it writes its
    /// checkpoint and ends.
    #[arg(long, value_name = "SECONDS", default_value_t = GRACE.as_secs_f64(), value_parser = grace)]
    grace: f64,
    #[command(subcommand)]
    then: Option<Hosting>,
  },
  /// Print each skill that a life on the record runs: those it enables, then those that the
  /// configs of the user
  /// and of the directory turn on, each with its life word.
  Skills {
    #[command(flatten)]
    place: Place,
  },
}

/// A grace in seconds, which is a duration.
fn grace(given: &str) -> Result<f64, CliError> {
  let seconds: f64 =
    given.parse().map_err(|source| CliError::Seconds { given: given.to_owned(), source })?;
  Duration::try_from_secs_f64(seconds)
    .map(|_| seconds)
    .map_err(|source| CliError::Grace { given: given.to_owned(), source })
}

/// What `furb host` does beside serving.
#[derive(Subcommand)]
enum Hosting {
  /// Serve in the place of the daemon that runs: it writes the checkpoint of each life and gives
  /// them to this one, and each window connects again.
  Upgrade,
}

impl Place {
  /// A life in the directory of this place, on its record, which it keeps what it says to when `keeps` says so.
  fn opening(&self, keeps: bool) -> Opening {
    let cwd = self.cwd.clone().unwrap_or_else(|| PathBuf::from("."));
    let directory = path::absolute(&cwd).unwrap_or(cwd).display().to_string();
    let record = self.record.as_ref().map(|one| one.display().to_string());
    Opening { directory, record, keeps: Some(keeps), ..Opening::default() }
  }
}

impl Stand {
  /// A life that stands on the actor and the roster of this stand.
  fn on(&self, opening: Opening) -> Opening {
    let roster = (!self.roster.is_empty()).then(|| self.roster.clone());
    Opening { actor: self.model.clone(), roster, ..opening }
  }
}

fn main() -> ExitCode {
  let furb = Furb::parse();
  world::catalog::keep_fresh();
  match commanded(furb) {
    Ok(()) | Err(CliError::Closed) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("furb: {why}");
      ExitCode::FAILURE
    }
  }
}

/// One line on stdout. A reader that closes the pipe early, as `head` does, ends the command
/// with no word.
fn say(line: impl Display) -> Result<(), CliError> {
  writeln!(io::stdout().lock(), "{line}").map_err(|why| match why.kind() {
    io::ErrorKind::BrokenPipe => CliError::Closed,
    _ => CliError::Printed(why),
  })
}

/// The command, run on the one runtime of the program. The runtime stops with the command and
/// waits for no read of stdin that the console left.
#[expect(
  clippy::disallowed_methods,
  reason = "the program blocks on the runtime of its command, which is all that it runs"
)]
fn commanded(furb: Furb) -> Result<(), CliError> {
  let runtime =
    runtime::Builder::new_multi_thread().enable_all().build().map_err(CliError::Runtime)?;
  let done = runtime.block_on(command(furb));
  runtime.shutdown_background();
  done
}

/// The command that the operator gave, or the TUI when he gave none.
async fn command(furb: Furb) -> Result<(), CliError> {
  match furb.command {
    Some(Command::Prompt { message, to, shape, place, mut stand }) => {
      // The actor of the prompt is one the life offers, which a roster that names models must
      // name too.
      if !stand.roster.is_empty() {
        stand.roster.extend(life::actor(&to).await.model);
      }
      prompt(stand.on(place.opening(true)), &shape, message, &to).await
    }
    Some(Command::Turns { record, cwd, stand }) => {
      let place = Place { record: Some(record), cwd };
      turns(stand.on(Opening { inspecting: Some(true), ..place.opening(false) })).await
    }
    Some(Command::Facts { record, act, chain }) => facts(&record, act.as_deref(), chain.as_deref()),
    Some(Command::Run { word, place, stand }) => run(stand.on(place.opening(true)), word).await,
    Some(Command::Skills { place }) => skills(place.opening(false)).await,
    Some(Command::Catalog { update }) => catalog(update).await,
    Some(Command::Host { grace, then }) => {
      host::serve(Duration::from_secs_f64(grace), matches!(then, Some(Hosting::Upgrade))).await
    }
    Some(Command::Checkpoint { record, cwd }) => {
      checkpoint(Place { record: Some(record.clone()), cwd }.opening(false), &record).await
    }
    None => {
      tui::launch(furb.demo, &furb.place, &furb.stand, &furb.more).await.map(|never| match never {})
    }
  }
}

/// Where the models of this machine came from, after an update when the operator asks for one.
async fn catalog(update: bool) -> Result<(), CliError> {
  let status = if update {
    // The update waits for the network on a thread that may block.
    task::spawn_blocking(world::catalog::updated).await.map_err(CliError::Updating)?
  } else {
    world::catalog::status()
  };
  let mut failed = false;
  for (name, one) in [("models.dev", &status.models_dev), ("pi-ai", &status.pi_ai)] {
    let (version, fetched) =
      (one.version.as_deref().unwrap_or("-"), one.fetched.as_deref().unwrap_or("never"));
    say(format_args!("{name}: version {version}, fetched {fetched}"))?;
    if let Some(why) = &one.error {
      say(format_args!("  error: {why}"))?;
      failed = true;
    }
  }
  if update && failed { Err(CliError::Update) } else { Ok(()) }
}

/// A prompt of the operator on the root of a life, awaited for the shape it asks for, and what it came to, as python
/// shows it.
///
/// A prompt the record already holds is taken up and never asked twice, so a command said again on a kept record
/// reads the answer of the life before it and asks no model for it. A prompt it holds that is not
/// done goes on: the open resumes it, and the command wakes it only when a pause holds it, so it
/// is woken once.
async fn prompt(opening: Opening, shape: &str, message: String, to: &str) -> Result<(), CliError> {
  let mut life = Life::lived(opening).await?;
  let to = life::actor(to).await.name;
  let id = if let Some(id) = life.again(shape, &message, &to) {
    if life.paused(&id)? {
      life.engine.wake(&id)?;
    }
    id
  } else {
    let with = verbs::Prompt { message: Some(message), to: Some(to), on: Some(life.root.clone()) };
    life.engine.prompt(world::Shape::object(shape), with)?.id().to_owned()
  };
  say(life.settled(&id).await?.py_repr())?;
  Ok(())
}

/// The turns of the root of a life made again from its record, each as the python a model reads of it.
async fn turns(opening: Opening) -> Result<(), CliError> {
  let mut life = Life::lived(opening).await?;
  let on = Some(life.root.clone());
  for turn in life.engine.turns(verbs::Turns { on })? {
    let text =
      |at: usize| entry(&turn.as_ref(), at).and_then(|one| one.as_str()).unwrap_or_default();
    say(format_args!("[{}] {}", text(0), text(1)))?;
  }
  Ok(())
}

/// Each fact of a record after the number of its line, as python shows it: those about an act, those on a chain, or
/// all of them. A fact is on the chain of the act it is about, and a chain is on itself.
fn facts(record: &Path, act: Option<&str>, chain: Option<&str>) -> Result<(), CliError> {
  if !record.is_file() {
    return Err(CliError::NoRecord(record.to_owned()));
  }
  let mut scopes = HashMap::<String, String>::new();
  for (at, kept) in world::kept(record).map_err(CliError::Unread)?.iter().enumerate() {
    let Some(fact) = entry(&kept.as_ref(), 0) else { continue };
    let word = |n: usize| {
      entry(&fact, n).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
    };
    let (kind, about) = (word(0), word(1));
    let number = about.strip_prefix(kind.as_str()).unwrap_or_default();
    if !number.is_empty() && number.bytes().all(|one| one.is_ascii_digit()) {
      scopes.insert(about.clone(), if kind == "chain" { about.clone() } else { word(3) });
    }
    let on = scopes.get(&about).map_or("", String::as_str);
    if act.is_some_and(|one| one != about) || chain.is_some_and(|one| one != on) {
      continue;
    }
    say(format_args!("{}: {}", at + 1, fact.py_repr()))?;
  }
  Ok(())
}

/// One word its caller wrote, run as a rung on the root of a life, awaited for what the word gave, as python shows it.
async fn run(opening: Opening, word: String) -> Result<(), CliError> {
  let mut life = Life::lived(opening).await?;
  let with = verbs::Rung { word: Some(word), on: Some(life.root.clone()), ..Default::default() };
  let id = life.engine.rung(with)?.id().to_owned();
  say(life.settled(&id).await?.py_repr())?;
  Ok(())
}

/// The checkpoint of the life of a record, made in a life that only inspects it and kept at its tip.
async fn checkpoint(opening: Opening, record: &Path) -> Result<(), CliError> {
  let (mut engine, _) = Opening { inspecting: Some(true), ..opening }.open::<String>([]).await?;
  let written = checkpoint::written(&mut engine)?;
  drop(engine);
  Ok(checkpoint::keep(record, written)?)
}

/// Each skill that a life on the record runs, as its name and its life word, or its name alone
/// when it has none,
/// one on each line.
async fn skills(opening: Opening) -> Result<(), CliError> {
  let mut life = Life::lived(opening).await?;
  for one in life.skills()? {
    match one.life.as_str() {
      "" => say(&one.name)?,
      life => say(format_args!("{}: {life}", one.name))?,
    }
  }
  Ok(())
}
