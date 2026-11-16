//! The list part over the window: Go to, with the commands, and each key, on a real daemon
//! whose lives a peer of the test drives.
#![expect(clippy::string_slice, reason = "a panic is how a test fails")]
#![cfg(test)]

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::time::Duration;

use furb_window::{Options, open};
use opentui::{style::Color, testing::TestApp};

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
  app.press("ctrl+p");
  let frame = shows(&app, "Search a thread, session, project");
  // The sessions that he used last stand among the places before he searches.
  let parts = ["Working", "Sessions", "Projects"];
  assert!(parts.iter().all(|part| frame.contains(part)), "{frame}");
  // The project counts the session with no thread once the machine listed its records.
  shows(&app, "3 sessions");
  // The arrows move the chosen row, and letters filter the rows as he types. The sessions stand
  // by the last change of their records, which each life that stands makes when it will, so the
  // capture is of a search.
  app.press("down");
  shows(&app, "2 / ");
  app.type_text("setup");
  let frame = shows(&app, "▌setup");
  assert!(frame.contains("setup-only") && frame.contains("1 / 1"), "{frame}");
  screen("part-go-to", &app);
  // Enter takes it: a session with no thread of his turns the input to a new thread of it.
  app.press("enter");
  assert!(gone(&app, "Search a thread"), "{}", app.text());
  shows(&app, "┃ Ask in a new thread");
  // A project takes its sessions as the filter, and escape closes the part with no choice.
  app.press("ctrl+p");
  shows(&app, "Projects");
  app.type_text("furb");
  shows(&app, "3 sessions");
  app.press("escape");
  assert!(gone(&app, "Search a thread"), "{}", app.text());
}

#[test]
fn the_commands_run_from_go_to_and_each_key_shows_its_forms() {
  let (app, _daemon, _peer) = machine();
  // Go to lists each command after the places, and its search finds one.
  app.press("ctrl+p");
  shows(&app, "Search a thread");
  app.type_text("hide the list");
  let frame = shows(&app, "Show or hide the list");
  assert!(frame.contains("⌃X L"), "{frame}");
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
  shows(&app, "Search a thread, session, project");
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
  // The project opens for a new session at once, and no session of it that he did not pick.
  let frame = shows(&app, "FU furb ▾");
  assert!(frame.contains("Ask for a change, a review or an answer"), "{frame}");
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
fn where_room_is_short_the_machine_gives_way_and_the_project_and_the_effort_stay() {
  let (app, _daemon) = offered();
  // The launcher row at a width, once it is the one that the test expects, or as it stands then.
  let launcher = |wide: u16, expected: &str| {
    app.resize(wide, 40);
    let row = || {
      let frame = app.text();
      let row = frame.lines().find(|one| one.contains("Python")).unwrap_or_default().to_owned();
      row.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    app.wait_until(Duration::from_secs(10), || row() == expected);
    row()
  };
  // With room for each, the launcher names the machine, the project, the model and its effort.
  let wide = "┃ Message Python This computer ▾ FT furb-try ▾ opus high";
  assert_eq!(launcher(112, wide), wide);
  // Where the room runs short, the machine gives way to a few letters first, and the project and
  // the effort, which a click changes in place, stay.
  let short = "┃ Message Python This… ▾ FT furb-try ▾ opus high";
  assert_eq!(launcher(102, short), short);
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
  // The room of the model and its effort beside the title of the thread on the launcher.
  app.resize(102, 30);
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
fn a_click_on_the_effort_opens_the_efforts_of_the_model_on_the_one_in_use() {
  let (app, _daemon) = offered();
  // The home names the effort in its words first, at the left of the launcher.
  clicked(&app, "high", 40, 0);
  let frame = shows(&app, "Search an effort");
  assert!(frame.contains("▌high"), "{frame}");
  app.press("down");
  app.press("enter");
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
  let both = || app.text().contains("FU furb ▾") && app.text().contains("opus  high");
  assert!(app.wait_until(PATIENCE, both), "{}", app.text());
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

#[test]
fn the_bar_of_the_chosen_row_and_the_badge_of_its_project_keep_their_hues_under_the_pointer() {
  let (app, _daemon, _peer) = machine();
  app.press("ctrl+p");
  let frame = shows(&app, "Search a thread, session, project");
  let chosen =
    frame.lines().enumerate().find(|(_, line)| line.contains('▌') && !line.contains("Search"));
  let (y, line) = chosen.unwrap_or_else(|| panic!("no row is chosen:\n{frame}"));
  let x = line.chars().position(|one| one == '▌').unwrap();
  let before = app.render();
  app.hover(u16::try_from(x + 6).unwrap(), u16::try_from(y).unwrap());
  let lit = app.render();
  let width = usize::try_from(lit.width).unwrap();
  // The cell under the pointer lights, and the bar and the two letters of the badge keep their
  // hues.
  assert_eq!(lit.cells[y * width + x + 6].fg, Color::hex(0xff_ffff), "{}", app.text());
  let (lit, before): (Vec<_>, Vec<_>) = (
    (x..x + 3).map(|at| lit.cells[y * width + at].fg).collect(),
    (x..x + 3).map(|at| before.cells[y * width + at].fg).collect(),
  );
  assert_eq!(lit, before, "{}", app.text());
}
