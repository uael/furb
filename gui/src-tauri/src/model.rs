//! The model of the views: the life as it stood at one fact of its log, read off the facts alone.
//!
//! Every view of the app draws a [`Life`], which this module makes from the facts that the app heard, up to a
//! cursor. A fact is a list of JSON as the wire gives it: its kind, the act it is about, who said it, and its words.
//! A question is a fact whose act is named by its kind, and its first word is the chain it is on.

use std::collections::HashMap;

use serde_json::Value;

/// The operator, as the facts name it.
pub const OPERATOR: &str = "operator";

/// What a thing of the life is doing, from the most urgent to the calmest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum State {
  /// It waits for the operator.
  Asks,
  /// It failed: a refusal, a raise, or a command that exited with a code that is not zero.
  Failed,
  /// A pause holds it.
  Paused,
  /// It runs.
  Runs,
  /// It waits for an act of another program.
  Waits,
  /// It ended well.
  Done,
  /// It holds nothing that runs.
  Rest,
}

/// One act of the life, as its question said it and as its facts settled it.
#[derive(Debug, Clone, PartialEq)]
pub struct Act {
  pub id: String,
  pub kind: String,
  pub by: String,
  pub on: String,
  /// The words of the question after the chain.
  pub words: Vec<Value>,
  /// The place in the log where it was made, where it was started, and where it was done.
  pub made: usize,
  pub started: Option<usize>,
  pub ended: Option<usize>,
  /// Who took it, when an ear said its started: the World, the console of the operator, or the act itself.
  pub taker: String,
  pub outcome: Option<Value>,
}

impl Act {
  /// One word of the question, as text, or nothing.
  pub fn text(&self, at: usize) -> &str {
    self.words.get(at).and_then(Value::as_str).unwrap_or_default()
  }

  /// Whether its done stands.
  pub fn over(&self) -> bool {
    self.ended.is_some()
  }

  /// Whether it came to an exception.
  pub fn failed(&self) -> bool {
    self.outcome.as_ref().is_some_and(|one| fault(one).is_some_and(|name| name != "CancelledError"))
      || self
        .outcome
        .as_ref()
        .and_then(|one| one.get("code"))
        .and_then(Value::as_i64)
        .is_some_and(|code| code != 0)
  }
}

/// The name of the exception that a value is, if it is one: a map whose `is` names a type with `args`.
pub fn fault(value: &Value) -> Option<&str> {
  let name = value.get("is")?.as_str()?;
  (value.get("args").is_some() && !matches!(name, "int" | "float" | "tuple" | "dict"))
    .then_some(name)
}

/// One rung of a ladder: the word it ran, and what came of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Rung {
  pub id: String,
  pub word: String,
  pub state: State,
  /// What the gate found, for a word that it refused.
  pub findings: Option<String>,
  /// What the word raised.
  pub raised: Option<String>,
  /// The acts that the word made, in order.
  pub acts: Vec<String>,
  /// The act that the word waits for now.
  pub awaits: Option<String>,
  /// Whether the model of the rung writes its word now.
  pub thinks: bool,
  pub made: usize,
}

/// A thread: a prompt, with its ladder, and the threads that the rungs of its ladder made.
#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
  pub prompt: String,
  pub depth: usize,
  pub state: State,
  /// The prompts of nothing that the chain made to tell this thread of an act of it that is done.
  pub messages: Vec<String>,
}

/// A message between two programs: a prompt from a rung of one chain to another chain or to the operator, and the
/// answer that comes back along it.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
  pub prompt: String,
  /// The chain of the rung that asked, or the operator.
  pub from: String,
  /// The rung that asked, when a rung asked.
  pub rung: Option<String>,
  /// The chain that the prompt is on, or the operator for a question to the operator.
  pub to: String,
  pub state: State,
  /// Where it was asked and where it was answered.
  pub made: usize,
  pub ended: Option<usize>,
}

/// One frame of the call stack: an act that waits, on its chain.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
  pub act: String,
  pub chain: String,
  /// What it waits for.
  pub awaits: Option<String>,
}

/// One chain: a program of the life.
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
  pub id: String,
  pub label: String,
  pub source: String,
  /// The chain the chain fact stands on, which made it, and nothing for the root.
  pub parent: String,
  pub depth: usize,
  /// The place among the chains, in the order they were made.
  pub born: usize,
  pub made: usize,
  pub actor: String,
  /// The dollars of its replies, and the share of the window of the last.
  pub cost: f64,
  pub share: f64,
  /// The cost and the share after each reply, by its place in the log.
  pub ledger: Vec<(usize, f64, f64)>,
  /// The prompts on the chain, in order.
  pub prompts: Vec<String>,
}

/// One binding that a chain told: a name and its value as python shows it, and where it was told.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
  pub name: String,
  pub value: String,
  pub at: usize,
}

/// The life at one fact of its log.
#[derive(Debug, Default, Clone)]
pub struct Life {
  /// The cursor: how many facts the life had said, and how many it said in all.
  pub at: usize,
  pub len: usize,
  pub acts: HashMap<String, Act>,
  /// Every act, in the order the life made them.
  pub order: Vec<String>,
  /// The chains, in the order the life made them.
  pub chains: Vec<String>,
  pub chain: HashMap<String, Chain>,
  /// The paragraphs told on each chain, each with its place in the log and the act it is of.
  pub told: HashMap<String, Vec<(usize, String, String)>>,
  /// What each command wrote, as it came.
  pub streams: HashMap<String, String>,
  /// The pauses that stand, by the act or the chain they name.
  pub paused: HashMap<String, bool>,
  /// The windows of the roster, by the name of each model.
  pub windows: HashMap<String, f64>,
  /// The word that each rung said ready.
  pub words: HashMap<String, String>,
}

/// Whether a fact is a question: its act is named by its kind and a number.
pub fn question(kind: &str, about: &str) -> bool {
  about.strip_prefix(kind).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The kind of an act, from its name.
pub fn kind_of(id: &str) -> &str {
  id.trim_end_matches(|one: char| one.is_ascii_digit())
}

impl Life {
  /// The life as it stood after the first `at` facts of a log.
  pub fn of(facts: &[Value], at: usize) -> Life {
    let at = at.min(facts.len());
    let mut life = Life { at, len: facts.len(), ..Life::default() };
    for (place, fact) in facts[..at].iter().enumerate() {
      life.heard(place, fact);
    }
    life
  }

  /// One fact, at its place in the log.
  fn heard(&mut self, place: usize, fact: &Value) {
    let Some(items) = fact.as_array() else { return };
    let text = |at: usize| items.get(at).and_then(Value::as_str).unwrap_or_default();
    let (kind, about, by) = (text(0), text(1), text(2));
    let words = items.get(3..).unwrap_or_default();
    if question(kind, about) {
      self.asked(place, kind, about, by, words);
      return;
    }
    match kind {
      "started" => {
        if let Some(act) = self.acts.get_mut(about)
          && act.started.is_none()
        {
          act.started = Some(place);
          act.taker = by.to_owned();
        }
      }
      "done" => self.done(place, about, words.first().cloned().unwrap_or(Value::Null)),
      "tell" => {
        let notes: Vec<&str> = words
          .first()
          .and_then(Value::as_array)
          .into_iter()
          .flatten()
          .filter_map(Value::as_str)
          .collect();
        let chain = self.scope(about).to_owned();
        self.told.entry(chain).or_default().push((place, about.to_owned(), notes.join("\n")));
      }
      "ready" => {
        self.words.insert(about.to_owned(), text(3).to_owned());
      }
      "out" => self.streams.entry(about.to_owned()).or_default().push_str(text(3)),
      "pause" | "wake" => {
        self.paused.insert(about.to_owned(), kind == "pause");
      }
      _ => {}
    }
  }

  /// A question: a new act, or a chain.
  fn asked(&mut self, place: usize, kind: &str, about: &str, by: &str, words: &[Value]) {
    if self.acts.contains_key(about) {
      return;
    }
    let on = words.first().and_then(Value::as_str).unwrap_or_default().to_owned();
    let act = Act {
      id: about.to_owned(),
      kind: kind.to_owned(),
      by: by.to_owned(),
      on: on.clone(),
      words: words.get(1..).unwrap_or_default().to_vec(),
      made: place,
      started: None,
      ended: None,
      taker: String::new(),
      outcome: None,
    };
    if kind == "chain" {
      let depth = self.chain.get(&on).map_or(0, |one| one.depth + 1);
      let chain = Chain {
        id: about.to_owned(),
        label: act.text(0).to_owned(),
        source: act.text(1).to_owned(),
        parent: on.clone(),
        depth,
        born: self.chains.len(),
        made: place,
        actor: self.chain.get(&on).map(|one| one.actor.clone()).unwrap_or_default(),
        cost: 0.0,
        share: 0.0,
        ledger: Vec::new(),
        prompts: Vec::new(),
      };
      self.chains.push(about.to_owned());
      self.chain.insert(about.to_owned(), chain);
    }
    if kind == "prompt"
      && let Some(chain) = self.chain.get_mut(&on)
    {
      chain.prompts.push(about.to_owned());
    }
    if kind == "reply"
      && let Some(chain) = self.chain.get_mut(&on)
    {
      chain.actor = act.text(0).to_owned();
    }
    self.order.push(about.to_owned());
    self.acts.insert(about.to_owned(), act);
  }

  /// A done: the first fills the outcome of its act.
  fn done(&mut self, place: usize, about: &str, value: Value) {
    let Some(act) = self.acts.get_mut(about) else { return };
    if act.ended.is_some() {
      return;
    }
    act.ended = Some(place);
    match act.kind.as_str() {
      "reply" => {
        let usage = value.get(2);
        let tokens = usage.and_then(|one| one.get(0)).and_then(Value::as_f64).unwrap_or_default();
        let dollars = usage.and_then(|one| one.get(4)).and_then(Value::as_f64).unwrap_or_default();
        let actor = act.text(0).to_owned();
        let model = actor.split('/').next().unwrap_or_default();
        let window = self.windows.get(model).copied().unwrap_or(200_000.0);
        if let Some(chain) = self.chain.get_mut(&act.on) {
          chain.cost += dollars;
          chain.share = tokens / window;
          chain.ledger.push((place, chain.cost, chain.share));
        }
      }
      "stand" => {
        for one in value.get(0).and_then(Value::as_array).into_iter().flatten() {
          let name = one.get(0).and_then(Value::as_str).unwrap_or_default();
          let window = one.get(2).and_then(Value::as_f64).unwrap_or(200_000.0);
          self.windows.insert(name.to_owned(), window);
        }
      }
      _ => {}
    }
    act.outcome = Some(value);
  }

  /// One act by its name.
  pub fn act(&self, id: &str) -> Option<&Act> {
    self.acts.get(id)
  }

  /// The chain an act is on, and itself for a chain.
  pub fn scope<'a>(&'a self, id: &'a str) -> &'a str {
    match self.acts.get(id) {
      Some(act) if act.kind == "chain" => id,
      Some(act) => &act.on,
      None => "",
    }
  }

  /// Whether a pause stands over an act: over it, or over its chain.
  pub fn holds(&self, id: &str) -> bool {
    let over = |one: &str| self.paused.get(one).copied().unwrap_or(false);
    over(id) || over(self.scope(id))
  }

  /// Whether a prompt waits for the operator: the console took it, and it is not done.
  pub fn owed(&self, prompt: &str) -> bool {
    self
      .acts
      .get(prompt)
      .is_some_and(|act| act.kind == "prompt" && act.taker == "console" && !act.over())
  }

  /// Every prompt that waits for the operator, the oldest first.
  pub fn debts(&self) -> Vec<String> {
    self.order.iter().filter(|one| self.owed(one)).cloned().collect()
  }

  /// The acts that one act made, in order.
  pub fn made_by(&self, id: &str) -> Vec<String> {
    self
      .order
      .iter()
      .filter(|one| self.acts.get(*one).is_some_and(|act| act.by == id))
      .cloned()
      .collect()
  }

  /// The paragraph of an act whose header says one thing, as #rung5 refused, without its header.
  fn paragraph(&self, id: &str, what: &str) -> Option<String> {
    let header = format!("#{id} {what}");
    let chain = self.scope(id);
    let told = self.told.get(chain)?;
    let found =
      told.iter().rev().find(|(_, _, text)| text.lines().next() == Some(header.as_str()))?;
    Some(found.2.lines().skip(1).collect::<Vec<_>>().join("\n"))
  }

  /// The rungs of the ladder of a prompt, in order.
  pub fn ladder(&self, prompt: &str) -> Vec<Rung> {
    let rungs = self.made_by(prompt).into_iter().filter(|one| kind_of(one) == "rung");
    rungs.filter_map(|id| self.rung(&id)).collect()
  }

  /// One rung, read off its facts.
  pub fn rung(&self, id: &str) -> Option<Rung> {
    let act = self.acts.get(id)?;
    let mut word = act.text(0).to_owned();
    let made = self.made_by(id);
    let thinks = made
      .iter()
      .any(|one| kind_of(one) == "reply" && self.acts.get(one).is_some_and(|reply| !reply.over()));
    if word.is_empty()
      && let Some(ready) = self.words.get(id)
    {
      word.clone_from(ready);
    }
    let findings = self.paragraph(id, "refused").map(|text| unbound(&text));
    let raised = self.paragraph(id, "raised").map(|text| unbound(&text));
    let runs = self.runs(id);
    let awaits = runs
      .iter()
      .flat_map(|run| self.made_by(run))
      .filter_map(|one| self.acts.get(&one))
      .find(|wants| wants.kind == "wants" && !wants.over())
      .map(|wants| wants.text(0).to_owned());
    let acts: Vec<String> =
      made.into_iter().filter(|one| !matches!(kind_of(one), "reply")).collect();
    let state = if findings.is_some() || raised.is_some() {
      Failed
    } else if !act.over() {
      if self.holds(id) {
        Paused
      } else if awaits.as_deref().is_some_and(|one| self.owed(one)) {
        Asks
      } else if awaits.is_some() {
        Waits
      } else {
        Runs
      }
    } else if act.failed() {
      Failed
    } else {
      Done
    };
    Some(Rung {
      id: id.to_owned(),
      word,
      state,
      findings,
      raised,
      acts,
      awaits,
      thinks,
      made: act.made,
    })
  }

  /// The runs of the word of a rung: those whose rung it is, and its told rung.
  fn runs(&self, id: &str) -> Vec<String> {
    let told = format!("{id}_told");
    let runs = self
      .order
      .iter()
      .filter_map(|one| self.acts.get(one))
      .filter(|run| run.kind == "run" && (run.text(0) == id || run.text(0) == told));
    runs.map(|run| run.id.clone()).collect()
  }

  /// What a chain is doing: the most urgent state of its open prompts and their rungs.
  pub fn state(&self, chain: &str) -> State {
    if self.holds(chain) {
      return Paused;
    }
    let Some(held) = self.chain.get(chain) else { return Rest };
    let open: Vec<&String> =
      held.prompts.iter().filter(|one| self.acts.get(*one).is_some_and(|p| !p.over())).collect();
    if open.is_empty() {
      let failed = held.prompts.last().and_then(|one| self.acts.get(one)).is_some_and(Act::failed);
      return if failed {
        Failed
      } else if held.prompts.is_empty() {
        Rest
      } else {
        Done
      };
    }
    let states = open.iter().filter_map(|one| {
      self.ladder(one).into_iter().rev().find(|rung| rung.state != Failed).map(|rung| rung.state)
    });
    states.min().unwrap_or(Runs)
  }

  /// What a prompt is doing.
  pub fn prompt_state(&self, prompt: &str) -> State {
    let Some(act) = self.acts.get(prompt) else { return Rest };
    if self.owed(prompt) {
      return Asks;
    }
    if act.over() {
      return if act.failed() { Failed } else { Done };
    }
    if self.holds(prompt) {
      return Paused;
    }
    self
      .ladder(prompt)
      .into_iter()
      .rev()
      .find(|rung| rung.state != Failed)
      .map_or(Runs, |rung| rung.state)
  }

  /// The tree of threads: each prompt, under the prompt whose ladder holds the rung that made it, and the
  /// acknowledgments of each as its messages, in the order of the life.
  pub fn threads(&self) -> Vec<Thread> {
    let prompts: Vec<&Act> = self
      .order
      .iter()
      .filter_map(|one| self.acts.get(one))
      .filter(|act| act.kind == "prompt")
      .collect();
    let mut messages: HashMap<String, Vec<String>> = HashMap::new();
    let mut tops = Vec::new();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for prompt in &prompts {
      if let Some(of) = self.acknowledges(prompt) {
        messages.entry(of).or_default().push(prompt.id.clone());
        continue;
      }
      match self.thread_of(&prompt.by) {
        Some(parent) => children.entry(parent).or_default().push(prompt.id.clone()),
        None => tops.push(prompt.id.clone()),
      }
    }
    let mut threads = Vec::new();
    let mut stack: Vec<(String, usize)> = tops.into_iter().rev().map(|one| (one, 0)).collect();
    while let Some((prompt, depth)) = stack.pop() {
      let state = self.prompt_state(&prompt);
      threads.push(Thread {
        prompt: prompt.clone(),
        depth,
        state,
        messages: messages.remove(&prompt).unwrap_or_default(),
      });
      for child in children.remove(&prompt).unwrap_or_default().into_iter().rev() {
        stack.push((child, depth + 1));
      }
    }
    threads
  }

  /// The thread that a maker serves: the prompt whose ladder holds a rung, and nothing for the operator.
  fn thread_of(&self, by: &str) -> Option<String> {
    let rung = self.acts.get(by).filter(|act| act.kind == "rung")?;
    self.acts.get(&rung.by).filter(|act| act.kind == "prompt").map(|act| act.id.clone())
  }

  /// The thread that an acknowledgment tells of an act: the thread of the rung that made the act it names.
  fn acknowledges(&self, prompt: &Act) -> Option<String> {
    if prompt.text(0) != "None" || kind_of(&prompt.by) != "chain" {
      return None;
    }
    let named = prompt.text(1).split(' ').next()?;
    let act = self.acts.get(named)?;
    self.thread_of(&act.by)
  }

  /// The messages between programs: each prompt from a rung of one chain to another chain or to the operator, and
  /// each prompt of the operator.
  pub fn edges(&self) -> Vec<Edge> {
    let prompts =
      self.order.iter().filter_map(|one| self.acts.get(one)).filter(|act| act.kind == "prompt");
    let mut edges = Vec::new();
    for prompt in prompts {
      let rung = self.acts.get(&prompt.by).filter(|act| act.kind == "rung");
      let from = rung.map_or(OPERATOR.to_owned(), |one| one.on.clone());
      let to = if prompt.taker == "console" { OPERATOR.to_owned() } else { prompt.on.clone() };
      if from == to || kind_of(&prompt.by) == "chain" {
        continue;
      }
      let state = self.prompt_state(&prompt.id);
      let (made, ended) = (prompt.made, prompt.ended);
      edges.push(Edge {
        prompt: prompt.id.clone(),
        from,
        rung: rung.map(|one| one.id.clone()),
        to,
        state,
        made,
        ended,
      });
    }
    edges
  }

  /// The call stack of a chain, from the innermost wait out: the last rung of the chain that has not ended, its
  /// prompt, then the rung that made that prompt and its prompt, and so up to the operator.
  pub fn stack(&self, chain: &str) -> Vec<Frame> {
    let mut frames = Vec::new();
    let open = self
      .order
      .iter()
      .rev()
      .filter_map(|one| self.acts.get(one))
      .find(|act| act.kind == "rung" && act.on == chain && !act.over());
    let mut at = open.map(|act| act.id.clone());
    while let Some(id) = at.take() {
      if frames.len() > 64 || frames.iter().any(|one: &Frame| one.act == id) {
        break;
      }
      let Some(act) = self.acts.get(&id) else { break };
      let awaits =
        if act.kind == "rung" { self.rung(&id).and_then(|rung| rung.awaits) } else { None };
      frames.push(Frame { act: id.clone(), chain: act.on.clone(), awaits });
      if matches!(act.kind.as_str(), "rung" | "prompt") && self.acts.contains_key(&act.by) {
        at = Some(act.by.clone());
      }
    }
    frames
  }

  /// The bindings that a chain told, each name once with its last value, in the order they were last told.
  pub fn bindings(&self, chain: &str) -> Vec<Binding> {
    let mut found: Vec<Binding> = Vec::new();
    for (at, _, text) in self.told.get(chain).into_iter().flatten() {
      for (name, value) in bound(text) {
        found.retain(|one| one.name != name);
        found.push(Binding { name, value, at: *at });
      }
    }
    found
  }
}

use State::{Asks, Done, Failed, Paused, Rest, Runs, Waits};

/// The bindings of a paragraph: each statement `name = value`, each quote `<s:name>`, and each binding of an act
/// `name: Act[T] = Act('name')`, which binds the act.
pub fn bound(text: &str) -> Vec<(String, String)> {
  let mut found = Vec::new();
  let mut lines = text.lines();
  while let Some(line) = lines.next() {
    if line.starts_with('#') {
      continue;
    }
    if let Some(name) = line.strip_prefix("<s:").and_then(|rest| rest.strip_suffix('>')) {
      let close = format!("</s:{name}>");
      let mut value = Vec::new();
      for inner in lines.by_ref() {
        match inner.strip_suffix(close.as_str()) {
          Some(last) => {
            value.push(last);
            break;
          }
          None => value.push(inner),
        }
      }
      found.push((name.to_owned(), value.join("\n")));
    } else if let Some((name, of)) = line.split_once(": Act[") {
      found.push((name.to_owned(), format!("Act[{}", of.split(" = ").next().unwrap_or_default())));
    } else if let Some((name, value)) = line.split_once(" = ")
      && name.chars().all(|one| one.is_alphanumeric() || one == '_')
    {
      found.push((name.to_owned(), value.to_owned()));
    }
  }
  found
}

/// The value of the one binding of a paragraph, as a refused or a raised paragraph holds it.
fn unbound(text: &str) -> String {
  bound(text)
    .into_iter()
    .next()
    .map_or_else(|| text.to_owned(), |(_, value)| value.trim_matches('"').to_owned())
}

#[cfg(test)]
#[path = "model.test.rs"]
mod test;
