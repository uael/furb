//! The watch of each life that the daemon holds. It stands outside the life: no word and no ear of
//! the life runs it, so it sees a life whose words are lost, which no watch that a word runs can.
//! A life is stalled when it holds an open act, a run of a word, a reply of a model or a command,
//! and says no fact for the bound of the daemon, [`STALL`] unless `host.json` says otherwise.
//! A rung, or a prompt to a model, is an open act too when nothing else lives on its chain: no
//! run, no reply, no command and no wait. That is a chain whose ear is lost.

use std::{collections::HashSet, time::Duration};

use serde_json::Value;
use tokio::time::Instant;

use crate::{
  event::Event,
  protocol::Open,
  view::{Act, Kind, Phase, operator},
};

/// How long a life that holds an open act says no fact before the daemon tells that it stalled,
/// unless the settings of the daemon say otherwise.
pub const STALL: Duration = Duration::from_mins(20);

/// What the watch holds of a life: when it said its last fact, each act that lives and that the
/// watch reads, whether it told the stall that stands, and each act that its own message made.
pub(crate) struct Quiet {
  bound: Duration,
  last: Instant,
  acts: Vec<Act>,
  told: Told,
  ours: HashSet<String>,
}

/// Whether the watch told the stall that stands: not yet, or with a message that the life did not
/// say yet, or with the prompt of that message open on its chain, or with that prompt done.
enum Told {
  No,
  Sent(String),
  Asking { prompt: String, chain: String },
  Said,
}

impl Quiet {
  pub(crate) fn new(bound: Duration) -> Self {
    let (last, acts, ours) = (Instant::now(), Vec::new(), HashSet::new());
    Quiet { bound, last, acts, told: Told::No, ours }
  }

  /// One event of the life. After a rebirth, the new open tells again each act that lives.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(fact) => self.fact(fact),
      Event::Act(act) => self.act(act),
      Event::Reborn(_) => self.acts.clear(),
      _ => {}
    }
  }

  /// One fact, which shows that the life lives. The fact of the prompt of the message of the watch
  /// names that prompt, and each act that an act of the message made is of the message too.
  fn fact(&mut self, fact: &Value) {
    let word = |at: usize| fact.get(at).and_then(Value::as_str).unwrap_or_default();
    let (kind, about, by) = (word(0), word(1), word(2));
    self.last = Instant::now();
    if matches!(&self.told, Told::Sent(message) if kind == "prompt" && word(5) == message) {
      self.told = Told::Asking { prompt: about.to_owned(), chain: word(3).to_owned() };
      self.ours.insert(about.to_owned());
    } else if self.ours.contains(by) && question(kind, about) {
      self.ours.insert(about.to_owned());
    }
  }

  /// One act as it stands after a fact. A new fact is a fact of an act that the message of the
  /// watch did not make, and, while the prompt of that message is open, of an act on another
  /// chain than its own, since the actor that the message woke works there.
  fn act(&mut self, act: &Act) {
    let done = matches!(act.phase, Phase::Done(_));
    if matches!(
      act.kind,
      Kind::Run | Kind::Reply | Kind::Bash | Kind::Wait | Kind::Rung | Kind::Prompt
    ) {
      self.acts.retain(|one| one.id != act.id);
      if !done {
        self.acts.push(act.clone());
      }
    }
    match &self.told {
      Told::Asking { prompt, .. } if *prompt == act.id && done => self.told = Told::Said,
      _ if self.ours.contains(&act.id) => {}
      Told::Asking { chain, .. } if *chain == act.chain => {}
      _ => self.told = Told::No,
    }
  }

  /// Whether something else lives on a chain: a run, a reply, a command or a wait, each of which
  /// ends with a fact.
  fn working(&self, chain: &str) -> bool {
    let kinds = [Kind::Run, Kind::Reply, Kind::Bash, Kind::Wait];
    self.acts.iter().any(|one| one.chain == chain && kinds.contains(&one.kind))
  }

  /// The acts that stay open with no fact to come: each run, reply and command, a rung that nothing
  /// else lives beside on its chain, and a prompt to a model whose chain holds no rung and nothing
  /// else. A paused act and a prompt to the operator wait on purpose.
  fn open(&self) -> Vec<Open> {
    let stuck = |one: &&Act| match one.kind {
      Kind::Rung => one.pause.is_none() && !self.working(&one.chain),
      Kind::Prompt => {
        let to = one.words.get(2).and_then(Value::as_str).unwrap_or_default();
        let ruled = self.acts.iter().any(|rung| rung.chain == one.chain && rung.kind == Kind::Rung);
        one.pause.is_none() && !one.operator && !operator(to) && !ruled && !self.working(&one.chain)
      }
      _ => matches!(one.kind, Kind::Run | Kind::Reply | Kind::Bash),
    };
    let open = |one: &Act| Open {
      act: one.id.clone(),
      kind: one.kind.to_string(),
      chain: one.chain.clone(),
    };
    self.acts.iter().filter(stuck).map(open).collect()
  }

  /// When the life stalls, while it holds an open act and the watch did not tell this stall.
  pub(crate) fn due(&self) -> Option<Instant> {
    let armed = matches!(self.told, Told::No) && !self.open().is_empty();
    armed.then(|| self.last + self.bound)
  }

  /// The stall told: each open act, and the notice that names each one and its chain, which the
  /// watch sends to the root as its message.
  pub(crate) fn stalled(&mut self) -> (Vec<Open>, String) {
    let open = self.open();
    let named: Vec<String> =
      open.iter().map(|one| format!("{} {} on {}", one.kind, one.act, one.chain)).collect();
    let notice = format!(
      "The life said no fact for {}, and these acts stay open: {}. The watch of the daemon, \
       which no word of the life runs, tells this once until a new fact.",
      spoken(self.bound),
      named.join(", "),
    );
    self.told = Told::Sent(notice.clone());
    (open, notice)
  }
}

/// Whether a fact is the question that makes its act, whose name is its kind and a number.
fn question(kind: &str, about: &str) -> bool {
  about.strip_prefix(kind).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// A bound as a person reads it: in whole minutes when it is, and else in seconds.
fn spoken(bound: Duration) -> String {
  let (seconds, minutes) = (bound.as_secs(), bound.as_secs() / 60);
  match minutes {
    1 if seconds == 60 && bound.subsec_nanos() == 0 => "1 minute".to_owned(),
    _ if minutes > 0 && seconds % 60 == 0 && bound.subsec_nanos() == 0 => {
      format!("{minutes} minutes")
    }
    _ => format!("{} seconds", bound.as_secs_f64()),
  }
}

#[cfg(test)]
#[path = "stall.test.rs"]
mod tests;
