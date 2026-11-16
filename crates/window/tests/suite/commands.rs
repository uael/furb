//! The commands that are a word of his, on a real daemon: `/rename` on the root of the session,
//! and `/cd` on the chain of the open thread.

#![cfg(test)]

use std::fs;

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
  // The row of hints says the command that ran, and the card the name. The head of the thread
  // names its thread, since the window is thread first.
  shows(&app, "✓ /rename Taxes");
  let named = shows(&app, "FU furb · Taxes");
  assert!(named.starts_with("● furb · sort the receipts of the year"), "{named}");
}

#[test]
fn cd_moves_the_chain_of_the_thread_as_a_word_of_his_and_one_with_no_folder_says_what_it_takes() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  fs::create_dir_all(daemon.file("furb").join("docs")).unwrap();
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("cd.jsonl"), true);
  peer.said(&life, "read the docs");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "read the docs"));
  shows(&app, "A reply opens this thread again");
  // A space ends the completion of the command, so enter sends it.
  app.type_text("/cd ");
  app.press("enter");
  shows(&app, "Name the folder: /cd <path>.");
  app.press("ctrl+c");
  // The leader then f keeps the command once, and the word stands in the thread as his turn.
  app.press("ctrl+x");
  app.press("f");
  app.type_text("docs");
  app.press("enter");
  let moved = shows(&app, "2  cd(\"docs\")");
  assert!(!moved.contains("/cd docs"), "{moved}");
  // The foot names where the next word runs: the folder that the chain came to.
  shows(&app, "/furb/docs");
}
