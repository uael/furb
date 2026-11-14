//! The open of a session at rest from its card, on a real daemon whose model answers from a script.
//! A test holds an open at the catalog of its models until it makes a file, so the clock of the
//! window, which the test moves, says how long the open took.

#![cfg(test)]

use std::{
  fs::File,
  path::{Path, PathBuf},
  time::Duration,
};

use furb_host::protocol::{Answer, Verb};
use furb_window::{Options, open};
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, Daemon, PATIENCE},
  screens::screen,
};

// What only the tests of a session at rest use, which run on unix alone.
#[cfg(unix)]
use {
  crate::thread::{at, lights},
  opentui::style::Color,
  std::fs,
};

/// The columns of the left side of the window, at the left of the list.
const LEFT: usize = 61;

/// The message of the thread of a session at rest, whose first line is its title.
fn message(title: &str) -> String {
  format!("{title}\n{ANSWERS}")
}

/// The options of a window in the project of its daemon, whose home folder is the folder of that
/// daemon, so a path of a record shows the same in each run.
fn homed(daemon: &Daemon) -> Options {
  let (folder, home) = (daemon.file("furb"), daemon.file("furb").parent().map(PathBuf::from));
  Options { folder, home, ..options(&daemon.file("cards.json")) }
}

/// A machine with sessions at rest, one for each title, whose model answered the one thread of
/// each: its daemon, which holds each open that a window asks, and the file that lets the opens
/// go. The window shows their cards.
#[cfg(unix)]
fn resting(app: &TestApp, titles: &[&str]) -> (Daemon, PathBuf) {
  let mut daemon = Daemon::bound(app);
  daemon.project("furb");
  let go = daemon.holding();
  daemon.served();
  let mut peer = daemon.peer();
  for (at, title) in titles.iter().enumerate() {
    peer.rested(&record(&daemon, at), &message(title), None);
  }
  open(app, vec![daemon.machine("mac")], homed(&daemon));
  // The daemon ends once it holds no peer and no life, so the peer of the test goes only once the
  // window holds the daemon and its cards.
  for title in titles {
    row(app, &title[..15]);
  }
  drop(peer);
  (daemon, go)
}

/// The record of a session at rest of a machine, by its place among the titles of the machine.
#[cfg(unix)]
fn record(daemon: &Daemon, at: usize) -> PathBuf {
  daemon.file("furb").join(".furb").join("sessions").join(format!("rest{at}.jsonl"))
}

/// The card of a thread clicked.
fn clicked(app: &TestApp, title: &str) {
  app.click(80, row(app, &title[..15]));
}

/// The rows of the left side of a frame.
fn left(app: &TestApp) -> Vec<String> {
  app.text().lines().map(|line| line.chars().take(LEFT).collect::<String>()).collect()
}

#[test]
fn a_session_whose_daemon_ended_opens_in_a_window_of_a_new_daemon_and_shows_the_same_thread() {
  let title = "remove the gate from the engine";
  let first = app();
  let mut daemon = Daemon::bound(&first);
  let sessions = daemon.project("furb");
  daemon.served();
  let record = sessions.join("gate.jsonl");
  let mut peer = daemon.peer();
  peer.rested(&record, &message(title), None);
  open(&first, vec![daemon.machine("mac")], homed(&daemon));
  clicked(&first, title);
  shows(&first, "Two stages hold most");
  let before = left(&first);
  // The daemon ends with the app whose runtime serves it, and its lives with it.
  drop((peer, first));
  let second = app();
  daemon.again(&second);
  daemon.served();
  let free = || File::open(lease(&record)).is_ok_and(|lease| lease.try_lock().is_ok());
  assert!(second.wait_until(PATIENCE, free), "the life of the first daemon keeps its record");
  open(&second, vec![daemon.machine("mac")], homed(&daemon));
  clicked(&second, title);
  shows(&second, "Two stages hold most");
  assert_eq!(left(&second), before);
  screen("open-stood", &second);
}

/// The file whose lock is the lease of a record.
fn lease(record: &Path) -> PathBuf {
  record.with_extension("jsonl.lock")
}

#[cfg(unix)]
#[test]
fn an_open_that_ends_under_400_ms_shows_no_bar_and_its_page_takes_the_rows_of_the_card() {
  let title = "remove the gate from the engine";
  let app = app();
  let (_daemon, go) = resting(&app, &[title]);
  clicked(&app, title);
  // The first frame shows the head, and the message of the thread as its card holds it.
  let frame = shows(&app, &format!("┃ {title}"));
  assert!(!frame.contains("Opening"), "{frame}");
  screen("open-first", &app);
  app.advance(Duration::from_millis(399));
  File::create(&go).unwrap();
  let mut bar = false;
  let page = app.wait_until(PATIENCE, || {
    let frame = app.text();
    bar |= frame.contains("Opening");
    frame.contains("Two stages hold most")
  });
  assert!(page && !bar, "{}", app.text());
}

#[cfg(unix)]
#[test]
fn the_head_names_the_stage_of_an_open_and_counts_its_seconds_then_shows_each_stage_after_5_s() {
  let title = "remove the gate from the engine";
  let app = app();
  let (daemon, go) = resting(&app, &[title]);
  clicked(&app, title);
  shows(&app, &format!("┃ {title}"));
  app.advance(Duration::from_millis(400));
  shows(&app, "Opening · reads the record");
  app.advance(Duration::from_millis(600));
  shows(&app, "1.0 s");
  // The record is read whole, and the open waits for the catalog: the card says that it opens,
  // with no share.
  shows(&app, "⠹ Opening▌");
  screen("open-bar", &app);
  app.advance(Duration::from_secs(4));
  let frame = shows(&app, "5.0 s");
  // The row of the open, then the row of its one stage, which counts the bytes of the record. The
  // record holds the paths of the folder of the test, so its size differs on each system, and the
  // rows stand here in place of a capture.
  let bytes = fs::metadata(record(&daemon, 0)).unwrap().len();
  let rows: Vec<String> = left(&app)[1..3].iter().map(|row| row.trim_end().to_owned()).collect();
  let size = format!("{} KB", (bytes + 500) / 1000);
  let stage = format!("    ⠹ reads the record{size:>27}     5.0 s");
  assert_eq!(rows, [format!("  Opening · reads the record{:>31}", "5.0 s"), stage], "{frame}");
  // Past its bound of 15 seconds, the row of the open takes the color of attention.
  app.advance(Duration::from_secs(11));
  let frame = shows(&app, "16 s, over 15 s");
  let opening = app.render();
  let (x, y) = at(&app, "Opening ·", 0);
  let cell = usize::from(y) * opening.width as usize + usize::from(x);
  assert_eq!(opening.cells[cell].fg, Color::hex(0xf5_a742), "{frame}");
  // The bar goes when the page comes, with no time of its own.
  File::create(&go).unwrap();
  let frame = shows(&app, "Two stages hold most");
  assert!(!frame.contains("Opening"), "{frame}");
}

#[cfg(unix)]
#[test]
fn a_message_written_while_the_session_opens_goes_once_it_stands_and_gets_its_answer() {
  let title = "remove the gate from the engine";
  let app = app();
  let (_daemon, go) = resting(&app, &[title]);
  clicked(&app, title);
  app.advance(Duration::from_secs(1));
  let frame = shows(&app, "┃ Write while it opens. Your message goes when it is open.");
  assert!(frame.contains("⎋ back"), "{frame}");
  app.paste(&format!("and the replay {ANSWERS}"));
  app.press("enter");
  shows(&app, "┃ and the replay");
  File::create(&go).unwrap();
  shows(&app, "3  found = 2");
}

#[cfg(unix)]
#[test]
fn an_escape_goes_back_while_it_opens_and_the_same_card_joins_the_same_open() {
  let (gate, time) = ("remove the gate from the engine", "where does the time of a resume go");
  let app = app();
  let (_daemon, go) = resting(&app, &[gate, time]);
  clicked(&app, gate);
  app.advance(Duration::from_secs(1));
  shows(&app, "1.0 s");
  app.press("escape");
  let frame = shows(&app, "go to a project");
  assert!(frame.contains("Opening"), "the open goes on:\n{frame}");
  // A second open runs beside the first.
  clicked(&app, time);
  app.advance(Duration::from_secs(1));
  let frame = shows(&app, &format!("furb · {}", &time[..20]));
  // The head of the thread, and the card of each open.
  assert_eq!(frame.matches(" Opening").count(), 3, "{frame}");
  app.press("escape");
  // Each record is read whole once the catalog holds its open, so no card shows a share.
  let read = app.wait_until(PATIENCE, || !app.text().contains('%'));
  assert!(read, "{}", app.text());
  screen("open-home", &app);
  // The first card again: its open counts from its first ask.
  clicked(&app, gate);
  shows(&app, "2.0 s");
  File::create(&go).unwrap();
  shows(&app, "Two stages hold most");
}

#[cfg(unix)]
#[test]
fn an_open_that_fails_says_why_and_how_to_go_back_and_loses_no_draft() {
  let title = "remove the gate from the engine";
  let app = app();
  let (daemon, go) = resting(&app, &[title]);
  // Another holder owns the lease of the record.
  let owner = File::create(lease(&record(&daemon, 0))).unwrap();
  owner.try_lock().unwrap();
  clicked(&app, title);
  app.advance(Duration::from_secs(1));
  app.paste("and the replay");
  app.press("enter");
  shows(&app, "┃ and the replay");
  app.paste("a draft");
  File::create(&go).unwrap();
  let frame = shows(&app, "Another process owns");
  assert!(frame.contains("The record stays as it was. ⎋ goes back."), "{frame}");
  assert!(!frame.contains("Opening"), "{frame}");
  let frame = shows(&app, "┃ and the replay");
  assert!(frame.contains("┃ a draft"), "{frame}");
  // The machine says that no life holds the record: the card rests again.
  shows(&app, "● now");
  screen("open-failed", &app);
  // The way back takes a click, as its key does, and lights under the pointer.
  let (x, y) = at(&app, "⎋ back", 0);
  lights(&app, x, y);
  app.click(x, y);
  shows(&app, "go to a project");
}

#[test]
fn a_session_that_a_life_holds_opens_with_no_bar_and_its_other_window_goes_on() {
  let title = "remove the gate from the engine";
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("gate.jsonl"), true);
  let prompt = peer.said(&life, &message(title));
  open(&app, vec![daemon.machine("mac")], homed(&daemon));
  clicked(&app, title);
  app.advance(Duration::from_secs(1));
  let frame = shows(&app, "Two stages hold most");
  assert!(!frame.contains("Opening"), "{frame}");
  // The other window of the life says one more message in the thread, which this one shows.
  let message = format!("and the replay {ANSWERS}");
  let reply = Verb::Prompt { life, shape: "None".to_owned(), message, to: None, on: Some(prompt) };
  assert!(matches!(peer.ask(reply), Answer::Act(_)));
  shows(&app, "┃ and the replay");
  shows(&app, "3  found = 2");
}
