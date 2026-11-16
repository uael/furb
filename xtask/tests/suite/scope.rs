//! The task `scope`: the paths of a change say which jobs of the gates run.

use std::{
  io::Write as _,
  path::Path,
  process::{Output, Stdio},
};

use crate::xtask;

const LINUX: &str = r#"["ubuntu-latest"]"#;
const EVERY: &str = r#"["ubuntu-latest","macos-latest","windows-latest"]"#;

/// The lines that the task prints for these paths, one on each line of its input, or for every
/// job with `--all`.
fn of(words: &[&str], paths: &[&str]) -> String {
  let mut child = xtask(Path::new("."), words)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .unwrap_or_else(|no| panic!("the task did not start: {no}"));
  if let Some(mut input) = child.stdin.take() {
    input.write_all(paths.join("\n").as_bytes()).unwrap_or_else(|no| panic!("the paths: {no}"));
  }
  let Output { status, stdout, .. } =
    child.wait_with_output().unwrap_or_else(|no| panic!("the task did not end: {no}"));
  assert!(status.success(), "{paths:?}");
  String::from_utf8_lossy(&stdout).into_owned()
}

/// The answer of the scope, as the lines of the outputs of a step: the systems of the suite, of
/// the bindings, of the crate and of the builds, and whether the change holds code.
fn answer([suite, ts, krate, build]: [&str; 4], code: bool) -> String {
  format!("suite_os={suite}\nts_os={ts}\ncrate_os={krate}\nbuild_os={build}\ncode={code}\n")
}

#[test]
fn a_change_to_the_bindings_alone_runs_them_on_each_system_and_the_suite_on_linux() {
  let got =
    of(&["scope"], &["bind/typescript/src/session.ts", "bind/typescript/test/engine.test.ts"]);
  assert_eq!(got, answer([LINUX, EVERY, LINUX, EVERY], true));
}

#[test]
fn a_change_to_the_tests_of_python_alone_runs_the_suite_on_each_system_and_the_bindings_on_linux() {
  assert_eq!(
    of(&["scope"], &["test/test_act.py", "pyproject.toml"]),
    answer([EVERY, LINUX, LINUX, EVERY], true)
  );
}

#[test]
fn a_change_to_the_window_alone_runs_the_crate_on_each_system_and_the_other_jobs_on_linux() {
  let got = of(&["scope"], &["window/src/model.rs", "window/tests/screens/start.snap"]);
  assert_eq!(got, answer([LINUX, LINUX, EVERY, EVERY], true));
}

#[test]
fn a_change_to_the_crate_or_to_a_path_that_no_rule_names_runs_every_job() {
  let every = answer([EVERY, EVERY, EVERY, EVERY], true);
  for path in [
    "src/furb/world.py",
    ".agents/skills/web/web.py",
    "Cargo.lock",
    "xtask/src/scope.rs",
    ".github/workflows/gates.yml",
    ".github/actions/setup/action.yml",
    "something-new.cfg",
  ] {
    assert_eq!(of(&["scope"], &[path]), every, "{path}");
  }
  assert_eq!(of(&["scope", "--all"], &[]), every, "a run by hand or of each night");
}

#[test]
fn a_change_to_the_engine_or_to_the_corpus_alone_runs_every_job_on_linux() {
  let linux = answer([LINUX, LINUX, LINUX, LINUX], true);
  let engine =
    ["src/furb/engine.py", "src/furb/engine.pyi", "src/furb/record.py", "src/furb/kernel.py"];
  assert_eq!(of(&["scope"], &engine), linux);
  assert_eq!(of(&["scope"], &["tests/corpus/next/work.jsonl"]), linux);
  assert_eq!(
    of(&["scope"], &["src/furb/engine.py", "src/world.rs"]),
    answer([EVERY, EVERY, EVERY, EVERY], true)
  );
}

#[test]
fn a_change_to_a_skill_of_the_project_runs_the_suite_on_linux() {
  let got = of(&["scope"], &[".furb/skills/lane/lane.py", ".furb/config.json"]);
  assert_eq!(got, answer([LINUX, LINUX, LINUX, LINUX], true));
}

#[test]
fn a_change_to_prose_to_the_tools_or_to_a_workflow_with_its_own_proof_runs_no_job_of_code() {
  let none = answer([LINUX, LINUX, LINUX, LINUX], false);
  let tools =
    ["AGENTS.md", "docs/models.md", "script/prove-npm.sh", "xtask/python/tokens.py", "LICENSE"];
  assert_eq!(of(&["scope"], &tools), none);
  let workflows =
    [".github/workflows/publish.yml", ".github/workflows/titles.yml", ".github/budgets.json"];
  assert_eq!(of(&["scope"], &workflows), none);
  assert_eq!(of(&["scope"], &[]), none);
}
