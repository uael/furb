//! What no door of the crate can make: a system that refuses the reactor its runtime.

use std::io;

use super::driven;

#[test]
fn a_reactor_that_the_system_refuses_gives_why_and_no_panic() {
  let refused = driven(|| Err(io::Error::new(io::ErrorKind::OutOfMemory, "too many open files")));
  let no = refused.err().map(|no| (no.kind(), no.to_string()));
  let why = "the system gives the ears no reactor: too many open files".to_owned();
  assert_eq!(no, Some((io::ErrorKind::OutOfMemory, why)));
}
