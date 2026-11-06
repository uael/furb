use std::time::Duration;

use furb::view::{Act, Kind, Pause, Phase, State};
use serde_json::{Value, json};

use super::Quiet;
use crate::event::Event;

fn act(kind: &str, id: &str, chain: &str, words: &[Value]) -> Act {
  Act {
    kind: Kind::from(kind),
    id: id.to_owned(),
    by: String::new(),
    on: chain.to_owned(),
    chain: chain.to_owned(),
    words: words.to_vec(),
    phase: Phase::Started,
    operator: false,
    pause: None,
    run: None,
    word: None,
    began: 0,
    ended: None,
    spend: None,
    state: State::Idle,
  }
}

fn heard(quiet: &mut Quiet, act: Act) {
  quiet.heard(&Event::Act(Box::new(act)));
}

fn finished(mut one: Act) -> Act {
  one.phase = Phase::Done(Value::Null);
  one
}

fn quiet() -> Quiet {
  Quiet::new(Duration::from_secs(60))
}

#[test]
fn a_rung_that_nothing_lives_beside_on_its_chain_arms_the_watch() {
  let mut quiet = quiet();
  assert!(quiet.due().is_none());
  heard(&mut quiet, act("rung", "rung2", "chain1", &[json!("x = 1"), json!("prompt1"), json!("")]));
  assert!(quiet.due().is_some());
  let (open, notice) = quiet.stalled();
  assert_eq!(open.len(), 1);
  assert!(notice.contains("rung rung2 on chain1"), "{notice}");
}

#[test]
fn a_rung_beside_an_open_run_reply_or_command_leaves_the_watch_unarmed_until_it_ends() {
  for kind in ["reply", "bash", "wait"] {
    let mut quiet = quiet();
    heard(&mut quiet, act("rung", "rung2", "chain1", &[json!(""), json!(""), json!("")]));
    heard(&mut quiet, act(kind, "act3", "chain1", &[]));
    assert!(!quiet.open().iter().any(|one| one.act == "rung2"), "{kind}");
    heard(&mut quiet, finished(act(kind, "act3", "chain1", &[])));
    assert!(quiet.open().iter().any(|one| one.act == "rung2"), "{kind}");
  }
}

#[test]
fn a_paused_rung_waits_on_purpose() {
  let mut quiet = quiet();
  let mut one = act("rung", "rung2", "chain1", &[json!(""), json!(""), json!("")]);
  one.pause = Some(Pause { by: "operator".to_owned(), why: None });
  heard(&mut quiet, one);
  assert!(quiet.due().is_none());
}

#[test]
fn a_prompt_to_a_model_with_no_rung_on_its_chain_arms_the_watch() {
  let mut quiet = quiet();
  let ask = |to: &str| act("prompt", "prompt4", "chain5", &[json!("str"), json!("q"), json!(to)]);
  heard(&mut quiet, ask("operator"));
  assert!(quiet.due().is_none(), "a prompt to the operator waits for him");
  heard(&mut quiet, finished(ask("operator")));
  heard(&mut quiet, ask("claude-cli:opus"));
  let (open, notice) = quiet.stalled();
  assert_eq!(open.len(), 1);
  assert!(notice.contains("prompt prompt4 on chain5"), "{notice}");
}

#[test]
fn a_prompt_to_a_model_beside_a_rung_arms_the_watch_by_the_rung_alone() {
  let mut quiet = quiet();
  let ask =
    act("prompt", "prompt4", "chain5", &[json!("str"), json!("q"), json!("claude-cli:opus")]);
  heard(&mut quiet, ask);
  heard(&mut quiet, act("rung", "rung6", "chain5", &[json!(""), json!(""), json!("")]));
  let open: Vec<String> = quiet.open().into_iter().map(|one| one.act).collect();
  assert_eq!(open, ["rung6"]);
}
