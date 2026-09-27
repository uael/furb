//! The core of an AI harness: the engine in a sandbox, behind an async API, on ears of your own.
//!
//! The engine is one python file, `src/furb/engine.py`, which this crate runs inside monty, a python interpreter
//! written in rust for untrusted code. [`Engine`] is one life of it: its methods are the verbs of the contract, made
//! from the contract when the crate is built, and an act is awaited as an [`Act`]. [`Ear`] is what the engine hears
//! by, a coroutine that [`ear::ear`] makes, and [`world`] holds the ears the crate writes: files, commands, time,
//! the store of the record, and the provider of models. [`extension`] says what an extension is and plays the ones a
//! life runs as rungs, and [`life::Opening`] opens a life on the record and on the ears of the crate, to which a host
//! adds its own. The Kernel and the gate are the crate's, and [`PREAMBLE`] stands in for the ears of the host inside
//! the sandbox. [`wire`] carries a value as JSON, as the record keeps it.

pub mod ear;
pub mod engine;
pub mod extension;
pub mod fact;
pub mod gate;
pub mod life;
mod sand;
pub mod value;
pub mod wire;
pub mod world;

pub mod binding;

pub use crate::{
  ear::{Ear, Heard, Step, Voice},
  engine::{Act, Engine, Plain, verbs},
  fact::Fact,
  value::{Exit, Fault, Object, ObjectRef, Text},
};

/// The engine: the one file the sandbox runs, and the whole system prompt of a model.
///
/// The crate carries the same file the python package ships, so the engine a host runs and the engine a model
/// reads are one thing.
pub const ENGINE: &str = include_str!("furb/engine.py");

/// The system prompt of every model: the engine minified in layout alone, and nothing else.
///
/// The build makes it from [`ENGINE`], and stops unless it is the same program.
pub const SYSTEM: &str = include_str!(concat!(env!("OUT_DIR"), "/system.py"));

/// The sheet the gate reads a word on, which the Kernel of the sandbox writes the same way the python package does.
pub const SHEET: &str = include_str!("furb/sheet.py");

/// The Kernel, which the sandbox loads in a module of its own, as the python package imports it.
pub const KERNEL: &str = include_str!("furb/kernel.py");

/// The stand-in, which runs in the sandbox in a module of its own.
pub const PREAMBLE: &str = include_str!("preamble.py");

#[cfg(test)]
mod tests {
  #[test]
  fn the_stand_in_carries_the_one_mark_of_the_crossing() {
    assert!(super::PREAMBLE.contains(&format!("IS = {:?}", super::value::IS)));
  }

  #[test]
  fn the_system_prompt_is_the_engine_in_a_layout_of_its_own() {
    let lines = super::SYSTEM.lines().collect::<Vec<_>>();
    assert!(super::SYSTEM.len() < super::ENGINE.len());
    assert!(lines.iter().all(|one| !one.is_empty() && !one.starts_with(' ')));
    assert!(lines.contains(&"def pause(id:str)->None:control(\"pause\",\"paused\",id)"));
  }
}
