//! The window of furb: what a person looks at, drawn in the terminal with OpenTUI.
//!
//! The window is a thin view of the daemon of each machine. It dials a daemon as `docs/protocol.md`
//! says, with the types of `furb-host`, and it starts the daemon of this machine when none listens.
//! Its model keeps what the daemons told and where the hand of the operator is. Each part of the
//! [`Screen`] shows a function of the model and draws only when its value changes. The window
//! keeps no state that a daemon or the fold of `furb::view` gives, and no rule of a card.
//!
//! [`run`] is the program `furb-window`. [`open`] draws the window on an app that stands already,
//! as the headless app of a test. [`tokens`] holds each color, each loader and each role of a text
//! of the design, which each screen reads.

mod code;
mod error;
mod feed;
mod hand;
mod home;
mod input;
mod kept;
mod keys;
mod line;
mod link;
mod list;
mod machine;
mod model;
mod opens;
mod prose;
mod question;
mod screen;
mod thread;
pub mod tokens;

use std::{
  env,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

use furb::world;
use jiff::tz::TimeZone;
use opentui::{
  App, AppOptions,
  host::HostOptions,
  native::{MouseTracking, TerminalOptions},
};

pub use error::Error;
pub use keys::System;
pub use machine::Machine;
use screen::Screen;

/// What a window stands on, beside its machines.
#[derive(Debug, Clone)]
pub struct Options {
  /// The folder of the window, whose project a new session starts in.
  pub folder: PathBuf,
  /// The home folder of the operator, which a path shows as `~`.
  pub home: Option<PathBuf>,
  /// The file where the window keeps the cards that it saw.
  pub kept: PathBuf,
  /// The zone of each hour that the window writes.
  pub zone: TimeZone,
  /// The second of the clock, from the epoch of unix.
  pub clock: fn() -> u64,
  /// The system of the client, which says how a text of the window writes a key.
  pub system: System,
}

impl Options {
  /// The window of the program: in the folder of its process, on the cache of furb, in the zone
  /// and on the clock of this machine.
  pub fn here() -> Result<Self, Error> {
    let folder = env::current_dir().map_err(Error::Folder)?;
    let kept = world::cache().ok_or(Error::NoCache)?.join("window").join("cards.json");
    let clock = || SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let (zone, system) = (TimeZone::system(), System::here());
    Ok(Self { folder, home: env::home_dir(), kept, zone, clock, system })
  }
}

/// The window on the terminal of the program, on the daemon of this machine, until the operator
/// leaves it. The app asks the terminal for its colors, so the window stands on its background,
/// and for each move of the pointer, so what takes a click rises under it.
pub fn run() -> Result<(), Error> {
  let (machine, options) = (Machine::here()?, Options::here()?);
  let terminal =
    TerminalOptions { mouse: MouseTracking::Motion, ..HostOptions::default().terminal };
  let host = HostOptions { terminal, ..HostOptions::default() };
  let app = App::new(AppOptions { palette_size: 16, host, ..AppOptions::default() })?;
  open(&app, vec![machine], options);
  Ok(app.run()?)
}

/// The window drawn on an app, on the daemon of each machine, which it dials at once. The first
/// machine is the one of the window.
pub fn open(app: &App, machines: Vec<Machine>, options: Options) {
  hand::opened(app, machines, options);
}
