//! furb, the command line: the TUI, and one command of the operator on one life.
//!
//! With no command, furb hands the terminal to the TUI. `prompt`, `turns` and `run` each open one life and print what
//! it came to. Every life runs on the engine of the crate, on the ears of the World that the crate writes, and on the
//! provider of the models of the claude command line, which are the models the crate offers.

mod console;
mod life;
mod tui;

use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand, ValueEnum};
use furb::{Object, value::entry, verbs};

use crate::life::Life;

/// What the command line takes, whose help says the description of the package.
#[derive(Parser)]
#[command(name = "furb", version, about, args_conflicts_with_subcommands = true)]
struct Furb {
  /// Open the TUI on its demo session, which asks no model.
  #[arg(long)]
  demo: bool,
  #[command(flatten)]
  place: Place,
  /// More words for the TUI, after --, such as --model provider:model.
  #[arg(last = true, value_name = "TUI WORDS")]
  more: Vec<String>,
  #[command(subcommand)]
  command: Option<Command>,
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

/// One command of the operator, on one life.
#[derive(Subcommand)]
enum Command {
  /// Prompt an actor on the root chain and print what the prompt gave.
  Prompt {
    /// The message, which the actor reads.
    message: String,
    /// The actor, as model/effort; the default actor of the chain when unsaid.
    #[arg(long, default_value = "", hide_default_value = true)]
    to: String,
    /// The shape of the response.
    #[arg(long, value_enum, default_value_t = Shape::None)]
    shape: Shape,
    #[command(flatten)]
    place: Place,
  },
  /// Print the turns of the root of a life made again from its record.
  Turns {
    /// The record to resume from.
    #[arg(long)]
    record: PathBuf,
    /// The directory the chains of the life start in, the current directory when unsaid.
    #[arg(long)]
    cwd: Option<PathBuf>,
  },
  /// Run a word the caller wrote on the root chain and print what it gave.
  Run {
    /// The python of the word.
    word: String,
    #[command(flatten)]
    place: Place,
  },
}

/// Every shape a prompt of the command line takes, by the name it is given on the line.
#[derive(Clone, Copy, ValueEnum)]
enum Shape {
  None,
  Str,
  Int,
  Float,
  Bool,
  List,
}

impl Shape {
  /// The name of the shape, as a prompt carries it.
  fn name(self) -> &'static str {
    match self {
      Shape::None => "None",
      Shape::Str => "str",
      Shape::Int => "int",
      Shape::Float => "float",
      Shape::Bool => "bool",
      Shape::List => "list",
    }
  }

  /// The shape as a prompt is given it: None itself, or the name of a type.
  fn object(self) -> Object {
    match self {
      Shape::None => Object::none(),
      one => Object::string(one.name()),
    }
  }
}

fn main() -> ExitCode {
  let furb = Furb::parse();
  let done = match furb.command {
    Some(Command::Prompt { message, to, shape, place }) => prompt(&place, shape, message, to),
    Some(Command::Turns { record, cwd }) => turns(record, cwd),
    Some(Command::Run { word, place }) => run(&place, word),
    None => tui::launch(furb.demo, &furb.place, &furb.more).map(|never| match never {}),
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
fn prompt(place: &Place, shape: Shape, message: String, to: String) -> Result<(), String> {
  let mut life = Life::lived(place.record.as_deref(), place.cwd.as_deref(), true)?;
  let root = life.root.clone();
  let id = match life.again(shape.name(), &message, &to) {
    Some(id) => {
      life.engine.wake(&id).map_err(|no| no.to_string())?;
      id
    }
    None => {
      let with = verbs::Prompt { message: Some(message), to: Some(to), on: Some(root) };
      life.engine.prompt(shape.object(), with).map_err(|no| no.to_string())?.id().to_owned()
    }
  };
  println!("{}", life.settled(&id)?.py_repr());
  Ok(())
}

/// The turns of the root of a life made again from its record, each as the python a model reads of it. The life only
/// reads the record, and keeps nothing.
fn turns(record: PathBuf, cwd: Option<PathBuf>) -> Result<(), String> {
  let mut life = Life::lived(Some(&record), cwd.as_deref(), false)?;
  let on = Some(life.root.clone());
  for turn in life.engine.turns(verbs::Turns { on }).map_err(|no| no.to_string())? {
    let text =
      |at: usize| entry(&turn.as_ref(), at).and_then(|one| one.as_str()).unwrap_or_default();
    println!("[{}] {}", text(0), text(1));
  }
  Ok(())
}

/// One word its caller wrote, run as a rung on the root of a life, awaited for what the word gave, as python shows it.
fn run(place: &Place, word: String) -> Result<(), String> {
  let mut life = Life::lived(place.record.as_deref(), place.cwd.as_deref(), true)?;
  let with = verbs::Rung { word: Some(word), on: Some(life.root.clone()), ..Default::default() };
  let id = life.engine.rung(with).map_err(|no| no.to_string())?.id().to_owned();
  println!("{}", life.settled(&id)?.py_repr());
  Ok(())
}
