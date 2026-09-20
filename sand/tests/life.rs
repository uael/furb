//! One life of the real engine in a sandbox of monty.
//!
//! `tests/life.rs` drives a life through a python namespace behind a pipe, which sandboxes nothing and proves the
//! crate against the real engine. This drives one through [`Sand`], which is the sandbox the engine is meant for,
//! and holds the two to the same answers: a life boots, a chain opens under the one name, and a World outside the
//! sandbox says where it stands and is heard.

use furb::{Ears, Fact, Gate, Life, Reply, Value, World, verb};
use furb_sand::Sand;

/// A World of a few facts: it says where a chain stands, and hears everything else.
struct Yard {
  /// Every kind of fact it heard, in order, which is what the test reads of it.
  heard: Vec<String>,
}

impl World for Yard {
  fn hears(&mut self, fact: &Fact) -> Reply {
    self.heard.push(fact.kind().to_owned());
    if fact.kind() == "stand" {
      // A roster of one actor, a directory, and the actor a prompt goes to.
      let actor = Value::Tuple(vec![
        Value::Str("m".to_owned()),
        Value::Tuple(vec![Value::Str("low".to_owned())]),
        Value::Int(200_000),
      ]);
      let standing =
        Value::Tuple(vec![Value::List(vec![actor]), Value::Str("/w".to_owned()), Value::Str("m/low".to_owned())]);
      Reply::say(Fact::new("done", fact.about(), "world", vec![standing]))
    } else {
      Reply::Nothing
    }
  }
}

/// A gate that accepts every word, since no word of a model runs in these tests.
struct Open;

impl Gate for Open {
  fn gate(&mut self, _word: &str, _ladder: &[String], _shape: &str) -> Vec<String> {
    Vec::new()
  }
}

#[test]
fn a_life_of_the_engine_boots_in_a_sandbox_of_monty() {
  let (_voice, ears) = Ears::made();
  let world = Yard { heard: Vec::new() };
  let mut held = Life::boot(Sand::default(), world, Open, ears, &[]).expect("a life of the real engine in monty");
  assert_eq!(held.root(), "chain://operator.1");

  // The World was asked where the root chain stands, which is the one question a life asks to open.
  assert!(held.world().heard.iter().any(|kind| kind == "stand"));

  // And the turns of that chain read what the opening said.
  let root = held.root().to_owned();
  let turns = held.calls(verb::Turns { on: &root }).expect("the turns of the root");
  assert_eq!(turns.len(), 1);
}
