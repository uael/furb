//! A test that runs again as a child of itself, with the variables of the environment that it
//! gives, as a user starts a host with them: the catalog of a life and the World read their
//! settings from the environment of their process, which a test in that process cannot set.

#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{
  env,
  path::{Path, PathBuf},
  process::Command,
};

/// The variable that names the yard of a test to the child that runs it again.
const CHILD: &str = "FURB_TEST_YARD";

/// The path of the test that calls it, as the runner of the tests names it.
macro_rules! here {
  () => {{
    fn here() {}
    let name = std::any::type_name_of_val(&here);
    let name = name.strip_suffix("::here").expect("a function of a test");
    name.split_once("::").expect("the crate of the test").1
  }};
}

/// The yard of the test when this process is the child that runs it again.
pub(crate) fn child() -> Option<PathBuf> {
  env::var_os(CHILD).map(PathBuf::from)
}

/// The test at this path run again as a child of this process, in the yard, with these variables,
/// and nothing of the catalog of this machine: no cache, no local server and no most thought. The
/// child fails the test when it fails.
pub(crate) fn ran(test: &str, at: &Path, vars: &[(&str, &str)]) {
  let mut command = Command::new(env::current_exe().expect("the program of the tests"));
  command.args([test, "--exact", "--nocapture"]).env(CHILD, at);
  command.env("XDG_CACHE_HOME", at.join("cache"));
  let names = ["FURB_LOCAL_URL", "FURB_LOCAL_KEY", "FURB_LOCAL_WINDOW", "FURB_LOCAL_TOKENS"];
  for name in names.into_iter().chain(["MAX_THINKING_TOKENS"]) {
    command.env_remove(name);
  }
  command.envs(vars.iter().copied());
  let output = command.output().expect("the child runs");
  let said = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
  assert!(
    output.status.success() && said(&output.stdout).contains("1 passed"),
    "{}{}",
    said(&output.stdout),
    said(&output.stderr)
  );
}

pub(crate) use here;
