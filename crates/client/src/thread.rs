//! A thread that a client reads: the exchanges that the daemon gave of it, page by page, and the
//! messages of the operator that it shows before the daemon tells them.

use std::{
  collections::{BTreeMap, HashMap},
  time::Duration,
};

use furb_wire::{
  Agent, Context, Exchange, OPERATOR, Page,
  protocol::{Card, Verb},
};

use crate::{Asked, Client, Key};

/// How many exchanges a page holds.
pub const PAGE: usize = 20;

/// A thread that the client reads: its card, the exchanges that the daemon gave, how many stand
/// before them, its agents, each page that it asked, the messages of the operator that it does not
/// show yet, and the context of its chain.
#[derive(Debug, Default)]
pub struct Thread {
  pub key: Key,
  pub exchanges: BTreeMap<usize, Exchange>,
  /// How many exchanges stand before the first that the client holds, and none before the first
  /// page came.
  pub above: Option<usize>,
  pub agents: Vec<Agent>,
  /// Each page that the client asked and that did not come, by the id of its request: the place
  /// of the exchange that it stands before, or none for the page of the end.
  pub asked: HashMap<u64, Option<usize>>,
  /// Why the daemon gave no page.
  pub fault: Option<String>,
  /// The messages of the operator that the thread does not show yet, in their order.
  pub pending: Vec<Pending>,
  /// Whether the client started the thread with a message of the operator.
  pub started: bool,
  /// When the client opened the thread, on the clock of its loaders.
  pub since: Duration,
  /// Whether the client asks the page of the end again, since the connection to its machine ended
  /// and the new one tells no change of the thread until the client reads its end there.
  pub again: bool,
  /// The context of the chain of the thread at its last answer.
  pub context: Option<Context>,
}

/// A message of the operator that the thread shows before the daemon tells it: its text, and the
/// act that it made once the daemon said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
  pub act: Option<String>,
  pub text: String,
}

impl Thread {
  /// A page that the daemon gave, or a change of the thread that it told: each exchange takes the
  /// place of the one before it, and the agents and the context are those of the thread now.
  pub fn took(&mut self, page: Page) {
    self.above = Some(self.above.map_or(page.above, |above| above.min(page.above)));
    (self.agents, self.context) = (page.agents, page.context);
    self.fault = None;
    self.exchanges.extend(page.exchanges.into_iter().map(|one| (one.at, one)));
    self.shown();
  }

  /// The messages of the operator that the thread shows now let go.
  pub fn shown(&mut self) {
    let told = |act: &String| self.exchanges.values().any(|one| one.message == *act);
    self.pending.retain(|one| !one.act.as_ref().is_some_and(told));
  }

  /// The page of the thread before the exchange at a place, or the page of its end.
  pub fn page(&self, before: Option<usize>) -> Verb {
    let (life, thread) = (self.key.1.clone(), self.key.2.clone());
    Verb::Thread { life, thread, before, limit: PAGE }
  }
}

impl Client {
  /// The page of the end of a thread that the operator reads, when the client holds none of it and
  /// asked none, once the daemon answered the open of its life. The daemon holds it until that
  /// life stands.
  pub fn page_due(&self, thread: &Thread) -> Option<Verb> {
    let held =
      thread.above.is_some() && !thread.again || !thread.asked.is_empty() || thread.fault.is_some();
    let reads = self.reads(thread.key.0, &thread.key.1);
    (!thread.key.2.is_empty() && !held && reads).then(|| thread.page(None))
  }

  /// What the operator saw of a thread whose end he reads, with its machine: the card says that
  /// the thread ended and that he did not read it. The card shows him as a reader at once, as the
  /// daemon will, so the thread that he reads never shows as unread, and the client says it once
  /// each time the thread ends. A page with no exchange shows him nothing to see, and a life that
  /// a fault ended takes nothing, this neither.
  pub fn seen_due(&mut self, thread: &Thread) -> Option<(usize, Verb)> {
    if thread.above.is_none() || thread.exchanges.is_empty() {
      return None;
    }
    let key = &thread.key;
    if self.fault_of(key.0, &key.1).is_some() {
      return None;
    }
    let login = self.machines[key.0].login.clone();
    // A card that the daemon told before it read the last Seen says unread once more.
    let saying = self.asked.iter().any(|(asked, what)| asked.0 == key.0 && *what == Asked::Seen);
    if saying || !self.card(key)?.thread.unread(&login) {
      return None;
    }
    let reader = format!("{OPERATOR}/{login}");
    let told = |card: &Card| card.life == key.1 && card.thread.prompt == key.2;
    let settled = self.settled.iter_mut().map(|one| &mut one.2);
    for card in self.machines[key.0].cards.iter_mut().chain(settled).filter(|card| told(card)) {
      card.thread.readers.push(reader.clone());
    }
    let (life, about) = (key.1.clone(), key.2.clone());
    Some((key.0, Verb::Seen { life, about }))
  }
}
