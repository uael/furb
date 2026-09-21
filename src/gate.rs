//! The gate: the type checker of monty, reading a sheet.
//!
//! The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word against
//! the rungs before it, and that a response which is not python is a finding like any other. Nothing of that is
//! a policy of a host: it is the contract, and the crate carries it.
//!
//! The sheet the word stands on is `furb.sheet`'s, written the same way by every Kernel, in this sandbox and in
//! the python package alike, so this reads a text and says what the checker found on it, each finding by its
//! line. What it reads against is `engine.pyi`, the contract, which the crate carries beside the engine and hands
//! to the checker as the module a sheet imports, and the typeshed of monty, which describes what the sandbox
//! runs: a word that imports what monty does not run is refused before it runs. The errors alone count, since a
//! warning refuses no word.

use std::cell::RefCell;

use monty_type_checking::{SourceFile, TypeChecker};
use monty_types::{TypeCheckingConfig, TypeCheckingFormat};
use ruff_python_ast::Stmt;
use ruff_python_parser::parse_module;

use crate::{CONTRACT, value::Fault};

/// The two names a chain binds of its own: the actor it stands on, and what the last rung raised.
const BOUND: [&str; 2] = ["actor", "raised"];
/// The sheet, which is the one file the checker reads.
const SHEET: &str = "sheet.py";
/// The contract, as the module the sheet imports it by.
const STUB: &str = "furb/engine.pyi";

thread_local! {
  /// The one checker of this thread, which every life of the thread and the Kernel of a python host read with:
  /// it holds one reading of the typeshed and of the contract, which cost once, so a reading of a sheet costs
  /// the sheet alone, in every life after the first as in the first.
  static CHECKER: RefCell<TypeChecker> = RefCell::new(TypeChecker::default());
}

/// What the checker found on a sheet: each error by the line it stands on, in the concise form it writes, and
/// no warning, since a warning refuses no word. A checker that could not read the sheet has said nothing of the
/// word, which is not a finding, so it is a fault of the gate.
pub fn checked(sheet: &str) -> Result<Vec<(usize, String)>, Fault> {
  let config = TypeCheckingConfig { format: TypeCheckingFormat::Concise, color: false };
  CHECKER.with_borrow_mut(|held| {
    let found = held
      .run(&SourceFile::new(sheet, SHEET), Some(&SourceFile::new(CONTRACT, STUB)), config)
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

/// Every name the engine defines at its top, and the two a chain binds of its own, which is what the globals of a
/// chain hold and what the sheet binds before it reads the ladder and the word.
///
/// The same walk as `furb.kernel.declared`, in rust, since the sandbox parses no python of its own.
pub fn named(source: &str) -> Vec<String> {
  let Ok(held) = parse_module(source) else {
    return BOUND.iter().map(|one| (*one).to_owned()).collect();
  };
  let mut said: Vec<String> = Vec::new();
  for one in &held.syntax().body {
    match one {
      Stmt::FunctionDef(held) => said.push(held.name.to_string()),
      Stmt::ClassDef(held) => said.push(held.name.to_string()),
      Stmt::TypeAlias(held) => {
        if let Some(name) = held.name.as_name_expr() {
          said.push(name.id.to_string());
        }
      }
      Stmt::AnnAssign(held) => {
        if let Some(name) = held.target.as_name_expr() {
          said.push(name.id.to_string());
        }
      }
      Stmt::Assign(held) => {
        for target in &held.targets {
          match target {
            one if one.is_name_expr() => {
              said.extend(one.as_name_expr().map(|held| held.id.to_string()));
            }
            one if one.is_tuple_expr() => {
              let held = one.as_tuple_expr().map(|held| &held.elts).into_iter().flatten();
              said
                .extend(held.filter_map(|one| one.as_name_expr().map(|held| held.id.to_string())));
            }
            _ => {}
          }
        }
      }
      _ => {}
    }
  }
  let mut seen = Vec::new();
  for one in said {
    if !seen.contains(&one) {
      seen.push(one);
    }
  }
  seen
}

#[cfg(test)]
mod tests {
  use std::fmt::Write as _;

  use super::*;
  use crate::ENGINE;

  /// The sheet of one word, on the ladder of no rung, as `furb.sheet` writes it.
  fn sheet(word: &str) -> (String, usize) {
    let mut head = String::from("import furb.engine as __engine\nasync def __body():\n");
    for name in named(ENGINE) {
      let _ = writeln!(head, "  {name} = __engine.{name}");
    }
    let above =
      format!("{head}  try:\n    __engine.lineage('')\n\n  except BaseException:\n    pass\n");
    let lines = above.lines().count();
    (format!("{above}  {word}\n"), lines)
  }

  fn found(text: &str) -> Vec<(usize, String)> {
    checked(text).expect("the gate reads the sheet")
  }

  #[test]
  fn the_names_of_the_sheet_are_the_names_the_globals_of_a_chain_hold() {
    let held = named(ENGINE);
    for one in ["send", "ask", "read", "bash", "prompt", "close", "Text", "Act"] {
      assert!(held.iter().any(|name| name == one), "{one} is a name a chain holds");
    }
    assert!(
      !held.iter().any(|name| name == "actor"),
      "the two names a chain binds of its own are the sheet's to add"
    );
    assert_eq!(held.iter().filter(|one| *one == "send").count(), 1, "a name stands once");
  }

  #[test]
  fn a_sheet_the_gate_accepts_gives_no_finding() {
    let (text, _) = sheet("close(len(read('a.txt').lines))");
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
