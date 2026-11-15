//! The thread of a card, on a real daemon whose lives a peer of the test drives.

#![cfg(test)]

use furb_host::protocol::{Answer, Verb};
use furb_window::{
  open,
  tokens::{AWAY, Loader},
};
use opentui::{native::Capture, style::Color, testing::TestApp};

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, DELEGATES, Daemon, FAILS, LONG, PATIENCE, Peer, READS, RETRIES, THINKS},
  screens::screen,
};

/// The columns of the thread, at the left of the list.
pub(crate) const LEFT: usize = 61;

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
  app.click(80, row(&app, &title[..15]));
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

/// The pointer on a cell of a thing that takes a click, a count of cells into a text that stands at
/// a column or after it: the text of that thing goes bright white, with no other change of any
/// cell, and comes back when the pointer leaves. The daemon serves the window on the runtime of
/// the app, so a frame of it can come between the frames of the hover and move the text: such a
/// hover is read again.
pub(crate) fn lights(app: &TestApp, text: &str, from: usize, inside: u16) {
  let mut hovers = 0;
  let lit = app.wait_until(PATIENCE, || {
    hovers += 1;
    app.hover(0, 0);
    let before = app.render();
    let (x, y) = place(&before.text(), text, from);
    let x = x + inside;
    app.hover(x, y);
    let lit = app.render();
    app.hover(0, 0);
    if app.render().cells != before.cells {
      return false;
    }
    let width = usize::try_from(lit.width).unwrap();
    let at = usize::from(y) * width + usize::from(x);
    assert_eq!(lit.cells[at].fg, Color::hex(0xff_ffff), "the cell {x},{y} is not lit");
    for (one, other) in before.cells.iter().zip(&lit.cells) {
      if one != other {
        let white = Color::hex(0xff_ffff);
        assert_eq!((&other.symbol, other.bg, other.fg), (&one.symbol, one.bg, white));
      }
    }
    true
  });
  assert!(lit, "no frame of the daemon left {text:?} still in {hovers} hovers");
}

/// A click on a cell of a thing, a count of cells into a text that stands at a column or after it,
/// once the cell lights under the pointer. The click hits the frame where it found the text.
pub(crate) fn clicked(app: &TestApp, text: &str, from: usize, inside: u16) {
  lights(app, text, from, inside);
  let (x, y) = at(app, text, from);
  app.click(x + inside, y);
}

/// The first cell of a text in the window, at a column or after it: its column and its row, in
/// the frame that the next click hits.
pub(crate) fn at(app: &TestApp, text: &str, from: usize) -> (u16, u16) {
  place(&app.text(), text, from)
}

/// The first cell of a text in a frame, at a column or after it: its column and its row.
fn place(frame: &str, text: &str, from: usize) -> (u16, u16) {
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

  // A step of the loaders draws their cells alone: the line that runs, the state of the hints, and
  // the letters of its word of work that the light passes.
  let before = app.render();
  app.advance(Loader::Command.step());
  let cells = moved(&before, &app.render());
  let frames: Vec<&str> =
    [Loader::Model, Loader::Command].iter().flat_map(|one| one.frames()).copied().collect();
  let (marks, letters): (Vec<_>, Vec<_>) =
    cells.iter().partition(|one| frames.contains(&one.2.as_str()));
  assert_eq!(marks.len(), 2, "{cells:?}");
  let last = cells.iter().map(|one| one.1).max();
  let lit = |one: &&(usize, usize, String)| Some(one.1) == last && "Working".contains(&one.2);
  assert!(letters.iter().all(lit), "{cells:?}");

  // The fold of the block takes a click, so it lights under the pointer, and it opens the block.
  clicked(&app, "4 lines", 0, 0);
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
  clicked(&app, "I read the stages", 0, 0);
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
  // The card and the link of the answer.
  for (text, from) in [("how long does an open", LEFT), ("the trace", 0)] {
    lights(&app, text, from, 2);
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
fn a_model_asked_again_says_its_attempt_its_wait_and_why_then_its_failure() {
  let message = format!("the bench stalls on a cold build\n{RETRIES}");
  let (app, _daemon, _peer) = opened(&message, "Attempt 2 of 10");
  let frame = app.text();
  // The attempt and its wait on the line that the model writes, and why under it.
  let lines = ["⠋  Attempt 2 of 10 in 4 s", "   the provider is overloaded"];
  assert!(lines.iter().all(|line| frame.contains(line)), "{frame}");
  screen("thread-retries", &app);
  // The second attempt fails for a cause that does not pass, so no attempt is left, and the
  // model asked again fails too: the provider pauses the thread, which shows its failure once.
  let failed = "✗  claude-cli:opus/high answered nothing";
  let frame = app.wait_until(PATIENCE, || {
    let frame = app.text();
    frame.contains(failed) && frame.contains("‖ Paused")
  });
  assert!(frame, "{}", app.text());
  let frame = app.text();
  assert_eq!(frame.matches("answered nothing").count(), 1, "{frame}");
  assert!(!frame.contains("Attempt"), "{frame}");
  // Once the hand left the list, the card stands with what needs him, as a pause of furb does.
  app.advance(AWAY);
  shows(&app, "Needs you 1");
  screen("thread-retry-failed", &app);
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
  app.click(80, row(&app, "ground the branch,"));
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

#[test]
fn the_dock_says_how_each_agent_stands_and_a_click_opens_its_thread_until_an_escape() {
  let message = format!("measure the open on three chains\n{DELEGATES}");
  let (app, _daemon, _peer) = opened(&message, "README.md");
  shows(&app, "✓ Resolved");
  // The agents from the first that works: each with its state and its model, the line where it
  // stands while it lives, and the one that ended among them with its mark. The card counts the
  // agents that work.
  let frame = app.text();
  let dock: Vec<String> = frame
    .lines()
    .skip_while(|line| !line.contains("└ Run the bench"))
    .take(4)
    .map(|line| line.chars().take(LEFT).collect::<String>().trim_end().to_owned())
    .collect();
  assert_eq!(
    dock,
    [
      "  └ Run the bench of the replay             ◐ Working  opus",
      "    await wait(600, chain)",
      "  └ Read the boot of the host               ⠋ Working  opus",
      "  └ Count the lines of README.md           ✓ Resolved  opus",
    ]
    .map(str::to_owned),
    "{frame}"
  );
  shows(&app, "opus └2");
  screen("thread-agents", &app);

  // A click on an agent opens its thread in place: its head says that it is one.
  clicked(&app, "└ Run the bench", 0, 2);
  shows(&app, " Back ");
  // The head names the agent once its card or its page came, which can follow the button.
  let head = || app.text().lines().next().unwrap().to_owned();
  let named = app
    .wait_until(PATIENCE, || head().starts_with("● furb · agent · Run the bench of the replay "));
  assert!(named, "{}", head());
  shows(&app, "await wait(600, chain)");
  screen("thread-agent", &app);

  // An escape opens the thread that it came from again, with its dock, whose key the row of hints
  // names.
  app.press("escape");
  shows(&app, " Resolve ");
  shows(&app, "└ Count the lines of README.md");
  shows(&app, "⌃X ↓ agents");

  // The key steps through the agents of the dock, from the first, and from the last to the first.
  let into = |title: &str| {
    app.press("ctrl+x");
    app.press("down");
    shows(&app, &format!(" furb · agent · {title} "));
  };
  for title in [
    "Run the bench of the replay",
    "Read the boot of the host",
    "Count the lines of README.md",
    "Run the bench of the replay",
  ] {
    into(title);
  }
  app.press("escape");
  shows(&app, " Resolve ");
}

/// The rows of the box of the context, from its head to the foot of the window, each cut to the
/// columns of the box and trimmed.
fn boxed(app: &TestApp) -> Vec<String> {
  let frame = app.text();
  let (x, y) = place(&frame, "Context", LEFT);
  let rows = frame.lines().skip(usize::from(y));
  rows
    .map(|line| line.chars().skip(usize::from(x)).collect::<String>().trim_end().to_owned())
    .collect()
}

#[test]
fn the_box_of_the_context_says_how_full_the_window_of_the_chain_is_and_a_click_shows_each_part() {
  let message = format!("read the trace of the open\n{READS}");
  let (app, _daemon, _peer) = opened(&message, "each chain.");
  // The last answer read 124k of the window of 1M of its model: its share, a bar in the colors
  // of its parts, and its counts in the row of the hints.
  let got = app.wait_until(PATIENCE, || boxed(&app)[0].contains('%'));
  assert!(got, "{}", app.text());
  let bar = "━".repeat(35);
  let counts = "124k of 1M · 98k cached       $0.75";
  assert_eq!(boxed(&app), ["Context ▸                       12%", &bar, counts]);
  // The pointer lights the text of the box and leaves the colors of its bar; a click opens one
  // row for each kind of content that the answer read, with its size and its share.
  clicked(&app, "Context", LEFT, 0);
  shows(&app, "Engine");
  // The script reads 124k at its one answer: so the engine holds what its few turns do not.
  let kinds = [
    "Context ▾                       12%",
    &bar,
    "■ Engine                 122k   12%",
    "■ Doctrine               1.1k   <1%",
    "■ Turns                   226   <1%",
    "■ Free                   876k   88%",
    counts,
  ];
  assert_eq!(boxed(&app), kinds);
  screen("thread-context", &app);
  // A click on the bar, whose cells keep their colors under the pointer, shows what came from the
  // cache, and a click on the head folds the box.
  let (x, y) = at(&app, &bar, LEFT);
  app.click(x, y);
  shows(&app, "From the cache");
  let parts = [
    "Context ▾                       12%",
    &bar,
    "■ From the cache          98k   10%",
    "■ Written to the cache   8.4k    1%",
    "■ New                     17k    2%",
    "■ Free                   876k   88%",
    counts,
  ];
  assert_eq!(boxed(&app), parts);
  clicked(&app, "Context", LEFT, 0);
  let folded = app.wait_until(PATIENCE, || !app.text().contains("From the cache"));
  assert!(folded, "{}", app.text());
  // While the list hides, and so its box, the ring of the launcher says the share, and a click on
  // it shows the list with each part of its box.
  assert!(!app.text().contains("○ 12%"), "{}", app.text());
  app.resize(80, 30);
  shows(&app, "○ 12%");
  screen("thread-context-ring", &app);
  clicked(&app, "○ 12%", 0, 0);
  shows(&app, "Engine");
}
