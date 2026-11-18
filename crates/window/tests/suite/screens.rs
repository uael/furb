//! The captures of the window: a frame as text, in a snapshot of insta in `tests/screens` that git
//! holds. `INSTA_UPDATE=always cargo test -p furb-window` draws each capture again, and a run with
//! no such setting writes a frame that differs beside its capture as `.snap.new`.
//! `FURB_CELLS=<folder>` also writes the cells of each capture there as JSON, which a rasterizer of
//! cells draws as an image with the colors of the frame.

#![cfg(test)]

use std::{env, fs, path::Path};

use opentui::{
  native::{Capture, Cell},
  style::Color,
  testing::TestApp,
};
use serde_json::json;

use crate::daemon::PATIENCE;

/// The frame of an app held against its capture. A capture holds no blank row after the last row
/// that shows something, as the hooks keep a file. A test runs in the folder of its package.
pub(crate) fn screen(name: &str, app: &TestApp) {
  if let Some(folder) = env::var_os("FURB_CELLS") {
    fs::write(Path::new(&folder).join(name).with_extension("json"), cells(&app.render())).unwrap();
  }
  // A capture holds the frame where the window settles, drawn and compared the same way: a frame
  // that only passes by, before an answer that the test does not wait for, never matches it.
  let frame = still(app);
  insta::with_settings!({
    snapshot_path => "../screens",
    prepend_module_to_snapshot => false,
    omit_expression => true,
  }, {
    insta::assert_snapshot!(name, frame);
  });
}

/// The frame of an app once it stays the same over many polls of a wait, between which a daemon
/// of the test answers what the window asked.
fn still(app: &TestApp) -> String {
  let (mut frame, mut same) = (app.text(), 0);
  app.wait_until(PATIENCE, || {
    let again = app.text();
    same = if again == frame { same + 1 } else { 0 };
    frame = again;
    same >= STILL
  });
  frame.trim_end().to_owned() + "\n"
}

/// How many polls of a wait show the same frame before a capture takes it.
const STILL: u32 = 50;

/// The cells of a frame as JSON: each row as its runs, and each run as its text, its color, the
/// color of its ground and its attributes.
fn cells(frame: &Capture) -> String {
  let hex = |color: Color| format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
  let rows = frame.cells.chunks(frame.width as usize).map(|row| {
    let runs = row.chunk_by(|one, next| {
      (one.fg, one.bg, one.attributes) == (next.fg, next.bg, next.attributes)
    });
    let run = |cells: &[Cell]| {
      let text: String = cells.iter().map(|cell| cell.symbol.as_str()).collect();
      json!([text, hex(cells[0].fg), hex(cells[0].bg), cells[0].attributes.bits()])
    };
    runs.map(run).collect::<Vec<_>>()
  });
  json!({ "cols": frame.width, "rows": frame.height, "lines": rows.collect::<Vec<_>>() })
    .to_string()
}
