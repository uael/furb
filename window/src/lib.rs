//! The window of furb: what a person looks at, drawn in the terminal with OpenTUI.
//!
//! The window is a thin view of the daemon of each machine. It dials a daemon as `docs/protocol.md`
//! says, with the types of `furb-host`, and it starts the daemon of this machine when none listens.
//! When a connection ends, it dials that machine again until it answers.
//! Its model keeps what the daemons told and where the hand of the operator is. Each part of the
//! [`Screen`] shows a function of the model and draws only when its value changes. The window
//! keeps no state that a daemon or the fold of `furb::view` gives, and no rule of a card.
//!
//! [`run`] is the program `furb-window`. [`open`] draws the window on an app that stands already,
//! as the headless app of a test. [`tokens`] holds each color, each loader and each role of a text
//! of the design, which each screen reads.

mod code;
mod complete;
mod context;
mod error;
mod feed;
mod hand;
mod history;
mod home;
mod input;
mod kept;
mod keys;
mod kit;
mod line;
mod link;
mod list;
mod machine;
mod model;
mod opens;
mod pick;
mod prose;
mod question;
mod review;
mod running;
mod screen;
mod thread;
mod time;
pub mod tokens;

use std::{
  env, io,
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
  /// The millisecond of the clock, from the epoch of unix.
  pub clock: fn() -> u64,
  /// Whether the clocks of the window stand still, as a capture of its frames asks with
  /// `FURB_STILL_CLOCK`, the second of its clock: no mark moves and no time passes, so two
  /// captures of one walk are equal.
  pub still: bool,
  /// The system of the client, which says how a text of the window writes a key.
  pub system: System,
  /// The opener of an address of a link in the browser of the client.
  pub browse: fn(&str) -> io::Result<()>,
}

impl Options {
  /// The window of the program: in the folder of its process, on the cache of furb, in the zone
  /// and on the clock of this machine.
  pub fn here() -> Result<Self, Error> {
    let folder = env::current_dir().map_err(Error::Folder)?;
    // The daemon names a record by its path with no link, so a new record of the folder takes it.
    let folder = dunce::canonicalize(&folder).unwrap_or(folder);
    let kept = world::cache().ok_or(Error::NoCache)?.join("window").join("cards.json");
    let clock = || {
      let wall = || {
        let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
      };
      still().unwrap_or_else(wall)
    };
    let (zone, system) = (TimeZone::system(), System::here());
    // The home holds no link either, so a path of the folder shows its home as `~`.
    let home = env::home_dir().map(|home| dunce::canonicalize(&home).unwrap_or(home));
    let browse = world::browse;
    Ok(Self { folder, home, kept, zone, clock, still: still().is_some(), system, browse })
  }
}

/// The millisecond at which the clocks of the window stand still, from the second that
/// `FURB_STILL_CLOCK` names.
fn still() -> Option<u64> {
  env::var("FURB_STILL_CLOCK").ok()?.parse::<u64>().ok().map(|second| second * 1000)
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
