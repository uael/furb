//! The name of a session, which the operator gives with `/rename`, on a real daemon.

#![cfg(test)]

use furb_window::open;

use crate::{
  cards::{app, options, row, shows},
  daemon::Daemon,
};

#[test]
fn rename_names_the_session_as_a_word_of_his_on_its_root_and_the_window_shows_the_name() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("rename.jsonl"), true);
  peer.said(&life, "sort the receipts of the year");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "sort the receipt"));
  shows(&app, "sort the receipts of the year");
  // The leader then n puts the command in the input, with the name that the session has now.
  app.press("ctrl+x");
  app.press("n");
  shows(&app, "/rename");
  app.type_text("Taxes");
  app.press("enter");
  // The row of hints says the command that ran, and the head of the thread and its card the name.
  shows(&app, "✓ /rename Taxes");
  let named = shows(&app, "FU furb · Taxes");
  assert!(named.starts_with("● furb · Taxes"), "{named}");
}
