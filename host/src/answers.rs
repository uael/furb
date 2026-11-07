//! The models of a life that a peer answers in their place, as a demo or a test of a window does:
//! each request of a model goes to that peer as a frame, and its turn comes back as a verb.
//!
//! The peer that opened the life with `answers` owns each request, by its connection: no other
//! peer gives a turn or a write for it. Once that peer is gone, the next peer that opens the life
//! with `answers` answers each later request in its place, and an open with `answers` is refused
//! while it lives. Each request of the peer that is gone fails at that change, so the new peer
//! never speaks for a request that it was not asked. A request leaves the broker when its turn
//! comes, when
//! the life drops it, or when the connection of its peer ends; a turn or a write that comes after
//! is refused, and the peer is not told that the life dropped the request.

use std::sync::MutexGuard;
use std::{
  collections::HashMap,
  sync::{Arc, Mutex, PoisonError},
};

use furb::{
  Object,
  world::{Hosted, Told},
};
use serde_json::Value;
use tokio::sync::oneshot;

use crate::{error::ProtocolError, protocol::Frame, serve::Outbox};

/// Why a model of a life gives no turn once the peer that answers it is gone.
const GONE: &str = "the window that answers the models of this life is gone";

/// The peer that answers the models of a life, and each request that waits for its turn.
#[derive(Clone)]
pub(crate) struct Answerer {
  life: String,
  waiting: Arc<Mutex<Waiting>>,
}

/// The peer that answers now, and the requests that wait for a turn, by their number, each with
/// what it tells as the peer writes.
struct Waiting {
  outbox: Outbox,
  next: u64,
  asks: HashMap<u64, (oneshot::Sender<Result<Object, String>>, Told)>,
}

impl Answerer {
  /// The peer of an outbox, as the answerer of the models of a life.
  pub(crate) fn new(life: String, outbox: Outbox) -> Self {
    let waiting = Waiting { outbox, next: 0, asks: HashMap::new() };
    Self { life, waiting: Arc::new(Mutex::new(waiting)) }
  }

  /// The peer of an outbox as the one that answers, in the place of a peer that is gone, or the
  /// refusal that the one that answers lives. Each request of the peer that is gone fails in the
  /// same step, so no later peer gives a turn or a write for it.
  pub(crate) fn taken_by(&self, outbox: &Outbox) -> Result<(), ProtocolError> {
    let mut waiting = self.lock();
    if waiting.outbox.same_channel(outbox) {
      return Ok(());
    }
    if !waiting.outbox.is_closed() {
      return Err(ProtocolError::Answering { life: self.life.clone() });
    }
    waiting.outbox = outbox.clone();
    for (_, (reply, _)) in waiting.asks.drain() {
      reply.send(Err(GONE.to_owned())).ok();
    }
    Ok(())
  }

  /// The outbox of the peer that answers now.
  pub(crate) fn outbox(&self) -> Outbox {
    self.lock().outbox.clone()
  }

  /// The function that stands in for the models of the life: it asks the peer, and waits for its
  /// turn, or for its end.
  pub(crate) fn hosted(&self) -> Hosted {
    let answerer = self.clone();
    Arc::new(move |request: Value, told: Told| {
      let (reply, replied) = oneshot::channel();
      let (ask, outbox) = {
        let mut waiting = answerer.lock();
        (waiting.put(reply, told), waiting.outbox.clone())
      };
      let pending = Pending { ask, waiting: Arc::clone(&answerer.waiting) };
      let frame = Frame::Ask { life: answerer.life.clone(), ask, request };
      let sent = outbox.send(frame).is_ok();
      Box::pin(async move {
        let _pending = pending;
        if !sent {
          return Err(GONE.to_owned());
        }
        tokio::select! {
          got = replied => got.unwrap_or_else(|_| Err(GONE.to_owned())),
          () = outbox.closed() => Err(GONE.to_owned()),
        }
      })
    })
  }

  /// The turn that the peer of an outbox gave for a request, or why it gives none.
  pub(crate) fn answered(
    &self,
    by: &Outbox,
    ask: u64,
    turn: Result<Object, String>,
  ) -> Result<(), ProtocolError> {
    self.owns(by, ask)?;
    let (reply, _) = self.lock().asks.remove(&ask).ok_or_else(|| self.unasked(ask))?;
    // The waiter drops its receiver only as it ends, and that end removes the request first.
    reply.send(turn).ok();
    Ok(())
  }

  /// What the peer of an outbox added to the text and to the thought of the turn of a request.
  pub(crate) fn wrote(
    &self,
    by: &Outbox,
    ask: u64,
    text: &str,
    thinking: &str,
  ) -> Result<(), ProtocolError> {
    self.owns(by, ask)?;
    let told = self.lock().asks.get(&ask).map(|(_, told)| Arc::clone(told));
    told.ok_or_else(|| self.unasked(ask))?(text, thinking);
    Ok(())
  }

  /// Whether the peer of an outbox is the one that answers, which alone speaks for a request.
  fn owns(&self, by: &Outbox, ask: u64) -> Result<(), ProtocolError> {
    if self.lock().outbox.same_channel(by) {
      Ok(())
    } else {
      Err(ProtocolError::Foreign { life: self.life.clone(), ask })
    }
  }

  fn lock(&self) -> MutexGuard<'_, Waiting> {
    self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
  }

  fn unasked(&self, ask: u64) -> ProtocolError {
    ProtocolError::Unasked { life: self.life.clone(), ask }
  }
}

/// A request that waits, which leaves the broker when its future ends or is dropped.
struct Pending {
  ask: u64,
  waiting: Arc<Mutex<Waiting>>,
}

impl Drop for Pending {
  fn drop(&mut self) {
    self.waiting.lock().unwrap_or_else(PoisonError::into_inner).asks.remove(&self.ask);
  }
}

impl Waiting {
  /// A request that waits, under the next number.
  fn put(&mut self, reply: oneshot::Sender<Result<Object, String>>, told: Told) -> u64 {
    self.next += 1;
    self.asks.insert(self.next, (reply, told));
    self.next
  }
}

#[cfg(test)]
#[expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]
mod tests {
  use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  };

  use furb::world::Told;
  use serde_json::json;
  use tokio::sync::mpsc;

  use super::{Answerer, GONE};
  use crate::{error::ProtocolError, protocol::Frame};

  const LIFE: &str = "/a.jsonl";

  /// What a request tells, and how many times it told it.
  fn counted() -> (Told, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&count);
    (Arc::new(move |_: &str, _: &str| _ = counter.fetch_add(1, Ordering::SeqCst)), count)
  }

  fn unasked(ask: u64) -> Result<(), ProtocolError> {
    Err(ProtocolError::Unasked { life: LIFE.to_owned(), ask })
  }

  fn asked(frame: Option<Frame>) -> u64 {
    match frame {
      Some(Frame::Ask { ask, .. }) => ask,
      other => panic!("no ask: {other:?}"),
    }
  }

  #[tokio::test]
  async fn a_request_that_the_life_drops_before_its_turn_takes_no_later_turn() {
    let (outbox, mut pushed) = mpsc::unbounded_channel();
    let answerer = Answerer::new(LIFE.to_owned(), outbox.clone());
    let (told, count) = counted();
    let waiting = answerer.hosted()(json!({}), told);
    let ask = asked(pushed.recv().await);
    drop(waiting);
    assert_eq!(answerer.wrote(&outbox, ask, "late", ""), unasked(ask));
    assert_eq!(answerer.answered(&outbox, ask, Err("late".to_owned())), unasked(ask));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(answerer.lock().asks.is_empty());
  }

  #[tokio::test]
  async fn a_write_or_a_turn_after_the_turn_of_a_request_is_refused() {
    let (outbox, mut pushed) = mpsc::unbounded_channel();
    let answerer = Answerer::new(LIFE.to_owned(), outbox.clone());
    let (told, count) = counted();
    let waiting = answerer.hosted()(json!({}), told);
    let ask = asked(pushed.recv().await);
    assert_eq!(answerer.answered(&outbox, ask, Err("given".to_owned())), Ok(()));
    assert_eq!(waiting.await, Err("given".to_owned()));
    assert_eq!(answerer.wrote(&outbox, ask, "late", ""), unasked(ask));
    assert_eq!(answerer.answered(&outbox, ask, Err("again".to_owned())), unasked(ask));
    assert_eq!(count.load(Ordering::SeqCst), 0);
  }

  #[tokio::test]
  async fn a_request_fails_and_leaves_the_broker_once_its_peer_is_gone() {
    let (outbox, mut pushed) = mpsc::unbounded_channel();
    let answerer = Answerer::new(LIFE.to_owned(), outbox);
    let waiting = answerer.hosted()(json!({}), counted().0);
    asked(pushed.recv().await);
    drop(pushed);
    assert_eq!(waiting.await, Err(GONE.to_owned()));
    assert!(answerer.lock().asks.is_empty());
    assert_eq!(answerer.hosted()(json!({}), counted().0).await, Err(GONE.to_owned()));
    assert!(answerer.lock().asks.is_empty());
  }

  #[tokio::test]
  async fn a_peer_that_takes_the_place_of_one_that_is_gone_cannot_speak_for_its_requests() {
    let (gone, mut pushed) = mpsc::unbounded_channel();
    let answerer = Answerer::new(LIFE.to_owned(), gone);
    let (told, count) = counted();
    let waiting = answerer.hosted()(json!({}), told);
    let ask = asked(pushed.recv().await);
    drop(pushed);
    let (next, _heard) = mpsc::unbounded_channel();
    assert_eq!(answerer.taken_by(&next), Ok(()));
    assert_eq!(answerer.wrote(&next, ask, "taken", ""), unasked(ask));
    assert_eq!(answerer.answered(&next, ask, Err("taken".to_owned())), unasked(ask));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(waiting.await, Err(GONE.to_owned()));
    assert!(answerer.lock().asks.is_empty());
  }

  #[tokio::test]
  async fn a_peer_that_does_not_answer_the_life_cannot_speak_for_its_request() {
    let (outbox, mut pushed) = mpsc::unbounded_channel();
    let (other, _heard) = mpsc::unbounded_channel();
    let answerer = Answerer::new(LIFE.to_owned(), outbox.clone());
    let (told, count) = counted();
    let waiting = answerer.hosted()(json!({}), told);
    let ask = asked(pushed.recv().await);
    let foreign = Err(ProtocolError::Foreign { life: LIFE.to_owned(), ask });
    assert_eq!(answerer.wrote(&other, ask, "taken", ""), foreign);
    assert_eq!(answerer.answered(&other, ask, Err("taken".to_owned())), foreign);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(answerer.wrote(&outbox, ask, "mine", ""), Ok(()));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(answerer.answered(&outbox, ask, Err("mine".to_owned())), Ok(()));
    assert_eq!(waiting.await, Err("mine".to_owned()));
  }
}
