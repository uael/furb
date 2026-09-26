//! The gate: the type checker of monty, reading a sheet.
//!
//! The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word after the
//! program of its chain, and that a response which is not python is a finding like any other. Nothing of that is
//! a policy of a host: it is the contract, and the crate carries it.
//!
//! The sheet the word stands on is `furb.sheet`'s, written the same way by every Kernel, in this sandbox and in
//! the python package alike: every name of the engine, bound from the engine as a module, then the program, then
//! the word. So this reads a text and says what the checker found on it, each finding by its line, and what it
//! reads against is the typeshed of monty, which describes what the sandbox runs: a word that imports what monty
//! does not run is refused before it runs. The errors alone count, since a warning refuses no word.

use std::cell::RefCell;

use monty_type_checking::{SourceFile, TypeChecker};
use monty_types::{TypeCheckingConfig, TypeCheckingFormat};

use crate::{ENGINE, value::Fault};

/// The sheet, which is the one file the checker reads.
const SHEET: &str = "sheet.py";

/// The name the checker holds the engine under, which is `MODULE` of `furb.sheet`: the sheet alone imports it.
const MODULE: &str = "__engine__";

thread_local! {
  /// The one checker of this thread, which every life of the thread and the Kernel of a python host read with,
  /// and nothing before its first reading. It holds one reading of the typeshed and one of the engine, which cost
  /// once, so a reading of a sheet costs the program and the word alone, in every life after the first as in the
  /// first.
  static CHECKER: RefCell<Option<TypeChecker>> = const { RefCell::new(None) };
}

/// What the checker found on a sheet: each error by the line it stands on, in the concise form it writes, and
/// no warning, since a warning refuses no word. A checker that could not read the sheet has said nothing of the
/// word, which is not a finding, so it is a fault of the gate.
pub fn checked(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
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
/// error, is no finding of the sheet, so it is dropped rather than read as one.
fn read(said: &str) -> Option<(usize, String)> {
  let (_, rest) = said.split_once(':')?;
  let (line, rest) = rest.split_once(':')?;
  let (_, rest) = rest.split_once(": ")?;
  if !rest.starts_with("error[") {
    return None;
  }
  Some((line.trim().parse().ok()?, rest.trim().to_owned()))
}

#[cfg(test)]
mod tests {
  use monty_types::NamedValues;

  use super::*;
  use crate::{
    PREAMBLE, SHEET as SOURCE,
    sand::Sand,
    value::{Object, entry},
  };

  /// The sheet of one word after a program, and how many lines stand above the word, as `furb.sheet` writes it in
  /// the sandbox from the module of the engine, made as a life makes it.
  fn sheet(program: &[&str], word: &str) -> (String, usize) {
    let code = "__engine = loaded(__source, {**MODULE})\n__sheet = loaded(__sheet_source, {})\n__sheet['sheet'](__engine, __program, __word)";
    let mut named = NamedValues::new();
    named.push("__source", Object::string(ENGINE));
    named.push("__sheet_source", Object::string(SOURCE));
    named.push("__program", Object::list(program.iter().map(|one| Object::string(*one))));
    named.push("__word", Object::string(word));
    let mut host = |_, name: &str, _| -> Result<Object, Fault> {
      Err(Fault::refused(format!("the sheet calls no host, and it called {name}")))
    };
    let mut sand = Sand::new();
    sand.run(PREAMBLE, NamedValues::new(), &mut host).expect("the preamble runs");
    let got = sand.run(code, named, &mut host).expect("furb.sheet writes the sheet");
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
  fn a_warning_refuses_no_word() {
    // A name that may be unbound is a warning of the checker, and a word that reads it is no word the gate refuses.
    assert_eq!(said(&[], "if chance() > 0.5:\n  maybe = 1\nclose(maybe)"), vec![]);
  }
}
