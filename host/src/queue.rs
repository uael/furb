//! The follow-ups that the operator queued on the chains of a life: the queue of each chain, folded
//! from the facts `queue` of the operator in the order of the log, so a reopen of the record finds
//! the queue as it stood. The life sends the first follow-up of a chain as a prompt once the chain
//! rests: no pause holds it, it asks the operator nothing, and no work of it lives.

use std::collections::{BTreeMap, HashMap};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
  event::Event,
  view::{Act, Kind, State},
};

/// The kind of the facts of the queue, which the operator says on the chain of the follow-up.
pub(crate) const QUEUE: &str = "queue";

/// The site of the operator, who says each fact of the queue.
const OPERATOR: &str = "operator";

/// One follow-up that waits on a chain: its entry, which the window that queued it chose, the shape
/// that its prompt wants, its message, and why its send failed, when it did. A follow-up that
/// failed holds the queue of its chain until the operator takes it out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FollowUp {
  pub entry: String,
  pub chain: String,
  pub shape: String,
  pub message: String,
  pub failed: Option<String>,
}

/// Where a follow-up stands: it waits, or it went as the act of a prompt, which is unknown between
/// the fact that begins its send and the prompt that the operator says after it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
  Waits,
  Sending,
  Sent(Option<String>),
  Dropped,
}

/// A follow-up and where it stands.
#[derive(Debug)]
struct Entry {
  follow: FollowUp,
  step: Step,
}

/// What the queue knows of a chain from its act: the state of its work, whether a pause holds it,
/// and the prompt of the last follow-up that it sent while that prompt lives.
#[derive(Debug, Default)]
struct Chain {
  state: Option<State>,
  paused: bool,
  sent: Option<String>,
}

/// The queue of each chain of a life.
#[derive(Debug, Default)]
pub(crate) struct Queue {
  entries: HashMap<String, Entry>,
  /// The entries of each chain, in the order that the operator queued them.
  order: BTreeMap<String, Vec<String>>,
  chains: HashMap<String, Chain>,
  /// The entry whose send began with the last fact of the queue, until the prompt after it.
  begun: Option<String>,
}

impl Queue {
  /// The follow-ups that wait on a chain, in order.
  pub(crate) fn waiting(&self, chain: &str) -> Vec<FollowUp> {
    let order = self.order.get(chain).map_or(&[][..], Vec::as_slice);
    let entries = order.iter().filter_map(|entry| self.entries.get(entry));
    entries.filter(|one| one.step == Step::Waits).map(|one| one.follow.clone()).collect()
  }

  /// Whether the queue holds an entry, in any step: a second queue of it changes nothing.
  pub(crate) fn holds(&self, entry: &str) -> bool {
    self.entries.contains_key(entry)
  }

  /// The follow-up that an operator may take out of its queue: one that waits.
  pub(crate) fn takes(&self, entry: &str) -> Option<&FollowUp> {
    self.entries.get(entry).filter(|one| one.step == Step::Waits).map(|one| &one.follow)
  }

  /// The first follow-up of each chain that rests, which the life sends now.
  pub(crate) fn due(&self) -> Vec<FollowUp> {
    let rests = |chain: &str| self.chains.get(chain).is_some_and(Chain::rests);
    let firsts = self.order.keys().filter(|chain| rests(chain));
    let firsts = firsts.filter_map(|chain| self.waiting(chain).into_iter().next());
    firsts.filter(|follow| follow.failed.is_none()).collect()
  }

  /// A send of a follow-up that the life made: the prompt that it went as, or why it failed. The
  /// facts of the send say it again later, as the log tells them.
  pub(crate) fn sent(&mut self, entry: &str, made: Result<String, String>) {
    let Some(one) = self.entries.get_mut(entry) else { return };
    match made {
      Ok(act) => {
        one.step = Step::Sent(Some(act.clone()));
        self.chains.entry(one.follow.chain.clone()).or_default().sent = Some(act);
      }
      Err(why) => one.follow.failed = Some(why),
    }
  }

  /// What an event of the life changes in the queue, and the event `queued` of each chain whose
  /// follow-ups that wait it changed.
  pub(crate) fn heard(&mut self, event: &Event) -> Option<Event> {
    match event {
      Event::Fact(fact) => self.fact(fact),
      Event::Act(act) => {
        self.act(act);
        None
      }
      _ => None,
    }
  }

  /// A fact: a fact of the queue of the operator changes the follow-ups of its chain, the first
  /// prompt of the operator after the fact that begins a send is the act of that send, and the end
  /// of that prompt lets the next follow-up of its chain go. A record before its checkpoint tells
  /// its facts alone, with no act, so the end is read from its fact.
  fn fact(&mut self, fact: &Value) -> Option<Event> {
    let text = |at: usize| fact.get(at).and_then(Value::as_str);
    if text(0) == Some("done") {
      self.ended(text(1).unwrap_or_default());
    }
    if text(2) != Some(OPERATOR) {
      return None;
    }
    let begun = self.begun.take();
    if text(0) == Some("prompt") {
      let one = self.entries.get_mut(&begun?)?;
      one.step = Step::Sent(text(1).map(str::to_owned));
      let chain = one.follow.chain.clone();
      return Some(self.queued(&chain));
    }
    let (Some(QUEUE), Some(chain), Some(step), Some(entry)) = (text(0), text(1), text(3), text(4))
    else {
      return None;
    };
    let words = (text(5).map(str::to_owned), text(6).map(str::to_owned));
    self.step(chain, step, entry, words)
  }

  /// One step of a follow-up, with the words of its fact after its entry.
  fn step(
    &mut self,
    chain: &str,
    step: &str,
    entry: &str,
    words: (Option<String>, Option<String>),
  ) -> Option<Event> {
    match (step, words) {
      ("add", (Some(shape), Some(message))) if !self.entries.contains_key(entry) => {
        let follow = FollowUp {
          entry: entry.to_owned(),
          chain: chain.to_owned(),
          shape,
          message,
          failed: None,
        };
        self.entries.insert(entry.to_owned(), Entry { follow, step: Step::Waits });
        self.order.entry(chain.to_owned()).or_default().push(entry.to_owned());
      }
      ("drop", _) => self.moved(entry, Step::Dropped)?,
      ("begin", _) => {
        self.begun = Some(entry.to_owned());
        self.moved(entry, Step::Sending)?;
      }
      ("sent", (act, _)) => self.moved(entry, Step::Sent(act))?,
      ("failed", (why, _)) => {
        let one = self.entries.get_mut(entry)?;
        one.step = Step::Waits;
        one.follow.failed = why;
      }
      _ => return None,
    }
    Some(self.queued(chain))
  }

  /// A follow-up moved to a step, once it waits or its send has no act yet, and nothing for an
  /// entry that the queue does not hold.
  fn moved(&mut self, entry: &str, step: Step) -> Option<()> {
    let one = self.entries.get_mut(entry)?;
    if !matches!(one.step, Step::Sent(Some(_)) | Step::Dropped) {
      one.step = step;
    }
    Some(())
  }

  /// The act of a chain, which says the state of its work and its pause.
  fn act(&mut self, act: &Act) {
    if act.kind == Kind::Chain {
      let chain = self.chains.entry(act.id.clone()).or_default();
      chain.state = Some(act.state);
      chain.paused = act.pause.is_some();
    }
  }

  /// The end of an act, which lets the next follow-up go when it is the prompt of the last one.
  fn ended(&mut self, id: &str) {
    for chain in self.chains.values_mut() {
      if chain.sent.as_deref() == Some(id) {
        chain.sent = None;
      }
    }
  }

  /// The event that tells the follow-ups that wait on a chain.
  fn queued(&self, chain: &str) -> Event {
    Event::Queued { chain: chain.to_owned(), waiting: self.waiting(chain) }
  }
}

impl Chain {
  /// Whether a chain rests: no pause holds it, it asks the operator nothing, no work of it lives,
  /// and the prompt of its last follow-up is done.
  fn rests(&self) -> bool {
    let Some(state) = self.state else { return false };
    !self.paused
      && self.sent.is_none()
      && !state.busy()
      && !matches!(state, State::Asks | State::Paused)
  }
}
