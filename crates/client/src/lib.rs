//! The client of furb with no drawing: what each machine told a client, in what order its cards
//! stand, and each life that the client holds with how far its open stands. It does no input and
//! no output: a client gives it each frame that a machine sends, so a window, a phone or a test
//! reads the machines the same way.

mod machine;
mod opens;

use std::collections::HashMap;

use furb_wire::{Part, protocol::Card};

pub use machine::{Heard, Key, Link, Listing, Told};
pub use opens::{Ask, Hold, Life, Step};

/// What each machine told, the cards of all of them in the order of the list as it last settled,
/// each with the part that it had then and with its state as its machine last told it, and each
/// life that the client holds.
#[derive(Debug, Default)]
pub struct Client {
  pub machines: Vec<Told>,
  pub settled: Vec<(Part, Key, Card)>,
  pub holds: HashMap<Life, Hold>,
}

impl Client {
  /// A client of the machines, with what each told before.
  pub fn new(machines: Vec<Told>) -> Self {
    Self { machines, ..Self::default() }
  }
}
