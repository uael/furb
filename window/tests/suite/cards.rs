//! The list of cards and the home, on a real daemon whose lives a peer of the test drives.

#![cfg(test)]

use std::{
  path::{Path, PathBuf},
  time::Duration,
};

use furb_host::protocol::Verb;
use furb_window::{Options, System, open};
use jiff::tz::TimeZone;
use opentui::testing::TestApp;

use crate::{
  daemon::{ASKS, Daemon, Hold, PATIENCE, Peer, WORKS, aged},
  screens::screen,
};

/// The second of the clock of each window of a test: 09:12 of the first day of 2026. Each life of
/// a test stands since a later second, so it shows no age, and a record of a test is as old as its
/// test makes it.
pub(crate) const CLOCK: u64 = 1_767_258_720;
const HOUR: u64 = 3600;

/// A window of 100 columns and 30 rows, as the design draws each screen.
pub(crate) fn app() -> TestApp {
  TestApp::new(100, 30)
}

/// The options of a window of a test. The system of the client is macOS, so a capture shows the
/// same keys on each runner.
pub(crate) fn options(kept: &Path) -> Options {
  let folder = PathBuf::from("/home/ada/src/furb-try");
  Options {
    folder,
    home: Some(PathBuf::from("/home/ada")),
    kept: kept.to_owned(),
    zone: TimeZone::UTC,
    clock: || CLOCK,
    system: System::MacOs,
  }
}

/// The frame of a window once it shows a text.
pub(crate) fn shows(app: &TestApp, text: &str) -> String {
  let shown = app.wait_until(PATIENCE, || app.text().contains(text));
  assert!(shown, "the window never showed {text:?}:\n{}", app.text());
  app.text()
}

/// The row of a window that shows a text.
pub(crate) fn row(app: &TestApp, text: &str) -> u16 {
  let at =
    app.text().lines().position(|line| line.contains(text)).expect("the window shows the text");
  u16::try_from(at).unwrap()
}

#[test]
fn the_first_frame_shows_the_cards_that_the_window_kept_then_each_machine_answers() {
  let first = app();
  let mut daemon = Daemon::bound(&first);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("gate.jsonl"), true);
  peer.said(&life, "remove the gate from the engine");
  let kept = daemon.file("cards.json");
  open(&first, vec![daemon.machine("mac")], options(&kept));
  shows(&first, "remove the gate from the…    ● now");
  // The window writes its cards a moment after it heard them.
  assert!(!kept.exists());
  first.advance(Duration::from_secs(1));
  assert!(kept.is_file());

  // A second window opens while its machine does not answer: its first frame holds the card,
  // with the hour at which the first window heard it.
  let second = app();
  let mut later = Daemon::bound(&second);
  open(&second, vec![later.machine("mac")], options(&kept));
  let frame = second.text();
  assert!(frame.contains("remove the gate from…  as of 09:12"), "{frame}");
  // The machine then answers for itself: it holds no record, so the card goes.
  later.served();
  let frame = shows(&second, "No thread yet in this project.");
  assert!(!frame.contains("remove the gate"), "{frame}");
}

#[test]
fn a_card_changes_its_state_at_once_and_its_place_once_the_hand_is_away() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("queue.jsonl"), true);
  let prompt = peer.said(&life, &format!("which key sends the queue\n{WORKS}"));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  shows(&app, " Working 1");

  // The hand is on the list when the thread stops: the card says so, under the same head.
  app.hover(80, 12);
  peer.ask(Verb::Cancel { life, act: prompt, why: None });
  let stopped = "which key sends the queue  Stopped";
  let frame = shows(&app, stopped);
  assert!(frame.contains(" Working 1") && !frame.contains(" Resting 1"), "{frame}");
  app.advance(Duration::from_millis(1499));
  let frame = app.text();
  assert!(frame.contains(" Working 1") && frame.contains(stopped), "{frame}");
  // The hand was away for a second and a half: the card goes to its place.
  app.advance(Duration::from_millis(1));
  let frame = app.text();
  assert!(
    frame.contains(" Resting 1") && frame.contains(stopped) && !frame.contains(" Working 1"),
    "{frame}"
  );
}

#[test]
fn a_machine_that_does_not_answer_holds_nothing_back_and_the_list_says_so_after_a_moment() {
  let app = app();
  let mut here = Daemon::bound(&app);
  let sessions = here.project("furb");
  here.served();
  let mut peer = here.peer();
  let life = peer.opened(&sessions.join("gate.jsonl"), true);
  peer.said(&life, "remove the gate from the engine");
  // A second machine takes the dial of the window and never answers it.
  let mut far = Daemon::bound(&app);
  open(&app, vec![here.machine("mac"), far.machine("studio")], options(&here.file("cards.json")));
  let frame = shows(&app, "remove the gate from the…    ● now");
  assert!(frame.contains("2 machines"), "{frame}");
  // A wait under 400 ms shows no loader.
  app.advance(Duration::from_millis(399));
  assert!(!app.text().contains("Dialing"));
  app.advance(Duration::from_millis(1));
  assert!(app.text().contains("⠴ Dialing studio"), "{}", app.text());
  // The machine is gone: the list says so, and the card of the other machine stands.
  far.lost();
  let frame = shows(&app, "studio offline");
  assert!(frame.contains("remove the gate from the…    ● now"), "{frame}");
  assert!(frame.contains("studio is offline: the daemon closed the connection"), "{frame}");
}

#[test]
fn the_first_run_says_what_the_window_is_for() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.project("furb-try");
  daemon.served();
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  // Nothing says that no thread stands before the machine said so.
  assert!(!app.text().contains("No thread yet"));
  shows(&app, "No thread yet in this project.");
  screen("first-run", &app);
}

/// The cards of a machine that is gone, which a window kept: one session of it worked with no
/// view.
fn studio(kept: &Path) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb-cloud");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("night.jsonl"), false);
  peer.said(&life, &format!("run the bench of replay each night\n{WORKS}"));
  open(&app, vec![daemon.machine("studio")], options(kept));
  shows(&app, "FC furb-cloud");
  app.advance(Duration::from_secs(1));
}

/// A machine at a start: five sessions of two projects were left at rest some hours ago, one with a
/// thread that the operator paused. One of them works again with no view, one more session asks
/// him, and one failed. The peer holds the views of the two last.
fn started(daemon: &mut Daemon) -> Peer {
  let (furb, photos) = (daemon.project("furb"), daemon.project("photowroom"));
  daemon.served();
  let mut peer = daemon.peer();
  let works = |title: &str| format!("{title}\n{WORKS}");
  let time = peer.rested(&furb.join("time.jsonl"), &works("where does the time go"), None);
  let gate = peer.rested(&furb.join("gate.jsonl"), "remove the gate from the engine", None);
  let seen = Some(Hold::Seen);
  let bench = peer.rested(&photos.join("bench.jsonl"), "which fold reads the whole record", seen);
  let queue = peer.rested(&photos.join("queue.jsonl"), &works("which key sends the queue"), None);
  let paused = Some(Hold::Paused);
  let ground =
    peer.rested(&furb.join("ground.jsonl"), &works("Ground the branch on the carrier"), paused);
  aged(&time, CLOCK, 11 * HOUR);
  aged(&gate, CLOCK, 5 * HOUR);
  aged(&bench, CLOCK, 48 * HOUR);
  aged(&queue, CLOCK, 11 * HOUR + 120);
  aged(&ground, CLOCK, 11 * HOUR + 120);
  peer.opened(&photos.join("queue.jsonl"), false);
  let scope = peer.opened(&furb.join("scope.jsonl"), true);
  peer.said(&scope, &format!("TUI scope for the new window\n{ASKS}"));
  let debug = peer.opened(&furb.join("debug.jsonl"), true);
  let failed = peer.said(&debug, &works("Debug stuck TUI after merge"));
  let refusal = r#"{"is": "Refused", "args": ["the model is out of reach"]}"#.to_owned();
  peer.ask(Verb::Close { life: debug, act: failed, value: refusal });
  peer
}

#[test]
fn the_start_shows_what_waits_and_what_worked_while_no_window_was_open() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let _peer = started(&mut daemon);
  let kept = daemon.file("cards.json");
  studio(&kept);
  let mut far = Daemon::bound(&app);
  open(&app, vec![daemon.machine("mac"), far.machine("studio")], options(&kept));
  far.lost();
  shows(&app, "studio offline");
  shows(&app, "✗ Failed");
  shows(&app, " Resolved 1");
  screen("start", &app);

  // The keys of the list: the next thread that needs him, the next card, and the resolved
  // threads, which a click on their head shows.
  app.press("alt+a");
  app.press("ctrl+x");
  app.press("j");
  app.click(70, row(&app, " Resolved 1 ▸"));
  app.hover(70, row(&app, "where does the time go"));
  screen("start-selected", &app);

  // Under 100 columns the list stands over the home, on a key.
  app.resize(80, 30);
  screen("start-narrow", &app);
  app.press("ctrl+x");
  app.press("l");
  screen("start-narrow-list", &app);
}
