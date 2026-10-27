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

use std::cell::RefCell;

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
  /// once, so a reading of a sheet costs the program and the word alone, in every life after the first as in the
  /// first.
  static CHECKER: RefCell<Option<TypeChecker>> = const { RefCell::new(None) };
}

/// What the gate found on a sheet, in the order of its lines: each error of the checker by the line it stands on,
/// in the concise form it writes, and no warning, since a warning refuses no word; each import that the
/// sandbox cannot run, in place of what the checker says of the imports; each await in a function that is not
/// async; and each name that the word binds and that the transcript binds. An error on a value of another type for
/// a name that a rung of the program declares names that rung. Each is read on the sheet with each literal that
/// holds more than `DEEP` brackets cut, which the checker could not read in time.
pub fn checked(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
  let sheet = &shallowed(sheet);
  let mut found = declared(sheet, typed(sheet)?);
  found.extend(unrun(sheet));
  found.extend(unawaited(sheet));
  found.extend(told(sheet));
  found.sort_by_key(|(line, _)| *line);
  Ok(found)
}

/// What the checker found on a sheet. A checker that could not read the sheet has said nothing of the word, which
/// is not a finding, so it is a fault of the gate.
fn typed(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
  let (sheet, stands) = rebound(sheet);
  let config = TypeCheckingConfig { format: TypeCheckingFormat::Concise, color: false };
  let path = format!("{MODULE}/__init__.py");
  let engine = SourceFile::new(ENGINE, &path);
  CHECKER.with_borrow_mut(|held| {
    // The checker writes again every file it is given and reads again all that hangs on it, so the engine is
    // given to a new checker alone.
    let given = held.is_none().then_some(&engine);
    let checker = held.get_or_insert_with(TypeChecker::default);
    match checker.run(&SourceFile::new(&sheet, SHEET), given, config) {
      Ok(found) => Ok(
        found
          .map(|said| {
            let found = said.to_string().lines().filter_map(read).collect::<Vec<_>>();
            found
              .into_iter()
              .map(|(line, why)| (stands.get(line - 1).copied().unwrap_or(line), why))
              .collect()
          })
          .unwrap_or_default(),
      ),
      Err(why) => {
        // A checker that failed may not hold the engine, so the next reading makes a new one and gives it again.
        *held = None;
        Err(Fault::refused(format!("the gate could not read the sheet: {why}")))
      }
    }
  })
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
fn rebound(sheet: &str) -> (String, Vec<usize>) {
  let lines: Vec<&str> = sheet.split_inclusive('\n').collect();
  let mut deleted: Vec<(usize, String)> = vec![];
  if let Ok(parsed) = parse_module(sheet) {
    let body = bodied(parsed.syntax());
    let mut bound: Vec<&str> = vec![];
    for stmt in body.into_iter().flatten() {
      let names: Vec<&str> = match stmt {
        Stmt::Try(one) if worded(one) => one.body.iter().flat_map(binds).collect(),
        other => binds(other),
      };
      if matches!(stmt, Stmt::Try(one) if worded(one)) {
        let mut again: Vec<&str> = vec![];
        for name in names.iter().filter(|name| bound.contains(name)) {
          if !again.contains(name) {
            again.push(name);
          }
        }
        if !again.is_empty() {
          let line = sheet[..stmt.start().to_usize()].matches('\n').count();
          let indent: String = lines[line].chars().take_while(|one| *one == ' ').collect();
          deleted.push((line, format!("{indent}del {}\n", again.join(", "))));
        }
      }
      bound.extend(names);
    }
  }
  let (mut text, mut stands) = (String::with_capacity(sheet.len()), vec![]);
  let mut deletes = deleted.into_iter().peekable();
  for (at, line) in lines.iter().enumerate() {
    if let Some((_, del)) = deletes.next_if(|(before, _)| *before == at) {
      text.push_str(&del);
      stands.push(at + 1);
    }
    text.push_str(line);
    stands.push(at + 1);
  }
  (text, stands)
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
fn declared(sheet: &str, found: Vec<(usize, String)>) -> Vec<(usize, String)> {
  if !found.iter().any(|(_, why)| why.starts_with(ASSIGN)) {
    return found;
  }
  let Ok(parsed) = parse_module(sheet) else { return found };
  let Some(body) = bodied(parsed.syntax()) else { return found };
  let rungs = strings(body, WORDS);
  let mut declares: Vec<(&str, Option<&str>)> = vec![];
  let tries = body.iter().filter_map(|stmt| match stmt {
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
  let (_, rest) = rest.split_once(": ")?;
  if !rest.starts_with("error[") || rest.starts_with(IMPORT) {
    return None;
  }
  Some((line.trim().parse().ok()?, rest.trim().to_owned()))
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
      Some((sheet[..at.start().to_usize()].matches('\n').count() + 1, format!("{IMPORT} {raised}")))
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
        "{AWAIT} `await` stands in `def {name}`, which is not async: write `async def {name}`, and await each call of it"
      );
      (line(at), why)
    })
    .collect()
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
        format!("{TOLD} the transcript binds `{name}`: a word reads it and never binds it"),
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
      vec![(2, "error[unresolved-import] ModuleNotFoundError: No module named 'types'".to_owned())]
    );
  }

  #[test]
  fn a_name_the_module_of_the_sandbox_lacks_is_a_finding_of_its_import() {
    let found = said(&[], "from typing import NoSuchName\nclose(1)");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, 1);
    assert!(found[0].1.starts_with("error[unresolved-import] ImportError: "), "{found:?}");
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
        format!("error[unresolved-import] ModuleNotFoundError: No module named '{MODULE}'")
      )]
    );
  }

  #[test]
  fn an_await_in_a_function_that_is_not_async_is_a_finding_of_its_line() {
    let found = said(&[], "def fix():\n  got = await wait(0)\n  return got\nfix()");
    let why = "`await` stands in `def fix`, which is not async: write `async def fix`, and await each call of it";
    assert_eq!(found, vec![(2, format!("{AWAIT} {why}"))]);
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
    let told = |name: &str| {
      format!("{TOLD} the transcript binds `{name}`: a word reads it and never binds it")
    };
    assert_eq!(
      got,
      vec![(2, told("bash1_stdout")), (3, told("prompt2")), (8, told("prompt2_value"))]
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

  #[test]
  fn a_warning_refuses_no_word() {
    // A name that may be unbound is a warning of the checker, and a word that reads it is no word the gate refuses.
    assert_eq!(said(&[], "if chance() > 0.5:\n  maybe = 1\nclose(maybe)"), vec![]);
  }
}
