//! furb, the command line: the TUI, a JSON-RPC on stdin and stdout, and one command of the operator on one life.
//!
//! With no command, furb hands the terminal to the TUI, or, with `--mode rpc`, serves a client on its stdin and its
//! stdout. `prompt`, `turns`, `run` and `extensions` each open one life and print what it came to, and `facts` prints a record
//! as it is. Every life runs on
//! the engine of the crate, on the ears of the World that the crate writes, the extensions among them, and on the
//! provider of the model of its default actor and of the models that `--roster` names, or of every model that the
//! catalog of the crate offers when `--roster` names none.
//!
//! Each run puts the program in the store of builds of the machine under its version, and a command that opens a life
//! on a record of another version runs the build of that version in its place, when the store holds one.

mod console;
mod life;
mod rpc;
mod tui;

use std::{
  collections::HashMap,
  env,
  path::{Path, PathBuf},
  process::{self, ExitCode},
};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum, error::ErrorKind};
use furb::{checkpoint, life::Opening, value::entry, verbs, world};

use crate::life::Life;

/// What the command line takes, whose help says the description of the package.
#[derive(Parser)]
#[command(name = "furb", version, about, args_conflicts_with_subcommands = true)]
struct Furb {
  /// What furb serves with no command: the TUI, or a JSON-RPC on stdin and stdout, which docs/rpc.md describes.
  #[arg(long, value_enum, default_value_t = Mode::Tui)]
  mode: Mode,
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

/// What furb serves with no command.
#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Mode {
  /// The TUI, which takes the terminal.
  Tui,
  /// A JSON-RPC: a command on each line of stdin, and a response or an event on each line of stdout.
  Rpc,
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
  /// command line, then of the network, then of the local server that FURB_LOCAL_URL names.
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
  /// Print each extension that a life on the record runs: those it enables, then those that the configs of the user
  /// and of the directory turn on, each with its life word.
  Extensions {
    #[command(flatten)]
    place: Place,
  },
}

impl Place {
  /// A life in the directory of this place, on its record, which it keeps what it says to when `keeps` says so.
  fn opening(&self, keeps: bool) -> Opening {
    let cwd = self.cwd.clone().unwrap_or_else(|| PathBuf::from("."));
    let directory = std::path::absolute(&cwd).unwrap_or(cwd).display().to_string();
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
  if let Ok(program) = env::current_exe() {
    let _ = world::shelve(&program);
  }
  let furb = Furb::parse();
  if let Some(ended) = handed(&furb) {
    return ended;
  }
  let done = match furb.command {
    Some(Command::Prompt { message, to, shape, place, mut stand }) => {
      // The actor of the prompt is one the life offers, which a roster that names models must name too.
      if !stand.roster.is_empty() {
        stand.roster.extend(life::actor(&to).1);
      }
      prompt(stand.on(place.opening(true)), &shape, message, to)
    }
    Some(Command::Turns { record, cwd, stand }) => {
      let place = Place { record: Some(record), cwd };
      turns(stand.on(Opening { inspecting: Some(true), ..place.opening(false) }))
    }
    Some(Command::Facts { record, act, chain }) => facts(&record, act.as_deref(), chain.as_deref()),
    Some(Command::Run { word, place, stand }) => run(stand.on(place.opening(true)), word),
    Some(Command::Extensions { place }) => extensions(place.opening(false)),
    Some(Command::Checkpoint { record, cwd }) => {
      checkpoint(Place { record: Some(record.clone()), cwd }.opening(false), &record)
    }
    None if furb.mode == Mode::Rpc => {
      if furb.demo || !furb.more.is_empty() {
        let why = "--demo and the words after -- are for the TUI, and --mode rpc serves no TUI";
        Furb::command().error(ErrorKind::ArgumentConflict, why).exit();
      }
      rpc::serve(furb.stand.on(furb.place.opening(true)), furb.place.record.as_deref())
    }
    None => {
      tui::launch(furb.demo, &furb.place, &furb.stand, &furb.more).map(|never| match never {})
    }
  };
  match done {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("furb: {why}");
      ExitCode::FAILURE
    }
  }
}

/// The record of the life that a command opens, and nothing for a command that opens no life on a record.
fn opened(furb: &Furb) -> Option<&Path> {
  match &furb.command {
    Some(
      Command::Prompt { place, .. } | Command::Run { place, .. } | Command::Extensions { place },
    ) => place.record.as_deref(),
    Some(Command::Turns { record, .. } | Command::Checkpoint { record, .. }) => Some(record),
    Some(Command::Facts { .. }) => None,
    None if furb.mode == Mode::Rpc => furb.place.record.as_deref(),
    None => None,
  }
}

/// The directory of the life that a command opens, when the command names one.
fn directory(furb: &Furb) -> Option<&Path> {
  match &furb.command {
    Some(
      Command::Prompt { place, .. } | Command::Run { place, .. } | Command::Extensions { place },
    ) => place.cwd.as_deref(),
    Some(Command::Turns { cwd, .. } | Command::Checkpoint { cwd, .. }) => cwd.as_deref(),
    Some(Command::Facts { .. }) => None,
    None => furb.place.cwd.as_deref(),
  }
}

/// How a record of another version that a command opens goes on, when the store of builds holds the build of that
/// version: only that build replays the record with no drift. A record of an older version is upgraded: its build
/// writes a checkpoint at its tip, and this build goes on from the checkpoint, which this gives as nothing. A record
/// of a newer version, or of an older build that writes no checkpoint, is handed over: that build runs the command in
/// the place of this one with the same words, and this gives how it ended. Nothing runs when the store holds no such
/// build, and the life says why.
fn handed(furb: &Furb) -> Option<ExitCode> {
  let record = opened(furb)?;
  let kept = world::kept(record).ok()?;
  let theirs = world::version(checkpoint::segment(&kept)).filter(|one| *one != world::VERSION)?;
  let program = world::stored(theirs)?;
  let ran = |words: Vec<std::ffi::OsString>| process::Command::new(&program).args(words).status();
  if theirs < world::VERSION {
    let mut words: Vec<std::ffi::OsString> =
      vec!["checkpoint".into(), "--record".into(), record.into()];
    if let Some(cwd) = directory(furb) {
      words.extend(["--cwd".into(), cwd.into()]);
    }
    if ran(words).is_ok_and(|status| status.success()) {
      return None;
    }
    eprintln!(
      "furb: the build of version {theirs} wrote no checkpoint, so it runs the command in the place of this one"
    );
  }
  match ran(env::args_os().skip(1).collect()) {
    Ok(status) => {
      Some(ExitCode::from(status.code().map_or(1, |code| u8::try_from(code).unwrap_or(1))))
    }
    Err(no) => {
      eprintln!("furb: the build of version {theirs} at {} does not run: {no}", program.display());
      Some(ExitCode::FAILURE)
    }
  }
}

/// A prompt of the operator on the root of a life, awaited for the shape it asks for, and what it came to, as python
/// shows it.
///
/// A prompt the record already holds is taken up and never asked twice, so a command said again on a kept record
/// reads the answer of the life before it and asks no model for it. A prompt it holds that is not done goes on, since
/// the command wakes it, and a wake of a prompt that is done says nothing.
fn prompt(opening: Opening, shape: &str, message: String, to: String) -> Result<(), String> {
  let mut life = Life::lived(opening)?;
  let to = life::actor(&to).0;
  let id = match life.again(shape, &message, &to) {
    Some(id) => {
      life.engine.wake(&id).map_err(|no| no.to_string())?;
      id
    }
    None => {
      let with =
        verbs::Prompt { message: Some(message), to: Some(to), on: Some(life.root.clone()) };
      life.engine.prompt(life::shape(shape), with).map_err(|no| no.to_string())?.id().to_owned()
    }
  };
  println!("{}", life.settled(&id)?.py_repr());
  Ok(())
}

/// The turns of the root of a life made again from its record, each as the python a model reads of it.
fn turns(opening: Opening) -> Result<(), String> {
  let mut life = Life::lived(opening)?;
  let on = Some(life.root.clone());
  for turn in life.engine.turns(verbs::Turns { on }).map_err(|no| no.to_string())? {
    let text =
      |at: usize| entry(&turn.as_ref(), at).and_then(|one| one.as_str()).unwrap_or_default();
    println!("[{}] {}", text(0), text(1));
  }
  Ok(())
}

/// Each fact of a record after the number of its line, as python shows it: those about an act, those on a chain, or
/// all of them. A fact is on the chain of the act it is about, and a chain is on itself.
fn facts(record: &Path, act: Option<&str>, chain: Option<&str>) -> Result<(), String> {
  if !record.is_file() {
    return Err(format!("No record at {}.", record.display()));
  }
  let mut scopes = HashMap::<String, String>::new();
  for (at, kept) in furb::world::kept(record).map_err(|no| no.message())?.iter().enumerate() {
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
    println!("{}: {}", at + 1, fact.py_repr());
  }
  Ok(())
}

/// One word its caller wrote, run as a rung on the root of a life, awaited for what the word gave, as python shows it.
fn run(opening: Opening, word: String) -> Result<(), String> {
  let mut life = Life::lived(opening)?;
  let mut pending = Vec::new();
  for (id, _) in life.engine.pending().map_err(|no| no.to_string())? {
    let act = life.engine.get(&id).map_err(|no| no.to_string())?;
    pending.push((id, act.map(|one| one.on().to_owned()).unwrap_or_default()));
  }
  if let Some((_, chain)) = pending.first() {
    let named: Vec<String> = pending.iter().map(|(id, on)| format!("{id} on {on}")).collect();
    eprintln!(
      "furb: an earlier life left {} pending, and it runs again only at a wake: from the word, as wake({chain:?}), \
       from the TUI, or from `furb --mode rpc`.",
      named.join(", ")
    );
  }
  let with = verbs::Rung { word: Some(word), on: Some(life.root.clone()), ..Default::default() };
  let id = life.engine.rung(with).map_err(|no| no.to_string())?.id().to_owned();
  println!("{}", life.settled(&id)?.py_repr());
  Ok(())
}

/// The checkpoint of the life of a record, made in a life that only inspects it and kept at its tip.
fn checkpoint(opening: Opening, record: &Path) -> Result<(), String> {
  let failed = |no: furb::Fault| no.to_string();
  let (mut engine, _) =
    Opening { inspecting: Some(true), ..opening }.boot::<String>([]).map_err(failed)?;
  let written = furb::checkpoint::written(&mut engine).map_err(failed)?;
  drop(engine);
  furb::checkpoint::keep(record, written).map_err(failed)
}

/// Each extension that a life on the record runs, as its name and its life word, or its name alone when it has none,
/// one on each line.
fn extensions(opening: Opening) -> Result<(), String> {
  let mut life = Life::lived(opening)?;
  for one in life.extensions()? {
    match one.life.as_str() {
      "" => println!("{}", one.name),
      life => println!("{}: {life}", one.name),
    }
  }
  Ok(())
}
