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
  daemon::{ANSWERS, ASKS, Daemon, PATIENCE, WORKS},
  screens::screen,
  thread::{LEFT, at, clicked},
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
  assert!(app.text().contains("+ new session in"), "{}", app.text());
  assert_eq!(bar(&app), BLUE);
  daemon.hold();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  // His message shows at once, before the model answers, and the input replies to its thread.
  let frame = app.text();
  assert!(frame.contains("┃ how long does an open take"), "{frame}");
  assert!(frame.contains("┃ Reply to this thread"), "{frame}");
  daemon.let_go();
  shows(&app, "Two stages hold most");
  // The card of the thread, which the model answered, as its machine tells it after the page. He
  // reads its end, so it waits for him in no part of the list, and rests with the resolved ones,
  // whose rows are closed.
  let frame = shows(&app, "Resolved 1");
  assert!(!frame.contains("Needs you"), "{frame}");
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
  // The up arrow walks the history from the draft: no message starts so, so the text stays once
  // the history came, and the thread still works.
  app.paste("a draft");
  app.press("up");
  let frame = shows(&app, "Nothing that you sent here starts so.");
  assert!(frame.contains("┃ a draft"), "{frame}");
  assert!(frame.contains(" Working 1") && !frame.contains("Stopped"), "{frame}");
  screen("input-in-order", &app);
}

/// The text of an input of one row, which stands above a blank row of its box, the launcher, the
/// last row of its box and the hints, once it shows a text: the window draws the text that the
/// editor takes at its next frame. The box of the context stands at its right.
fn typed(app: &TestApp, text: &str) -> String {
  let row = || {
    let frame = app.text();
    let lines: Vec<&str> = frame.lines().collect();
    let input: String = lines[lines.len() - 5].chars().take(LEFT).collect();
    input.trim_start_matches(['┃', ' ']).trim_end().to_owned()
  };
  let shown = app.wait_until(PATIENCE, || row() == text);
  assert!(shown, "the input never held {text:?}:\n{}", app.text());
  row()
}

#[test]
fn the_up_arrow_walks_what_he_sent_in_the_project_from_the_newest_and_from_what_he_wrote() {
  let (app, daemon, sessions) = started();
  let (first, second) = (format!("fix the build {ANSWERS}"), format!("say what moved {ANSWERS}"));
  sent(&app, &first);
  shows(&app, "Two stages hold most");
  sent(&app, &second);
  assert!(app.wait_until(PATIENCE, || told(&daemon, &sessions).len() == 2));
  // From an empty input, the newest first, and the oldest stays.
  app.press("up");
  typed(&app, &second);
  app.press("up");
  typed(&app, &first);
  app.press("up");
  typed(&app, &first);
  app.press("down");
  typed(&app, &second);
  app.press("down");
  typed(&app, "A reply opens this thread again");
  // From what he wrote, only what starts with it, and down gives it back.
  app.paste("say");
  app.press("up");
  typed(&app, &second);
  app.press("up");
  typed(&app, &second);
  app.press("down");
  typed(&app, "say");
  // A text that starts no message stays, and the row of hints says why.
  app.paste(" nothing");
  app.press("up");
  typed(&app, "say nothing");
  shows(&app, "Nothing that you sent here starts so.");
}

#[test]
fn the_input_shows_in_grey_the_rest_of_what_he_sent_and_the_right_arrow_takes_it() {
  let (app, daemon, sessions) = started();
  let (first, second) = (format!("fix the build {ANSWERS}"), format!("say what moved {ANSWERS}"));
  sent(&app, &first);
  shows(&app, "Two stages hold most");
  sent(&app, &second);
  assert!(app.wait_until(PATIENCE, || told(&daemon, &sessions).len() == 2));
  // The feed shows the answer to the second message, whose lines go on from the first, so it
  // stands still for the capture.
  shows(&app, "    4  close(");
  // The rest of the newest text that starts with what he wrote shows after it, in grey.
  app.paste("fi");
  typed(&app, &first);
  let frame = app.render();
  let row = (frame.height as usize - 5) * frame.width as usize;
  let (written, rest) = (&frame.cells[row + 2], &frame.cells[row + 4]);
  assert_ne!(written.fg, rest.fg, "the rest is grey and what he wrote is not");
  screen("input-suggested", &app);
  app.press("right");
  typed(&app, &first);
  assert_eq!(app.render().cells[row + 4].fg, written.fg, "the right arrow took the rest");
  // One word of it with alt and the right arrow.
  app.press("ctrl+c");
  app.paste("say");
  app.press("alt+right");
  app.paste("!");
  typed(&app, "say what!");
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
  assert!(frame.contains("↩ ⌃N"), "{frame}");
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
fn two_escapes_stop_the_work_of_the_thread_and_one_says_that_it_armed_the_stop() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ ⎋ stop");
  app.press("escape");
  shows(&app, "⎋ again stops the work");
  // Another key forgets the first escape, so the next one arms the stop again.
  app.press("ctrl+n");
  app.press("ctrl+n");
  app.press("escape");
  app.advance(Duration::from_millis(300));
  let frame = shows(&app, "⎋ again stops the work");
  assert!(!frame.contains("Stopped"), "{frame}");
  app.press("escape");
  let frame = shows(&app, "Stopped");
  assert!(!frame.contains("Working · ⎋ ⎋ stop"), "{frame}");
}

#[test]
fn a_thread_that_ended_turns_the_input_to_it_and_a_reply_opens_it_and_the_button_resolves_it() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Two stages hold most");
  // The model resolved the thread: its head says so, its button and the input say that a reply
  // opens it again.
  let frame = shows(&app, "✓ Resolved    Reopen ");
  assert!(frame.contains("┃ A reply opens this thread again"), "{frame}");
  screen("input-resolved", &app);
  // The key of the button turns the input to the thread and sends nothing.
  app.press("ctrl+n");
  shows(&app, "┃ Ask in a new thread");
  app.press("ctrl+x");
  app.press("r");
  shows(&app, "┃ A reply opens this thread again");
  assert_eq!(told(&daemon, &sessions).len(), 1);
  // A reply opens it again, and its model asks him.
  sent(&app, &format!("and which key sends the queue {ASKS}"));
  shows(&app, "Which key sends the queue?");
  let frame = shows(&app, " Resolve ");
  assert!(!frame.contains("✓ Resolved"), "{frame}");
  // The button resolves it: its close ends the question, and the head says that it is resolved.
  clicked(&app, " Resolve ", 0, 2);
  shows(&app, "✓ Resolved    Reopen ");
  shows(&app, "◆ Which key sends the…     refused");
  assert_eq!(told(&daemon, &sessions).len(), 2);
}

#[test]
fn a_resolve_of_a_thread_that_works_waits_for_its_confirm() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ ⎋ stop");
  paged(&app);
  app.press("ctrl+x");
  app.press("r");
  shows(&app, " Stop and resolve ");
  screen("input-resolve-confirm", &app);
  // Another key leaves the confirm, and the thread still works.
  app.press("escape");
  let frame = shows(&app, " Resolve ");
  assert!(frame.contains("Working · ⎋ ⎋ stop") && !frame.contains("Stop and resolve"), "{frame}");
  let (x, y) = at(&app, " Resolve ", 0);
  app.click(x, y);
  shows(&app, " Stop and resolve ");
  let (x, y) = at(&app, " Stop and resolve ", 0);
  app.click(x, y);
  let frame = shows(&app, "✓ Resolved    Reopen ");
  assert!(!frame.contains("Working · ⎋ ⎋ stop"), "{frame}");
}

#[test]
fn ctrl_c_empties_the_input_then_names_his_last_message_that_runs_and_takes_it_back() {
  let (app, daemon, sessions) = started();
  app.paste("a draft to drop");
  shows(&app, "┃ a draft to drop");
  app.press("ctrl+c");
  shows(&app, "┃ Ask for a change, a review or an answer");
  // A message on a thread that works is read at its next step, so the thread is what still runs.
  for word in ["one", "two", "three"] {
    sent(&app, &format!("{word} {WORKS}"));
  }
  assert!(app.wait_until(PATIENCE, || told(&daemon, &sessions).len() == 3));
  paged(&app);
  app.press("ctrl+c");
  let frame = shows(&app, "⌃C again takes back “one (the model works…");
  assert!(frame.contains("Working 1"), "{frame}");
  screen("input-take-back", &app);
  app.press("ctrl+c");
  let frame = shows(&app, "Stopped");
  assert!(frame.contains(&format!("┃ one {WORKS}")), "{frame}");
  // The first press empties the input, and the second says that nothing of his runs now.
  app.press("ctrl+c");
  app.press("ctrl+c");
  shows(&app, "Nothing that you sent still runs here.");
}

#[test]
fn ctrl_d_leaves_on_an_empty_input_once_it_named_it_and_another_key_forgets_it() {
  let (app, _daemon, _sessions) = started();
  app.press("ctrl+d");
  shows(&app, "⌃D again leaves the window");
  app.press("ctrl+n");
  let left = app.wait_until(PATIENCE, || !app.text().contains("again leaves"));
  assert!(left, "{}", app.text());
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
  screen("input-refused", &app);
  assert!(frame.contains("┃ draft after"), "{frame}");
  assert!(!frame.contains("Reply to this thread"), "{frame}");
}

#[test]
fn the_python_switch_runs_a_word_of_the_operator() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Two stages hold most");
  // The card of the thread, which the model answered, as its machine tells it after the page, once
  // he read its end.
  shows(&app, "Resolved 1");
  clicked(&app, " Python ", 0, 1);
  shows(&app, "┃ A word of Python to run");
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
fn the_row_of_hints_says_what_a_word_came_to_until_an_escape() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Resolved 1");
  clicked(&app, " Python ", 0, 1);
  shows(&app, "┃ A word of Python to run");
  // A word that waits shows a mark that moves while it runs, then a check once it ran.
  sent(&app, "await wait(1, __name__)\nanswer = 41 + 1");
  let running = shows(&app, "await wait(1, __name__)\\nanswer = 41 + 1");
  assert!(!running.contains("✓ await"), "{running}");
  shows(&app, "✓ await wait(1, __name__)\\nanswer = 41 + 1");
  screen("input-word-ran", &app);
  app.press("escape");
  let gone = app.wait_until(PATIENCE, || !app.text().contains("✓ await"));
  assert!(gone, "{}", app.text());
  sent(&app, "raise ValueError('no')");
  shows(&app, "✗ ValueError: no · raise ValueError('no')");
  screen("input-word-raised", &app);
  app.press("escape");
  let gone = app.wait_until(PATIENCE, || !app.text().contains("✗ ValueError"));
  assert!(gone, "{}", app.text());
}

#[test]
fn the_picker_of_the_sessions_filters_and_picks_the_session_that_the_input_sends_to() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ ⎋ stop");
  paged(&app);
  clicked(&app, "→ w", 0, 0);
  let frame = shows(&app, "Search a session");
  assert!(frame.contains("+ new session") && frame.contains("2 / 2"), "{frame}");
  screen("input-picker", &app);
  app.type_text("new");
  let frame = shows(&app, "▌new");
  assert!(!frame.contains("where does the time go     "), "{frame}");
  app.press("enter");
  shows(&app, "┃ Ask for a change, a review or an answer");
  shows(&app, "+ new session in");
  // The key opens it too, and an escape closes it with no choice.
  app.press("ctrl+x");
  app.press("o");
  shows(&app, "Search a session");
  app.press("escape");
  let closed = app.wait_until(PATIENCE, || !app.text().contains("Search a session"));
  assert!(closed, "{}", app.text());
  assert!(app.text().contains("+ new session in"), "{}", app.text());
}
