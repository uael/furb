//! The law of speed of the watch: a look reads each act once, so a large life keeps its thread. The
//! suite of the host proves each stall that the watch tells, through the daemon, but the stall of a
//! chain that can answer its notice beside an act that stays open, which no door makes.

use std::{sync::Arc, time, time::Duration};

use furb::view::{Act, Kind, Maker, Phase, State};
use serde_json::{Value, json};
use tokio::time::Instant;

use super::Quiet;
use crate::event::Event;

fn act(kind: &str, id: &str, chain: &str, words: &[Value]) -> Act {
  Act {
    kind: Kind::from(kind),
    id: id.into(),
    by: "".into(),
    maker: Maker::Engine,
    on: chain.into(),
    chain: chain.into(),
    words: words.to_vec(),
    phase: Phase::Started,
    operator: false,
    asking: false,
    taken: Vec::new(),
    pause: None,
    run: None,
    word: None,
    began: 0,
    ended: None,
    spend: None,
    closer: None,
    state: State::Idle,
  }
}

fn heard(quiet: &mut Quiet, act: Act) {
  quiet.heard(&Event::Act(Arc::new(act)));
}

fn quiet() -> Quiet {
  Quiet::new(Duration::from_secs(60))
}

/// The least time of a few looks of the watch for when its next stall comes, on a life of `n`
/// chains, each with a rung that has a word and a reply.
fn looked(n: usize) -> Duration {
  let mut quiet = quiet();
  for one in 0..n {
    let chain = format!("chain{one}");
    heard(
      &mut quiet,
      act("rung", &format!("rung{one}"), &chain, &[json!("x"), json!(""), json!("")]),
    );
    heard(&mut quiet, act("reply", &format!("reply{one}"), &chain, &[]));
  }
  (0..5)
    .map(|_| {
      let at = time::Instant::now();
      assert!(quiet.due().is_some());
      at.elapsed()
    })
    .min()
    .unwrap()
}

#[test]
fn the_watch_reads_its_acts_once_for_each_look_so_a_large_life_keeps_its_thread() {
  let (small, large) = (looked(500), looked(4000));
  // Eight times the acts cost about eight times the time; a pass over each act for each act cost
  // about sixty four times.
  assert!(large < small * 24, "500 chains: {small:?}, 4000 chains: {large:?}");
}

/// One act as the life tells it after a fact, in a phase and a state.
fn told(quiet: &mut Quiet, mut one: Act, phase: Phase, state: State) {
  (one.phase, one.state) = (phase, state);
  heard(quiet, one);
}

// No door makes this stall: a reply that stays open blocks its chain, which then cannot answer the
// notice, and an open ends each act that its life does not hold. So the test feeds the watch the
// events of a chain that answers the notice beside an act that stays open.
#[test]
fn a_chain_that_answered_the_notice_of_its_stall_is_told_no_more_while_nothing_new_comes() {
  let mut quiet = quiet();
  told(&mut quiet, act("chain", "chain2", "chain2", &[]), Phase::Started, State::Delegates);
  told(&mut quiet, act("reply", "reply9", "chain2", &[]), Phase::Started, State::Idle);
  let first = Instant::now() + Duration::from_mins(10);
  let stalls: Vec<_> = quiet.stalled(first).into_iter().filter(|one| one.chain.is_some()).collect();
  let [stall] = stalls.as_slice() else { panic!("one stall of chain2: {}", stalls.len()) };
  let notice = stall.notice.clone();
  // The life takes the notice as a prompt on the chain, whose model answers it.
  let said = |quiet: &mut Quiet, fact: Value| quiet.heard(&Event::Fact(fact));
  said(&mut quiet, json!(["prompt", "prompt5", "watch", "chain2", "None", notice, ""]));
  let prompt = act("prompt", "prompt5", "chain2", &[json!("None"), json!(notice), json!("")]);
  told(&mut quiet, prompt.clone(), Phase::Started, State::Idle);
  said(&mut quiet, json!(["rung", "rung7", "prompt5", "chain2", "", "", "claude-cli:opus/high"]));
  told(&mut quiet, act("chain", "chain2", "chain2", &[]), Phase::Started, State::Thinks);
  let rung = act("rung", "rung7", "chain2", &[json!(""), json!(""), json!("")]);
  told(&mut quiet, rung.clone(), Phase::Started, State::Idle);
  said(&mut quiet, json!(["reply", "reply10", "rung7", "chain2", "claude-cli:opus/high"]));
  told(&mut quiet, act("reply", "reply10", "chain2", &[]), Phase::Started, State::Idle);
  told(&mut quiet, act("reply", "reply10", "chain2", &[]), Phase::Done(json!([])), State::Idle);
  told(&mut quiet, rung, Phase::Done(Value::Null), State::Idle);
  told(&mut quiet, prompt, Phase::Done(Value::Null), State::Idle);
  told(&mut quiet, act("chain", "chain2", "chain2", &[]), Phase::Started, State::Delegates);
  let later = first + Duration::from_mins(20);
  let again: Vec<_> = quiet.stalled(later).into_iter().filter(|one| one.chain.is_some()).collect();
  assert!(again.is_empty(), "{:?}", again.iter().map(|one| &one.notice).collect::<Vec<_>>());
}
