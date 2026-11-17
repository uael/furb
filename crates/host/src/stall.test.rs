//! The law of speed of the watch: a look reads each act once, so a large life keeps its thread. The
//! suite of the host proves each stall that the watch tells, through the daemon.

use std::{sync::Arc, time, time::Duration};

use furb::view::{Act, Kind, Maker, Phase, State};
use serde_json::{Value, json};

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
