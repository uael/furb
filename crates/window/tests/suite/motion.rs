//! What moves in the window while something works, frame by frame over the clock of the window.
#![expect(clippy::string_slice, reason = "a panic is how a test fails")]
#![cfg(test)]

use std::time::Duration;

use furb_window::{open, tokens::Tokens};
use opentui::{style::Color, testing::TestApp};

use crate::{
  cards::{app, options, row, shows},
  daemon::{ASKS, AWAITS, Daemon, LONG, PONDERS, THINKS},
};

/// A window on a machine with one session whose thread says a message, opened by a click on its
/// card, once the thread shows a text.
fn opened(message: &str, shown: &str) -> (TestApp, Daemon) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("motion.jsonl"), true);
  peer.said(&life, message);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, &message[..15]));
  shows(&app, shown);
  (app, daemon)
}

/// The color of each letter of a text in a frame, on the last row that shows it from a column.
fn letters(app: &TestApp, text: &str, from: usize) -> Vec<Color> {
  let frame = app.render();
  let width = frame.width as usize;
  let rows = frame.text();
  let found = rows.lines().enumerate().filter_map(|(y, line)| {
    let after: String = line.chars().skip(from).collect();
    after.find(text).map(|at| (from + after[..at].chars().count(), y))
  });
  let (x, y) = found.last().unwrap_or_else(|| panic!("the window shows no {text:?}:\n{rows}"));
  (x..x + text.chars().count()).map(|at| frame.cells[y * width + at].fg).collect()
}

/// The colors of each letter of a text, in each frame of a stretch of the clock of the window,
/// a frame every 40 ms.
fn over(app: &TestApp, text: &str, from: usize, millis: u64) -> Vec<Vec<Color>> {
  (0..millis / 40)
    .map(|_| {
      app.advance(Duration::from_millis(40));
      letters(app, text, from)
    })
    .collect()
}

fn bright(color: Color) -> u16 {
  u16::from(color.r) + u16::from(color.g) + u16::from(color.b)
}

/// The place of the brightest letter of a frame, the first of the brightest: none while each
/// letter has one color.
fn front(colors: &[Color]) -> Option<usize> {
  let same = colors.iter().all(|one| *one == colors[0]);
  let first = colors.iter().enumerate().max_by_key(|(at, one)| (bright(**one), usize::MAX - at));
  (!same).then(|| first.map(|(at, _)| at)).flatten()
}

#[test]
fn a_light_coasts_over_the_word_of_work_from_its_left_then_whitens_it_whole_then_fades() {
  let (app, _daemon) = opened(&format!("where does the time of a resume go\n{LONG}"), "Working ·");
  let tokens = Tokens::on(None);
  // Two beats of the light, on the row under the input.
  let frames = over(&app, "Working", 0, 4800);
  let whole = |color: Color| frames.iter().any(|one| one.iter().all(|letter| *letter == color));
  assert!(whole(tokens.hover) && whole(tokens.accent), "{frames:?}");
  // While it passes, its front only moves right, and crosses the word.
  let fronts: Vec<Option<usize>> = frames.iter().map(|one| front(one)).collect();
  let sweeps: Vec<Vec<usize>> =
    fronts.split(Option::is_none).map(|run| run.iter().flatten().copied().collect()).collect();
  let crossed = sweeps.iter().filter(|run| run.len() > 3);
  assert!(crossed.clone().count() > 0, "{fronts:?}");
  for run in crossed {
    let fades = run.iter().position(|at| *at == 0 && run[0] != 0).unwrap_or(run.len());
    let passing = &run[..fades];
    assert!(passing.is_sorted() && passing.first() < passing.last(), "{fronts:?}");
  }
}

#[test]
fn a_light_passes_over_the_word_of_a_model_that_thinks_and_not_once_it_thought() {
  let (app, _daemon) = opened(&format!("why is the seed slow\n{PONDERS}"), "thinks ·");
  let frames = over(&app, "thinks", 0, 2400);
  assert!(frames.iter().any(|one| front(one).is_some()), "{frames:?}");
  let (app, _daemon) = opened(&format!("why is the seed slow\n{THINKS}"), "thought ·");
  let frames = over(&app, "thought", 0, 2400);
  assert!(frames.iter().all(|one| *one == frames[0]), "{frames:?}");
}

#[test]
fn the_call_of_prompt_that_a_line_awaits_shines_while_its_agent_works() {
  let (app, _daemon) = opened(&format!("count the lines of the record\n{AWAITS}"), "lines = await");
  let frames = over(&app, "prompt(", 0, 2400);
  assert!(frames.iter().any(|one| front(one).is_some()), "{frames:?}");
  // The rest of the line keeps its colors.
  let rest = over(&app, "lines = await", 0, 2400);
  assert!(rest.iter().all(|one| *one == rest[0]), "{rest:?}");
}

#[test]
fn the_arrow_of_an_agent_pulses_from_muted_to_its_color_and_back() {
  let (app, _daemon) = opened(&format!("count the lines of the record\n{AWAITS}"), "lines = await");
  let tokens = Tokens::on(None);
  let frames = over(&app, "↳", 0, 1200);
  let colors: Vec<Color> = frames.iter().map(|one| one[0]).collect();
  assert!(colors.contains(&tokens.muted), "{colors:?}");
  let peak = colors.iter().max_by_key(|one| bright(**one)).copied();
  assert!(peak.is_some_and(|one| one != tokens.muted), "{colors:?}");
  let again = over(&app, "↳", 0, 1200);
  assert_eq!(again.iter().map(|one| one[0]).collect::<Vec<_>>(), colors);
}

#[test]
fn a_card_that_begins_to_call_its_operator_pulses_in_its_hue_and_settles() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("pulse.jsonl"), true);
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  peer.said(&life, &format!("which key sends the queue\n{ASKS}"));
  shows(&app, "◆ Input");
  let hues = Tokens::on(None).hues;
  // The bar at the left of the card and its color, as the clock of the window moves.
  let bar = |app: &TestApp| {
    let frame = app.render();
    let text = frame.text();
    let y = text.lines().position(|line| line.contains("which key sends")).unwrap_or_default();
    let cell = &frame.cells[y * frame.width as usize + 62];
    (cell.symbol.clone(), cell.fg)
  };
  let lit: Vec<(String, Color)> = (0..40)
    .map(|_| {
      app.advance(Duration::from_millis(40));
      bar(&app)
    })
    .collect();
  let pulses = |one: &(String, Color)| one.0 == "▐" && hues.contains(&one.1);
  assert!(lit.iter().any(pulses) && !lit.last().is_some_and(pulses), "{lit:?}");
}
