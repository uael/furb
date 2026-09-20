//! The core of an AI harness, with the engine in a sandbox and a World of your own.
//!
//! The engine is one python file, `src/furb/engine.py`, which the contract beside it specifies. This crate runs
//! that file inside a sandbox and gives its surface to a host that is not python: the verbs it calls, the World it
//! implements, and the two bindings, one for typescript and one for python.
//!
//! What the crate takes care of is the Kernel. The engine has the Kernel gate and run the word of every rung, and
//! that word is written by a model, so it is the one part of a life that must not be trusted. The word runs in the
//! sandbox where the engine itself runs, in the module of its chain, and it is gated before it runs.
//!
//! Because the engine runs in there too, nothing of the engine crosses to the host: a text, an act and a refusal
//! are the sandbox's own, the engine calls its own shows and filters in there, and the one thing that crosses is
//! a fact, plain.
//!
//! # How a host reaches the engine
//!
//! The engine takes the World and the Kernel as generators. A host of another language is no generator, so the
//! crate runs [`PREAMBLE`] in a module of its own and hands the engine the two generators that stand in its place.
//! Every fact crosses that boundary as a [`fact::Value`], which is plain data, a shape such as a text, or an
//! exception, and nothing of python. `script/sanded.py` proves the whole of it: it runs the suite of the engine,
//! in this interpreter and unedited, against a life of the crate.
//!
//! Nothing of the preamble is bound in the engine, and nothing of the engine is bound in the preamble, so the
//! globals of a chain hold what the file defines and nothing more, which is what a model reads.
//!
//! # Where it runs
//!
//! The sandbox is monty, and it is [`Sand`]. There is no other and no seam for one: the crate is the engine in
//! that sandbox, so a build of it that could not run the engine would be a build of nothing.

pub mod fact;
#[cfg(feature = "gate")]
pub mod gate;
pub mod host;
pub mod life;
pub mod record;
pub mod sand;
pub mod verb;
pub mod voice;
pub mod world;

pub use crate::sand::Sand;
pub use crate::{
  fact::{Fact, Value},
  host::{Gate, Outside},
  life::{HOST, Host, Life, Refusal},
  record::{Drift, Entry},
  verb::{Act, Exit, Filter, Show, Text, Verb},
  voice::{Ears, Said, Voice},
  world::{Reply, World},
};

/// The engine: the one file the sandbox runs, and the whole system prompt of a model.
///
/// The crate carries the same file the python package ships, so the engine a host runs and the engine a model
/// reads are one thing.
pub const ENGINE: &str = include_str!("furb/engine.py");

/// The contract: the typed surface of the engine, whose docstrings hold every law.
///
/// The gate reads the word of a rung against this, since what a word may say is what the contract declares.
pub const CONTRACT: &str = include_str!("furb/engine.pyi");

/// The boundary of a host that is not python, which the crate runs in a module of its own.
pub const PREAMBLE: &str = include_str!("preamble.py");

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_engine_the_crate_carries_is_the_engine_the_package_ships() {
    let shipped = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/furb/engine.py")).unwrap();
    assert_eq!(ENGINE, shipped);
    assert!(ENGINE.contains("def boot(record=(), **outside):"));
  }

  #[test]
  fn the_preamble_carries_the_two_marks_of_the_plain_form() {
    assert!(PREAMBLE.contains("def outside("));
    assert!(PREAMBLE.contains(&format!("TUPLE = {:?}", fact::TUPLE)));
    assert!(PREAMBLE.contains(&format!("SHOW = {:?}", fact::SHOW)));
  }
}
