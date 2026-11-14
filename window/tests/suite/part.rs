//! The list part over the window: Go to, the commands and each key, on a real daemon whose lives a
//! peer of the test drives.

#![cfg(test)]

use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, shows},
  daemon::{ANSWERS, Daemon, Hold, PATIENCE, Peer, WORKS},
  screens::screen,
  thread::{at, lights},
};

/// A window on a machine with a session that works, a session at rest whose thread the model
/// answered, and a session that holds no thread of the operator.
fn machine() -> (TestApp, Daemon, Peer) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let works = peer.opened(&sessions.join("bench.jsonl"), true);
  peer.said(&works, &format!("run the bench of replay\n{WORKS}"));
  // He read the thread of the session at rest, so its card stands with the resolved ones.
  let gate = sessions.join("gate.jsonl");
  peer.rested(&gate, &format!("remove the gate\n{ANSWERS}"), Some(Hold::Seen));
  peer.opened(&sessions.join("setup-only.jsonl"), true);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  // The list stands once it holds the card of each session, so the frames of the part show it as
  // it stays.
  shows(&app, "Resolved 1");
  shows(&app, "Resting 1");
  (app, daemon, peer)
}

/// Whether the window shows a text no more, once a frame drew its change.
fn gone(app: &TestApp, text: &str) -> bool {
  app.wait_until(PATIENCE, || !app.text().contains(text))
}

#[test]
fn go_to_lists_what_works_and_the_projects_and_finds_a_session_with_no_thread_of_his() {
  let (app, _daemon, _peer) = machine();
  app.press("ctrl+o");
  let frame = shows(&app, "Search a thread, a session, a project");
  assert!(frame.contains("Sessions that work") && frame.contains("Projects"), "{frame}");
  assert!(!frame.contains("setup-only"), "a session shows only once he searches: {frame}");
  screen("part-go-to", &app);
  // The arrows move the chosen row, and letters filter the rows as he types.
  app.press("down");
  shows(&app, "2 / 2");
  app.type_text("setup");
  let frame = shows(&app, "▌setup");
  assert!(frame.contains("setup-only") && frame.contains("1 / 1"), "{frame}");
  // Enter takes it: a session with no thread of his turns the input to a new thread of it.
  app.press("enter");
  assert!(gone(&app, "Search a thread"), "{}", app.text());
  shows(&app, "┃ Ask in a new thread");
  // A project takes its sessions as the filter, and escape closes the part with no choice.
  app.press("ctrl+o");
  shows(&app, "Projects");
  app.type_text("furb");
  shows(&app, "3 sessions");
  app.press("escape");
  assert!(gone(&app, "Search a thread"), "{}", app.text());
}

#[test]
fn the_commands_run_from_their_list_and_each_key_shows_its_forms() {
  let (app, _daemon, _peer) = machine();
  app.press("ctrl+p");
  let frame = shows(&app, "Search a command");
  assert!(frame.contains("Show or hide the list") && frame.contains("⌃X L"), "{frame}");
  screen("part-commands", &app);
  // A row takes a click, and its text lights under the pointer.
  let (x, y) = at(&app, "Show or hide the list", 0);
  lights(&app, x, y);
  app.click(x, y);
  assert!(gone(&app, "All projects"), "the list hid: {}", app.text());
  app.press("ctrl+x");
  app.press("?");
  let frame = shows(&app, "Search a key");
  assert!(frame.contains("⌥↓ or ⌃X J") && frame.contains("the next card"), "{frame}");
  screen("part-keys", &app);
  // A click beside the part closes it.
  app.click(1, 1);
  assert!(gone(&app, "Search a key"), "{}", app.text());
}
