//! The thread of a card, on a real daemon whose lives a peer of the test drives.

#![cfg(test)]

use std::fs;

use furb_host::protocol::{Answer, Draft, Frame, Role, Verb};
use furb_window::{
  Machine, open,
  tokens::{AWAY, Loader},
};
use opentui::{native::Capture, style::Color, testing::TestApp};

use crate::{
  cards::{app, options, row, shows},
  daemon::{
    ANSWERS, DELEGATES, Daemon, EDITS, FAILS, FANS, LONG, OPENS, PATIENCE, Peer, READS, RETRIES,
    THINKS, WORKS,
  },
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
fn a_model_asked_again_says_its_attempt_its_wait_and_why_then_its_failure_and_the_way_out() {
  let message = format!("the bench stalls on a cold build\n{RETRIES}");
  let (app, _daemon, mut peer) = opened(&message, "Attempt 2 of 10");
  let frame = app.text();
  // The attempt and its wait on the line that the model writes, and why under it.
  let lines = ["⠋  Attempt 2 of 10 in 4 s", "   the provider is overloaded"];
  assert!(lines.iter().all(|line| frame.contains(line)), "{frame}");
  screen("thread-retries", &app);
  // The second attempt fails for a cause that does not pass, so no attempt is left, and the
  // model asked again fails too: the provider pauses the thread, which shows its failure once.
  let failed = "✗  opus/high answered nothing";
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
  // The thread and the hint row say that the pause holds, that nothing is lost and what ends it.
  let frame = app.text();
  let way = ["‖ This thread is paused. Nothing is lost.", "Resume", "Change the model"];
  assert!(way.iter().all(|line| frame.contains(line)), "{frame}");
  assert!(frame.contains("‖ Paused · ⌃X Z or a message resumes it"), "{frame}");
  // The key wakes the thread, which asks the model again: the daemon says the wake of the
  // operator, then a reply of the model, which fails again here, so furb pauses the chain again.
  app.press("ctrl+x");
  app.press("z");
  let fact = |name: &'static str, operator: bool| {
    move |frame: &Frame| {
      let data = &serde_json::json!(frame)["event"]["data"];
      data[0] == name && (!operator || data[2] != "provider")
    }
  };
  for (name, operator) in [("wake", true), ("reply", false), ("pause", false)] {
    peer.heard(&app, fact(name, operator));
  }
  // A message wakes it too, and asks a line of its own, which fails again.
  app.paste("try once more");
  app.press("enter");
  let again = app.wait_until(PATIENCE, || {
    let frame = app.text();
    frame.matches(failed).count() == 2 && frame.contains("‖ Paused")
  });
  assert!(again, "{}", app.text());
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
  // The first message came in view at a row that the arrival of the older page moved, so one more
  // page stands the feed at its head.
  app.press("pageup");
  screen("thread-older", &app);
}

#[test]
fn the_pointer_on_a_line_that_ran_shows_what_it_gave_beside_its_code() {
  let message = format!("open the readme\n{OPENS}");
  let (app, _daemon, _peer) = opened(&message, "It holds 3 lines.");
  let line = |app: &TestApp| {
    let frame = app.text();
    let found = frame.lines().find(|one| one.contains("2  found = read("));
    found.map(|one| one.chars().take(LEFT).collect::<String>().trim_end().to_owned())
  };
  assert_eq!(line(&app).as_deref(), Some("    2  found = read('notes.md')"));
  // As a debugger shows a value, the line says what its read gave while the pointer is on it.
  app.hover(20, row(&app, "2  found = read("));
  let shown = app.wait_until(PATIENCE, || line(&app).is_some_and(|one| one.contains("→ Text(")));
  assert!(shown, "{}", app.text());
  app.hover(0, 0);
  let gone = app.wait_until(PATIENCE, || line(&app).is_some_and(|one| !one.contains("→")));
  assert!(gone, "{}", app.text());
}

#[test]
fn a_line_that_wrote_a_file_shows_its_path_what_it_changed_and_the_diff_under_it() {
  let message = format!("edit the notes of the day\n{EDITS}");
  let (app, _daemon, _peer) = opened(&message, "notes.md has 4 lines.");
  let rows = |app: &TestApp, count: usize| -> Vec<String> {
    let frame = app.text();
    let block = frame.lines().skip_while(|line| !line.contains("1  write(Text("));
    block.take(count).map(|line| line.chars().take(LEFT).collect::<String>()).collect()
  };
  let trimmed = |rows: Vec<String>| rows.iter().map(|one| one.trim_end().to_owned()).collect();
  // Each file that a line wrote stands under it with what changed: the life read no notes.md
  // before the first write, so that write says how many lines it wrote.
  let rolled: Vec<String> = trimmed(rows(&app, 6));
  assert_eq!(
    rolled,
    [
      "    1  write(Text('notes.md', 'a\\nb\\nc\\n'))",
      "       └ notes.md  wrote 3 lines",
      "    2  m = read('notes.md')",
      "    3  write(m.replace('b\\n', 'B\\nb2\\n'))",
      "       └ notes.md  +2 −1",
      "    4  close('notes.md has 4 lines.')",
    ]
  );
  // A click on a file opens the block, which shows the diff of each file under its line.
  clicked(&app, "notes.md  +2", 0, 0);
  let open = app.wait_until(PATIENCE, || app.text().contains("+B"));
  assert!(open, "{}", app.text());
  // The rows move as the block opens, so the pointer leaves them, where it would light a row.
  app.hover(0, 0);
  let opened: Vec<String> = trimmed(rows(&app, 15));
  assert_eq!(
    opened[..13],
    [
      "    1  write(Text('notes.md', 'a\\nb\\nc\\n'))",
      "       └ notes.md  wrote 3 lines",
      "         +a",
      "         +b",
      "         +c",
      "    2  m = read('notes.md')",
      "    3  write(m.replace('b\\n', 'B\\nb2\\n'))",
      "       └ notes.md  +2 −1",
      "          a",
      "         -b",
      "         +B",
      "         +b2",
      "          c",
    ]
  );
  screen("thread-files", &app);
}

#[test]
fn a_revert_writes_back_what_the_last_answer_wrote_as_a_word_of_his_and_a_second_restores_it() {
  let message = format!("edit the notes of the day\n{EDITS}");
  let (app, daemon, _peer) = opened(&message, "notes.md has 4 lines.");
  let notes = daemon.file("furb").join("notes.md");
  assert_eq!(fs::read_to_string(&notes).unwrap(), "a\nB\nb2\nc\n");
  clicked(&app, "↶ Revert notes.md", 0, 0);
  let reverted = app.wait_until(PATIENCE, || fs::read_to_string(&notes).unwrap() == "a\nb\nc\n");
  assert!(reverted, "{}", app.text());
  // The revert is a word of his on the thread, which says what it wrote back, and which a second
  // revert reverts.
  let shown = app.wait_until(PATIENCE, || {
    let frame = app.text();
    frame.contains("notes.md  +1 −2") && frame.matches("↶ Revert notes.md").count() == 1
  });
  assert!(shown, "{}", app.text());
  println!("{}", app.text());
  screen("thread-reverted", &app);
  clicked(&app, "↶ Revert notes.md", 0, 0);
  let restored =
    app.wait_until(PATIENCE, || fs::read_to_string(&notes).unwrap() == "a\nB\nb2\nc\n");
  assert!(restored, "{}", app.text());
}

#[test]
fn each_agent_stands_under_the_line_that_made_it_and_a_click_opens_its_thread() {
  let message = format!("measure the open on three chains\n{DELEGATES}");
  let (app, _daemon, _peer) = opened(&message, "README.md");
  shows(&app, "✓ Resolved");
  // Each agent stands in the box under the line of the block that made it, at the column of its
  // code, with the line where it stands while it lives, the value that it gave once it rests, and
  // else its title, then its state and its model. The card counts the agents that work.
  let frame = app.text();
  let agents: Vec<String> = frame
    .lines()
    .skip_while(|line| !line.contains("1  bench = prompt("))
    .take(7)
    .map(|line| line.chars().take(LEFT).collect::<String>().trim_end().to_owned())
    .collect();
  assert_eq!(
    agents,
    [
      "    1  bench = prompt(str, 'Run the bench of the repl…+42",
      "       └ await wait(600, chain)           ◐ Working  opus",
      "    2  boot = prompt(str, 'Read the boot of the host\\…+44",
      "       └ Read the boot of the host        ⠋ Working  opus",
      "    3  lines = await prompt(str, 'Count the lines of…+52",
      "       └ 'README.md has 3 lines.'        ✓ Resolved  opus",
      "    ◐  await wait(600, bench)",
    ]
    .map(str::to_owned),
    "{frame}"
  );
  shows(&app, "opus ↳2");
  screen("thread-agents", &app);

  // A click on an agent opens its thread in place: its head says that it is one.
  clicked(&app, "└ await wait(600, chain)", 0, 2);
  shows(&app, " Back ");
  // The head names the agent once its card or its page came, which can follow the button.
  let head = || app.text().lines().next().unwrap().to_owned();
  let named = app
    .wait_until(PATIENCE, || head().starts_with("● furb · agent · Run the bench of the replay "));
  assert!(named, "{}", head());
  shows(&app, "await wait(600, chain)");
  screen("thread-agent", &app);

  // An escape opens the thread that it came from again, with its agents, whose key the row of
  // hints names.
  app.press("escape");
  shows(&app, " Resolve ");
  shows(&app, "└ 'README.md has 3 lines.'");
  shows(&app, "⌃X ↓ agents");

  // The key steps through the agents of the thread, from the first, and from the last to the first.
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
  // The script reads 124k at its one answer: so the engine holds what its few turns do not. The
  // turns name the folders of the test, whose length changes from run to run, so only the name of
  // each kind and the free room are the same at each run.
  let named: Vec<String> = boxed(&app)[2..6]
    .iter()
    .map(|row| row.split("  ").next().unwrap_or_default().to_owned())
    .collect();
  assert_eq!(named, ["■ Engine", "■ Doctrine", "■ Turns", "■ Free"]);
  assert_eq!(boxed(&app)[5], "■ Free                   876k   88%");
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
  screen("thread-context", &app);
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

#[test]
fn another_operator_on_the_session_shows_in_the_head_of_the_thread_and_where_he_writes() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served_as("rex", Role::Write);
  let mut rex = daemon.peer();
  let life = rex.opened(&sessions.join("time.jsonl"), true);
  let prompt = rex.said(&life, "where does the time of a resume go");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "where does the t"));
  shows(&app, "◉ rex");
  let draft = Some(Draft { on: Some(prompt.clone()) });
  assert_eq!(rex.ask(Verb::Writes { life, draft }), Answer::Done);
  shows(&app, "◉ rex writes…");
  // His window tells where he writes, so the window of rex shows it too.
  app.type_text("and the seed");
  let writes = |frame: &Frame| match frame {
    Frame::Presence { operators, .. } => operators.iter().any(|one| {
      one.operator != "rex" && one.writes.as_ref().and_then(|it| it.on.as_ref()) == Some(&prompt)
    }),
    _ => false,
  };
  rex.heard(&app, writes);
}

#[test]
fn the_chains_under_the_bar_of_the_time_stand_as_a_tree_under_the_chain_that_made_each() {
  let message = format!("measure the open on three chains\n{DELEGATES}");
  let (app, _daemon, _peer) = opened(&message, "README.md");
  app.press("ctrl+x");
  app.press("t");
  let labels = |frame: &str| -> Vec<String> {
    let rows = frame.lines().filter(|line| line.contains('▪'));
    let marks = ['─', '━', '│', '◆', '‹'];
    let left = rows.map(|line| line.chars().take(LEFT).collect::<String>());
    left.map(|line| line.rsplit(marks).next().unwrap_or_default().trim_end().to_owned()).collect()
  };
  let told = app.wait_until(PATIENCE, || labels(&app.text()).len() > 1);
  let frame = app.text();
  assert!(told, "the chains never showed:\n{frame}");
  let labels = labels(&frame);
  // The chain that rests shows no more, and each agent that works stands under the root.
  assert_eq!(labels, [" root", "   bench", "   boot"].map(str::to_owned), "{frame}");
}

#[test]
fn a_login_that_only_reads_sees_the_session_live_and_sends_nothing() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served_as("rex", Role::Read);
  let mut rex = daemon.peer();
  let life = rex.opened(&sessions.join("time.jsonl"), true);
  let prompt = rex.said(&life, "where does the time of a resume go");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "where does the t"));
  let frame = shows(&app, "Read only · you watch this session");
  assert!(!frame.contains("Reply to this thread"), "a field to write in shows:\n{frame}");
  screen("thread-reads-only", &app);
  // What rex says later on the thread shows live, and its model works on it.
  let message = format!("and the seed of a large record {WORKS}");
  let more = Verb::Prompt {
    life: life.clone(),
    shape: "None".to_owned(),
    message,
    to: None,
    on: Some(prompt),
  };
  assert!(matches!(rex.ask(more), Answer::Done | Answer::Act(_)));
  shows(&app, "and the seed of a large record");
  // A key goes to no field.
  app.type_text("try the image");
  app.press("enter");
  let frame = shows(&app, "Working");
  assert!(!frame.contains("try the image"), "{frame}");
  // Rex leaves, so this window is the last view of the session that works.
  assert_eq!(rex.ask(Verb::Detach { life: life.clone() }), Answer::Done);
  // A leave asks nothing of a session that the login cannot keep, pause or stop.
  app.press("ctrl+d");
  app.press("ctrl+d");
  assert!(app.wait_until(PATIENCE, || app.app().is_quitting()), "{}", app.text());
}

#[test]
fn a_window_that_asks_to_read_only_reads_a_session_its_login_may_write() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served_as("rex", Role::Write);
  let mut rex = daemon.peer();
  let life = rex.opened(&sessions.join("time.jsonl"), true);
  rex.said(&life, "where does the time of a resume go");
  let mac = Machine { reads: true, ..daemon.machine("mac") };
  open(&app, vec![mac], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "where does the t"));
  let frame = shows(&app, "Read only · you watch this session");
  assert!(
    !frame.contains("+ ⌃N") && !frame.contains(" Resolve "),
    "a control that writes:\n{frame}"
  );
}

#[test]
fn a_prompt_of_one_chain_to_another_flows_between_their_tracks_and_a_click_opens_it() {
  let message = format!("measure the open on three chains\n{DELEGATES}");
  let (app, _daemon, _peer) = opened(&message, "README.md");
  app.press("ctrl+x");
  app.press("t");
  // The root asks bench: a tee on its track and an arrow down to the track of bench.
  let cell = |frame: &str, label: &str, sign: char| {
    let left = |line: &str| line.chars().take(LEFT).collect::<String>();
    let line = frame.lines().map(left).position(|line| line.trim_end().ends_with(label))?;
    let column = frame.lines().nth(line)?.chars().take(LEFT).position(|one| one == sign)?;
    Some((column, line))
  };
  let flowed = app.wait_until(PATIENCE, || {
    let frame = app.text();
    cell(&frame, " root", '┬').is_some() && cell(&frame, "bench", '▼').is_some()
  });
  let frame = app.text();
  assert!(flowed, "no flow joins the root and bench:\n{frame}");
  let (x, y) = cell(&frame, "bench", '▼').unwrap();
  let (x, y) = (u16::try_from(x).unwrap(), u16::try_from(y).unwrap());
  app.hover(x, y);
  shows(&app, "root asks bench: Run the bench of the replay");
  app.click(x, y);
  // The prompt opens as an agent opens, at its own head.
  shows(&app, "┃ Run the bench of the replay");
}

#[test]
fn a_message_queued_while_the_chain_works_shows_as_a_chip_and_a_cross_takes_it_back() {
  let message = format!("why is the seed slow on a big record\n{THINKS}");
  let (app, _daemon, _peer) = opened(&message, "proof = await bash(");
  app.type_text("then rebase on main");
  app.press("alt+enter");
  // The life holds it until the chain rests, and each window of the life shows it where it will
  // stand, as a message of his with a chip.
  let frame = shows(&app, " queued ");
  assert!(frame.contains("┃ then rebase on main"), "{frame}");
  assert!(!frame.contains("▌then rebase on main"), "the input is empty again:\n{frame}");
  let row = row(&app, " queued ");
  let line = app.text().lines().nth(usize::from(row)).unwrap_or_default().to_owned();
  let cross = u16::try_from(line.chars().position(|one| one == '✕').unwrap()).unwrap();
  app.click(cross, row);
  let back = app.wait_until(PATIENCE, || !app.text().contains(" queued "));
  assert!(back, "the follow-up stays:\n{}", app.text());
  shows(&app, "then rebase on main");
}

#[test]
fn a_click_on_the_mark_of_a_cut_line_shows_it_whole_and_a_click_on_it_cuts_it_again() {
  let message = format!("where does the time go\n{ANSWERS}");
  let (app, _daemon, _peer) = opened(&message, "See the trace");
  let whole = "furb/issues/988).')";
  assert!(!app.text().contains(whole), "{}", app.text());
  let (x, y) = place(&app.text(), "…+", 0);
  app.click(x, y);
  let frame = shows(&app, whole);
  assert!(!frame.contains("…+"), "{frame}");
  app.click(x, y);
  assert!(app.wait_until(PATIENCE, || !app.text().contains(whole)), "{}", app.text());
  shows(&app, "…+");
}

#[test]
fn a_line_that_awaits_more_than_one_act_says_how_many_ended() {
  let (app, _daemon, _peer) = opened(&format!("Run both checks\n{FANS}"), "1 agent done");
  shows(&app, "1 of 2 ended");
}
