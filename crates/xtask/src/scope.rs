//! The task scope: which jobs of the gates a change needs, from the paths that it changes.
//!
//! The jobs on Linux always run: they hold the whole proof of the logic. A job on macOS or on
//! Windows proves the system alone, and a runner of macOS waits in a queue for minutes, so a change
//! runs those jobs only when one of its paths can behave in another way there. The paths come one
//! on each line from the standard input, and the answer is the lines `key=value` that a step of
//! GitHub takes into its outputs. A path that no rule names runs every job.

use std::{
  io::{self, BufRead},
  path::PathBuf,
  process::ExitCode,
};

use fancy_regex::Regex;
use serde_json::{Map, Value, json};

use crate::Fault;

const LINUX: &[&str] = &["ubuntu-latest"];
const EVERY: &[&str] = &["ubuntu-latest", "macos-latest", "windows-latest"];

/// What a path is, by the jobs that read it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
  /// A path that no job needs on another system: prose, the tools of the repository, and the
  /// workflows that hold their own proof.
  Inert,
  /// A path of pure logic that every job reads, but that behaves the same on each system: the
  /// engine and the code that monty runs beside it, whose sandbox gives them no access to the
  /// machine, and the corpus of the records. The World alone speaks to the machine. In 36 hours
  /// of main, 19 of 125 commits touched them, and 6 of them touched no other path that runs the
  /// jobs on macOS and on Windows.
  Logic,
  /// The extensions of the project and the config that enables them, which the suite of python
  /// reads on Linux alone: its suite in .furb/skills checks each word as a chain plays it.
  Project,
  /// A path that only the suite of python reads.
  Python,
  /// A path that only the bindings of TypeScript read.
  Typescript,
  /// A path that only the window reads. No other package depends on the window, so its change
  /// runs the tests of the crate on each system, where it draws with a native library of that
  /// system, and the other jobs on Linux alone.
  Window,
  /// A path that the crate builds, which every job reads.
  Rust,
  /// A path that no rule names, which every job reads.
  Other,
}

/// Each rule in its order: the first that matches a whole path names its kind.
const RULES: [(&str, Kind); 7] = [
  (
    r"(.*\.md|docs/(?!examples/).*|LICENSE|\.gitignore|\.furb/(?!skills/|config\.json$).*|\.agents/(?!skills/).*|\.claude/.*|script/.*|crates/xtask/python/.*|crates/reserve/.*|\.github/(ISSUE_TEMPLATE/.*|budgets\.json|workflows/(publish|release|titles|contract)\.yml)|release-please-config\.json|\.release-please-manifest\.json)",
    Kind::Inert,
  ),
  (r"src/furb/(engine\.pyi?|record\.py|kernel\.py)|tests/corpus/.*", Kind::Logic),
  (r"\.furb/(skills/.*|config\.json)", Kind::Project),
  (
    r"tests/([^/]+\.py|outside/.*|checkpoint/.*)|crates/python/.*|pyproject\.toml|uv\.lock|docs/examples/python/.*",
    Kind::Python,
  ),
  (
    r"crates/typescript/.*|package\.json|bun\.lock|biome\.json|tsconfig.*\.json|docs/examples/typescript/.*",
    Kind::Typescript,
  ),
  (r"crates/window/.*", Kind::Window),
  (
    r"(src|crates/cli|tests|extensions|\.cargo)/.*|Cargo\.(toml|lock)|build\.rs|rust-toolchain\.toml|docs/examples/rust/.*",
    Kind::Rust,
  ),
];

/// Say the outputs of the scope of the paths of the standard input, or of every job.
pub(crate) fn run(all: bool) -> Result<ExitCode, Fault> {
  let paths = if all {
    // A run by hand with no base and the run of each night give no paths, and a path that no
    // rule names runs every job.
    vec!["every".to_owned()]
  } else {
    let read = io::stdin().lock().lines().collect::<Result<Vec<_>, _>>();
    read.map_err(|source| Fault::Read { path: PathBuf::from("stdin"), source })?
  };
  print!("{}", lines(&scope(&paths)?));
  Ok(ExitCode::SUCCESS)
}

/// The systems of the suite, of the bindings and of the tests of the crate, the systems that
/// build for them, and whether the change holds code.
fn scope(paths: &[String]) -> Result<Map<String, Value>, Fault> {
  let rules = RULES
    .iter()
    .map(|(rule, kind)| Ok((Regex::new(&format!("^(?:{rule})$"))?, *kind)))
    .collect::<Result<Vec<_>, Fault>>()?;
  let mut kinds = Vec::new();
  for path in paths.iter().map(|path| path.trim()).filter(|path| !path.is_empty()) {
    let mut kind = Kind::Other;
    for (rule, named) in &rules {
      if rule.is_match(path)? {
        kind = *named;
        break;
      }
    }
    if kind != Kind::Inert {
      kinds.push(kind);
    }
  }
  let has = |kind| kinds.contains(&kind);
  let every = has(Kind::Rust) || has(Kind::Other);
  let os = |also| if every || also { EVERY } else { LINUX };
  let systems = [os(has(Kind::Python)), os(has(Kind::Typescript)), os(has(Kind::Window))];
  // Each system that a job of the suite, of the bindings or of the crate runs on builds once, for
  // each of them.
  let built = if systems.contains(&EVERY) { EVERY } else { LINUX };
  let mut answer = Map::new();
  for (key, value) in ["suite_os", "ts_os", "crate_os", "build_os"]
    .into_iter()
    .zip([systems[0], systems[1], systems[2], built])
  {
    answer.insert(key.to_owned(), json!(value));
  }
  answer.insert("code".to_owned(), json!(!kinds.is_empty()));
  Ok(answer)
}

/// The answer as the lines of an output of a step.
fn lines(answer: &Map<String, Value>) -> String {
  answer.iter().map(|(key, value)| format!("{key}={value}\n")).collect::<Vec<_>>().concat()
}
