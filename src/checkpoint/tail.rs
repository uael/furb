//! The tail of another engine: the facts that a build of another engine or of another version said
//! after the last checkpoint of a record. This build does not replay them, since its engine can
//! make other acts from the same facts. It carries them as data, in a checkpoint that it writes at
//! the tip of the record from the last checkpoint and the facts of the tail as the reader tells
//! them with no life, as the life of the tail would have written it at its tip: each chain keeps
//! its turns, each open prompt stays open, and each act that a value names keeps its value.
//!
//! The module of each chain loses what the words of the tail changed, since no word runs again:
//! each name that a word of a model or of the operator binds, deletes or changes in place through a
//! method, an item or an attribute, which the chain is told as a name that it lost. A word that
//! changes a name through a function that it calls changes it out of sight, so that name keeps its
//! value of the checkpoint. A word that a tell binds holds only values, which the chain keeps.
//!
//! The reader gives every fact of the tail while the log beside the record agrees with it. Else it
//! gives only what the record kept, which holds no tell and no act that a word made, so the chain
//! keeps the replies of its models alone, and a chain or a prompt that a word of the tail made is
//! lost.

use std::{
  collections::{HashMap, HashSet},
  path::Path,
};

use ruff_python_ast::{
  Comprehension, ExceptHandler, Expr, ExprContext, Pattern, Stmt,
  visitor::{self, Visitor},
};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

use crate::{
  engine::ROOT,
  fact::{Fact, named},
  value::{Fault, Object, ObjectRef, field},
  world::{self, CHECKPOINT_SCHEMA, VERSION},
};

use super::{CHECKPOINT, fact, last};

/// The kinds of act that the checkpoint carries when a value names one.
const VERBS: [&str; 7] = ["prompt", "bash", "read", "write", "wait", "cd", "chance"];

/// The checkpoint that this build writes at the tip of the record at a path when a build of another
/// engine or of another version made the facts after its last checkpoint. Nothing when this build
/// made them, or when the record holds no checkpoint of the major of the schema that this build
/// writes.
pub(crate) fn crossed(path: &Path) -> Result<Option<Object>, Fault> {
  folded(path, false)
}

/// The checkpoint that carries the facts after the last checkpoint of the record at a path as data,
/// whatever build made them, as [`crossed`] carries the tail of another build. A host keeps it at
/// the tip of a record whose life a fault stopped, since a replay of that tail runs again the word
/// that the fault cut. Nothing when the record holds no checkpoint of the schema that this build
/// writes.
pub fn carry(path: &Path) -> Result<Option<Object>, Fault> {
  folded(path, true)
}

/// The checkpoint that carries the tail of the record at a path, when a build of another engine or
/// of another version made it or when the tail of any build is asked.
fn folded(path: &Path, any: bool) -> Result<Option<Object>, Fault> {
  if !path.exists() {
    return Ok(None);
  }
  let heads = world::heads(path, &[world::MADE, CHECKPOINT])?;
  let Some(at) = last(&heads) else { return Ok(None) };
  let tail = &heads[at + 1..];
  let theirs = world::made(tail).is_some_and(|one| one != world::engine())
    || world::version(tail).is_some_and(|one| one != VERSION);
  let data = fact(&heads[at]).and_then(|a| a.word(0).map(world::plain));
  let ours = |one: &Object| {
    let data = one.as_ref();
    let held = field(&data, "schema");
    held.is_some() && CHECKPOINT_SCHEMA.opens("checkpoint", held).is_ok()
  };
  let Some(data) = data.filter(|one| (any || theirs) && ours(one)) else {
    return Ok(None);
  };
  let (mut entries, mut kept) = (0, 0);
  world::each(path, &mut |one| {
    entries += 1;
    if fact(&one).is_some_and(|a| a.kind() == CHECKPOINT) {
      kept = entries;
    }
  })?;
  let mut fold = Fold::new(data.as_ref());
  world::retold(path)?.after(kept, &mut |one| fold.hear(one))?;
  Ok(Some(fold.checkpoint()))
}

/// The facts of a tail, heard in order on the checkpoint that it goes on from.
struct Fold {
  /// The highest number that each kind took.
  taken: HashMap<String, i64>,
  /// The chains and the acts of the checkpoint, as its data holds them.
  chains: Vec<Object>,
  acts: Vec<Object>,
  /// The question of each act of the tail.
  asked: HashMap<String, Fact>,
  /// The chain of each act that a fact names, and of each chain itself.
  scopes: HashMap<String, String>,
  /// What the transcript of each chain holds, as the engine of the tail held it.
  held: HashMap<String, Vec<Fact>>,
  /// The rung that seeded each chain of the checkpoint in the life of the tail, which said its
  /// prefix.
  seeds: HashMap<String, String>,
  /// Each run of the tail, in order.
  runs: Vec<Fact>,
  /// The value of each act that is done.
  done: HashMap<String, Object>,
  /// Whether the last pause or wake of the tail on a chain paused it.
  paused: HashMap<String, bool>,
  /// The ear of the outside that started each act.
  started: HashMap<String, String>,
}

/// What a checkpoint holds at the tip of the tail beside its chains: each chain that lives, in the
/// order of its number, each prompt that is open, each act that it cut, and the name of each act
/// that it carries.
struct Tip {
  live: Vec<String>,
  opened: Vec<String>,
  cut: Vec<String>,
  ids: HashSet<String>,
}

/// What the module of a chain holds after the tail: its values, as python shows them, the names
/// that it lost, in order, and each name that a word of the tail changed.
#[derive(Default)]
struct Module {
  values: Vec<(String, String)>,
  lost: Vec<String>,
  changed: HashSet<String>,
}

impl Module {
  fn bound(&mut self, name: &str, text: &str) {
    self.values.retain(|one| one.0 != name);
    self.values.push((name.to_owned(), text.to_owned()));
    self.lost.retain(|one| one != name);
  }

  fn changed(&mut self, name: &str) {
    self.values.retain(|one| one.0 != name);
    self.changed.insert(name.to_owned());
    if !self.lost.iter().any(|one| one == name) {
      self.lost.push(name.to_owned());
    }
  }
}

impl Fold {
  fn new(data: ObjectRef<'_>) -> Fold {
    let listed = |name: &str| -> Vec<Object> {
      let items = field(&data, name).and_then(|one| one.items()).unwrap_or_default();
      items.iter().map(ObjectRef::to_owned).collect()
    };
    let (chains, acts) = (listed("chains"), listed("acts"));
    let pairs = field(&data, "taken").and_then(|one| one.pairs()).unwrap_or_default();
    let taken = pairs.iter().filter_map(|(k, v)| Some((k.as_str()?.to_owned(), v.as_int()?)));
    let mut fold = Fold {
      taken: taken.collect(),
      chains,
      acts,
      asked: HashMap::new(),
      scopes: HashMap::new(),
      held: HashMap::new(),
      seeds: HashMap::new(),
      runs: Vec::new(),
      done: HashMap::new(),
      paused: HashMap::new(),
      started: HashMap::new(),
    };
    for c in &fold.chains {
      let id = text(c, "id");
      let turns = field(&c.as_ref(), "turns").and_then(|one| one.items()).unwrap_or_default();
      fold.held.insert(id.clone(), turns.into_iter().filter_map(Fact::of).collect());
      fold.scopes.insert(id.clone(), id);
    }
    for a in &fold.acts {
      let on = text(a, "on");
      let chain = fold.scopes.get(&on).cloned().unwrap_or(on);
      fold.scopes.insert(text(a, "id"), chain);
    }
    fold
  }

  /// The next fact of the tail.
  fn hear(&mut self, one: Object) {
    let Some(a) = Fact::owned(one) else { return };
    let (kind, about) = (a.kind().to_owned(), a.about().to_owned());
    let chain = match a.question() {
      true => {
        let n = about.strip_prefix(kind.as_str()).and_then(|one| one.parse::<i64>().ok());
        let most = self.taken.entry(kind.clone()).or_default();
        *most = (*most).max(n.unwrap_or_default());
        let chain = self.scopes.get(a.on()).cloned().unwrap_or_default();
        let own = if kind == "chain" { about.clone() } else { chain.clone() };
        self.scopes.insert(about.clone(), own);
        if kind == "run" {
          self.runs.push(a.clone());
        }
        self.asked.insert(about.clone(), a.clone());
        chain
      }
      false => self.scopes.get(&about).cloned().unwrap_or_default(),
    };
    match kind.as_str() {
      "done" => {
        self.done.insert(about, a.word(0).map_or_else(Object::none, |w| w.to_owned()));
      }
      "started" if !self.scopes.contains_key(a.by()) => {
        self.started.entry(about).or_insert_with(|| a.by().to_owned());
      }
      "pause" | "wake" if chain == about => {
        self.paused.insert(about, kind == "pause");
      }
      _ => {}
    }
    if chain.is_empty() {
      return;
    }
    if kind != "prefix" {
      self.held.entry(chain).or_default().push(a);
      return;
    }
    let ours = self.chains.iter().any(|c| text(c, "id") == chain);
    if ours && named(a.by(), "rung") && !self.seeds.contains_key(&chain) {
      self.seeds.insert(chain.clone(), a.by().to_owned());
    }
    let said = a.word(0).and_then(|one| one.items()).unwrap_or_default();
    self.held.insert(chain, said.into_iter().filter_map(Fact::of).collect());
  }
}

impl Fold {
  /// The checkpoint at the tip of the tail, as the record keeps it.
  fn checkpoint(self) -> Object {
    let modules = self.modules();
    let opened = self.opened();
    let live = self.live(&modules, &opened);
    let (acts, cut) = self.carried(&modules, &opened, &live);
    let ids = acts.iter().map(|one| text(one, "id")).collect();
    let tip = Tip { live, opened, cut, ids };
    let chains = tip.live.iter().map(|c| self.chain(c, &modules[c.as_str()], &tip));
    let data = [
      ("schema", CHECKPOINT_SCHEMA.object()),
      ("engine", Object::string(world::engine())),
      ("taken", Object::dict(self.taken.iter().map(|(k, v)| (s(k), Object::int(*v))))),
      ("chains", Object::list(chains.collect::<Vec<_>>())),
      ("acts", Object::list(acts)),
    ];
    let fact = [s(CHECKPOINT), s(ROOT), s("store"), Object::dict(data.map(|(k, v)| (s(k), v)))];
    Object::list([Object::list(fact)])
  }

  /// The chain of an act or of a chain.
  fn scope(&self, id: &str) -> &str {
    self.scopes.get(id).map_or("", String::as_str)
  }

  /// The module of each chain after the words of the tail ran on it, but the word that seeded it.
  fn modules(&self) -> HashMap<String, Module> {
    let mut modules: HashMap<String, Module> = HashMap::new();
    for c in &self.chains {
      let values = field(&c.as_ref(), "values").and_then(|one| one.pairs()).unwrap_or_default();
      let values = values.iter().filter_map(|(k, v)| Some((k.as_str()?, v.as_str()?)));
      let module = Module {
        values: values.map(|(k, v)| (k.to_owned(), v.to_owned())).collect(),
        lost: strings(c, "lost"),
        changed: HashSet::new(),
      };
      modules.insert(text(c, "id"), module);
    }
    for run in &self.runs {
      let chain = self.scope(run.about());
      let (rung, word) = (said(run, 1), said(run, 2));
      if chain.is_empty() || self.seeds.get(chain) == Some(&rung) {
        continue;
      }
      let module = modules.entry(chain.to_owned()).or_default();
      let Ok(parsed) = parse_module(&word) else { continue };
      let body = &parsed.syntax().body;
      if rung.ends_with("_told") {
        told(&word, body).for_each(|(name, value)| module.bound(name, value));
        continue;
      }
      let mut changes = Changes::default();
      changes.visit_body(body);
      for name in changes.bound {
        module.changed(name);
      }
      let held: Vec<&str> = changes
        .called
        .into_iter()
        .filter(|one| module.values.iter().any(|v| v.0 == *one))
        .collect();
      for name in held {
        module.changed(name);
      }
    }
    modules
  }

  /// Each prompt that is open at the tip of the tail.
  fn opened(&self) -> Vec<String> {
    let mut opened: Vec<String> = self
      .acts
      .iter()
      .filter(|a| field(&a.as_ref(), "value").is_none())
      .map(|a| text(a, "id"))
      .filter(|id| !self.done.contains_key(id))
      .collect();
    let asked =
      self.asked.values().filter(|q| q.kind() == "prompt" && !self.done.contains_key(q.about()));
    for q in asked {
      if !opened.iter().any(|one| one == q.about()) {
        opened.push(q.about().to_owned());
      }
    }
    opened
  }

  /// Each chain that lives at the tip of the tail, in the order of its number: each chain of the
  /// checkpoint, each chain of an open prompt, and each chain that a value of a chain that lives
  /// names, or that holds an act that such a value names.
  fn live(&self, modules: &HashMap<String, Module>, opened: &[String]) -> Vec<String> {
    let mut live: Vec<String> = self.chains.iter().map(|c| text(c, "id")).collect();
    live.extend(opened.iter().map(|one| self.scope(one).to_owned()));
    let mut at = 0;
    while at < live.len() {
      let values = modules.get(&live[at]).map(|one| one.values.as_slice()).unwrap_or_default();
      for name in values.iter().flat_map(|one| acts(&one.1)) {
        let chain = self.scope(&name).to_owned();
        if !chain.is_empty() && !live.contains(&chain) {
          live.push(chain);
        }
      }
      at += 1;
    }
    live.retain(|one| !one.is_empty());
    live.sort_by_key(|one| number(one));
    live.dedup();
    live
  }

  /// Each act that the checkpoint carries, in the order of the data of a checkpoint, and each that
  /// it cut: the last stand and the last clock of the root, each act of the checkpoint, each open
  /// prompt of a chain that lives, and each act that a value names, with its value.
  fn carried(
    &self,
    modules: &HashMap<String, Module>,
    opened: &[String],
    live: &[String],
  ) -> (Vec<Object>, Vec<String>) {
    let mut named: Vec<String> =
      opened.iter().filter(|one| live.iter().any(|c| c == self.scope(one))).cloned().collect();
    let values = live.iter().filter_map(|c| modules.get(c)).flat_map(|one| one.values.iter());
    named.extend(values.flat_map(|one| acts(&one.1)));
    let (mut acts, mut cut, mut seen) = (Vec::new(), Vec::new(), HashSet::new());
    for kind in ["stand", "clock"] {
      let last = self.asked.values().filter(|q| {
        q.kind() == kind && self.scope(q.about()) == ROOT && self.done.contains_key(q.about())
      });
      match last.max_by_key(|q| number(q.about())) {
        Some(q) => acts.push(settled(q.about(), q.on(), self.done[q.about()].clone(), kind)),
        None => acts.extend(self.acts.iter().filter(|a| text(a, "kind") == kind).cloned()),
      }
    }
    let mut rest: Vec<Object> = Vec::new();
    for a in self.acts.iter().filter(|a| !matches!(text(a, "kind").as_str(), "stand" | "clock")) {
      let id = text(a, "id");
      seen.insert(id.clone());
      rest.push(match self.done.get(&id).filter(|_| field(&a.as_ref(), "value").is_none()) {
        Some(value) => settled(&id, &text(a, "on"), value.clone(), &text(a, "kind")),
        None => a.clone(),
      });
    }
    for id in named {
      let Some(q) = self.asked.get(&id).filter(|q| VERBS.contains(&q.kind())) else { continue };
      if seen.insert(id.clone()) {
        rest.push(self.act(q, &mut cut));
      }
    }
    rest.sort_by_key(|a| (text(a, "kind"), number(&text(a, "id"))));
    acts.extend(rest);
    (acts, cut)
  }

  /// The data of an act of the tail that the checkpoint carries: with its value when it is done, as
  /// an open prompt, or else as an act that the checkpoint cut, which it names in cut.
  fn act(&self, q: &Fact, cut: &mut Vec<String>) -> Object {
    let (id, kind, on) = (q.about(), q.kind(), q.on());
    if let Some(value) = self.done.get(id) {
      return settled(id, on, value.clone(), kind);
    }
    if kind == "prompt" {
      let held = self.started.get(id).map_or("", String::as_str);
      let words = [("shape", 1), ("message", 2), ("to", 3)].map(|(k, at)| (k, s(&said(q, at))));
      let fields = [("kind", s(kind)), ("id", s(id)), ("on", s(on))].into_iter().chain(words);
      return Object::dict(fields.chain([("held", s(held))]).map(|(k, v)| (s(k), v)));
    }
    cut.push(id.to_owned());
    let why = match kind {
      "bash" => {
        let command = quoted(&said(q, 1));
        format!("{command} was interrupted: the life that ran it ended before it did")
      }
      _ => format!("{id} was cut by the checkpoint"),
    };
    settled(id, on, Fault::refused(why).object(), kind)
  }

  /// The data of a chain that lives at the tip of the tail: what the checkpoint holds of it, with
  /// the turns, the module and the acts of the tail.
  fn chain(&self, c: &str, module: &Module, tip: &Tip) -> Object {
    let asking = tip.opened.iter().any(|one| tip.ids.contains(one) && self.scope(one) == c);
    let cut = tip.cut.iter().filter(|one| self.scope(one) == c);
    let ours = self.chains.iter().find(|one| text(one, "id") == c);
    let made = self.asked.get(c);
    let pairs: Vec<(Object, Object)> = ours
      .and_then(|one| one.as_ref().pairs())
      .map(|pairs| pairs.iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect())
      .unwrap_or_default();
    let was =
      |name: &str| ours.and_then(|one| field(&one.as_ref(), name).and_then(|v| v.as_bool()));
    let on = ours
      .map_or_else(|| made.map_or_else(String::new, |q| q.on().to_owned()), |one| text(one, "on"));
    let definitions =
      ours.map_or_else(String::new, |one| stripped(&text(one, "definitions"), &module.changed));
    let paused = asking || self.paused.get(c).copied().or_else(|| was("paused")).unwrap_or(false);
    let made = |at: usize| made.map_or_else(String::new, |q| said(q, at));
    let fields = [
      ("id", s(c)),
      ("on", s(if tip.live.contains(&on) { &on } else { "" })),
      ("label", ours.map_or_else(|| s(&made(1)), |one| s(&text(one, "label")))),
      ("source", ours.map_or_else(|| s(&made(2)), |one| s(&text(one, "source")))),
      ("paused", Object::bool(paused)),
      ("turns", Object::list(self.turns(c, tip))),
      ("definitions", s(&definitions)),
      ("values", Object::dict(module.values.iter().map(|(k, v)| (s(k), s(v))))),
      ("lost", Object::list(module.lost.iter().map(|one| s(one)))),
      ("cut", Object::list(cut.map(|one| s(one)))),
      ("again", Object::list([])),
    ];
    let mut data = pairs;
    for (key, value) in fields {
      match data.iter_mut().find(|one| one.0.as_ref().as_str() == Some(key)) {
        Some(one) => one.1 = value,
        None => data.push((s(key), value)),
      }
    }
    Object::dict(data)
  }

  /// The turns of a chain at the tip of the tail, as a checkpoint holds them: each fact of its
  /// transcript but its module and its carried turns, and of the questions only each reply and each
  /// act that the checkpoint carries, with each start and done of an act that stays.
  fn turns(&self, c: &str, tip: &Tip) -> Vec<Object> {
    let ids = &tip.ids;
    let held = self.held.get(c).map(Vec::as_slice).unwrap_or_default();
    let turns: Vec<&Fact> = held
      .iter()
      .filter(|x| !matches!(x.kind(), "module" | "carried"))
      .filter(|x| !x.question() || x.kind() == "reply" || ids.contains(x.about()))
      .collect();
    let mut stays: HashSet<&str> =
      turns.iter().filter(|x| x.question()).map(|x| x.about()).collect();
    stays.insert(ROOT);
    stays.extend(tip.live.iter().chain(ids).map(String::as_str));
    turns
      .into_iter()
      .filter(|x| !matches!(x.kind(), "started" | "done") || stays.contains(x.about()))
      .map(|x| {
        Object::tuple(x.0.as_ref().items().unwrap_or_default().iter().map(ObjectRef::to_owned))
      })
      .collect()
  }
}

/// The value of a text.
fn s(text: &str) -> Object {
  Object::string(text)
}

/// The text of a field of a map, and an empty text when it holds none.
fn text(one: &Object, name: &str) -> String {
  field(&one.as_ref(), name).and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// The texts of a field of a map that holds a list.
fn strings(one: &Object, name: &str) -> Vec<String> {
  let items = field(&one.as_ref(), name).and_then(|v| v.items()).unwrap_or_default();
  items.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()
}

/// The text of a word of a fact, and an empty text when it is no text.
fn said(fact: &Fact, at: usize) -> String {
  fact.word(at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
}

/// The number of the name of an act, as `rung12` gives 12.
fn number(name: &str) -> i64 {
  name
    .trim_start_matches(|one: char| one.is_ascii_lowercase() || one == '_')
    .parse()
    .unwrap_or_default()
}

/// The data of an act that the checkpoint carries with its value.
fn settled(id: &str, on: &str, value: Object, kind: &str) -> Object {
  let fields = [("kind", s(kind)), ("id", s(id)), ("on", s(on)), ("value", value)];
  Object::dict(fields.map(|(k, v)| (s(k), v)))
}

/// A text in double quotes, as a debug of python writes it.
fn quoted(text: &str) -> String {
  format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Each name that a word of a tell binds, with the text of its value: a tell writes each value as a
/// literal, so the value holds no state of the module.
fn told<'a>(word: &'a str, body: &'a [Stmt]) -> impl Iterator<Item = (&'a str, &'a str)> {
  body.iter().filter_map(move |stmt| {
    let (target, value) = match stmt {
      Stmt::Assign(one) => match one.targets.as_slice() {
        [target] => (target, &*one.value),
        _ => return None,
      },
      Stmt::AnnAssign(one) => (&*one.target, one.value.as_deref()?),
      _ => return None,
    };
    let Expr::Name(name) = target else { return None };
    Some((name.id.as_str(), &word[value.range()]))
  })
}

/// Each act that the text of a value names, as `Act('bash3')`.
fn acts(value: &str) -> Vec<String> {
  let Ok(parsed) = parse_module(value) else { return Vec::new() };
  let mut found = Named(Vec::new());
  found.visit_body(&parsed.syntax().body);
  found.0
}

/// The definitions of a checkpoint, but each statement that binds a name that the tail changed.
fn stripped(definitions: &str, gone: &HashSet<String>) -> String {
  let Ok(parsed) = parse_module(definitions) else { return definitions.to_owned() };
  let kept = parsed.syntax().body.iter().filter(|stmt| {
    let mut changes = Changes::default();
    changes.visit_stmt(stmt);
    !changes.bound.iter().any(|one| gone.contains(*one))
  });
  kept.map(|stmt| &definitions[stmt.range()]).collect::<Vec<_>>().join("\n\n")
}

/// Each act that a value names by `Act` and a literal of its name.
struct Named(Vec<String>);

impl<'a> Visitor<'a> for Named {
  fn visit_expr(&mut self, expr: &'a Expr) {
    if let Expr::Call(call) = expr
      && let Expr::Name(name) = &*call.func
      && name.id.as_str() == "Act"
      && let [Expr::StringLiteral(id)] = &*call.arguments.args
    {
      self.0.push(id.value.to_str().to_owned());
    }
    visitor::walk_expr(self, expr);
  }
}

/// What the statements of a word change in the module that it runs in: each name that they bind or
/// delete, as a def, a class, an import, a target, a capture of a pattern or the name of an except,
/// or whose item or attribute they set or delete; and each name whose method they call, which can
/// change the value in place. A def, a class and a lambda change nothing by their body until they
/// run.
#[derive(Default)]
struct Changes<'a> {
  bound: Vec<&'a str>,
  called: Vec<&'a str>,
}

impl<'a> Visitor<'a> for Changes<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(one) => self.bound.push(one.name.as_str()),
      Stmt::ClassDef(one) => self.bound.push(one.name.as_str()),
      Stmt::Import(one) => self.bound.extend(one.names.iter().map(|alias| {
        alias
          .asname
          .as_ref()
          .map_or_else(|| alias.name.split('.').next().unwrap_or_default(), |name| name.as_str())
      })),
      Stmt::ImportFrom(one) => self.bound.extend(
        one.names.iter().map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str()),
      ),
      _ => visitor::walk_stmt(self, stmt),
    }
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    let set = |ctx: ExprContext| matches!(ctx, ExprContext::Store | ExprContext::Del);
    match expr {
      Expr::Lambda(_) => return,
      Expr::Name(one) if set(one.ctx) => self.bound.push(one.id.as_str()),
      Expr::Attribute(one) if set(one.ctx) => self.bound.extend(base(&one.value)),
      Expr::Subscript(one) if set(one.ctx) => self.bound.extend(base(&one.value)),
      Expr::Call(one) => {
        if let Expr::Attribute(method) = &*one.func {
          self.called.extend(base(&method.value));
        }
      }
      _ => {}
    }
    visitor::walk_expr(self, expr);
  }

  fn visit_comprehension(&mut self, comprehension: &'a Comprehension) {
    self.visit_expr(&comprehension.iter);
    for one in &comprehension.ifs {
      self.visit_expr(one);
    }
  }

  fn visit_pattern(&mut self, pattern: &'a Pattern) {
    let name = match pattern {
      Pattern::MatchAs(one) => one.name.as_ref(),
      Pattern::MatchStar(one) => one.name.as_ref(),
      Pattern::MatchMapping(one) => one.rest.as_ref(),
      _ => None,
    };
    self.bound.extend(name.map(|one| one.as_str()));
    visitor::walk_pattern(self, pattern);
  }

  fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
    let ExceptHandler::ExceptHandler(one) = handler;
    self.bound.extend(one.name.iter().map(|one| one.as_str()));
    visitor::walk_except_handler(self, handler);
  }
}

/// The name that an item or an attribute stands on, as `queue` of `queue[0].items`.
fn base(expr: &Expr) -> Option<&str> {
  match expr {
    Expr::Name(one) => Some(one.id.as_str()),
    Expr::Attribute(one) => base(&one.value),
    Expr::Subscript(one) => base(&one.value),
    _ => None,
  }
}
