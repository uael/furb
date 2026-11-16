//! A thread that a client reads: the exchanges that the daemon gave of it, page by page, and the
//! messages of the operator that it shows before the daemon tells them.

use std::{
  collections::{BTreeMap, HashMap, HashSet},
  mem,
  time::Duration,
};

use furb_wire::{
  Agent, Context, Exchange, Mark, OPERATOR, Page, Saying, Standing, card,
  protocol::{Card, Verb},
};

use crate::{Asked, Client, Form, Key, Parts, Time, time::rewinds};

/// How many exchanges a page holds.
pub const PAGE: usize = 20;

/// The thread that the operator reads: what the client holds of it, where the operator reads it,
/// what he opened, the form of its question that the input answers, and the thread whose agent it
/// is.
#[derive(Debug, Default)]
pub struct Opened {
  pub thread: Thread,
  /// The place at the top of the view while the operator reads away from its end, and none while
  /// the view follows its end.
  pub top: Option<Place>,
  /// The blocks, the thoughts and the whole lines that the operator opened.
  pub unfolded: HashSet<Fold>,
  /// The form of the question of the thread that the input answers.
  pub form: Option<Form>,
  /// The thread whose agent this thread is, which the operator opens again where he read it.
  pub from: Option<Box<Self>>,
  /// The thread of an agent that the operator went back from, which he goes forward to again
  /// until he opens another one.
  pub ahead: Option<Box<Self>>,
  /// The line that the operator went to from a name, by its number, and that name, which the line
  /// binds and shows in the accent.
  pub went: Option<(usize, String)>,
}

impl Opened {
  /// A part opened when it was folded, and folded when it was open.
  pub fn fold(&mut self, fold: Fold) {
    if !self.unfolded.remove(&fold) {
      self.unfolded.insert(fold);
    }
  }

  /// The agents that the dock shows, from the first that does not rest to the last, and how many
  /// stand before them.
  pub fn docked(&self) -> (usize, &[Agent]) {
    let agents = &self.thread.agents;
    let first = agents.iter().position(|agent| !rests(agent)).unwrap_or(agents.len());
    (first, &agents[first..])
  }

  /// The agents that the line of a block numbered so made, or those of the dock that no line made,
  /// from the first that does not rest to the last, and how many of them stand before it. A line
  /// whose agents all rest shows each, as what it did; the dock then shows none.
  pub fn made(&self, at: Option<usize>) -> (usize, Vec<&Agent>) {
    let agents: Vec<&Agent> = self.thread.agents.iter().filter(|one| one.at == at).collect();
    let rested = if at.is_some() { 0 } else { agents.len() };
    let first = agents.iter().position(|agent| !rests(agent)).unwrap_or(rested);
    (first, agents[first..].to_vec())
  }

  /// The card of the thread of an agent, as the dock of the thread that it came from holds it,
  /// which says its title and its state before its own page comes.
  pub fn agent(&self) -> Option<&card::Card> {
    let from = self.from.as_deref()?;
    let agents = from.thread.agents.iter();
    agents.map(|one| &one.card).find(|one| one.prompt == self.thread.key.2)
  }

  /// The sub thread that this thread is, as the thread that it is on holds it: a thread about a
  /// line of a file, which resolves as a thread of the list does.
  pub fn sub(&self) -> Option<&Agent> {
    self.thread.sub.as_deref()
  }

  /// The thread that the operator opened from the list, whose agent this thread is, or this one.
  pub fn root(&self) -> &Self {
    self.from.as_deref().map_or(self, Self::root)
  }
}

/// Whether an agent rests: it ended and asks nothing more. A failure waits for a person.
pub fn rests(agent: &Agent) -> bool {
  matches!(agent.card.standing, Standing::Resolved | Standing::Rests | Standing::Stopped)
}

/// A place in a thread that holds when exchanges come above it: an exchange, and an offset from
/// its start in the units of the view, which is negative for a place above that exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
  pub at: usize,
  pub row: isize,
}

/// What the operator opens or folds in a thread: the block or the thought of the answer to a
/// message, a line of a block whole, by its number in the thread, or the parts of the context.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Fold {
  Block(String),
  Thought(String),
  Line(usize),
  /// The parts of the context.
  Context,
}

/// How long the number of a line that ended settles, from the accent of its work to the grey of a
/// line that ran.
pub const ENDS: Duration = Duration::from_millis(600);

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
  /// The thread itself when it is a sub thread, as the thread that it is on holds it.
  pub sub: Option<Box<Agent>>,
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
  /// The folder of the chain of the thread, where its next word runs.
  pub folder: Option<String>,
  /// The kinds of what an answer of the chain read, once the client parted its turns.
  pub parts: Option<Parts>,
  /// The bar of its time, while the operator holds it.
  pub time: Option<Time>,
  /// Whether the next page takes the place of each exchange held, since the thread moved in time.
  pub replaces: bool,
  /// When each line that ran while the client read it ended, by its number, on the clock of the
  /// loaders: the moment from which its number settles.
  pub ended: BTreeMap<usize, Duration>,
}

/// A message of the operator that the thread shows before the daemon tells it: its text, and the
/// act that it made once the daemon said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
  pub act: Option<String>,
  pub text: String,
}

impl Thread {
  /// What the thread said to the operator last, an answer or a notice, as the model wrote it,
  /// among what the client holds of it.
  pub fn last_said(&self) -> Option<&str> {
    let said = self.exchanges.values().rev().flat_map(|one| one.said.iter().rev());
    let mut told = said.filter(|one| matches!(one.kind, Saying::Answer | Saying::Notice));
    told.find(|one| !one.text.trim().is_empty()).map(|one| one.text.as_str())
  }

  /// Each agent of the thread that a rewind made, in order: a branch of the thread from one of its
  /// moments, whose thread is the second branch, the third, and so on.
  pub fn branches(&self) -> Vec<&Agent> {
    let lines = self.exchanges.values().filter_map(|one| one.block.as_ref());
    let lines = lines.flat_map(|block| &block.lines);
    let words: Vec<usize> =
      lines.filter(|line| rewinds(&line.text)).map(|line| line.number).collect();
    self.agents.iter().filter(|one| one.at.is_some_and(|at| words.contains(&at))).collect()
  }

  /// A page that the daemon gave, or a change of the thread that it told: each exchange takes the
  /// place of the one before it, and the agents and the context are those of the thread now.
  pub fn took(&mut self, page: Page, now: Duration) {
    let marked = |exchanges: &BTreeMap<usize, Exchange>, mark: fn(&Mark) -> bool| -> Vec<usize> {
      let lines = exchanges.values().filter_map(|one| one.block.as_ref());
      let lines = lines.flat_map(|block| &block.lines);
      lines.filter(|line| mark(&line.mark)).map(|line| line.number).collect()
    };
    let ran = marked(&self.exchanges, |mark| matches!(mark, Mark::Runs { .. }));
    if mem::take(&mut self.replaces) {
      (self.exchanges, self.above) = (BTreeMap::new(), None);
    }
    if let Some(time) = self.time.as_mut() {
      time.heard(&page.exchanges);
    }
    self.above = Some(self.above.map_or(page.above, |above| above.min(page.above)));
    (self.agents, self.context, self.sub, self.folder) =
      (page.agents, page.context, page.sub, page.folder);
    self.fault = None;
    self.exchanges.extend(page.exchanges.into_iter().map(|one| (one.at, one)));
    let ended = marked(&self.exchanges, |mark| *mark == Mark::Ran);
    let ended = ended.into_iter().filter(|one| ran.contains(one));
    self.ended.retain(|_, at| now.saturating_sub(*at) < ENDS);
    self.ended.extend(ended.map(|number| (number, now)));
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
    let at = self.time.as_ref().and_then(|time| time.at);
    Verb::Thread { life, thread, before, limit: PAGE, at }
  }

  /// The thread moved in time: it asks its page again, and no page asked before counts.
  pub fn moved(&mut self) {
    (self.again, self.replaces) = (true, true);
    self.asked.clear();
  }

  /// Whether the thread shows a moment of the past, where no change of now reaches it.
  pub fn past(&self) -> bool {
    self.time.as_ref().is_some_and(|time| time.at.is_some())
  }
}

impl Client {
  /// The page of the end of the thread that the operator reads, when the client holds none of it
  /// and asked none, once the daemon answered the open of its life. The daemon holds it until that
  /// life stands.
  pub fn page_due(&self) -> Option<Verb> {
    let thread = &self.thread.as_ref()?.thread;
    let held =
      thread.above.is_some() && !thread.again || !thread.asked.is_empty() || thread.fault.is_some();
    let reads = self.reads(thread.key.0, &thread.key.1);
    (!thread.key.2.is_empty() && !held && reads).then(|| thread.page(None))
  }

  /// What the operator saw of the thread that he reads, once the view follows its end, with its
  /// machine: the card says that the thread ended and that he did not read it. The card shows him
  /// as a reader at once, as the daemon will, so the thread that he reads never shows as unread,
  /// and the client says it once each time the thread ends. A page with no exchange shows him
  /// nothing to see, and a life that a fault ended takes nothing, this neither.
  pub fn seen_due(&mut self) -> Option<(usize, Verb)> {
    let thread = &self.thread.as_ref().filter(|one| one.top.is_none())?.thread;
    if thread.above.is_none() || thread.exchanges.is_empty() {
      return None;
    }
    let key = &thread.key.clone();
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

#[cfg(test)]
#[path = "thread.test.rs"]
mod tests;
