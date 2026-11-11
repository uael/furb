//! The tests that need a process of their own, since they count each byte that the process
//! allocates or set a limit of the process: each runs as a child of this binary, which runs that
//! test alone, while the other tests of the suite run beside it.

#![expect(clippy::expect_used, reason = "a test fails by a panic where its child fails to start")]

use std::{env, process::Command};

/// The variable that names the test that a child of the suite runs alone.
const ALONE: &str = "FURB_ALONE";

/// Runs the ignored test `name` in a child of this binary, alone, and fails with what the child
/// wrote when it fails.
pub(crate) fn run(name: &str) {
  let program = env::current_exe().expect("the suite knows its program");
  let args = [name, "--exact", "--ignored", "--nocapture", "--test-threads", "1"];
  let out = Command::new(program)
    .args(args)
    .env(ALONE, name)
    .output()
    .expect("the child of the suite starts");
  let said = String::from_utf8_lossy(&out.stdout) + String::from_utf8_lossy(&out.stderr);
  assert!(out.status.success(), "{name} failed alone:\n{said}");
  assert!(said.contains("1 passed"), "{name} did not run alone:\n{said}");
  println!("{said}");
}

/// Whether this process is the child that runs the test `name` alone: the same test that another
/// run of the suite reaches does nothing.
pub(crate) fn alone(name: &str) -> bool {
  env::var(ALONE).is_ok_and(|one| one == name)
}
