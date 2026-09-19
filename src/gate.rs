//! The gate: ty, as a library, reading the word of a rung before it runs.
//!
//! The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word against
//! the rungs before it and against the name of the shape it must give, and that a response which is not python is
//! a finding like any other. Nothing of that is a policy of a host: it is the contract, and the crate carries it.
//!
//! ty is a rust library, so the crate reads the word itself rather than asking a host to. What it reads against
//! is `engine.pyi`, the contract, which the crate carries beside the engine and hands to ty as the stub of the
//! module a word stands in.
//!
//! The word stands on a sheet of its own, which is what ty is given:
//!
//! ```text
//! import typing
//! import furb.engine as __engine
//! async def __body():
//!   send = __engine.send          # every name the globals of a chain hold, one to a line
//!   ...
//!   @typing.overload
//!   def close(value: int, id: typing.Literal[""] = "") -> None: ...
//!   ...
//!   try:
//!     __engine.lineage('')
//!     <the ladder of the chain>
//!   except BaseException:
//!     pass
//!   <the word>
//! ```
//!
//! Every line of it is there for a reason. The body is async, so a word may await at its top level. A name is
//! bound on a line of its own rather than imported, since a word may rebind a name and no word may rebind an
//! import. `close` is overloaded under the shape, so a close that carries the wrong value is a finding while a
//! close that names another act still takes anything. The ladder stands in a try, so a rung that raised or that
//! never ends leaves the word reachable. And the word and the ladder keep their own lines, so a finding is
//! counted back to the line the model wrote.

use std::{fmt::Write as _, sync::Arc};

use ruff_db::{
  Db as SourceDb,
  diagnostic::{Diagnostic, DiagnosticFormat, DisplayDiagnosticConfig, DisplayDiagnostics},
  file_revision::FileRevision,
  files::{File, FileRootKind, Files, system_path_to_file},
  system::{DbWithTestSystem, DbWithWritableSystem as _, SystemPath, SystemPathBuf, System, TestSystem},
  vendored::VendoredFileSystem,
};
use ruff_python_ast::{self as ast, PythonVersion, Stmt};
use ruff_python_parser::parse_module;
use salsa::Setter as _;
use ty_module_resolver::{Db as ModuleResolverDb, FallibleStrategy, SearchPathSettings};
use ty_python_core::{
  Db as PythonCoreDb, ProgramFile,
  platform::PythonPlatform,
  program::{Program, ProgramSettings},
};
use ty_python_semantic::{
  AnalysisSettings, Db, PythonVersionSource, PythonVersionWithSource, check_file_unwrap, default_lint_registry,
  lint::{LintRegistry, RuleSelection},
};

use crate::{CONTRACT, ENGINE, host::Gate};

/// The name the sheet holds the engine under, which no word may say.
const ALIAS: &str = "__engine";
/// The verb a word answers by, which the sheet binds under the shape the word must give.
const CLOSE: &str = "close";
/// The two names a chain binds of its own: the actor it stands on, and what the last rung raised.
const BOUND: [&str; 2] = ["actor", "raised"];
/// Where every file of the reading stands, since nothing of it is on a disk.
const ROOT: &str = "/";
/// The sheet the word stands on, which is the one file ty reads.
const SHEET: &str = "sheet.py";

/// The gate of the crate: ty, reading a word against the contract and the ladder of its chain.
///
/// One of these serves one life. It holds the reading of the contract, which is read once, and the names the
/// globals of a chain hold, which are read off the engine once, so a gate of a word costs the reading alone.
pub struct Ty {
  /// Where ty reads its files, which is a memory and no disk.
  db: Memory,
  /// The names the globals of a chain hold, which the sheet binds before it reads the ladder and the word.
  names: Vec<String>,
  /// The revision of the last write, so that two writes of one path are two writes to ty.
  revision: u128,
}

impl Default for Ty {
  fn default() -> Self {
    Self::new()
  }
}

impl Ty {
  /// A gate that reads a word against the contract the crate carries.
  #[must_use]
  pub fn new() -> Self {
    let mut held = Ty { db: Memory::default(), names: named(ENGINE), revision: 0 };
    held.wrote("furb/__init__.pyi", "");
    held.wrote("furb/engine.pyi", CONTRACT);
    held
  }

  /// One file into the memory ty reads, under a revision of its own.
  ///
  /// ty reads a file again when its revision moves, and a revision is made from the time a file was written, so
  /// two writes of one path inside one tick of the clock would be one write to ty. The revision is said here
  /// instead, which makes every write a write.
  fn wrote(&mut self, path: &str, source: &str) -> Option<File> {
    let at = SystemPathBuf::from(ROOT).join(path);
    self.db.write_file(&at, source).ok()?;
    self.revision = self.revision.wrapping_add(1);
    let mut held = Vec::new();
    let mut walk = Some(at.as_path());
    while let Some(one) = walk {
      if let Some(file) = self.db.files().try_system(&self.db, one) {
        held.push(file);
      }
      walk = one.parent();
    }
    for file in held {
      file.set_revision(&mut self.db).to(FileRevision::new(self.revision));
    }
    system_path_to_file(&self.db, &at).ok()
  }

  /// What ty found on the sheet, each finding as one line of the concise form it writes.
  fn found(&mut self, sheet: &str) -> Vec<String> {
    let Some(file) = self.wrote(SHEET, sheet) else {
      return vec!["the gate could not be given the word".to_owned()];
    };
    let held: Vec<Diagnostic> = check_file_unwrap(&self.db, self.db.program_file(file));
    if held.is_empty() {
      return Vec::new();
    }
    let config = DisplayDiagnosticConfig::new("furb").format(DiagnosticFormat::Concise).color(false);
    let mut said = String::new();
    let _ = write!(said, "{}", DisplayDiagnostics::new(&self.db, &config, &held));
    said.lines().filter(|one| !one.trim().is_empty()).map(str::to_owned).collect()
  }
}

impl Gate for Ty {
  fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String> {
    if word.contains(ALIAS) {
      return vec![format!("{ALIAS} is a name of the gate")];
    }
    let said = python(word);
    if !said.is_empty() {
      return said;
    }
    let (sheet, closing, above) = sheet(&self.names, ladder, word, shape);
    let mut said = Vec::new();
    for one in self.found(&sheet) {
      let Some((line, why)) = read(&one) else { continue };
      if closing.contains(&line) {
        return vec![format!("{shape} is no shape")];
      }
      if line > above {
        said.push(format!("line {}: {why}", line - above));
      }
    }
    said
  }
}

/// The word on a sheet of its own: the sheet, the lines the shape stands on, and how many lines stand above the
/// word, which every finding is counted back by.
///
/// The shape is the gate's own writing and no word of anybody, so a finding on those lines is a finding against
/// the shape itself and never against the word.
fn sheet(names: &[String], ladder: &[String], word: &str, shape: &str) -> (String, std::ops::Range<usize>, usize) {
  let shape = if shape.is_empty() || shape == "None" { "object" } else { shape };
  let mut head = format!("import typing\nimport furb.engine as {ALIAS}\nasync def __body():\n");
  for name in names.iter().filter(|one| *one != CLOSE) {
    let _ = writeln!(head, "  {name} = {ALIAS}.{name}");
  }
  let first = head.lines().count() + 1;
  let closing = format!(
    "  @typing.overload\n  def {CLOSE}(value: {shape}, id: typing.Literal[\"\"] = \"\") -> None: ...\n  \
     @typing.overload\n  def {CLOSE}(value: object, id: str) -> None: ...\n  \
     def {CLOSE}(value: object, id: str = \"\") -> None: ...\n"
  );
  let held = closing.lines().count();
  let above = format!(
    "{head}{closing}  try:\n    {ALIAS}.lineage('')\n{}  except BaseException:\n    pass\n",
    laid(&ladder.join("\n"), 4)
  );
  let lines = above.lines().count();
  (format!("{above}{}", laid(word, 2)), first..first + held, lines)
}

/// What the word is as python, before ty reads what it means.
///
/// The word of a rung runs as the body of a module, and the sheet stands it inside a function so that it may
/// await, which makes a return legal there where the engine would not take one. So the word is read as the body
/// it really is first, and what that reading finds is a finding of the gate like any other.
fn python(word: &str) -> Vec<String> {
  let held = match parse_module(word) {
    Ok(held) => held,
    Err(no) => return vec![format!("line {}: {no}", lined(word, no.location.start().into()))],
  };
  let mut said: Vec<String> = held
    .errors()
    .iter()
    .map(|no| format!("line {}: {no}", lined(word, no.location.start().into())))
    .collect();
  said.extend(
    returns(&held.syntax().body)
      .into_iter()
      .map(|at| format!("line {}: 'return' outside function", lined(word, at))),
  );
  said
}

/// The line a place in the text stands on, counted from one.
fn lined(text: &str, at: usize) -> usize {
  text.bytes().take(at.min(text.len())).filter(|one| *one == b'\n').count() + 1
}

/// Every return of a body that stands in no function of its own, which is no python at the top of a module.
fn returns(body: &[Stmt]) -> Vec<usize> {
  let mut said = Vec::new();
  for one in body {
    match one {
      Stmt::Return(held) => said.push(held.range.start().into()),
      Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
      Stmt::If(held) => {
        said.extend(returns(&held.body));
        for clause in &held.elif_else_clauses {
          said.extend(returns(&clause.body));
        }
      }
      Stmt::For(held) => {
        said.extend(returns(&held.body));
        said.extend(returns(&held.orelse));
      }
      Stmt::While(held) => {
        said.extend(returns(&held.body));
        said.extend(returns(&held.orelse));
      }
      Stmt::With(held) => said.extend(returns(&held.body)),
      Stmt::Match(held) => {
        for case in &held.cases {
          said.extend(returns(&case.body));
        }
      }
      Stmt::Try(held) => {
        said.extend(returns(&held.body));
        said.extend(returns(&held.orelse));
        said.extend(returns(&held.finalbody));
        for handler in &held.handlers {
          let ast::ExceptHandler::ExceptHandler(handler) = handler;
          said.extend(returns(&handler.body));
        }
      }
      _ => {}
    }
  }
  said
}

/// The text as it stands on the sheet, indented into the body, with every line where it was, so a finding keeps
/// the line it was found on.
fn laid(text: &str, depth: usize) -> String {
  let mut held = String::new();
  for line in text.split('\n') {
    if line.trim().is_empty() {
      held.push('\n');
    } else {
      let _ = writeln!(held, "{}{line}", " ".repeat(depth));
    }
  }
  held
}

/// One finding of ty in its concise form, as the line it stands on and what it says.
///
/// The form is `path:line:column: severity[rule] why`, and a line that is not of that form is no finding of the
/// word, so it is dropped rather than read as one.
fn read(said: &str) -> Option<(usize, String)> {
  let (_, rest) = said.split_once(':')?;
  let (line, rest) = rest.split_once(':')?;
  let (_, rest) = rest.split_once(": ")?;
  Some((line.trim().parse().ok()?, rest.trim().to_owned()))
}

/// Every name the engine defines at its top, which is what the globals of a chain hold of the engine.
///
/// A chain holds what the file defines and the two names it binds of its own, so the sheet binds the same, and a
/// word that reads a name no chain holds is a finding like any other.
fn named(source: &str) -> Vec<String> {
  let Ok(held) = parse_module(source) else { return BOUND.iter().map(|one| (*one).to_owned()).collect() };
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
            one if one.is_name_expr() => said.extend(one.as_name_expr().map(|held| held.id.to_string())),
            one if one.is_tuple_expr() => {
              let held = one.as_tuple_expr().map(|held| &held.elts).into_iter().flatten();
              said.extend(held.filter_map(|one| one.as_name_expr().map(|held| held.id.to_string())));
            }
            _ => {}
          }
        }
      }
      _ => {}
    }
  }
  said.extend(BOUND.iter().map(|one| (*one).to_owned()));
  let mut seen = Vec::new();
  for one in said {
    if !seen.contains(&one) {
      seen.push(one);
    }
  }
  seen
}

/// The memory ty reads its files from, which is no disk and holds one reading of the contract.
///
/// It is the database of ty, wired to the typeshed that ty carries. One of these belongs to one gate: salsa
/// gives one owner of a storage, so it is never cloned and never shared.
#[salsa::db]
struct Memory {
  /// The storage of salsa, which every reading is cached in.
  storage: salsa::Storage<Self>,
  /// The files of the reading.
  files: Files,
  /// The file system of the reading, which is a memory.
  system: TestSystem,
  /// The typeshed, as ty carries it.
  vendored: VendoredFileSystem,
  /// Which findings ty makes, which is every finding it knows.
  rules: Arc<RuleSelection>,
  /// How ty reads, which is how ty reads by default.
  analysis: Arc<AnalysisSettings>,
  /// The python the word is read for, which is the python that runs it.
  program: ProgramSettings,
}

impl std::fmt::Debug for Memory {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Memory").finish_non_exhaustive()
  }
}

impl Default for Memory {
  fn default() -> Self {
    let root = SystemPathBuf::from(ROOT);
    let vendored = ty_vendored::file_system().clone();
    let mut db = Memory {
      storage: salsa::Storage::new(None),
      system: TestSystem::default(),
      program: ProgramSettings::empty(&vendored),
      vendored,
      files: Files::default(),
      rules: Arc::new(RuleSelection::from_registry(default_lint_registry())),
      analysis: AnalysisSettings::default().into(),
    };
    db.files().try_add_root(&db, &root, FileRootKind::Project);
    let paths = SearchPathSettings::new(vec![root])
      .to_search_paths(db.system(), db.vendored(), &FallibleStrategy)
      .expect("the typeshed ty carries always resolves");
    paths.try_register_static_roots(&db);
    db.program = ProgramSettings {
      python_version: PythonVersionWithSource {
        version: PythonVersion::PY314,
        source: PythonVersionSource::default(),
      },
      python_platform: PythonPlatform::default(),
      search_paths: paths,
    };
    db
  }
}

/// What the reading needs of the memory to make its program once.
#[salsa::db]
trait ProgramDb: Db {
  /// The settings every file of this memory is read under.
  fn settings(&self) -> &ProgramSettings;
}

/// The program of the memory, made once and cached.
#[salsa::tracked(returns(copy))]
fn program(db: &dyn ProgramDb) -> Program<'_> {
  Program::from_settings(db, db.settings().clone())
}

impl DbWithTestSystem for Memory {
  fn test_system(&self) -> &TestSystem {
    &self.system
  }

  fn test_system_mut(&mut self) -> &mut TestSystem {
    &mut self.system
  }
}

#[salsa::db]
impl SourceDb for Memory {
  fn vendored(&self) -> &VendoredFileSystem {
    &self.vendored
  }

  fn system(&self) -> &dyn System {
    &self.system
  }

  fn files(&self) -> &Files {
    &self.files
  }
}

#[salsa::db]
impl PythonCoreDb for Memory {
  fn should_check_file(&self, file: File) -> bool {
    !file.path(self).is_vendored_path()
  }
}

#[salsa::db]
impl Db for Memory {
  fn check_file(&self, file: File) -> Vec<Diagnostic> {
    if self.should_check_file(file) {
      check_file_unwrap(self, self.program_file(file))
    } else {
      Vec::new()
    }
  }

  fn program_file(&self, file: File) -> ProgramFile<'_> {
    program(self).program_file(self, file)
  }

  fn python_version_with_source(&self, _file: File) -> &PythonVersionWithSource {
    &self.program.python_version
  }

  fn rule_selection(&self, _file: File) -> &RuleSelection {
    &self.rules
  }

  fn lint_registry(&self) -> &LintRegistry {
    default_lint_registry()
  }

  fn analysis_settings(&self, _file: File) -> &AnalysisSettings {
    &self.analysis
  }

  fn verbose(&self) -> bool {
    false
  }

  fn is_open_file(&self, _file: File) -> bool {
    false
  }

  fn dyn_clone(&self) -> Box<dyn Db> {
    // One memory belongs to one gate: salsa gives one owner of a storage, and nothing of the crate drives the
    // fixes of ty, which is the one thing that asks for a second handle.
    panic!("a gate holds one memory, and nothing of the crate asks ty for a second")
  }
}

#[salsa::db]
impl ModuleResolverDb for Memory {}

#[salsa::db]
impl ProgramDb for Memory {
  fn settings(&self) -> &ProgramSettings {
    &self.program
  }
}

#[salsa::db]
impl salsa::Database for Memory {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_names_of_the_sheet_are_the_names_the_globals_of_a_chain_hold() {
    let held = named(ENGINE);
    for one in ["send", "ask", "read", "bash", "prompt", "close", "Text", "Act", "actor", "raised"] {
      assert!(held.iter().any(|name| name == one), "{one} is a name a chain holds");
    }
    assert_eq!(held.iter().filter(|one| *one == "send").count(), 1, "a name stands once");
  }

  #[test]
  fn a_word_that_says_the_name_of_the_sheet_is_refused_before_it_is_read() {
    let mut held = Ty::new();
    assert_eq!(held.gate("__engine.acts", &[], "int"), [format!("{ALIAS} is a name of the gate")]);
  }

  #[test]
  fn a_word_the_gate_accepts_gives_no_finding() {
    let mut held = Ty::new();
    assert_eq!(held.gate("close(1)", &[], "int"), Vec::<String>::new());
    assert_eq!(held.gate("x = read('a.txt')\nclose(len(x.lines))", &[], "int"), Vec::<String>::new());
  }

  #[test]
  fn the_gate_refuses_a_word_whose_close_carries_a_value_that_does_not_have_the_shape() {
    let mut held = Ty::new();
    let said = held.gate("close('one')", &[], "int");
    assert!(!said.is_empty(), "a close of str is no int");
    assert!(said[0].starts_with("line 1: "), "{said:?}");
  }

  #[test]
  fn a_response_that_is_not_python_is_a_finding_like_any_other() {
    let mut held = Ty::new();
    assert!(!held.gate("close(", &[], "int").is_empty());
    assert!(!held.gate("return 1", &[], "int").is_empty(), "a top level return is no python");
  }

  #[test]
  fn a_word_may_await_an_act_at_its_top_level() {
    let mut held = Ty::new();
    // The code of an exit is a number or nothing, which is the shape the word must be read against.
    assert_eq!(held.gate("close((await bash('ls')).code)", &[], "int | None"), Vec::<String>::new());
    assert!(!held.gate("close((await bash('ls')).code)", &[], "int").is_empty(), "a code that may be nothing is no int");
  }

  #[test]
  fn the_gate_reads_the_word_against_the_rungs_before_it() {
    let mut held = Ty::new();
    let ladder = ["held = 1".to_owned()];
    assert_eq!(held.gate("close(held)", &ladder, "int"), Vec::<String>::new());
    assert!(!held.gate("close(held)", &[], "int").is_empty(), "a name no rung bound is a finding");
  }

  #[test]
  fn a_finding_keeps_the_line_the_model_wrote_it_on() {
    let mut held = Ty::new();
    let said = held.gate("x = 1\ny = 2\nclose(nowhere)", &[], "int");
    assert!(said.iter().any(|one| one.starts_with("line 3: ")), "{said:?}");
  }
}
