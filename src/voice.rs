//! How a host speaks into a life when nothing asked it to.
//!
//! A World answers most facts where it hears them, and [`crate::Reply`] carries those answers back. The work that
//! finishes later is not like that: a command says what it wrote as it writes it, says its code when it ends, and
//! a model answers an ask long after the ask was heard. The engine says so plainly: the World speaks by yielding
//! a fact, or by calling send under its own name when it speaks from its loop. This is that second way.
//!
//! What a host says there is a fact of its own, or a control over an act: a prompt of the operator is answered by
//! closing the prompt with the line the operator wrote, and a chain whose actor has gone quiet twice is paused.
//! [`Said`] is those three, and the life does each of them in the order it was said.
//!
//! A [`Voice`] is what a host holds to speak, and [`Ears`] is what the life drains. The two are made together and
//! belong together: what is said into a Voice is heard by the life that holds its Ears, and by no other.
//!
//! A Voice may be cloned and carried anywhere the host does its work, including another thread, since what it
//! says waits in order until the life is ready to hear it. A life drains its Ears where it is safe to say a fact,
//! and never in the middle of hearing one, so nothing of the host lands inside a fact of its own.

use std::{
  collections::VecDeque,
  sync::{Arc, Condvar, Mutex},
  time::Duration,
};

use crate::fact::{Fact, Value};

/// One thing a host said while nothing asked it to.
#[derive(Debug, Clone, PartialEq)]
pub enum Said {
  /// One fact, said under the name of whoever said it.
  Fact(Fact),
  /// One act, closed with a value, which is how a prompt of the operator is answered.
  Closed {
    /// The act that is closed.
    id: String,
    /// What it is done with.
    value: Value,
  },
  /// One chain, paused, which a World does when it cannot answer and the fault of it stands.
  Paused(String),
}

impl Said {
  /// What a host said, as the boundary reads it: its mark, and then what it carries.
  pub fn plain(&self) -> Value {
    let held = match self {
      Said::Fact(one) => vec![
        Value::Str("fact".to_owned()),
        Value::Str(one.kind().to_owned()),
        Value::Str(one.about().to_owned()),
        Value::Str(one.by().to_owned()),
        Value::List(one.words().to_vec()),
      ],
      Said::Closed { id, value } => {
        vec![Value::Str("close".to_owned()), Value::Str(id.clone()), value.clone()]
      }
      Said::Paused(id) => vec![Value::Str("pause".to_owned()), Value::Str(id.clone())],
    };
    Value::Tuple(held).plain()
  }
}

/// What a host has said and the life has not heard yet, held in the order it was said, and the word that wakes
/// whoever waits for it.
type Waiting = Arc<(Mutex<VecDeque<Said>>, Condvar)>;

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
  /// A word said after the life is gone is dropped, since there is nobody to hear it.
  pub fn send(&self, fact: Fact) {
    self.holds(Said::Fact(fact));
  }

  /// One act, closed with a value, which is how the work a host started answers the act that asked for it.
  pub fn close(&self, id: impl Into<String>, value: Value) {
    self.holds(Said::Closed { id: id.into(), value });
  }

  /// One chain, paused, which stops it until the operator wakes it.
  pub fn pause(&self, id: impl Into<String>) {
    self.holds(Said::Paused(id.into()));
  }

  /// Whether anything said into this Voice is still waiting to be heard.
  pub fn waiting(&self) -> bool {
    self.waiting.0.lock().is_ok_and(|held| !held.is_empty())
  }

  /// One thing said, held until the life hears it, and whoever waits for it woken.
  fn holds(&self, one: Said) {
    if let Ok(mut held) = self.waiting.0.lock() {
      held.push_back(one);
      self.waiting.1.notify_all();
    }
  }
}

/// What a life drains: everything its host has said and it has not heard.
#[derive(Debug)]
pub struct Ears {
  /// What has been said and not heard.
  waiting: Waiting,
}

impl Ears {
  /// A Voice and the Ears that hear it, made together.
  #[must_use]
  pub fn made() -> (Voice, Ears) {
    let waiting: Waiting = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
    (Voice { waiting: Arc::clone(&waiting) }, Ears { waiting })
  }

  /// Wait until the host says something, or until this long has passed, and say whether anything waits.
  ///
  /// A life goes on when a fact is said in it, and while it waits for a model, a command or a person, nothing
  /// of it moves until the host speaks. This is how a host waits for that without asking over and over.
  pub fn waits(&mut self, how_long: Duration) -> bool {
    let Ok(held) = self.waiting.0.lock() else { return false };
    if !held.is_empty() {
      return true;
    }
    match self.waiting.1.wait_timeout(held, how_long) {
      Ok((held, _)) => !held.is_empty(),
      Err(_) => false,
    }
  }

  /// Everything said since the last drain, in the order it was said.
  ///
  /// A life drains where it is safe to say a fact, so what a host says lands between the facts of the life and
  /// never inside one.
  pub fn drained(&mut self) -> Vec<Said> {
    match self.waiting.0.lock() {
      Ok(mut held) => held.drain(..).collect(),
      Err(_) => Vec::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{thread, time::Instant};

  use super::*;

  /// One fact of a command, as the World of a host says it while the command runs.
  fn out(text: &str) -> Fact {
    Fact::new("out", "bash://operator.1.1", "world", vec![Value::Str(text.to_owned()), Value::Str("stdout".to_owned())])
  }

  #[test]
  fn what_a_host_says_is_heard_in_the_order_it_was_said() {
    let (voice, mut ears) = Ears::made();
    assert!(!voice.waiting());
    voice.send(out("one\n"));
    voice.send(out("two\n"));
    assert!(voice.waiting());
    let held = ears.drained();
    assert_eq!(held.len(), 2);
    assert_eq!(held[0], Said::Fact(out("one\n")));
    assert_eq!(held[1], Said::Fact(out("two\n")));
    assert!(!voice.waiting());
    assert!(ears.drained().is_empty());
  }

  #[test]
  fn a_host_answers_a_prompt_by_closing_it_and_quiets_a_chain_by_pausing_it() {
    let (voice, mut ears) = Ears::made();
    voice.close("prompt://operator.2", Value::Int(3));
    voice.pause("chain://operator.1");
    let held = ears.drained();
    assert_eq!(held[0], Said::Closed { id: "prompt://operator.2".to_owned(), value: Value::Int(3) });
    assert_eq!(held[1], Said::Paused("chain://operator.1".to_owned()));
  }

  #[test]
  fn what_a_host_says_carries_its_mark_so_the_boundary_reads_which_of_the_three_it_is() {
    let held = Said::Closed { id: "prompt://operator.2".to_owned(), value: Value::Int(3) };
    let words = Value::of_plain(&held.plain());
    assert_eq!(words.as_entries().unwrap()[0], Value::Str("close".to_owned()));
    assert_eq!(Value::of_plain(&Said::Fact(out("one\n")).plain()).as_entries().unwrap().len(), 5);
  }

  #[test]
  fn a_life_waits_for_its_host_and_never_asks_it_over_and_over() {
    let (voice, mut ears) = Ears::made();
    let waited = Instant::now();
    assert!(!ears.waits(Duration::from_millis(30)));
    assert!(waited.elapsed() >= Duration::from_millis(25), "it waits for as long as it was given");
    let held = voice.clone();
    thread::spawn(move || {
      thread::sleep(Duration::from_millis(20));
      held.send(out("late\n"));
    });
    let waited = Instant::now();
    assert!(ears.waits(Duration::from_secs(10)));
    assert!(waited.elapsed() < Duration::from_secs(5), "it wakes when the host speaks and not at its end");
    assert_eq!(ears.drained().len(), 1);
    voice.send(out("now\n"));
    assert!(ears.waits(Duration::from_secs(0)), "what is said already needs no wait at all");
  }

  #[test]
  fn a_voice_is_carried_to_wherever_the_host_does_its_work() {
    let (voice, mut ears) = Ears::made();
    let held = voice.clone();
    thread::spawn(move || held.send(out("from the command\n"))).join().unwrap();
    let said = ears.drained();
    assert_eq!(said.len(), 1);
    assert_eq!(said[0], Said::Fact(out("from the command\n")));
  }

  #[test]
  fn a_word_said_after_the_life_is_gone_is_dropped() {
    let (voice, ears) = Ears::made();
    drop(ears);
    voice.send(out("nobody hears this\n"));
    assert!(voice.waiting());
  }
}
