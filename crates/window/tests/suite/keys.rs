//! The keys that the old window gave, which the one table of the keys gives again beside their
//! forms after the leader. Each test feeds the bytes that a terminal with no keys of Kitty writes,
//! so it proves the parse of the terminal as well as the table.

#![cfg(test)]

use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, shows},
  daemon::{Daemon, PATIENCE},
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

/// The frame that a key of the old window gives is the frame that its twin after the leader gives:
/// the leader and its letter, then the same again to undo it, then the bytes of the old key.
fn twin(app: &TestApp, letter: &str, undo: &str, bytes: &[u8]) {
  let before = app.text();
  app.press("ctrl+x");
  app.press(letter);
  assert!(app.wait_until(PATIENCE, || app.text() != before), "{}", app.text());
  let led = app.text();
  if undo == letter {
    app.press("ctrl+x");
  }
  app.press(undo);
  assert!(app.wait_until(PATIENCE, || app.text() == before), "{}", app.text());
  app.feed(bytes);
  let same = app.wait_until(PATIENCE, || app.text() == led);
  assert!(same, "the old key did not do what its twin does:\n{}\n{led}", app.text());
}

#[test]
fn ctrl_backslash_shows_or_hides_the_list_as_the_leader_and_l_do() {
  let (app, _daemon) = opened();
  twin(&app, "l", "l", b"\x1c");
}
