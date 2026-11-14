//! The form of a question, on a real daemon whose model asks the operator from a script: the form
//! takes the place of the input in the thread that asks, and a question of another thread is a mark
//! on its card.

#![cfg(test)]

use std::time::Duration;

use furb_host::protocol::{Answer, Verb};
use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  daemon::{
    ANSWERS, ASKS, CHOOSES, CONFIRMS, COUNTS, Daemon, PATIENCE, Peer, READ, TELLS, TICKS, TWICE,
  },
  screens::screen,
  thread::{at, lights},
};

/// How long a form takes no key after it shows.
const HOLD: Duration = Duration::from_millis(400);

/// The head of each form, which names who asks: the model of the test.
const ASKER: &str = "◆ opus/high asks";

/// The columns of the left side of the window, at the left of the list.
const LEFT: usize = 61;

/// A window on a machine with one session whose thread says a message, opened by a click on its
/// card, once the thread shows a text. The life of the thread and its prompt.
fn opened(message: &str, shown: &str) -> (TestApp, Daemon, Peer, String, String) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("side.jsonl"), true);
  let prompt = peer.said(&life, message);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  let title = message.lines().next().unwrap();
  app.click(80, row(&app, &title[..title.len().min(20)]));
  shows(&app, shown);
  (app, daemon, peer, life, prompt)
}

/// A window whose open thread asks the operator, once its form shows and takes keys.
fn asked(message: &str) -> (TestApp, Daemon, Peer) {
  let (app, daemon, peer, ..) = opened(message, ASKER);
  app.advance(HOLD);
  (app, daemon, peer)
}

/// The frame of a window once a row of its left side holds a text alone.
fn said(app: &TestApp, text: &str) -> String {
  let alone = |line: &str| line.chars().take(LEFT).collect::<String>().trim() == text;
  let shown = app.wait_until(PATIENCE, || app.text().lines().any(alone));
  assert!(shown, "the window never showed {text:?} alone:\n{}", app.text());
  app.text()
}

/// Whether the window shows no form.
fn formless(app: &TestApp) -> bool {
  !app.text().contains(ASKER)
}

#[test]
fn a_choice_is_answered_by_its_key_and_the_word_that_asked_goes_on() {
  let (app, _daemon, _peer, ..) =
    opened(&format!("where does the list stand\n{CHOOSES}"), "type another answer");
  screen("question-choice", &app);
  // A key in the first moments of the form answers nothing, and moves nothing.
  app.press("down");
  app.press("enter");
  app.advance(HOLD);
  let frame = app.text();
  assert!(frame.contains("› right") && frame.contains(ASKER), "{frame}");
  app.press("down");
  shows(&app, "› left");
  app.press("enter");
  // The question stays as one line: what closed it and who. The word went on, and closed its
  // thread with the value that he chose.
  shows(&app, "left · you");
  said(&app, "left");
  shows(&app, "● now");
  assert!(formless(&app), "{}", app.text());
  assert!(app.text().contains("┃ Reply to this thread"), "{}", app.text());
  screen("question-answered", &app);
}

#[test]
fn a_click_on_a_choice_answers_it() {
  let (app, _daemon, _peer) = asked(&format!("where does the list stand\n{CHOOSES}"));
  let (x, y) = at(&app, "┃   left", 0);
  lights(&app, x + 4, y);
  app.click(x + 4, y);
  shows(&app, "left · you");
}

#[test]
fn his_own_words_on_the_last_row_reach_the_word_that_asked_as_a_refusal() {
  let (app, _daemon, _peer) = asked(&format!("where does the list stand\n{CHOOSES}"));
  // A letter on a row of a value writes nothing.
  app.type_text("x");
  app.press("down");
  app.press("down");
  app.type_text("at the top");
  shows(&app, "› at the top");
  screen("question-own", &app);
  app.press("enter");
  shows(&app, "refused: at the top · you");
}

#[test]
fn a_truth_is_answered_yes_or_no() {
  let (app, _daemon, _peer) = asked(&format!("is the bar orange\n{CONFIRMS}"));
  let frame = app.text();
  assert!(frame.contains("› yes") && frame.contains("  no"), "{frame}");
  app.press("enter");
  shows(&app, "yes · you");
  said(&app, "True");
}

#[test]
fn values_to_tick_are_answered_as_a_list() {
  let (app, _daemon, _peer) = asked(&format!("which keys does the form take\n{TICKS}"));
  app.press("space");
  app.press("down");
  app.press("down");
  app.press("space");
  shows(&app, "› [x] tab");
  let frame = app.text();
  assert!(frame.contains("  [x] up") && frame.contains("  [ ] down"), "{frame}");
  screen("question-ticks", &app);
  app.press("enter");
  said(&app, "up tab");
}

#[test]
fn a_line_that_the_shape_refuses_comes_back_with_why_and_his_text_stays() {
  let (app, _daemon, _peer) = asked(&format!("how many rows\n{COUNTS}"));
  shows(&app, "Write a whole number");
  app.type_text("many");
  app.press("enter");
  shows(&app, "\"many\" is no int");
  assert!(app.text().contains("┃ many"), "{}", app.text());
  screen("question-refused", &app);
  app.press("ctrl+c");
  shows(&app, "Write a whole number");
  app.type_text("6");
  app.press("enter");
  shows(&app, "6 · you");
}

#[test]
fn ctrl_c_empties_the_form_then_refuses_the_question_and_the_model_reads_the_refusal() {
  let (app, _daemon, _peer) = asked(&format!("how many rows\n{COUNTS}"));
  app.type_text("12");
  shows(&app, "┃ 12");
  app.press("ctrl+c");
  shows(&app, "Write a whole number");
  assert!(!formless(&app), "{}", app.text());
  app.press("ctrl+c");
  shows(&app, "refused · you");
  shows(&app, READ);
}

#[test]
fn a_question_that_lands_while_he_writes_keeps_his_draft() {
  let (app, _daemon, mut peer, life, prompt) =
    opened(&format!("how long does an open take\n{ANSWERS}"), "Two stages hold most");
  app.paste("the bar is too wide");
  shows(&app, "┃ the bar is too wide");
  let message = format!("and the side\n{CHOOSES}");
  let verb = Verb::Prompt { life, shape: "None".to_owned(), message, to: None, on: Some(prompt) };
  assert!(matches!(peer.ask(verb), Answer::Act(_)));
  shows(&app, "A question waits");
  // The card of the thread, which asks again, as its machine tells it after the page.
  let frame = shows(&app, "◆ Input");
  assert!(frame.contains("┃ the bar is too wide") && formless(&app), "{frame}");
  screen("question-waits", &app);
  // His message goes, and the form shows once the input is empty.
  app.press("enter");
  shows(&app, ASKER);
  assert!(!app.text().contains("A question waits"), "{}", app.text());
}

#[test]
fn two_questions_of_a_thread_show_one_after_the_other_with_their_count() {
  let (app, _daemon, _peer) = asked(&format!("what does the head say\n{TWICE}"));
  shows(&app, "1 of 2");
  let frame = app.text();
  assert!(frame.contains("┃ Does the head count the questions?"), "{frame}");
  screen("question-two", &app);
  app.press("tab");
  let frame = shows(&app, "┃ What does the head say?");
  assert!(frame.contains("2 of 2"), "{frame}");
  app.press("tab");
  shows(&app, "┃ Does the head count the questions?");
  app.press("enter");
  shows(&app, "yes · you");
  let frame = shows(&app, "┃ What does the head say?");
  assert!(!frame.contains("1 of 2"), "{frame}");
  app.advance(HOLD);
  app.type_text("its count");
  app.press("enter");
  said(&app, "True its count");
}

#[test]
fn a_question_of_another_thread_marks_its_card_and_shows_no_form() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let rests = peer.opened(&sessions.join("rests.jsonl"), true);
  peer.said(&rests, &format!("how long does an open take\n{ANSWERS}"));
  let asks = peer.opened(&sessions.join("asks.jsonl"), true);
  peer.said(&asks, &format!("which key sends the queue\n{ASKS}"));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "how long does an ope"));
  shows(&app, "Two stages hold most");
  let frame = shows(&app, "◆ 1 other thread asks");
  assert!(
    frame.contains("Needs you") && frame.contains("which key sends the queue  ◆ Input"),
    "{frame}"
  );
  assert!(formless(&app) && frame.contains("┃ Reply to this thread"), "{frame}");
  screen("question-elsewhere", &app);
  // The count takes a click, which goes to the thread that asks.
  let (x, y) = at(&app, "1 other thread asks", 0);
  lights(&app, x, y);
  app.click(x, y);
  shows(&app, ASKER);
}

#[test]
fn a_notice_is_a_line_of_its_thread_with_no_form() {
  let (app, _daemon, ..) = opened(&format!("run the bench\n{TELLS}"), "The bench ran.");
  let came = app.wait_until(PATIENCE, || app.text().contains("● now"));
  assert!(came && formless(&app), "{}", app.text());
  assert!(app.text().contains("┃ Reply to this thread"), "{}", app.text());
}
