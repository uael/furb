use std::time::Duration;

use furb::view::{Act, Kind, Pause, Phase, State};
use serde_json::{Value, json};
use tokio::time::Instant;

use super::{Quiet, STALL};
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
    asking: false,
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

fn paused(mut one: Act) -> Act {
  one.pause = Some(Pause { by: "operator".to_owned(), why: None });
  one
}

fn quiet() -> Quiet {
  Quiet::new(Duration::from_secs(60))
}

fn after(seconds: u64) -> Instant {
  Instant::now() + Duration::from_secs(seconds)
}

/// The name of each act that a stall tells at a time, with the chain that takes the stall.
fn told(quiet: &mut Quiet, at: Instant) -> Vec<(String, Option<String>)> {
  let stalls = quiet.stalled(at);
  stalls
    .into_iter()
    .flat_map(|one| one.acts.into_iter().map(move |act| (act.act, one.chain.clone())))
    .collect()
}

fn rung(id: &str, chain: &str) -> Act {
  act("rung", id, chain, &[json!(""), json!(""), json!("")])
}

#[test]
fn a_rung_that_nothing_lives_beside_on_its_chain_arms_the_watch() {
  let mut quiet = quiet();
  assert!(quiet.due().is_none());
  heard(&mut quiet, act("rung", "rung2", "chain1", &[json!("x = 1"), json!("prompt1"), json!("")]));
  assert!(quiet.due().is_some());
  let stalls = quiet.stalled(after(61));
  assert_eq!(stalls.len(), 1);
  assert_eq!(stalls[0].chain.as_deref(), Some("chain1"));
  assert!(stalls[0].notice.contains("rung rung2 on chain1"), "{}", stalls[0].notice);
}

#[test]
fn a_rung_beside_an_open_run_reply_or_command_leaves_the_watch_unarmed_until_it_ends() {
  for kind in ["reply", "bash", "wait"] {
    let mut quiet = quiet();
    heard(&mut quiet, rung("rung2", "chain1"));
    heard(&mut quiet, act(kind, "act3", "chain1", &[]));
    assert!(!told(&mut quiet, after(3600)).iter().any(|(one, _)| one == "rung2"), "{kind}");
    heard(&mut quiet, finished(act(kind, "act3", "chain1", &[])));
    assert!(told(&mut quiet, after(3600)).iter().any(|(one, _)| one == "rung2"), "{kind}");
  }
}

#[test]
fn a_paused_rung_waits_on_purpose() {
  let mut quiet = quiet();
  heard(&mut quiet, paused(rung("rung2", "chain1")));
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
  let stalls = quiet.stalled(after(61));
  assert_eq!(stalls.len(), 1);
  assert!(stalls[0].notice.contains("prompt prompt4 on chain5"), "{}", stalls[0].notice);
}

#[test]
fn a_prompt_to_a_model_beside_a_rung_arms_the_watch_by_the_rung_alone() {
  let mut quiet = quiet();
  let ask =
    act("prompt", "prompt4", "chain5", &[json!("str"), json!("q"), json!("claude-cli:opus")]);
  heard(&mut quiet, ask);
  heard(&mut quiet, rung("rung6", "chain5"));
  assert_eq!(told(&mut quiet, after(61)), [("rung6".to_owned(), Some("chain5".to_owned()))]);
}

#[test]
fn a_chain_stuck_beside_a_chain_that_works_stalls_after_a_minute_and_takes_the_message_alone() {
  let mut quiet = Quiet::new(STALL);
  heard(&mut quiet, rung("rung2", "chain2"));
  let stuck = Instant::now();
  heard(&mut quiet, act("run", "run3", "chain3", &[]));
  for n in 0..5 {
    quiet.heard(&Event::Fact(json!(["tell", "run3", "chain3", [format!("line {n}")]])));
    heard(&mut quiet, act("run", "run3", "chain3", &[]));
  }
  let due = quiet.due().expect("the stuck chain arms the watch");
  assert!(due <= stuck + Duration::from_secs(61), "the chain stalls after a minute");
  assert!(quiet.stalled(stuck + Duration::from_secs(30)).is_empty());
  let stalls = quiet.stalled(stuck + Duration::from_secs(61));
  assert_eq!(stalls.len(), 1);
  assert_eq!(stalls[0].chain.as_deref(), Some("chain2"), "the message goes to the stuck chain");
  assert_eq!(stalls[0].acts.iter().map(|one| one.act.as_str()).collect::<Vec<_>>(), ["rung2"]);
  assert!(stalls[0].notice.contains("1 minute"), "{}", stalls[0].notice);
  let next = quiet.due().expect("the run keeps the bound of the life");
  assert!(next >= stuck + STALL - Duration::from_secs(1), "the chain is told once");
}

#[test]
fn the_bound_of_the_life_holds_each_run_reply_and_command() {
  let mut quiet = Quiet::new(STALL);
  for (kind, id) in [("run", "run1"), ("reply", "reply2"), ("bash", "bash3")] {
    heard(&mut quiet, act(kind, id, "chain1", &[]));
  }
  assert!(quiet.stalled(after(61)).is_empty(), "a run, a reply or a command waits for the life");
  let stalls = quiet.stalled(after(STALL.as_secs() + 1));
  assert_eq!(stalls.len(), 1);
  assert_eq!(stalls[0].chain, None, "the stall of the life goes to the root");
  assert_eq!(stalls[0].acts.len(), 3);
  assert!(stalls[0].notice.contains("20 minutes"), "{}", stalls[0].notice);
}

#[test]
fn a_prompt_to_the_operator_or_a_paused_act_never_stalls_its_chain() {
  let mut quiet = Quiet::new(STALL);
  let ask = |id: &str, chain: &str, to: &str| {
    act("prompt", id, chain, &[json!("str"), json!("q"), json!(to)])
  };
  heard(&mut quiet, ask("prompt1", "chain5", "operator"));
  heard(&mut quiet, paused(rung("rung2", "chain6")));
  heard(&mut quiet, paused(ask("prompt3", "chain7", "claude-cli:opus")));
  assert!(quiet.due().is_none());
  assert!(quiet.stalled(after(3 * STALL.as_secs())).is_empty());
}

#[test]
fn the_message_of_a_chain_tells_nothing_new_until_an_act_that_it_did_not_make_changes() {
  let mut quiet = quiet();
  heard(&mut quiet, rung("rung2", "chain2"));
  let stalls = quiet.stalled(after(61));
  let notice = &stalls[0].notice;
  quiet
    .heard(&Event::Fact(json!(["prompt", "prompt9", "operator", "chain2", "None", notice, null])));
  let ask = act("prompt", "prompt9", "chain2", &[json!("None"), json!(notice), Value::Null]);
  heard(&mut quiet, ask.clone());
  quiet.heard(&Event::Fact(json!(["rung", "rung10", "prompt9", "chain2", "", "", ""])));
  heard(&mut quiet, rung("rung10", "chain2"));
  assert!(quiet.due().is_none(), "an act that the message made is no new fact");
  heard(&mut quiet, finished(ask));
  heard(&mut quiet, finished(rung("rung10", "chain2")));
  assert!(quiet.due().is_none());
  heard(&mut quiet, rung("rung2", "chain2"));
  assert!(quiet.due().is_some(), "a new fact on the chain arms the watch again");
}
