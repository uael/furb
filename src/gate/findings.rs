//! The findings of the gate: the rule that each breaks, what the checker says in its concise form,
//! and each rule that the gate checks itself, on the word and its imports.

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  fmt,
  sync::OnceLock,
};

use monty_types::NamedValues;
use ruff_python_ast::{
  ExceptHandler, Expr, ExprContext, Parameter, Pattern, Stmt,
  statement_visitor::{StatementVisitor, walk_stmt},
  visitor::{self, Visitor},
};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange};

use super::{ACTS, MODULE, Parsed, WORDS, binds, deleted, named, strings, worded};
use crate::{
  ENGINE,
  sand::{Nobody, Sand},
};

/// The rule that a finding of the gate breaks: a rule that the gate checks itself, or any other
/// that the checker names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
  /// An import that the sandbox cannot run, a rule that the checker names too.
  Import,
  /// An await in a function that is not async.
  Await,
  /// A read of asyncio in a word.
  Asyncio,
  /// A name that the transcript binds.
  Told,
  /// A value that is not of the type that its name is declared with.
  Assign,
  /// Any other rule of the checker, by its name.
  Checker(String),
}

impl Rule {
  /// The rule that the checker names.
  fn named(name: &str) -> Rule {
    match name {
      "unresolved-import" => Rule::Import,
      "invalid-assignment" => Rule::Assign,
      other => Rule::Checker(other.to_owned()),
    }
  }

  /// The name of the rule, as the model reads it between the brackets.
  fn name(&self) -> &str {
    match self {
      Rule::Import => "unresolved-import",
      Rule::Await => "await-outside-async",
      Rule::Asyncio => "asyncio-in-word",
      Rule::Told => "transcript-name",
      Rule::Assign => "invalid-assignment",
      Rule::Checker(name) => name,
    }
  }
}

impl fmt::Display for Rule {
  fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(out, "error[{}]", self.name())
  }
}

/// One finding of the gate: where it stands on the sheet, the rule it breaks and what it says.
/// Its text is what the model reads, `error[rule] (column N) why`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
  /// The line of the sheet that it stands on, counted from 1.
  pub line: usize,
  /// The column of that line, counted from 1 in characters.
  pub column: usize,
  /// The rule that it breaks.
  pub rule: Rule,
  /// What it says, with no rule and no column.
  pub why: String,
}

impl fmt::Display for Finding {
  fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(out, "{} (column {})", self.rule, self.column)?;
    if self.why.is_empty() { Ok(()) } else { write!(out, " {}", self.why) }
  }
}

/// The statements of the word, which stand after the last try of a word of the program.
fn word_of(body: &[Stmt]) -> &[Stmt] {
  body
    .iter()
    .rposition(|stmt| matches!(stmt, Stmt::Try(one) if worded(one)))
    .map_or(&body[body.len()..], |at| &body[at + 1..])
}

/// Each finding with the rung that declares the name that its line binds, when the finding is an
/// error on the type of that value and a word of the program declares the name.
///
/// A def, a class, an import and a name with a type declare that name, and the checker refuses a
/// later value of another type for it, though python binds it. The checker says only that the
/// value is not assignable to the type, and the model cannot see that an extension declared the
/// name, so it binds the name again. So the finding names the rung that declared the name last, of
/// the names that the sheet binds as `WORDS`, and says to give the value another name. A name that
/// the word binds in a def, a class, a lambda or a comprehension is its own, and names no rung.
pub(super) fn declared(
  sheet: &Parsed<'_>,
  found: Vec<Finding>,
  program: Option<&HashMap<String, usize>>,
) -> Vec<Finding> {
  if !found.iter().any(|one| one.rule == Rule::Assign) {
    return found;
  }
  let Some(body) = sheet.bodied() else { return found };
  let declares = declarers(body, program);
  let mut stored = Stored(vec![]);
  stored.visit_body(word_of(body));
  found
    .into_iter()
    .map(|it| {
      let by =
        stored.0.iter().filter(|(start, _)| sheet.line(*start) == it.line).find_map(|(_, name)| {
          declares.iter().find(|(held, _)| held == name).map(|(_, rung)| (*name, *rung))
        });
      match by {
        Some((name, rung)) if it.rule == Rule::Assign => {
          let who = rung.map_or("a word of the program of this chain".to_owned(), |one| {
            format!("the word of {one}")
          });
          let why = format!(
            "{}; {who} declares `{name}` with that type, so give this value another name",
            it.why
          );
          Finding { why, ..it }
        }
        _ => it,
      }
    })
    .collect()
}

/// Each name that a word of the program declares, with the rung of the word that declares it last:
/// from the layout of the program when the gate laid it out, else from each try of a word of the
/// sheet.
fn declarers<'a>(
  body: &'a [Stmt],
  program: Option<&'a HashMap<String, usize>>,
) -> Vec<(&'a str, Option<&'a str>)> {
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
    for name in deleted(&one.body) {
      declares.retain(|(held, _)| *held != name);
    }
  }
  declares
}

/// The names that a body binds in its own scope, each by where it starts: each target of a
/// statement or of an expression, but none in a def, a class, a lambda or a comprehension, which
/// bind in a scope of their own.
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
        self.0.push((one.start().to_usize(), one.id.as_str()));
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

/// One finding of the checker in its concise form, as the line it stands on and what it says.
///
/// The form is `path:line:column: severity[rule] why`, and a line that is not of that form, or that
/// is no error, is no finding of the sheet, so it is dropped rather than read as one. What the
/// checker says of an import is dropped too, since the sandbox says it.
pub(super) fn read(said: &str) -> Option<Finding> {
  let (_, rest) = said.split_once(':')?;
  let (line, rest) = rest.split_once(':')?;
  let (column, rest) = rest.split_once(": ")?;
  let (rule, why) = rest.trim().split_once(' ').unwrap_or((rest.trim(), ""));
  let rule = Rule::named(rule.strip_prefix("error[")?.strip_suffix(']')?);
  if rule == Rule::Import {
    return None;
  }
  Some(Finding {
    line: line.trim().parse().ok()?,
    column: column.trim().parse().ok()?,
    rule,
    why: why.to_owned(),
  })
}

/// The most characters of a signature that a finding quotes.
const SIGNED: usize = 300;

/// Each finding that names a function, as the checker writes it, "function `name`", with the
/// signature of that function: its name, its parameters and what it gives, as the last def of that
/// name in the sources reads, which stand from the nearest to the word to the farthest, and then in
/// the engine, whose defs the gate reads once. A model that called a function wrongly then reads in
/// the same finding how to call it.
pub(super) fn signed(found: Vec<Finding>, sources: &[&str]) -> Vec<Finding> {
  static ENGINE_SIGNED: OnceLock<HashMap<String, String>> = OnceLock::new();
  let engine = ENGINE_SIGNED.get_or_init(|| signatures(ENGINE));
  found
    .into_iter()
    .map(|one| {
      let name = one
        .why
        .split_once("function `")
        .and_then(|(_, rest)| rest.split_once('`'))
        .map(|(name, _)| name)
        .filter(|name| !name.contains('.'));
      let sign = name.and_then(|name| {
        let near = sources.iter().find_map(|source| signatures(source).remove(name));
        near.or_else(|| engine.get(name).cloned()).map(|sign| (name, sign))
      });
      let Some((name, sign)) = sign else { return one };
      let why = format!("{}; the signature of `{name}` is `{sign}`", one.why);
      Finding { why, ..one }
    })
    .collect()
}

/// The signature of the last def of each name in a source, on one line and cut to `SIGNED`
/// characters.
fn signatures(source: &str) -> HashMap<String, String> {
  let Ok(parsed) = parse_module(source) else { return HashMap::new() };
  let mut defs = Defs { source, found: HashMap::new() };
  defs.visit_body(&parsed.syntax().body);
  defs.found
}

/// The last def of each name in a body, at any depth, read from the name to the end of what it
/// gives.
struct Defs<'a> {
  source: &'a str,
  found: HashMap<String, String>,
}

impl<'a> StatementVisitor<'a> for Defs<'a> {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    if let Stmt::FunctionDef(def) = stmt {
      let end = def.returns.as_ref().map_or(def.parameters.end(), |one| one.end());
      let text = &self.source[def.name.start().to_usize()..end.to_usize()];
      let sign = text.split_whitespace().collect::<Vec<_>>().join(" ");
      self.found.insert(def.name.to_string(), sign.chars().take(SIGNED).collect());
    }
    walk_stmt(self, stmt);
  }
}

/// Each import of the sheet that the sandbox cannot run, by the line it stands on, with what the
/// sandbox raised.
///
/// An import runs the same in every sandbox, since monty runs no module that code defines, so a
/// sandbox of its own says what the sandbox of the word will, and the gate keeps what it said of
/// each import. Each import is read where it stands, in a function or under a test that is false
/// alike, as the checker reads it. The import of the engine that opens the sheet is the sheet's
/// own, and a word that imports the engine by that name is refused, since no chain holds a module
/// of that name.
pub(super) fn unrun(sheet: &Parsed<'_>) -> Vec<Finding> {
  // A sheet that does not parse is refused by what the checker says of it, whatever it imports.
  let Some(body) = sheet.body() else { return vec![] };
  let mut imports = Imports(vec![]);
  imports.visit_body(body);
  let opening = format!("import {MODULE}");
  imports
    .0
    .into_iter()
    .filter(|at| !(at.start() == 0.into() && sheet.text[*at] == opening))
    .filter_map(|at| {
      let why = raised(&sheet.text[at])?;
      let start = at.start().to_usize();
      let (line, column) = (sheet.line(start), sheet.column(start));
      Some(Finding { line, column, rule: Rule::Import, why })
    })
    .collect()
}

/// The most imports that the gate keeps what the sandbox said of, before it runs each again.
const IMPORTS: usize = 4096;

thread_local! {
  /// What the sandbox raised on each import that the gate ran, by its text, or nothing when the
  /// import ran.
  static RAISED: RefCell<HashMap<String, Option<String>>> = RefCell::new(HashMap::new());
}

/// What the sandbox raises on an import, from what it said before when the gate ran it before.
fn raised(import: &str) -> Option<String> {
  if let Some(said) = RAISED.with_borrow(|held| held.get(import).cloned()) {
    return said;
  }
  let ran = Sand::new().run(import, NamedValues::new(), &mut Nobody("an import"));
  let said = ran.err().map(|why| why.to_string());
  RAISED.with_borrow_mut(|held| {
    if held.len() >= IMPORTS {
      held.clear();
    }
    held.insert(import.to_owned(), said.clone());
  });
  said
}

/// Each await of the sheet in a function that is not async, by the line it stands on.
///
/// Python refuses such a word, but the checker of monty lets it pass, though it refuses an async
/// for and an async with there. The sandbox then refuses the word with the `SyntaxError` of python
/// before it runs, which the model sees only after its turn, so the gate refuses it first and names
/// the fix. An await at the top of the sheet is the word's own, which a word may hold.
pub(super) fn unawaited(sheet: &Parsed<'_>) -> Vec<Finding> {
  let Some(body) = sheet.body() else { return vec![] };
  let mut awaits = Awaits { function: None, found: vec![] };
  awaits.visit_body(body);
  awaits
    .found
    .into_iter()
    .map(|(at, name)| Finding {
      line: sheet.line(at),
      column: sheet.column(at),
      rule: Rule::Await,
      why: format!(
        "`await` stands in `def {name}`, which is not async: write `async def {name}`, and await each call of it"
      ),
    })
    .collect()
}

/// Each name of asyncio but `CancelledError` that the word reads, by the line it stands on.
///
/// A word runs in a call of the host, where the sandbox refuses each future of asyncio:
/// `asyncio.sleep` raises a `RuntimeError`, and `asyncio.gather` refuses an act with a `TypeError`,
/// since an act is a name. The stubs of monty type both as right, so the gate refuses each read of
/// asyncio in the word first, and names the verbs that do its work.
pub(super) fn unscheduled(sheet: &Parsed<'_>) -> Vec<Finding> {
  let Some(body) = sheet.bodied() else { return vec![] };
  let mut modules = Asyncio(HashSet::new());
  modules.visit_body(body);
  let mut reads = Scheduled { modules: &modules.0, found: vec![] };
  reads.visit_body(word_of(body));
  reads
    .found
    .into_iter()
    .map(|(at, name)| Finding {
      line: sheet.line(at),
      column: sheet.column(at),
      rule: Rule::Asyncio,
      why: format!(
        "`asyncio.{name}` cannot run in a word, which runs in a call of the host: await wait(seconds) to pause, and await each act by itself"
      ),
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

/// The reads of asyncio in a word, but `CancelledError`, each by where it starts, with the name
/// that it reads.
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

/// The awaits of a sheet that stand in a function that is not async: the name of the innermost
/// function while it is not async, and each await found, by where it starts, with the name of its
/// function.
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
/// The transcript binds the name of each act of the chain, which the sheet binds as `ACTS`, and
/// each name of a value that a tell of that act binds, as `bash12_stdout`: the name of the act, an
/// underscore and a word. A word that binds one says what no act gave, as the output of a command
/// that did not run, and a model reads it as true, so the gate refuses it, also for an act that the
/// chain did not tell yet. The word is what stands after the last try of a word of the program, and
/// a name that it binds at any depth counts.
pub(super) fn told(sheet: &Parsed<'_>) -> Vec<Finding> {
  let Some(body) = sheet.bodied() else { return vec![] };
  let acts = strings(body, ACTS);
  let word = word_of(body);
  let mut bound = Bound(vec![]);
  bound.visit_body(word);
  bound
    .0
    .into_iter()
    .filter(|(_, name)| {
      acts.iter().any(|act| {
        name.strip_prefix(act).is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
      })
    })
    .map(|(at, name)| Finding {
      line: sheet.line(at),
      column: sheet.column(at),
      rule: Rule::Told,
      why: format!("the transcript binds `{name}`: a word reads it and never binds it"),
    })
    .collect()
}

/// The names that a body binds at any depth, each by where it starts: each target of a statement or
/// of an expression, a def, a class, an import, a parameter, the name of an except and each capture
/// of a pattern.
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
