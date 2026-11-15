//! A frame: the grid of a terminal at one moment, as rows of runs of one look. Its JSON is the one
//! that each frame of the window and of another program takes, so one reader reads both.

use std::{array, fs, path::Path};

use avt::{Color, Line, Pen, Terminal};
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;

use crate::error::Error;

/// The bits of the attributes of a run.
pub(crate) const BOLD: u8 = 1;
pub(crate) const DIM: u8 = 2;
pub(crate) const ITALIC: u8 = 4;
pub(crate) const UNDERLINE: u8 = 8;
pub(crate) const REVERSE: u8 = 32;
pub(crate) const STRIKE: u8 = 128;

/// The colors of a cell that says none: those of the terminal of the captures of the other
/// programs, so a frame of any program reads the same default.
const FOREGROUND: Rgb = Rgb([0xd4, 0xd4, 0xd4]);
const BACKGROUND: Rgb = Rgb([0x1e, 0x1e, 0x1e]);

/// The 16 colors of the terminal.
const PALETTE: [u32; 16] = [
  0x00_0000, 0xcd_3131, 0x0d_bc79, 0xe5_e510, 0x24_72c8, 0xbc_3fbc, 0x11_a8cd, 0xe5_e5e5,
  0x66_6666, 0xf1_4c4c, 0x23_d18b, 0xf5_f543, 0x3b_8eea, 0xd6_70d6, 0x29_b8db, 0xff_ffff,
];

/// A color of a cell, which the JSON of a frame writes `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Rgb(pub(crate) [u8; 3]);

impl Rgb {
  pub(crate) const fn hex(value: u32) -> Self {
    let [_, r, g, b] = value.to_be_bytes();
    Self([r, g, b])
  }

  /// The color halfway toward another, as a dim text takes it.
  fn half(self, other: Self) -> Self {
    Self(array::from_fn(|at| u8::midpoint(self.0[at], other.0[at])))
  }
}

impl From<Rgb> for String {
  fn from(color: Rgb) -> Self {
    let [r, g, b] = color.0;
    format!("#{r:02x}{g:02x}{b:02x}")
  }
}

impl TryFrom<String> for Rgb {
  type Error = Error;

  fn try_from(text: String) -> Result<Self, Error> {
    let digits = text.strip_prefix('#').filter(|digits| digits.len() == 6);
    let value = digits.and_then(|digits| u32::from_str_radix(digits, 16).ok());
    value.map(Self::hex).ok_or(Error::Color(text))
  }
}

/// A text of one look: its color, the color of its ground and its attributes. The JSON writes it
/// as `[text, fg, bg, attributes]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "(String, Rgb, Rgb, u8)", into = "(String, Rgb, Rgb, u8)")]
pub(crate) struct Run {
  pub(crate) text: String,
  pub(crate) fg: Rgb,
  pub(crate) bg: Rgb,
  pub(crate) attributes: u8,
}

impl From<(String, Rgb, Rgb, u8)> for Run {
  fn from((text, fg, bg, attributes): (String, Rgb, Rgb, u8)) -> Self {
    Self { text, fg, bg, attributes }
  }
}

impl From<Run> for (String, Rgb, Rgb, u8) {
  fn from(run: Run) -> Self {
    (run.text, run.fg, run.bg, run.attributes)
  }
}

/// One cell of a frame: its glyph, empty in the second cell of a wide glyph, and its look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cell {
  pub(crate) glyph: String,
  pub(crate) fg: Rgb,
  pub(crate) bg: Rgb,
  pub(crate) attributes: u8,
}

/// The grid of a terminal at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Frame {
  pub(crate) cols: usize,
  pub(crate) rows: usize,
  pub(crate) lines: Vec<Vec<Run>>,
}

impl Frame {
  /// The screen that a terminal shows. A wide glyph takes its two cells, and its text holds it
  /// once.
  pub(crate) fn of(terminal: &Terminal) -> Self {
    let (cols, rows) = terminal.size();
    let screen = terminal.screen();
    let lines = (0..rows).map(|row| {
      let cells = screen.line(row).map(Line::cells).unwrap_or_default();
      let mut runs: Vec<Run> = Vec::new();
      for cell in cells.iter().filter(|cell| cell.width() > 0) {
        let (fg, bg, attributes) = look(cell.pen());
        match runs.last_mut() {
          Some(run) if (run.fg, run.bg, run.attributes) == (fg, bg, attributes) => {
            run.text.push(cell.char());
          }
          _ => runs.push(Run { text: cell.char().to_string(), fg, bg, attributes }),
        }
      }
      runs
    });
    Self { cols, rows, lines: lines.collect() }
  }

  /// A frame of rows of cells, each row as wide as the first: a wide glyph stands in its first
  /// cell and leaves the second empty.
  pub(crate) fn of_cells(cells: &[Vec<Cell>]) -> Self {
    let row = |cells: &Vec<Cell>| {
      let mut runs: Vec<Run> = Vec::new();
      for cell in cells.iter().filter(|cell| !cell.glyph.is_empty()) {
        match runs.last_mut() {
          Some(run) if (run.fg, run.bg, run.attributes) == (cell.fg, cell.bg, cell.attributes) => {
            run.text.push_str(&cell.glyph);
          }
          _ => runs.push(Run {
            text: cell.glyph.clone(),
            fg: cell.fg,
            bg: cell.bg,
            attributes: cell.attributes,
          }),
        }
      }
      runs
    };
    let cols = cells.first().map_or(0, Vec::len);
    Self { cols, rows: cells.len(), lines: cells.iter().map(row).collect() }
  }

  /// A frame from its JSON in a file.
  pub(crate) fn read(path: &Path) -> Result<Self, Error> {
    let text =
      fs::read_to_string(path).map_err(|source| Error::Read { path: path.to_owned(), source })?;
    Ok(serde_json::from_str(&text)?)
  }

  /// The text of each row, with no blank at its end, as `tmux capture-pane -p` writes a pane.
  pub(crate) fn text(&self) -> String {
    let row = |runs: &Vec<Run>| runs.iter().map(|run| run.text.as_str()).collect::<String>();
    self.lines.iter().map(|runs| row(runs).trim_end().to_owned() + "\n").collect()
  }

  /// The cells of each row, each as wide as the frame, so a column of a row is an index.
  pub(crate) fn cells(&self) -> Vec<Vec<Cell>> {
    let blank = Cell { glyph: " ".to_owned(), fg: FOREGROUND, bg: BACKGROUND, attributes: 0 };
    let row = |runs: &Vec<Run>| {
      let mut cells: Vec<Cell> = Vec::with_capacity(self.cols);
      for run in runs {
        let look =
          |glyph: String| Cell { glyph, fg: run.fg, bg: run.bg, attributes: run.attributes };
        for glyph in run.text.chars() {
          match glyph.width() {
            Some(0) | None => match cells.last_mut() {
              Some(cell) => cell.glyph.push(glyph),
              None => cells.push(look(glyph.to_string())),
            },
            Some(1) => cells.push(look(glyph.to_string())),
            Some(_) => cells.extend([look(glyph.to_string()), look(String::new())]),
          }
        }
      }
      cells.resize(self.cols, blank.clone());
      cells
    };
    self.lines.iter().map(row).collect()
  }
}

/// The colors and the attributes of a pen, as the cells of a frame hold them: a reversed pen swaps
/// its colors, and a dim one takes its color halfway toward its ground.
fn look(pen: &Pen) -> (Rgb, Rgb, u8) {
  let fg = pen.foreground().map_or(FOREGROUND, rgb);
  let bg = pen.background().map_or(BACKGROUND, rgb);
  let (fg, bg) = if pen.is_inverse() { (bg, fg) } else { (fg, bg) };
  let fg = if pen.is_faint() { fg.half(bg) } else { fg };
  let bits = [
    (pen.is_bold(), BOLD),
    (pen.is_faint(), DIM),
    (pen.is_italic(), ITALIC),
    (pen.is_underline(), UNDERLINE),
    (pen.is_inverse(), REVERSE),
    (pen.is_strikethrough(), STRIKE),
  ];
  (fg, bg, bits.iter().filter(|(on, _)| *on).fold(0, |all, (_, bit)| all | bit))
}

/// A color of the terminal: one of its 16, of the cube of 216 or of the ramp of greys, or one that
/// it was told.
fn rgb(color: Color) -> Rgb {
  match color {
    Color::RGB(told) => Rgb([told.r, told.g, told.b]),
    Color::Indexed(n) if n < 16 => Rgb::hex(PALETTE[usize::from(n)]),
    Color::Indexed(n) if n < 232 => {
      let step = |level: u8| if level == 0 { 0 } else { 55 + level * 40 };
      let n = n - 16;
      Rgb([step(n / 36), step(n / 6 % 6), step(n % 6)])
    }
    Color::Indexed(n) => {
      let grey = 8 + (n - 232) * 10;
      Rgb([grey; 3])
    }
  }
}
