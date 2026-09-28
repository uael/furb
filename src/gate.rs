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
//! of its own, and one that raises is refused before the word runs. The errors alone count, since a warning refuses
//! no word.

use std::cell::RefCell;

use monty_type_checking::{SourceFile, TypeChecker};
use monty_types::{NamedValues, TypeCheckingConfig, TypeCheckingFormat};
use ruff_python_ast::{
  Stmt,
  statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange};

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

thread_local! {
  /// The one checker of this thread, which every life of the thread and the Kernel of a python host read with,
  /// and nothing before its first reading. It holds one reading of the typeshed and one of the engine, which cost
  /// once, so a reading of a sheet costs the program and the word alone, in every life after the first as in the
  /// first.
  static CHECKER: RefCell<Option<TypeChecker>> = const { RefCell::new(None) };
}

/// What the gate found on a sheet, in the order of its lines: each error of the checker by the line it stands on,
/// in the concise form it writes, and no warning, since a warning refuses no word; and each import that the
/// sandbox cannot run, in place of what the checker says of the imports.
pub fn checked(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
  let mut found = typed(sheet)?;
  found.extend(unrun(sheet));
  found.sort_by_key(|(line, _)| *line);
  Ok(found)
}

/// What the checker found on a sheet. A checker that could not read the sheet has said nothing of the word, which
/// is not a finding, so it is a fault of the gate.
fn typed(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
  let config = TypeCheckingConfig { format: TypeCheckingFormat::Concise, color: false };
  let path = format!("{MODULE}/__init__.py");
  let engine = SourceFile::new(ENGINE, &path);
  CHECKER.with_borrow_mut(|held| {
    // The checker writes again every file it is given and reads again all that hangs on it, so the engine is
    // given to a new checker alone.
    let given = held.is_none().then_some(&engine);
    let checker = held.get_or_insert_with(TypeChecker::default);
    match checker.run(&SourceFile::new(sheet, SHEET), given, config) {
      Ok(found) => Ok(
        found.map(|said| said.to_string().lines().filter_map(read).collect()).unwrap_or_default(),
      ),
      Err(why) => {
        // A checker that failed may not hold the engine, so the next reading makes a new one and gives it again.
        *held = None;
        Err(Fault::refused(format!("the gate could not read the sheet: {why}")))
      }
    }
  })
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
      let raised = ran(&sheet[at]).err()?;
      Some((sheet[..at.start().to_usize()].matches('\n').count() + 1, format!("{IMPORT} {raised}")))
    })
    .collect()
}

/// One import, run in a sandbox of its own, which calls no host.
fn ran(import: &str) -> Result<(), Fault> {
  Sand::new().run(import, NamedValues::new(), &mut Nobody("an import")).map(drop)
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
    let code = "__engine = {}\nexec(__source, __engine)\n__sheet = {}\nexec(__sheet_source, __sheet)\n__sheet['sheet'](__engine, __program, __word)";
    let mut named = NamedValues::new();
    named.push("__source", Object::string(ENGINE));
    named.push("__sheet_source", Object::string(SOURCE));
    named.push("__program", Object::list(program.iter().map(|one| Object::string(*one))));
    named.push("__word", Object::string(word));
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
    assert!(SOURCE.contains(&format!("MODULE = \"{MODULE}\"")));
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
  fn the_gate_reads_one_sheet_after_another_the_same() {
    let (text, _) = sheet(&[], "close(nowhere)");
    assert_eq!(found(&text).len(), 1);
    assert_eq!(said(&[], "close(1)"), vec![]);
    assert_eq!(found(&text).len(), 1);
  }

  #[test]
  fn a_finding_says_the_line_of_the_sheet_it_stands_on() {
    let (text, above) = sheet(&["k = 1"], "k = 2\nclose(nowhere)");
    let found = found(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, above + 2);
    assert!(found[0].1.contains("unresolved-reference"), "{found:?}");
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
  fn a_warning_refuses_no_word() {
    // A name that may be unbound is a warning of the checker, and a word that reads it is no word the gate refuses.
    assert_eq!(said(&[], "if chance() > 0.5:\n  maybe = 1\nclose(maybe)"), vec![]);
  }
}
