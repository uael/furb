//! The walk with no dead click: on each screen at rest, the pointer goes over each cell, and each
//! thing that lights under it takes a click that changes the window, in a fresh window of the same
//! screen.

#![cfg(test)]

use std::{fs, path::PathBuf, time::Duration};

use furb_window::{Options, open};
use opentui::{style::Color, testing::TestApp};

use crate::{
  cards::{app, options, shows},
  daemon::{ANSWERS, CHOOSES, Daemon, Hold, PATIENCE, Peer},
  part, question,
};

/// A screen of the walk: its window and the daemon that serves it, which the walk keeps until it
/// leaves the screen.
type Screen = (TestApp, Daemon, Option<Peer>);

/// The home of a project with no session, with a folder that stands, and the cards of a machine:
/// one thread at rest that he read, which rests with the resolved ones.
fn home() -> Screen {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  peer.rested(
    &sessions.join("gate.jsonl"),
    &format!("remove the gate\n{ANSWERS}"),
    Some(Hold::Seen),
  );
  let folder = daemon.file("furb-try");
  fs::create_dir_all(&folder).unwrap();
  open(&app, vec![daemon.machine("mac")], still(folder, &daemon));
  shows(&app, "Resolved 1");
  (app, daemon, None)
}

/// The list of a machine with a session that works, a session at rest and a session with no thread
/// of his, and no thread open.
fn list() -> Screen {
  let (app, daemon, peer) = part::machine();
  (app, daemon, Some(peer))
}

/// A thread whose model asks him to choose, open, with its form.
fn question() -> Screen {
  let message = format!("where does the list stand\n{CHOOSES}");
  let (app, daemon, peer, ..) = question::opened(&message, "type another answer");
  app.advance(Duration::from_millis(400));
  (app, daemon, Some(peer))
}

/// A thread at rest, open, whose model answered his message.
fn thread() -> Screen {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.served();
  let folder = daemon.file("furb-try");
  fs::create_dir_all(&folder).unwrap();
  open(&app, vec![daemon.machine("mac")], still(folder, &daemon));
  shows(&app, "No thread yet in this project.");
  app.paste(&format!("how long does an open take {ANSWERS}"));
  app.press("enter");
  shows(&app, "Two stages hold most");
  shows(&app, "Resolved 1");
  (app, daemon, None)
}

/// The options of a window of the walk in a folder, whose clocks stand still, so that no loader
/// moves a cell between two frames of the walk.
fn still(folder: PathBuf, daemon: &Daemon) -> Options {
  Options { folder, still: true, ..options(&daemon.file("cards.json")) }
}

/// A thing that lights under the pointer: a cell of it, and its text.
struct Lit {
  x: u16,
  y: u16,
  text: String,
}

/// Each thing that lights under the pointer on a screen at rest, once: the cells that go bright
/// white as the pointer stands on one of its cells.
fn lit(app: &TestApp) -> Vec<Lit> {
  let white = Color::hex(0xff_ffff);
  app.hover(0, 0);
  let base = app.render();
  let (mut seen, mut things) = (Vec::<Vec<usize>>::new(), Vec::new());
  let (width, height) = (u16::try_from(base.width).unwrap(), u16::try_from(base.height).unwrap());
  for y in 0..height {
    for x in 0..width {
      app.hover(x, y);
      let frame = app.render();
      let cells = base.cells.iter().zip(&frame.cells).enumerate();
      let lit: Vec<usize> = cells
        .filter(|(_, (one, other))| one != other && other.fg == white)
        .map(|(at, _)| at)
        .collect();
      if lit.is_empty() || seen.contains(&lit) {
        continue;
      }
      let text = lit.iter().map(|at| frame.cells[*at].symbol.clone()).collect();
      things.push(Lit { x, y, text });
      seen.push(lit);
    }
  }
  things
}

/// Each thing that lights on a screen takes a click that changes the window, but the things that
/// the screen names as quiet, each a text that it holds: a fresh window of the screen for each
/// thing, so no click before it changes what the next one finds.
fn walked(screen: fn() -> Screen, quiet: &[&str]) -> Vec<String> {
  let (app, daemon, peer) = screen();
  let things = lit(&app);
  assert!(!things.is_empty(), "nothing lights on the screen:\n{}", app.text());
  drop((app, daemon, peer));
  let mut dead = Vec::new();
  for thing in things.into_iter().filter(|thing| !quiet.iter().any(|one| thing.text.contains(one)))
  {
    let (app, _daemon, _peer) = screen();
    app.hover(0, 0);
    let before = app.render();
    app.click(thing.x, thing.y);
    let changed = app.wait_until(PATIENCE, || {
      app.hover(0, 0);
      app.render().cells != before.cells
    });
    if !changed {
      dead.push(format!("{:?} at {},{}", thing.text, thing.x, thing.y));
    }
  }
  dead
}

#[test]
fn each_thing_that_lights_on_the_home_takes_a_click() {
  assert_eq!(walked(home, &[]), Vec::<String>::new());
}

#[test]
fn each_thing_that_lights_on_a_thread_at_rest_takes_a_click() {
  // The card of the open thread, which stays in view when the resolved threads fold: its click
  // moves to the end of the thread, which the feed already shows.
  assert_eq!(walked(thread, &["how long does an open"]), Vec::<String>::new());
}

#[test]
fn each_thing_that_lights_on_the_list_of_a_machine_takes_a_click() {
  assert_eq!(walked(list, &[]), Vec::<String>::new());
}

#[test]
fn each_thing_that_lights_on_a_question_takes_a_click() {
  // The card of the open thread shows its end, which the window shows already.
  assert_eq!(walked(question, &["where does the list"]), Vec::<String>::new());
}
