//! The task `targets`: clippy on the systems of the gates that this machine does not build.

use std::{
  path::Path,
  process::{Command as Process, ExitCode},
};

use crate::Fault;

/// Each system of the gates that a Mac does not build, as rust names it and as zig names it.
const SYSTEMS: [(&str, &str); 2] = [
  ("x86_64-unknown-linux-gnu", "x86_64-linux-gnu"),
  ("x86_64-pc-windows-gnu", "x86_64-windows-gnu"),
];

/// What each run lints. The window and its frames build the native library of OpenTUI for this
/// machine alone, the build of the addon of N-API looks for the library of node on Windows, and
/// the tests of the command line run the window, so these stay out.
const RUNS: [&[&str]; 2] = [
  &[
    "--workspace",
    "--exclude",
    "furb-window",
    "--exclude",
    "furb-frames",
    "--exclude",
    "furb-node",
    "--exclude",
    "furb-cli",
    "--all-targets",
  ],
  &["--package", "furb-cli", "--bins"],
];

/// Lint the workspace on each target of `SYSTEMS`, so code under a cfg that this machine does not
/// compile shows its warnings here, before the gates of CI fail on it. zig compiles the C of the
/// dependencies for each target, with no flag of cc-rs, since zig reads no target of rust.
pub(crate) fn lint(root: &Path) -> Result<ExitCode, Fault> {
  for run in RUNS {
    let mut clippy = Process::new("cargo");
    clippy.current_dir(root).env("CRATE_CC_NO_DEFAULTS", "1").arg("clippy");
    for (rust, zig) in SYSTEMS {
      let key = rust.replace('-', "_");
      clippy.env(format!("CC_{key}"), format!("zig cc -target {zig}"));
      clippy.env(format!("CXX_{key}"), format!("zig c++ -target {zig}"));
      clippy.env(format!("AR_{key}"), "zig ar");
      clippy.args(["--target", rust]);
    }
    let status = clippy
      .args(run)
      .args(["--", "-D", "warnings"])
      .status()
      .map_err(|source| Fault::Spawn { program: "cargo".to_owned(), source })?;
    if !status.success() {
      return Ok(ExitCode::FAILURE);
    }
  }
  Ok(ExitCode::SUCCESS)
}
