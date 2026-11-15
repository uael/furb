//! The client of furb with no drawing: what each machine told a client, in what order its cards
//! stand, each life that the client holds with how far its open stands, and what it sends: each
//! message in one queue, and each verb that is due. It does no input and no output: a client gives
//! it each frame that a machine sends and sends each verb that is due, so a window, a phone or a
//! test drives the machines the same way.

mod machine;
mod opens;
mod outbox;

use std::collections::{HashMap, VecDeque};

use furb_wire::{Part, protocol::Card};

pub use machine::{Heard, Key, Link, Listing, Told};
pub use opens::{Ask, Hold, Life, Step};
pub use outbox::{Asked, Outgoing, Target};

/// What each machine told, the cards of all of them in the order of the list as it last settled,
/// each with the part that it had then and with its state as its machine last told it, each life
/// that the client holds, the messages that wait to be sent, the first one first, and each request
/// that waits for its answer, by its machine and its id.
#[derive(Debug, Default)]
pub struct Client {
  pub machines: Vec<Told>,
  pub settled: Vec<(Part, Key, Card)>,
  pub holds: HashMap<Life, Hold>,
  pub outgoing: VecDeque<Outgoing>,
  pub asked: HashMap<(usize, u64), Asked>,
}

impl Client {
  /// A client of the machines, with what each told before.
  pub fn new(machines: Vec<Told>) -> Self {
    Self { machines, ..Self::default() }
  }
}
