//! The acts of a life, as the facts that it says make them, and as the model of a life in TypeScript reads them: each
//! act with its place in time, whether it is done and what it came to, whether a pause holds it, and for a rung how its
//! word ran. The app sends these acts to its window, and the window derives every view from them with the module
//! `@furb/engine/life`, as the TUI does. The facts are those that `furb::world::retold` reads from a record, which
//! makes what a record keeps no fact of, so the acts are whole as far as the record shows them.
//!
//! A fact is a list of JSON as the wire gives it: its kind, the act it is about, who said it, and its words. A question
//! is a fact whose act is named by its kind, and its first word is the chain it is on.

use std::collections::HashMap;

use serde_json::{Value, json};

/// The window of a model whose stand names none.
const WINDOW: f64 = 200_000.0;

/// The questions that the engine asks on the way to what an act does, which are no acts that a person follows.
const STEPS: [&str; 8] = ["stand", "clock", "chance", "gate", "merged", "run", "wants", "reply"];

/// How many characters a text of a fact keeps: the window shows no more of a value, and a life of many facts fits in
/// memory.
pub const LONG: usize = 16_384;

/// How many characters the text of a file keeps: the window shows only its path and the count of its lines.
pub const TEXT: usize = 2048;

/// How many characters of a value the watch shows.
const SHOWN: usize = 400;

/// One act of the life, as its question said it and as its facts settled it.
#[derive(Debug, Clone, PartialEq)]
pub struct Act {
  pub id: String,
  pub kind: String,
  pub by: String,
  /// The chain it stands on, and the chain itself for a chain.
  pub on: String,
  /// The words of the question after the chain.
  pub words: Vec<Value>,
  /// How many facts the life had said when it was made, and when it was done.
  pub began: usize,
  pub ended: Option<usize>,
  /// Who took it, when an ear said its started.
  pub taker: String,
  pub value: Value,
}

/// One binding that a chain told: a name and its value as python shows it, where it was told, and the act whose name
/// the name starts with.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
  pub name: String,
  pub value: String,
  pub at: usize,
  pub act: Option<String>,
}

/// One answer of a model: the chain of its reply, the actor, where it was done, and its usage.
#[derive(Debug, Clone, PartialEq)]
struct Answer {
  at: usize,
  actor: String,
  tokens: f64,
  dollars: f64,
}

/// The acts of a life, after each fact that it said.
#[derive(Debug, Default, Clone)]
pub struct Life {
  /// How many facts the life said.
  pub len: usize,
  acts: HashMap<String, Act>,
  /// The acts, in the order the life made them.
  order: Vec<String>,
  /// The chain of each question: itself for a chain.
  scopes: HashMap<String, String>,
  /// The rung of each run, and what the run of each rung came to.
  runs: HashMap<String, String>,
  ran: HashMap<String, Value>,
  /// The last paragraph told under each header, as #rung5 refused, without its header.
  heads: HashMap<String, String>,
  /// The paragraphs told on each chain, each with its place.
  told: HashMap<String, Vec<(usize, String)>>,
  /// The last pause or wake of each name, oldest first: true for a pause.
  controls: Vec<(String, bool)>,
  /// What each command printed, by its stream.
  printed: HashMap<String, (String, String)>,
  /// The word that each rung said ready.
  pub words: HashMap<String, String>,
  /// The chain and the actor of each reply that is not done, and the answers on each chain.
  replies: HashMap<String, (String, String)>,
  answers: HashMap<String, Vec<Answer>>,
  /// The windows of the roster, by the name of each model.
  windows: HashMap<String, f64>,
  /// How many changes the acts had, and the count at the last change of each act.
  pub count: u64,
  changed: HashMap<String, u64>,
}

/// Whether a fact is a question: its act is named by its kind and a number.
pub fn question(kind: &str, about: &str) -> bool {
  about.strip_prefix(kind).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The kind of an act, from its name.
pub fn kind_of(id: &str) -> &str {
  id.trim_end_matches(|one: char| one.is_ascii_digit())
}

/// The name of the exception that a value is, if it is one: a map whose `is` names a type with `args`.
pub fn fault(value: &Value) -> Option<&str> {
  let name = value.get("is")?.as_str()?;
  (value.get("args").is_some() && !matches!(name, "int" | "float" | "tuple" | "dict"))
    .then_some(name)
}

/// The items of a value that is a list or a tuple, which the wire marks as a map whose `is` is tuple.
fn items(value: &Value) -> Option<&Vec<Value>> {
  match value.get("is").and_then(Value::as_str) {
    Some("tuple") => value.get("args")?.get(0)?.as_array(),
    _ => value.as_array(),
  }
}

/// A value as a line shows it: a string as its text, and any other value as JSON.
fn display(value: &Value) -> String {
  value.as_str().map_or_else(|| value.to_string(), str::to_owned)
}

/// A text cut to a number of characters, with an ellipsis when it was cut.
fn cut(text: &str, most: usize) -> String {
  match text.char_indices().nth(most) {
    Some((at, _)) => format!("{}…", &text[..at]),
    None => text.to_owned(),
  }
}

/// A fact as the app keeps it: the text of each file cut to [`TEXT`] characters with the count of its lines that are
/// not empty, and each other long text cut to [`LONG`] characters. The words of a rung, of a prompt and of a chain, and
/// the word that a rung said ready, which are the code and the messages that the window shows whole, stay whole.
pub fn lean(fact: Value) -> Value {
  let word = |at: usize| fact.get(at).and_then(Value::as_str).unwrap_or_default();
  let whole = word(0) == "ready"
    || (question(word(0), word(1)) && matches!(word(0), "rung" | "prompt" | "chain"));
  if whole {
    return fact;
  }
  fn shorter(value: Value) -> Value {
    match value {
      Value::String(text) if text.len() > LONG => Value::String(cut(&text, LONG)),
      Value::Array(items) => Value::Array(items.into_iter().map(shorter).collect()),
      Value::Object(mut map) => {
        let long = map.get("content").and_then(Value::as_str).filter(|one| one.len() > TEXT);
        if let Some(content) = long {
          let lines = content.lines().filter(|line| !line.is_empty()).count();
          let short = cut(content, TEXT);
          map.insert("lines".to_owned(), Value::from(lines));
          map.insert("content".to_owned(), Value::String(short));
        }
        Value::Object(map.into_iter().map(|(key, one)| (key, shorter(one))).collect())
      }
      other => other,
    }
  }
  shorter(fact)
}

impl Life {
  /// The life after the facts of a log.
  #[cfg(test)]
  pub fn of(facts: &[Value]) -> Life {
    let mut life = Life::default();
    for fact in facts {
      life.add(fact);
    }
    life
  }

  /// The life after one more fact, at the end of its log.
  pub fn add(&mut self, fact: &Value) {
    self.len += 1;
    let at = self.len;
    let Some(items) = fact.as_array() else { return };
    let text = |at: usize| items.get(at).and_then(Value::as_str).unwrap_or_default();
    let (kind, about, by) = (text(0), text(1), text(2));
    if question(kind, about) {
      self.asked(at, kind, about, by, items.get(3..).unwrap_or_default());
      return;
    }
    let value = items.get(3).cloned().unwrap_or(Value::Null);
    match kind {
      "started" => {
        if let Some(act) = self.acts.get_mut(about) {
          if act.taker.is_empty() {
            act.taker = by.to_owned();
          }
          if act.kind == "bash" && act.ended.is_none() {
            self.printed.insert(about.to_owned(), Default::default());
          }
          self.mark(about);
        }
      }
      "done" => self.done(at, about, value),
      "tell" => {
        let notes: Vec<&str> =
          value.as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        for note in &notes {
          let (head, body) = note.split_once('\n').unwrap_or((note, ""));
          self.heads.insert(head.to_owned(), body.to_owned());
        }
        if self.heads.contains_key(&format!("#{about} refused")) {
          self.mark(about);
        }
        let chain = self.scopes.get(about).cloned().unwrap_or_default();
        self.told.entry(chain).or_default().push((at, notes.join("\n")));
      }
      "ready" => {
        self.words.insert(about.to_owned(), text(3).to_owned());
        self.mark(about);
      }
      "out" => {
        let printed = self.printed.entry(about.to_owned()).or_default();
        let stream = if text(4) == "stderr" { &mut printed.1 } else { &mut printed.0 };
        stream.push_str(text(3));
        self.mark(about);
      }
      "pause" | "wake" => {
        self.controls.retain(|(one, _)| one != about);
        self.controls.push((about.to_owned(), kind == "pause"));
        let open: Vec<String> = self
          .order
          .iter()
          .filter(|one| self.acts[*one].ended.is_none() && self.covers(about, one))
          .cloned()
          .collect();
        for one in open {
          self.mark(&one);
        }
      }
      _ => {}
    }
  }

  /// A change of an act, counted.
  fn mark(&mut self, id: &str) {
    if self.acts.contains_key(id) {
      self.count += 1;
      self.changed.insert(id.to_owned(), self.count);
    }
  }

  /// A question: a new act, or a step of the engine that the life keeps aside.
  fn asked(&mut self, at: usize, kind: &str, about: &str, by: &str, words: &[Value]) {
    let on = words.first().and_then(Value::as_str).unwrap_or_default().to_owned();
    self
      .scopes
      .entry(about.to_owned())
      .or_insert_with(|| if kind == "chain" { about.to_owned() } else { on.clone() });
    let text = |at: usize| words.get(at).and_then(Value::as_str).unwrap_or_default().to_owned();
    match kind {
      "run" => {
        let rung = text(1);
        if let Some(word) = words.get(2).and_then(Value::as_str) {
          self.words.entry(rung.clone()).or_insert_with(|| word.to_owned());
        }
        self.runs.insert(about.to_owned(), rung);
      }
      "reply" => {
        self.replies.insert(about.to_owned(), (on.clone(), text(1)));
      }
      _ => {}
    }
    if STEPS.contains(&kind) || self.acts.contains_key(about) {
      return;
    }
    let act = Act {
      id: about.to_owned(),
      kind: kind.to_owned(),
      by: by.to_owned(),
      on: if kind == "chain" { about.to_owned() } else { on },
      words: words.get(1..).unwrap_or_default().to_vec(),
      began: at,
      ended: None,
      taker: String::new(),
      value: Value::Null,
    };
    self.order.push(about.to_owned());
    self.acts.insert(about.to_owned(), act);
    self.mark(about);
  }

  /// A done: the first gives its act its end and its value, and a done of a reply, of a stand or of a run tells the
  /// answers, the windows and the run of a rung.
  fn done(&mut self, at: usize, about: &str, value: Value) {
    if let Some((chain, actor)) = self.replies.remove(about) {
      let turn = items(&value);
      let usage = turn.and_then(|one| one.get(2));
      let number =
        |at: usize| usage.and_then(|one| one.get(at)).and_then(Value::as_f64).unwrap_or_default();
      self.answers.entry(chain).or_default().push(Answer {
        at,
        actor,
        tokens: number(0),
        dollars: number(4),
      });
    }
    if kind_of(about) == "stand" {
      for one in value.get(0).and_then(Value::as_array).into_iter().flatten() {
        let name = one.get(0).and_then(Value::as_str).unwrap_or_default();
        self.windows.insert(name.to_owned(), one.get(2).and_then(Value::as_f64).unwrap_or(WINDOW));
      }
    }
    if let Some(rung) = self.runs.get(about).cloned() {
      self.ran.entry(rung.clone()).or_insert_with(|| value.clone());
      self.mark(&rung);
    }
    let Some(act) = self.acts.get_mut(about) else { return };
    if act.ended.is_some() {
      return;
    }
    act.ended = Some(at);
    act.value = value;
    self.mark(about);
    let made: Vec<String> = self
      .order
      .iter()
      .filter(|one| self.acts[*one].by == about && self.acts[*one].kind == "rung")
      .cloned()
      .collect();
    for one in made {
      self.mark(&one);
    }
  }

  /// Whether a control of a name covers an act: the act itself, an act on the chain it names, or an act that it made.
  fn covers(&self, name: &str, id: &str) -> bool {
    if self.scopes.get(id).is_some_and(|chain| chain == name && kind_of(name) == "chain") {
      return true;
    }
    let mut at = id;
    for _ in 0..64 {
      if at == name {
        return true;
      }
      match self.acts.get(at) {
        Some(act) => at = &act.by,
        None => return false,
      }
    }
    false
  }

  /// Whether a pause holds an act: the last control that covers it is a pause.
  pub fn paused(&self, id: &str) -> bool {
    self
      .controls
      .iter()
      .rev()
      .find(|(name, _)| self.covers(name, id))
      .is_some_and(|(_, pause)| *pause)
  }

  /// How the word of a rung ran: it runs, it is done, or it failed with a reason. A word that the gate refused fails
  /// with the findings of the gate, and a word that raised fails with its exception; a cancel of a rung whose prompt
  /// ended well is no failure.
  pub fn run(&self, act: &Act) -> (&'static str, String) {
    if let Some(findings) = self.heads.get(&format!("#{} refused", act.id)) {
      return (
        "failed",
        bound(findings).into_iter().next().map(|(_, one)| literal(&one)).unwrap_or_default(),
      );
    }
    let ran = self.ran.get(&act.id);
    let value = ran.unwrap_or(&act.value);
    match fault(value) {
      Some(name) => {
        let parent = self.acts.get(&act.by);
        let ended_well = |one: &Act| one.ended.is_some() && fault(&one.value).is_none();
        let spared = name == "CancelledError"
          && (act.words.get(1).and_then(Value::as_str).is_some_and(|one| !one.is_empty())
            || ended_well(act)
            || parent.is_some_and(|one| one.kind == "prompt" && ended_well(one)));
        if spared {
          return ("done", String::new());
        }
        let args = value.get("args").and_then(Value::as_array).into_iter().flatten().map(display);
        ("failed", format!("{name}: {}", args.collect::<Vec<_>>().join(", ")))
      }
      None if ran.is_some() || act.ended.is_some() => ("done", String::new()),
      None => ("running", String::new()),
    }
  }

  /// One act as the model of a life in TypeScript reads it, a LiveAct: a command that runs holds what it printed.
  pub fn live(&self, act: &Act) -> Value {
    let value = match (&act.ended, self.printed.get(&act.id)) {
      (None, Some((out, err))) => json!({
        "is": "Exit", "code": null,
        "stdout": {"is": "Text", "path": format!("{}/stdout", act.id), "content": out},
        "stderr": {"is": "Text", "path": format!("{}/stderr", act.id), "content": err},
      }),
      _ => act.value.clone(),
    };
    let mut live = json!({
      "id": act.id, "kind": act.kind, "by": act.by, "on": act.on, "words": act.words,
      "done": act.ended.is_some(), "value": value, "paused": act.ended.is_none() && self.paused(&act.id),
      "began": act.began, "ended": act.ended,
    });
    if act.kind == "rung" {
      let (status, reason) = self.run(act);
      live["run"] = json!({"status": status, "reason": reason});
    }
    live
  }

  /// The acts that changed after a count of changes, in the order the life made them.
  pub fn since(&self, count: u64) -> Vec<Value> {
    let changed = |one: &&String| self.changed.get(*one).is_some_and(|at| *at > count);
    self.order.iter().filter(changed).map(|one| self.live(&self.acts[one])).collect()
  }

  /// One act by its name.
  #[cfg(test)]
  pub fn act(&self, id: &str) -> Option<&Act> {
    self.acts.get(id)
  }

  /// Whether the life works now: a model writes, or a command runs.
  pub fn busy(&self) -> bool {
    let runs = |one: &String| {
      self.acts.get(one).is_some_and(|act| act.kind == "bash" && act.ended.is_none())
    };
    !self.replies.is_empty() || self.order.iter().any(runs)
  }

  /// The chains of the life, in the order the life made them.
  pub fn chains(&self) -> Vec<String> {
    self.order.iter().filter(|one| kind_of(one) == "chain").cloned().collect()
  }

  /// The prompts that wait for the operator: the console took them, and they are not done.
  pub fn owed(&self) -> Vec<String> {
    let owed = |act: &&Act| act.kind == "prompt" && act.taker == "console" && act.ended.is_none();
    self
      .order
      .iter()
      .filter_map(|one| self.acts.get(one))
      .filter(owed)
      .map(|one| one.id.clone())
      .collect()
  }

  /// The measure of each chain whose models answered or that a grant holds, as the TUI measures it: the dollars of its
  /// answers, the dollars since the grant that stands on it and its ceiling, the share of the window that its last
  /// answer read, and how many answers it had.
  pub fn measures(&self) -> Value {
    let grant = |chain: &str| {
      self
        .order
        .iter()
        .rev()
        .filter_map(|one| self.acts.get(one))
        .find(|act| act.kind == "grant" && act.on == chain && act.ended.is_none())
    };
    let mut out = serde_json::Map::new();
    for (chain, answers) in &self.answers {
      let held = grant(chain);
      let last = answers.last();
      let window = |actor: &str| {
        self.windows.get(actor.split('/').next().unwrap_or_default()).copied().unwrap_or(WINDOW)
      };
      out.insert(chain.clone(), json!({
        "dollars": answers.iter().map(|one| one.dollars).sum::<f64>(),
        "spent": answers.iter().filter(|one| held.is_some_and(|grant| one.at > grant.began)).map(|one| one.dollars).sum::<f64>(),
        "ceiling": held.and_then(|grant| grant.words.first().and_then(Value::as_f64)),
        "share": last.map(|one| (one.tokens / window(&one.actor) * 10_000.0).round() / 10_000.0),
        "answers": answers.len(),
        "actor": last.map(|one| one.actor.clone()),
      }));
    }
    for act in self.order.iter().filter_map(|one| self.acts.get(one)) {
      if act.kind == "grant" && act.ended.is_none() && !out.contains_key(&act.on) {
        let ceiling = act.words.first().and_then(Value::as_f64);
        out.insert(
          act.on.clone(),
          json!({"dollars": 0, "spent": 0, "ceiling": ceiling, "answers": 0}),
        );
      }
    }
    Value::Object(out)
  }

  /// Every binding that a chain told, in the order it told them, each value cut to a few hundred characters.
  pub fn bindings(&self, chain: &str) -> Vec<Binding> {
    let mut found = Vec::new();
    for (at, text) in self.told.get(chain).into_iter().flatten() {
      for (name, value) in bound(text) {
        let act =
          name.split('_').next().filter(|one| self.acts.contains_key(*one)).map(str::to_owned);
        found.push(Binding { name, value: cut(&value, SHOWN), at: *at, act });
      }
    }
    found
  }
}

/// A string as python writes it, read back: the text between its quotes, with each escape as the character it stands
/// for. A text with no quotes around it stays as it is.
pub fn literal(python: &str) -> String {
  let quoted = python.len() >= 2
    && (python.starts_with('\'') || python.starts_with('"'))
    && python.ends_with(&python[..1]);
  if !quoted {
    return python.to_owned();
  }
  let mut out = String::new();
  let mut chars = python[1..python.len() - 1].chars();
  while let Some(one) = chars.next() {
    if one != '\\' {
      out.push(one);
      continue;
    }
    match chars.next() {
      Some('n') => out.push('\n'),
      Some('t') => out.push('\t'),
      Some('r') => out.push('\r'),
      Some(other) => out.push(other),
      None => out.push('\\'),
    }
  }
  out
}

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

#[cfg(test)]
#[path = "model.test.rs"]
mod test;
