use std::time::Duration;

use monty_types::NamedValues;

use super::{BOUND, Nobody, Sand};

#[test]
fn a_call_of_the_operating_system_raises_where_the_code_made_it_and_the_run_goes_on() {
  let code =
    "try:\n  open('a.txt')\n  got = 'opened'\nexcept RuntimeError as no:\n  got = str(no)\ngot";
  let got =
    Sand::new().run(code, NamedValues::new(), &mut Nobody("the code")).expect("the run goes on");
  assert_eq!(
    got.as_ref().as_str(),
    Some("open asks the operating system, and a life asks nothing of it")
  );
}

#[test]
fn an_import_of_a_module_that_monty_does_not_hold_raises_module_not_found_and_the_run_goes_on() {
  let code = "try:\n  import numpy\n  got = 'imported'\nexcept ModuleNotFoundError as no:\n  got = str(no)\ngot";
  let got =
    Sand::new().run(code, NamedValues::new(), &mut Nobody("the code")).expect("the run goes on");
  assert_eq!(got.as_ref().as_str(), Some("No module named 'numpy'"));
}

#[test]
fn a_stretch_past_the_bound_raises_a_timeout_where_the_code_stands_which_the_code_may_catch() {
  BOUND.set(Duration::from_millis(100));
  let code = "try:\n  while True:\n    pass\nexcept TimeoutError as no:\n  got = str(no)\ngot";
  let got = Sand::new()
    .run(code, NamedValues::new(), &mut Nobody("the code"))
    .expect("the code catches it");
  assert_eq!(got.as_ref().as_str(), Some("turn time limit exceeded: 100ms"));
}
