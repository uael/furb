//! The keeper of the conversations of the claude command line: one task that lends each
//! conversation to one turn at a time, takes it back, and sweeps the processes and the
//! conversations that no turn needs.

use std::{
  collections::{HashMap, VecDeque},
  sync::Arc,
  time::{Duration, SystemTime},
};

use rig_core::{completion::CompletionError, message::Message};
use tokio::sync::{mpsc, oneshot};

use super::{
  accounts::Accounts,
  conversation::{Conversation, canonical},
  failed,
};

/// How many conversations keep a process at most.
const WARM: usize = 8;

/// How many conversations the command line holds at most.
const HELD: usize = 64;

/// How long a conversation with no turn keeps its process.
const IDLE: Duration = Duration::from_secs(300);

/// What the keeper of the conversations is asked: to lend the conversation of a key to a turn, or
/// to take it back.
pub(super) enum Kept {
  Lend {
    key: String,
    fresh: Box<Conversation>,
    messages: Vec<Message>,
    lent: oneshot::Sender<Lent>,
  },
  Back {
    key: String,
    conversation: Box<Conversation>,
  },
}

/// The conversation of a key as the keeper holds it: at hand, or lent to a turn, with the turns
/// that wait for it.
enum Slot {
  Idle(Box<Conversation>),
  Lent(VecDeque<oneshot::Sender<Lent>>),
}

/// A conversation lent to a turn, which goes back to the keeper however the turn ends. The keeper
/// hands it to the turn as it is, so a turn that ends after the hand and before it takes it, as a
/// reply cancelled while it waits, gives it back too: the hand drops it, and the drop gives it
/// back.
pub(super) struct Lent {
  key: String,
  conversation: Option<Box<Conversation>>,
  keeper: mpsc::WeakUnboundedSender<Kept>,
}

impl Lent {
  #[expect(clippy::expect_used, reason = "debt: an expect where a type can hold the invariant")]
  pub(super) fn conversation(&mut self) -> &mut Conversation {
    self.conversation.as_mut().expect("a lent conversation is held until it goes back")
  }

  /// The conversation, which then goes back to no keeper, as the keeper takes it from a hand that
  /// no turn took.
  fn taken(mut self) -> Option<Box<Conversation>> {
    self.conversation.take()
  }
}

impl Drop for Lent {
  fn drop(&mut self) {
    // A keeper that is gone takes nothing back, and the conversation and its process go here.
    if let Some(conversation) = self.conversation.take()
      && let Some(keeper) = self.keeper.upgrade()
    {
      let key = std::mem::take(&mut self.key);
      drop(keeper.send(Kept::Back { key, conversation }));
    }
  }
}

/// The keeper of the conversations: it lends each to one turn at a time, and the next turn of a
/// conversation lent waits for it; it takes each back, and then sweeps; and it sweeps once in each
/// span that a process may sit idle. It ends when the last clone of the command line goes, and
/// every conversation goes with it, since each conversation that it lends holds only a weak way
/// back to it.
pub(super) async fn kept(
  mut asked: mpsc::UnboundedReceiver<Kept>,
  accounts: Arc<Accounts>,
  back: mpsc::WeakUnboundedSender<Kept>,
) {
  let mut keeper = Keeper { held: HashMap::new(), accounts, back };
  let mut ticks = tokio::time::interval(IDLE);
  ticks.tick().await;
  loop {
    tokio::select! {
      one = asked.recv() => match one {
        None => return,
        Some(Kept::Lend { key, fresh, messages, lent }) => {
          keeper.lend(key, fresh, &messages, lent);
        }
        Some(Kept::Back { key, conversation }) => {
          keeper.back(key, conversation);
          keeper.swept();
        }
      },
      _ = ticks.tick() => keeper.swept(),
    }
  }
}

/// What the keeper holds: the conversation of each key, the accounts that a new conversation takes,
/// and the way back to the keeper that each conversation it lends takes.
struct Keeper {
  held: HashMap<String, Slot>,
  accounts: Arc<Accounts>,
  back: mpsc::WeakUnboundedSender<Kept>,
}

impl Keeper {
  /// The conversation of a key, lent to a turn: the one the keeper holds, or a new one. A turn of a
  /// conversation lent waits for it.
  fn lend(
    &mut self,
    key: String,
    fresh: Box<Conversation>,
    messages: &[Message],
    lent: oneshot::Sender<Lent>,
  ) {
    let conversation = match self.held.remove(&key) {
      Some(Slot::Idle(conversation)) => conversation,
      Some(Slot::Lent(mut waiting)) => {
        waiting.push_back(lent);
        self.held.insert(key, Slot::Lent(waiting));
        return;
      }
      None => self.made(fresh, messages),
    };
    self.handed(key, conversation, VecDeque::from([lent]));
  }

  /// A new conversation, which forks the idle conversation it grows out of when there is one, so it
  /// takes all that conversation holds. It takes the account that the policy gives, and forks only
  /// a conversation that stands in the same config dir.
  fn made(&self, mut made: Box<Conversation>, messages: &[Message]) -> Box<Conversation> {
    let incoming: Vec<String> = messages.iter().map(canonical).collect();
    made.account = self.accounts.pick(SystemTime::now());
    let store = self.accounts.store(made.account);
    let donor = self
      .held
      .values()
      .filter_map(|one| match one {
        Slot::Idle(one) => Some(one),
        Slot::Lent(_) => None,
      })
      .filter(|one| {
        let chain = one.chain.len();
        chain > 0
          && one.name == made.name
          && one.system == made.system
          && self.accounts.store(one.account) == store
          && chain < incoming.len()
          && incoming[..chain] == one.chain[..]
          && messages[chain..].iter().all(|one| !matches!(one, Message::Assistant { .. }))
      })
      .max_by_key(|one| one.chain.len());
    if let Some(donor) = donor {
      made.parent = Some(donor.id.clone());
      made.chain.clone_from(&donor.chain);
    }
    made
  }

  /// A conversation taken back: lent to the next turn that waits for it.
  fn back(&mut self, key: String, conversation: Box<Conversation>) {
    let waiting = match self.held.remove(&key) {
      Some(Slot::Lent(waiting)) => waiting,
      _ => VecDeque::new(),
    };
    self.handed(key, conversation, waiting);
  }

  /// A conversation handed to the first of the turns that wait for it that still stands, which the
  /// others wait behind, or kept at hand when none stands.
  fn handed(
    &mut self,
    key: String,
    conversation: Box<Conversation>,
    mut waiting: VecDeque<oneshot::Sender<Lent>>,
  ) {
    let mut conversation = Some(conversation);
    while let Some(next) = waiting.pop_front() {
      let back = self.back.clone();
      match next.send(Lent { key: key.clone(), conversation: conversation.take(), keeper: back }) {
        Ok(()) => {
          self.held.insert(key, Slot::Lent(waiting));
          return;
        }
        Err(again) => conversation = again.taken(),
      }
    }
    if let Some(conversation) = conversation {
      self.held.insert(key, Slot::Idle(conversation));
    }
  }

  /// What no turn needs goes: the process of a conversation that sat idle too long, or that stands
  /// past the warm ones, the least used first, and a conversation with no process past the held
  /// ones. A conversation lent stands warm.
  fn swept(&mut self) {
    let count = self.held.len();
    let mut idle: Vec<_> = self
      .held
      .iter_mut()
      .filter_map(|(key, one)| match one {
        Slot::Idle(one) => Some((key.clone(), one)),
        Slot::Lent(_) => None,
      })
      .collect();
    idle.sort_by_key(|(_, one)| one.used);
    let mut warm = count - idle.len();
    warm += idle.iter().filter(|(_, one)| one.process.is_some()).count();
    let mut gone = Vec::new();
    for (key, one) in &mut idle {
      if one.process.is_some() && (warm > WARM || one.used.elapsed() > IDLE) {
        one.process = None;
        warm -= 1;
      }
      if one.process.is_none() && count - gone.len() > HELD {
        gone.push(key.clone());
      }
    }
    drop(idle);
    for key in gone {
      self.held.remove(&key);
    }
  }
}

/// The conversation of a key, lent to this turn alone by the keeper, to which it goes back however
/// the turn ends.
pub(super) async fn borrow(
  keeper: &mpsc::UnboundedSender<Kept>,
  key: String,
  fresh: Box<Conversation>,
  messages: Vec<Message>,
) -> Result<Lent, CompletionError> {
  let (lent, borrowed) = oneshot::channel();
  let gone = || failed("the keeper of the conversations of claude is gone");
  if keeper.send(Kept::Lend { key, fresh, messages, lent }).is_err() {
    return Err(gone());
  }
  let Ok(lent) = borrowed.await else { return Err(gone()) };
  Ok(lent)
}
