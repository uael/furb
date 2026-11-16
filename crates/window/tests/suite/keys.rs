//! The keys that the old window gave, which the one table of the keys gives again beside their
//! forms after the leader. Each test feeds the bytes that a terminal with no keys of Kitty writes,
//! so it proves the parse of the terminal as well as the table.

#![cfg(test)]

use std::{fs, path::Path, process};

use furb_window::{Options, open};
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

#[test]
fn f5_with_no_thread_open_says_that_a_reload_needs_one() {
  let (app, _daemon) = opened();
  app.feed(b"\x1b[15~");
  shows(&app, "Open a thread to reload its session.");
}

#[test]
fn f5_runs_the_reload_on_the_root_of_the_session_of_the_open_thread() {
  let (app, daemon) = threaded();
  app.feed(b"\x1b[15~");
  let record = daemon.file("furb").join(".furb").join("sessions").join("keys.jsonl");
  let ran = || fs::read_to_string(&record).is_ok_and(|text| text.contains("await reload()"));
  assert!(app.wait_until(PATIENCE, ran), "{}", app.text());
}

/// A command that Go to finds by what it does, run with Enter.
fn ran(app: &TestApp, says: &str) {
  app.press("ctrl+p");
  shows(app, "Search a thread");
  app.type_text(says);
  shows(app, &capitalized(says));
  app.press("enter");
}

/// A text with its first letter in capitals, as a row of Go to writes what a command does.
fn capitalized(text: &str) -> String {
  let mut letters = text.chars();
  letters.next().map(|one| one.to_uppercase().chain(letters).collect()).unwrap_or_default()
}

#[test]
fn the_command_status_says_the_machine_the_project_the_model_and_the_build() {
  let (app, _daemon) = opened();
  ran(&app, "what the input runs on");
  // The notice starts on the row under the input, which cuts what its width does not hold.
  shows(&app, "mac · furb-try · opus at high · furb ");
}

#[test]
fn the_command_login_opens_the_sign_in_of_codex() {
  let (app, _daemon) = opened();
  ran(&app, "sign in to the models of Codex");
  shows(&app, "Sign in with ChatGPT");
}

#[test]
fn alt_e_writes_the_input_in_the_editor_of_the_operator_and_takes_back_what_it_saved() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.project("furb");
  daemon.served();
  let edit = |file: &Path| fs::write(file, format!("{} and more\n", fs::read_to_string(file)?));
  let options = Options { edit, ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  shows(&app, "opus  high");
  app.type_text("a draft");
  app.feed(b"\x1be");
  shows(&app, "a draft and more");
}

#[test]
fn the_command_export_copies_the_thread_and_with_no_thread_says_so() {
  let (app, _daemon) = opened();
  ran(&app, "copy the whole thread as Markdown");
  shows(&app, "No thread is open to copy.");
  let (app, _daemon) = threaded();
  ran(&app, "copy the whole thread as Markdown");
  shows(&app, "This terminal takes no copy.");
}

#[test]
fn the_command_delete_offers_the_trash_among_the_choices_of_the_session_and_its_choice_moves_it() {
  let (app, daemon) = threaded();
  ran(&app, "move a session to the trash");
  shows(&app, "Move it to the trash of its project");
  let sessions = daemon.file("furb").join(".furb");
  let record = sessions.join("sessions").join("keys.jsonl");
  assert!(record.exists(), "the command alone moves nothing");
  app.type_text("trash");
  app.press("enter");
  let moved = || !record.exists() && sessions.join("trash").is_dir();
  assert!(app.wait_until(PATIENCE, moved), "{}", app.text());
}

#[test]
fn a_send_keeps_what_the_operator_saw_beside_the_cards_of_the_window() {
  let (app, daemon) = threaded();
  app.type_text("does it hold");
  app.press("enter");
  let seen = daemon.file(&format!("seen-{}.json", process::id()));
  let kept = || fs::read_to_string(&seen).unwrap_or_default();
  assert!(app.wait_until(PATIENCE, || kept().contains("keys.jsonl")), "{}", kept());
  assert!(kept().contains("\"cols\":100") && kept().contains("\"rows\":30"), "{}", kept());
}

/// A window on the thread of a session that answered, open.
pub(crate) fn threaded() -> (TestApp, Daemon) {
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
