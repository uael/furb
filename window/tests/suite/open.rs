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
use furb_window::{Options, open, tokens::AWAY};
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, Daemon, PATIENCE},
  screens::screen,
};

// What only the tests of a session at rest use, which run on unix alone.
#[cfg(unix)]
use {
  crate::{
    cards::CLOCK,
    daemon::aged,
    thread::{at, clicked},
  },
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
/// go. The window shows their cards, each record a second newer than the one before it, so the
/// list gives them one order whatever the second at which each life ended.
#[cfg(unix)]
fn resting(app: &TestApp, titles: &[&str]) -> (Daemon, PathBuf) {
  let mut daemon = Daemon::bound(app);
  daemon.project("furb");
  let go = daemon.holding();
  daemon.served();
  let mut peer = daemon.peer();
  for (at, title) in titles.iter().enumerate() {
    let life = peer.rested(&record(&daemon, at), &message(title), None);
    aged(&life, CLOCK, u64::try_from(titles.len() - at).unwrap());
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
fn card(app: &TestApp, title: &str) {
  app.click(80, row(app, &title[..15]));
}

/// The rows of the left side of a frame, with no blank at their end, which the list beside them
/// pads.
fn left(app: &TestApp) -> Vec<String> {
  let left = |line: &str| line.chars().take(LEFT).collect::<String>().trim_end().to_owned();
  app.text().lines().map(left).collect()
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
  card(&first, title);
  shows(&first, "Two stages hold most");
  // The window says that he read the end of the thread, and the record keeps it once its card
  // tells it.
  first.advance(AWAY);
  shows(&first, "Resolved 1");
  let before = left(&first);
  // The daemon ends with the app whose runtime serves it, and its lives with it.
  drop((peer, first));
  let second = app();
  daemon.again(&second);
  daemon.served();
  let free = || File::open(lease(&record)).is_ok_and(|lease| lease.try_lock().is_ok());
  assert!(second.wait_until(PATIENCE, free), "the life of the first daemon keeps its record");
  open(&second, vec![daemon.machine("mac")], homed(&daemon));
  // So the thread stands with the resolved ones.
  second.click(80, row(&second, "Resolved 1"));
  card(&second, title);
  shows(&second, "Two stages hold most");
  assert_eq!(left(&second), before);
  screen("open-stood", &second);
}

#[test]
fn a_window_dials_its_machine_again_after_a_handover_and_its_thread_comes_back_where_he_read() {
  let title = "remove the gate from the engine";
  let (app, host, upgraded) = (app(), app(), app());
  let mut daemon = Daemon::bound(&host);
  let sessions = daemon.project("furb");
  daemon.served();
  let record = sessions.join("gate.jsonl");
  let mut peer = daemon.peer();
  peer.rested(&record, &message(title), None);
  open(&app, vec![daemon.machine("mac")], homed(&daemon));
  card(&app, title);
  shows(&app, "Two stages hold most");
  app.advance(AWAY);
  shows(&app, "Resolved 1");
  let before = left(&app);
  // A daemon of a new build takes the place of the one that runs, which closes each window: the
  // window dials the machine again, and the life of its thread opens there again.
  daemon.upgraded(&upgraded);
  assert!(app.wait_until(PATIENCE, || daemon.watched()), "{}", app.text());
  shows(&app, "this machine");
  assert_eq!(left(&app), before);
}

/// While the machine is away, the input keeps his words, and they go once he sends them after it
/// answers again. The end of the runtime that serves a daemon closes its peers on unix; the test
/// of a handover proves the return of the window on each system.
#[cfg(unix)]
#[test]
fn while_the_machine_is_away_the_input_keeps_his_words_until_it_answers() {
  let title = "remove the gate from the engine";
  let (app, host, again) = (app(), app(), app());
  let mut daemon = Daemon::bound(&host);
  let sessions = daemon.project("furb");
  daemon.served();
  let record = sessions.join("gate.jsonl");
  let mut peer = daemon.peer();
  peer.rested(&record, &message(title), None);
  open(&app, vec![daemon.machine("mac")], homed(&daemon));
  card(&app, title);
  shows(&app, "Two stages hold most");
  // The daemon ends with the app whose runtime serves it, and its lives with it.
  drop((peer, host));
  shows(&app, "mac offline");
  app.paste("and the seed?");
  app.press("enter");
  let frame = shows(&app, "mac is away, so nothing went");
  assert!(frame.contains("┃ and the seed?") && frame.contains("Two stages hold most"), "{frame}");
  let free = || File::open(lease(&record)).is_ok_and(|lease| lease.try_lock().is_ok());
  assert!(app.wait_until(PATIENCE, free), "the life of the first daemon keeps its record");
  daemon.back(&again);
  daemon.served();
  shows(&app, "this machine");
  app.press("enter");
  let mut peer = daemon.peer();
  let life = record.display().to_string();
  let said = || {
    let Answer::Cards(cards) = peer.ask(Verb::Cards) else { return false };
    let Some(card) = cards.into_iter().find(|card| card.life == life) else { return false };
    let page =
      Verb::Thread { life: life.clone(), thread: card.thread.prompt, before: None, limit: 5 };
    // The card can stand before the life that the window opens again lives, which refuses a page.
    matches!(peer.tried(page), Ok(Answer::Page(page)) if page.exchanges.iter().any(|one| one.text == "and the seed?"))
  };
  assert!(app.wait_until(PATIENCE, said), "{}", app.text());
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
  card(&app, title);
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
  card(&app, title);
  shows(&app, &format!("┃ {title}"));
  app.advance(Duration::from_millis(400));
  shows(&app, "Opening · reads the record");
  app.advance(Duration::from_millis(600));
  shows(&app, "1.0 s");
  // The record is read whole, and the open waits for the catalog: the card says that it opens,
  // with no share.
  let whole = |frame: String| frame.lines().any(|line| line.trim_end().ends_with("⠹ Opening ▌"));
  assert!(app.wait_until(PATIENCE, || whole(app.text())), "{}", app.text());
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
  card(&app, title);
  app.advance(Duration::from_secs(1));
  let frame = shows(&app, "┃ Your message goes once the session is open");
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
  card(&app, gate);
  app.advance(Duration::from_secs(1));
  shows(&app, "1.0 s");
  app.press("escape");
  let frame = shows(&app, "go to a project");
  assert!(frame.contains("Opening"), "the open goes on:\n{frame}");
  // A second open runs beside the first.
  card(&app, time);
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
  card(&app, gate);
  shows(&app, "2.0 s");
  File::create(&go).unwrap();
  shows(&app, "Two stages hold most");
}

#[cfg(unix)]
#[test]
fn an_open_that_fails_says_why_and_how_to_go_back_and_loses_no_draft() {
  let title = "remove the gate from the engine";
  let app = app();
  let (daemon, _) = resting(&app, &[title]);
  // Another holder owns the lease of the record, so the daemon refuses the open at once.
  let owner = File::create(lease(&record(&daemon, 0))).unwrap();
  owner.try_lock().unwrap();
  card(&app, title);
  app.advance(Duration::from_secs(1));
  let frame = shows(&app, "The record stays as it was.");
  assert!(frame.contains("Another process owns"), "{frame}");
  assert!(!frame.contains("Opening"), "{frame}");
  // A message to the thread is refused the same way, and comes back to the input whole. The
  // thread shows it while its open goes, under the input that is empty and says what a reply
  // does, so the wait is for the input that holds it.
  app.paste("and the replay");
  app.press("enter");
  let back = app.wait_until(PATIENCE, || {
    let frame = app.text();
    frame.contains("┃ and the replay") && !frame.contains("A reply opens this thread again")
  });
  assert!(back, "the message never came back to the input:\n{}", app.text());
  assert!(app.text().contains("Another process owns"), "{}", app.text());
  // The machine says that no life holds the record: the card rests again.
  shows(&app, "● now");
  screen("open-failed", &app);
  // The way back takes a click, as its key does, and lights under the pointer.
  clicked(&app, "⎋ back", 0, 0);
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
  card(&app, title);
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

/// A session whose record stands before the stable cut opens in no life: the window still reads
/// its thread, says why it ended with no work that moves, and takes no message for it.
#[cfg(unix)]
#[test]
fn a_session_that_a_fault_ended_shows_its_thread_and_why_and_takes_no_message() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.project("furb");
  daemon.served();
  let before = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/replay/speaker.jsonl");
  fs::copy(before, record(&daemon, 0)).unwrap();
  open(&app, vec![daemon.machine("mac")], homed(&daemon));
  // Its cards rest with the resolved ones, which a click on their head unfolds, and each says
  // that the window only reads it.
  shows(&app, "Resolved 4 ▸");
  clicked(&app, "Resolved 4 ▸", 0, 0);
  clicked(&app, "read only", 0, 0);
  shows(&app, "1  b = await bash('echo a')");
  let frame = shows(&app, "The window reads this session and sends it nothing.");
  let left: Vec<String> = frame.lines().map(|line| line.chars().take(LEFT).collect()).collect();
  assert_eq!(
    left[1].trim_end(),
    "  The record comes from a build before the stable cut of",
    "{frame}"
  );
  assert!(frame.contains("The window sends nothing to this session"), "{frame}");
  // The thread shows no work and no button that resolves it; the list beside it keeps its parts.
  let left = left.join("\n");
  assert!(!left.contains("Working") && !left.contains("Resolve"), "{frame}");
  app.paste("Run it.");
  app.press("enter");
  shows(&app, "Nothing went: the record comes from");
}
