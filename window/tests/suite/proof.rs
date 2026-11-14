//! The proof screen of the base: the window dials a daemon and says what that daemon told.

#![cfg(test)]

use std::time::Duration;

use furb_window::{open, tokens::Loader};
use opentui::testing::TestApp;

use crate::{daemon::Daemon, screens::screen};

/// How long a test waits for a daemon before it fails.
const PATIENCE: Duration = Duration::from_secs(60);

#[test]
fn a_window_shows_a_loader_while_it_dials_then_the_machine_and_what_its_daemon_told() {
  let app = TestApp::new(100, 30);
  let mut daemon = Daemon::bound(&app);
  daemon.records(3);
  open(&app, daemon.machine());
  screen("dials", &app);
  daemon.served(&app);
  assert!(app.wait_until(PATIENCE, || app.text().contains("records")), "{}", app.text());
  screen("machine", &app);
}

#[test]
fn only_the_cell_of_the_loader_is_drawn_again_while_the_window_dials() {
  let app = TestApp::new(100, 30);
  let daemon = Daemon::bound(&app);
  open(&app, daemon.machine());
  let before = app.render();
  app.advance(Loader::Model.step());
  let after = app.render();
  let cells = before.cells.iter().zip(&after.cells).enumerate();
  let changed: Vec<_> = cells.filter(|(_, (was, is))| was != is).map(|(at, _)| at).collect();
  // The loader stands in row 2, at the column where a text starts.
  assert_eq!(changed, [2 * 100 + 2]);
  assert_eq!(
    (before.row(2).trim(), after.row(2).trim()),
    ("⠋ Dials the daemon of test-machine", "⠙ Dials the daemon of test-machine")
  );
}
