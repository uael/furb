//! The gate: ty, as a library, reading a sheet.
//!
//! The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word against
//! the rungs before it, and that a response which is not python is a finding like any other. Nothing of that is
//! a policy of a host: it is the contract, and the crate carries it.
//!
//! The sheet the word stands on is `furb.sheet`'s, written the same way by every Kernel, in this sandbox and in
//! the python package alike, so this reads a text and says what ty found on it, each finding by its line. What it
//! reads against is `engine.pyi`, the contract, which the crate carries beside the engine and hands to ty as the
//! stub of the module a sheet imports.
//!
//! ty is pinned to the commit the `ty` command line of the package is built from, so a word is judged the same
//! by the crate and by the command line: the same engine, the same typeshed, the same python, and the errors
//! alone, since a warning refuses no word.

use std::{cell::RefCell, fmt::Write as _, rc::Rc, sync::Arc};

use ruff_db::{
  Db as SourceDb,
  diagnostic::{Diagnostic, DiagnosticFormat, DisplayDiagnosticConfig, DisplayDiagnostics, Severity},
  file_revision::FileRevision,
  files::{File, FileRootKind, Files, system_path_to_file},
  system::{DbWithTestSystem, DbWithWritableSystem as _, System, SystemPathBuf, TestSystem},
  vendored::VendoredFileSystem,
};
use ruff_python_ast::{PythonVersion, Stmt};
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
  dependency::DependencyMetadata,
  lint::{LintRegistry, RuleSelection},
};

use crate::CONTRACT;

/// The two names a chain binds of its own: the actor it stands on, and what the last rung raised.
const BOUND: [&str; 2] = ["actor", "raised"];
/// Where every file of the reading stands, since nothing of it is on a disk.
const ROOT: &str = "/";
/// The sheet, which is the one file ty reads.
const SHEET: &str = "sheet.py";

/// The gate of the crate: ty, reading a sheet against the contract.
///
/// One of these serves every life of a thread. It holds one reading of the contract and of the typeshed, which
/// cost once, so a reading of a sheet costs the sheet alone, in every life after the first as in the first. A
/// clone is the same gate, and a life is given one.
#[derive(Clone)]
pub struct Ty {
  /// The reading, which every clone shares, and which one clone reads at a time.
  held: Rc<RefCell<Reading>>,
}

impl Default for Ty {
  fn default() -> Self {
    Self::new()
  }
}

impl Ty {
  /// A gate that reads a sheet against the contract the crate carries.
  #[must_use]
  pub fn new() -> Self {
    let mut held = Reading { db: Memory::default(), revision: 0 };
    held.wrote("furb/__init__.pyi", "");
    held.wrote("furb/engine.pyi", CONTRACT);
    Ty { held: Rc::new(RefCell::new(held)) }
  }

  /// What ty found on a sheet: each error by the line it stands on, in the concise form ty writes, and no
  /// warning, since a warning refuses no word.
  pub fn checked(&self, sheet: &str) -> Vec<(usize, String)> {
    self.held.borrow_mut().checked(sheet)
  }
}

/// One reading of ty: where it reads its files, and the revision of the last write.
struct Reading {
  /// Where ty reads its files, which is a memory and no disk.
  db: Memory,
  /// The revision of the last write, so that two writes of one path are two writes to ty.
  revision: u128,
}

impl Reading {
  /// What ty found on a sheet, as [`Ty::checked`] says it.
  fn checked(&mut self, sheet: &str) -> Vec<(usize, String)> {
    let Some(file) = self.wrote(SHEET, sheet) else {
      return vec![(0, "the gate could not be given the sheet".to_owned())];
    };
    let held: Vec<Diagnostic> = check_file_unwrap(&self.db, self.db.program_file(file))
      .into_iter()
      .filter(|one| one.severity() >= Severity::Error)
      .collect();
    if held.is_empty() {
      return Vec::new();
    }
    let config = DisplayDiagnosticConfig::new("furb").format(DiagnosticFormat::Concise).color(false);
    let mut said = String::new();
    let _ = write!(said, "{}", DisplayDiagnostics::new(&self.db, &config, &held));
    said.lines().filter_map(read).collect()
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
}

/// One finding of ty in its concise form, as the line it stands on and what it says.
///
/// The form is `path:line:column: severity[rule] why`, and a line that is not of that form is no finding of the
/// sheet, so it is dropped rather than read as one.
fn read(said: &str) -> Option<(usize, String)> {
  let (_, rest) = said.split_once(':')?;
  let (line, rest) = rest.split_once(':')?;
  let (_, rest) = rest.split_once(": ")?;
  Some((line.trim().parse().ok()?, rest.trim().to_owned()))
}

/// Every name the engine defines at its top, and the two a chain binds of its own, which is what the globals of a
/// chain hold and what the sheet binds before it reads the ladder and the word.
pub fn named(source: &str) -> Vec<String> {
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
/// gives one owner of a storage, so it is never cloned, and the gate shares it by handing out handles to itself.
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
  /// The python the sheet is read for, which is the python that runs the word.
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
      python_version: PythonVersionWithSource { version: PythonVersion::PY314, source: PythonVersionSource::default() },
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
  Program::from_settings(db, db.settings())
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
    if self.should_check_file(file) { check_file_unwrap(self, self.program_file(file)) } else { Vec::new() }
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

  fn dependency_metadata(&self, _file: File) -> Option<&DependencyMetadata> {
    None
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
  use crate::ENGINE;

  /// The sheet of one word, on the ladder of no rung, as `furb.sheet` writes it.
  fn sheet(word: &str) -> (String, usize) {
    let mut head = String::from("import furb.engine as __engine\nasync def __body():\n");
    for name in named(ENGINE) {
      let _ = writeln!(head, "  {name} = __engine.{name}");
    }
    let above = format!("{head}  try:\n    __engine.lineage('')\n\n  except BaseException:\n    pass\n");
    let lines = above.lines().count();
    (format!("{above}  {word}\n"), lines)
  }

  #[test]
  fn the_names_of_the_sheet_are_the_names_the_globals_of_a_chain_hold() {
    let held = named(ENGINE);
    for one in ["send", "ask", "read", "bash", "prompt", "close", "Text", "Act"] {
      assert!(held.iter().any(|name| name == one), "{one} is a name a chain holds");
    }
    assert!(!held.iter().any(|name| name == "actor"), "the two names a chain binds of its own are the sheet's to add");
    assert_eq!(held.iter().filter(|one| *one == "send").count(), 1, "a name stands once");
  }

  #[test]
  fn a_sheet_the_gate_accepts_gives_no_finding() {
    let held = Ty::new();
    let (text, _) = sheet("close(len(read('a.txt').lines))");
    assert_eq!(held.checked(&text), Vec::<(usize, String)>::new());
  }

  #[test]
  fn a_clone_of_the_gate_is_the_same_gate_and_reads_a_sheet_the_same() {
    let held = Ty::new();
    let same = held.clone();
    let (text, above) = sheet("close(nowhere)");
    assert_eq!(same.checked(&text).len(), 1);
    assert_eq!(held.checked(&text)[0].0, above + 1);
    assert_eq!(same.checked(&sheet("close(1)").0), Vec::<(usize, String)>::new());
  }

  #[test]
  fn a_finding_says_the_line_of_the_sheet_it_stands_on() {
    let held = Ty::new();
    let (text, above) = sheet("close(nowhere)");
    let found = held.checked(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, above + 1);
    assert!(found[0].1.contains("unresolved-reference"), "{found:?}");
  }

  #[test]
  fn a_word_may_await_an_act_at_its_top_level() {
    let held = Ty::new();
    let (text, _) = sheet("close((await bash('ls')).code)");
    assert_eq!(held.checked(&text), Vec::<(usize, String)>::new());
  }

  #[test]
  fn a_warning_refuses_no_word() {
    let held = Ty::new();
    // A name that may be unbound is a warning of ty, and a word that reads it is no word the gate refuses.
    let (text, _) = sheet("if chance() > 0.5:\n    maybe = 1\n  close(maybe)");
    assert_eq!(held.checked(&text), Vec::<(usize, String)>::new());
  }
}
