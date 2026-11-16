//! The bound of a stretch of code, where no door shortens it: the engine bounds a turn at 30 s, so
//! only a test that sets the bound of its thread proves it. The suite proves the refusals of the
//! sandbox through a word of a life.

use std::{thread, time::Duration};

use monty_types::NamedValues;

use super::{BOUND, DEPTH, Nobody, Sand};
use crate::Object;

#[test]
fn a_stretch_past_the_bound_raises_a_timeout_where_the_code_stands_which_the_code_may_catch() {
  BOUND.set(Duration::from_millis(100));
  let code = "try:\n  while True:\n    pass\nexcept TimeoutError as no:\n  got = str(no)\ngot";
  let got = Sand::new()
    .run(code, NamedValues::new(), &mut Nobody("the code"))
    .expect("the code catches it");
  assert_eq!(got.as_ref().as_str(), Some("turn time limit exceeded: 100ms"));
}

#[test]
fn a_word_walks_a_value_as_deep_as_the_depth_on_a_small_stack_and_recurses_no_deeper() {
  let ran = |code: &'static str, n: usize| {
    let mut inputs = NamedValues::new();
    inputs.push("n", Object::int(i64::try_from(n).unwrap()));
    Sand::new().run(code, inputs, &mut Nobody("the code"))
  };
  let walks = "x = []\nfor i in range(n):\n  x = [x]\nlen(repr(x))";
  let recurses = "def f(k):\n  return 0 if k == 0 else 1 + f(k - 1)\nf(n)";
  // A thread of 256 KiB holds a few hundred levels: the run grows the stack that the depth takes.
  let small = thread::Builder::new().stack_size(256 * 1024);
  let got = small.spawn(move || (ran(walks, DEPTH - 100), ran(recurses, DEPTH + 100)));
  let (within, past) = got.unwrap().join().unwrap();
  let shown = within.expect("a value within the depth").as_ref().as_int();
  assert_eq!(shown, Some(2 * i64::try_from(DEPTH - 99).unwrap()), "a [ and a ] for each level");
  assert_eq!(past.expect_err("a call past the depth").name, "RecursionError");
}
