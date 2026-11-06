//! The watch of each life that the daemon holds. It stands outside the life: no word and no ear of
//! the life runs it, so it sees a life whose words are lost, which no watch that a word runs can.
//! A life is stalled when it holds an open act, a run of a word, a reply of a model or a command,
//! and says no fact for the bound of the daemon, [`STALL`] unless `host.json` says otherwise.
//! A chain is stalled when it holds a rung, or a prompt to a model, while nothing else lives on it,
//! no run, no reply, no command and no wait, and no act of it changes for [`HUNG`], even while
//! other chains work. That is a chain whose ear is lost, and the watch wakes that chain alone.

use std::{
  collections::{HashMap, HashSet},
  time::Duration,
};

use serde_json::Value;
use tokio::time::Instant;

use furb::view::{Act, Kind, Phase, operator};

use crate::{event::Event, protocol::Open};

/// How long a life that holds an open act says no fact before the daemon tells that it stalled,
/// unless the settings of the daemon say otherwise.
pub const STALL: Duration = Duration::from_mins(20);

/// How long a chain that holds a rung or a prompt to a model, with nothing else that lives on it,
/// stays with no change before the daemon tells that it stalled. The bound of the life caps it.
pub const HUNG: Duration = Duration::from_secs(60);

/// What the watch holds of a life: when it said its last fact, each act that lives and that the
/// watch reads, whether it told the stall of the life that stands, each act that a message of the
/// watch made, and what it holds of each chain.
pub(crate) struct Quiet {
  bound: Duration,
  last: Instant,
  acts: Vec<Act>,
  told: Told,
  ours: HashSet<String>,
  chains: HashMap<String, Chain>,
}

/// What the watch holds of one chain: when an act of it changed last, other than an act of a
/// message of the watch, and whether the watch told the stall of the chain that stands.
struct Chain {
  last: Instant,
  told: Told,
}

/// One stall that the watch tells: each act that stays open, the notice that names them, and the
/// chain that takes the notice as a prompt of None, or none for the root of the life.
pub(crate) struct Stall {
  pub(crate) acts: Vec<Open>,
  pub(crate) notice: String,
  pub(crate) chain: Option<String>,
}

/// Whether the watch told a stall that stands: not yet, or with a message that the life did not
/// say yet, or with the prompt of that message open on its chain, or with that prompt done.
enum Told {
  No,
  Sent(String),
  Asking { prompt: String, chain: String },
  Said,
}

impl Quiet {
  pub(crate) fn new(bound: Duration) -> Self {
    let (last, acts, ours, chains) = (Instant::now(), Vec::new(), HashSet::new(), HashMap::new());
    Quiet { bound, last, acts, told: Told::No, ours, chains }
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

  /// One fact, which shows that the life lives. The fact of the prompt of a message of the watch
  /// names that prompt, and each act that an act of a message made is of the message too.
  fn fact(&mut self, fact: &Value) {
    let word = |at: usize| fact.get(at).and_then(Value::as_str).unwrap_or_default();
    let (kind, about, by) = (word(0), word(1), word(2));
    let sent =
      |told: &Told| kind == "prompt" && matches!(told, Told::Sent(message) if word(5) == message);
    let asking = || Told::Asking { prompt: about.to_owned(), chain: word(3).to_owned() };
    self.last = Instant::now();
    if sent(&self.told) {
      self.told = asking();
      self.ours.insert(about.to_owned());
    } else if let Some(chain) = self.chains.values_mut().find(|one| sent(&one.told)) {
      chain.told = asking();
      self.ours.insert(about.to_owned());
    } else if self.ours.contains(by) && question(kind, about) {
      self.ours.insert(about.to_owned());
    }
  }

  /// One act as it stands after a fact. For the life, a new fact is a fact of an act that no
  /// message of the watch made, and, while the prompt of its message is open, of an act on another
  /// chain than its own, since the actor that the message woke works there. For a chain, a new
  /// fact is a fact of an act on it that no message of the watch made.
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
    let ours = self.ours.contains(&act.id);
    match &self.told {
      Told::Asking { prompt, .. } if *prompt == act.id && done => self.told = Told::Said,
      _ if ours => {}
      Told::Asking { chain, .. } if *chain == act.chain => {}
      _ => self.told = Told::No,
    }
    let now = Instant::now();
    let chain = self.chains.entry(act.chain.clone()).or_insert(Chain { last: now, told: Told::No });
    match &chain.told {
      Told::Asking { prompt, .. } if *prompt == act.id && done => chain.told = Told::Said,
      _ if ours => {}
      _ => *chain = Chain { last: now, told: Told::No },
    }
  }

  /// Whether something else lives on a chain: a run, a reply, a command or a wait, each of which
  /// ends with a fact.
  fn working(&self, chain: &str) -> bool {
    let kinds = [Kind::Run, Kind::Reply, Kind::Bash, Kind::Wait];
    self.acts.iter().any(|one| one.chain == chain && kinds.contains(&one.kind))
  }

  /// Whether an act stalls its chain: a rung that nothing else lives beside on its chain, or a
  /// prompt to a model whose chain holds no rung and nothing else. A paused act and a prompt to the
  /// operator wait on purpose.
  fn hung(&self, one: &Act) -> bool {
    match one.kind {
      Kind::Rung => one.pause.is_none() && !self.working(&one.chain),
      Kind::Prompt => {
        let to = one.words.get(2).and_then(Value::as_str).unwrap_or_default();
        let ruled = self.acts.iter().any(|rung| rung.chain == one.chain && rung.kind == Kind::Rung);
        one.pause.is_none() && !one.operator && !operator(to) && !ruled && !self.working(&one.chain)
      }
      _ => false,
    }
  }

  /// How long a chain stays with no change before it stalls.
  fn wait(&self) -> Duration {
    HUNG.min(self.bound)
  }

  /// When the next stall comes: the stall of the life, while it holds a run, a reply or a command
  /// and the watch did not tell it, or the stall of a chain, while it holds an act that stalls it
  /// and the watch did not tell it.
  pub(crate) fn due(&self) -> Option<Instant> {
    let armed = matches!(self.told, Told::No) && self.acts.iter().any(lasting);
    let life = armed.then(|| self.last + self.bound);
    let stalls = |one: &Act| {
      let chain = self.chains.get(&one.chain)?;
      (matches!(chain.told, Told::No) && self.hung(one)).then(|| chain.last + self.wait())
    };
    life.into_iter().chain(self.acts.iter().filter_map(stalls)).min()
  }

  /// Each stall that is due at a time: the stall of the life, whose notice goes to the root, and
  /// the stall of each chain, whose notice goes to that chain. Each one is told once until a new
  /// fact.
  pub(crate) fn stalled(&mut self, now: Instant) -> Vec<Stall> {
    let mut stalls = Vec::new();
    let lasting: Vec<Open> = self.acts.iter().filter(|one| lasting(one)).map(open).collect();
    if matches!(self.told, Told::No) && self.last + self.bound <= now && !lasting.is_empty() {
      let notice = format!(
        "The life said no fact for {}, and these acts stay open: {}. The watch of the daemon, \
         which no word of the life runs, tells this once until a new fact.",
        spoken(self.bound),
        named(&lasting),
      );
      self.told = Told::Sent(notice.clone());
      stalls.push(Stall { acts: lasting, notice, chain: None });
    }
    let (wait, mut hung): (Duration, Vec<Open>) =
      (self.wait(), self.acts.iter().filter(|one| self.hung(one)).map(open).collect());
    while let Some(first) = hung.first() {
      let name = first.chain.clone();
      let acts;
      (acts, hung) = hung.into_iter().partition(|one| one.chain == name);
      let Some(chain) = self.chains.get_mut(&name) else { continue };
      if !matches!(chain.told, Told::No) || chain.last + wait > now {
        continue;
      }
      let notice = format!(
        "The chain {name} had no change for {}, and these acts on it stay open with nothing that \
         works beside them: {}. The watch of the daemon, which no word of the life runs, tells \
         this to the chain once until a new fact on it.",
        spoken(wait),
        named(&acts),
      );
      chain.told = Told::Sent(notice.clone());
      stalls.push(Stall { acts, notice, chain: Some(name) });
    }
    stalls
  }
}

/// Whether an act stays open with no fact to come only when the life is lost: a run, a reply or a
/// command, which the bound of the life holds.
fn lasting(one: &Act) -> bool {
  matches!(one.kind, Kind::Run | Kind::Reply | Kind::Bash)
}

fn open(one: &Act) -> Open {
  Open { act: one.id.clone(), kind: one.kind.to_string(), chain: one.chain.clone() }
}

/// Each open act as a notice names it, with its kind and its chain.
fn named(open: &[Open]) -> String {
  let named: Vec<String> =
    open.iter().map(|one| format!("{} {} on {}", one.kind, one.act, one.chain)).collect();
  named.join(", ")
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
