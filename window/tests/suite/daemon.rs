//! The daemon of a test.

#![cfg(test)]
#![expect(clippy::disallowed_methods, reason = "a test blocks on the runtime of its app")]

use std::{
  fs,
  path::Path,
  process,
  sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
  },
};

use furb::{
  Object,
  life::{Answer, Opening},
};
use furb_host::{
  GRACE, STALL, Settings, run,
  transport::{Listener, Local},
};
use furb_window::Machine;
use opentui::testing::TestApp;
use serde_json::json;
use tokio::process::Command;

use crate::yard::Yard;

/// A daemon in a folder of one test, which goes with it. The runtime of the app of the test serves
/// it, so it ends with that app.
pub(crate) struct Daemon {
  yard: Yard,
  local: Local,
  /// The transport that listens, until the daemon is served on it.
  listener: Option<Listener>,
}

impl Daemon {
  /// A daemon that listens and answers no peer until it is served: a window that dials it stays in
  /// its dial for as long as the test needs.
  pub(crate) fn bound(app: &TestApp) -> Self {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    // A short name, since the path of a socket has a bound.
    let yard = Yard::new(format!("fw{}-{n}", process::id()));
    let local = Local::new(yard.join("host"));
    let listener = app.runtime().block_on(local.listen()).expect("the folder of the test is free");
    Self { yard, local, listener: Some(listener) }
  }

  /// One project of the machine with a count of records, which the daemon lists.
  pub(crate) fn records(&self, count: usize) {
    let project = self.yard.join("project");
    let sessions = project.join(".furb").join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    for n in 0..count {
      fs::write(sessions.join(format!("{n}.jsonl")), "").unwrap();
    }
    let list = json!({ "workspaces": [{ "directory": project.display().to_string() }] });
    fs::create_dir_all(self.yard.join("config")).unwrap();
    fs::write(self.yard.join("config").join("workspaces.json"), list.to_string()).unwrap();
  }

  /// The daemon served, on the runtime of the app.
  pub(crate) fn served(&mut self, app: &TestApp) {
    let listener = self.listener.take().expect("a daemon is served once");
    let settings = Settings {
      opening: scripted(&self.yard),
      grace: GRACE,
      stall: STALL,
      tailnet: None,
      door: None,
      host: None,
      heap: None,
    };
    let local = self.local.clone();
    drop(app.runtime().spawn(async move { run(&local, listener, settings).await }));
  }

  /// The machine of the daemon, as a window dials it. Its daemon listens, so the window never runs
  /// the command that serves it.
  pub(crate) fn machine(&self) -> Machine {
    let folder = self.yard.join("host");
    Machine { name: "test-machine".to_owned(), folder, served: Command::new("furb") }
  }
}

/// The opening of each life of the daemon of a test: the config of the test, no skill, and a model
/// that closes each prompt with None, with no command line of this machine.
fn scripted(at: &Path) -> Opening {
  let answer = Answer(Arc::new(|_, _| {
    let turn = ["assistant", "close(None)"].map(Object::string).into_iter();
    Box::pin(async move { Ok(Object::tuple(turn.chain([Object::none(), Object::none()]))) })
  }));
  Opening {
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    claude: Some(at.join("no-claude").display().to_string()),
    skills: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}
