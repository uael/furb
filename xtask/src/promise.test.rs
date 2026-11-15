//! The tests of the proofs of the promise.

use std::{fs, process::id};

use fancy_regex::Regex;

use super::{proofs, proven, runs};
use crate::{root, yard::Yard};

#[test]
fn the_page_of_the_promise_names_its_proofs_of_python_and_of_rust_and_each_goes_to_its_target() {
  let found = proofs(&fs::read_to_string(root().join("docs/promise.md")).unwrap()).unwrap();
  let proof = "each_proof_that_the_page_of_the_promise_names_stands_in_its_file";
  assert!(found.contains(&("tests/suite/promise.rs".to_owned(), proof.to_owned())));
  assert!(
    found.iter().all(|(file, _)| !file.contains("rollback")),
    "a proof not yet runs in no gate"
  );
  let commands: Vec<String> =
    runs(&found).iter().map(|run| format!("{} {}", run.program, run.args.join(" "))).collect();
  for start in [
    "uv run --no-sync python -m pytest",
    "cargo test -p furb-host --test suite",
    "cargo test -p furb --lib",
  ] {
    assert!(commands.iter().any(|one| one.starts_with(start)), "{start}: {commands:?}");
  }
}

#[test]
fn a_proof_that_fails_is_named_and_the_same_proof_that_passes_lets_the_release_go() {
  let yard = Yard::new(format!("xtask-promise-{}", id()));
  // A pytest.ini roots the proof at its folder, so pytest walks no tree above it.
  fs::write(yard.join("pytest.ini"), "[pytest]\n").unwrap();
  let file = yard.join("test_proof.py").to_string_lossy().into_owned();
  let found = [(file.clone(), "test_holds".to_owned())];
  fs::write(&file, "def test_holds():\n  assert False\n").unwrap();
  assert_eq!(proven(&root(), &found).unwrap(), found);
  fs::write(&file, "def test_holds():\n  pass\n").unwrap();
  assert_eq!(proven(&root(), &found).unwrap(), []);
}

#[test]
fn a_proof_named_by_a_path_of_windows_matches_each_line_by_which_pytest_says_it_passed() {
  let page = "## The proofs that a machine runs\n\n- `C:\\Users\\x\\test_proof.py::test_holds`\n\nWhat no machine proves yet\n";
  let found = proofs(page).unwrap();
  assert_eq!(found, [("C:\\Users\\x\\test_proof.py".to_owned(), "test_holds".to_owned())]);
  let pattern = Regex::new(&runs(&found)[0].passed[0]).unwrap();
  for line in [
    "PASSED C:\\Users\\x\\test_proof.py::test_holds",
    "PASSED ../x/test_proof.py::test_holds [case]",
    "PASSED ::test_holds",
  ] {
    assert!(pattern.is_match(line).unwrap(), "{line}");
  }
  assert!(!pattern.is_match("PASSED other/test_other.py::test_holds").unwrap());
}
