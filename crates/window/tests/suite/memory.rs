//! The memory of a window: what a window holds once it opens a thread of a life, which the size of
//! the life leaves as it is, since a window reads the cards and the page of the open thread, and no
//! fact. The test counts each byte that the process holds, the daemon of the test among them, so
//! what the window holds is what the process holds more once the window opened the thread.
#![cfg(test)]

use std::time::{Duration, Instant};

use furb_host::protocol::{Answer, Verb};
use furb_window::{open, tokens::AWAY};
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  counted::Counted,
  daemon::{ANSWERS, Daemon, PATIENCE},
};

/// The bytes by which the heap of a process that does nothing may move.
const CALM: usize = 64 << 10;

/// How long the heap of a process that does nothing stays within [`CALM`].
const STILL: Duration = Duration::from_millis(200);

/// The most that a window holds once it opened a thread: ten times what it held on 11-17, 0.4 MiB.
const MOST: usize = 4 << 20;

/// The most that a window may hold more on a life four times as large: the daemon folds the facts
/// of a life for a page in its own time, and no part of what the window holds grows with them.
const MORE: usize = 1 << 20;

/// How many numbers one word of the test draws: each draw is an act, which the life tells as two
/// facts.
const DRAWS: usize = 1_000;

/// What a window holds once it opens the thread of a life that drew so many numbers after its
/// message, and what the process held before the window, the life of the daemon among it.
fn held(draws: usize) -> (usize, usize) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  // The peer holds the life by a pin and reads none of its facts.
  let life = peer.opened(&sessions.join("drawn.jsonl"), false);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  for _ in 0..draws / DRAWS {
    let word = format!("for i in range({DRAWS}):\n  chance()");
    let Answer::Act(rung) = peer.ask(Verb::Run { life: life.clone(), word, on: None }) else {
      panic!("the word made no rung")
    };
    let ended = peer.ask(Verb::Ended { life: life.clone(), act: rung });
    assert!(matches!(ended, Answer::Acts(_)), "{ended:?}");
  }
  let before = settled(&app);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "Two stages hold most");
  app.advance(AWAY);
  shows(&app, "Resolved 1");
  (settled(&app).saturating_sub(before), before)
}

/// The bytes that the process holds once they move no more: the daemon writes the image of a life
/// and folds its facts for a window in its own time, so a heap that moved less than [`CALM`] for
/// [`STILL`] says that it is done.
fn settled(app: &TestApp) -> usize {
  let (mut last, mut since) = (Counted::live(), Instant::now());
  let calm = app.wait_until(PATIENCE, || {
    let now = Counted::live();
    if now.abs_diff(last) >= CALM {
      (last, since) = (now, Instant::now());
    }
    since.elapsed() >= STILL
  });
  assert!(calm, "the heap of the process never settled: {last} bytes");
  Counted::live()
}

#[test]
fn a_window_holds_no_more_on_a_life_four_times_as_large() {
  let (small, life) = held(DRAWS);
  let (large, larger) = held(4 * DRAWS);
  println!("a window holds {small} bytes on a life of {life} bytes, {large} on one of {larger}");
  // The process held 20 MiB before the window on the smaller life and 40 MiB on the larger on 11-17.
  assert!(larger > life + (10 << 20), "the larger life holds {larger} bytes, the smaller {life}");
  assert!(small < MOST && large < MOST, "a window holds {small} and {large} bytes, past {MOST}");
  assert!(
    large < small + MORE,
    "a window holds {large} bytes on the larger life, {small} on the other"
  );
}
