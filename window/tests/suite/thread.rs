//! The thread of a card, on a real daemon whose lives a peer of the test drives.

#![cfg(test)]

use furb_host::protocol::{Answer, Verb};
use furb_window::{open, tokens::Loader};
use opentui::{native::Capture, style::Color, testing::TestApp};

use crate::{
  cards::{AWAY, app, options, row, shows},
  daemon::{ANSWERS, Daemon, FAILS, LONG, PATIENCE, Peer, THINKS},
  screens::screen,
};

/// The columns of the thread, at the left of the list.
const LEFT: usize = 61;

/// A window on a machine with one session whose thread says a message, opened by a click on its
/// card, once the thread shows a text.
fn opened(message: &str, shown: &str) -> (TestApp, Daemon, Peer) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("time.jsonl"), true);
  peer.said(&life, message);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  let title = message.lines().next().unwrap();
  app.click(80, row(&app, &title[..20]));
  shows(&app, shown);
  (app, daemon, peer)
}

/// The cells of the thread that differ between two frames: their column, their row and their mark.
fn moved(before: &Capture, after: &Capture) -> Vec<(usize, usize, String)> {
  let width = before.width as usize;
  let cells = before.cells.iter().zip(&after.cells).enumerate();
  let differ = cells.filter(|(at, (one, other))| at % width < LEFT && one != other);
  differ.map(|(at, (_, other))| (at % width, at / width, other.symbol.clone())).collect()
}

/// The pointer on a cell of a thing that takes a click: the text of that thing goes bright white,
/// with no other change of any cell, and comes back when the pointer leaves.
pub(crate) fn lights(app: &TestApp, x: u16, y: u16) {
  app.hover(0, 0);
  let before = app.render();
  app.hover(x, y);
  let lit = app.render();
  let width = usize::try_from(lit.width).unwrap();
  let at = usize::from(y) * width + usize::from(x);
  assert_eq!(lit.cells[at].fg, Color::hex(0xff_ffff), "the cell {x},{y} is not lit");
  for (one, other) in before.cells.iter().zip(&lit.cells) {
    if one != other {
      assert_eq!((&other.symbol, other.bg, other.fg), (&one.symbol, one.bg, Color::hex(0xff_ffff)));
    }
  }
  app.hover(0, 0);
  assert_eq!(app.render().cells, before.cells, "the pointer left the cell {x},{y}");
}

/// The first cell of a text in the window, at a column or after it: its column and its row.
pub(crate) fn at(app: &TestApp, text: &str, from: usize) -> (u16, u16) {
  let frame = app.text();
  let found = frame.lines().enumerate().find_map(|(y, line)| {
    let after: String = line.chars().skip(from).collect();
    after.find(text).map(|at| (from + after[..at].chars().count(), y))
  });
  let (x, y) = found.unwrap_or_else(|| panic!("the window shows no {text:?}:\n{frame}"));
  (u16::try_from(x).unwrap(), u16::try_from(y).unwrap())
}

/// The feed moved up a page at a time until its frame is as a test wants, as far as it goes.
fn up_until(app: &TestApp, wanted: impl Fn(&str) -> bool) -> bool {
  (0..40).any(|_| {
    app.press("pageup");
    wanted(&app.text())
  })
}

#[test]
fn a_thread_that_works_rolls_its_block_in_seven_rows_and_only_its_loaders_move() {
  let message = format!("where does the time of a resume go\n{LONG}");
  let (app, _daemon, _peer) = opened(&message, "await wait(600, chain)");
  let frame = app.text();
  // Nine lines and more to come: the fold of the four first, five lines, the line that runs, and
  // the row of what comes.
  let block: Vec<&str> =
    frame.lines().skip_while(|line| !line.contains("…  4 lines")).take(7).collect();
  let first = |line: &&str| {
    line.chars().skip(3).collect::<String>().split_whitespace().next().map(str::to_owned)
  };
  let numbers: Vec<String> = block.iter().filter_map(first).collect();
  assert_eq!(numbers[..5], ["…", "5", "6", "7", "8"], "{frame}");
  assert!(block[5].contains("await wait(600, chain)") && block[6].trim() == "…", "{frame}");
  screen("thread-works", &app);

  // A step of the loaders draws their cells alone: the line that runs, and the state of the hints.
  let before = app.render();
  app.advance(Loader::Command.step());
  let cells = moved(&before, &app.render());
  let frames: Vec<&str> =
    [Loader::Model, Loader::Command].iter().flat_map(|one| one.frames()).copied().collect();
  let places: Vec<(usize, usize)> = cells.iter().map(|one| (one.0, one.1)).collect();
  assert_eq!(places.len(), 2, "{cells:?}");
  assert!(cells.iter().all(|one| frames.contains(&one.2.as_str())), "{cells:?}");

  // The fold of the block takes a click, so it lights under the pointer, and it opens the block.
  let (x, y) = at(&app, "4 lines", 0);
  lights(&app, x, y);
  app.click(x, y);
  shows(&app, "▴  9 lines");
  assert!(app.text().contains("1  birth = "), "{}", app.text());
}

#[test]
fn a_model_that_thinks_shows_its_thought_on_one_row_and_the_line_that_it_writes() {
  let message = format!("why is the seed slow on a big record\n{THINKS}");
  let (app, _daemon, _peer) = opened(&message, "proof = await bash(");
  let frame = app.text();
  assert!(frame.contains("1  cost = tokens(word)"), "{frame}");
  // The model wrote after it thought: the thought ended, and shows its first sentence.
  let thought = frame.lines().filter(|line| line.contains("I read the stages")).count();
  assert_eq!(thought, 1, "{frame}");
  screen("thread-writes", &app);

  // The thought takes a click: it lights under the pointer, and opens whole.
  let (x, y) = at(&app, "I read the stages", 0);
  lights(&app, x, y);
  app.click(x, y);
  shows(&app, "Then the seed");
}

#[test]
fn a_thread_at_rest_shows_its_answer_as_prose_and_a_hover_lights_only_the_text() {
  let message = format!("how long does an open take\n{ANSWERS}");
  let (app, _daemon, _peer) = opened(&message, "Two stages hold most");
  // He reads the end of the thread, so its card rests with the resolved ones, which he opens.
  app.advance(AWAY);
  app.click(80, row(&app, "Resolved 1"));
  shows(&app, "Resolved 1 ▾");
  screen("thread-rests", &app);
  // The card, the button that resolves, and the link of the answer.
  for (text, from) in [("how long does an open", LEFT), ("[ Resolve ]", 0), ("the trace", 0)] {
    let (x, y) = at(&app, text, from);
    lights(&app, x + 2, y);
  }
}

#[test]
fn a_line_that_failed_shows_its_cross_and_its_error_under_it() {
  let message = format!("why is the window stuck after a switch of model\n{FAILS}");
  let (app, _daemon, _peer) = opened(&message, "ValueError");
  // He reads the end of the thread, so its card rests with the resolved ones.
  app.advance(AWAY);
  let frame = shows(&app, "Resolved 1");
  assert!(frame.contains("✗  raise ValueError("), "{frame}");
  screen("thread-failed", &app);
}

#[test]
fn an_older_page_keeps_the_rows_that_the_operator_reads() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("long.jsonl"), true);
  let prompt = peer.said(&life, "ground the branch, message by message");
  for n in 1..24 {
    let message = format!("and the step {n}");
    let on = Some(prompt.clone());
    let verb = Verb::Prompt { life: life.clone(), shape: "None".to_owned(), message, to: None, on };
    assert!(matches!(peer.ask(verb), Answer::Act(_)));
  }
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "ground the branch, m"));
  // The model answers the messages in their order: once the last has its answer, each has, so no
  // answer lands while the operator scrolls.
  shows(&app, "24  close(None)");
  // He reads the end of the thread, so its card goes with the resolved threads once the hand is
  // away from it.
  app.advance(AWAY);
  shows(&app, "Resolved 1");
  // The page of the end holds the twenty last exchanges. Near its top the window asks the four
  // before them, which come above the rows that he reads, and the feed goes on up to the first.
  let first = "┃ ground the branch, message by message";
  assert!(up_until(&app, |text| text.contains("earlier messages") || text.contains(first)));
  let came = app.wait_until(PATIENCE, || !app.text().contains("earlier messages"));
  assert!(came, "the older page never came:\n{}", app.text());
  assert!(up_until(&app, |text| text.contains(first)), "{}", app.text());
  screen("thread-older", &app);
}
