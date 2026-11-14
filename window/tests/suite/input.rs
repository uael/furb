//! The input and the line of the launcher under it, on a real daemon whose model answers from a
//! script: a message starts a session or goes to a thread, and the thread shows it.

#![cfg(test)]

use std::{
  fs,
  path::{Path, PathBuf},
  time::Duration,
};

use furb_host::protocol::{Answer, Verb};
use furb_window::{Options, open};
use opentui::{style::Color, testing::TestApp};

use crate::{
  cards::{app, options, shows},
  daemon::{ANSWERS, Daemon, PATIENCE, WORKS},
  screens::screen,
  thread::{at, lights},
};

/// The blue of the bar of a new thread.
const BLUE: Color = Color::hex(0x76_99fe);

/// A window in the folder of a project that the list of the machine does not name yet, whose home
/// folder is the folder of the test, so a path of it shows the same in each run. The folder of the
/// records of the sessions that the window starts.
fn started() -> (TestApp, Daemon, PathBuf) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.served();
  let folder = daemon.file("furb-try");
  fs::create_dir_all(&folder).unwrap();
  let sessions = folder.join(".furb").join("sessions");
  let home = folder.parent().map(PathBuf::from);
  let options = Options { folder, home, ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  shows(&app, "No thread yet in this project.");
  (app, daemon, sessions)
}

/// The frame of a window once the page of its open thread came: the block of the thread names the
/// model of the test with its effort, which no card names.
fn paged(app: &TestApp) -> String {
  shows(app, "opus/high")
}

/// The text of the input sent.
fn sent(app: &TestApp, text: &str) {
  app.paste(text);
  app.press("enter");
}

/// The color of the bar of the input of one row, which stands above the launcher and the hints.
fn bar(app: &TestApp) -> Color {
  let frame = app.render();
  frame.cells[(frame.height as usize - 3) * frame.width as usize].fg
}

/// The texts of the messages of the thread of the one session of a project, as its daemon holds
/// them: none while the daemon holds no card of it.
fn told(daemon: &Daemon, sessions: &Path) -> Vec<String> {
  let Ok(records) = fs::read_dir(sessions) else { return Vec::new() };
  let records = records.flatten().map(|one| one.path());
  let mut records = records.filter(|one| one.extension().is_some_and(|ext| ext == "jsonl"));
  let Some(life) = records.next().map(|one| one.display().to_string()) else { return Vec::new() };
  let mut peer = daemon.peer();
  let Answer::Cards(cards) = peer.ask(Verb::Cards) else { panic!("the daemon gives the cards") };
  let Some(card) = cards.into_iter().find(|card| card.life == life) else { return Vec::new() };
  let page = Verb::Thread { life, thread: card.thread.prompt, before: None, limit: 20 };
  let Answer::Page(page) = peer.ask(page) else { panic!("the daemon gives a page") };
  page.exchanges.into_iter().map(|one| one.text).collect()
}

#[test]
fn a_message_starts_a_new_session_and_its_answer_shows_in_its_thread() {
  let (app, daemon, sessions) = started();
  assert!(app.text().contains("This computer   furb-try · new session ▾"), "{}", app.text());
  assert_eq!(bar(&app), BLUE);
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  // His message shows at once, before the daemon answers.
  let frame = app.text();
  assert!(frame.contains("┃ how long does an open take"), "{frame}");
  assert!(frame.contains("┃ Reply to this thread"), "{frame}");
  shows(&app, "Two stages hold most");
  // The card of the thread, which the model answered, as its machine tells it after the page.
  shows(&app, "FT furb-try");
  shows(&app, "● now");
  screen("input-new-session", &app);
  assert_eq!(told(&daemon, &sessions), [format!("how long does an open take {ANSWERS}")]);
}

#[test]
fn three_enters_in_a_row_send_three_messages_in_order_and_the_up_arrow_cancels_nothing() {
  let (app, daemon, sessions) = started();
  for word in ["one", "two", "three"] {
    sent(&app, &format!("{word} {WORKS}"));
  }
  let frame = app.text();
  let at = |word: &str| frame.find(&format!("┃ {word} ")).unwrap_or_else(|| panic!("{frame}"));
  assert!(at("one") < at("two") && at("two") < at("three"), "{frame}");
  let all = |texts: Vec<String>| texts.len() == 3;
  assert!(app.wait_until(PATIENCE, || all(told(&daemon, &sessions))));
  let words = ["one", "two", "three"].map(|word| format!("{word} {WORKS}"));
  assert_eq!(told(&daemon, &sessions), words);
  // The card of the thread, which works, and its page came.
  shows(&app, " Working 1");
  paged(&app);
  // The up arrow moves in the input: the text stays, and the thread still works.
  app.paste("a draft");
  app.press("up");
  let frame = shows(&app, "┃ a draft");
  assert!(frame.contains(" Working 1") && !frame.contains("Stopped"), "{frame}");
  screen("input-in-order", &app);
}

#[test]
fn the_key_of_a_new_thread_turns_the_input_and_its_bar_and_a_reply_reaches_a_thread_that_works() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ ⎋ stop");
  paged(&app);
  let hue = bar(&app);
  app.press("ctrl+n");
  let frame = shows(&app, "┃ Ask in a new thread");
  assert!(frame.contains("⌃N back to the thread"), "{frame}");
  assert_eq!(bar(&app), BLUE);
  screen("input-new-thread", &app);
  app.press("ctrl+n");
  shows(&app, "┃ Reply to this thread");
  assert_eq!(bar(&app), hue);
  // A reply while the thread works.
  sent(&app, "and the seed");
  let reached = |texts: Vec<String>| texts.len() == 2;
  assert!(app.wait_until(PATIENCE, || reached(told(&daemon, &sessions))));
  let frame = app.text();
  assert_eq!(frame.matches("┃ and the seed").count(), 1, "{frame}");
}

#[test]
fn two_escapes_stop_the_work_of_the_thread_and_one_does_not() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ ⎋ stop");
  app.press("escape");
  app.advance(Duration::from_millis(2001));
  app.press("escape");
  app.advance(Duration::from_millis(300));
  assert!(app.text().contains("Working · ⎋ ⎋ stop"), "{}", app.text());
  app.press("escape");
  app.press("escape");
  let frame = shows(&app, "Stopped");
  assert!(!frame.contains("Working · ⎋ ⎋ stop"), "{frame}");
}

#[test]
fn ctrl_c_empties_the_input() {
  let (app, _daemon, _sessions) = started();
  app.paste("a draft to drop");
  shows(&app, "┃ a draft to drop");
  app.press("ctrl+c");
  shows(&app, "┃ Ask for a change, a review or an answer");
}

#[test]
fn a_refused_message_comes_back_to_the_input_with_why() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.served();
  // The folder of the window holds a file where the sessions of its project would stand, so no
  // life can make its record there.
  let folder = daemon.file("broken");
  fs::create_dir_all(folder.join(".furb")).unwrap();
  fs::write(folder.join(".furb").join("sessions"), "").unwrap();
  let home = folder.parent().map(PathBuf::from);
  let options = Options { folder, home, ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  shows(&app, "No thread yet in this project.");
  sent(&app, "how long does an open take");
  app.paste("draft after");
  // The row of hints says why the daemon refused the open, which no other frame says.
  let frame = shows(&app, "  Refused: ");
  assert!(frame.contains("┃ how long does an open take"), "{frame}");
  // The card of the record that the refused open left, as its machine tells it.
  shows(&app, "BR broken");
  screen("input-refused", &app);
  assert!(frame.contains("┃ draft after"), "{frame}");
  assert!(!frame.contains("Reply to this thread"), "{frame}");
}

#[test]
fn the_python_switch_runs_a_word_of_the_operator() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Two stages hold most");
  // The card of the thread, which the model answered, as its machine tells it after the page.
  shows(&app, "FT furb-try");
  shows(&app, "● now");
  let (x, y) = at(&app, "○ Python", 0);
  lights(&app, x, y);
  app.click(x, y);
  let frame = shows(&app, "● Python");
  assert!(frame.contains("┃ A word of Python to run"), "{frame}");
  app.paste("answer = 41 + 1");
  screen("input-python", &app);
  app.press("enter");
  // The word ran on the root of the session: its name holds its value there.
  let record = fs::read_dir(&sessions).unwrap().flatten().map(|one| one.path());
  let life = record.filter(|one| one.extension().is_some_and(|ext| ext == "jsonl")).last();
  let life = life.expect("the window made a record").display().to_string();
  let mut peer = daemon.peer();
  let inspect = || Verb::Inspect { life: life.clone(), name: "answer".to_owned(), on: None };
  let ran = app.wait_until(
    PATIENCE,
    || matches!(peer.tried(inspect()), Ok(Answer::Inspected { repr, .. }) if repr == "42"),
  );
  assert!(ran, "{:?}", peer.tried(inspect()));
  shows(&app, "┃ A word of Python to run");
}

#[test]
fn the_picker_of_the_sessions_filters_and_picks_the_session_that_the_input_sends_to() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ ⎋ stop");
  paged(&app);
  let (x, y) = at(&app, "This computer", 0);
  let x = x + 16;
  lights(&app, x, y);
  app.click(x, y);
  let frame = shows(&app, "Filter the sessions");
  assert!(frame.contains("+ new session"), "{frame}");
  screen("input-picker", &app);
  app.type_text("new");
  let frame = shows(&app, "▌new");
  assert!(!frame.contains("where does the time go     "), "{frame}");
  app.press("enter");
  shows(&app, "┃ Ask for a change, a review or an answer");
  shows(&app, "furb-try · new session ▾");
  // The key opens it too, and an escape closes it with no choice.
  app.press("ctrl+x");
  app.press("o");
  shows(&app, "Filter the sessions");
  app.press("escape");
  let closed = app.wait_until(PATIENCE, || !app.text().contains("Filter the sessions"));
  assert!(closed, "{}", app.text());
  assert!(app.text().contains("furb-try · new session ▾"), "{}", app.text());
}
