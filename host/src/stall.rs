//! The watch of each life that the daemon holds. It stands outside the life: no word and no ear of
//! the life runs it, so it sees a life whose words are lost, which no watch that a word runs can.
//! A life is stalled when it holds an open act, a run of a word, a reply of a model or a command,
//! and says no fact and no model writes for the bound of the daemon, [`STALL`] unless `host.json`
//! says otherwise. A chain is stalled when it holds a rung, or a prompt to a model, while nothing
//! else lives on it, no run, no reply, no command and no wait, and no act of it changes for
//! [`HUNG`], or when a reply of a model on it writes nothing and no act of it changes for
//! [`THINK`], even while other chains work. That is a chain whose ear or model is lost, and the
//! watch wakes that chain alone. Work that waits for the operator waits on purpose and never
//! stalls: a paused act, a prompt to him, an act on a chain that asks him, and a rung or a run
//! whose word awaits such a prompt.

use std::{
  collections::{HashMap, HashSet},
  time::Duration,
};

use serde_json::Value;
use tokio::time::Instant;

use furb::view::{Act, Kind, Phase, State, operator};

use crate::{event::Event, protocol::Open};

/// How long a life that holds an open act says no fact before the daemon tells that it stalled,
/// unless the settings of the daemon say otherwise.
pub const STALL: Duration = Duration::from_mins(20);

/// How long a chain that holds a rung or a prompt to a model, with nothing else that lives on it,
/// stays with no change before the daemon tells that it stalled. The bound of the life caps it.
pub const HUNG: Duration = Duration::from_secs(60);

/// How long a reply of a model writes nothing while no act of its chain changes before the daemon
/// tells that its chain stalled: a model that thinks that long with no word is stuck. The bound of
/// the life caps it.
pub(crate) const THINK: Duration = Duration::from_mins(5);

/// What the watch holds of a life: when it said its last fact, each act that lives and that the
/// watch reads, whether it told the stall of the life that stands, each act of a message of the
/// watch that lives, and what it holds of each chain that lives.
pub(crate) struct Quiet {
  bound: Duration,
  last: Instant,
  acts: Vec<Act>,
  told: Told,
  ours: HashSet<String>,
  chains: HashMap<String, Chain>,
}

/// What the watch holds of one chain: when an act of it changed or a model wrote for it last,
/// other than an act of a message of the watch, whether the watch told the stall of the chain that
/// stands, and the state of its work.
struct Chain {
  last: Instant,
  told: Told,
  state: State,
}

impl Chain {
  fn new(last: Instant) -> Self {
    Self { last, told: Told::No, state: State::Idle }
  }
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
    Self { bound, last, acts, told: Told::No, ours, chains }
  }

  /// One event of the life. After a rebirth, the new open tells again each act that lives.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(fact) => self.fact(fact),
      Event::Act(act) => self.act(act),
      Event::Text { chain, .. } => self.wrote(chain),
      Event::Reborn(_) => {
        self.acts.clear();
        self.ours.clear();
        self.chains.clear();
      }
      _ => {}
    }
  }

  /// One fact, which shows that the life lives. The fact of the prompt of a message of the watch
  /// names that prompt, and each act that an act of a message made is of the message too, as is
  /// the run of the word of a rung of a message, which its chain makes.
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
    } else if question(kind, about)
      && (self.ours.contains(by) || kind == "run" && self.ours.contains(word(4)))
    {
      self.ours.insert(about.to_owned());
    }
  }

  /// What a model wrote for a chain, which shows that the life and that chain live.
  fn wrote(&mut self, chain: &str) {
    let now = Instant::now();
    self.last = now;
    if let Some(one) = self.chains.get_mut(chain) {
      one.last = now;
    }
  }

  /// One act as it stands after a fact. For the life, a new fact is a fact of an act that no
  /// message of the watch made, and, while the prompt of its message is open, of an act on another
  /// chain than its own, since the actor that the message woke works there. For a chain, a new
  /// fact is a fact of an act on it that no message of the watch made. A chain that ends, and an
  /// act of a message that ends, leave the watch.
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
    let chain = self.chains.entry(act.chain.clone()).or_insert_with(|| Chain::new(now));
    match &chain.told {
      Told::Asking { prompt, .. } if *prompt == act.id && done => chain.told = Told::Said,
      _ if ours => {}
      _ => (chain.last, chain.told) = (now, Told::No),
    }
    if act.kind == Kind::Chain && done {
      self.chains.remove(&act.id);
    } else if act.kind == Kind::Chain {
      self.chains.entry(act.id.clone()).or_insert_with(|| Chain::new(now)).state = act.state;
    }
    if done {
      self.ours.remove(&act.id);
    }
  }

  /// Whether something else lives on a chain: a run, a reply, a command or a wait, each of which
  /// ends with a fact.
  fn working(&self, chain: &str) -> bool {
    let kinds = [Kind::Run, Kind::Reply, Kind::Bash, Kind::Wait];
    self.acts.iter().any(|one| one.chain == chain && kinds.contains(&one.kind))
  }

  /// Whether an act asks the operator or waits for him: a pause holds it, the operator owes it, it
  /// is a prompt to him, or it stands on a chain that asks him or that a pause holds.
  fn asks(&self, one: &Act) -> bool {
    let to = one.words.get(2).and_then(Value::as_str).unwrap_or_default();
    let chain = self.chains.get(&one.chain).map(|chain| chain.state);
    one.pause.is_some()
      || one.owed()
      || one.kind == Kind::Prompt && operator(to)
      || matches!(chain, Some(State::Asks | State::Paused))
  }

  /// Whether an act waits on purpose: it asks the operator, or it is a rung or a run whose word
  /// awaits an act that asks him, the act that the rung retells or a prompt that the word made.
  fn waits(&self, one: &Act) -> bool {
    let (rung, retells) = match one.kind {
      Kind::Rung => (Some(one.id.as_str()), one.words.get(1).and_then(Value::as_str)),
      Kind::Run => (one.words.first().and_then(Value::as_str), None),
      _ => (None, None),
    };
    let awaited = |other: &Act| {
      Some(other.id.as_str()) == retells
        || other.kind == Kind::Prompt && Some(other.by.as_str()) == rung
    };
    self.asks(one) || self.acts.iter().any(|other| awaited(other) && self.asks(other))
  }

  /// How long the chain of an act stays with no change before the act stalls it, or none for an
  /// act that never stalls its chain: a rung that nothing else lives beside on its chain, or a
  /// prompt to a model whose chain holds no rung and nothing else, for [`HUNG`], and a reply for
  /// [`THINK`], each capped by the bound of the life. Work that waits on purpose never stalls.
  fn hung(&self, one: &Act) -> Option<Duration> {
    let ruled = || self.acts.iter().any(|rung| rung.chain == one.chain && rung.kind == Kind::Rung);
    let bound = match one.kind {
      Kind::Rung if !self.working(&one.chain) => HUNG,
      Kind::Prompt if !self.working(&one.chain) && !ruled() => HUNG,
      Kind::Reply => THINK,
      _ => return None,
    };
    (!self.waits(one)).then(|| bound.min(self.bound))
  }

  /// Whether an act stays open with no fact to come only when the life is lost: a run, a reply or a
  /// command that does not wait on purpose, which the bound of the life holds.
  fn lasting(&self, one: &Act) -> bool {
    matches!(one.kind, Kind::Run | Kind::Reply | Kind::Bash) && !self.waits(one)
  }

  /// When the next stall comes: the stall of the life, while it holds a run, a reply or a command
  /// and the watch did not tell it, or the stall of a chain, while it holds an act that stalls it
  /// and the watch did not tell it.
  pub(crate) fn due(&self) -> Option<Instant> {
    let armed = matches!(self.told, Told::No) && self.acts.iter().any(|one| self.lasting(one));
    let life = armed.then(|| self.last + self.bound);
    let stalls = |one: &Act| {
      let chain = self.chains.get(&one.chain).filter(|chain| matches!(chain.told, Told::No))?;
      Some(chain.last + self.hung(one)?)
    };
    life.into_iter().chain(self.acts.iter().filter_map(stalls)).min()
  }

  /// Each stall that is due at a time: the stall of the life, whose notice goes to the root, and
  /// the stall of each chain, whose notice goes to that chain. Each one is told once until a new
  /// fact.
  pub(crate) fn stalled(&mut self, now: Instant) -> Vec<Stall> {
    let mut stalls = Vec::new();
    let lasting: Vec<Open> = self.acts.iter().filter(|one| self.lasting(one)).map(open).collect();
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
    let mut hung: Vec<(Open, Duration)> =
      self.acts.iter().filter_map(|one| Some((open(one), self.hung(one)?))).collect();
    while let Some((first, _)) = hung.first() {
      let name = first.chain.clone();
      let mine: Vec<(Open, Duration)>;
      (mine, hung) = hung.into_iter().partition(|(one, _)| one.chain == name);
      let Some(chain) = self.chains.get_mut(&name) else { continue };
      let due: Vec<(Open, Duration)> =
        mine.into_iter().filter(|(_, bound)| chain.last + *bound <= now).collect();
      let Some(wait) = due.iter().map(|(_, bound)| *bound).min() else { continue };
      if !matches!(chain.told, Told::No) {
        continue;
      }
      let acts: Vec<Open> = due.into_iter().map(|(one, _)| one).collect();
      let notice = format!(
        "The chain {name} had no change for {}, and these acts on it stay open with no progress: \
         {}. The watch of the daemon, which no word of the life runs, tells this to the chain \
         once until a new fact on it.",
        spoken(wait),
        named(&acts),
      );
      chain.told = Told::Sent(notice.clone());
      stalls.push(Stall { acts, notice, chain: Some(name) });
    }
    stalls
  }
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
