//! The gate: the type checker of monty, reading a sheet.
//!
//! The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word after the
//! program of its chain, and that a response which is not python is a finding like any other. Nothing of that is
//! a policy of a host: it is the contract, and the crate carries it.
//!
//! The sheet the word stands on is `furb.sheet`'s, written the same way by every Kernel, in this sandbox and in
//! the python package alike: every name of the engine, bound from the engine as a module, then the program, then
//! the word. So this reads a text and says what it found on it, each finding by its line. The checker reads against
//! the typeshed of monty, which describes what the sandbox runs but for its modules: it holds stubs that its other
//! stubs import, `types` among them, which the sandbox does not run, and it lacks some modules that the sandbox
//! runs, `ast` among them. So the sandbox alone says which import runs: each import of the sheet runs in a sandbox
//! of its own, and one that raises is refused before the word runs. A word that binds a name that the transcript
//! binds, which the sheet names, is refused too. A value of another type for a name that a rung of the program
//! declares, which a def, a class, an import or a name with a type does, names that rung, as the sheet names it, so a
//! model reads where the type of the name comes from. The errors alone count, since a warning refuses no word.
//!
//! The checker infers a file again whole when its text changes, and the program grows by a word at each reading, so
//! the gate lays each run of words of the program out as a module of its own, which the checker reads once, and only
//! the words after the last full module stand with the word: a reading costs about the new words, and not the whole
//! program, which held a life still for minutes on a program of hours.

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use monty_type_checking::{SourceFile, TypeChecker};
use monty_types::{NamedValues, TypeCheckingConfig, TypeCheckingFormat};
use ruff_python_ast::{
  ExceptHandler, Expr, ExprContext, Parameter, Pattern, Stmt,
  statement_visitor::{StatementVisitor, walk_stmt},
  token::TokenKind,
  visitor::{self, Visitor},
};
use ruff_python_parser::{Mode, ParseOptions, parse_module, parse_unchecked};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
  ENGINE,
  sand::{Nobody, Sand},
  value::Fault,
};

/// The sheet, which is the one file the checker reads.
const SHEET: &str = "sheet.py";

/// The name the checker holds the engine under, which is `MODULE` of `furb.sheet`: the sheet alone imports it.
const MODULE: &str = "__engine__";

/// The rule of the checker that a finding on an import is under, which the findings of the sandbox take too.
const IMPORT: &str = "error[unresolved-import]";

/// The rule of a finding on an await in a function that is not async.
const AWAIT: &str = "error[await-outside-async]";

/// The rule of a finding on a read of asyncio in a word.
const ASYNCIO: &str = "error[asyncio-in-word]";

/// The most brackets, one in another, that the checker reads in a literal. The checker takes time of about the fourth
/// power of the depth of a literal, and more again when a part has no type that it knows, as an empty list: a list of
/// 32 levels with an empty list at the bottom takes 6 s. monty writes a value of about 990 levels as a literal, which
/// a tell puts in the program. Each shape of lists, tuples and dicts that was measured reads in less than 0.05 s at 12
/// levels.
const DEEP: usize = 12;

/// The rule of a finding on a name that the transcript binds.
const TOLD: &str = "error[transcript-name]";

/// The name the sheet binds to the names of the acts of the chain, which is `ACTS` of `furb.sheet`.
const ACTS: &str = "__acts";

/// The name the sheet binds to the names of the rungs of the program, which is `WORDS` of `furb.sheet`.
const WORDS: &str = "__words";

/// The rule of a finding on a value that is not of the type that its name is declared with.
const ASSIGN: &str = "error[invalid-assignment]";

thread_local! {
  /// The one checker of this thread, which every life of the thread and the Kernel of a python host read with,
  /// and nothing before its first reading. It holds one reading of the typeshed and one of the engine, which cost
  /// once, and one of each module of a program, so a reading of a sheet costs the words after the last module and
  /// the word alone, in every life after the first as in the first.
  static CHECKER: RefCell<Option<Held>> = const { RefCell::new(None) };
}

/// The checker of a thread, and the path of each module of a program that it read.
struct Held {
  checker: TypeChecker,
  read: HashSet<String>,
}

/// What the gate found on a sheet, in the order of its lines: each error of the checker by the line it stands on,
/// in the concise form it writes, and no warning, since a warning refuses no word; each import that the
/// sandbox cannot run, in place of what the checker says of the imports; each await in a function that is not
/// async; and each name that the word binds and that the transcript binds. An error on a value of another type for
/// a name that a rung of the program declares names that rung. Each is read on the sheet with each literal that
/// holds more than `DEEP` brackets cut, which the checker could not read in time.
pub fn checked(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
  let lines: Vec<&str> = sheet.split_inclusive('\n').collect();
  let shaped = program_lines(&lines).and_then(|(first, tail)| {
    let short = shallowed(&shortened(&lines, first, tail));
    laid(&lines, first, tail, &short).map(|laid| (short, laid))
  });
  let Some((short, laid)) = shaped else {
    // A sheet that is not laid out as `furb.sheet` writes it is read whole.
    let sheet = &shallowed(sheet);
    let (text, stands) = rebound(sheet);
    let mut found = declared(sheet, typed(&[], &text, &stands)?, None);
    found.extend(unrun(sheet));
    found.extend(unawaited(sheet));
    found.extend(unscheduled(sheet));
    found.extend(told(sheet));
    found.sort_by_key(|(line, _)| *line);
    return Ok(signed(found, &[sheet, ENGINE]));
  };
  let mut found =
    declared(&short, typed(&laid.modules, &laid.main, &laid.lines)?, Some(&laid.declares));
  found.extend(unrun(&short));
  found.extend(unawaited(&short));
  found.extend(unscheduled(&short));
  found.extend(told(&short));
  found.sort_by_key(|(line, _)| *line);
  let mut sources: Vec<&str> = vec![&laid.main];
  sources.extend(laid.modules.iter().rev().map(|(_, module)| &**module));
  sources.push(ENGINE);
  Ok(signed(found, &sources))
}

/// What the checker found on a text that stands on modules, each by the line of the sheet that its line stands for.
/// The checker reads each module that it did not read before, then the text. A checker that could not read them has
/// said nothing of the word, which is not a finding, so it is a fault of the gate.
fn typed(
  modules: &[(String, Rc<str>)],
  text: &str,
  stands: &[usize],
) -> Result<Vec<(usize, String)>, Fault> {
  let config = TypeCheckingConfig { format: TypeCheckingFormat::Concise, color: false };
  let path = format!("{MODULE}/__init__.py");
  let engine = SourceFile::new(ENGINE, &path);
  CHECKER.with_borrow_mut(|held| {
    if held.as_ref().is_some_and(|one| one.read.len() > MODULES) {
      *held = None;
    }
    // The checker writes again every file it is given and reads again all that hangs on it, so the engine is
    // given to a new checker alone.
    let mut given = held.is_none().then_some(&engine);
    let one =
      held.get_or_insert_with(|| Held { checker: TypeChecker::default(), read: HashSet::new() });
    let mut failed = None;
    for (path, module) in modules {
      if one.read.contains(path) {
        continue;
      }
      match one.checker.run(&SourceFile::new(module, path), given.take(), config) {
        Ok(_) => {
          one.read.insert(path.clone());
        }
        Err(why) => {
          failed = Some(why);
          break;
        }
      }
    }
    let got = match failed {
      Some(why) => Err(why),
      None => one.checker.run(&SourceFile::new(text, SHEET), given, config).map(|found| {
        found
          .map(|said| {
            let found = said.to_string().lines().filter_map(read).collect::<Vec<_>>();
            found
              .into_iter()
              .map(|(line, why)| (stands.get(line - 1).copied().unwrap_or(line), why))
              .collect()
          })
          .unwrap_or_default()
      }),
    };
    got.map_err(|why| {
      // A checker that failed may not hold the engine, so the next reading makes a new one and gives it again.
      *held = None;
      Fault::refused(format!("the gate could not read the sheet: {why}"))
    })
  })
}

/// The words of the program that one module of the checker holds. A module holds the words of a run of `CHUNK`
/// words and stays the same while the program grows, so the checker infers each word of the program once, and a
/// reading costs the words after the last full module and the word.
const CHUNK: usize = 64;

/// The most modules that the checker holds before it starts again, so the modules of programs that changed or that
/// a life no longer reads do not grow without end.
const MODULES: usize = 4096;

/// The sheet laid out as the checker reads it: the modules of the program, each by its path and its text, the text
/// of the sheet that stands on them, the line of the given sheet that each of its lines stands for, and the place
/// in the program of the word that declares each name last.
struct Laid {
  modules: Vec<(String, Rc<str>)>,
  main: String,
  lines: Vec<usize>,
  declares: HashMap<String, usize>,
}

/// What the program binds after its full modules: the module that bound each name last, each name that a word
/// declares and the place of the word that declares it last, each name that a word binds, and the modules.
#[derive(Clone, Default)]
struct Laying {
  source: HashMap<String, String>,
  declared: HashMap<String, usize>,
  bound: HashSet<String>,
  modules: Vec<(String, Rc<str>)>,
}

/// The most states of a program after a full module that the gate keeps, before it lays each program out again.
const STATES: usize = 4096;

thread_local! {
  /// The state of a program after each of its full modules, by a digest of the words before it, so a reading lays
  /// out only the words after the last full module.
  static STATES_HELD: RefCell<HashMap<u64, Rc<Laying>>> = RefCell::new(HashMap::new());
}

/// The sheet laid out as modules, so the checker infers each word of the program once.
///
/// The checker reads a module whose text it read before from its cache, and the sheet writes the whole program in one
/// function, which it infers again whole at each reading. So each run of `CHUNK` words of the program stands at the
/// top of a module of its own, with the names of the engine in a module under them all, and only the words after the
/// last full module stand with the word, in the function of the sheet, which the word stands in as it stands in the
/// short sheet. A module binds each name that its words name and that a module before it bound, from the last module
/// that bound it, so each name keeps the type that the words before gave it: a name that a word declares, with a def,
/// a class, an import or a type, is imported, which keeps its declaration, and any other name is bound to the
/// attribute of that module, which a later value of another type may bind again. Each module is named by a digest
/// of its text, so two chains share no module that differs.
fn laid(lines: &[&str], first: usize, tail: usize, short: &str) -> Option<Laid> {
  use std::hash::{Hash, Hasher};
  let (mut base, mut unbound) = (format!("import {MODULE}\n"), String::new());
  for line in &lines[2..first] {
    if line.starts_with(&format!("  {WORDS} = ")) || line.starts_with(&format!("  {ACTS} = ")) {
      continue;
    }
    if line.ends_with(": object\n") && !line.contains(" = ") {
      unbound.push_str(line);
    } else {
      base.push_str(line.strip_prefix("  ").unwrap_or(line));
    }
  }
  let mut units: Vec<(usize, usize)> = vec![];
  let mut at = first;
  while at < tail {
    let close = (at + 2..tail).find(|one| lines[*one] == "  except BaseException:\n")?;
    units.push((at, close + 1));
    at = close + 2;
  }
  let digest = |seed: u64, part: &[&str]| {
    let mut hasher = std::hash::DefaultHasher::new();
    seed.hash(&mut hasher);
    part.hash(&mut hasher);
    hasher.finish()
  };
  let full = units.len() / CHUNK * CHUNK;
  // The digest of the program before each full module, from the last one whose state the gate holds.
  let mut key = digest(0, &[base.as_str()]);
  let mut keys = vec![key];
  for chunk in units[..full].chunks(CHUNK) {
    key = digest(key, &lines[chunk[0].0..=chunk[chunk.len() - 1].1]);
    keys.push(key);
  }
  let held = STATES_HELD.with_borrow(|held| {
    (0..keys.len()).rev().find_map(|at| held.get(&keys[at]).map(|one| (at, one.clone())))
  });
  let (done, mut state) = match held {
    Some((at, one)) => (at, (*one).clone()),
    None => {
      let parsed = parse_module(&base).ok()?;
      let name = module_name(&base);
      let mut state = Laying {
        modules: vec![(format!("{name}.py"), Rc::from(base.as_str()))],
        ..Laying::default()
      };
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
      remember(keys[0], &state);
      (0, state)
    }
  };
  for (at, chunk) in units[..full].chunks(CHUNK).enumerate().skip(done) {
    let mut module = String::new();
    let mut named: Vec<String> = vec![];
    let mut binders: Vec<String> = vec![];
    let mut declares: Vec<(String, usize)> = vec![];
    for (place, (from, to)) in chunk.iter().enumerate() {
      let (text, unit) = rebinding_unit(&lines[*from..=*to], &mut state.bound);
      module.push_str(&text);
      named.extend(unit.named.iter().cloned());
      binders.extend(unit.names.iter().chain(&unit.inner).cloned());
      declares.extend(unit.declares.iter().map(|one| (one.clone(), at * CHUNK + place)));
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
    remember(keys[at + 1], &state);
  }
  let shorts: Vec<&str> = short.split_inclusive('\n').collect();
  let word: String = shorts[tail.min(shorts.len())..]
    .iter()
    .map(|line| line.strip_prefix("  ").unwrap_or(line))
    .collect();
  let parsed = parse_unchecked(&word, ParseOptions::from(Mode::Module));
  let mut named: Vec<String> = parsed
    .syntax()
    .as_module()
    .map_or(vec![], |one| Names::of(&one.body).0.into_iter().map(str::to_owned).collect());
  let mut rest = String::new();
  let mut stands: Vec<usize> = vec![];
  for (place, (from, to)) in units.iter().enumerate().skip(full) {
    let (text, unit) = rebinding_unit(&lines[*from..=*to], &mut state.bound);
    named.extend(unit.named.iter().cloned());
    for one in &unit.declares {
      state.declared.insert(one.clone(), place);
    }
    let added = text.lines().count() - (to - from + 1);
    for (at, line) in text.split_inclusive('\n').enumerate() {
      rest.push_str("  ");
      rest.push_str(line);
      stands.push(if at < added { from + 1 } else { from + 1 + at - added });
    }
  }
  for (at, line) in shorts.iter().enumerate().skip(tail) {
    rest.push_str(line);
    stands.push(at + 1);
  }
  let named: Vec<&str> = named.iter().map(String::as_str).collect();
  let (seeds, imports) = seeded(&named, &state.source, &state.declared, "  ");
  let header = format!("{imports}async def __body():\n{unbound}{seeds}");
  let mut lines_of = vec![1; header.matches('\n').count()];
  lines_of.extend(stands);
  Some(Laid {
    modules: state.modules,
    main: header + &rest,
    lines: lines_of,
    declares: state.declared,
  })
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

/// The text of one word of the program at the top of a module, after the line that rebinds the names that it binds
/// again, and what the gate read of it. Each name that it binds is bound after it.
fn rebinding_unit(lines: &[&str], bound: &mut HashSet<String>) -> (String, Rc<Unit>) {
  let unit = read_unit(lines);
  let names: Vec<&str> = unit.names.iter().map(String::as_str).collect();
  let inner: Vec<&str> = unit.inner.iter().map(String::as_str).collect();
  let added = rebinding(&names, &inner, |name| bound.contains(name))
    .map_or(String::new(), |line| format!("{line}\n"));
  bound.extend(unit.names.iter().chain(&unit.inner).cloned());
  (added + &unit.text, unit)
}

/// The sheet with each word of the program cut, which keeps its lines, for the readings of the word alone: an empty
/// try of a word stands at the end of the lines of the program, so the word still stands after a try of a word.
fn shortened(lines: &[&str], first: usize, tail: usize) -> String {
  let mut text = lines[..first].concat();
  text.extend(std::iter::repeat_n('\n', tail - first - 4));
  text.push_str("  try:\n    acting()\n  except BaseException:\n    pass\n");
  text.push_str(&lines[tail..].concat());
  text
}

/// The line where the program of a sheet starts and the line where the word starts, as `furb.sheet` lays them: each
/// word of the program in a try that calls `acting()` first and passes on `BaseException`, one for each name of
/// `WORDS`, or one empty try when the program holds no word.
fn program_lines(lines: &[&str]) -> Option<(usize, usize)> {
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

/// What the gate reads of one word of the program, in its try: its text at the top of a module, the names that the
/// statements at its top bind, the names that a statement beneath them binds, the names that it declares, and each
/// name that it names.
struct Unit {
  text: String,
  names: Vec<String>,
  inner: Vec<String>,
  declares: Vec<String>,
  named: Vec<String>,
}

/// The most words of the program that the gate keeps what it read of, before it reads each again.
const UNITS: usize = 200_000;

thread_local! {
  /// What the gate read of each word of the program, by a digest of its lines, so a reading parses only the words
  /// that it did not read before.
  static UNITS_READ: RefCell<HashMap<u64, Rc<Unit>>> = RefCell::new(HashMap::new());
}

/// What the gate reads of the lines of one word of the program, in its try, from what it read before when it can.
fn read_unit(lines: &[&str]) -> Rc<Unit> {
  use std::hash::{Hash, Hasher};
  let mut hasher = std::hash::DefaultHasher::new();
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
  let (names, inner, declares) = tried.map_or((vec![], vec![], vec![]), |inside| {
    let mut nested = Nested(vec![]);
    nested.visit_body(inside);
    (owned(inside.iter().flat_map(binds).collect()), owned(nested.0), owned(Declared::of(inside)))
  });
  let named = owned(Names::of(body).0);
  let unit = Rc::new(Unit { text: text.clone(), names, inner, declares, named });
  UNITS_READ.with_borrow_mut(|held| {
    if held.len() >= UNITS {
      held.clear();
    }
    held.insert(key, unit.clone());
  });
  unit
}

/// The lines that bind each name that statements name from the module that bound it last, at an indent, and the
/// imports of those modules. A name that a statement before declared is imported, so it keeps its declaration.
fn seeded(
  names: &[&str],
  source: &HashMap<String, String>,
  declared: &HashMap<String, usize>,
  indent: &str,
) -> (String, String) {
  let mut seeds = String::new();
  let mut modules: Vec<&str> = vec![];
  let mut sorted: Vec<&str> = unique(names.iter().copied());
  sorted.sort_unstable();
  for name in sorted {
    let Some(module) = source.get(name) else { continue };
    if declared.contains_key(name) {
      seeds.push_str(&format!("{indent}from {module} import {name}\n"));
    } else {
      seeds.push_str(&format!("{indent}{name} = {module}.{name}\n"));
      if !modules.contains(&module.as_str()) {
        modules.push(module);
      }
    }
  }
  modules.sort_unstable();
  (seeds, modules.iter().map(|module| format!("import {module}\n")).collect())
}

/// The name of the module of a text, a digest of it, which python can import.
fn module_name(text: &str) -> String {
  use std::hash::{Hash, Hasher};
  let mut hasher = std::hash::DefaultHasher::new();
  text.hash(&mut hasher);
  format!("__w{:016x}", hasher.finish())
}

/// Each name that statements name, at any depth: each name that an expression reads, binds or deletes, a def, a class,
/// an import, a capture of a pattern and the name of an except.
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
      Stmt::Global(one) => self.0.extend(one.names.iter().map(|name| name.as_str())),
      Stmt::Nonlocal(one) => self.0.extend(one.names.iter().map(|name| name.as_str())),
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
    self.0.extend(name.map(|name| name.as_str()));
    visitor::walk_pattern(self, pattern);
  }

  fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
    let ExceptHandler::ExceptHandler(one) = handler;
    self.0.extend(one.name.iter().map(|name| name.as_str()));
    visitor::walk_except_handler(self, handler);
  }
}

/// Each name that statements declare in their scope, at any depth but in a def or a class: a def, a class, an import,
/// a type alias and a name with a type.
struct Declared<'a>(Vec<&'a str>);

impl<'a> Declared<'a> {
  fn of(body: &'a [Stmt]) -> Vec<&'a str> {
    let mut declared = Declared(vec![]);
    for stmt in body {
      declared.visit_stmt(stmt);
    }
    declared.0
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
        self.0.extend(binds(stmt));
      }
      Stmt::AnnAssign(one) => self.0.extend(named(&one.target)),
      _ => walk_stmt(self, stmt),
    }
  }
}

/// The sheet with each literal of a list, a tuple, a set or a dict that holds more than `DEEP` brackets, one in
/// another, cut to an empty one of its kind, which keeps its lines. A value that a tell wrote as a literal stays in
/// the program, so each later word reads it again. The whole literal is cut, and not its deep parts, so no part of a
/// type that the checker does not know stands deep in a union. The cut reads the tokens and not the tree, since the
/// parser gives no tree past about 200 brackets.
fn shallowed(sheet: &str) -> String {
  let parsed = parse_unchecked(sheet, ParseOptions::from(Mode::Module));
  // Each open bracket: where it starts, its kind, whether it opens a literal, whether a colon stands in it as an
  // item of its own, and the most brackets that stand one in another in it, itself among them.
  let mut open: Vec<(TextSize, TokenKind, bool, bool, usize)> = vec![];
  let mut cut: Vec<(TextRange, (&str, &str))> = vec![];
  let (mut before, mut strings) = (TokenKind::Newline, 0usize);
  for token in parsed.tokens().iter() {
    let kind = token.kind();
    match kind {
      TokenKind::FStringStart | TokenKind::TStringStart => strings += 1,
      TokenKind::FStringEnd | TokenKind::TStringEnd => strings = strings.saturating_sub(1),
      TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => {
        // A bracket after a value opens a call or a subscript, and a bracket in an f-string opens a part of it.
        let literal = strings == 0 && (kind == TokenKind::Lbrace || !valued(before));
        open.push((token.start(), kind, literal, false, 1));
      }
      TokenKind::Colon => {
        if let Some(top) = open.last_mut() {
          top.3 = true;
        }
      }
      TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace => {
        if let Some((start, opened, literal, colon, deepest)) = open.pop() {
          if let Some(around) = open.last_mut() {
            around.4 = around.4.max(deepest + 1);
          }
          if literal && deepest > DEEP {
            let marks = match opened {
              TokenKind::Lsqb => ("[", "]"),
              TokenKind::Lpar => ("tuple([", "])"),
              _ if colon => ("{", "}"),
              _ => ("set(", ")"),
            };
            cut.push((TextRange::new(start, token.end()), marks));
          }
        }
      }
      _ => {}
    }
    if !matches!(kind, TokenKind::Comment | TokenKind::NonLogicalNewline) {
      before = kind;
    }
  }
  cut.sort_by_key(|(range, _)| range.start());
  let (mut text, mut at) = (String::with_capacity(sheet.len()), 0);
  for (range, (first, last)) in cut {
    let (from, to) = (range.start().to_usize(), range.end().to_usize());
    // A literal in a literal that is cut goes with it.
    if from < at {
      continue;
    }
    text.push_str(&sheet[at..from]);
    text.push_str(first);
    text.extend(std::iter::repeat_n('\n', sheet[from..to].matches('\n').count()));
    text.push_str(last);
    at = to;
  }
  text.push_str(&sheet[at..]);
  text
}

/// Whether a token ends a value, so that a bracket after it opens a call or a subscript. A soft keyword counts as a
/// name, so a bracket after one is never cut.
fn valued(token: TokenKind) -> bool {
  matches!(
    token,
    TokenKind::Name
      | TokenKind::Int
      | TokenKind::Float
      | TokenKind::Complex
      | TokenKind::String
      | TokenKind::FStringEnd
      | TokenKind::TStringEnd
      | TokenKind::Rpar
      | TokenKind::Rsqb
      | TokenKind::Rbrace
      | TokenKind::None
      | TokenKind::True
      | TokenKind::False
      | TokenKind::Ellipsis
      | TokenKind::Case
      | TokenKind::Match
      | TokenKind::Type
      | TokenKind::Lazy
  )
}

/// The sheet as the checker reads it, and the line of the sheet that each of its lines stands for.
///
/// Each word of the program stands in a try of its own on the sheet, and the checker reads the name after a try
/// through its except, which may come before any statement of the try, so it gives a name that the word binds again
/// the union of each type it was bound to, where python gives it the last. So before the try of each word, the
/// names that the statements at the top of that word bind, and that a statement before it bound, are deleted: the
/// checker reads each through the except as unbound or as the word bound it, which is the type of its last binding
/// and a warning at most. A try of a word is the one that `furb.sheet` writes, which calls `acting()` first and
/// passes on any exception, and the sheet holds the program in the one async function it defines.
///
/// A name that only a nested statement of a word binds, as the target of a loop or a name bound under an if, keeps
/// each binding before it, and the checker reads each of them again at each use of the name. A name that every word
/// binds in a loop then costs the checker a time of the square of the words. So before the try of such a word, each
/// such name that a statement before it bound is bound to itself, which gives it the same type, as one binding.
fn rebound(sheet: &str) -> (String, Vec<usize>) {
  let lines: Vec<&str> = sheet.split_inclusive('\n').collect();
  let inserted = rebinds(sheet);
  let (mut text, mut stands) = (String::with_capacity(sheet.len()), vec![]);
  let mut inserts = inserted.into_iter().peekable();
  for (at, line) in lines.iter().enumerate() {
    if let Some((_, added)) = inserts.next_if(|(before, _)| *before == at) {
      text.push_str(&added);
      stands.push(at + 1);
    }
    text.push_str(line);
    stands.push(at + 1);
  }
  (text, stands)
}

/// The lines that `rebound` inserts, each by the line of the sheet that it stands before.
fn rebinds(sheet: &str) -> Vec<(usize, String)> {
  let lines: Vec<&str> = sheet.split_inclusive('\n').collect();
  let mut inserted: Vec<(usize, String)> = vec![];
  if let Ok(parsed) = parse_module(sheet) {
    let body = bodied(parsed.syntax());
    let mut bound: HashSet<&str> = HashSet::new();
    for stmt in body.into_iter().flatten() {
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
        let line = sheet[..stmt.start().to_usize()].matches('\n').count();
        let indent: String = lines[line].chars().take_while(|one| *one == ' ').collect();
        inserted.push((line, format!("{indent}{added}\n")));
      }
      bound.extend(names);
      bound.extend(inner);
    }
  }
  inserted
}

/// The statement that stands before the try of a word: it deletes each name that the statements at the top of the word
/// bind and that a statement before it bound, and binds to itself each other name that a statement of the word binds
/// and that a statement before it bound. Nothing when no such name is.
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

/// The names, in the order they first stand, with each once.
fn unique<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
  let mut seen: HashSet<&str> = HashSet::new();
  names.filter(|name| seen.insert(name)).collect()
}

/// The names that a statement and the statements beneath it bind in its scope: each target, a def, a class, an import
/// and each capture of a pattern, but none in a def, a class, a lambda or a comprehension, which bind in a scope of
/// their own, and not the name of an except, which python deletes at the end of the except.
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
    self.0.extend(name.map(|name| name.as_str()));
    visitor::walk_pattern(self, pattern);
  }
}

/// The body of the one async function of the sheet, which holds the program and the word.
fn bodied(module: &ruff_python_ast::ModModule) -> Option<&[Stmt]> {
  module.body.iter().find_map(|stmt| match stmt {
    Stmt::FunctionDef(def) if def.is_async => Some(def.body.as_slice()),
    _ => None,
  })
}

/// The strings of the tuple that the body binds to a name, as the sheet binds `ACTS` and `WORDS`.
fn strings<'a>(body: &'a [Stmt], name: &str) -> Vec<&'a str> {
  body
    .iter()
    .find_map(|stmt| match stmt {
      Stmt::Assign(one) if matches!(one.targets.as_slice(), [Expr::Name(it)] if it.id.as_str() == name) => {
        match &*one.value {
          Expr::Tuple(names) => Some(
            names
              .elts
              .iter()
              .filter_map(|one| match one {
                Expr::StringLiteral(text) => Some(text.value.to_str()),
                _ => None,
              })
              .collect(),
          ),
          _ => None,
        }
      }
      _ => None,
    })
    .unwrap_or_default()
}

/// The statements of the word, which stand after the last try of a word of the program.
fn word_of(body: &[Stmt]) -> &[Stmt] {
  body
    .iter()
    .rposition(|stmt| matches!(stmt, Stmt::Try(one) if worded(one)))
    .map_or(&body[body.len()..], |at| &body[at + 1..])
}

/// Each finding with the rung that declares the name that its line binds, when the finding is an error on the type of
/// that value and a word of the program declares the name.
///
/// A def, a class, an import and a name with a type declare that name, and the checker refuses a later value of
/// another type for it, though python binds it. The checker says only that the value is not assignable to the type,
/// and the model cannot see that an extension declared the name, so it binds the name again. So the finding names
/// the rung that declared the name last, of the names that the sheet binds as `WORDS`, and says to give the value
/// another name. A name that the word binds in a def, a class, a lambda or a comprehension is its own, and names no
/// rung.
fn declared(
  sheet: &str,
  found: Vec<(usize, String)>,
  program: Option<&HashMap<String, usize>>,
) -> Vec<(usize, String)> {
  if !found.iter().any(|(_, why)| why.starts_with(ASSIGN)) {
    return found;
  }
  let Ok(parsed) = parse_module(sheet) else { return found };
  let Some(body) = bodied(parsed.syntax()) else { return found };
  let rungs = strings(body, WORDS);
  let mut declares: Vec<(&str, Option<&str>)> = program
    .map(|held| {
      let words = held.iter().filter(|(_, at)| **at != usize::MAX);
      words.map(|(name, at)| (name.as_str(), rungs.get(*at).copied())).collect()
    })
    .unwrap_or_default();
  let tries = body.iter().filter(|_| program.is_none()).filter_map(|stmt| match stmt {
    Stmt::Try(one) if worded(one) => Some(one),
    _ => None,
  });
  for (at, one) in tries.enumerate() {
    for stmt in &one.body {
      let names = match stmt {
        Stmt::AnnAssign(it) => named(&it.target),
        Stmt::FunctionDef(_)
        | Stmt::ClassDef(_)
        | Stmt::TypeAlias(_)
        | Stmt::Import(_)
        | Stmt::ImportFrom(_) => binds(stmt),
        _ => vec![],
      };
      for name in names {
        declares.retain(|(held, _)| *held != name);
        declares.push((name, rungs.get(at).copied()));
      }
    }
  }
  let mut stored = Stored(vec![]);
  stored.visit_body(word_of(body));
  let line = |at: usize| sheet[..at].matches('\n').count() + 1;
  found
    .into_iter()
    .map(|(at, why)| {
      let by = stored.0.iter().filter(|(start, _)| line(*start) == at).find_map(|(_, name)| {
        declares.iter().find(|(held, _)| held == name).map(|(_, rung)| (*name, *rung))
      });
      match by {
        Some((name, rung)) if why.starts_with(ASSIGN) => {
          let who = rung.map_or("a word of the program of this chain".to_owned(), |one| {
            format!("the word of {one}")
          });
          (
            at,
            format!(
              "{why}; {who} declares `{name}` with that type, so give this value another name"
            ),
          )
        }
        _ => (at, why),
      }
    })
    .collect()
}

/// The names that a body binds in its own scope, each by where it starts: each target of a statement or of an
/// expression, but none in a def, a class, a lambda or a comprehension, which bind in a scope of their own.
struct Stored<'a>(Vec<(usize, &'a str)>);

impl<'a> Visitor<'a> for Stored<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
      visitor::walk_stmt(self, stmt);
    }
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    match expr {
      Expr::Name(one) if matches!(one.ctx, ExprContext::Store) => {
        self.0.push((one.start().to_usize(), one.id.as_str()))
      }
      Expr::Lambda(_)
      | Expr::ListComp(_)
      | Expr::SetComp(_)
      | Expr::DictComp(_)
      | Expr::Generator(_) => {}
      _ => visitor::walk_expr(self, expr),
    }
  }
}

/// Whether a try is the try of a word of the program, as `furb.sheet` writes it: `acting()` first, and one except
/// of `BaseException` that passes.
fn worded(one: &ruff_python_ast::StmtTry) -> bool {
  let opens = matches!(one.body.first(), Some(Stmt::Expr(call)) if matches!(
    &*call.value,
    Expr::Call(called) if called.arguments.is_empty()
      && matches!(&*called.func, Expr::Name(name) if name.id.as_str() == "acting")
  ));
  let passes = matches!(one.handlers.as_slice(), [ExceptHandler::ExceptHandler(caught)] if caught.name.is_none()
    && matches!(caught.body.as_slice(), [Stmt::Pass(_)])
    && matches!(caught.type_.as_deref(), Some(Expr::Name(name)) if name.id.as_str() == "BaseException"));
  opens && passes && one.orelse.is_empty() && one.finalbody.is_empty()
}

/// The names that a statement binds when it runs to its end, though it may read them first: an assignment, an
/// annotated assignment with a value, a def, a class, a type alias and an import. A name that only a nested statement
/// binds may keep the value it had, and an augmented assignment reads the name it binds.
fn binds(stmt: &Stmt) -> Vec<&str> {
  match stmt {
    Stmt::Assign(one) => one.targets.iter().flat_map(named).collect(),
    Stmt::AnnAssign(one) if one.value.is_some() => named(&one.target),
    Stmt::FunctionDef(one) => vec![one.name.as_str()],
    Stmt::ClassDef(one) => vec![one.name.as_str()],
    Stmt::TypeAlias(one) => named(&one.name),
    Stmt::Import(one) => one
      .names
      .iter()
      .map(|alias| match &alias.asname {
        Some(name) => name.as_str(),
        None => alias.name.as_str().split('.').next().unwrap_or_default(),
      })
      .collect(),
    Stmt::ImportFrom(one) => {
      one.names.iter().map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str()).collect()
    }
    _ => vec![],
  }
}

/// The names that a target of an assignment binds.
fn named(target: &Expr) -> Vec<&str> {
  match target {
    Expr::Name(one) => vec![one.id.as_str()],
    Expr::Tuple(one) => one.elts.iter().flat_map(named).collect(),
    Expr::List(one) => one.elts.iter().flat_map(named).collect(),
    Expr::Starred(one) => named(&one.value),
    _ => vec![],
  }
}

/// One finding of the checker in its concise form, as the line it stands on and what it says.
///
/// The form is `path:line:column: severity[rule] why`, and a line that is not of that form, or that is no
/// error, is no finding of the sheet, so it is dropped rather than read as one. What the checker says of an import
/// is dropped too, since the sandbox says it.
fn read(said: &str) -> Option<(usize, String)> {
  let (_, rest) = said.split_once(':')?;
  let (line, rest) = rest.split_once(':')?;
  let (column, rest) = rest.split_once(": ")?;
  if !rest.starts_with("error[") || rest.starts_with(IMPORT) {
    return None;
  }
  let (rule, why) = rest.trim().split_once(' ').unwrap_or((rest.trim(), ""));
  Some((
    line.trim().parse().ok()?,
    format!("{rule} (column {}) {why}", column.trim()).trim_end().to_owned(),
  ))
}

/// The column of a place of a text, counted from 1 in characters, as the checker counts it.
fn column(text: &str, at: usize) -> usize {
  text[..at].rsplit('\n').next().map_or(0, |line| line.chars().count()) + 1
}

/// The most characters of a signature that a finding quotes.
const SIGNED: usize = 300;

/// Each finding that names a function, as the checker writes it, "function `name`", with the signature of that
/// function: its name, its parameters and what it gives, as the last def of that name in the sources reads, which
/// stand from the nearest to the word to the farthest, the engine last. A model that called a function wrongly then
/// reads in the same finding how to call it.
fn signed(found: Vec<(usize, String)>, sources: &[&str]) -> Vec<(usize, String)> {
  found
    .into_iter()
    .map(|(line, why)| {
      let name = why
        .split_once("function `")
        .and_then(|(_, rest)| rest.split_once('`'))
        .map(|(name, _)| name)
        .filter(|name| !name.contains('.'));
      match name.and_then(|name| sources.iter().find_map(|source| signature(source, name))) {
        Some(sign) => (
          line,
          format!("{why}; the signature of `{name}` is `{sign}`", name = name.unwrap_or_default()),
        ),
        None => (line, why),
      }
    })
    .collect()
}

/// The signature of the last def of a name in a source, on one line and cut to `SIGNED` characters.
fn signature(source: &str, name: &str) -> Option<String> {
  let parsed = parse_module(source).ok()?;
  let mut defs = Defs { source, name, found: None };
  defs.visit_body(&parsed.syntax().body);
  defs.found.map(|sign| sign.chars().take(SIGNED).collect())
}

/// The last def of a name in a body, at any depth, read from the name to the end of what it gives.
struct Defs<'a> {
  source: &'a str,
  name: &'a str,
  found: Option<String>,
}

impl<'a> StatementVisitor<'a> for Defs<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(def) if def.name.as_str() == self.name => {
        let end = def.returns.as_ref().map_or(def.parameters.end(), |one| one.end());
        let text = &self.source[def.name.start().to_usize()..end.to_usize()];
        self.found = Some(text.split_whitespace().collect::<Vec<_>>().join(" "));
      }
      _ => {}
    }
    walk_stmt(self, stmt);
  }
}

/// Each import of the sheet that the sandbox cannot run, by the line it stands on, with what the sandbox raised.
///
/// An import runs the same in every sandbox, since monty runs no module that code defines, so a sandbox of its own
/// says what the sandbox of the word will. Each import is read where it stands, in a function or under a test that
/// is false alike, as the checker reads it. The import of the engine that opens the sheet is the sheet's own, and a
/// word that imports the engine by that name is refused, since no chain holds a module of that name.
fn unrun(sheet: &str) -> Vec<(usize, String)> {
  // A sheet that does not parse is refused by what the checker says of it, whatever it imports.
  let Ok(parsed) = parse_module(sheet) else { return vec![] };
  let mut imports = Imports(vec![]);
  imports.visit_body(&parsed.syntax().body);
  let opening = format!("import {MODULE}");
  imports
    .0
    .into_iter()
    .filter(|at| !(at.start() == 0.into() && sheet[*at] == opening))
    .filter_map(|at| {
      let raised =
        Sand::new().run(&sheet[at], NamedValues::new(), &mut Nobody("an import")).err()?;
      let start = at.start().to_usize();
      let (line, column) = (sheet[..start].matches('\n').count() + 1, column(sheet, start));
      Some((line, format!("{IMPORT} (column {column}) {raised}")))
    })
    .collect()
}

/// Each await of the sheet in a function that is not async, by the line it stands on.
///
/// Python refuses such a word, but the checker of monty lets it pass, though it refuses an async for and an async
/// with there. The sandbox then refuses the word with the SyntaxError of python before it runs, which the model sees
/// only after its turn, so the gate refuses it first and names the fix. An await at the top of the sheet is the word's
/// own, which a word may hold.
fn unawaited(sheet: &str) -> Vec<(usize, String)> {
  let Ok(parsed) = parse_module(sheet) else { return vec![] };
  let mut awaits = Awaits { function: None, found: vec![] };
  awaits.visit_body(&parsed.syntax().body);
  let line = |at: usize| sheet[..at].matches('\n').count() + 1;
  awaits
    .found
    .into_iter()
    .map(|(at, name)| {
      let why = format!(
        "{AWAIT} (column {}) `await` stands in `def {name}`, which is not async: write `async def {name}`, and await each call of it",
        column(sheet, at)
      );
      (line(at), why)
    })
    .collect()
}

/// Each name of asyncio but `CancelledError` that the word reads, by the line it stands on.
///
/// A word runs in a call of the host, where the sandbox refuses each future of asyncio: `asyncio.sleep` raises a
/// RuntimeError, and `asyncio.gather` refuses an act with a TypeError, since an act is a name. The stubs of monty type
/// both as right, so the gate refuses each read of asyncio in the word first, and names the verbs that do its work.
fn unscheduled(sheet: &str) -> Vec<(usize, String)> {
  let Ok(parsed) = parse_module(sheet) else { return vec![] };
  let Some(body) = bodied(parsed.syntax()) else { return vec![] };
  let mut modules = Asyncio(HashSet::new());
  modules.visit_body(body);
  let mut reads = Scheduled { modules: &modules.0, found: vec![] };
  reads.visit_body(word_of(body));
  let line = |at: usize| sheet[..at].matches('\n').count() + 1;
  reads
    .found
    .into_iter()
    .map(|(at, name)| {
      let why = format!(
        "{ASYNCIO} (column {}) `asyncio.{name}` cannot run in a word, which runs in a call of the host: await wait(seconds) to pause, and await each act by itself",
        column(sheet, at)
      );
      (line(at), why)
    })
    .collect()
}

/// The names that the sheet binds to the module asyncio, at any depth.
struct Asyncio<'a>(HashSet<&'a str>);

impl<'a> StatementVisitor<'a> for Asyncio<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    if let Stmt::Import(one) = stmt {
      for alias in one.names.iter().filter(|alias| alias.name.as_str() == "asyncio") {
        self.0.insert(alias.asname.as_ref().unwrap_or(&alias.name).as_str());
      }
    }
    walk_stmt(self, stmt);
  }
}

/// The reads of asyncio in a word, but `CancelledError`, each by where it starts, with the name that it reads.
struct Scheduled<'a, 'b> {
  modules: &'b HashSet<&'a str>,
  found: Vec<(usize, &'a str)>,
}

impl<'a> Visitor<'a> for Scheduled<'a, '_> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    if let Stmt::ImportFrom(one) = stmt
      && one.module.as_ref().is_some_and(|module| module.as_str() == "asyncio")
    {
      self.found.extend(
        one
          .names
          .iter()
          .filter(|alias| alias.name.as_str() != "CancelledError")
          .map(|alias| (alias.start().to_usize(), alias.name.as_str())),
      );
    }
    visitor::walk_stmt(self, stmt);
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    if let Expr::Attribute(one) = expr
      && let Expr::Name(module) = &*one.value
      && self.modules.contains(module.id.as_str())
      && one.attr.as_str() != "CancelledError"
    {
      self.found.push((expr.start().to_usize(), one.attr.as_str()));
    }
    visitor::walk_expr(self, expr);
  }
}

/// The awaits of a sheet that stand in a function that is not async: the name of the innermost function while it
/// is not async, and each await found, by where it starts, with the name of its function.
struct Awaits<'a> {
  function: Option<&'a str>,
  found: Vec<(usize, &'a str)>,
}

impl<'a> Visitor<'a> for Awaits<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(def) => {
        // The decorators, the defaults and the annotations run in the scope around the function.
        for decorator in &def.decorator_list {
          self.visit_decorator(decorator);
        }
        self.visit_parameters(&def.parameters);
        if let Some(returns) = &def.returns {
          self.visit_annotation(returns);
        }
        let around =
          std::mem::replace(&mut self.function, (!def.is_async).then_some(def.name.as_str()));
        self.visit_body(&def.body);
        self.function = around;
      }
      _ => visitor::walk_stmt(self, stmt),
    }
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    if let (Expr::Await(_), Some(name)) = (expr, self.function) {
      self.found.push((expr.start().to_usize(), name));
    }
    visitor::walk_expr(self, expr);
  }
}

/// Each name that the word binds and that the transcript binds, by the line it stands on.
///
/// The transcript binds the name of each act of the chain, which the sheet binds as `ACTS`, and each name of a value
/// that a tell of that act binds, as `bash12_stdout`: the name of the act, an underscore and a word. A word that binds
/// one says what no act gave, as the output of a command that did not run, and a model reads it as true, so the gate
/// refuses it, also for an act that the chain did not tell yet. The word is what stands after the last try of a word
/// of the program, and a name that it binds at any depth counts.
fn told(sheet: &str) -> Vec<(usize, String)> {
  let Ok(parsed) = parse_module(sheet) else { return vec![] };
  let Some(body) = bodied(parsed.syntax()) else { return vec![] };
  let acts = strings(body, ACTS);
  let word = word_of(body);
  let mut bound = Bound(vec![]);
  bound.visit_body(word);
  let line = |at: usize| sheet[..at].matches('\n').count() + 1;
  bound
    .0
    .into_iter()
    .filter(|(_, name)| {
      acts.iter().any(|act| {
        name.strip_prefix(act).is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
      })
    })
    .map(|(at, name)| {
      (
        line(at),
        format!(
          "{TOLD} (column {}) the transcript binds `{name}`: a word reads it and never binds it",
          column(sheet, at)
        ),
      )
    })
    .collect()
}

/// The names that a body binds at any depth, each by where it starts: each target of a statement or of an
/// expression, a def, a class, an import, a parameter, the name of an except and each capture of a pattern.
struct Bound<'a>(Vec<(usize, &'a str)>);

impl<'a> Visitor<'a> for Bound<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(one) => self.0.push((one.name.start().to_usize(), one.name.as_str())),
      Stmt::ClassDef(one) => self.0.push((one.name.start().to_usize(), one.name.as_str())),
      Stmt::Import(_) | Stmt::ImportFrom(_) => {
        self.0.extend(binds(stmt).into_iter().map(|name| (stmt.start().to_usize(), name)));
      }
      _ => {}
    }
    visitor::walk_stmt(self, stmt);
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    let stored = match expr {
      Expr::Name(one) if matches!(one.ctx, ExprContext::Store) => Some(one),
      _ => None,
    };
    self.0.extend(stored.map(|one| (one.start().to_usize(), one.id.as_str())));
    visitor::walk_expr(self, expr);
  }

  fn visit_parameter(&mut self, parameter: &'a Parameter) {
    self.0.push((parameter.name.start().to_usize(), parameter.name.as_str()));
    visitor::walk_parameter(self, parameter);
  }

  fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
    let ExceptHandler::ExceptHandler(one) = handler;
    self.0.extend(one.name.iter().map(|name| (name.start().to_usize(), name.as_str())));
    visitor::walk_except_handler(self, handler);
  }

  fn visit_pattern(&mut self, pattern: &'a Pattern) {
    let name = match pattern {
      Pattern::MatchAs(one) => one.name.as_ref(),
      Pattern::MatchStar(one) => one.name.as_ref(),
      Pattern::MatchMapping(one) => one.rest.as_ref(),
      _ => None,
    };
    self.0.extend(name.map(|name| (name.start().to_usize(), name.as_str())));
    visitor::walk_pattern(self, pattern);
  }
}

/// The imports of a body and of every body beneath it, each by where it stands.
struct Imports(Vec<TextRange>);

impl StatementVisitor<'_> for Imports {
  fn visit_stmt(&mut self, stmt: &Stmt) {
    match stmt {
      Stmt::Import(_) | Stmt::ImportFrom(_) => self.0.push(stmt.range()),
      _ => walk_stmt(self, stmt),
    }
  }
}

#[cfg(test)]
mod tests {
  use monty_types::NamedValues;

  use super::*;
  use crate::{
    SHEET as SOURCE,
    sand::{Nobody, Sand},
    value::{Object, entry},
  };

  /// The sheet of one word after a program, and how many lines stand above the word, as `furb.sheet` writes it in
  /// the sandbox from the module of the engine, made as a life makes it.
  fn sheet(program: &[&str], word: &str) -> (String, usize) {
    acted(program, &[], word)
  }

  /// The sheet of one word after a program, on a chain whose acts have the names it is given. Each word of the
  /// program is of the rung `rung<n>`, its place in the program from 1.
  fn acted(program: &[&str], acts: &[&str], word: &str) -> (String, usize) {
    let code = "__engine = {}\nexec(__source, __engine)\n__sheet = {}\nexec(__sheet_source, __sheet)\n__program = {f'rung{n}': one for n, one in enumerate(__words, 1)}\n__sheet['sheet'](__engine, __program, __word, __acts)";
    let mut named = NamedValues::new();
    named.push("__source", Object::string(ENGINE));
    named.push("__sheet_source", Object::string(SOURCE));
    named.push("__words", Object::list(program.iter().map(|one| Object::string(*one))));
    named.push("__word", Object::string(word));
    named.push("__acts", Object::list(acts.iter().map(|one| Object::string(*one))));
    let got =
      Sand::new().run(code, named, &mut Nobody("the sheet")).expect("furb.sheet writes the sheet");
    let got = got.as_ref();
    let text = entry(&got, 0).and_then(|one| one.as_str()).expect("a sheet is a text").to_owned();
    let above =
      entry(&got, 1).and_then(|one| one.as_int()).expect("a sheet counts the lines above the word");
    (text, usize::try_from(above).expect("a count of lines is no negative number"))
  }

  fn found(text: &str) -> Vec<(usize, String)> {
    checked(text).expect("the gate reads the sheet")
  }

  /// What the checker found below the word, each by its line in the word.
  fn said(program: &[&str], word: &str) -> Vec<(usize, String)> {
    let (text, above) = sheet(program, word);
    found(&text)
      .into_iter()
      .filter(|(line, _)| *line > above)
      .map(|(line, why)| (line - above, why))
      .collect()
  }

  #[test]
  fn the_sheet_imports_the_engine_under_the_name_the_gate_gives_it() {
    assert!(sheet(&[], "close(1)").0.starts_with(&format!("import {MODULE}\n")));
  }

  #[test]
  fn the_names_of_the_engine_bound_from_its_module_give_no_finding_of_their_own() {
    let (text, above) = sheet(&["k = 1", "x = read('a')"], "close(k)");
    assert_eq!(
      found(&text),
      Vec::<(usize, String)>::new(),
      "the engine reads clean on the typeshed of the sandbox"
    );
    assert_eq!(text.lines().count(), above + 1);
  }

  #[test]
  fn an_import_the_sandbox_cannot_run_is_a_finding_of_its_line() {
    // The typeshed of monty holds `types` for the annotations of `typing`, and the sandbox runs no `types`.
    let found = said(&["k = 1"], "k = 2\nimport types\nclose(types.SimpleNamespace(k=k))");
    assert_eq!(
      found,
      vec![(
        2,
        "error[unresolved-import] (column 3) ModuleNotFoundError: No module named 'types'"
          .to_owned()
      )]
    );
  }

  #[test]
  fn a_name_the_module_of_the_sandbox_lacks_is_a_finding_of_its_import() {
    let found = said(&[], "from typing import NoSuchName\nclose(1)");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, 1);
    assert!(
      found[0].1.starts_with("error[unresolved-import] (column 3) ImportError: "),
      "{found:?}"
    );
  }

  #[test]
  fn an_import_the_sandbox_runs_is_no_finding_though_the_typeshed_lacks_it() {
    assert_eq!(said(&[], "import ast\nclose(ast.PyCF_ALLOW_TOP_LEVEL_AWAIT)"), vec![]);
  }

  #[test]
  fn a_word_that_imports_the_engine_by_the_name_of_the_checker_is_refused() {
    let found = said(&[], &format!("import {MODULE}\nclose(1)"));
    assert_eq!(
      found,
      vec![(
        1,
        format!(
          "error[unresolved-import] (column 3) ModuleNotFoundError: No module named '{MODULE}'"
        )
      )]
    );
  }

  #[test]
  fn a_read_of_asyncio_in_the_word_is_a_finding_of_its_line_but_its_cancelled_error() {
    let found = said(
      &[],
      "import asyncio as aio\nfrom asyncio import gather, CancelledError\ntry:\n  await aio.sleep(1)\nexcept aio.CancelledError:\n  pass",
    );
    let why = |column: usize, name: &str| {
      format!(
        "{ASYNCIO} (column {column}) `asyncio.{name}` cannot run in a word, which runs in a call of the host: await wait(seconds) to pause, and await each act by itself"
      )
    };
    assert_eq!(found, vec![(2, why(23, "gather")), (4, why(11, "sleep"))]);
  }

  #[test]
  fn an_await_in_a_function_that_is_not_async_is_a_finding_of_its_line() {
    let found = said(&[], "def fix():\n  got = await wait(0)\n  return got\nfix()");
    let why = "`await` stands in `def fix`, which is not async: write `async def fix`, and await each call of it";
    assert_eq!(found, vec![(2, format!("{AWAIT} (column 11) {why}"))]);
  }

  #[test]
  fn an_await_in_a_function_that_is_not_async_in_an_async_one_is_a_finding() {
    let found =
      said(&[], "async def outer():\n  def inner():\n    return await wait(0)\n  return inner");
    assert_eq!(found.iter().map(|(line, _)| *line).collect::<Vec<_>>(), vec![3], "{found:?}");
    assert!(found[0].1.contains("`def inner`"), "{found:?}");
  }

  #[test]
  fn an_await_in_an_async_function_or_at_the_top_of_the_word_is_no_finding() {
    let word = "async def fix():\n  return await wait(0)\nawait fix()\nawait wait(0)\nclose(1)";
    assert_eq!(said(&[], word), vec![]);
  }

  #[test]
  fn a_name_bound_again_has_the_type_of_its_last_binding() {
    assert_eq!(
      said(
        &["x = 1", "def x() -> int:\n  return 1"],
        "y: int = x()\ndef z() -> int:\n  return x()"
      ),
      vec![]
    );
    let found = said(&["x = ''", "x = x + 'a'\nx = 1"], "y: str = x");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.contains("`Literal[1]` is not assignable to `str`"), "{found:?}");
  }

  #[test]
  fn a_name_a_nested_statement_binds_again_keeps_each_type() {
    let found = said(&["x = 1", "if chance() > 0.5:\n  def x() -> int:\n    return 1"], "x()");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.starts_with("error[call-non-callable]"), "{found:?}");
  }

  #[test]
  fn a_name_that_the_transcript_binds_is_a_finding_of_the_line_that_binds_it() {
    let word = "x = bash1\nbash1_stdout = 'made up'\nfor prompt2 in []:\n  pass\ndef f(bash12: int) -> int:\n  return bash12\nprompt21 = 2\nimport json as prompt2_value\nclose(x)";
    let (text, above) = acted(&["bash1 = 1"], &["bash1", "prompt2"], word);
    let got: Vec<_> = found(&text)
      .into_iter()
      .filter(|(line, _)| *line > above)
      .map(|(line, why)| (line - above, why))
      .collect();
    let told = |column: usize, name: &str| {
      format!(
        "{TOLD} (column {column}) the transcript binds `{name}`: a word reads it and never binds it"
      )
    };
    assert_eq!(
      got,
      vec![(2, told(3, "bash1_stdout")), (3, told(7, "prompt2")), (8, told(3, "prompt2_value"))]
    );
  }

  #[test]
  fn a_value_of_another_type_for_a_name_that_a_rung_declares_names_that_rung() {
    let program = ["def pr() -> int:\n  return 1", "import json\nmerged: set[int] = set()\nk = 1"];
    let word = "pr = 'a'\nfor merged in [[1]]:\n  pass\nk = 'b'\ndef f() -> int:\n  json = 1\n  return json\nclose(pr)";
    let (text, above) = sheet(&program, word);
    let got: Vec<_> = found(&text)
      .into_iter()
      .filter(|(line, _)| *line > above)
      .map(|(line, why)| (line - above, why))
      .collect();
    assert_eq!(got.iter().map(|(line, _)| *line).collect::<Vec<_>>(), vec![1, 2], "{got:?}");
    assert!(got[0].1.starts_with(ASSIGN), "{got:?}");
    assert!(
      got[0].1.ends_with(
        "; the word of rung1 declares `pr` with that type, so give this value another name"
      ),
      "{got:?}"
    );
    assert!(
      got[1].1.ends_with(
        "; the word of rung2 declares `merged` with that type, so give this value another name"
      ),
      "{got:?}"
    );
    let (text, above) = sheet(&program, "pr = 'a'");
    let unnamed = found(&text.replace("__words = ('rung1', 'rung2')", "__words = ()"));
    let tail = "; a word of the program of this chain declares `pr` with that type, so give this value another name";
    assert_eq!(
      unnamed.iter().map(|(line, _)| *line).collect::<Vec<_>>(),
      vec![above + 1],
      "{unnamed:?}"
    );
    assert!(unnamed[0].1.ends_with(tail), "{unnamed:?}");
    assert_eq!(said(&program, "pr: str = 'a'\nclose(pr)"), vec![]);
  }

  /// A name that a tell binds to a value of lists, `depth` levels deep, as monty writes it.
  fn lists(depth: usize) -> String {
    format!("#bash1 exited 0\nbash1_value = {}{}", "[".repeat(depth), "]".repeat(depth))
  }

  /// A literal `depth` levels deep, each level a list that holds an int, the level below, and a dict, with an empty
  /// list at the bottom, which the checker reads the slowest of the shapes that the gate measured.
  fn mixed(depth: usize) -> String {
    (1..depth).fold("[]".to_owned(), |below, _| format!("[1, {below}, {{'k': (1, 'a')}}]"))
  }

  #[test]
  fn a_literal_past_the_bound_is_cut_whole_to_an_empty_one_that_keeps_its_lines() {
    let (open, close) = ("[".repeat(DEEP), "]".repeat(DEEP));
    assert_eq!(shallowed(&format!("x = {open}{{1: [\n2]}}{close}\ny = 1")), "x = [\n]\ny = 1");
    let deep = mixed(DEEP + 1);
    let items = &deep[1..deep.len() - 1];
    let word = format!("x = ({deep},)\ny = {{1: {deep}}}\nz = {{{items}}}");
    assert_eq!(shallowed(&word), "x = tuple([])\ny = {}\nz = set()");
  }

  #[test]
  fn a_literal_within_the_bound_a_call_and_a_subscript_stay_as_they_stand() {
    let deep = "f(".repeat(DEEP + 1) + &")".repeat(DEEP + 1);
    let word = format!(
      "{}\nx = {{'a': [(1, {{2}})]}}\n{deep}\ny = x{}\nz = f'{{x}}'",
      lists(DEEP),
      "[0]".repeat(DEEP + 1)
    );
    assert_eq!(shallowed(&word), word);
  }

  #[test]
  fn a_value_that_a_tell_wrote_as_a_deep_literal_stalls_no_word() {
    let start = std::time::Instant::now();
    for deep in [
      lists(990),
      format!("bash1_value = {}", mixed(100)),
      format!("bash1_value = {}", mixed(DEEP)),
    ] {
      assert_eq!(said(&[&deep], "y: int = len(bash1_value)\nclose(bash1_value[0])"), vec![]);
    }
    let took = start.elapsed();
    assert!(took.as_secs() < 3, "{took:?}");
  }

  #[test]
  fn a_word_reads_a_value_that_was_cut_as_a_collection_of_its_kind() {
    let deep = mixed(DEEP + 1);
    let program = format!("a = ({deep},)\nb = {{'k': {deep}}}\nc = {deep}");
    assert_eq!(said(&[&program], "x = a[3][0]\ny = b['j'][1]\nz: int = len(c[5])"), vec![]);
    let found = said(&[&program], "w: str = len(c)");
    assert_eq!(found.len(), 1, "{found:?}");
  }

  /// A word of the program of a life of many hours, which binds the same names in a loop and under an if, as a model
  /// writes a word again and again.
  fn looped(n: usize) -> String {
    format!(
      "for one in read('a{n}').lines:\n  if one.startswith('x'):\n    got = bash(one)\n    pr = {{'labels': [one], 'n': ['{n}']}}\nlabel = pr['labels'][0] if pr else ''\ndebug(t'{{label=}} {{got=}} {{one=}}')"
    )
  }

  #[test]
  fn a_name_that_each_word_binds_in_a_loop_stalls_no_word() {
    let program: Vec<String> = (0..800).map(looped).collect();
    let program: Vec<&str> = program.iter().map(String::as_str).collect();
    let (text, _) = sheet(&program, "close(one + label)");
    let start = std::time::Instant::now();
    let got = found(&text);
    let took = start.elapsed();
    assert_eq!(got, vec![]);
    assert!(took.as_secs() < 3, "{took:?}");
  }

  #[test]
  fn a_name_that_a_nested_statement_binds_again_in_each_word_keeps_each_type() {
    let program = [
      "x = 1",
      "for x in ['a']:\n  pass",
      "if chance() > 0.5:\n  x = b'a'",
      "for x in [1.5]:\n  pass",
    ];
    let found = said(&program, "y: None = x");
    assert_eq!(found.len(), 1, "{found:?}");
    for kind in ["1", "\"a\"", "b\"a\"", "float"] {
      assert!(found[0].1.contains(kind), "{found:?}");
    }
  }

  #[test]
  fn a_name_keeps_its_type_and_its_declaration_across_the_modules_of_the_program() {
    let mut program = vec![
      "x = 1".to_owned(),
      "def pr() -> int:\n  return 1".to_owned(),
      "merged: set[int] = set()".to_owned(),
    ];
    program.extend((0..2 * CHUNK).map(|n| format!("k = {n}")));
    program.push("for x in ['a']:\n  pass".to_owned());
    let program: Vec<&str> = program.iter().map(String::as_str).collect();
    let found = said(&program, "y: None = x\npr = 'a'\nz: None = merged\nw: None = k");
    let lines: Vec<usize> = found.iter().map(|(line, _)| *line).collect();
    assert_eq!(lines, vec![1, 2, 3, 4], "{found:?}");
    assert!(found[0].1.contains("Literal[1, \"a\"]"), "{found:?}");
    assert!(
      found[1].1.ends_with(
        "the word of rung2 declares `pr` with that type, so give this value another name"
      ),
      "{found:?}"
    );
    assert!(found[2].1.contains("set[int]"), "{found:?}");
    assert!(found[3].1.contains(&format!("Literal[{}]", 2 * CHUNK - 1)), "{found:?}");
  }

  #[test]
  fn a_gate_after_one_more_word_reads_the_words_after_the_last_module_alone() {
    let program: Vec<String> = (0..2000).map(looped).collect();
    let program: Vec<&str> = program.iter().map(String::as_str).collect();
    let (text, _) = sheet(&program[..1999], "close(one + label)");
    let start = std::time::Instant::now();
    found(&text);
    let cold = start.elapsed();
    let (text, _) = sheet(&program, "close(one + label)");
    let start = std::time::Instant::now();
    found(&text);
    let warm = start.elapsed();
    assert!(warm * 4 < cold, "cold {cold:?}, warm {warm:?}");
  }

  #[test]
  fn a_warning_refuses_no_word() {
    // A name that may be unbound is a warning of the checker, and a word that reads it is no word the gate refuses.
    assert_eq!(said(&[], "if chance() > 0.5:\n  maybe = 1\nclose(maybe)"), vec![]);
  }
}
