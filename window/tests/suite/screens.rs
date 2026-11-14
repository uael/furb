//! The captures of the window: a frame as text, in a file of `tests/screens` that git holds.
//! `FURB_DRAW=1 cargo test -p furb-window` draws each capture again. `FURB_CELLS=<folder>` also
//! writes the cells of each capture there as JSON, which a rasterizer of cells draws as an image
//! with the colors of the frame.

#![cfg(test)]

use std::{env, fs, path::Path};

use opentui::{
  native::{Capture, Cell},
  style::Color,
  testing::TestApp,
};
use serde_json::json;

/// The frame of an app held against its capture, or written as its capture when `FURB_DRAW` is
/// set. A capture holds no blank row after the last row that shows something, as the hooks keep a
/// file. A test runs in the folder of its package.
pub(crate) fn screen(name: &str, app: &TestApp) {
  if let Some(folder) = env::var_os("FURB_CELLS") {
    fs::write(Path::new(&folder).join(name).with_extension("json"), cells(&app.render())).unwrap();
  }
  let capture = Path::new("tests/screens").join(name).with_extension("txt");
  let frame = app.text().trim_end().to_owned() + "\n";
  if env::var_os("FURB_DRAW").is_some() {
    fs::write(&capture, frame).unwrap();
    return;
  }
  let kept = fs::read_to_string(&capture).unwrap_or_default();
  let how = "`FURB_DRAW=1 cargo test -p furb-window` draws it again";
  assert_eq!(frame, kept, "the frame is not the capture {}: {how}", capture.display());
}

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
