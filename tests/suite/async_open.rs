//! A life that a program opens on a runtime of its own, in a process of its own, so that no other
//! test starts the reactor of the process: each test of the suite that boots a life on a thread
//! with no runtime starts it, so a child of the suite runs this one alone.
#![cfg(unix)]

use crate::yard;

#[cfg(target_os = "linux")]
use std::fs;
use std::{
  env,
  path::Path,
  process::{self, Command},
};

use furb::{
  Exit,
  life::{Opening, Progress},
  verbs,
};
use tokio::task::LocalSet;

/// The names of the threads of this process, which Linux gives in its proc file system.
#[cfg(target_os = "linux")]
fn threads() -> Vec<String> {
  let Ok(tasks) = fs::read_dir("/proc/self/task") else { return Vec::new() };
  tasks
    .filter_map(|one| fs::read_to_string(one.ok()?.path().join("comm")).ok())
    .map(|name| name.trim().to_owned())
    .collect()
}

/// A life with no model and no extension, on a directory of this test.
fn opening(at: &Path) -> Opening {
  Opening {
    claude: Some(at.join("no-claude").display().to_string()),
    directory: at.display().to_string(),
    config: Some(at.join("config").display().to_string()),
    roster: Some(Vec::new().into()),
    skills: Some(false),
    ..Opening::default()
  }
}

/// The variable that names the test that a child of the suite runs alone, as in the suite of the
/// host, and the name of this one.
const ALONE: &str = "FURB_ALONE";
const NAME: &str =
  "async_open::a_life_opens_and_runs_its_ears_on_the_runtime_of_the_thread_that_awaits_it";

#[test]
fn it_runs_alone() {
  let child = Command::new(env::current_exe().expect("the suite knows its program"))
    .args([NAME, "--exact", "--ignored", "--nocapture"])
    .env(ALONE, NAME)
    .output()
    .expect("the child of the suite starts");
  let said = String::from_utf8_lossy(&child.stdout) + String::from_utf8_lossy(&child.stderr);
  assert!(child.status.success() && said.contains("1 passed"), "{said}");
}

#[expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that tokio starts for it"
)]
#[tokio::test(flavor = "current_thread")]
#[ignore = "a child of the suite runs it alone"]
async fn a_life_opens_and_runs_its_ears_on_the_runtime_of_the_thread_that_awaits_it() {
  // Beside the other ignored tests of the suite, which share its process, it proves nothing.
  if env::var(ALONE).as_deref() != Ok(NAME) {
    return;
  }
  let at = yard::Yard::new(format!("furb-async-open-{}", process::id()));
  LocalSet::new()
    .run_until(async {
      let (mut engine, _) = opening(&at).open::<String>([], Progress::default()).await.unwrap();
      let on = || Some("chain1".to_owned());
      let with = verbs::Bash { on: on(), ..Default::default() };
      let exit: Exit = engine.bash("echo hi", with).unwrap().await.unwrap();
      assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "hi\n"));
      let waited = engine.wait(&["chain1"], verbs::Wait { seconds: Some(0.05), on: on() });
      assert_eq!(waited.unwrap().await.unwrap(), None::<String>);
    })
    .await;
  #[cfg(target_os = "linux")]
  assert!(!threads().is_empty() && !threads().contains(&"furb-reactor".to_owned()));
}
