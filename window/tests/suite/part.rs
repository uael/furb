//! The list part over the window: Go to, the commands and each key, on a real daemon whose lives a
//! peer of the test drives.

#![cfg(test)]

use std::fs;

use furb_window::{Options, open};
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, shows},
  daemon::{ANSWERS, Daemon, Hold, PATIENCE, Peer, WORKS},
  screens::screen,
  thread::clicked,
};

/// A window on a machine with a session that works, a session at rest whose thread the model
/// answered, and a session that holds no thread of the operator.
pub(crate) fn machine() -> (TestApp, Daemon, Peer) {
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
  // The list stands once it holds the card of each thread, so the frames of the part show it as
  // it stays. The session that holds no thread of his has no card.
  shows(&app, "Resolved 1");
  shows(&app, "Working 1");
  (app, daemon, peer)
}

/// Whether the window shows a text no more, once a frame drew its change.
fn gone(app: &TestApp, text: &str) -> bool {
  app.wait_until(PATIENCE, || !app.text().contains(text))
}

#[test]
fn go_to_lists_the_threads_that_work_and_the_projects_and_finds_a_session_with_no_thread_of_his() {
  let (app, _daemon, _peer) = machine();
  app.press("ctrl+o");
  let frame = shows(&app, "Search a thread, a session, a project");
  assert!(frame.contains("Working") && frame.contains("Projects"), "{frame}");
  assert!(!frame.contains("setup-only"), "a session shows only once he searches: {frame}");
  // The project counts the session with no thread once the machine listed its records.
  shows(&app, "3 sessions");
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
  clicked(&app, "Show or hide the list", 0, 0);
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

#[test]
fn a_click_on_the_scope_of_the_list_opens_go_to() {
  let (app, _daemon, _peer) = machine();
  clicked(&app, "All projects ▾", 0, 0);
  shows(&app, "Search a thread, a session, a project");
}

#[test]
fn a_click_on_the_project_of_a_new_session_lists_the_projects_and_one_aims_the_input() {
  let (app, _daemon, _peer) = machine();
  clicked(&app, "furb-try ▾", 0, 0);
  let frame = shows(&app, "Search a project");
  assert!(!frame.contains("Send to"), "{frame}");
  // The project of the daemon is a row of the part, with its count of sessions.
  shows(&app, "3 sessions");
  let (x, y) = cell(&app, "FU furb", "3 sessions");
  app.click(x, y);
  shows(&app, "FU furb ▾");
}

/// A window in a project with no session, in a folder that stands, on a machine whose catalog
/// offers the models of claude, once its launcher shows the default actor of that catalog.
#[cfg(unix)]
fn offered() -> (TestApp, Daemon) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.catalog();
  daemon.served();
  let folder = daemon.file("furb-try");
  fs::create_dir_all(&folder).unwrap();
  let options = Options { folder, ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  shows(&app, "opus  high");
  (app, daemon)
}

#[cfg(unix)]
#[test]
fn a_click_on_the_model_lists_the_models_of_each_provider_and_one_is_the_model_of_the_next_session()
{
  let (app, _daemon) = offered();
  clicked(&app, "opus  high", 0, 0);
  let frame = shows(&app, "Search a model");
  assert!(frame.contains("Claude") && frame.contains("sonnet"), "{frame}");
  let (x, y) = cell(&app, "sonnet", "high");
  app.click(x, y);
  shows(&app, "sonnet  high");
  shows(&app, "with sonnet at high effort.");
  // The session that his message starts stands on that model: its block names it.
  app.paste(&format!("how long does an open take {ANSWERS}"));
  app.press("enter");
  shows(&app, "sonnet/high");
}

#[cfg(unix)]
#[test]
fn a_model_chosen_on_a_thread_answers_its_next_message() {
  let (app, _daemon) = offered();
  app.paste(&format!("how long does an open take {ANSWERS}"));
  app.press("enter");
  shows(&app, "Two stages hold most");
  clicked(&app, "opus  high", 0, 0);
  shows(&app, "Search a model");
  let (x, y) = cell(&app, "sonnet", "high");
  app.click(x, y);
  shows(&app, "sonnet  high");
  app.paste(&format!("and the seed {ANSWERS}"));
  app.press("enter");
  shows(&app, "sonnet/high");
}

#[cfg(unix)]
#[test]
fn a_click_on_the_effort_takes_the_next_effort_of_the_model() {
  let (app, _daemon) = offered();
  // The home names the effort in its words first, at the left of the launcher.
  clicked(&app, "high", 40, 0);
  let frame = shows(&app, "with opus at xhigh effort.");
  assert!(frame.contains("opus  xhigh"), "{frame}");
}

/// The first cell of a text in the first row of the window that holds another text too.
fn cell(app: &TestApp, text: &str, beside: &str) -> (u16, u16) {
  let frame = app.text();
  let found = frame.lines().enumerate().find_map(|(y, line)| {
    let at = line.find(text).filter(|_| line.contains(beside))?;
    Some((line[..at].chars().count(), y))
  });
  let (x, y) = found.unwrap_or_else(|| panic!("no row holds {text:?} beside {beside:?}:\n{frame}"));
  (u16::try_from(x).unwrap(), u16::try_from(y).unwrap())
}

#[test]
fn a_new_session_on_the_model_of_the_last_card_of_its_project_shows_its_default_effort() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  peer.rested(
    &sessions.join("gate.jsonl"),
    &format!("remove the gate\n{ANSWERS}"),
    Some(Hold::Seen),
  );
  let folder = daemon.file("furb");
  let options = Options { folder, ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  shows(&app, "Resolved 1");
  // The card names its model alone; the launcher names the effort that a new session takes.
  shows(&app, "FU ▾   opus  high");
}

#[test]
fn a_slash_that_starts_the_input_offers_the_commands_and_an_escape_hides_them() {
  let (app, _daemon, _peer) = machine();
  app.type_text("/li");
  shows(&app, "Commands and skills");
  shows(&app, "Show or hide the list");
  // An escape hides the list and keeps the slash, so a message can start with one.
  app.press("escape");
  assert!(gone(&app, "Commands and skills"), "{}", app.text());
  app.type_text("st holds the logs");
  shows(&app, "┃ /list holds the logs");
  assert!(!app.text().contains("Commands and skills"), "{}", app.text());
}
