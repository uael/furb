//! How a host speaks into a life when nothing asked it to.
//!
//! A World answers most facts where it hears them, and [`crate::Reply`] carries those answers back. The facts of
//! the World about the acts that complete later are not like that: a command says what it wrote as it writes it,
//! says its code when it ends, and a model answers an ask long after the ask was heard. The engine says so
//! plainly: the World speaks by yielding a fact, or by calling send under its own name when it speaks from its
//! loop. This is that second way.
//!
//! A [`Voice`] is what a host holds to speak, and [`Ears`] is what the life drains. The two are made together and
//! belong together: a fact said into a Voice is heard by the life that holds its Ears, and by no other.
//!
//! A Voice may be cloned and carried anywhere the host does its work, including another thread, since what it
//! says waits in order until the life is ready to hear it. A life drains its Ears where it is safe to say a fact,
//! and never in the middle of hearing one, so nothing of the host lands inside a fact of its own.

use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
};

use crate::fact::Fact;

/// The facts a host has said and the life has not heard yet, held in the order they were said.
type Waiting = Arc<Mutex<VecDeque<Fact>>>;

/// What a host speaks into a life with, for what it does not answer where it hears it.
///
/// It is cheap to clone and it may be carried across threads, so one command, one model and one clock may each
/// hold one. What it says is heard in the order it was said.
#[derive(Debug, Clone)]
pub struct Voice {
  /// What has been said and not heard.
  waiting: Waiting,
}

impl Voice {
  /// One fact, said into the life.
  ///
  /// A fact said after the life is gone is dropped, since there is nobody to hear it.
  pub fn say(&self, fact: Fact) {
    if let Ok(mut held) = self.waiting.lock() {
      held.push_back(fact);
    }
  }

  /// Whether anything said into this Voice is still waiting to be heard.
  pub fn waiting(&self) -> bool {
    self.waiting.lock().is_ok_and(|held| !held.is_empty())
  }
}

/// What a life drains: the facts its host has said and it has not heard.
#[derive(Debug)]
pub struct Ears {
  /// What has been said and not heard.
  waiting: Waiting,
}

impl Ears {
  /// A Voice and the Ears that hear it, made together.
  #[must_use]
  pub fn made() -> (Voice, Ears) {
    let waiting: Waiting = Arc::new(Mutex::new(VecDeque::new()));
    (Voice { waiting: Arc::clone(&waiting) }, Ears { waiting })
  }

  /// Everything said since the last drain, in the order it was said.
  ///
  /// A life drains where it is safe to say a fact, so what a host says lands between the facts of the life and
  /// never inside one.
  pub fn drained(&mut self) -> Vec<Fact> {
    match self.waiting.lock() {
      Ok(mut held) => held.drain(..).collect(),
      Err(_) => Vec::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::thread;

  use super::*;
  use crate::fact::Value;

  /// One fact of a command, as the World of a host says it while the command runs.
  fn out(text: &str) -> Fact {
    Fact::new(
      "out",
      "bash://operator.1.1",
      "world",
      vec![Value::Str(text.to_owned()), Value::Str("stdout".to_owned())],
    )
  }

  #[test]
  fn what_a_host_says_is_heard_in_the_order_it_was_said() {
    let (voice, mut ears) = Ears::made();
    assert!(!voice.waiting());
    voice.say(out("one\n"));
    voice.say(out("two\n"));
    assert!(voice.waiting());
    let held = ears.drained();
    assert_eq!(held.len(), 2);
    assert_eq!(held[0].words()[0], Value::Str("one\n".to_owned()));
    assert_eq!(held[1].words()[0], Value::Str("two\n".to_owned()));
    assert!(!voice.waiting());
    assert!(ears.drained().is_empty());
  }

  #[test]
  fn a_voice_is_carried_to_wherever_the_host_does_its_work() {
    let (voice, mut ears) = Ears::made();
    let held = voice.clone();
    thread::spawn(move || held.say(out("from the command\n"))).join().unwrap();
    let said = ears.drained();
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].kind(), "out");
    assert_eq!(said[0].by(), "world");
  }

  #[test]
  fn a_fact_said_after_the_life_is_gone_is_dropped() {
    let (voice, ears) = Ears::made();
    drop(ears);
    voice.say(out("nobody hears this\n"));
    assert!(voice.waiting());
  }
}
