//! The input and the line of the launcher under it, on a real daemon whose model answers from a
//! script: a message starts a session or goes to a thread, and the thread shows it.

#![cfg(test)]

use std::{
  fs,
  path::{Path, PathBuf},
  process::Command,
  time::Duration,
};

use furb_host::protocol::{
  Answer, Axis, Frame, Gesture, Mount, Node, Run, Shape, Tone, Tree, Verb, ViewEvent,
};
use furb_window::{Options, open};
use opentui::{input::Event, style::Color, testing::TestApp};

use crate::{
  cards::{app, options, shows},
  daemon::{ANSWERS, ASKS, Daemon, PATIENCE, WORKS},
  screens::screen,
  thread::{LEFT, at, clicked},
};

/// The orange of the accent: the bar of a new thread, and each letter that a search matches.
const ORANGE: Color = Color::hex(0xfa_b283);

/// A window in the folder of a project that the list of the machine does not name yet, whose home
/// folder is the folder of the test, so a path of it shows the same in each run. The folder of the
/// records of the sessions that the window starts.
pub(crate) fn started() -> (TestApp, Daemon, PathBuf) {
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
pub(crate) fn sent(app: &TestApp, text: &str) {
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
pub(crate) fn told(daemon: &Daemon, sessions: &Path) -> Vec<String> {
  let Ok(records) = fs::read_dir(sessions) else { return Vec::new() };
  let records = records.flatten().map(|one| one.path());
  let mut records = records.filter(|one| one.extension().is_some_and(|ext| ext == "jsonl"));
  let Some(life) = records.next().map(|one| one.display().to_string()) else { return Vec::new() };
  let mut peer = daemon.peer();
  let Answer::Cards(cards) = peer.ask(Verb::Cards) else { panic!("the daemon gives the cards") };
  let Some(card) = cards.into_iter().find(|card| card.life == life) else { return Vec::new() };
  let page = Verb::Thread { life, thread: card.thread.prompt, before: None, limit: 20, at: None };
  let Answer::Page(page) = peer.ask(page) else { panic!("the daemon gives a page") };
  page.exchanges.into_iter().map(|one| one.text).collect()
}

#[test]
fn a_message_starts_a_new_session_and_its_answer_shows_in_its_thread() {
  let (app, daemon, sessions) = started();
  assert!(app.text().contains("This computer ▾"), "{}", app.text());
  assert_eq!(bar(&app), ORANGE);
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
  // From an empty input, the newest first, and the oldest stays. The box says where the walk
  // stands.
  app.press("up");
  typed(&app, &second);
  shows(&app, "↑↓ history 1 / 2");
  app.press("up");
  typed(&app, &first);
  shows(&app, "↑↓ history 2 / 2");
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
fn ctrl_r_searches_what_he_sent_here_from_what_he_wrote_and_writes_the_text_that_he_takes() {
  let (app, daemon, sessions) = started();
  let (first, second) = (format!("fix the build {ANSWERS}"), format!("say what moved {ANSWERS}"));
  sent(&app, &first);
  shows(&app, "Two stages hold most");
  sent(&app, &second);
  assert!(app.wait_until(PATIENCE, || told(&daemon, &sessions).len() == 2));
  // The search starts from what he wrote, and lists only what matches it, its letters lit.
  app.paste("mvd");
  app.press("ctrl+r");
  let frame = shows(&app, "History");
  assert!(frame.contains("say what moved") && frame.contains("1 / 1"), "{frame}");
  // The history rises from the input, so its row of the text stands under the same text in the
  // feed.
  let text = app.text();
  let lines: Vec<&str> = text.lines().collect();
  let y = lines.iter().rposition(|line| line.contains("moved")).unwrap();
  let x = lines[y][..lines[y].find("moved").unwrap()].chars().count();
  let (x, y) = (u16::try_from(x).unwrap(), u16::try_from(y).unwrap());
  let frame = app.render();
  let cell = |x: u16| frame.cells[usize::from(y) * frame.width as usize + usize::from(x)].fg;
  let lit: Vec<bool> = (x..x + 5).map(|x| cell(x) == ORANGE).collect();
  assert_eq!(lit, [true, false, true, false, true], "the letters m, v and d are lit");
  // Enter writes the text into the input, and closes the list.
  app.press("enter");
  typed(&app, &second);
  assert!(!app.text().contains("Search what you sent here"), "{}", app.text());
}

#[test]
fn the_input_shows_in_grey_the_rest_of_what_he_sent_and_the_right_arrow_takes_it() {
  let (app, daemon, sessions) = started();
  let (first, second) = (format!("fix the build {ANSWERS}"), format!("say what moved {ANSWERS}"));
  sent(&app, &first);
  shows(&app, "Two stages hold most");
  sent(&app, &second);
  assert!(app.wait_until(PATIENCE, || told(&daemon, &sessions).len() == 2));
  // The feed shows the answer to the second message, whose lines go on from the first, and the
  // card says that the thread rests, so it stands still for the capture.
  shows(&app, "    4  close(");
  shows(&app, "Resolved 1");
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
  // The rest shows in grey once the window holds what he wrote, and only then a key takes it.
  typed(&app, &second);
  app.press("alt+right");
  app.paste("!");
  typed(&app, "say what!");
}

#[test]
fn the_key_of_a_new_thread_turns_the_input_and_its_bar_and_a_reply_reaches_a_thread_that_works() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ stop");
  paged(&app);
  let hue = bar(&app);
  app.press("ctrl+n");
  let frame = shows(&app, "┃ Ask in a new thread");
  assert!(frame.contains("↩ ⌃N"), "{frame}");
  assert_eq!(bar(&app), ORANGE);
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
  shows(&app, "Working · ⎋ stop");
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
  assert!(!frame.contains("Working · ⎋ stop"), "{frame}");
}

#[test]
fn a_thread_that_ended_turns_the_input_to_it_and_a_reply_opens_it_and_the_button_resolves_it() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Two stages hold most");
  // The model resolved the thread: its head says so, and the input says that a reply opens it
  // again, so the head shows no button that would change nothing.
  let frame = shows(&app, "✓ Resolved");
  assert!(frame.contains("┃ A reply opens this thread again"), "{frame}");
  assert!(!frame.contains("Reopen"), "{frame}");
  screen("input-resolved", &app);
  // While the input sends elsewhere, the button stands; it turns the input to the thread and
  // sends nothing.
  app.press("ctrl+n");
  shows(&app, "┃ Ask in a new thread");
  clicked(&app, " Reopen ", 0, 2);
  shows(&app, "┃ A reply opens this thread again");
  app.press("ctrl+n");
  shows(&app, " Reopen ");
  app.press("ctrl+x");
  app.press("r");
  shows(&app, "┃ A reply opens this thread again");
  assert_eq!(told(&daemon, &sessions).len(), 1);
  // A reply opens it again, and its model asks him.
  sent(&app, &format!("and which key sends the queue {ASKS}"));
  shows(&app, "Which key sends the queue?");
  // The card says that the thread asks him once its machine told it, and only then a resolve
  // needs no confirm of a stop of its work.
  shows(&app, "◆ Input");
  let frame = shows(&app, " Resolve ");
  assert!(!frame.contains("✓ Resolved"), "{frame}");
  // The button resolves it: its close ends the question, and the head says that it is resolved.
  clicked(&app, " Resolve ", 0, 2);
  shows(&app, "✓ Resolved");
  // The close withdrew the question, which no one answered, and its line says so.
  let withdrawn = |line: &str| line.contains("◆ Which key") && line.contains("withdrawn · ");
  let shown = app.wait_until(PATIENCE, || app.text().lines().any(withdrawn));
  assert!(shown, "{}", app.text());
  assert_eq!(told(&daemon, &sessions).len(), 2);
}

#[test]
fn a_resolve_of_a_thread_that_works_waits_for_its_confirm() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ stop");
  paged(&app);
  app.press("ctrl+x");
  app.press("r");
  let frame = shows(&app, " Stop and resolve ");
  assert!(frame.contains("⌃X R again stops and resolves the thread"), "{frame}");
  screen("input-resolve-confirm", &app);
  // Another key leaves the confirm, and the thread still works.
  app.press("escape");
  let frame = shows(&app, " Resolve ");
  assert!(frame.contains("Working · ⎋ stop") && !frame.contains("Stop and resolve"), "{frame}");
  let (x, y) = at(&app, " Resolve ", 0);
  app.click(x, y);
  shows(&app, " Stop and resolve ");
  let (x, y) = at(&app, " Stop and resolve ", 0);
  app.click(x, y);
  let frame = shows(&app, "✓ Resolved");
  assert!(!frame.contains("Working · ⎋ stop"), "{frame}");
}

#[test]
fn ctrl_c_empties_the_input_then_names_his_last_message_that_runs_and_takes_it_back() {
  let (app, daemon, sessions) = started();
  app.paste("a draft to drop");
  shows(&app, "┃ a draft to drop");
  app.press("ctrl+c");
  shows(&app, "┃ Ask for a change, a review or an answer");
  // Each message on a thread is a question that runs until its answer, so the last one runs.
  for word in ["one", "two", "three"] {
    sent(&app, &format!("{word} {WORKS}"));
  }
  assert!(app.wait_until(PATIENCE, || told(&daemon, &sessions).len() == 3));
  paged(&app);
  app.press("ctrl+c");
  shows(&app, "⌃C again takes back “three (the model works");
  screen("input-take-back", &app);
  app.press("ctrl+c");
  // The question of the message stops, and its text comes back to the input.
  let frame = shows(&app, "stopped");
  assert_eq!(frame.matches(&format!("┃ three {WORKS}")).count(), 2, "{frame}");
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
  // The row of hints says why the daemon refused the open, which no other frame says. The daemon
  // refuses it at once, so a draft written after the send has no time to come first: the model
  // test of a refused open proves that a draft stays after the text that comes back.
  let frame = shows(&app, "  Refused: ");
  assert!(frame.contains("┃ how long does an open take"), "{frame}");
  // The launcher names the model of a new session once the machine gave its models.
  shows(&app, "opus  high");
  screen("input-refused", &app);
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
  shows(&app, "┃ >>> A word of Python to run");
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
  shows(&app, "┃ >>> A word of Python to run");
}

#[test]
fn a_word_of_python_shows_in_its_thread_as_his_code_and_a_line_that_raised_says_why() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Resolved 1");
  clicked(&app, " Python ", 0, 1);
  shows(&app, "┃ >>> A word of Python to run");
  // The word is a box of his code in the thread, on the bar of his messages, line by line.
  sent(&app, "await wait(1, __name__)\nanswer = 41 + 1");
  let ran = shows(&app, "  answer = 41 + 1");
  let his =
    |frame: &str, code: &str| frame.lines().any(|one| one.starts_with('┃') && one.contains(code));
  assert!(his(&ran, "await wait(1, __name__)") && his(&ran, "answer = 41 + 1"), "{ran}");
  screen("input-word-ran", &app);
  // A line that raised shows its cross in the place of its number, and why under it.
  sent(&app, "raise ValueError('no')");
  let raised = shows(&app, "✗  raise ValueError('no')");
  assert!(his(&raised, "ValueError: no"), "{raised}");
  screen("input-word-raised", &app);
}

#[test]
fn a_message_takes_the_place_of_the_hints_of_the_word_that_ran_before_it() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Resolved 1");
  // A word for a new thread runs on the root of the session, so only the hint row says it.
  app.press("ctrl+n");
  clicked(&app, " Python ", 0, 1);
  sent(&app, "raise ValueError('no')");
  let hint = |frame: &str| frame.lines().last().is_some_and(|one| one.contains("ValueError"));
  assert!(app.wait_until(PATIENCE, || hint(&app.text())), "{}", app.text());
  clicked(&app, " Message ", 0, 1);
  sent(&app, "and now a message");
  shows(&app, "and now a message");
  assert!(app.wait_until(PATIENCE, || !hint(&app.text())), "{}", app.text());
}

#[test]
fn the_picker_of_the_sessions_filters_and_picks_the_session_that_the_input_sends_to() {
  let (app, _daemon, _sessions) = started();
  sent(&app, &format!("where does the time go {WORKS}"));
  shows(&app, "Working · ⎋ stop");
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
  // The launcher names the project of the new session, and the machine gives way to the effort.
  shows(&app, "This… ▾   FT ▾");
  // The key opens it too, and an escape closes it with no choice.
  app.press("ctrl+x");
  app.press("o");
  shows(&app, "Search a session");
  app.press("escape");
  let closed = app.wait_until(PATIENCE, || !app.text().contains("Search a session"));
  assert!(closed, "{}", app.text());
  assert!(app.text().contains("This… ▾   FT ▾"), "{}", app.text());
}

#[test]
fn a_mention_offers_the_files_of_the_project_and_takes_one_by_its_path() {
  let (app, daemon, _) = started();
  let deep = daemon.file("furb-try").join("src/deep/storage.py");
  fs::create_dir_all(deep.parent().unwrap()).unwrap();
  fs::write(&deep, "").unwrap();
  app.type_text("look at @stor");
  let shown = shows(&app, "storage.py  src/deep/");
  assert!(shown.contains("Files of furb-try"), "{shown}");
  app.press("tab");
  shows(&app, "look at @src/deep/storage.py");
  assert!(!app.text().contains("Files of furb-try"), "{}", app.text());
}

#[test]
fn the_review_lists_the_changed_files_of_the_project_and_the_diff_of_each_and_the_branch_opens_it()
{
  let (app, daemon, _) = started();
  let project = daemon.file("furb-try");
  let git = |args: &[&str]| {
    let done = Command::new("git").arg("-C").arg(&project).args(args).status().unwrap();
    assert!(done.success(), "git {args:?}");
  };
  fs::write(project.join("a.py"), "one = 1\n").unwrap();
  git(&["init", "-q", "-b", "trunk"]);
  git(&["add", "a.py"]);
  git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "a"]);
  fs::write(project.join("a.py"), "one = 2\n").unwrap();
  fs::write(project.join("b.py"), "two = 2\n").unwrap();
  app.press("ctrl+x");
  app.press("g");
  let shown = shows(&app, "one = 2");
  assert!(shown.contains(" furb-try · ⑂ trunk · a.py"), "{shown}");
  assert!(shown.contains("Changes  2 files"), "{shown}");
  assert!(shown.contains("M a.py"), "{shown}");
  assert!(shown.contains("U b.py"), "{shown}");
  app.press("down");
  shows(&app, "two = 2");
  app.press("escape");
  let gone = app.wait_until(PATIENCE, || !app.text().contains("Changes  2 files"));
  assert!(gone, "{}", app.text());
  // The branch of the row of hints opens the review, as its key does.
  shows(&app, "⑂ trunk ±2");
  let (x, y) = at(&app, "⑂ trunk ±2", 0);
  app.click(x, y);
  shows(&app, "Changes  2 files");
}

#[test]
fn a_file_changed_with_no_life_counts_once_the_window_takes_the_focus_again() {
  let (app, daemon, _) = started();
  let project = daemon.file("furb-try");
  let git = |args: &[&str]| {
    let done = Command::new("git").arg("-C").arg(&project).args(args).status().unwrap();
    assert!(done.success(), "git {args:?}");
  };
  fs::write(project.join("a.py"), "one = 1\n").unwrap();
  git(&["init", "-q", "-b", "trunk"]);
  git(&["add", "a.py"]);
  git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "a"]);
  fs::write(project.join("a.py"), "one = 2\n").unwrap();
  // The window asked the changes of its folder at its start, before the folder was a checkout.
  app.event(Event::Focus(true));
  shows(&app, "⑂ trunk ±1");
  // The operator leaves for an editor, writes a file there, and comes back to the window.
  app.event(Event::Focus(false));
  fs::write(project.join("b.py"), "two = 2\n").unwrap();
  app.event(Event::Focus(true));
  shows(&app, "⑂ trunk ±2");
}

#[test]
fn a_view_of_a_skill_shows_under_its_thread_live_and_takes_a_press_and_an_input() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("keep my list {ANSWERS}"));
  paged(&app);
  let mut record = fs::read_dir(&sessions).unwrap().map(|one| one.unwrap().path());
  let life = record.find(|one| one.extension().is_some_and(|ext| ext == "jsonl")).unwrap();
  let life = life.display().to_string();
  let mut skill = daemon.skill("todo");
  assert_eq!(skill.ask(Verb::Attach { life, from: u64::MAX }), Answer::Done);
  let Frame::ViewMount(Mount { viewer, slot, .. }) =
    skill.heard(&app, |frame| matches!(frame, Frame::ViewMount(_)))
  else {
    unreachable!()
  };
  let node = |id: &str, shape| Node { id: id.to_owned(), shape };
  let tree = |items: &[&str], version| {
    let rows = items.iter().map(|one| {
      node(one, Shape::Text { runs: vec![Run { text: (*one).to_owned(), tone: Tone::Text }] })
    });
    let add = node(
      "add",
      Shape::Input { value: String::new(), hint: Some("a task".to_owned()), inert: false },
    );
    let clear = node("clear", Shape::Button { label: "Clear".to_owned(), inert: false });
    let tools = node(
      "tools",
      Shape::Box { axis: Axis::Row, gap: 2, border: false, children: vec![add, clear] },
    );
    let list = node("list", Shape::List { items: rows.collect() });
    let shape =
      Shape::Box { axis: Axis::Column, gap: 0, border: true, children: vec![list, tools] };
    Verb::ViewTree(Tree {
      viewer: viewer.clone(),
      slot: slot.clone(),
      version,
      tree: node("todo", shape),
    })
  };
  assert_eq!(skill.ask(tree(&["buy milk"], 1)), Answer::Done);
  let shown = shows(&app, "• buy milk");
  assert!(shown.contains("a task"), "{shown}");
  clicked(&app, "Clear", 0, 0);
  let pressed = skill.heard(&app, |frame| matches!(frame, Frame::ViewEvent(_)));
  let Frame::ViewEvent(ViewEvent { node: pressed, kind, .. }) = pressed else { unreachable!() };
  assert_eq!((pressed.as_str(), kind), ("clear", Gesture::Press));
  clicked(&app, "a task", 0, 0);
  shows(&app, "The input fills add of the view todo");
  sent(&app, "walk the dog");
  let submitted = skill.heard(&app, |frame| matches!(frame, Frame::ViewEvent(_)));
  let Frame::ViewEvent(ViewEvent { node: added, kind, value, .. }) = submitted else {
    unreachable!()
  };
  assert_eq!(
    (added.as_str(), kind, value.as_deref()),
    ("add", Gesture::Submit, Some("walk the dog"))
  );
  assert_eq!(skill.ask(tree(&["buy milk", "walk the dog"], 2)), Answer::Done);
  shows(&app, "• walk the dog");
  // The model of the thread answers while the view lives, so the capture waits for its end.
  shows(&app, "✓ Resolved");
  screen("a view of a skill", &app);
}

#[test]
fn go_to_names_each_session_that_runs_and_a_pause_ends_its_life() {
  let (app, _daemon, _) = started();
  sent(&app, &format!("work on it {WORKS}"));
  paged(&app);
  app.press("ctrl+o");
  shows(&app, "working · here");
  app.press("escape");
  app.press("ctrl+x");
  app.press("s");
  shows(&app, "Kill it: its work stops");
  app.type_text("pause");
  app.press("enter");
  app.press("ctrl+o");
  let ended = app.wait_until(PATIENCE, || {
    app.press("escape");
    app.press("ctrl+o");
    !app.text().contains("working · here") && app.text().contains("Go to")
  });
  assert!(ended, "{}", app.text());
}

#[test]
fn a_session_of_go_to_takes_a_kill_which_stops_its_work() {
  let (app, _daemon, _) = started();
  sent(&app, &format!("work on it {WORKS}"));
  paged(&app);
  app.press("ctrl+o");
  shows(&app, "working · here");
  app.type_text("working here");
  app.press("enter");
  shows(&app, "Kill it: its work stops");
  app.type_text("kill");
  app.press("enter");
  let stopped = app.wait_until(PATIENCE, || {
    app.press("escape");
    app.press("ctrl+o");
    !app.text().contains("working · here") && app.text().contains("Go to")
  });
  assert!(stopped, "{}", app.text());
}

#[test]
fn a_leave_while_a_session_works_asks_and_keeps_it_running_in_the_background() {
  let (app, daemon, _) = started();
  sent(&app, &format!("work on it {WORKS}"));
  paged(&app);
  app.press("ctrl+d");
  app.press("ctrl+d");
  let asked = shows(&app, "Leave the window");
  assert!(!app.app().is_quitting(), "{asked}");
  app.type_text("keep");
  app.press("enter");
  assert!(app.wait_until(PATIENCE, || app.app().is_quitting()), "{}", app.text());
  let Answer::Lives(lives) = daemon.peer().ask(Verb::List { config: None }) else { unreachable!() };
  let pinned = lives.iter().any(|one| one.held && one.pins == ["background"]);
  assert!(pinned, "{lives:?}");
}

#[test]
fn a_leave_names_a_thread_that_asks_him_and_keeps_its_session_in_the_background() {
  let (app, daemon, _) = started();
  sent(&app, &format!("which key sends the queue {ASKS}"));
  shows(&app, "◆ Input");
  app.press("ctrl+d");
  app.press("ctrl+d");
  let asked = shows(&app, "Leave the window");
  let row = asked.lines().find(|line| line.contains("FT which key sends the queue"));
  assert!(row.is_some_and(|line| line.contains("◆ Input")), "{asked}");
  assert!(asked.contains("keep it running in the background"), "{asked}");
  app.press("enter");
  assert!(app.wait_until(PATIENCE, || app.app().is_quitting()), "{}", app.text());
  let Answer::Lives(lives) = daemon.peer().ask(Verb::List { config: None }) else { unreachable!() };
  let pinned = lives.iter().any(|one| one.held && one.pins == ["background"]);
  assert!(pinned, "{lives:?}");
}

#[test]
fn a_leave_asks_nothing_of_a_session_that_another_window_still_views() {
  let (app, daemon, sessions) = started();
  sent(&app, &format!("work on it {WORKS}"));
  paged(&app);
  let mut records = fs::read_dir(&sessions).unwrap().flatten().map(|one| one.path());
  let life = records.find(|one| one.extension().is_some_and(|ext| ext == "jsonl")).unwrap();
  // A second window of the operator views the session, so the window that leaves is not its last.
  let mut other = daemon.peer();
  other.opened(&life, true);
  // The daemon pushed the presence of both windows to this one before it answered the open: the
  // app reads that frame in a tick.
  app.wait_until(Duration::from_millis(200), || false);
  app.press("ctrl+d");
  app.press("ctrl+d");
  let left = app.wait_until(PATIENCE, || app.app().is_quitting());
  assert!(left, "the leave asked:\n{}", app.text());
}

#[test]
fn a_slash_offers_the_commands_and_runs_the_one_that_he_takes() {
  let (app, _daemon, _) = started();
  app.type_text("/mod");
  shows(&app, "/model");
  app.press("enter");
  let shown = shows(&app, "Search a model");
  assert!(!shown.contains("/mod"), "the command leaves the input:\n{shown}");
}

#[test]
fn go_to_opens_on_the_command_key_too_and_runs_a_command_as_its_slash_does() {
  let (app, _daemon, _) = started();
  app.press("super+p");
  shows(&app, "Search a thread, a session");
  app.type_text("the model of the input");
  app.press("enter");
  shows(&app, "Search a model");
}

#[test]
fn a_click_on_the_list_keeps_the_keys_with_the_input() {
  let (app, _daemon, _) = started();
  sent(&app, &format!("how long does an open take {ANSWERS}"));
  shows(&app, "Resolved 1");
  clicked(&app, "Resolved 1", LEFT, 0);
  app.type_text("and the seed?");
  assert_eq!(typed(&app, "and the seed?"), "and the seed?");
}

/// The mouse alone runs each command: a click opens Go to, the wheel reaches a row that the list
/// part held out of view, and a click on it runs it.
#[test]
fn the_mouse_alone_reaches_and_runs_each_command_of_go_to() {
  let (app, _daemon, _) = started();
  clicked(&app, "go to anything", 0, 0);
  shows(&app, "Search a thread, a session");
  assert!(!app.text().contains("The model of the input"), "the row is in view before the wheel");
  let (x, y) = at(&app, "Each key", 0);
  for _ in 0..30 {
    app.scroll(x, y, true);
  }
  shows(&app, "The model of the input");
  clicked(&app, "The model of the input", 0, 0);
  shows(&app, "Search a model");
}
