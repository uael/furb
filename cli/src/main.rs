//! furb, the command line: the TUI, a JSON-RPC on stdin and stdout, and one command of the operator on one life.
//!
//! With no command, furb hands the terminal to the TUI, or, with `--mode rpc`, serves a client on its stdin and its
//! stdout. `prompt`, `turns`, `run` and `extensions` each open one life and print what it came to. Every life runs on
//! the engine of the crate, on the ears of the World that the crate writes, the extensions among them, and on the
//! provider of the model of its default actor and of the models that `--roster` names, from the catalog of the crate.

mod console;
mod life;
mod rpc;
mod tui;

use std::{path::PathBuf, process::ExitCode};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum, error::ErrorKind};
use furb::{life::Opening, value::entry, verbs, world::SHAPES};

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
  /// The record to keep, and to resume from.
  #[arg(long)]
  record: Option<PathBuf>,
  /// The directory the chains of the life start in, the current directory when unsaid.
  #[arg(long)]
  cwd: Option<PathBuf>,
}

/// What a life stands on: the actor a prompt goes to when it names none, and the models it offers beside the model
/// of that actor. The standing of every chain tells the roster, so a life offers the models it names and no more.
#[derive(Args, Default)]
struct Stand {
  /// The actor a prompt goes to when it names none, as provider:model/effort; the first model the catalog offers when
  /// unsaid.
  #[arg(long)]
  model: Option<String>,
  /// A model the life offers beside the model of that actor, as provider:model; say it again for each model.
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
    /// The actor, as provider:model/effort, or as an id that one model alone holds; the default actor of the chain
    /// when unsaid.
    #[arg(long, default_value = "", hide_default_value = true)]
    to: String,
    /// The shape of the response, as python names it.
    #[arg(long, default_value = "None", value_parser = SHAPES)]
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
  /// Run a word the caller wrote on the root chain and print what it gave.
  Run {
    /// The python of the word.
    word: String,
    #[command(flatten)]
    place: Place,
    #[command(flatten)]
    stand: Stand,
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
    let opening = Opening::new(life::directory(self.cwd.as_deref()));
    match &self.record {
      Some(record) => opening.record(record, keeps),
      None => opening,
    }
  }
}

impl Stand {
  /// A life that stands on the actor and the roster of this stand.
  fn on(&self, opening: Opening) -> Opening {
    let roster = (!self.roster.is_empty()).then(|| self.roster.clone());
    opening.actor(self.model.clone()).roster(roster)
  }
}

fn main() -> ExitCode {
  let furb = Furb::parse();
  let done = match furb.command {
    Some(Command::Prompt { message, to, shape, place, mut stand }) => {
      // The actor of the prompt is one the life offers.
      stand.roster.extend(life::model(&to));
      prompt(stand.on(place.opening(true)), &shape, message, to)
    }
    Some(Command::Turns { record, cwd, stand }) => {
      let opening = Opening::new(life::directory(cwd.as_deref())).record(record, false);
      turns(stand.on(opening.inspecting()))
    }
    Some(Command::Run { word, place, stand }) => run(stand.on(place.opening(true)), word),
    Some(Command::Extensions { place }) => extensions(place.opening(false)),
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

/// A prompt of the operator on the root of a life, awaited for the shape it asks for, and what it came to, as python
/// shows it.
///
/// A prompt the record already holds is taken up and never asked twice, so a command said again on a kept record
/// reads the answer of the life before it and asks no model for it. A prompt it holds that is not done goes on, since
/// the command wakes it, and a wake of a prompt that is done says nothing.
fn prompt(opening: Opening, shape: &str, message: String, to: String) -> Result<(), String> {
  let mut life = Life::lived(opening)?;
  let to = life::actor(&to);
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

/// One word its caller wrote, run as a rung on the root of a life, awaited for what the word gave, as python shows it.
fn run(opening: Opening, word: String) -> Result<(), String> {
  let mut life = Life::lived(opening)?;
  let with = verbs::Rung { word: Some(word), on: Some(life.root.clone()), ..Default::default() };
  let id = life.engine.rung(with).map_err(|no| no.to_string())?.id().to_owned();
  println!("{}", life.settled(&id)?.py_repr());
  Ok(())
}

/// Each extension that a life on the record runs, as its name and its life word, one on each line.
fn extensions(opening: Opening) -> Result<(), String> {
  let mut life = Life::lived(opening)?;
  for one in life.extensions()? {
    println!("{}: {}", one.name, one.life);
  }
  Ok(())
}
