//! The views that a word draws for the operator, on a real daemon: a prompt to him whose message
//! is a tree of elements, which the window draws in its slot, and whose answer is what a person or
//! a model did to one of its nodes.

#![cfg(test)]

use furb_host::protocol::Verb;
use furb_window::open;

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, DRAWS, Daemon, PATIENCE},
  thread::clicked,
};

/// The word that draws a view of a text and a button, on a chain of its own, as `v`.
const VIEW: &str = r#"import json
side = chain('views')
def drawn(said):
  text = {"id": "said", "node": "text", "runs": [{"text": said}]}
  clear = {"id": "clear", "node": "button", "label": "Clear"}
  tree = {"id": "todo", "node": "box", "axis": "column", "gap": 0, "border": True, "children": [text, clear]}
  return json.dumps({"view": 1, "slot": "footer", "tree": tree})
v = prompt(str, drawn("buy milk"), to=OPERATOR, on=side)"#;

#[test]
fn a_view_draws_in_the_footer_and_a_click_or_a_close_of_a_model_answers_its_prompt_alike() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("views.jsonl"), true);
  peer.said(&life, "keep my list");
  let run = |word: &str| Verb::Run { life: life.clone(), word: word.to_owned(), on: None };
  peer.ask(run(VIEW));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "keep my list"));
  shows(&app, "buy milk");
  // A click answers the prompt of the view with the node that it pressed, which the word reads.
  clicked(&app, "Clear", 0, 0);
  let gone = app.wait_until(PATIENCE, || !app.text().contains("buy milk"));
  assert!(gone, "{}", app.text());
  peer.ask(run(
    "w = prompt(str, drawn('pressed ' + json.loads(peek(v))['node']), to=OPERATOR, on=side)",
  ));
  shows(&app, "pressed clear");
  // A model closes the prompt of a view with the same gesture, as the word of a model writes it.
  peer.ask(run(r#"close('{"node": "clear", "value": null}', w)"#));
  let gone = app.wait_until(PATIENCE, || !app.text().contains("pressed clear"));
  assert!(gone, "{}", app.text());
  peer
    .ask(run("prompt(str, drawn('closed ' + json.loads(peek(w))['node']), to=OPERATOR, on=side)"));
  shows(&app, "closed clear");
}

#[test]
fn a_view_that_the_word_of_a_thread_draws_is_no_question_of_that_thread() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("drawn.jsonl"), true);
  let prompt = Verb::Prompt {
    life,
    shape: "None".to_owned(),
    message: format!("keep a list {DRAWS}"),
    to: None,
    on: None,
  };
  peer.ask(prompt);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "keep a list"));
  let shown = shows(&app, "buy milk");
  // The view stands in the footer, under the thread, and the thread asks nothing: no form of a
  // question, which names the model that asks, and no state of a question on its card.
  let (thread, view) = (shown.find("keep a list").unwrap(), shown.rfind("buy milk").unwrap());
  assert!(thread < view, "{shown}");
  assert!(!shown.contains("opus/high asks") && !shown.contains("⏎ answer"), "{shown}");
  let card = peer.card(|card| card.thread.title.contains("keep a list"));
  assert_ne!(serde_json::json!(card.thread.standing)["is"], "asks");
}

#[test]
fn a_message_after_a_pause_opens_a_new_life_whose_thread_and_view_go_on() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("paused.jsonl"), true);
  peer.said(&life, "keep my list");
  peer.ask(Verb::Run { life: life.clone(), word: VIEW.to_owned(), on: None });
  peer.left(&life);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "keep my list"));
  shows(&app, "buy milk");
  app.press("ctrl+x");
  app.press("s");
  shows(&app, "Pause it");
  app.type_text("pause");
  app.press("enter");
  let ended = app.wait_until(PATIENCE, || !app.text().starts_with('●'));
  assert!(ended, "{}", app.text());
  // The message opens a new life on the record, whose thread tells the answer of its model, and
  // the view of the record stands again.
  app.type_text(&format!("what is left {ANSWERS}"));
  app.press("enter");
  let shown = shows(&app, "The open takes");
  assert!(shown.starts_with('●') && shown.contains("buy milk"), "{shown}");
}

#[test]
fn the_keys_reach_each_control_of_a_view_as_a_click_does() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("keys.jsonl"), true);
  peer.said(&life, "keep my list");
  peer.ask(Verb::Run { life: life.clone(), word: VIEW.to_owned(), on: None });
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "keep my list"));
  shows(&app, "buy milk");
  // The leader and V give the keys to the first control, and the row under the view says them.
  app.press("ctrl+x");
  app.press("v");
  shows(&app, "use · ");
  // Escape gives the keys back to the input, and Enter then uses the control that they reach.
  app.press("escape");
  let back = app.wait_until(PATIENCE, || !app.text().contains("use · "));
  assert!(back, "{}", app.text());
  app.press("ctrl+x");
  app.press("v");
  shows(&app, "use · ");
  app.press("enter");
  let gone = app.wait_until(PATIENCE, || !app.text().contains("buy milk"));
  assert!(gone, "{}", app.text());
  let run = |word: &str| Verb::Run { life: life.clone(), word: word.to_owned(), on: None };
  peer
    .ask(run("prompt(str, drawn('pressed ' + json.loads(peek(v))['node']), to=OPERATOR, on=side)"));
  shows(&app, "pressed clear");
}
