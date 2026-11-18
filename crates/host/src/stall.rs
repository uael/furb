//! The watch of each life that the daemon holds. It stands outside the life: no word and no ear of
//! the life runs it, so it sees a life whose words are lost, which no watch that a word runs can.
//! A life is stalled when it holds an open act, a run of a word, a reply of a model or a command,
//! and says no fact, and no model writes or is asked again, for the bound of the daemon,
//! [`STALL`] unless `host.json` says otherwise. A chain is stalled when it holds a rung with a word
//! while nothing else lives on it, no run, no reply, no command and no wait, and no act of it
//! changes for [`HUNG`], or when a reply of a model on it writes nothing, is not asked again, and
//! no act of it changes for [`THINK`], even while other chains work. That is a chain whose word or
//! model is lost, and the watch wakes that chain alone.
//! A rung with no word, or a prompt to a model, with nothing else on its chain rests: the
//! chain asks a thread only for a fact of its own, so a thread that rests waits for that fact
//! on purpose, and the watch never wakes it. Work that waits for the operator waits on purpose
//! and never stalls: a paused act, a prompt to him, an act on a chain that asks him, and a rung
//! or a run whose word awaits such a prompt.

use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
  time::Duration,
};

use serde_json::Value;
use tokio::time::Instant;

use furb::view::{Act, Kind, Name, Phase, State, operator};

use crate::{event::Event, protocol::Open};

/// How long a life that holds an open act says no fact before the daemon tells that it stalled,
/// unless the settings of the daemon say otherwise.
pub const STALL: Duration = Duration::from_mins(20);

/// How long a chain that holds a rung with a word, with nothing else that lives on it, stays with
/// no change before the daemon tells that it stalled. The bound of the life caps it.
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
  chains: HashMap<Name, Chain>,
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

  /// One event of the life. After the event of a birth, the open tells again each act that lives.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(fact) => self.fact(fact),
      Event::Act(act) => self.act(act),
      Event::Text { chain, .. } | Event::Retried { chain, .. } => self.wrote(chain),
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

  /// What a model wrote for a chain, or a retry of its call, which shows that the life and that
  /// chain live.
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
  /// fact is a fact of an act on it that no message of the watch made. The move of the state of a
  /// chain is no new fact. A chain that ends, and an act of a message that ends, leave the watch.
  fn act(&mut self, act: &Act) {
    let done = matches!(act.phase, Phase::Done(_));
    // An act that asked the operator ends: each act that waited on it stops waiting now, so the
    // bound of its chain runs from now, and not from the last change of that chain before the
    // answer.
    if done && self.asks(act) {
      let now = Instant::now();
      let waited: Vec<Name> = self
        .acts
        .iter()
        .filter(|one| waited_on(one, act))
        .map(|one| Arc::clone(&one.chain))
        .collect();
      for name in waited {
        if let Some(chain) = self.chains.get_mut(&name) {
          chain.last = now;
        }
      }
    }
    if matches!(
      act.kind,
      Kind::Run | Kind::Reply | Kind::Bash | Kind::Wait | Kind::Rung | Kind::Prompt
    ) {
      self.acts.retain(|one| one.id != act.id);
      if !done {
        self.acts.push(act.clone());
      }
    }
    // A chain moves only as the acts on it move, each of which the watch hears for itself, so the
    // move of its state is no fact of its own.
    let moved = act.kind == Kind::Chain && !done && self.chains.contains_key(&*act.id);
    let ours = self.ours.contains(&*act.id) || moved;
    match &self.told {
      Told::Asking { prompt, .. } if *prompt == *act.id && done => self.told = Told::Said,
      _ if ours => {}
      Told::Asking { chain, .. } if *chain == *act.chain => {}
      _ => self.told = Told::No,
    }
    let now = Instant::now();
    let chain = self.chains.entry(Arc::clone(&act.chain)).or_insert_with(|| Chain::new(now));
    match &chain.told {
      Told::Asking { prompt, .. } if *prompt == *act.id && done => chain.told = Told::Said,
      _ if ours => {}
      _ => (chain.last, chain.told) = (now, Told::No),
    }
    if act.kind == Kind::Chain && done {
      self.chains.remove(&*act.id);
    } else if act.kind == Kind::Chain {
      if let Some(chain) = self.chains.get_mut(&*act.id) {
        chain.state = act.state;
      } else {
        self.chains.insert(Arc::clone(&act.id), Chain { state: act.state, ..Chain::new(now) });
      }
    }
    if done {
      self.ours.remove(&*act.id);
    }
  }

  /// What the watch reads of its acts at one look, with one pass over them.
  fn look(&self) -> Look<'_> {
    let mut look = Look::default();
    let kinds = [Kind::Run, Kind::Reply, Kind::Bash, Kind::Wait];
    for one in &self.acts {
      if kinds.contains(&one.kind) {
        look.working.insert(&one.chain);
      }
      if self.asks(one) {
        look.asking.insert(&one.id);
        if one.kind == Kind::Prompt {
          look.asked_by.insert(&one.by);
        }
      }
    }
    look
  }

  /// Whether an act asks the operator or waits for him: a pause holds it, the operator owes it, it
  /// is a prompt to him, or it stands on a chain that asks him or that a pause holds.
  fn asks(&self, one: &Act) -> bool {
    let to = one.words.get(2).and_then(Value::as_str).unwrap_or_default();
    let chain = self.chains.get(&*one.chain).map(|chain| chain.state);
    one.pause.is_some()
      || one.owed()
      || one.kind == Kind::Prompt && operator(to)
      || matches!(chain, Some(State::Asks | State::Paused))
  }

  /// How long the chain of an act stays with no change before the act stalls it, or none for an
  /// act that never stalls its chain: a rung with a word that nothing else lives beside on its
  /// chain stalls it after [`HUNG`], and a reply after [`THINK`], each capped by the bound of the
  /// life. A rung with no word rests, and work that waits on purpose never stalls.
  fn hung(&self, one: &Act, look: &Look<'_>) -> Option<Duration> {
    let worded = one.words.first().and_then(Value::as_str).is_some_and(|word| !word.is_empty());
    let bound = match one.kind {
      Kind::Rung if worded && !look.working.contains(one.chain.as_ref()) => HUNG,
      Kind::Reply => THINK,
      _ => return None,
    };
    (!look.waits(one)).then(|| bound.min(self.bound))
  }

  /// Whether an act stays open with no fact to come only when the life is lost: a run, a reply or a
  /// command that does not wait on purpose, which the bound of the life holds.
  fn lasting(one: &Act, look: &Look<'_>) -> bool {
    matches!(one.kind, Kind::Run | Kind::Reply | Kind::Bash) && !look.waits(one)
  }

  /// When the next stall comes: the stall of the life, while it holds a run, a reply or a command
  /// and the watch did not tell it, or the stall of a chain, while it holds an act that stalls it
  /// and the watch did not tell it.
  pub(crate) fn due(&self) -> Option<Instant> {
    let look = self.look();
    let armed =
      matches!(self.told, Told::No) && self.acts.iter().any(|one| Self::lasting(one, &look));
    let life = armed.then(|| self.last + self.bound);
    let stalls = |one: &Act| {
      let chain = self.chains.get(&*one.chain).filter(|chain| matches!(chain.told, Told::No))?;
      Some(chain.last + self.hung(one, &look)?)
    };
    life.into_iter().chain(self.acts.iter().filter_map(stalls)).min()
  }

  /// Each stall that is due at a time: the stall of the life, whose notice goes to the root, and
  /// the stall of each chain, whose notice goes to that chain. Each one is told once until a new
  /// fact.
  pub(crate) fn stalled(&mut self, now: Instant) -> Vec<Stall> {
    let mut stalls = Vec::new();
    let look = self.look();
    let lasting: Vec<Open> =
      self.acts.iter().filter(|one| Self::lasting(one, &look)).map(open).collect();
    let mut hung: Vec<(Open, Duration)> =
      self.acts.iter().filter_map(|one| Some((open(one), self.hung(one, &look)?))).collect();
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
    while let Some((first, _)) = hung.first() {
      let name = first.chain.clone();
      let mine: Vec<(Open, Duration)>;
      (mine, hung) = hung.into_iter().partition(|(one, _)| one.chain == name);
      let Some(chain) = self.chains.get_mut(name.as_str()) else { continue };
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

/// What the watch reads of its acts at one look: each chain that holds a run, a reply, a command
/// or a wait, each act that asks the operator, and the rung of each prompt that asks him, so that
/// no question of an act passes over each other act, which on a life of thousands of acts held its
/// thread at each event.
#[derive(Default)]
struct Look<'a> {
  working: HashSet<&'a str>,
  asking: HashSet<&'a str>,
  asked_by: HashSet<&'a str>,
}

impl Look<'_> {
  /// Whether an act waits on purpose: it asks the operator, or it is a rung or a run whose word
  /// awaits an act that asks him, the act that the rung retells or a prompt that the word made.
  fn waits(&self, one: &Act) -> bool {
    let (rung, retells) = match one.kind {
      Kind::Rung => (Some(one.id.as_ref()), one.words.get(1).and_then(Value::as_str)),
      Kind::Run => (one.words.first().and_then(Value::as_str), None),
      _ => (None, None),
    };
    self.asking.contains(one.id.as_ref())
      || retells.is_some_and(|id| self.asking.contains(id))
      || rung.is_some_and(|id| self.asked_by.contains(id))
  }
}

/// Whether an act waited on an act that asks the operator, as [`Look::waits`] reads it: a rung that
/// retells it, or the rung that made a prompt and the run of its word.
fn waited_on(one: &Act, asked: &Act) -> bool {
  let made = asked.kind == Kind::Prompt;
  match one.kind {
    Kind::Rung => {
      one.words.get(1).and_then(Value::as_str) == Some(asked.id.as_ref())
        || made && one.id == asked.by
    }
    Kind::Run => made && one.words.first().and_then(Value::as_str) == Some(asked.by.as_ref()),
    _ => false,
  }
}

fn open(one: &Act) -> Open {
  Open { act: one.id.to_string(), kind: one.kind.to_string(), chain: one.chain.to_string() }
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
