//! The reading of a sheet: the checker of this thread on the modules of its program, and each
//! finding of the gate on what it read.

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use monty_type_checking::{SourceFile, TypeChecker};
use monty_types::{TypeCheckingConfig, TypeCheckingFormat};

use super::{
  MODULE, Parsed,
  findings::{Finding, contracted, declared, read, signed, told, unawaited, unrun, unscheduled},
  layout::{laid, program_lines, rebound, shortened},
  shallow::shallowed,
};
use crate::{ENGINE, value::Fault};

/// The sheet, which is the one file the checker reads.
const SHEET: &str = "sheet.py";

/// The most modules that the checker holds before it starts again, so the modules of programs that
/// changed or that a life no longer reads do not grow without end.
const MODULES: usize = 4096;

thread_local! {
  /// The one checker of this thread, which every life of the thread and the Kernel of a python host
  /// read with, and nothing before its first reading. It holds one reading of the typeshed and one
  /// of the engine, which cost once, and one of each module of a program, so a reading of a sheet
  /// costs the words after the last module and the word alone, in every life after the first as in
  /// the first.
  static CHECKER: RefCell<Option<Held>> = const { RefCell::new(None) };
}

/// The checker of a thread, the text of the engine that it read, and the path of each module of a
/// program that it read.
struct Held {
  checker: TypeChecker,
  engine: String,
  read: HashSet<String>,
}

/// What the gate found on a sheet, in the order of its lines: each error of the checker by the line
/// it stands on, in the concise form it writes, and no warning, since a warning refuses no word;
/// each import that the sandbox cannot run, in place of what the checker says of the imports; each
/// await in a function that is not async; and each name that the word binds and that the transcript
/// binds. An error on a value of another type for a name that a rung of the program declares names
/// that rung. Each is read on the sheet with each literal that holds more than `DEEP` brackets cut,
/// which the checker could not read in time.
pub fn checked(sheet: &str) -> Result<Vec<Finding>, Fault> {
  checked_on(ENGINE, sheet)
}

/// What the gate found on a sheet, as [`checked`] gives it, read on the text of an engine: this
/// build's own, or the one that the record of the life holds. The checker reads the engine again
/// when the sheet before stood on another.
pub fn checked_on(engine: &str, sheet: &str) -> Result<Vec<Finding>, Fault> {
  let lines: Vec<&str> = sheet.split_inclusive('\n').collect();
  let shaped = program_lines(&lines).and_then(|(first, tail)| {
    let short = shallowed(&shortened(&lines, first, tail));
    laid(&lines, first, tail, &short).map(|laid| (short, laid))
  });
  let Some((short, laid)) = shaped else {
    // A sheet that is not laid out as `furb.sheet` writes it is read whole.
    let sheet = shallowed(sheet);
    let parsed = Parsed::new(&sheet);
    let (text, stands) = rebound(&parsed);
    let found = declared(&parsed, typed(engine, &[], &text, &stands)?, None);
    return Ok(signed(gated(&parsed, found), &[&sheet], engine));
  };
  let parsed = Parsed::new(&short);
  let found =
    declared(&parsed, typed(engine, &laid.modules, &laid.main, &laid.lines)?, Some(&laid.declares));
  let mut sources: Vec<&str> = vec![&laid.main];
  sources.extend(laid.modules.iter().rev().map(|(_, module)| &**module));
  Ok(signed(gated(&parsed, found), &sources, engine))
}

/// What the checker found with each finding of a rule that the gate checks itself, in the order of
/// their lines.
fn gated(sheet: &Parsed<'_>, mut found: Vec<Finding>) -> Vec<Finding> {
  found.extend(unrun(sheet));
  found.extend(unawaited(sheet));
  found.extend(unscheduled(sheet));
  found.extend(told(sheet));
  found.extend(contracted(sheet));
  found.sort_by_key(|one| one.line);
  found
}

/// What the checker found on a text that stands on modules, each by the line of the sheet that its
/// line stands for. The checker reads each module that it did not read before, then the text. A
/// checker that could not read them has said nothing of the word, which is not a finding, so it is
/// a fault of the gate.
fn typed(
  engine: &str,
  modules: &[(String, Rc<str>)],
  text: &str,
  stands: &[usize],
) -> Result<Vec<Finding>, Fault> {
  let config = TypeCheckingConfig { format: TypeCheckingFormat::Concise, color: false };
  let path = format!("{MODULE}/__init__.py");
  let source = engine;
  let engine = SourceFile::new(source, &path);
  CHECKER.with_borrow_mut(|held| {
    if held.as_ref().is_some_and(|one| one.read.len() > MODULES || one.engine != source) {
      *held = None;
    }
    // The checker writes again every file it is given and reads again all that hangs on it, so the
    // engine is given to a new checker alone.
    let mut given = held.is_none().then_some(&engine);
    let one = held.get_or_insert_with(|| Held {
      checker: TypeChecker::default(),
      engine: source.to_owned(),
      read: HashSet::new(),
    });
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
              .map(|one| {
                let line = stands.get(one.line - 1).copied().unwrap_or(one.line);
                Finding { line, ..one }
              })
              .collect()
          })
          .unwrap_or_default()
      }),
    };
    got.map_err(|why| {
      // A checker that failed may not hold the engine, so the next reading makes a new one and
      // gives it again.
      *held = None;
      Fault::refused(format!("the gate could not read the sheet: {why}"))
    })
  })
}
