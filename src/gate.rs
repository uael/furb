//! The gate: the type checker of monty, reading a sheet.
//!
//! The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word after the
//! program of its chain, and that a response which is not python is a finding like any other. Nothing of that is
//! a policy of a host: it is the contract, and the crate carries it.
//!
//! The sheet the word stands on is `furb.sheet`'s, written the same way by every Kernel, in this sandbox and in
//! the python package alike: the engine itself, laid as the first rung of the chain, then the program, then the
//! word. So this reads a text and says what the checker found on it, each finding by its line, and what it reads
//! against is the typeshed of monty, which describes what the sandbox runs: a word that imports what monty does
//! not run is refused before it runs. The errors alone count, since a warning refuses no word.

use std::cell::RefCell;

use monty_type_checking::{SourceFile, TypeChecker};
use monty_types::{TypeCheckingConfig, TypeCheckingFormat};

use crate::value::Fault;

/// The sheet, which is the one file the checker reads.
const SHEET: &str = "sheet.py";

thread_local! {
  /// The one checker of this thread, which every life of the thread and the Kernel of a python host read with:
  /// it holds one reading of the typeshed, which costs once, so a reading of a sheet costs the sheet alone, in
  /// every life after the first as in the first.
  static CHECKER: RefCell<TypeChecker> = RefCell::new(TypeChecker::default());
}

/// What the checker found on a sheet: each error by the line it stands on, in the concise form it writes, and
/// no warning, since a warning refuses no word. A checker that could not read the sheet has said nothing of the
/// word, which is not a finding, so it is a fault of the gate.
pub fn checked(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
  let config = TypeCheckingConfig { format: TypeCheckingFormat::Concise, color: false };
  CHECKER.with_borrow_mut(|held| {
    let found = held
      .run(&SourceFile::new(sheet, SHEET), None, config)
      .map_err(|why| Fault::refused(format!("the gate could not read the sheet: {why}")))?;
    Ok(found.map(|said| said.to_string().lines().filter_map(read).collect()).unwrap_or_default())
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
  use super::*;
  use crate::ENGINE;

  /// The sheet of one word, after the program of no rung, as `furb.sheet` writes it: the engine first, whole.
  fn sheet(word: &str) -> (String, usize) {
    let mut above = String::from("async def __body():\n");
    for line in ENGINE.lines() {
      if line.trim().is_empty() {
        above.push('\n');
      } else {
        above.push_str("  ");
        above.push_str(line);
        above.push('\n');
      }
    }
    above.push_str(
      "  actor = \"\"\n  raised: BaseException | None = None\n  try:\n    acting()\n  except BaseException:\n    pass\n",
    );
    let lines = above.lines().count();
    (format!("{above}  {word}\n"), lines)
  }

  fn found(text: &str) -> Vec<(usize, String)> {
    checked(text).expect("the gate reads the sheet")
  }

  #[test]
  fn the_engine_laid_as_the_first_rung_gives_no_finding_of_its_own() {
    let (text, above) = sheet("close(1)");
    let above_word: Vec<_> = found(&text).into_iter().filter(|(line, _)| *line <= above).collect();
    assert_eq!(
      above_word,
      Vec::<(usize, String)>::new(),
      "the engine reads clean on the typeshed of the sandbox"
    );
  }

  #[test]
  fn a_sheet_the_gate_accepts_gives_no_finding() {
    let (text, _) = sheet("close(len(read('a.txt').lines))");
    assert_eq!(found(&text), Vec::<(usize, String)>::new());
  }

  #[test]
  fn every_name_the_engine_binds_is_a_name_a_word_may_say() {
    let (text, _) = sheet("close((re.compile('a'), CancelledError, Counter()))");
    assert_eq!(found(&text), Vec::<(usize, String)>::new());
  }

  #[test]
  fn the_gate_reads_one_sheet_after_another_the_same() {
    let (text, above) = sheet("close(nowhere)");
    assert_eq!(found(&text).len(), 1);
    assert_eq!(found(&sheet("close(1)").0), Vec::<(usize, String)>::new());
    assert_eq!(found(&text)[0].0, above + 1);
  }

  #[test]
  fn a_finding_says_the_line_of_the_sheet_it_stands_on() {
    let (text, above) = sheet("close(nowhere)");
    let found = found(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, above + 1);
    assert!(found[0].1.contains("unresolved-reference"), "{found:?}");
  }

  #[test]
  fn a_word_may_await_an_act_at_its_top_level() {
    let (text, _) = sheet("close((await bash('ls')).code)");
    assert_eq!(found(&text), Vec::<(usize, String)>::new());
  }

  #[test]
  fn a_warning_refuses_no_word() {
    // A name that may be unbound is a warning of the checker, and a word that reads it is no word the gate refuses.
    let (text, _) = sheet("if chance() > 0.5:\n    maybe = 1\n  close(maybe)");
    assert_eq!(found(&text), Vec::<(usize, String)>::new());
  }

  #[test]
  fn a_word_that_imports_what_the_sandbox_does_not_run_is_refused() {
    let (text, above) = sheet("import subprocess\n  close(subprocess.run)");
    let found = found(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, above + 1);
    assert!(found[0].1.contains("unresolved-import"), "{found:?}");
  }
}
