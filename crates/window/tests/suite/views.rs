//! The views that a word draws for the operator, on a real daemon: a prompt to him whose message
//! is a tree of elements, which the window draws in its slot, and whose answer is what a person or
//! a model did to one of its nodes.

#![cfg(test)]

use furb_host::protocol::Verb;
use furb_window::open;

use crate::{
  cards::{app, options, row, shows},
  daemon::{Daemon, PATIENCE},
  thread::clicked,
};

/// The word that draws a view of a text and a button, on a chain of its own, as `v`.
const DRAWS: &str = r#"import json
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
  peer.ask(run(DRAWS));
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
