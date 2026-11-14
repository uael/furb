//! The captures of the window: a frame as text, in a file of `tests/screens` that git holds.
//! `FURB_DRAW=1 cargo test -p furb-window` draws each capture again.

#![cfg(test)]

use std::{env, fs, path::Path};

use opentui::testing::TestApp;

/// The frame of an app held against its capture, or written as its capture when `FURB_DRAW` is
/// set. A capture holds no blank row after the last row that shows something, as the hooks keep a
/// file. A test runs in the folder of its package.
pub(crate) fn screen(name: &str, app: &TestApp) {
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
