//! The replay of a recording, of the window or of another program, into its key frames: one at
//! each marker, and one at each step of time where the screen changed. Each key frame is written as
//! its text, its cells as JSON and, for a marker, its image.

use std::{collections::HashMap, fmt::Write, fs, path::Path, time::Duration};

use opentui::native::{
  Buffer, Context, ContextOptions, EmbeddedTerminal, EmbeddedTerminalOptions, WidthMethod,
};

use crate::{
  cast::{Cast, What},
  error::Error,
  frame::{BACKGROUND, FOREGROUND, Frame, PALETTE, Rgb},
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
/// `CSI ? 2026 l`, show at once at its end, so no screen holds half of a frame of the program. It
/// is the terminal of Ghostty that OpenTUI embeds, whose cell holds a whole character as a terminal
/// joins it, as a flag of an emoji with its joiners.
pub(crate) struct Screen {
  terminal: EmbeddedTerminal,
  buffer: Buffer,
  inside: bool,
  shown: Frame,
}

impl Screen {
  pub(crate) fn new(cast: &Cast) -> Result<Self, Error> {
    let (cols, rows) = sized(cast.cols, cast.rows)?;
    let context = Context::new(ContextOptions::default())?;
    let options = EmbeddedTerminalOptions { cols, rows, max_scrollback: 0 };
    let terminal = EmbeddedTerminal::new(&context, options)?;
    terminal.write(colors()?.as_bytes())?;
    let buffer = Buffer::new(&context, u32::from(cols), u32::from(rows), WidthMethod::Unicode)?;
    let shown = shot(&terminal, &buffer)?;
    Ok(Self { terminal, buffer, inside: false, shown })
  }

  /// What the program wrote, or the new size of the terminal.
  pub(crate) fn feed(&mut self, what: &What) -> Result<(), Error> {
    match what {
      What::Output(data) => {
        self.terminal.write(data.as_bytes())?;
        self.inside = match (data.rfind(BEGIN), data.rfind(END)) {
          (Some(begin), Some(end)) => begin > end,
          (Some(_), None) => true,
          (None, Some(_)) => false,
          (None, None) => self.inside,
        };
      }
      What::Resize { cols, rows } => {
        let (cols, rows) = sized(*cols, *rows)?;
        self.terminal.resize(cols, rows)?;
        self.buffer.resize(u32::from(cols), u32::from(rows))?;
      }
      What::Input(_) | What::Marker(_) | What::Other => return Ok(()),
    }
    if !self.inside {
      self.shown = shot(&self.terminal, &self.buffer)?;
    }
    Ok(())
  }

  /// The frame that the screen shows.
  pub(crate) fn frame(&self) -> &Frame {
    &self.shown
  }
}

/// The columns and the rows of a terminal, each from 1 to the most that a terminal takes.
fn sized(cols: usize, rows: usize) -> Result<(u16, u16), Error> {
  let size = |count: usize| u16::try_from(count).ok().filter(|count| *count > 0);
  size(cols).zip(size(rows)).ok_or(Error::Cast("its size is 0 or past 65535"))
}

/// The frame that a terminal shows now, drawn into a buffer of its size.
fn shot(terminal: &EmbeddedTerminal, buffer: &Buffer) -> Result<Frame, Error> {
  terminal.compose(buffer, 0, 0)?;
  Ok(Frame::of(&buffer.capture()?))
}

/// What sets the colors of the terminal of a replay to those that a frame reads as the default,
/// so a frame of any program reads the same colors: the ground, the text and the 16 colors.
fn colors() -> Result<String, Error> {
  let rgb = |Rgb([r, g, b]): Rgb| format!("rgb:{r:02x}/{g:02x}/{b:02x}");
  let mut set = format!("\x1b]10;{}\x1b\\\x1b]11;{}\x1b\\", rgb(FOREGROUND), rgb(BACKGROUND));
  for (at, one) in PALETTE.into_iter().enumerate() {
    write!(set, "\x1b]4;{at};{}\x1b\\", rgb(Rgb::hex(one)))?;
  }
  Ok(set)
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
  // The key of the last tick, whose frame the next tick compares.
  let mut last: Option<usize> = None;
  let mut labels: HashMap<&str, usize> = HashMap::new();
  for event in &cast.events {
    while let (Some(at), Some(step)) = (tick, every) {
      if at > event.at {
        break;
      }
      if last.and_then(|one| keys.get(one)).map(|one| &one.frame) != Some(screen.frame()) {
        let name = format!("t{:06}", at.as_millis());
        last = Some(keys.len());
        keys.push(Key { name, at, frame: screen.frame().clone(), marker: false });
      }
      tick = Some(at + step);
    }
    screen.feed(&event.what)?;
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
    screen.feed(&event.what)?;
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
