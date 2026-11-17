//! The memory of a window: what a window holds once it opens a thread of a life, which the size of
//! the life leaves as it is, since a window reads the cards and the page of the open thread, and no
//! fact. The test counts each byte that the process holds, the daemon of the test among them, so
//! what the window holds is what the process holds more once the window opened the thread.
#![cfg(test)]

use std::{
  env::consts::OS,
  fs::File,
  time::{Duration, Instant},
};

use furb_host::protocol::{Answer, Verb};
use furb_window::{open, tokens::AWAY};
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  counted::Counted,
  daemon::{ANSWERS, Daemon, MARKDOWN, PATIENCE, Peer, STREAMED, STREAMS},
  here::Here,
};

/// The most allocations that a gesture makes, on the system that measured it. The count holds each
/// allocation of the process, std and the allocator of the system among them, which differ from
/// one system to the next, so a bound holds on its own system alone. A system with no bound yet
/// prints its count, from which its bound is set, and bounds nothing.
type Bound = Option<usize>;

/// A bound measured on macOS.
const fn on_macos(most: usize) -> Bound {
  if cfg!(target_os = "macos") { Some(most) } else { None }
}

/// A count of allocations held to its bound on this system, or printed where none is set.
fn bounded(count: usize, bound: Bound, what: &str) {
  if let Some(most) = bound {
    assert!(count <= most, "{what} made {count} allocations on {OS}, past {most}");
  } else {
    println!("{what} makes {count} allocations on {OS}, which no bound holds yet");
  }
}

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
  // The process held 20 MiB before the window on the smaller life and 40 MiB on the larger, on
  // 11-17.
  assert!(larger > life + (10 << 20), "the larger life holds {larger} bytes, the smaller {life}");
  assert!(small < MOST && large < MOST, "a window holds {small} and {large} bytes, past {MOST}");
  assert!(
    large < small + MORE,
    "a window holds {large} bytes on the larger life, {small} on the other"
  );
}

/// The most allocations that a move of the pointer makes, from one place of a thread at rest to
/// another that lights nothing: 23 on macOS on 11-17, where each such move laid out the list and
/// the left side again, with 900, and copied the folder and the life of the target of the input,
/// with 41, before.
const PER_MOVE: Bound = on_macos(26);

#[test]
fn a_move_of_the_pointer_that_changes_nothing_asks_no_frame_and_lays_out_nothing() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("still.jsonl"), false);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "Two stages hold most");
  app.advance(AWAY);
  shows(&app, "Resolved 1");
  settled(&app);
  // Blank cells of the feed, under its text and over the input.
  let blank = [(30, 19), (31, 19), (30, 20), (5, 20)];
  app.hover(5, 19);
  app.render();
  let calls = Here::calls();
  for (x, y) in blank {
    app.hover(x, y);
    assert!(!app.needs_frame(), "a move to {x},{y} asks a frame:\n{}", app.text());
  }
  let moves = (Here::calls() - calls) / blank.len();
  println!("a move of the pointer that changes nothing makes {moves} allocations");
  bounded(moves, PER_MOVE, "a move of the pointer");
}

/// The most allocations that a key typed in the input makes, with its frame, on a thread at rest:
/// 176 on macOS on 11-17, where each key laid out each part of the feed and the list again and
/// copied each row, with 1,587, asked the renderer to measure each character and copied the
/// folder and the life of the target of the input, with 413, and copied each run of a row that
/// fits its room, laid out the box of the context to find the row under the pointer, and wrote
/// each line of the input to count them, with 296, before.
const PER_KEY: Bound = on_macos(190);

#[test]
fn a_key_typed_in_the_input_lays_out_no_part_of_the_feed_again() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("typed.jsonl"), false);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "Two stages hold most");
  app.advance(AWAY);
  shows(&app, "Resolved 1");
  app.type_text("a");
  app.advance(AWAY);
  settled(&app);
  // A machine tells the window what it changed in its own time, so the least of the keys is the
  // cost of a key alone.
  let keys = "bcdefghijk";
  let typed = keys.chars().map(|key| {
    let calls = Here::calls();
    app.type_text(key.encode_utf8(&mut [0; 4]));
    app.present().expect("a frame");
    Here::calls() - calls
  });
  let typed = typed.min().unwrap_or_default();
  println!("a key typed in the input makes {typed} allocations");
  assert!(app.text().contains("abcdefghijk"), "{}", app.text());
  bounded(typed, PER_KEY, "a key typed in the input");
}

/// How long a window hears no frame once a part of a reply showed, so the daemon told the whole of
/// the part.
const QUIET: Duration = Duration::from_millis(100);

/// The answers above a reply that streams, as the words that make the model answer them and how
/// many: a short answer, a long answer that fills the feed above the reply, and fifty of them.
const ABOVE: [(&str, usize); 3] = [(ANSWERS, 1), (MARKDOWN, 1), (MARKDOWN, 50)];

/// How many allocations more than under the short answer a part of a reply may make under the
/// others: a part lays out the part that it changes, which is the same under each, and no other.
const GROWTH: usize = 64;

#[test]
fn a_part_of_a_reply_lays_out_only_the_part_that_it_changes() {
  let parts = ABOVE.map(|(answer, count)| streamed_part(answer, count));
  println!("a part of a reply under {ABOVE:?} makes {parts:?} allocations");
  let least = parts.iter().copied().min().unwrap_or_default();
  assert!(
    parts.iter().all(|one| *one <= least + GROWTH),
    "a part makes {parts:?} allocations under {ABOVE:?}: it lays out the parts above it again"
  );
}

/// The allocations that the window makes for a part of a reply that streams into a thread under a
/// count of answers, with each frame that the part takes: the middle of its parts after the first,
/// which opens the block of its word. Each part makes a few more than the one before, as the block
/// of the reply grows.
fn streamed_part(answer: &str, count: usize) -> usize {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("streams.jsonl"), false);
  let first = peer.said(&life, &format!("show the answer {answer}"));
  for more in 1..count {
    asked(&mut peer, &life, &first, format!("show it again {answer}"));
    answered(&mut peer, (&life, &first), more + 1);
  }
  asked(&mut peer, &life, &first, format!("where does the time go {STREAMS}"));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "show the answer"));
  shows(&app, "said = '''");
  settled(&app);
  let (process, parts): (Vec<usize>, Vec<usize>) = (0..STREAMED)
    .map(|part| {
      let calls = (Counted::calls(), Here::calls());
      File::create(daemon.file(&format!("part{part}"))).expect("a part");
      assert!(app.wait_until(PATIENCE, || app.needs_frame()), "part {part} shows nothing");
      app.present().expect("a frame");
      // The daemon tells a part in more than one frame, each in its own time.
      while app.wait_until(QUIET, || app.needs_frame()) {
        app.present().expect("a frame");
      }
      (Counted::calls() - calls.0, Here::calls() - calls.1)
    })
    .unzip();
  println!("under {count} of {answer}, the parts make {parts:?} in the window, {process:?} in all");
  shows(&app, &format!("part {} of", STREAMED - 1));
  let mut after: Vec<usize> = parts.iter().skip(1).copied().collect();
  after.sort_unstable();
  after.get(after.len() / 2).copied().unwrap_or_default()
}

/// A message said on a thread.
fn asked(peer: &mut Peer, life: &str, thread: &str, message: String) {
  let (life, on) = (life.to_owned(), Some(thread.to_owned()));
  peer.ask(Verb::Prompt { life, shape: "None".to_owned(), message, to: None, on });
}

/// The thread once it holds a count of exchanges, the last of which said its answer, as the page of
/// its end says it: each ask waits for the answer of the daemon.
fn answered(peer: &mut Peer, (life, thread): (&str, &str), exchanges: usize) {
  let start = Instant::now();
  loop {
    let (life, thread) = (life.to_owned(), thread.to_owned());
    let verb = Verb::Thread { life, thread, before: None, limit: 1, at: None };
    let Answer::Page(page) = peer.ask(verb) else { panic!("no page") };
    let said = page.exchanges.last().is_some_and(|one| !one.said.is_empty());
    if page.above + page.exchanges.len() == exchanges && said {
      return;
    }
    assert!(start.elapsed() < PATIENCE, "the thread holds {} exchanges", page.above + 1);
  }
}

/// The most allocations that a step of the wheel makes on a long thread, with its frame: 414 on
/// macOS on 11-17.
const PER_STEP: Bound = on_macos(460);

#[test]
fn a_step_of_the_wheel_on_a_long_thread_stays_in_its_bound() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("long.jsonl"), false);
  peer.said(&life, &format!("show each part of markdown {MARKDOWN}"));
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "show each part of"));
  shows(&app, "What a step can break");
  app.advance(AWAY);
  settled(&app);
  let steps: Vec<usize> = (0..10)
    .map(|_| {
      let calls = Here::calls();
      app.scroll(30, 10, false);
      app.present().expect("a frame");
      Here::calls() - calls
    })
    .collect();
  println!("the steps of the wheel make {steps:?} allocations");
  assert!(!app.text().contains("What a step can break"), "{}", app.text());
  let step = steps.iter().copied().min().unwrap_or_default();
  println!("a step of the wheel makes {step} allocations");
  bounded(step, PER_STEP, "a step of the wheel");
}
