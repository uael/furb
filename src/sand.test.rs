use monty_types::NamedValues;

use super::{Nobody, Sand};

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
