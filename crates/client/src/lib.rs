//! The client of furb with no drawing: what each machine told a client, in what order its cards
//! stand, each life that the client holds with how far its open stands, and what it sends: each
//! message in one queue, each verb that is due, and the thread that it reads, page by page. It does
//! no input and no output: a client gives it each frame that a machine sends and sends each verb
//! that is due, so a window, a phone or a test drives the machines the same way. It holds the
//! thread that the operator reads and the input, with its target and its history, so each client
//! maps the same state.

mod changes;
mod complete;
mod context;
mod follow;
mod history;
mod input;
mod inspect;
mod machine;
mod matched;
mod opens;
mod outbox;
mod picture;
mod presence;
mod question;
mod running;
mod settings;
mod sign_in;
mod thread;
mod time;
mod trail;
mod views;
mod whole;

use std::{
  collections::{HashMap, VecDeque},
  time::Duration,
};

use furb_wire::{
  Part,
  protocol::{Card, Verb},
};

pub use changes::{Review, Seen};
pub use complete::{Completed, Completes, Completion, Kind, Offer, mentioned};
pub use context::{Content, Parts, Split};
pub use furb_wire::PICTURE;
pub use furb_wire::anchor::{Anchor, Where};
pub use history::{History, Walk};
pub use input::{ASIDE, CD, Draft, FOLD, FOLDED, Folded, Pasted, RENAME, checkout};
pub use inspect::Inspection;
pub use machine::{Heard, Key, Link, Listing, Told};
pub use matched::{Matched, pathed, ranked};
pub use opens::{Ask, Hold, Life, Step};
pub use outbox::{Asked, Chosen, Outgoing, Target, greeting};
pub use picture::{Held, Picture, Shown};
pub use question::{Form, HOLD};
pub use running::{BACKGROUND, Control, Live, Running};
pub use settings::Settings;
pub use sign_in::SignIn;
pub use thread::{ENDS, Fold, Opened, PAGE, Pending, Place, Thread, rests};
pub use time::{Chains, Flow, Time, resting};
pub use whole::{Command, Whole};
pub use trail::Trail;
pub use views::{Filled, View, ViewControl};

/// What each machine told, the cards of all of them in the order of the list as it last settled,
/// each with the part that it had then and with its state as its machine last told it, each life
/// that the client holds, the thread that the operator reads, the input, the messages that wait to
/// be sent, the first one first, and each request that waits for its answer, by its machine and
/// its id.
#[derive(Debug, Default)]
pub struct Client {
  pub machines: Vec<Told>,
  pub settled: Vec<(Part, Key, Card)>,
  /// The card that the operator selected, whose thread he reads.
  pub selected: Option<Key>,
  pub holds: HashMap<Life, Hold>,
  /// The thread that the operator reads.
  pub thread: Option<Opened>,
  /// The places that the operator left, and those that he went back from.
  pub trail: Trail,
  /// The folder of the client, whose project a new session on its own machine starts in.
  pub folder: String,
  /// Where the operator turned the input, away from the open thread.
  pub aim: Option<Target>,
  /// The text of the input, and whether it is a word of python that runs.
  pub draft: Draft,
  /// The text that the input takes after a refusal, a take back, a step of the history or a command
  /// that writes in it, until the client gives it to the editor of its input.
  pub restore: Option<String>,
  /// The drafts that the operator emptied, the last last, which the input takes back.
  pub stash: Vec<String>,
  /// The history of the input, for its target.
  pub history: Option<History>,
  /// The walk through the history of the input, from its first step until he writes or sends.
  pub walk: Option<Walk>,
  /// The completion of the word at the caret of the input, while it completes one.
  pub completion: Option<Completion>,
  /// The commands that a slash at the start of a message offers, which a client gives.
  pub commands: Vec<Offer>,
  /// How the box of the context splits what the last answer of the open thread read.
  pub split: Split,
  /// The system prompt of a chain, which the box of the context counts: the engine of the build of
  /// the window, which a chain on an older engine reads with a few lines that differ.
  pub system: &'static str,
  /// The review of the changes of a project, while the operator reads it.
  pub review: Option<Review>,
  /// The views that the client mounted, in the order that it mounted them.
  pub views: Vec<View>,
  /// What the operator did to the nodes of views, with the machine of each, until it goes: an
  /// event of a view that a skill serves, or the answer of the prompt of a view that a word draws.
  pub gestures: Vec<(usize, Verb, Asked)>,
  /// The input node of a view that the input of the client fills, while it fills one.
  pub filled: Option<Filled>,
  /// The place of the control of the views that the keys reach, among [`Client::controls`], while
  /// the keys reach one.
  pub aimed: Option<usize>,
  /// The settings of a session, while the operator reads them.
  pub settings: Option<Settings>,
  /// The sign-in of Codex, while it runs.
  pub signing: Option<SignIn>,
  /// The name of the code of a thread that the pointer is on, and what its machine told of it.
  pub inspected: Option<Inspection>,
  /// The images that the open thread shows, by machine, life and uri, and where each stands.
  pub pictures: HashMap<Shown, Picture>,
  /// The whole of what each command wrote that the open thread shows open, and where each stands.
  pub wholes: HashMap<Command, Whole>,
  /// The commands whose whole the window shows and the client did not ask yet.
  wanted: Vec<Command>,
  /// The thread whose follow-ups the client asks next, after a change of its queue: its machine,
  /// its life and its prompt.
  following: Option<(usize, String, String)>,
  /// Where the client last told that the operator writes: the machine, the life, and the thread,
  /// none for a new thread of it.
  wrote: Option<(usize, String, Option<String>)>,
  /// What the operator did to sessions that run, with the machine of each, until it goes.
  pub controls: Vec<(usize, Verb)>,
  /// What the last thing that the operator did did not do, and why, until the next one.
  pub notice: Option<String>,
  /// The millisecond of the clock of the machine of the client.
  pub now: u64,
  /// How long the client ran, on the clock of its loaders.
  pub moved: Duration,
  /// When each card last began to call its operator, on the clock of the loaders: a question or a
  /// failure for him, or an end that he did not read. A client draws a pulse on the card there.
  pub called: HashMap<Key, Duration>,
  pub outgoing: VecDeque<Outgoing>,
  /// The actor that the operator chose last, for its target.
  pub chosen: Option<Chosen>,
  pub asked: HashMap<(usize, u64), Asked>,
  /// The id of the next request, which no request of the client took.
  next: u64,
}

impl Client {
  /// A client of the machines, with what each told before, in a folder.
  pub fn new(machines: Vec<Told>, folder: String) -> Self {
    Self { machines, folder, next: outbox::FIRST, ..Self::default() }
  }
}
