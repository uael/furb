//! The tests of how the watch names what failed in the log of a part, which only a whole check
//! reaches through its door.

use std::{fs, process};

use crate::yard::Yard;

use super::named;

#[test]
fn a_suite_past_its_limits_with_each_test_green_is_named_by_its_budget() {
  let yard = Yard::new(format!("xtask-watch-named-{}", process::id()));
  let log = yard.join("watch-rust.log");
  let text = "     Summary [  58.0s] 1165 tests run: 1165 passed, 12 skipped\n\
              budget: cargo took 58 s of 60 s and 378 s of CPU of 360 s\n\
              budget: cargo used 378 s of CPU, 18 s over its limit.\n\
              budget: make the run faster, and never raise a limit. Its slowest tests:\n";
  fs::write(&log, text).unwrap_or_else(|no| panic!("the log: {no}"));
  assert_eq!(
    named(&yard, "rust", &log),
    ["budget of rust: cargo used 378 s of CPU, 18 s over its limit."]
  );
  fs::write(&log, "budget: pytest passed its wall limit by 4 s.\n")
    .unwrap_or_else(|no| panic!("the log: {no}"));
  assert_eq!(
    named(&yard, "python", &log),
    ["budget of python: pytest passed its wall limit by 4 s."]
  );
}
