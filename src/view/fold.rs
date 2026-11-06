//! The fold of the facts of a life into its acts, each with its meaning, as the facts come.

use std::{
  collections::{BTreeMap, HashMap, HashSet},
  mem,
};

use crate::fact::named;
use serde_json::Value;

use super::{
  act::{Act, Answer, Kind, Pause, Phase, RunState, Spend, fault},
  state::{State, chain_state},
};
use crate::world::CONSOLE;

/// The questions that the engine asks on the way to what an act does, which no person follows: each
/// is answered for the act that asked it, which shows what it came to.
const STEPS: [&str; 11] =
  ["setup", "stand", "clock", "chance", "gate", "cd", "merged", "run", "writing", "wants", "reply"];

/// Whether a person follows an act of a kind, so that a pause can hold it.
pub fn followed(kind: &str) -> bool {
  !STEPS.contains(&kind)
}

/// What the fold holds of a chain, each act by the place it began: its live acts of work, its last
/// act of work, its last prompt, and the live prompts that its words made on other chains. These
/// are all the acts that the state of the chain reads.
#[derive(Default)]
struct Chain {
  open: BTreeMap<u64, String>,
  last: Option<String>,
  prompt: Option<String>,
  remote: BTreeMap<u64, String>,
}

/// A reply that is not done: the chain that it stands on, the actor that answers it, and the rung
/// that waits for the word that it writes.
struct Reply {
  chain: String,
  actor: String,
  maker: String,
}

/// The acts of a life, folded from its facts in the order of its log. It reads from each fact what
/// the fact says alone, and takes from the engine what the engine alone can say: which acts a pause
/// or a wake covers.
#[derive(Default)]
pub struct Fold {
  acts: HashMap<String, Act>,
  /// How many facts the fold heard, which is the place in time of the fact that it hears.
  clock: u64,
  chains: HashMap<String, Chain>,
  replies: HashMap<String, Reply>,
  /// The rung of each run, what the run of the word of each rung gave once it is done, and the
  /// findings of the gate on each word that it refused.
  runs: HashMap<String, String>,
  ran: HashMap<String, Value>,
  refused: HashMap<String, String>,
  children: HashMap<String, Vec<String>>,
  /// The acts that a person follows and that are not done, by the place they began.
  live: BTreeMap<u64, String>,
  /// The acts that the fact that the fold hears changed, in order.
  moved: Vec<String>,
}

impl Fold {
  /// The chain that an act stands in.
  pub fn chain_of(&self, id: &str) -> Option<&str> {
    self.acts.get(id).map(|act| act.chain.as_str())
  }

  /// The live acts whose pause a pause, or a wake, would change: each that no pause holds, or each
  /// that one holds.
  pub fn open(&self, pausing: bool) -> Vec<String> {
    let acts = self.live.values().filter_map(|id| self.acts.get(id));
    acts.filter(|act| act.pause.is_some() != pausing).map(|act| act.id.clone()).collect()
  }

  /// The acts that a fact changes, each as it stands after the fact. `covered` names each live act
  /// that a pause or a wake covers, and `born` the pause that holds a new act, as the engine found.
  pub fn hear(&mut self, fact: &Value, covered: &[String], born: Option<Pause>) -> Vec<Act> {
    self.clock += 1;
    let text = |at: usize| fact.get(at).and_then(Value::as_str).unwrap_or_default();
    let (kind, about, by) = (text(0), text(1), text(2));
    let rest = fact.as_array().map_or(&[][..], |all| all.get(3..).unwrap_or_default());
    if named(about, kind) {
      self.asked([kind, about, by], rest, born);
    } else {
      self.said([kind, about, by], fact, covered);
    }
    self.settled()
  }

  /// A new act: its kind, its name and its maker, then what it is on and its words.
  fn asked(&mut self, [kind, id, by]: [&str; 3], rest: &[Value], born: Option<Pause>) {
    let text = |at: usize| rest.get(at).and_then(Value::as_str).unwrap_or_default().to_owned();
    let on = text(0);
    let followed = followed(kind);
    let kind = Kind::from(kind);
    let chain = if kind == Kind::Chain {
      id.to_owned()
    } else {
      self.chain_of(&on).unwrap_or(&on).to_owned()
    };
    match kind {
      Kind::Run => {
        let rung = text(1);
        if let Some(act) =
          self.acts.get_mut(&rung).filter(|act| act.kind == Kind::Rung && act.word.is_none())
        {
          act.word = Some(text(2));
          self.moved.push(rung.clone());
        }
        self.runs.insert(id.to_owned(), rung);
      }
      Kind::Reply => {
        let reply = Reply { chain: on.clone(), actor: text(1), maker: by.to_owned() };
        self.replies.insert(id.to_owned(), reply);
      }
      _ => {}
    }
    let run = (kind == Kind::Rung).then_some(RunState::Running);
    let act = Act {
      kind,
      id: id.to_owned(),
      by: by.to_owned(),
      on,
      chain,
      words: rest.get(1..).unwrap_or_default().to_vec(),
      phase: Phase::Asked,
      operator: false,
      pause: born.filter(|_| followed),
      run,
      word: None,
      began: self.clock,
      ended: None,
      spend: None,
      state: State::Idle,
    };
    self.placed(&act);
    self.moved.push(act.id.clone());
    self.acts.insert(act.id.clone(), act);
  }

  /// A new act in the places that the fold reads it from: the live acts, the children of its maker,
  /// and the acts of its chain and of the chain of its maker that its state reads.
  fn placed(&mut self, act: &Act) {
    if !followed(act.kind.as_str()) {
      return;
    }
    self.live.insert(act.began, act.id.clone());
    self.children.entry(act.by.clone()).or_default().push(act.id.clone());
    if !act.kind.work() {
      return;
    }
    let held = self.chains.entry(act.chain.clone()).or_default();
    held.open.insert(act.began, act.id.clone());
    held.last = Some(act.id.clone());
    if act.kind == Kind::Prompt {
      held.prompt = Some(act.id.clone());
      if let Some(maker) = self.remote_of(act) {
        self.chains.entry(maker).or_default().remote.insert(act.began, act.id.clone());
      }
    }
  }

  /// The chain of the rung that made a prompt on another chain.
  fn remote_of(&self, prompt: &Act) -> Option<String> {
    let maker = self.acts.get(&prompt.by).filter(|maker| maker.kind == Kind::Rung)?;
    (prompt.kind == Kind::Prompt && maker.chain != prompt.chain).then(|| maker.chain.clone())
  }

  /// A fact about an act: its kind, what it is about and who said it, then the fact whole.
  fn said(&mut self, [kind, about, by]: [&str; 3], fact: &Value, covered: &[String]) {
    match kind {
      "pause" | "wake" => {
        let pause = Pause::of(fact);
        for id in covered {
          if let Some(act) = self.acts.get_mut(id) {
            act.pause.clone_from(&pause);
            self.moved.push(id.clone());
          }
        }
      }
      "started" => {
        if let Some(act) = self.acts.get_mut(about) {
          act.phase = Phase::Started;
          act.operator |= by == CONSOLE;
          self.moved.push(about.to_owned());
        }
      }
      "done" => self.done(about, fact.get(3).cloned().unwrap_or_default()),
      "ready" => {
        let word = fact.get(3).and_then(Value::as_str);
        if let Some((act, word)) =
          self.acts.get_mut(about).filter(|act| act.kind == Kind::Rung).zip(word)
        {
          act.word = Some(word.to_owned());
          self.moved.push(about.to_owned());
        }
      }
      "tell" => {
        let header = fact.get(3).and_then(|notes| notes.get(0)).and_then(Value::as_str);
        let lines = header.and_then(|note| note.strip_prefix(&format!("#{about} refused\n")));
        if let Some(found) = lines.and_then(|lines| bound(lines, &format!("{about}_findings"))) {
          self.refused.insert(about.to_owned(), found);
          self.update_run(about);
        }
      }
      _ => {}
    }
  }

  /// The end of an act, with what it came to: the answer of a reply costs its chain and its rung,
  /// the run of a word ends its rung, and the act ends with its value.
  fn done(&mut self, about: &str, value: Value) {
    if let Some(reply) = self.replies.remove(about) {
      let usage = value.get(2).and_then(Value::as_array);
      if let Some(usage) = usage {
        let part = |at: usize| usage.get(at).and_then(Value::as_f64).unwrap_or_default();
        let answer =
          Answer { actor: reply.actor, usage: [part(0), part(1), part(2), part(3), part(4)] };
        for id in [reply.chain, reply.maker] {
          if let Some(act) =
            self.acts.get_mut(&id).filter(|act| matches!(act.kind, Kind::Chain | Kind::Rung))
          {
            act.spend.get_or_insert_with(Spend::default).add(&answer);
            self.moved.push(id);
          }
        }
      }
    }
    if let Some(rung) = self.runs.get(about).cloned() {
      self.ran.insert(rung.clone(), value.clone());
      self.update_run(&rung);
    }
    let Some(act) = self.acts.get(about) else { return };
    let (began, chain, remote) = (act.began, act.chain.clone(), self.remote_of(act));
    self.live.remove(&began);
    if let Some(held) = self.chains.get_mut(&chain) {
      held.open.remove(&began);
    }
    if let Some(held) = remote.and_then(|maker| self.chains.get_mut(&maker)) {
      held.remote.remove(&began);
    }
    if let Some(act) = self.acts.get_mut(about) {
      act.phase = Phase::Done(value);
      act.ended.get_or_insert(self.clock);
      self.moved.push(about.to_owned());
    }
    self.update_run(about);
    for child in self.children.get(about).cloned().unwrap_or_default() {
      self.update_run(&child);
    }
  }

  /// How the word of a rung ran, from the findings of the gate, the value of its run, and its own
  /// value. A cancel ends it well when it retells, when the rung itself ended well, or when its
  /// prompt did.
  fn update_run(&mut self, id: &str) {
    let Some(act) = self.acts.get(id).filter(|act| act.kind == Kind::Rung) else { return };
    let ran = self.ran.get(id);
    let run = match (self.refused.get(id), ran.or(act.value()).and_then(fault)) {
      (Some(found), _) => RunState::Refused(found.clone()),
      (None, Some((is, args))) => {
        let well = |one: &Act| one.done() && one.value().and_then(fault).is_none();
        let parent = self.acts.get(&act.by).filter(|one| one.kind == Kind::Prompt);
        let retells = act.words.get(1).is_some_and(truthy);
        let reason = format!("{is}: {}", args.iter().map(display).collect::<Vec<_>>().join(", "));
        match is {
          "CancelledError" if retells || well(act) || parent.is_some_and(well) => RunState::Done,
          "CancelledError" => RunState::Cancelled(reason),
          _ => RunState::Failed(reason),
        }
      }
      (None, None) if ran.is_some() => RunState::Done,
      (None, None) => RunState::Running,
    };
    if let Some(act) = self.acts.get_mut(id) {
      act.run = Some(run);
      self.moved.push(id.to_owned());
    }
  }

  /// The acts that the fact moved, each in its state, and each chain whose state the fact changed.
  fn settled(&mut self) -> Vec<Act> {
    let mut moved = mem::take(&mut self.moved);
    let mut seen = HashSet::new();
    moved.retain(|id| seen.insert(id.clone()));
    let mut chains: Vec<String> = Vec::new();
    for id in &moved {
      let Some(act) = self.acts.get(id) else { continue };
      chains.push(act.chain.clone());
      chains.extend(self.remote_of(act));
      if act.kind != Kind::Chain {
        let state = State::of(act);
        self.acts.entry(id.clone()).and_modify(|act| act.state = state);
      }
    }
    for chain in chains {
      let state = self.chain_state(&chain);
      let Some(act) = self.acts.get_mut(&chain).filter(|act| act.kind == Kind::Chain) else {
        continue;
      };
      if act.state != state {
        act.state = state;
        if !moved.contains(&chain) {
          moved.push(chain);
        }
      }
    }
    moved.iter().filter_map(|id| self.acts.get(id)).cloned().collect()
  }

  /// The state of a chain, from the acts of it that the state reads.
  fn chain_state(&self, chain: &str) -> State {
    let Some(held) = self.chains.get(chain) else { return chain_state(&[], &[]) };
    let mut on: Vec<&Act> = held
      .open
      .values()
      .chain(&held.last)
      .chain(&held.prompt)
      .filter_map(|id| self.acts.get(id))
      .collect();
    on.sort_by_key(|act| act.began);
    on.dedup_by_key(|act| act.began);
    let remote: Vec<&Act> = held.remote.values().filter_map(|id| self.acts.get(id)).collect();
    chain_state(&on, &remote)
  }
}

/// Whether a value is true as python reads it.
fn truthy(value: &Value) -> bool {
  match value {
    Value::Null => false,
    Value::Bool(one) => *one,
    Value::Number(one) => one.as_f64().is_some_and(|one| one != 0.0),
    Value::String(one) => !one.is_empty(),
    Value::Array(all) => !all.is_empty(),
    Value::Object(all) => !all.is_empty(),
  }
}

/// A value as a person reads it: a string as it is, and any other value as JSON.
fn display(value: &Value) -> String {
  match value {
    Value::String(one) => one.clone(),
    one => serde_json::to_string_pretty(one).unwrap_or_default(),
  }
}

/// The string that the lines of a note bind to a name: the text of its quote, whose name may take
/// more underscores, or the string of its statement as python writes it.
fn bound(lines: &str, name: &str) -> Option<String> {
  let all: Vec<&str> = lines.split('\n').collect();
  for (at, line) in all.iter().enumerate() {
    let quote = line.strip_prefix("<s:").and_then(|rest| rest.strip_suffix('>'));
    if let Some(quote) =
      quote.filter(|one| one.strip_prefix(name).is_some_and(|tail| tail.bytes().all(|b| b == b'_')))
    {
      let close = format!("</s:{quote}>");
      let end = all[at + 1..].iter().position(|one| one.ends_with(&close))? + at + 1;
      let last = &all[end][..all[end].len() - close.len()];
      let body: Vec<&str> = all[at + 1..end].iter().copied().chain([last]).collect();
      return Some(body.join("\n"));
    }
    if let Some(said) = line.strip_prefix(name).and_then(|rest| rest.strip_prefix(" = ")) {
      return python(said);
    }
  }
  None
}

/// A string as python writes it, read back: the text between its quotes, with each escape as the
/// character that it stands for.
fn python(said: &str) -> Option<String> {
  let inner = said.get(1..said.len().checked_sub(1)?)?;
  if !matches!(said.chars().next(), Some('\'' | '"'))
    || !matches!(said.chars().last(), Some('\'' | '"'))
  {
    return None;
  }
  let mut out = String::new();
  let mut chars = inner.chars();
  while let Some(one) = chars.next() {
    if one != '\\' {
      out.push(one);
      continue;
    }
    let code = |chars: &mut std::str::Chars<'_>, n: usize| {
      let hex: String = chars.by_ref().take(n).collect();
      u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
    };
    match chars.next() {
      Some('n') => out.push('\n'),
      Some('r') => out.push('\r'),
      Some('t') => out.push('\t'),
      Some('x') => out.extend(code(&mut chars, 2)),
      Some('u') => out.extend(code(&mut chars, 4)),
      Some('U') => out.extend(code(&mut chars, 8)),
      Some(other) => out.push(other),
      None => {}
    }
  }
  Some(out)
}
