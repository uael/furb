//! The task `targets`: clippy on the systems of the gates that this machine does not build.

use std::{
  path::Path,
  process::{Command as Process, ExitCode},
};

use crate::Fault;

/// Each system of the gates that a Mac does not build, as rust names it and as zig names it, with
/// what its run lints: what the clippy of the gates lints there. Windows lints the gnu target of
/// rust, whose C zig compiles, while the gates take msvc: there napi looks for the library of node
/// in its build, so the addon of N-API and the feature that it turns on stay out. The build of the
/// window links nothing in a lint, as `FURB_LINT_ONLY` tells it.
const SYSTEMS: [(&str, &str, &[&str]); 2] = [
  (
    "x86_64-unknown-linux-gnu",
    "x86_64-linux-gnu",
    &["--workspace", "--all-targets", "--features", "furb/typescript"],
  ),
  (
    "x86_64-pc-windows-gnu",
    "x86_64-windows-gnu",
    &["--workspace", "--all-targets", "--exclude", "furb-node"],
  ),
];

/// Lint the workspace on each target of `SYSTEMS`, so code under a cfg that this machine does not
/// compile shows its warnings here, before the gates of CI fail on it. zig compiles the C of the
/// dependencies for each target, with no flag of cc-rs, since zig reads no target of rust.
pub(crate) fn lint(root: &Path) -> Result<ExitCode, Fault> {
  for (rust, zig, run) in SYSTEMS {
    let key = rust.replace('-', "_");
    let status = Process::new("cargo")
      .current_dir(root)
      .env("CRATE_CC_NO_DEFAULTS", "1")
      .env("FURB_LINT_ONLY", "1")
      .env(format!("CC_{key}"), format!("zig cc -target {zig}"))
      .env(format!("CXX_{key}"), format!("zig c++ -target {zig}"))
      .env(format!("AR_{key}"), "zig ar")
      .args(["clippy", "--target", rust])
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
