//! The work of the window, which no frame shows: a test fills the model of a window with no machine
//! and counts its layouts, which only the module that holds the window reaches.

use std::{
  path::PathBuf,
  time::{Duration, Instant},
};

use furb::view::{self, Block, Exchange, Line, Maker, Mark, Standing, Work};
use furb_client::{Link, Opened, Thread, Told};
use furb_host::protocol::Card;
use jiff::tz::TimeZone;
use opentui::testing::TestApp;

use super::{Shared, tell, windowed};
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
    program: None,
    reloaded: false,
    tailnet: None,
    listen: None,
  }
}

/// A window on its thread, whose answer holds a block of a count of lines, the last of which runs
/// a command, and whose card says that it ended and that the operator did not read it.
fn window(app: &TestApp, lines: usize) -> Shared {
  let window = windowed(app, &[], options());
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
  tell(app, &window, |model| {
    model.client.machines.push(told);
    model.client.thread = Some(Opened { thread, ..Opened::default() });
  });
  // The window says that a machine is slow once it waited for it a moment.
  app.advance(Duration::from_secs(1));
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

fn layouts(window: &Shared) -> u64 {
  window.borrow().screen.model.layouts.get()
}

/// A speed law: each change of what the window shows lays it out once, also the change that marks
/// the open thread read, and a step of its loaders lays out nothing.
#[test]
fn a_change_lays_the_window_out_once_and_a_step_of_its_loaders_not_at_all() {
  let app = TestApp::new(200, 60);
  let window = window(&app, 50);
  let before = layouts(&window);
  tell(&app, &window, |model| model.client.notice = Some("Copied the selection.".to_owned()));
  assert_eq!(layouts(&window) - before, 1);
  // The thread ends again, unread: the change that shows it marks it read too.
  let before = layouts(&window);
  tell(&app, &window, |model| model.client.machines[0].cards[0].thread.readers.clear());
  let read = window.borrow().screen.model.client.machines[0].cards[0].thread.readers.clone();
  assert_eq!((layouts(&window) - before, read), (1, vec!["operator/uael".to_owned()]));
  let before = layouts(&window);
  app.advance(Duration::from_millis(400));
  assert_eq!(layouts(&window) - before, 0);
}

/// What a change costs, with its layout, on a block of more lines:
/// `cargo test -p furb-window --lib -- --ignored --nocapture laid_out`.
#[test]
#[ignore = "a measure, which prints its times"]
fn laid_out() {
  for lines in [114, 500, 2000] {
    let app = TestApp::new(200, 60);
    let window = window(&app, lines);
    let runs = 500u32;
    let start = Instant::now();
    for at in 0..runs {
      tell(&app, &window, |model| model.client.notice = Some(format!("{at}")));
    }
    println!("{lines} lines: {:?} a change", start.elapsed() / runs);
  }
}
