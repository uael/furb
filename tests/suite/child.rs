//! A test that runs again as a child of itself, with the variables of the environment that it
//! gives, as a user starts a host with them: the catalog of a life and the World read their
//! settings from the environment of their process, which a test in that process cannot set.

use std::{
  env,
  path::{Path, PathBuf},
  process::Command,
};

use serde_json::Value;

/// The snapshot of the catalog that the crate carries.
const SNAPSHOT: &str = include_str!("../../src/world/provider/catalog.json");

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
/// and nothing of the catalog of this machine: no cache, no credential, no place of an address, no
/// local server and no most thought. The child fails the test when it fails.
pub(crate) fn ran(test: &str, at: &Path, vars: &[(&str, &str)]) {
  let mut command = Command::new(env::current_exe().expect("the program of the tests"));
  command.args([test, "--exact", "--nocapture"]).env(CHILD, at);
  command.env("XDG_CACHE_HOME", at.join("cache"));
  for name in catalogued() {
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

/// Each variable that the catalog reads: the credentials and the places of the providers of the
/// snapshot, what Azure and Google read beside them, the local server and the most thought.
fn catalogued() -> Vec<String> {
  let snapshot: Value = serde_json::from_str(SNAPSHOT).expect("the snapshot is JSON");
  let mut names: Vec<String> = [
    "AZURE_OPENAI_BASE_URL",
    "AZURE_OPENAI_DEPLOYMENT_NAME_MAP",
    "AZURE_OPENAI_API_VERSION",
    "GOOGLE_CLOUD_PROJECT",
    "GCLOUD_PROJECT",
    "GOOGLE_CLOUD_LOCATION",
    "FURB_LOCAL_URL",
    "FURB_LOCAL_KEY",
    "FURB_LOCAL_WINDOW",
    "FURB_LOCAL_TOKENS",
    "MAX_THINKING_TOKENS",
  ]
  .map(str::to_owned)
  .to_vec();
  for provider in snapshot.as_object().into_iter().flat_map(|all| all.values()) {
    let strings = |key: &str| {
      let held = provider[key].as_array().into_iter().flatten().filter_map(Value::as_str);
      held.map(str::to_owned).collect::<Vec<_>>()
    };
    names.extend(strings("env"));
    for (place, alias) in provider["aliases"].as_object().into_iter().flatten() {
      names.extend([place.clone()].into_iter().chain(alias.as_str().map(str::to_owned)));
    }
    let text = provider.to_string();
    names
      .extend(text.split("${").skip(1).filter_map(|one| Some(one.split_once('}')?.0.to_owned())));
  }
  names.sort();
  names.dedup();
  names
}

pub(crate) use here;
