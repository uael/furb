//! The Kernel of the crate: it gates the word of a rung, and it runs that word in a sandbox.
//!
//! The engine holds the laws of a chain. To judge a word before it runs, and to run it, are machinery, so they
//! stand behind the Kernel that the contract declares. What makes this Kernel different from the one of the
//! interpreter is where the word runs: a model writes the word, so the word is the one part of a life that is not
//! trusted, and it runs in a sandbox that reaches the engine through [`Sandbox`] and reaches nothing else.
//!
//! The Kernel speaks four facts and hears four. It answers a `gate` with its findings, begins a `run` in the
//! module of the chain the run names, says `wants` for the act a run waits for, takes a `sent` of what that act
//! came to, and says `ran` with what the word gave. A `cancel` or a `close` over a run drops its frame, and the
//! run is over with a cancel of its own, since the one that runs the word is the one that ends it.

use crate::{
  fact::{Fact, Value},
  world::{Kernel, Reply},
};

/// What one step of a word came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
  /// The word is over: nothing for a word that ran to its end, and the exception for a word that raised.
  Ran(Option<Value>),
  /// The word waits for an act, by the name of it.
  Wants(String),
}

/// Where the word of a rung runs.
///
/// One of these serves one life, since the frames it holds and the ladders it gates against are that life's own.
/// Everything the word says to the engine goes out through the host that holds the sandbox, so the sandbox itself
/// reaches no disk, no machine and no name of the outside.
pub trait Sandbox {
  /// What the gate finds against a word, read against the ladder of its chain and the shape it must give.
  ///
  /// Nothing at all means the word may run.
  fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String>;

  /// A run begun: the word runs in the module of its chain, up to its first await or to its end.
  fn begin(&mut self, rung: &str, chain: &str, word: &str) -> Step;

  /// The run stepped with what the act it waited for came to.
  fn carry(&mut self, rung: &str, got: &Value) -> Step;

  /// The frame of a run that a control is over, dropped.
  fn drop_frame(&mut self, rung: &str);
}

/// The Kernel that gates and runs, over any sandbox.
#[derive(Debug, Default)]
pub struct Native<S> {
  /// Where the words run.
  pub sandbox: S,
  /// The ladder of every chain, in the order its words were accepted, which the gate reads a word against.
  ladders: Vec<(String, Vec<String>)>,
  /// The runs that stand, each by its name and the chain it runs on.
  frames: Vec<(String, String)>,
}

impl<S: Sandbox> Native<S> {
  /// A Kernel over one sandbox, for one life.
  pub fn new(sandbox: S) -> Self {
    Native { sandbox, ladders: Vec::new(), frames: Vec::new() }
  }

  /// The ladder of a chain, which is empty for a chain that has run no word.
  fn ladder(&mut self, chain: &str) -> &mut Vec<String> {
    if !self.ladders.iter().any(|(held, _)| held == chain) {
      self.ladders.push((chain.to_owned(), Vec::new()));
    }
    let found = self.ladders.iter_mut().find(|(held, _)| held == chain);
    &mut found.expect("the ladder was made above").1
  }

  /// What a step of a run says to the chain that had it run.
  fn said(&mut self, rung: &str, step: Step) -> Reply {
    match step {
      Step::Ran(got) => {
        self.frames.retain(|(held, _)| held != rung);
        Reply::say(Fact::new("ran", rung, rung, vec![got.unwrap_or(Value::None)]))
      }
      Step::Wants(act) => Reply::say(Fact::new("wants", rung, rung, vec![Value::Str(act)])),
    }
  }

  /// Every run a control is over, which is the run it names and every run under it.
  fn under(&self, about: &str) -> Vec<String> {
    self
      .frames
      .iter()
      .filter(|(held, _)| held == about || held.starts_with(&format!("{about}.")) || under(held, about))
      .map(|(held, _)| held.clone())
      .collect()
  }
}

/// Whether one act is another or was made by it, which their lineages say.
fn under(name: &str, of: &str) -> bool {
  let (name, of) = (lineage(name), lineage(of));
  !of.is_empty() && (name == of || name.starts_with(&format!("{of}.")))
}

/// The lineage of an act, which its name holds after its kind.
fn lineage(name: &str) -> &str {
  name.split_once("://").map_or(name, |(_, held)| held)
}

impl<S: Sandbox> Kernel for Native<S> {
  fn hears(&mut self, fact: &Fact) -> Reply {
    let words = fact.words();
    match fact.kind() {
      "gate" => {
        let (chain, word, shape) = (word_at(words, 0), word_at(words, 1), word_at(words, 2));
        let ladder = self.ladder(&chain).clone();
        let found = self.sandbox.gate(&word, &ladder, &shape);
        Reply::say(Fact::new(
          "done",
          fact.about(),
          "kernel",
          vec![Value::List(found.into_iter().map(Value::Str).collect())],
        ))
      }
      "run" => {
        let (chain, word) = (word_at(words, 0), word_at(words, 1));
        let rung = fact.about().to_owned();
        self.ladder(&chain).push(word.clone());
        self.frames.push((rung.clone(), chain.clone()));
        let step = self.sandbox.begin(&rung, &chain, &word);
        self.said(&rung, step)
      }
      "sent" => {
        let rung = fact.about().to_owned();
        if !self.frames.iter().any(|(held, _)| *held == rung) {
          return Reply::Nothing;
        }
        let got = words.first().cloned().unwrap_or(Value::None);
        let step = self.sandbox.carry(&rung, &got);
        self.said(&rung, step)
      }
      "cancel" | "close" => {
        let mut said = Vec::new();
        for rung in self.under(fact.about()) {
          self.sandbox.drop_frame(&rung);
          self.frames.retain(|(held, _)| *held != rung);
          said.push(Fact::new(
            "ran",
            &rung,
            &rung,
            vec![Value::Error { name: "CancelledError".to_owned(), args: Vec::new() }],
          ));
        }
        if said.is_empty() { Reply::Nothing } else { Reply::Say(said) }
      }
      _ => Reply::Nothing,
    }
  }
}

/// One word of a fact as text, and nothing at all when the fact holds no such word.
fn word_at(words: &[Value], at: usize) -> String {
  words.get(at).and_then(Value::as_str).unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A sandbox of the test: it refuses a word that holds BAD, and it runs a word by a script of steps.
  #[derive(Default)]
  struct Said {
    steps: Vec<Step>,
    begun: Vec<(String, String, String)>,
    gated: Vec<(String, Vec<String>, String)>,
    dropped: Vec<String>,
  }

  impl Sandbox for Said {
    fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String> {
      self.gated.push((word.to_owned(), ladder.to_vec(), shape.to_owned()));
      if word.contains("BAD") {
        vec![format!("BAD in rung, against {shape} after {} rungs", ladder.len())]
      } else {
        Vec::new()
      }
    }

    fn begin(&mut self, rung: &str, chain: &str, word: &str) -> Step {
      self.begun.push((rung.to_owned(), chain.to_owned(), word.to_owned()));
      self.steps.remove(0)
    }

    fn carry(&mut self, _rung: &str, _got: &Value) -> Step {
      self.steps.remove(0)
    }

    fn drop_frame(&mut self, rung: &str) {
      self.dropped.push(rung.to_owned());
    }
  }

  /// A run fact of the engine, which names the chain whose module the word runs in.
  fn run(rung: &str, chain: &str, word: &str) -> Fact {
    Fact::new(
      "run",
      rung,
      "chain://operator.1",
      vec![Value::Str(chain.to_owned()), Value::Str(word.to_owned())],
    )
  }

  #[test]
  fn the_kernel_says_ran_with_nothing_for_a_word_that_ran_to_its_end() {
    let mut kernel = Native::new(Said { steps: vec![Step::Ran(None)], ..Said::default() });
    let said = kernel.hears(&run("rung://operator.1.1", "chain://operator.1", "close(1)"));
    assert_eq!(
      said,
      Reply::say(Fact::new("ran", "rung://operator.1.1", "rung://operator.1.1", vec![Value::None]))
    );
    assert_eq!(kernel.sandbox.begun.len(), 1);
    assert_eq!(kernel.sandbox.begun[0].1, "chain://operator.1");
  }

  #[test]
  fn the_kernel_says_wants_for_the_act_a_run_waits_for_and_carries_it_at_the_sent() {
    let mut kernel = Native::new(Said {
      steps: vec![Step::Wants("bash://operator.1.1.1".to_owned()), Step::Ran(None)],
      ..Said::default()
    });
    let said = kernel.hears(&run("rung://operator.1.1", "chain://operator.1", "close((await x).code)"));
    assert_eq!(
      said,
      Reply::say(Fact::new(
        "wants",
        "rung://operator.1.1",
        "rung://operator.1.1",
        vec![Value::Str("bash://operator.1.1.1".to_owned())]
      ))
    );
    let sent = Fact::new("sent", "rung://operator.1.1", "chain://operator.1", vec![Value::Int(0)]);
    assert_eq!(
      kernel.hears(&sent),
      Reply::say(Fact::new("ran", "rung://operator.1.1", "rung://operator.1.1", vec![Value::None]))
    );
  }

  #[test]
  fn the_gate_reads_a_word_against_the_rungs_of_its_chain_before_it_and_the_shape_it_must_give() {
    let mut kernel = Native::new(Said { steps: vec![Step::Ran(None)], ..Said::default() });
    let one = Fact::new(
      "gate",
      "gate://operator.1.1",
      "chain://operator.1",
      vec![
        Value::Str("chain://operator.1".to_owned()),
        Value::Str("k = 1".to_owned()),
        Value::Str("int".to_owned()),
      ],
    );
    assert_eq!(
      kernel.hears(&one),
      Reply::say(Fact::new("done", "gate://operator.1.1", "kernel", vec![Value::List(vec![])]))
    );
    kernel.hears(&run("rung://operator.1.1", "chain://operator.1", "k = 1"));
    let two = Fact::new(
      "gate",
      "gate://operator.1.2",
      "chain://operator.1",
      vec![
        Value::Str("chain://operator.1".to_owned()),
        Value::Str("BAD".to_owned()),
        Value::Str("int".to_owned()),
      ],
    );
    let found = kernel.hears(&two);
    assert_eq!(
      found,
      Reply::say(Fact::new(
        "done",
        "gate://operator.1.2",
        "kernel",
        vec![Value::List(vec![Value::Str("BAD in rung, against int after 1 rungs".to_owned())])]
      ))
    );
    assert_eq!(kernel.sandbox.gated[1].1, vec!["k = 1".to_owned()]);
  }

  #[test]
  fn a_cancel_of_a_rung_is_the_kernels_to_do_since_the_kernel_is_the_one_running_the_word() {
    let mut kernel = Native::new(Said {
      steps: vec![Step::Wants("bash://operator.1.1.1".to_owned())],
      ..Said::default()
    });
    kernel.hears(&run("rung://operator.1.1", "chain://operator.1", "await x"));
    let cancel = Fact::new("cancel", "prompt://operator.1", "operator", vec![]);
    let said = kernel.hears(&cancel);
    assert_eq!(kernel.sandbox.dropped, vec!["rung://operator.1.1".to_owned()]);
    match said {
      Reply::Say(held) => {
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].kind(), "ran");
        assert_eq!(held[0].words()[0], Value::Error { name: "CancelledError".to_owned(), args: vec![] });
      }
      other => panic!("a cancel over a run says a ran of its own, and said {other:?}"),
    }
    assert_eq!(kernel.hears(&cancel), Reply::Nothing);
  }
}
