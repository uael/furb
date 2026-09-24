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
    let code = "__engine = module(__source, {**MODULE})\n__sheet = module(__sheet_source, {})\n__sheet['sheet'](__engine, __program, __word)";
    let mut named = NamedValues::new();
    named.push("__source", Object::string(ENGINE));
    named.push("__sheet_source", Object::string(SOURCE));
    named.push("__program", Object::list(program.iter().map(|one| Object::string(*one))));
    named.push("__word", Object::string(word));
    let mut host = |_, name: &str, _| -> Result<Object, Fault> {
      Err(Fault::refused(format!("the sheet calls no host, and it called {name}")))
    };
    let mut sand = Sand::default();
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
    let (text, above) = sheet(&["k = 1", "x = clock()"], "close(k)");
    assert_eq!(
      found(&text),
      Vec::<(usize, String)>::new(),
      "the engine reads clean on the typeshed of the sandbox"
    );
    assert_eq!(text.lines().count(), above + 1);
  }

  #[test]
  fn a_sheet_the_gate_accepts_gives_no_finding() {
    assert_eq!(said(&[], "close(len(turns()))"), vec![]);
  }

  #[test]
  fn every_name_the_engine_binds_is_a_name_a_word_may_say() {
    assert_eq!(said(&[], "close((re.compile('a'), CancelledError, Counter()))"), vec![]);
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
  fn a_word_may_await_an_act_at_its_top_level() {
    assert_eq!(said(&[], "close(await wait(0))"), vec![]);
  }

  #[test]
  fn a_warning_refuses_no_word() {
    // A name that may be unbound is a warning of the checker, and a word that reads it is no word the gate refuses.
    assert_eq!(said(&[], "if chance() > 0.5:\n  maybe = 1\nclose(maybe)"), vec![]);
  }

  #[test]
  fn a_builtin_the_sandbox_runs_is_accepted_and_one_it_does_not_run_is_refused() {
    for word in [
      "close(hasattr(1, 'a'))",
      "close(getattr(1, 'a', None))",
      "setattr(Refused, 'a', 1)",
      "close(open)",
    ] {
      assert_eq!(said(&[], word), vec![], "{word}");
    }
    for word in
      ["close(vars())", "close(dir())", "close(__import__('os'))", "close(exit)", "close(IOError)"]
    {
      let found = said(&[], word);
      assert_eq!(found.len(), 1, "{word}: {found:?}");
      assert_eq!(found[0].0, 1);
      assert!(found[0].1.contains("unresolved-reference"), "{found:?}");
    }
  }

  #[test]
  fn a_word_that_imports_what_the_sandbox_does_not_run_is_refused() {
    let found = said(&[], "import subprocess\nclose(subprocess.run)");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, 1);
    assert!(found[0].1.contains("unresolved-import"), "{found:?}");
  }

  #[test]
  fn a_word_that_imports_the_engine_by_its_package_is_refused() {
    for word in ["import furb\nclose(furb)", "from furb.engine import clock\nclose(clock)"] {
      let found = said(&[], word);
      assert_eq!(found.len(), 1, "{found:?}");
      assert!(found[0].1.contains("unresolved-import"), "{found:?}");
    }
  }

  #[test]
  fn a_word_binds_a_name_of_the_engine_again_to_any_value() {
    assert_eq!(said(&[], "clock = 1\nclose(clock)"), vec![]);
    assert_eq!(said(&[], "old = OPERATOR\nOPERATOR = old\nclose(len(OPERATOR) + 1)"), vec![]);
    assert_eq!(said(&[], "x = clock()\nclock = 1\nclose(x)"), vec![]);
  }

  #[test]
  fn a_word_reads_a_name_of_the_engine_as_the_program_bound_it_last() {
    let found = said(&["clock = 1"], "close(clock())");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.contains("call-non-callable"), "{found:?}");
  }

  #[test]
  fn a_word_may_use_the_async_forms_at_its_top_level() {
    let word = "async def g():\n  yield 1\nasync for x in g():\n  close([y async for y in g()])";
    assert_eq!(said(&[], word), vec![]);
  }

  #[test]
  fn the_word_of_every_builtin_passes_the_gate_after_the_words_before_it() {
    let words = crate::extension::words(&crate::extension::builtins());
    for (at, word) in words.iter().enumerate() {
      let before: Vec<&str> = words[..at].iter().map(String::as_str).collect();
      assert_eq!(said(&before, word), vec![], "the word of builtin {at}");
    }
    let all: Vec<&str> = words.iter().map(String::as_str).collect();
    assert_eq!(found(&sheet(&all, "close(1)").0), vec![]);
  }

  #[test]
  fn a_word_reads_a_name_an_extension_bound_in_the_program() {
    let words = crate::extension::words(&crate::extension::builtins());
    assert_eq!(said(&[&words[0]], "close(read('a').lines)"), vec![]);
    let found = said(&[], "close(read('a').lines)");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.contains("unresolved-reference"), "{found:?}");
  }

  #[test]
  fn each_word_of_the_program_stands_in_a_try_of_its_own() {
    let (text, _) = sheet(&["a = 1", "b = 2"], "close(a + b)");
    assert_eq!(text.matches("  try:\n    acting()\n").count(), 2);
    assert_eq!(said(&["raise ValueError('x')\na = 1", "b = 2"], "close(b)"), vec![]);
  }
}
