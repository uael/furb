//! The models of a life that a peer answers in their place, as a demo or a test of a window does:
//! each request of a model goes to that peer as a frame, and its turn comes back as a verb.

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
  outbox: Outbox,
  waiting: Arc<Mutex<Waiting>>,
}

/// The requests that wait for a turn, by their number, each with what it tells as the peer
/// writes.
#[derive(Default)]
struct Waiting {
  next: u64,
  asks: HashMap<u64, (oneshot::Sender<Result<Object, String>>, Told)>,
}

impl Answerer {
  /// The peer of an outbox, as the answerer of the models of a life.
  pub(crate) fn new(life: String, outbox: Outbox) -> Self {
    Answerer { life, outbox, waiting: Arc::default() }
  }

  /// The function that stands in for the models of the life: it asks the peer, and waits for its
  /// turn, or for its end.
  pub(crate) fn hosted(&self) -> Hosted {
    let answerer = self.clone();
    Arc::new(move |request: Value, told: Told| {
      let (reply, replied) = oneshot::channel();
      let ask = answerer.lock().put(reply, told);
      let frame = Frame::Ask { life: answerer.life.clone(), ask, request };
      let sent = answerer.outbox.send(frame).is_ok();
      let answerer = answerer.clone();
      Box::pin(async move {
        let got = if sent {
          tokio::select! {
            got = replied => got.unwrap_or_else(|_| Err(GONE.to_owned())),
            () = answerer.outbox.closed() => Err(GONE.to_owned()),
          }
        } else {
          Err(GONE.to_owned())
        };
        answerer.lock().asks.remove(&ask);
        got
      })
    })
  }

  /// The turn that the peer gave for a request, or why it gives none.
  pub(crate) fn answered(
    &self,
    ask: u64,
    turn: Result<Object, String>,
  ) -> Result<(), ProtocolError> {
    let (reply, _) = self.lock().asks.remove(&ask).ok_or_else(|| self.unasked(ask))?;
    reply.send(turn).ok();
    Ok(())
  }

  /// What the peer added to the text and to the thought of the turn of a request.
  pub(crate) fn wrote(&self, ask: u64, text: &str, thinking: &str) -> Result<(), ProtocolError> {
    let told = self.lock().asks.get(&ask).map(|(_, told)| Arc::clone(told));
    told.ok_or_else(|| self.unasked(ask))?(text, thinking);
    Ok(())
  }

  fn lock(&self) -> std::sync::MutexGuard<'_, Waiting> {
    self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
  }

  fn unasked(&self, ask: u64) -> ProtocolError {
    ProtocolError::Unasked { life: self.life.clone(), ask }
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
