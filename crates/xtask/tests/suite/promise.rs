//! The task `promise`, on a workspace in a yard whose page names proofs of python and of rust.
#![expect(clippy::panic, reason = "a panic is how a test fails")]

use std::{env, fs, path::Path, process};

use crate::{ended, xtask, yard::Yard};

/// The page of the promise: each proof under its heading, and one after what no machine proves
/// yet, which stands in no file.
const PAGE: &str = "# The promise\n\n## The proofs that a machine runs\n\n\
  - `test_proof.py::test_holds`\n- `src/lib.rs::in_lib`\n- `tests/suite/promise.rs::in_suite`\n\
  - `crates/host/tests/suite/main.rs::in_host`\n\n\
  What no machine proves yet:\n\n- `rollback.py::test_later`\n";

/// The files of the workspace: the crates furb and furb-host, each test that the page names
/// passing, or failing for the names given.
fn workspace(yard: &Path, failing: &[&str]) {
  let body = |name: &str| if failing.contains(&name) { "panic!(\"no\")" } else { "" };
  let python = if failing.contains(&"test_holds") { "assert False" } else { "pass" };
  let crate_of =
    |name: &str| format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
  let files = [
    ("promise.md", PAGE.to_owned()),
    ("pytest.ini", "[pytest]\n".to_owned()),
    ("test_proof.py", format!("def test_holds():\n  {python}\n")),
    ("Cargo.toml", format!("{}\n[workspace]\nmembers = [\"crates/host\"]\n", crate_of("furb"))),
    ("src/lib.rs", format!("#[test]\nfn in_lib() {{ {} }}\n", body("in_lib"))),
    ("tests/suite/main.rs", "mod promise;\n".to_owned()),
    ("tests/suite/promise.rs", format!("#[test]\nfn in_suite() {{ {} }}\n", body("in_suite"))),
    ("crates/host/Cargo.toml", crate_of("furb-host")),
    ("crates/host/src/lib.rs", String::new()),
    (
      "crates/host/tests/suite/main.rs",
      format!("#[test]\nfn in_host() {{ {} }}\n", body("in_host")),
    ),
  ];
  for (file, text) in files {
    let path = yard.join(file);
    let made =
      path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::write(&path, text));
    made.unwrap_or_else(|no| panic!("{file} was not made: {no}"));
  }
}

/// The code of the task and each line by which it says how its proofs ended.
fn promised(yard: &Path) -> (Option<i32>, Vec<String>) {
  // The proofs of python run in the environment of the repository, which holds pytest.
  let venv = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.venv");
  let mut command = xtask(yard, ["promise", "promise.md"]);
  command.env("VIRTUAL_ENV", venv).env_remove("CARGO_TARGET_DIR");
  let (code, out, _) = ended(&mut command);
  let said = out.lines().filter(|line| line.ends_with("did not pass") || line.ends_with("passed"));
  (code, said.map(str::to_owned).collect())
}

#[test]
fn a_proof_that_fails_is_named_and_the_same_proof_that_passes_lets_the_release_go() {
  let yard = Yard::new(format!("xtask-promise-{}", process::id()));
  workspace(&yard, &["test_holds", "in_host"]);
  let failed = |proof: &str| format!("promise: {proof} did not pass");
  assert_eq!(
    promised(&yard),
    (
      Some(1),
      vec![
        failed("test_proof.py::test_holds"),
        failed("crates/host/tests/suite/main.rs::in_host"),
        "promise: 2 of 4 proofs passed".to_owned()
      ]
    )
  );
  workspace(&yard, &[]);
  assert_eq!(
    promised(&yard),
    (Some(0), vec!["promise: 4 of 4 proofs passed".to_owned()]),
    "each proof of python and of rust runs in the target that holds it, and no proof after what \
     no machine proves yet"
  );
}
