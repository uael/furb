//! A life that a program opens on a runtime of its own, in a binary of its own, so that no other test starts the
//! reactor of the process.
#![cfg(unix)]

use std::{fs, path::Path};

use furb::{Exit, life::Opening, verbs};
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
    directory: at.display().to_string(),
    config: Some(at.join("config").display().to_string()),
    roster: Some(Vec::new()),
    extensions: Some(false),
    ..Opening::default()
  }
}

#[expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that tokio starts for it"
)]
#[tokio::test(flavor = "current_thread")]
async fn a_life_opens_and_runs_its_ears_on_the_runtime_of_the_thread_that_awaits_it() {
  let at = std::env::temp_dir().join(format!("furb-async-open-{}", std::process::id()));
  if at.exists() {
    fs::remove_dir_all(&at).unwrap();
  }
  fs::create_dir_all(&at).unwrap();
  LocalSet::new()
    .run_until(async {
      let (mut engine, _) = opening(&at).open::<String>([]).await.unwrap();
      let on = || Some("chain1".to_owned());
      let with = verbs::Bash { on: on(), ..Default::default() };
      let exit: Exit = engine.bash("echo hi", with).unwrap().await.unwrap();
      assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "hi\n"));
      let waited = engine.wait(&[], verbs::Wait { seconds: Some(0.05), on: on() });
      assert_eq!(waited.unwrap().await.unwrap(), None::<String>);
    })
    .await;
  #[cfg(target_os = "linux")]
  assert!(!threads().is_empty() && !threads().contains(&"furb-reactor".to_owned()));
  fs::remove_dir_all(&at).unwrap();
}
