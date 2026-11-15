//! The replay of a recording, of the window or of another program, into its key frames: one at
//! each marker, and one at each step of time where the screen changed. Each key frame is written as
//! its text, its cells as JSON and, for a marker, its image.

use std::{collections::HashMap, fs, path::Path, time::Duration};

use avt::Terminal;

use crate::{
  cast::{Cast, What},
  error::Error,
  frame::Frame,
  raster,
};

/// A key frame: its name, the time of the recording that it shows, and the frame. A marker names
/// its frame by its label; a step of time by `t` and its milliseconds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Key {
  pub(crate) name: String,
  pub(crate) at: Duration,
  pub(crate) frame: Frame,
  pub(crate) marker: bool,
}

/// A terminal as a person sees it: the writes of a synchronized update, from `CSI ? 2026 h` to
/// `CSI ? 2026 l`, show at once at its end, so no screen holds half of a frame of the program.
pub(crate) struct Screen {
  terminal: Terminal,
  inside: bool,
  shown: Frame,
}

impl Screen {
  pub(crate) fn new(cast: &Cast) -> Result<Self, Error> {
    let terminal = Terminal::new(cast.cols, cast.rows).ok().ok_or(Error::Cast("its size is 0"))?;
    let shown = Frame::of(&terminal);
    Ok(Self { terminal, inside: false, shown })
  }

  /// What the program wrote, or the new size of the terminal.
  pub(crate) fn feed(&mut self, what: &What) {
    match what {
      What::Output(data) => {
        self.terminal.feed([data.as_str()]);
        self.inside = match (data.rfind(BEGIN), data.rfind(END)) {
          (Some(begin), Some(end)) => begin > end,
          (Some(_), None) => true,
          (None, Some(_)) => false,
          (None, None) => self.inside,
        };
      }
      What::Resize { cols, rows } => self.terminal.resize(*cols, *rows),
      What::Input(_) | What::Marker(_) | What::Other => return,
    }
    if !self.inside {
      self.shown = Frame::of(&self.terminal);
    }
  }

  /// The frame that the screen shows.
  pub(crate) fn frame(&self) -> &Frame {
    &self.shown
  }
}

/// The start and the end of a synchronized update.
const BEGIN: &str = "\x1b[?2026h";
const END: &str = "\x1b[?2026l";

/// The key frames of a recording. A frame at a marker shows what the program wrote before it. A
/// label that comes again takes its count, as `a-2`.
pub(crate) fn keys(cast: &Cast, every: Option<Duration>) -> Result<Vec<Key>, Error> {
  let mut screen = Screen::new(cast)?;
  let mut keys: Vec<Key> = Vec::new();
  let mut tick = every;
  let mut last: Option<Frame> = None;
  let mut labels: HashMap<&str, usize> = HashMap::new();
  for event in &cast.events {
    while let (Some(at), Some(step)) = (tick, every) {
      if at > event.at {
        break;
      }
      let frame = screen.frame().clone();
      if last.as_ref() != Some(&frame) {
        let name = format!("t{:06}", at.as_millis());
        keys.push(Key { name, at, frame: frame.clone(), marker: false });
        last = Some(frame);
      }
      tick = Some(at + step);
    }
    screen.feed(&event.what);
    if let What::Marker(label) = &event.what {
      let seen = labels.entry(label.as_str()).and_modify(|seen| *seen += 1).or_insert(1);
      let name = if *seen == 1 { label.clone() } else { format!("{label}-{seen}") };
      keys.push(Key { name, at: event.at, frame: screen.frame().clone(), marker: true });
    }
  }
  Ok(keys)
}

/// Whether the screen of a recording shows a text at a moment from a time to a time after it: the
/// replay holds what tmux showed while the program moved, as a loader moves.
pub(crate) fn shows(
  cast: &Cast,
  from: Duration,
  until: Duration,
  text: &str,
) -> Result<bool, Error> {
  let mut screen = Screen::new(cast)?;
  for event in cast.events.iter().take_while(|event| event.at <= until) {
    screen.feed(&event.what);
    if event.at >= from && screen.frame().text() == text {
      return Ok(true);
    }
  }
  Ok(false)
}

/// Each key frame of a recording written in a folder as `<stem>.<name>.txt` and `.json`, and a
/// frame at a marker as `.png` too. It gives the glyphs that no font of the image holds, each
/// once, in the order that they came.
pub(crate) fn write(keys: &[Key], folder: &Path, stem: &str) -> Result<Vec<char>, Error> {
  fs::create_dir_all(folder).map_err(|source| Error::Write { path: folder.to_owned(), source })?;
  let mut missing: Vec<char> = Vec::new();
  for key in keys {
    let path = |suffix: &str| folder.join(format!("{stem}.{}.{suffix}", key.name));
    put(&path("txt"), key.frame.text().as_bytes())?;
    put(&path("json"), serde_json::to_string(&key.frame)?.as_bytes())?;
    if key.marker {
      let (image, lacks) = raster::png(&key.frame)?;
      put(&path("png"), &image)?;
      missing
        .extend(lacks.into_iter().filter(|glyph| !missing.contains(glyph)).collect::<Vec<_>>());
    }
  }
  Ok(missing)
}

pub(crate) fn put(path: &Path, bytes: &[u8]) -> Result<(), Error> {
  fs::write(path, bytes).map_err(|source| Error::Write { path: path.to_owned(), source })
}

#[cfg(test)]
#[path = "replay.test.rs"]
mod test;
