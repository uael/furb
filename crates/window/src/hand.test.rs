//! The work of the window, which no frame shows: a test fills the model of a window with no machine
//! and counts its layouts, which only the module that holds the window reaches.

use std::{
  path::PathBuf,
  sync::mpsc,
  time::{Duration, Instant},
};

use furb::view::{self, Block, Exchange, Line, Maker, Mark, Standing, Work};
use furb_client::{Chains, Link, Opened, Thread, Time, Told};
use furb_host::protocol::Card;
use jiff::tz::TimeZone;
use opentui::testing::TestApp;

use super::{Window, windowed};
use crate::{Options, keys::System};

const LIFE: &str = "/furb/b.jsonl";

fn options() -> Options {
  Options {
    folder: PathBuf::from("/furb"),
    home: None,
    kept: PathBuf::from("/furb/cards.json"),
    zone: TimeZone::UTC,
    clock: || 0,
    still: false,
    system: System::MacOs,
    browse: |_| Ok(()),
    edit: |_| Ok(()),
    program: None,
    reloaded: false,
    tailnet: None,
    listen: None,
  }
}

/// A window on its thread, whose answer holds a block of a count of lines, the last of which runs
/// a command, and whose card says that it ended and that the operator did not read it.
fn window(app: &TestApp, lines: usize) -> Window {
  let mut window = windowed(app, &[], options(), mpsc::channel().0);
  let line = |number: usize| Line {
    number,
    text: format!("found = await bash(f\"grep -n step{number} src/hand.rs\")"),
    mark: if number == lines {
      Mark::Runs { work: Work::Command, began: Some(0) }
    } else {
      Mark::Ran
    },
    output: None,
    made: Vec::new(),
    writes: Vec::new(),
    awaits: Vec::new(),
    pictures: Vec::new(),
  };
  let block = Block {
    maker: Maker::default(),
    lines: (1..=lines).map(line).collect(),
    more: true,
    thought: None,
  };
  let exchange = Exchange {
    at: 0,
    message: "rung3".to_owned(),
    said_at: 0,
    by: Maker::default(),
    text: String::new(),
    python: true,
    runs: true,
    block: Some(block),
    said: Vec::new(),
  };
  let key = (0, LIFE.to_owned(), "prompt1".to_owned());
  let thread =
    Thread { key, exchanges: [(0, exchange)].into(), above: Some(0), ..Thread::default() };
  let (name, login) = ("mac".to_owned(), "uael".to_owned());
  let told = Told { name, login, link: Link::Listed, cards: vec![card()], ..Told::default() };
  window.tell(app, |model| {
    model.client.machines.push(told);
    model.client.thread = Some(Opened { thread, ..Opened::default() });
  });
  // The window says that a machine is slow once it waited for it a moment.
  app.advance(Duration::from_secs(1));
  window.drain(app);
  window
}

/// The card of the thread, which says that it ended and that the operator did not read it.
fn card() -> Card {
  Card {
    life: LIFE.to_owned(),
    session: None,
    project: None,
    refused: None,
    held: true,
    viewers: Vec::new(),
    pins: Vec::new(),
    since: 0,
    thread: view::Card {
      prompt: "prompt1".to_owned(),
      title: "the speed".to_owned(),
      standing: Standing::Resolved,
      model: None,
      agents: 0,
      readers: Vec::new(),
    },
  }
}

fn layouts(window: &Window) -> u64 {
  window.screen.model.layouts.get()
}

/// A speed law: each change of what the window shows lays it out once, also the change that marks
/// the open thread read, and a step of its loaders lays out nothing.
#[test]
fn a_change_lays_the_window_out_once_and_a_step_of_its_loaders_not_at_all() {
  let app = TestApp::new(200, 60);
  let mut window = window(&app, 50);
  let before = layouts(&window);
  window.tell(&app, |model| model.client.notice = Some("Copied the selection.".to_owned()));
  assert_eq!(layouts(&window) - before, 1);
  // The thread ends again, unread: the change that shows it marks it read too.
  let before = layouts(&window);
  window.tell(&app, |model| model.client.machines[0].cards[0].thread.readers.clear());
  let read = window.screen.model.client.machines[0].cards[0].thread.readers.clone();
  assert_eq!((layouts(&window) - before, read), (1, vec!["operator/uael".to_owned()]));
  let before = layouts(&window);
  app.advance(Duration::from_millis(400));
  window.drain(&app);
  assert_eq!(layouts(&window) - before, 0);
}

/// What a change costs, with its layout, on a block of more lines:
/// `cargo test -p furb-window --lib -- --ignored --nocapture laid_out`.
#[test]
#[ignore = "a measure, which prints its times"]
fn laid_out() {
  for lines in [114, 500, 2000] {
    let app = TestApp::new(200, 60);
    let mut window = window(&app, lines);
    let runs = 500u32;
    let start = Instant::now();
    for at in 0..runs {
      window.tell(&app, |model| model.client.notice = Some(format!("{at}")));
    }
    println!("{lines} lines: {:?} a change", start.elapsed() / runs);
  }
}

/// A window on a long thread: a count of answers, each a message that wraps and a block of twelve
/// lines.
fn long(app: &TestApp, answers: usize) -> Window {
  let mut window = windowed(app, &[], options(), mpsc::channel().0);
  let line = |number: usize| Line {
    number,
    text: format!("found = await bash(f\"grep -n step{number} src/hand.rs\")"),
    mark: Mark::Ran,
    output: None,
    made: Vec::new(),
    writes: Vec::new(),
    awaits: Vec::new(),
    pictures: Vec::new(),
  };
  let exchange = |at: usize| Exchange {
    at,
    message: format!("rung{at}"),
    said_at: 0,
    by: Maker::default(),
    text: format!(
      "The step {at} reads the feed and says what it found, in a paragraph that wraps over more \
       than one row of the window, as the answer of a model does, with `code` and **marks**."
    ),
    python: false,
    runs: false,
    block: Some(Block {
      maker: Maker::default(),
      lines: (12 * at + 1..=12 * at + 12).map(line).collect(),
      more: false,
      thought: None,
    }),
    said: Vec::new(),
  };
  let key = (0, LIFE.to_owned(), "prompt1".to_owned());
  let exchanges = (0..answers).map(|at| (at, exchange(at))).collect();
  let thread = Thread { key, exchanges, above: Some(0), ..Thread::default() };
  let (name, login) = ("mac".to_owned(), "uael".to_owned());
  let told = Told { name, login, link: Link::Listed, cards: vec![card()], ..Told::default() };
  window.tell(app, |model| {
    model.client.machines.push(told);
    model.client.thread = Some(Opened { thread, ..Opened::default() });
  });
  app.advance(Duration::from_secs(1));
  window.drain(app);
  window
}

/// What a step of the wheel costs on a long thread, with its draw, and what a burst of steps that
/// come in one frame costs: `cargo test -p furb-window --lib -- --ignored --nocapture wheeled`.
#[test]
#[ignore = "a measure, which prints its times"]
fn wheeled() {
  for answers in [50, 400] {
    let app = TestApp::new(200, 60);
    let mut window = long(&app, answers);
    app.render();
    let runs = 60u32;
    let (before, start) = (layouts(&window), Instant::now());
    for _ in 0..runs {
      app.scroll(10, 20, false);
      window.drain(&app);
      app.advance(Duration::ZERO);
      window.drain(&app);
      app.render();
    }
    let step = start.elapsed() / runs;
    let laid = layouts(&window) - before;
    let start = Instant::now();
    for _ in 0..runs {
      for _ in 0..5 {
        app.scroll(10, 20, true);
        window.drain(&app);
      }
      app.advance(Duration::ZERO);
      window.drain(&app);
      app.render();
    }
    let burst = start.elapsed() / runs;
    println!(
      "{answers} answers: a step {step:?} with {laid} layouts for {runs}, a burst of 5 {burst:?}"
    );
  }
}

/// The bar of the time of the open thread: none, open on its moments, or open with the chains of a
/// life that holds many, each of which asks the one before it.
fn timed(window: &mut Window, app: &TestApp, chains: Option<usize>) {
  let moments: Vec<u64> = (0..3000).map(|at| at * 10).collect();
  let messages = (0..100).map(|at| (at * 300, format!("message {at}"))).collect();
  let chains = chains.map(|count| {
    let track = |at: usize| {
      let moments: Vec<u64> = (0..400).map(|one| (one * 70 + at as u64 * 3) % 30_000).collect();
      let threads: Vec<serde_json::Value> = (0..5)
        .map(|one| {
          let by = (at > 0).then(|| format!("chain{}", at - 1));
          serde_json::json!({"prompt": format!("prompt{at}x{one}"), "at": one * 5000 + at as u64,
            "by": by, "ended": one * 5000 + at as u64 + 900, "says": "count the lines"})
        })
        .collect();
      let state = if at.is_multiple_of(2) { "runs" } else { "done" };
      serde_json::json!({"chain": format!("chain{at}"), "label": format!("work {at}"),
        "parent": (at > 0).then_some("chain0"), "state": state, "moments": moments, "threads": threads})
    };
    let tracks = (0..count).map(track).collect::<Vec<_>>();
    let tracks = serde_json::from_value(serde_json::Value::Array(tracks)).expect("tracks read");
    Chains { tracks, stale: false, find: String::new() }
  });
  window.tell(app, |model| {
    let time = Time { moments, messages, chains, ..Time::default() };
    if let Some(opened) = model.client.thread.as_mut() {
      opened.thread.time = Some(time);
    }
  });
}

/// What a change and a move of the pointer over the bar cost, with the bar of the time at now,
/// open, and open with sixty chains:
/// `cargo test -p furb-window --lib -- --ignored --nocapture time_laid_out`.
#[test]
#[ignore = "a measure, which prints its times"]
fn time_laid_out() {
  for (name, chains) in [("closed", None), ("open", Some(0)), ("chains", Some(60))] {
    let app = TestApp::new(200, 60);
    let mut window = window(&app, 114);
    if let Some(count) = chains {
      timed(&mut window, &app, (count > 0).then_some(count));
    }
    let runs = 200u32;
    let start = Instant::now();
    for at in 0..runs {
      window.tell(&app, |model| model.client.notice = Some(format!("{at}")));
    }
    let change = start.elapsed() / runs;
    let start = Instant::now();
    for at in 0..runs {
      let x = u16::try_from(10 + at % 150).unwrap_or_default();
      window.steadily(&app, |model| model.pointer = Some((x, 58)));
    }
    let moved = start.elapsed() / runs;
    println!("{name}: {change:?} a change, {moved:?} a move of the pointer");
  }
}
