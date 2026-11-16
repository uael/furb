//! The bound of a stretch of code, where no door shortens it: the engine bounds a turn at 30 s, so
//! only a test that sets the bound of its thread proves it. The suite proves the refusals of the
//! sandbox through a word of a life.

use std::time::Duration;

use monty_types::NamedValues;

use super::{BOUND, Nobody, Sand};

#[test]
fn a_stretch_past_the_bound_raises_a_timeout_where_the_code_stands_which_the_code_may_catch() {
  BOUND.set(Duration::from_millis(100));
  let code = "try:\n  while True:\n    pass\nexcept TimeoutError as no:\n  got = str(no)\ngot";
  let got = Sand::new()
    .run(code, NamedValues::new(), &mut Nobody("the code"))
    .expect("the code catches it");
  assert_eq!(got.as_ref().as_str(), Some("turn time limit exceeded: 100ms"));
}
