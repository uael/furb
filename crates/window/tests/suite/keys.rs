//! The keys that the old window gave, which the one table of the keys gives again beside their
//! forms after the leader. Each test feeds the bytes that a terminal with no keys of Kitty writes,
//! so it proves the parse of the terminal as well as the table.

#![cfg(test)]

use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, shows},
  daemon::Daemon,
};

/// A window on one machine with one project and no thread yet.
fn opened() -> (TestApp, Daemon) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.project("furb");
  daemon.served();
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  shows(&app, "This computer");
  (app, daemon)
}

#[test]
fn f1_shows_each_key_and_the_list_names_it_beside_the_leader() {
  let (app, _daemon) = opened();
  app.feed(b"\x1bOP");
  shows(&app, "Search a key");
  shows(&app, "⌃X ? or F1");
}
