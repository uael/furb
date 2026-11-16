//! The copy of what a thread said last, which the operator takes with `/copy`, and of a text that
//! he selects with the pointer, on a real daemon.
//! The clipboard of the system and of the terminal stand outside a test: a window with no terminal
//! takes no copy, and says so, where a terminal says that it copied.

#![cfg(test)]

use furb_window::open;

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, Daemon},
  thread::at,
};

#[test]
fn copy_with_no_terminal_says_that_nothing_was_copied_and_on_a_thread_that_said_nothing_says_so() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("copy.jsonl"), true);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  // The thread that says nothing stands in a session of its own: the model of a test reads its
  // whole chain, so on the chain of the first it would answer as the first asks.
  let quiet = peer.opened(&sessions.join("quiet.jsonl"), true);
  peer.said(&quiet, "say nothing");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "The open takes");
  app.press("ctrl+x");
  app.press("i");
  shows(&app, "This terminal takes no copy.");
  app.click(80, row(&app, "say nothing"));
  shows(&app, "A reply opens this thread again");
  app.press("ctrl+x");
  app.press("i");
  shows(&app, "Nothing to copy yet.");
}

#[test]
fn a_drag_over_the_feed_selects_its_text_and_copies_it_at_once_with_a_notice() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("select.jsonl"), true);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "The open takes");
  let (x, y) = at(&app, "The open takes", 0);
  app.drag((x, y), (x + 13, y));
  assert_eq!(app.app().selected_text().as_deref(), Some("The open takes"));
  // A window with no terminal takes no copy, and says so, as a terminal says that it copied.
  shows(&app, "This terminal takes no copy.");
}
