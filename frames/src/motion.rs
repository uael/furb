//! The motion of a span of a recording, of the window or of another program: a sheet of its
//! screens at a step of time, cut to the cells that move, which a model reads in one look; the
//! measures that no image gives, from the times of the recording; and a GIF of the whole screen,
//! which a person plays to judge how the motion feels.

use std::{
  collections::HashMap,
  fmt::Write as _,
  fs::{self, File},
  io::BufWriter,
  path::{Path, PathBuf},
  time::Duration,
};

use gif::{Encoder, Repeat};

use crate::{
  cast::{Cast, What},
  error::Error,
  frame::{Cell, Frame, Rgb},
  raster::{self, Image},
  replay::{Screen, put},
};

/// The longest that a cell may show another look before it comes back for the measures to call it
/// a flicker: two frames of a screen at 60 Hz.
const FLICKER: Duration = Duration::from_millis(34);

/// The most columns of tiles that a row of the sheet holds side by side.
const SHEET: usize = 160;

/// The least columns of a tile, which its time needs.
const TILE: usize = 16;

/// The most tiles of a sheet, and the most rows of cells that its tiles take, so a sheet stays an
/// image that a model reads in one look: a span that holds more steps shows its first ones, and
/// says so.
const TILES: usize = 48;
const DEEP: usize = 120;

/// The look of the time over each tile, and of the ground between tiles.
const LABEL: Rgb = Rgb::hex(0x9a_9a9a);
const GROUND: Rgb = Rgb::hex(0x00_0000);

/// A span of a recording, from a moment to a moment, and how its sheet looks at it: the step of
/// time between two tiles, and the rows to cut to, or none for the cells that move most.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Span {
  pub(crate) from: Duration,
  pub(crate) until: Duration,
  pub(crate) every: Duration,
  pub(crate) rows: Option<(usize, usize)>,
}

/// A screen of a recording from the moment that it showed, and its cells.
struct Shown {
  at: Duration,
  frame: Frame,
  cells: Vec<Vec<Cell>>,
}

/// The cells of a row that showed another look for a moment: the row, the first and the last of
/// those columns, how many cells, the text between them, for how long and from when.
struct Flicker {
  row: usize,
  from: usize,
  until: usize,
  cells: usize,
  shown: String,
  lasted: Duration,
  at: Duration,
}

/// A key that the program read, at its moment.
struct Key {
  at: Duration,
  read: String,
}

impl Shown {
  fn of(at: Duration, frame: Frame) -> Self {
    let cells = frame.cells();
    Self { at, frame, cells }
  }
}

/// The sheet, the GIF and the measures of a span, written beside a path as `<path>.sheet.png`,
/// `<path>.gif` and `<path>.motion.txt`. It gives the measures.
pub(crate) fn motion(cast: &Cast, span: Span, out: &Path) -> Result<String, Error> {
  let (screens, keys) = shown(cast, span)?;
  let moving = moving(&screens, span.rows);
  let every = span.every;
  let mut measures = measures(&screens, &keys, span, moving);
  let (sheet, tiles) = sheet(&screens, span, every, moving);
  let steps = span.until.saturating_sub(span.from).as_millis() / every.as_millis().max(1) + 1;
  if steps > tiles as u128 {
    let last = span.from + every * u32::try_from(tiles - 1).unwrap_or(u32::MAX);
    writeln!(
      measures,
      "sheet: the first {tiles} of {steps} steps, to {:.3} s: a shorter span or fewer rows show the rest",
      last.as_secs_f64()
    )
    .ok();
  }
  let named = out.as_os_str().to_owned();
  let path = |suffix: &str| {
    let mut path = named.clone();
    path.push(format!(".{suffix}"));
    PathBuf::from(path)
  };
  if let Some(folder) = out.parent() {
    fs::create_dir_all(folder)
      .map_err(|source| Error::Write { path: folder.to_owned(), source })?;
  }
  put(&path("sheet.png"), &raster::image(&sheet).0.png()?)?;
  animated(&screens, span, &path("gif"))?;
  put(&path("motion.txt"), measures.as_bytes())?;
  Ok(measures)
}

/// Each screen of a recording in a span, the one that stands at its start first, and each key that
/// the program read in it with its moment.
fn shown(cast: &Cast, span: Span) -> Result<(Vec<Shown>, Vec<Key>), Error> {
  let mut screen = Screen::new(cast)?;
  let (mut screens, mut keys): (Vec<Shown>, _) = (Vec::new(), Vec::new());
  for event in cast.events.iter().take_while(|event| event.at <= span.until) {
    if event.at >= span.from && screens.is_empty() {
      screens.push(Shown::of(span.from, screen.frame().clone()));
    }
    match &event.what {
      What::Output(_) | What::Resize { .. } => screen.feed(&event.what),
      What::Input(read) if event.at >= span.from => {
        keys.push(Key { at: event.at, read: read.clone() });
      }
      What::Input(_) | What::Marker(_) | What::Other => continue,
    }
    let frame = screen.frame().clone();
    // The writes of one millisecond make one screen: a program draws a frame in several writes, a
    // person sees the last alone, and a recording of a rival stamps no finer.
    if screens.len() > 1
      && screens.last().is_some_and(|last| last.at.as_millis() == event.at.as_millis())
    {
      screens.pop();
    }
    if !screens.is_empty() && screens.last().is_none_or(|last| last.frame != frame) {
      screens.push(Shown::of(event.at, frame));
    }
  }
  if screens.is_empty() {
    screens.push(Shown::of(span.from, screen.frame().clone()));
  }
  Ok((screens, keys))
}

/// The rows and the columns of the cells that move most in a span, each from the first to the
/// last, or none when no cell changes.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Moving {
  rows: (usize, usize),
  cols: (usize, usize),
}

/// The cells that move most: each cell that changes at least a quarter as often as the cell that
/// changes most, so a loader that turns stands out from a line that came once. In the rows that a
/// word names, each cell of them that changes.
fn moving(screens: &[Shown], only: Option<(usize, usize)>) -> Option<Moving> {
  let mut counts: HashMap<(usize, usize), usize> = HashMap::new();
  for pair in screens.windows(2) {
    for (row, cells) in pair[1].cells.iter().enumerate() {
      if only.is_some_and(|(first, last)| row < first || row > last) {
        continue;
      }
      for (col, cell) in cells.iter().enumerate() {
        if pair[0].cells.get(row).and_then(|cells| cells.get(col)) != Some(cell) {
          *counts.entry((row, col)).or_default() += 1;
        }
      }
    }
  }
  let most = counts.values().copied().max()?;
  let least = if only.is_some() { 1 } else { most.div_ceil(4) };
  let spread = |(low, high): (usize, usize), at: usize| (low.min(at), high.max(at));
  let mut found: Option<Moving> = None;
  for (&(row, col), _) in counts.iter().filter(|(_, count)| **count >= least) {
    found = Some(match found {
      None => Moving { rows: (row, row), cols: (col, col) },
      Some(moving) => Moving { rows: spread(moving.rows, row), cols: spread(moving.cols, col) },
    });
  }
  found
}

/// The measures of a span: the time from each key to the first change after it, how often the
/// moving cells change while they move, how regular those changes are, and each flicker.
fn measures(screens: &[Shown], keys: &[Key], span: Span, moving: Option<Moving>) -> String {
  let seconds = |at: Duration| at.as_secs_f64();
  let mut text = format!(
    "span {:.3} s to {:.3} s: {} screens\n",
    seconds(span.from),
    seconds(span.until),
    screens.len()
  );
  // The moments that the moving cells change, so a loader is measured apart from the rest.
  let changes: Vec<Duration> = screens
    .windows(2)
    .filter(|pair| inside(&pair[0].cells, moving) != inside(&pair[1].cells, moving))
    .map(|pair| pair[1].at)
    .collect();
  match moving {
    Some(moving) => writeln!(
      text,
      "moving cells: rows {} to {}, columns {} to {}",
      moving.rows.0, moving.rows.1, moving.cols.0, moving.cols.1
    ),
    None => writeln!(text, "moving cells: none"),
  }
  .ok();
  text += &latencies(keys, &changes);
  text += &rate(&changes);
  let flickers = flickers(screens);
  let cells: usize = flickers.iter().map(|row| row.cells).sum();
  match flickers.len() {
    0 => text += "flicker: none\n",
    rows => {
      let rows = if rows == 1 { "1 row".to_owned() } else { format!("{rows} rows") };
      writeln!(text, "flicker: {cells} cells on {rows}").ok();
    }
  }
  for row in flickers.iter().take(8) {
    writeln!(
      text,
      "  row {} columns {} to {}, {} cells: {:?} for {} ms at {:.3} s",
      row.row,
      row.from,
      row.until,
      row.cells,
      row.shown,
      row.lasted.as_millis(),
      row.at.as_secs_f64()
    )
    .ok();
  }
  text
}

/// The time from each key to the first change of the moving cells after it.
fn latencies(keys: &[Key], changes: &[Duration]) -> String {
  if keys.is_empty() {
    return "keys: none in the recording\n".to_owned();
  }
  let line = |key: &Key| {
    let at = key.at.as_secs_f64();
    match changes.iter().find(|change| **change > key.at) {
      Some(change) => format!(
        "key {:?} at {at:.3} s: first change after {} ms\n",
        key.read,
        change.saturating_sub(key.at).as_millis()
      ),
      None => format!("key {:?} at {at:.3} s: no change after it\n", key.read),
    }
  };
  keys.iter().map(line).collect()
}

/// How often the moving cells change while they move, and how regular the intervals between two
/// changes are: their mean, the least and the most, and their spread, the deviation from the mean.
fn rate(changes: &[Duration]) -> String {
  let (Some(first), Some(last)) = (changes.first(), changes.last()) else { return String::new() };
  let moved = last.saturating_sub(*first).as_millis().max(1);
  let count = u128::try_from(changes.len()).unwrap_or(u128::MAX);
  let tenths = (count - 1) * 10_000 / moved;
  let mut text = format!(
    "changes of the moving cells: {count} from {:.3} s to {:.3} s, {}.{} per second while they move\n",
    first.as_secs_f64(),
    last.as_secs_f64(),
    tenths / 10,
    tenths % 10
  );
  let intervals: Vec<u128> =
    changes.windows(2).map(|pair| pair[1].saturating_sub(pair[0]).as_millis()).collect();
  if let (Some(least), Some(most)) = (intervals.iter().min(), intervals.iter().max()) {
    let many = u128::try_from(intervals.len()).unwrap_or(u128::MAX);
    let mean = intervals.iter().sum::<u128>() / many;
    let spread =
      (intervals.iter().map(|one| one.abs_diff(mean).pow(2)).sum::<u128>() / many).isqrt();
    let share = (spread * 100).checked_div(mean).unwrap_or_default();
    writeln!(
      text,
      "intervals: mean {mean} ms, least {least}, most {most}, spread {spread} ms ({share}% of the mean)"
    )
    .ok();
  }
  text
}

/// The cells of a screen that move, row by row.
fn inside(cells: &[Vec<Cell>], moving: Option<Moving>) -> Vec<&[Cell]> {
  let Some(moving) = moving else { return Vec::new() };
  (moving.rows.0..=moving.rows.1)
    .filter_map(|row| cells.get(row).and_then(|cells| cells.get(moving.cols.0..=moving.cols.1)))
    .collect()
}

/// Each row of a screen whose cells take another look and come back to their look before within
/// two frames: one for each row and moment, so a word that flickers reads as one.
fn flickers(screens: &[Shown]) -> Vec<Flicker> {
  let mut found = Vec::new();
  for three in screens.windows(3) {
    let [before, between, after] = three else { continue };
    let lasted = after.at.saturating_sub(between.at);
    if lasted > FLICKER {
      continue;
    }
    for (row, cells) in between.cells.iter().enumerate() {
      let flicked: Vec<usize> = (0..cells.len())
        .filter(|&col| {
          let was = before.cells.get(row).and_then(|cells| cells.get(col));
          was == after.cells.get(row).and_then(|cells| cells.get(col)) && was != cells.get(col)
        })
        .collect();
      let (Some(&from), Some(&until)) = (flicked.first(), flicked.last()) else { continue };
      found.push(Flicker {
        row,
        from,
        until,
        cells: flicked.len(),
        shown: cells[from..=until].iter().map(|cell| cell.glyph.as_str()).collect(),
        lasted,
        at: between.at,
      });
    }
  }
  found
}

/// The sheet: the screen at each step of the span, cut to the cells that move with a margin, each
/// under its time from the start of the span, side by side in rows of tiles. A time that shows a
/// new screen says so with a mark. It gives the sheet and how many tiles it holds.
fn sheet(screens: &[Shown], span: Span, every: Duration, moving: Option<Moving>) -> (Frame, usize) {
  let first = &screens[0].frame;
  let (rows, cols) = match moving {
    Some(moving) => {
      let rows = (moving.rows.0.saturating_sub(1), (moving.rows.1 + 1).min(first.rows - 1));
      let wide = (moving.cols.1 - moving.cols.0 + 5).max(TILE).min(first.cols);
      let left = moving.cols.0.saturating_sub(2).min(first.cols - wide);
      (rows, (left, left + wide - 1))
    }
    None => ((0, first.rows - 1), (0, first.cols - 1)),
  };
  let width = cols.1 - cols.0 + 1;
  let height = rows.1 - rows.0 + 2;
  let most = TILES.min((SHEET / (width + 1)).max(1) * (DEEP / (height + 1)).max(1));
  let ground =
    |glyph: &str, fg: Rgb| Cell { glyph: glyph.to_owned(), fg, bg: GROUND, attributes: 0 };
  let mut tiles: Vec<Vec<Vec<Cell>>> = Vec::new();
  let mut at = span.from;
  let mut before: Option<usize> = None;
  loop {
    let shown = screens.iter().rposition(|screen| screen.at <= at).unwrap_or(0);
    let mark = if before.is_some_and(|one| one != shown) { " •" } else { "" };
    let label = format!("{:.3} s{mark}", at.saturating_sub(span.from).as_secs_f64());
    let mut tile = vec![label_row(&label, width, &ground)];
    let cells = &screens[shown].cells;
    tile.extend((rows.0..=rows.1).map(|row| cells[row][cols.0..=cols.1].to_vec()));
    tiles.push(tile);
    before = Some(shown);
    at += every;
    if at > span.until || every.is_zero() || tiles.len() == most {
      break;
    }
  }
  // As many tiles across as make the sheet about as wide as it is high, a cell being about twice
  // as high as it is wide, and never wider than a screen of tiles.
  let (wide, deep) = (width + 1, tiles.len() * height * 2);
  let mut across = (deep / wide).isqrt();
  if across * across * wide < deep {
    across += 1;
  }
  let across = across.clamp(1, (SHEET / wide).max(1));
  let mut sheet: Vec<Vec<Cell>> = Vec::new();
  for band in tiles.chunks(across) {
    for line in 0..height {
      let mut row = Vec::new();
      for tile in band {
        row.extend(tile[line].iter().cloned());
        row.push(ground(" ", LABEL));
      }
      row.resize(across * (width + 1), ground(" ", LABEL));
      sheet.push(row);
    }
    sheet.push(vec![ground(" ", LABEL); across * (width + 1)]);
  }
  (Frame::of_cells(&sheet), tiles.len())
}

/// A row that says a text at its left, as wide as a tile.
fn label_row(label: &str, width: usize, ground: &impl Fn(&str, Rgb) -> Cell) -> Vec<Cell> {
  let mut row: Vec<Cell> = label.chars().map(|glyph| ground(&glyph.to_string(), LABEL)).collect();
  row.resize(width, ground(" ", LABEL));
  row.truncate(width);
  row
}

/// The GIF of the whole screen over the span: each screen for as long as it showed, and a screen
/// that showed less than 5 ms left out, since a GIF counts its delays in hundredths of a second.
fn animated(screens: &[Shown], span: Span, path: &Path) -> Result<(), Error> {
  let wrote = |source| Error::Write { path: path.to_owned(), source };
  let file = File::create(path).map_err(wrote)?;
  let first = raster::image(&screens[0].frame).0;
  let size = |pixels: usize| u16::try_from(pixels).unwrap_or(u16::MAX);
  let mut encoder = Encoder::new(BufWriter::new(file), size(first.width), size(first.height), &[])?;
  encoder.set_repeat(Repeat::Infinite)?;
  for (at, screen) in screens.iter().enumerate() {
    let next = screens.get(at + 1).map_or(span.until, |next| next.at);
    let hundredths = (next.saturating_sub(screen.at).as_millis() + 5) / 10;
    if hundredths == 0 {
      continue;
    }
    let Image { width, height, pixels } = raster::image(&screen.frame).0;
    let mut frame = gif::Frame::from_rgb_speed(size(width), size(height), &pixels, 10);
    frame.delay = u16::try_from(hundredths).unwrap_or(u16::MAX);
    encoder.write_frame(&frame)?;
  }
  Ok(())
}

#[cfg(test)]
#[path = "motion.test.rs"]
mod test;
