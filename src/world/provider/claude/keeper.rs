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
    lent: oneshot::Sender<Box<Conversation>>,
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
  Lent(VecDeque<oneshot::Sender<Box<Conversation>>>),
}

/// A conversation lent to a turn, which goes back to the keeper however the turn ends.
pub(super) struct Lent {
  key: String,
  conversation: Option<Box<Conversation>>,
  keeper: mpsc::UnboundedSender<Kept>,
}

impl Lent {
  #[expect(clippy::expect_used, reason = "debt: an expect where a type can hold the invariant")]
  pub(super) fn conversation(&mut self) -> &mut Conversation {
    self.conversation.as_mut().expect("a lent conversation is held until it goes back")
  }
}

impl Drop for Lent {
  fn drop(&mut self) {
    if let Some(conversation) = self.conversation.take() {
      let key = std::mem::take(&mut self.key);
      drop(self.keeper.send(Kept::Back { key, conversation }));
    }
  }
}

/// The keeper of the conversations: it lends each to one turn at a time, and the next turn of a
/// conversation lent waits for it; it takes each back, and then sweeps; and it sweeps once in each
/// span that a process may sit idle. It ends when the last clone of the command line goes, and
/// every conversation goes with it.
pub(super) async fn kept(mut asked: mpsc::UnboundedReceiver<Kept>, accounts: Arc<Accounts>) {
  let mut held: HashMap<String, Slot> = HashMap::new();
  let mut ticks = tokio::time::interval(IDLE);
  ticks.tick().await;
  loop {
    tokio::select! {
      one = asked.recv() => match one {
        None => return,
        Some(Kept::Lend { key, fresh, messages, lent }) => {
          lend(&mut held, &accounts, key, fresh, &messages, lent);
        }
        Some(Kept::Back { key, conversation }) => {
          back(&mut held, key, conversation);
          swept(&mut held);
        }
      },
      _ = ticks.tick() => swept(&mut held),
    }
  }
}

/// The conversation of a key, lent to a turn: the one the keeper holds, or a new one, which forks
/// the idle conversation it grows out of when there is one, so it takes all that conversation
/// holds. A turn of a conversation lent waits for it. A new conversation takes the account that the
/// policy gives, and forks only a conversation that stands in the same config dir.
fn lend(
  held: &mut HashMap<String, Slot>,
  accounts: &Accounts,
  key: String,
  fresh: Box<Conversation>,
  messages: &[Message],
  lent: oneshot::Sender<Box<Conversation>>,
) {
  let conversation = match held.remove(&key) {
    Some(Slot::Idle(conversation)) => conversation,
    Some(Slot::Lent(mut waiting)) => {
      waiting.push_back(lent);
      held.insert(key, Slot::Lent(waiting));
      return;
    }
    None => {
      let incoming: Vec<String> = messages.iter().map(canonical).collect();
      let mut made = fresh;
      made.account = accounts.pick(SystemTime::now());
      let store = accounts.store(made.account);
      let donor = held
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
            && accounts.store(one.account) == store
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
  };
  match lent.send(conversation) {
    Ok(()) => held.insert(key, Slot::Lent(VecDeque::new())),
    // The turn went before it was lent the conversation, which stays at hand.
    Err(conversation) => held.insert(key, Slot::Idle(conversation)),
  };
}

/// A conversation taken back: lent to the next turn that waits for it and still stands, or kept at
/// hand.
fn back(held: &mut HashMap<String, Slot>, key: String, mut conversation: Box<Conversation>) {
  let mut waiting = match held.remove(&key) {
    Some(Slot::Lent(waiting)) => waiting,
    _ => VecDeque::new(),
  };
  while let Some(next) = waiting.pop_front() {
    match next.send(conversation) {
      Ok(()) => {
        held.insert(key, Slot::Lent(waiting));
        return;
      }
      Err(again) => conversation = again,
    }
  }
  held.insert(key, Slot::Idle(conversation));
}

/// What no turn needs goes: the process of a conversation that sat idle too long, or that stands
/// past the warm ones, the least used first, and a conversation with no process past the held ones.
/// A conversation lent stands warm.
fn swept(held: &mut HashMap<String, Slot>) {
  let count = held.len();
  let mut idle: Vec<_> = held
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
    held.remove(&key);
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
  let asked = Kept::Lend { key: key.clone(), fresh, messages, lent };
  if keeper.send(asked).is_err() {
    return Err(gone());
  }
  let Ok(conversation) = borrowed.await else { return Err(gone()) };
  Ok(Lent { key, conversation: Some(conversation), keeper: keeper.clone() })
}
