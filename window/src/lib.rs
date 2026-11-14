//! The window of furb: what a person looks at, drawn in the terminal with OpenTUI.
//!
//! The window is a thin view of the daemon of a machine. It dials the daemon as `docs/protocol.md`
//! says, with the types of `furb-host`, and it starts the daemon when none listens. A [`Screen`]
//! keeps what the daemon told, and each part of it shows a function of that and draws only when
//! its value changes. The window keeps no state that the daemon or the fold of `furb::view` gives.
//!
//! [`run`] is the program `furb-window`. [`open`] draws the window on an app that stands already,
//! as the headless app of a test. [`tokens`] holds each color, each loader and each role of a text
//! of the design, which each screen reads.

mod error;
mod link;
mod machine;
mod proof;
mod screen;
mod spinner;
pub mod tokens;

use opentui::{App, AppOptions};

pub use error::Error;
pub use machine::Machine;
use screen::Screen;
use tokens::Tokens;

/// The window on the terminal of the program, on the daemon of this machine, until the operator
/// leaves it. The app asks the terminal for its colors, so the window stands on its background.
pub fn run() -> Result<(), Error> {
  let machine = Machine::here()?;
  let app = App::new(AppOptions { palette_size: 16, ..AppOptions::default() })?;
  open(&app, machine);
  Ok(app.run()?)
}

/// The window drawn on an app, on the daemon of a machine, which it dials at once.
pub fn open(app: &App, machine: Machine) {
  let tokens = Tokens::on(app.palette().and_then(|palette| palette.background));
  app.set_theme(tokens.theme());
  let screen = proof::drawn(app, &tokens, &machine.name);
  link::dialed(app, machine, screen);
}
