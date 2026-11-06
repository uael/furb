//! The gate: the type checker of monty, reading a sheet.
//!
//! The contract says the word of a rung runs only if the gate accepts it, that the gate reads the
//! word after the program of its chain, and that a response which is not python is a finding like
//! any other. Nothing of that is a policy of a host: it is the contract, and the crate carries it.
//!
//! The sheet the word stands on is `furb.sheet`'s, written the same way by every Kernel, in this
//! sandbox and in the python package alike: every name of the engine, bound from the engine as a
//! module, then the program, then the word. So this reads a text and says what it found on it, each
//! finding by its line. The checker reads against the typeshed of monty, which describes what the
//! sandbox runs but for its modules: it holds stubs that its other stubs import, `types` among
//! them, which the sandbox does not run, and it lacks some modules that the sandbox runs, `ast`
//! among them. So the sandbox alone says which import runs: each import of the sheet runs in a
//! sandbox of its own, and one that raises is refused before the word runs. A word that binds a
//! name that the transcript binds, which the sheet names, is refused too. A value of another type
//! for a name that a rung of the program declares, which a def, a class, an import or a name with a
//! type does, names that rung, as the sheet names it, so a model reads where the type of the name
//! comes from. The errors alone count, since a warning refuses no word.
//!
//! The checker infers a file again whole when its text changes, and the program grows by a word at
//! each reading, so the gate lays each run of words of the program out as a module of its own,
//! which the checker reads once, and only the words after the last full module stand with the word:
//! a reading costs about the new words, and not the whole program, which held a life still for
//! minutes on a program of hours.
//!
//! `check` reads a sheet: it parses the text that the finders read once, and gives the text to the
//! checker of its thread. `layout` lays the program out as modules, `shallow` cuts each literal
//! that the checker could not read in time, and `findings` holds each finding and each rule that
//! the gate checks itself.

mod check;
mod findings;
mod layout;
mod shallow;

pub use check::{checked, checked_on};
pub use findings::{Finding, Rule};
use ruff_python_ast::{ExceptHandler, Expr, ModModule, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

/// The key of a config that turns the gate off when it is false, as `{"gate": false}`.
pub const KEY: &str = "gate";

/// Whether the gate reads each word of a life when no config names [`KEY`].
pub const ON: bool = true;

/// Whether the gate reads each word of a life, as the configs of the user and of the project say
/// it through [`crate::world::config::Layers`], a later layer winning: true unless the key [`KEY`]
/// says false. Configs that do not read, as with a value of the key that is no bool, leave it on.
/// With the gate
/// off, its ear answers each word with no finding, so the word runs one statement at a time and
/// stops at the first that raises.
pub fn configured(user: &std::path::Path, project: &std::path::Path) -> bool {
  let layers = crate::world::config::Layers::new(project, Some(user.to_path_buf()));
  layers.values().ok().and_then(|values| values.get(KEY)?.as_bool()).unwrap_or(ON)
}

/// The name the checker holds the engine under, which is `MODULE` of `furb.sheet`: the sheet alone
/// imports it.
const MODULE: &str = "__engine__";

/// The name the sheet binds to the names of the acts of the chain, which is `ACTS` of `furb.sheet`.
const ACTS: &str = "__acts";

/// The name the sheet binds to the names of the rungs of the program, which is `WORDS` of
/// `furb.sheet`.
const WORDS: &str = "__words";

/// The name the sheet binds to the shape of the prompt whose step the word answers, when that
/// prompt wants a value, which is `VALUED` of `furb.sheet`.
const VALUED: &str = "__valued";

/// A text of python parsed once, with the place where each of its lines starts, which each finder
/// of the gate reads in place of a parse and a count of lines of its own.
struct Parsed<'a> {
  text: &'a str,
  module: Option<ruff_python_parser::Parsed<ModModule>>,
  lines: Vec<usize>,
}

impl<'a> Parsed<'a> {
  fn new(text: &'a str) -> Self {
    let starts = text.match_indices('\n').map(|(at, _)| at + 1);
    let lines = std::iter::once(0).chain(starts).collect();
    Parsed { text, module: parse_module(text).ok(), lines }
  }

  /// The statements of the text, when it parses.
  fn body(&self) -> Option<&[Stmt]> {
    self.module.as_ref().map(|one| one.syntax().body.as_slice())
  }

  /// The body of the one async function of the sheet, which holds the program and the word.
  fn bodied(&self) -> Option<&[Stmt]> {
    self.body()?.iter().find_map(|stmt| match stmt {
      Stmt::FunctionDef(def) if def.is_async => Some(def.body.as_slice()),
      _ => None,
    })
  }

  /// The line of a place of the text, counted from 1.
  fn line(&self, at: usize) -> usize {
    self.lines.partition_point(|start| *start <= at)
  }

  /// The column of a place of the text, counted from 1 in characters, as the checker counts it.
  fn column(&self, at: usize) -> usize {
    self.text[self.lines[self.line(at) - 1]..at].chars().count() + 1
  }
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

/// Whether a try is the try of a word of the program, as `furb.sheet` writes it: `acting()` first,
/// and one except of `BaseException` that passes.
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

/// The names that a statement binds when it runs to its end, though it may read them first: an
/// assignment, an annotated assignment with a value, a def, a class, a type alias and an import. A
/// name that only a nested statement binds may keep the value it had, and an augmented assignment
/// reads the name it binds.
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

/// Each name of the engine that a statement at the top of a word binds or deletes, once and in its
/// order, by the rule that [`findings::contracted`] holds for a word: in a block, by a walrus, by a
/// capture or by a `global` too. A word may import a name that the engine imports under that name,
/// since it binds the same thing, and it binds the names that a chain binds as its own.
pub(crate) fn engine_bound(word: &str, engine: &str) -> Vec<String> {
  let (word, held) = (Parsed::new(word), Parsed::new(engine));
  let (Some(body), Some(held)) = (word.body(), held.body()) else { return vec![] };
  let names: Vec<&str> =
    held.iter().flat_map(binds).filter(|name| !findings::OWN.contains(name)).collect();
  let imports = findings::imported(engine);
  let mut found: Vec<String> = vec![];
  let mut bound = findings::bound(body);
  let defined = body.iter().filter(|stmt| matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)));
  bound.extend(
    defined
      .flat_map(|stmt| binds(stmt).into_iter().map(|name| (stmt.start().to_usize(), name, false))),
  );
  bound.sort_by_key(|(at, ..)| *at);
  for (_, name, import) in bound {
    let rebinds = names.contains(&name) && !(import && imports.iter().any(|one| one == name));
    if rebinds && !found.iter().any(|one| one == name) {
      found.push(name.to_owned());
    }
  }
  found
}

/// The names that the statements at the top of a body leave deleted: each name that a `del` deletes
/// and that no later statement at the top binds again.
fn deleted(body: &[Stmt]) -> Vec<&str> {
  let mut gone: Vec<&str> = vec![];
  for stmt in body {
    let bound = binds(stmt);
    gone.retain(|name| !bound.contains(name));
    if let Stmt::Delete(one) = stmt {
      for name in one.targets.iter().flat_map(named) {
        if !gone.contains(&name) {
          gone.push(name);
        }
      }
    }
  }
  gone
}

#[cfg(test)]
#[path = "gate.test.rs"]
mod tests;
