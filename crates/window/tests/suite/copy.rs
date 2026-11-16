//! The copy of what a thread said last, which the operator takes with `/copy`, on a real daemon.
//! The clipboard of the system and of the terminal stand outside a test: a window with no terminal
//! takes no copy, and says so, where a terminal says that it copied.

#![cfg(test)]

use furb_window::open;

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, Daemon},
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
  peer.said(&life, "say nothing");
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
