//! The tests that need a process of their own, since they count each byte that the process
//! allocates or set a limit of the process: each runs as a child of this binary, which runs that
//! test alone, while the other tests of the suite run beside it. The harness starts the tests in
//! the order of their names, so these, the longest, start first.

#![expect(clippy::expect_used, reason = "a test fails by a panic where its child fails to start")]

use std::{
  env,
  process::{Command, Stdio},
};

/// The variable that names the test that a child of the suite runs alone.
const ALONE: &str = "FURB_ALONE";

/// Runs the ignored tests `names`, each in a child of this binary, alone, all at once, and fails
/// with what each child that failed wrote. The test that waits for them holds one thread of the
/// harness.
pub(crate) fn run(names: &[&str]) {
  let program = env::current_exe().expect("the suite knows its program");
  let children: Vec<_> = names
    .iter()
    .map(|name| {
      let args = [name, "--exact", "--ignored", "--nocapture", "--test-threads", "1"];
      let mut command = Command::new(&program);
      command.args(args).env(ALONE, name).stdout(Stdio::piped()).stderr(Stdio::piped());
      (name, command.spawn().expect("the child of the suite starts"))
    })
    .collect();
  let mut failed = Vec::new();
  for (name, child) in children {
    let out = child.wait_with_output().expect("the child of the suite ends");
    let said = String::from_utf8_lossy(&out.stdout) + String::from_utf8_lossy(&out.stderr);
    if out.status.success() && said.contains("1 passed") {
      println!("{said}");
    } else {
      failed.push(format!("{name} failed alone:\n{said}"));
    }
  }
  assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn each_runs_alone() {
  run(&[
    "heap::alone",
    "memory::heap_alone",
    "memory::window_alone",
    "memory::rebirth_alone",
    #[cfg(unix)]
    "unkept::rebirth_alone",
    #[cfg(unix)]
    "unkept::backlog_alone",
  ]);
}

/// Whether this process is the child that runs the test `name` alone: the same test that another
/// run of the suite reaches does nothing.
pub(crate) fn alone(name: &str) -> bool {
  env::var(ALONE).is_ok_and(|one| one == name)
}
