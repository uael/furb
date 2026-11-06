//! The layout of a sheet: each run of words of the program as a module of its own, which the
//! checker reads once, and the bindings that keep the type of each name across the tries of the
//! words.

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  hash::{DefaultHasher, Hash, Hasher},
  rc::Rc,
};

use ruff_python_ast::{
  ExceptHandler, Expr, ExprContext, Identifier, Pattern, Stmt,
  statement_visitor::{StatementVisitor, walk_stmt},
  visitor::{self, Visitor},
};
use ruff_python_parser::{Mode, ParseOptions, parse_module, parse_unchecked};
use ruff_text_size::Ranged;

use super::{
  ACTS, MODULE, Parsed, WORDS, binds, deleted, named, shallow::shallowed, strings, worded,
};

/// The words of the program that one module of the checker holds. A module holds the words of a run
/// of `CHUNK` words and stays the same while the program grows, so the checker infers each word of
/// the program once, and a reading costs the words after the last full module and the word.
pub(super) const CHUNK: usize = 64;

/// The sheet laid out as the checker reads it: the modules of the program, each by its path and its
/// text, the text of the sheet that stands on them, the line of the given sheet that each of its
/// lines stands for, and the place in the program of the word that declares each name last.
pub(super) struct Laid {
  pub(super) modules: Vec<(String, Rc<str>)>,
  pub(super) main: String,
  pub(super) lines: Vec<usize>,
  pub(super) declares: HashMap<String, usize>,
  pub(super) coroutines: HashSet<String>,
}

/// What the program binds after its full modules: the module that bound each name last, each name
/// that a word declares and the place of the word that declares it last, each name that a word
/// binds, each name that a def of async declares last, and the modules.
#[derive(Clone, Default)]
struct Laying {
  source: HashMap<String, String>,
  declared: HashMap<String, usize>,
  coroutines: HashSet<String>,
  bound: HashSet<String>,
  modules: Vec<(String, Rc<str>)>,
}

impl Laying {
  /// One more word of the program: each name that it binds is no longer async, then each name that
  /// it declares is async when its last declaration is an async def, and each name that it deletes
  /// is not.
  fn asynchronous(&mut self, unit: &Unit) {
    for name in unit.names.iter().chain(&unit.inner) {
      self.coroutines.remove(name);
    }
    for name in &unit.declares {
      if unit.asyncs.contains(name) {
        self.coroutines.insert(name.clone());
      } else {
        self.coroutines.remove(name);
      }
    }
    for name in &unit.deletes {
      self.coroutines.remove(name);
    }
  }
}

/// The most states of a program after a full module that the gate keeps, before it lays each
/// program out again.
const STATES: usize = 4096;

thread_local! {
  /// The state of a program after each of its full modules, by a digest of the words before it, so
  /// a reading lays out only the words after the last full module.
  static STATES_HELD: RefCell<HashMap<u64, Rc<Laying>>> = RefCell::new(HashMap::new());
}

/// The sheet laid out as modules, so the checker infers each word of the program once.
///
/// The checker reads a module whose text it read before from its cache, and the sheet writes the
/// whole program in one function, which it infers again whole at each reading. So each run of
/// `CHUNK` words of the program stands at the top of a module of its own, with the names of the
/// engine in a module under them all, and only the words after the last full module stand with the
/// word, in the function of the sheet, which the word stands in as it stands in the short sheet. A
/// module binds each name that its words name and that a module before it bound, from the last
/// module that bound it, so each name keeps the type that the words before gave it: a name that a
/// word declares, with a def, a class, an import or a type, is imported, which keeps its
/// declaration, and any other name is bound to the attribute of that module, which a later value
/// of another type may bind again. Each module is named by a digest of its text, so two chains
/// share no module that differs.
pub(super) fn laid(lines: &[&str], first: usize, tail: usize, short: &str) -> Option<Laid> {
  let (base, unbound) = based(&lines[2..first]);
  let units = units(lines, first, tail)?;
  let full = units.len() / CHUNK * CHUNK;
  let keys = keyed(&base, lines, &units[..full]);
  let held = STATES_HELD.with_borrow(|held| {
    (0..keys.len()).rev().find_map(|at| held.get(&keys[at]).map(|one| (at, Rc::clone(one))))
  });
  let (done, mut state) = match held {
    Some((at, one)) => (at, Laying::clone(&one)),
    None => (0, opened(&base, keys[0])?),
  };
  for (at, chunk) in units[..full].chunks(CHUNK).enumerate().skip(done) {
    chunked(&mut state, lines, chunk, at);
    remember(keys[at + 1], &state);
  }
  let shorts: Vec<&str> = short.split_inclusive('\n').collect();
  let word: String = shorts[tail.min(shorts.len())..]
    .iter()
    .map(|line| line.strip_prefix("  ").unwrap_or(line))
    .collect();
  let parsed = parse_unchecked(&word, ParseOptions::from(Mode::Module));
  let named = parsed
    .syntax()
    .as_module()
    .map_or(vec![], |one| Names::of(&one.body).0.into_iter().map(str::to_owned).collect());
  let mut rest = Rest { named, ..Rest::default() };
  for (place, unit) in units.iter().enumerate().skip(full) {
    rest.add(&mut state, lines, *unit, place);
  }
  for (at, line) in shorts.iter().enumerate().skip(tail) {
    rest.text.push_str(line);
    rest.stands.push(at + 1);
  }
  let named: Vec<&str> = rest.named.iter().map(String::as_str).collect();
  let (seeds, imports) = seeded(&named, &state.source, &state.declared, "  ");
  let header = format!("{imports}async def __body():\n{unbound}{seeds}");
  let mut stands = vec![1; header.matches('\n').count()];
  stands.extend(rest.stands);
  Some(Laid {
    modules: state.modules,
    main: header + &rest.text,
    lines: stands,
    declares: state.declared,
    coroutines: state.coroutines,
  })
}

/// The module of the names of the engine that the head of the sheet binds, under each module of the
/// program, and the names that it declares with no value, which stand in the function of the
/// sheet.
fn based(head: &[&str]) -> (String, String) {
  let (mut base, mut unbound) = (format!("import {MODULE}\n"), String::new());
  for line in head {
    if line.starts_with(&format!("  {WORDS} = ")) || line.starts_with(&format!("  {ACTS} = ")) {
      continue;
    }
    if line.ends_with(": object\n") && !line.contains(" = ") {
      unbound.push_str(line);
    } else {
      base.push_str(line.strip_prefix("  ").unwrap_or(line));
    }
  }
  (base, unbound)
}

/// The first and the last line of each word of the program, in its try, as `furb.sheet` writes it.
fn units(lines: &[&str], first: usize, tail: usize) -> Option<Vec<(usize, usize)>> {
  let mut units = vec![];
  let mut at = first;
  while at < tail {
    let close = (at + 2..tail).find(|one| lines[*one] == "  except BaseException:\n")?;
    units.push((at, close + 1));
    at = close + 2;
  }
  Some(units)
}

/// The digest of the program before each full module: of the module of the engine, then of each
/// run of `CHUNK` words after it.
fn keyed(base: &str, lines: &[&str], full: &[(usize, usize)]) -> Vec<u64> {
  let mut key = digest(0, &[base]);
  let mut keys = vec![key];
  for chunk in full.chunks(CHUNK) {
    key = digest(key, &lines[chunk[0].0..=chunk[chunk.len() - 1].1]);
    keys.push(key);
  }
  keys
}

/// A digest of lines after the digest of what stands before them.
fn digest(seed: u64, part: &[&str]) -> u64 {
  let mut hasher = DefaultHasher::new();
  seed.hash(&mut hasher);
  part.hash(&mut hasher);
  hasher.finish()
}

/// The state of a program before its first full module, which the gate keeps by its digest: the
/// module of the engine, and each name that it binds.
fn opened(base: &str, key: u64) -> Option<Laying> {
  let parsed = parse_module(base).ok()?;
  let name = module_name(base);
  let mut state =
    Laying { modules: vec![(format!("{name}.py"), Rc::from(base))], ..Laying::default() };
  for stmt in &parsed.syntax().body {
    for one in binds(stmt) {
      state.bound.insert(one.to_owned());
      state.source.insert(one.to_owned(), name.clone());
      // A name that the sheet declares, which no word of the program declares.
      if matches!(stmt, Stmt::AnnAssign(_)) {
        state.declared.insert(one.to_owned(), usize::MAX);
      }
    }
  }
  remember(key, &state);
  Some(state)
}

/// The state of a program after one more full module, the one at `at`, of the words that `chunk`
/// holds: the module binds each name that its words name from the module that bound it last.
fn chunked(state: &mut Laying, lines: &[&str], chunk: &[(usize, usize)], at: usize) {
  let mut module = String::new();
  let mut named: Vec<String> = vec![];
  let mut binders: Vec<String> = vec![];
  let mut declares: Vec<(String, usize)> = vec![];
  let (mut unsourced, mut undeclared): (Vec<String>, Vec<String>) = (vec![], vec![]);
  for (place, (from, to)) in chunk.iter().enumerate() {
    let (text, unit, _) = rebinding_unit(&lines[*from..=*to], &mut state.bound);
    module.push_str(&text);
    named.extend(unit.named.iter().cloned());
    unsourced.retain(|one| !unit.names.contains(one) && !unit.inner.contains(one));
    binders.extend(unit.names.iter().chain(&unit.inner).cloned());
    undeclared.retain(|one| !unit.declares.contains(one));
    declares.extend(unit.declares.iter().map(|one| (one.clone(), at * CHUNK + place)));
    state.asynchronous(&unit);
    for one in &unit.deletes {
      declares.retain(|(name, _)| name != one);
      undeclared.push(one.clone());
      if builtin(one) {
        binders.push(one.clone());
        unsourced.retain(|name| name != one);
      } else {
        binders.retain(|name| name != one);
        unsourced.push(one.clone());
      }
    }
  }
  for one in &unsourced {
    state.source.remove(one);
  }
  for one in &undeclared {
    state.declared.remove(one);
  }
  let named: Vec<&str> = named.iter().map(String::as_str).collect();
  let (seeds, imports) = seeded(&named, &state.source, &state.declared, "");
  let module = imports + &seeds + &module;
  let name = module_name(&module);
  for one in binders {
    state.source.insert(one, name.clone());
  }
  state.declared.extend(declares);
  state.modules.push((format!("{name}.py"), Rc::from(module.as_str())));
}

/// The words of the program after its full modules, as they stand in the function of the sheet:
/// their text, the line of the given sheet that each of its lines stands for, and each name that
/// they and the word name.
#[derive(Default)]
struct Rest {
  text: String,
  stands: Vec<usize>,
  named: Vec<String>,
}

impl Rest {
  /// One more word of the program, the one at `place`, from the line `from` to the line `to`.
  fn add(&mut self, state: &mut Laying, lines: &[&str], (from, to): (usize, usize), place: usize) {
    let (text, unit, added) = rebinding_unit(&lines[from..=to], &mut state.bound);
    self.named.extend(unit.named.iter().cloned());
    for one in &unit.declares {
      state.declared.insert(one.clone(), place);
    }
    state.asynchronous(&unit);
    for one in &unit.deletes {
      state.declared.remove(one);
    }
    for (at, line) in text.split_inclusive('\n').enumerate() {
      self.text.push_str("  ");
      self.text.push_str(line);
      self.stands.push(if at < added { from + 1 } else { from + 1 + (at - added).min(to - from) });
    }
  }
}

/// The state of a program after a full module, kept by the digest of the words before it.
fn remember(key: u64, state: &Laying) {
  STATES_HELD.with_borrow_mut(|held| {
    if held.len() >= STATES {
      held.clear();
    }
    held.insert(key, Rc::new(state.clone()));
  });
}

/// The text of one word of the program at the top of a module, after the line that rebinds the
/// names that it binds again and before the line that restores the names that it deletes, what the
/// gate read of it, and how many lines stand before the word. Each name that it binds is bound
/// after it, and each name that it deletes is not, but a builtin.
fn rebinding_unit(lines: &[&str], bound: &mut HashSet<String>) -> (String, Rc<Unit>, usize) {
  let unit = read_unit(lines);
  let names: Vec<&str> = unit.names.iter().map(String::as_str).collect();
  let inner: Vec<&str> = unit.inner.iter().map(String::as_str).collect();
  let gone: Vec<&str> = unit.deletes.iter().map(String::as_str).collect();
  let added = rebinding(&names, &inner, |name| bound.contains(name))
    .map_or(String::new(), |line| format!("{line}\n"));
  let after = restoring(&gone).map_or(String::new(), |line| format!("{line}\n"));
  bound.extend(unit.names.iter().chain(&unit.inner).cloned());
  bound.retain(|name| !gone.contains(&name.as_str()) || builtin(name));
  let before = added.matches('\n').count();
  (added + &unit.text + &after, unit, before)
}

/// The sheet with each word of the program cut, which keeps its lines, for the readings of the word
/// alone: an empty try of a word stands at the end of the lines of the program, so the word still
/// stands after a try of a word.
pub(super) fn shortened(lines: &[&str], first: usize, tail: usize) -> String {
  let mut text = lines[..first].concat();
  text.extend(std::iter::repeat_n('\n', tail - first - 4));
  text.push_str("  try:\n    acting()\n  except BaseException:\n    pass\n");
  text.push_str(&lines[tail..].concat());
  text
}

/// The line where the program of a sheet starts and the line where the word starts, as `furb.sheet`
/// lays them: each word of the program in a try that calls `acting()` first and passes on
/// `BaseException`, one for each name of `WORDS`, or one empty try when the program holds no word.
pub(super) fn program_lines(lines: &[&str]) -> Option<(usize, usize)> {
  let opens =
    |at: usize| lines.get(at) == Some(&"  try:\n") && lines.get(at + 1) == Some(&"    acting()\n");
  let first = (2..lines.len()).find(|at| opens(*at))?;
  let words = lines[2..first]
    .iter()
    .find(|line| line.starts_with(&format!("  {WORDS} = ")))
    .and_then(|line| parse_module(&line[2..]).ok())
    .map_or(0, |parsed| strings(&parsed.syntax().body, WORDS).len());
  let mut at = first;
  for _ in 0..words.max(1) {
    if !opens(at) {
      return None;
    }
    at = (at + 2..lines.len()).find(|one| {
      lines[*one] == "  except BaseException:\n" && lines.get(one + 1) == Some(&"    pass\n")
    })?
      + 2;
  }
  Some((first, at))
}

/// What the gate reads of one word of the program, in its try: its text at the top of a module, the
/// names that the statements at its top bind, the names that a statement beneath them binds, the
/// names that it declares, the names that the statements at its top leave deleted, each name that
/// it declares last with an async def, and each name that it names.
struct Unit {
  text: String,
  names: Vec<String>,
  inner: Vec<String>,
  declares: Vec<String>,
  deletes: Vec<String>,
  asyncs: Vec<String>,
  named: Vec<String>,
}

/// The most words of the program that the gate keeps what it read of, before it reads each again.
const UNITS: usize = 200_000;

thread_local! {
  /// What the gate read of each word of the program, by a digest of its lines, so a reading parses
  /// only the words that it did not read before.
  static UNITS_READ: RefCell<HashMap<u64, Rc<Unit>>> = RefCell::new(HashMap::new());
}

/// What the gate reads of the lines of one word of the program, in its try, from what it read
/// before when it can.
fn read_unit(lines: &[&str]) -> Rc<Unit> {
  let mut hasher = DefaultHasher::new();
  lines.hash(&mut hasher);
  let key = hasher.finish();
  if let Some(held) = UNITS_READ.with_borrow(|held| held.get(&key).cloned()) {
    return held;
  }
  let text = shallowed(
    &lines.iter().map(|line| line.strip_prefix("  ").unwrap_or(line)).collect::<String>(),
  );
  let parsed = parse_unchecked(&text, ParseOptions::from(Mode::Module));
  let body = parsed.syntax().as_module().map_or(&[][..], |one| one.body.as_slice());
  let tried = body.iter().find_map(|stmt| match stmt {
    Stmt::Try(one) => Some(one.body.as_slice()),
    _ => None,
  });
  let owned = |names: Vec<&str>| names.into_iter().map(str::to_owned).collect::<Vec<_>>();
  let (names, inner, declares, deletes, asyncs) =
    tried.map_or((vec![], vec![], vec![], vec![], vec![]), |inside| {
      let mut nested = Nested(vec![]);
      nested.visit_body(inside);
      let names = owned(inside.iter().flat_map(binds).collect());
      let declared = Declared::of(inside);
      (names, owned(nested.0), owned(declared.0), owned(deleted(inside)), owned(declared.1))
    });
  let unit = Rc::new(Unit {
    text,
    names,
    inner,
    declares,
    deletes,
    asyncs,
    named: owned(Names::of(body).0),
  });
  UNITS_READ.with_borrow_mut(|held| {
    if held.len() >= UNITS {
      held.clear();
    }
    held.insert(key, Rc::clone(&unit));
  });
  unit
}

/// The lines that bind each name that statements name from the module that bound it last, at an
/// indent, and the imports of those modules. A name that a statement before declared is imported,
/// so it keeps its declaration.
fn seeded(
  names: &[&str],
  source: &HashMap<String, String>,
  declared: &HashMap<String, usize>,
  indent: &str,
) -> (String, String) {
  let mut seeds: Vec<String> = vec![];
  let mut modules: Vec<&str> = vec![];
  let mut sorted: Vec<&str> = unique(names.iter().copied());
  sorted.sort_unstable();
  for name in sorted {
    let Some(module) = source.get(name) else { continue };
    if declared.contains_key(name) {
      seeds.push(format!("{indent}from {module} import {name}\n"));
    } else {
      seeds.push(format!("{indent}{name} = {module}.{name}\n"));
      if !modules.contains(&module.as_str()) {
        modules.push(module);
      }
    }
  }
  modules.sort_unstable();
  let imports: Vec<String> = modules.iter().map(|module| format!("import {module}\n")).collect();
  (seeds.concat(), imports.concat())
}

/// The name of the module of a text, a digest of it, which python can import.
fn module_name(text: &str) -> String {
  let mut hasher = DefaultHasher::new();
  text.hash(&mut hasher);
  format!("__w{:016x}", hasher.finish())
}

/// Each name that statements name, at any depth: each name that an expression reads, binds or
/// deletes, a def, a class, an import, a capture of a pattern and the name of an except.
struct Names<'a>(Vec<&'a str>);

impl<'a> Names<'a> {
  fn of(body: &'a [Stmt]) -> Self {
    let mut names = Names(vec![]);
    names.visit_body(body);
    names
  }
}

impl<'a> Visitor<'a> for Names<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(one) => self.0.push(one.name.as_str()),
      Stmt::ClassDef(one) => self.0.push(one.name.as_str()),
      Stmt::Import(_) | Stmt::ImportFrom(_) => self.0.extend(binds(stmt)),
      Stmt::Global(one) => self.0.extend(one.names.iter().map(Identifier::as_str)),
      Stmt::Nonlocal(one) => self.0.extend(one.names.iter().map(Identifier::as_str)),
      _ => {}
    }
    visitor::walk_stmt(self, stmt);
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    if let Expr::Name(one) = expr {
      self.0.push(one.id.as_str());
    }
    visitor::walk_expr(self, expr);
  }

  fn visit_pattern(&mut self, pattern: &'a Pattern) {
    let name = match pattern {
      Pattern::MatchAs(one) => one.name.as_ref(),
      Pattern::MatchStar(one) => one.name.as_ref(),
      Pattern::MatchMapping(one) => one.rest.as_ref(),
      _ => None,
    };
    self.0.extend(name.map(Identifier::as_str));
    visitor::walk_pattern(self, pattern);
  }

  fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
    let ExceptHandler::ExceptHandler(one) = handler;
    self.0.extend(one.name.iter().map(Identifier::as_str));
    visitor::walk_except_handler(self, handler);
  }
}

/// Each name that statements declare in their scope, at any depth but in a def or a class: a def, a
/// class, an import, a type alias and a name with a type. Then each of them whose last declaration
/// is an async def.
struct Declared<'a>(Vec<&'a str>, Vec<&'a str>);

impl<'a> Declared<'a> {
  fn of(body: &'a [Stmt]) -> Self {
    let mut declared = Declared(vec![], vec![]);
    for stmt in body {
      declared.visit_stmt(stmt);
    }
    declared
  }
}

impl<'a> StatementVisitor<'a> for Declared<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(_)
      | Stmt::ClassDef(_)
      | Stmt::Import(_)
      | Stmt::ImportFrom(_)
      | Stmt::TypeAlias(_) => {
        let names = binds(stmt);
        self.1.retain(|name| !names.contains(name));
        if matches!(stmt, Stmt::FunctionDef(def) if def.is_async) {
          self.1.extend(&names);
        }
        self.0.extend(names);
      }
      Stmt::AnnAssign(one) => {
        let names = named(&one.target);
        self.1.retain(|name| !names.contains(name));
        self.0.extend(names);
      }
      _ => walk_stmt(self, stmt),
    }
  }
}

/// The sheet as the checker reads it, and the line of the sheet that each of its lines stands for.
///
/// Each word of the program stands in a try of its own on the sheet, and the checker reads the name
/// after a try through its except, which may come before any statement of the try, so it gives a
/// name that the word binds again the union of each type it was bound to, where python gives it the
/// last. So before the try of each word, the names that the statements at the top of that word
/// bind, and that a statement before it bound, are deleted: the checker reads each through the
/// except as unbound or as the word bound it, which is the type of its last binding and a warning
/// at most. A try of a word is the one that `furb.sheet` writes, which calls `acting()` first and
/// passes on any exception, and the sheet holds the program in the one async function it defines.
///
/// A name that only a nested statement of a word binds, as the target of a loop or a name bound
/// under an if, keeps each binding before it, and the checker reads each of them again at each use
/// of the name. A name that every word binds in a loop then costs the checker a time of the square
/// of the words. So before the try of such a word, each such name that a statement before it bound
/// is bound to itself, which gives it the same type, as one binding.
pub(super) fn rebound(sheet: &Parsed<'_>) -> (String, Vec<usize>) {
  let lines: Vec<&str> = sheet.text.split_inclusive('\n').collect();
  let inserted = rebinds(sheet);
  let (mut text, mut stands) = (String::with_capacity(sheet.text.len()), vec![]);
  let mut inserts = inserted.into_iter().peekable();
  for (at, line) in lines.iter().enumerate() {
    while let Some((_, added)) = inserts.next_if(|(before, _)| *before == at) {
      text.push_str(&added);
      stands.push(at + 1);
    }
    text.push_str(line);
    stands.push(at + 1);
  }
  for (_, added) in inserts {
    text.push_str(&added);
    stands.push(lines.len());
  }
  (text, stands)
}

/// The lines that `rebound` inserts, each by the line of the sheet that it stands before.
fn rebinds(sheet: &Parsed<'_>) -> Vec<(usize, String)> {
  let lines: Vec<&str> = sheet.text.split_inclusive('\n').collect();
  let mut inserted: Vec<(usize, String)> = vec![];
  if let Some(body) = sheet.bodied() {
    let mut bound: HashSet<&str> = HashSet::new();
    for stmt in body {
      let (names, inner): (Vec<&str>, Vec<&str>) = match stmt {
        Stmt::Try(one) if worded(one) => {
          let mut nested = Nested(vec![]);
          nested.visit_body(&one.body);
          (one.body.iter().flat_map(binds).collect(), nested.0)
        }
        other => (binds(other), vec![]),
      };
      if matches!(stmt, Stmt::Try(one) if worded(one))
        && let Some(added) = rebinding(&names, &inner, |name| bound.contains(name))
      {
        let line = sheet.line(stmt.start().to_usize()) - 1;
        let indent: String = lines[line].chars().take_while(|one| *one == ' ').collect();
        inserted.push((line, format!("{indent}{added}\n")));
      }
      bound.extend(names);
      bound.extend(inner);
      if let Stmt::Try(one) = stmt
        && worded(one)
      {
        let gone = deleted(&one.body);
        if let Some(added) = restoring(&gone) {
          let line = sheet.line(stmt.start().to_usize()) - 1;
          let indent: String = lines[line].chars().take_while(|one| *one == ' ').collect();
          let after = sheet.line(stmt.end().to_usize());
          inserted.push((after, format!("{indent}{added}\n")));
        }
        bound.retain(|name| !gone.contains(name) || builtin(name));
      }
    }
  }
  inserted
}

/// The statement that stands before the try of a word: it deletes each name that the statements at
/// the top of the word bind and that a statement before it bound, and binds to itself each other
/// name that a statement of the word binds and that a statement before it bound. Nothing when no
/// such name is.
fn rebinding(names: &[&str], inner: &[&str], bound: impl Fn(&str) -> bool) -> Option<String> {
  let again = unique(names.iter().copied().filter(|name| bound(name)));
  let kept = unique(inner.iter().copied().filter(|name| bound(name) && !names.contains(name)));
  let mut parts: Vec<String> = vec![];
  if !again.is_empty() {
    parts.push(format!("del {}", again.join(", ")));
  }
  parts.extend(kept.iter().map(|name| format!("{name} = {name}")));
  (!parts.is_empty()).then(|| parts.join("; "))
}

/// The name under which the statement after the try of a word reads the builtins, which it deletes
/// at once.
const BUILTINS: &str = "__builtins_of_the_gate";

/// Whether python gives a name that no module binds from its builtins, as `sum`.
fn builtin(name: &str) -> bool {
  ruff_python_stdlib::builtins::is_python_builtin(name, 14, false)
}

/// The statement that stands after the try of a word that deletes names at its top: it binds each
/// builtin of them to that builtin, and deletes each other one again.
///
/// The module of a chain gives a name that a word deleted from its builtins, or from none, so a
/// later word that calls `sum` after a word bound `sum = bash(...)` and another ran `del sum` calls
/// the builtin. But the sheet holds the program in a function, where a name is local and has no
/// builtin, and the checker reads the name after the try through its except, as it was bound before
/// the `del`. So the checker would refuse the call of the builtin, and let a word read a name that
/// no module holds. Nothing when the word deletes no name.
fn restoring(names: &[&str]) -> Option<String> {
  let (kept, gone): (Vec<&str>, Vec<&str>) = names.iter().copied().partition(|name| builtin(name));
  let mut parts: Vec<String> = vec![];
  if !kept.is_empty() {
    parts.push(format!("import builtins as {BUILTINS}"));
    parts.extend(kept.iter().map(|name| format!("{name} = {BUILTINS}.{name}")));
    parts.push(format!("del {BUILTINS}"));
  }
  if !gone.is_empty() {
    parts.push(format!("del {}", gone.join(", ")));
  }
  (!parts.is_empty()).then(|| parts.join("; "))
}

/// The names, in the order they first stand, with each once.
fn unique<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
  let mut seen: HashSet<&str> = HashSet::new();
  names.filter(|name| seen.insert(name)).collect()
}

/// The names that a statement and the statements beneath it bind in its scope: each target, a def,
/// a class, an import and each capture of a pattern, but none in a def, a class, a lambda or a
/// comprehension, which bind in a scope of their own, and not the name of an except, which python
/// deletes at the end of the except.
struct Nested<'a>(Vec<&'a str>);

impl<'a> Visitor<'a> for Nested<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(_) | Stmt::ClassDef(_) | Stmt::Import(_) | Stmt::ImportFrom(_) => {
        self.0.extend(binds(stmt));
      }
      _ => visitor::walk_stmt(self, stmt),
    }
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    match expr {
      Expr::Name(one) if matches!(one.ctx, ExprContext::Store) => self.0.push(one.id.as_str()),
      Expr::Lambda(_)
      | Expr::ListComp(_)
      | Expr::SetComp(_)
      | Expr::DictComp(_)
      | Expr::Generator(_) => {}
      _ => visitor::walk_expr(self, expr),
    }
  }

  fn visit_pattern(&mut self, pattern: &'a Pattern) {
    let name = match pattern {
      Pattern::MatchAs(one) => one.name.as_ref(),
      Pattern::MatchStar(one) => one.name.as_ref(),
      Pattern::MatchMapping(one) => one.rest.as_ref(),
      _ => None,
    };
    self.0.extend(name.map(Identifier::as_str));
    visitor::walk_pattern(self, pattern);
  }
}
