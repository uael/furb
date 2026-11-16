//! The keys that the old window gave, which the one table of the keys gives again beside their
//! forms after the leader. Each test feeds the bytes that a terminal with no keys of Kitty writes,
//! so it proves the parse of the terminal as well as the table.

#![cfg(test)]

use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, Daemon, PATIENCE},
};

/// A window on one machine with one project and no thread yet, once its models came.
fn opened() -> (TestApp, Daemon) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.project("furb");
  daemon.served();
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  // The models of the machine come at any moment and take their room in the launcher.
  shows(&app, "opus  high");
  (app, daemon)
}

#[test]
fn f1_shows_each_key_and_the_list_names_it_beside_the_leader() {
  let (app, _daemon) = opened();
  app.feed(b"\x1bOP");
  shows(&app, "Search a key");
  shows(&app, "⌃X ? or F1");
}

/// A window on the thread of a session that answered, open.
fn threaded() -> (TestApp, Daemon) {
  let (app, mut daemon) = opened();
  let sessions = daemon.project("furb");
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("keys.jsonl"), true);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "The open takes");
  (app, daemon)
}

/// The frame that a key of the old window gives is the frame that its twin after the leader gives:
/// the bytes of the old key change the frame, up to a text that shows once the change came whole,
/// then keys undo it and the twin does it again.
fn twin(app: &TestApp, bytes: &[u8], whole: &str, again: &[&str]) {
  let before = app.text();
  app.feed(bytes);
  let changed = app.wait_until(PATIENCE, || app.text() != before && app.text().contains(whole));
  assert!(changed, "{}", app.text());
  let old = app.text();
  for key in again {
    app.press(key);
  }
  let same = app.wait_until(PATIENCE, || app.text() == old);
  assert!(same, "the twin did not do what the old key does:\n{}\n{old}", app.text());
}

#[test]
fn ctrl_backslash_shows_or_hides_the_list_as_the_leader_and_l_do() {
  let (app, _daemon) = opened();
  twin(&app, b"\x1c", "", &["ctrl+x", "l", "ctrl+x", "l"]);
}

#[test]
fn ctrl_b_shows_the_chains_of_the_life_as_the_leader_and_t_do() {
  let (app, _daemon) = threaded();
  // The chains come from the machine after the bar shows.
  twin(&app, b"\x02", "no chain works now", &["ctrl+x", "t", "ctrl+x", "t"]);
}
