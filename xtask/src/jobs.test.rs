//! The tests of the budgets of the jobs of the gates.

use super::{named, seconds};
use crate::root;

#[test]
fn the_budgets_are_set_for_the_jobs_that_the_gates_name_and_for_no_other() {
  let kept = named(&root().join(".github/workflows/gates.yml")).unwrap();
  let has = |name: &str| kept.iter().any(|one| one.is_match(name).unwrap());
  assert!(
    has("the jobs on macos-latest / the suite on macos-latest"),
    "a job of a matrix has one for each system"
  );
  assert!(has("the jobs on windows-latest / the bindings on windows-latest"));
  assert!(!has("the jobs on macos-latest"), "the job that calls has none of its own");
  assert!(!has("the app on macos-latest"), "a job that no workflow names has none");
  assert!(!has("the gates"), "the job that only holds the check has none");
  assert_eq!(seconds("2026-11-15T12:00:30Z") - seconds("2026-11-14T23:59:00Z"), 43_290);
}
