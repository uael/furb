//! The paths of a change say which jobs of the gates run.

use serde_json::{Value, json};

use super::{EVERY, LINUX, lines, scope};

fn of(paths: &[&str]) -> Value {
  let paths: Vec<String> = paths.iter().map(|one| (*one).to_owned()).collect();
  Value::Object(scope(&paths).expect("the rules of the scope"))
}

/// The answer of the scope: the systems of the suite, of the bindings, of the crate and of the
/// builds, and whether the change holds code.
fn answer([suite, ts, krate, build]: [&[&str]; 4], code: bool) -> Value {
  json!({"suite_os": suite, "ts_os": ts, "crate_os": krate, "build_os": build, "code": code})
}

#[test]
fn a_change_to_the_bindings_alone_runs_them_on_each_system_and_the_suite_on_linux() {
  let got = of(&["bind/typescript/src/session.ts", "bind/typescript/test/engine.test.ts"]);
  assert_eq!(got, answer([LINUX, EVERY, LINUX, EVERY], true));
}

#[test]
fn a_change_to_the_tests_of_python_alone_runs_the_suite_on_each_system_and_the_bindings_on_linux() {
  assert_eq!(
    of(&["test/test_act.py", "pyproject.toml"]),
    answer([EVERY, LINUX, LINUX, EVERY], true)
  );
}

#[test]
fn a_change_to_the_window_alone_runs_the_crate_on_each_system_and_the_other_jobs_on_linux() {
  let got = of(&["window/src/model.rs", "window/tests/screens/start.txt"]);
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
    assert_eq!(of(&[path]), every, "{path}");
  }
}

#[test]
fn a_change_to_the_engine_or_to_the_corpus_alone_runs_every_job_on_linux() {
  let linux = answer([LINUX, LINUX, LINUX, LINUX], true);
  let engine =
    ["src/furb/engine.py", "src/furb/engine.pyi", "src/furb/record.py", "src/furb/kernel.py"];
  assert_eq!(of(&engine), linux);
  assert_eq!(of(&["tests/corpus/next/work.jsonl"]), linux);
  assert_eq!(
    of(&["src/furb/engine.py", "src/world.rs"]),
    answer([EVERY, EVERY, EVERY, EVERY], true)
  );
}

#[test]
fn a_change_to_a_skill_of_the_project_runs_the_suite_on_linux() {
  let got = of(&[".furb/skills/lane/lane.py", ".furb/config.json"]);
  assert_eq!(got, answer([LINUX, LINUX, LINUX, LINUX], true));
}

#[test]
fn a_change_to_prose_to_the_tools_or_to_a_workflow_with_its_own_proof_runs_no_job_of_code() {
  let none = answer([LINUX, LINUX, LINUX, LINUX], false);
  let tools =
    ["AGENTS.md", "docs/models.md", "script/prove-npm.sh", "xtask/python/tokens.py", "LICENSE"];
  assert_eq!(of(&tools), none);
  let workflows =
    [".github/workflows/publish.yml", ".github/workflows/titles.yml", ".github/budgets.json"];
  assert_eq!(of(&workflows), none);
  assert_eq!(of(&[]), none);
}

#[test]
fn the_answer_is_the_lines_of_the_outputs_of_a_step() {
  let said = lines(&scope(&["bind/typescript/src/session.ts".to_owned()]).expect("the rules"));
  assert_eq!(
    said,
    "suite_os=[\"ubuntu-latest\"]\nts_os=[\"ubuntu-latest\",\"macos-latest\",\"windows-latest\"]\n\
     crate_os=[\"ubuntu-latest\"]\nbuild_os=[\"ubuntu-latest\",\"macos-latest\",\"windows-latest\"]\ncode=true\n"
  );
}
