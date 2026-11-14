//! The probes of a frame of the window, which need no judge. Each finding names its row and its
//! column, counted from 0 as the cells of the JSON of a frame count them, and the rule of the whole
//! that it breaks: one set of colors and one scale of spacing, in the tokens of the window alone;
//! each box as wide inside on its left as on its right, and as high above as below; each slot of a
//! repeated row in one column; and each box the same at each size.

use std::{array, collections::BTreeMap};

use furb_window::tokens::{HEAD, SYNTAX, TEXT, Tokens};

use crate::frame::{Cell, Frame, Rgb};

/// A finding of a probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
  pub(crate) row: usize,
  pub(crate) col: usize,
  pub(crate) says: String,
}

/// The insets of a box that the scale of spacing names: none, the column of a head, and the column
/// of a text under it.
fn scale() -> [usize; 3] {
  [0, usize::from(HEAD), usize::from(TEXT)]
}

/// An inset this small or smaller is a padding: on the right, it holds a part that the box aligns
/// on its right, so it answers the inset on its left. A larger one is free room.
const PAD: usize = 3;

/// The glyphs of a bar or a border, which stand on the edge of a box and are not its content.
const BARS: &str = "┃│▌▐▍▎▏█▕";

/// The look of a cell: its color, its ground and its attributes.
type Look = (Rgb, Rgb, u8);

/// A piece of a row: its look, and its first and its last column.
type Piece = (Look, usize, usize);

/// The rows of one repeated part: each row, and the first and the last column of each of its
/// slots.
type Rows = Vec<(usize, Vec<(usize, usize)>)>;

/// The insets of a part at each size, by its ground and its rank.
type Ranks<'a> = BTreeMap<(Rgb, usize), Vec<(&'a str, [usize; 4])>>;

/// The findings of a frame: its colors, its boxes and its repeated rows.
pub(crate) fn probe(frame: &Frame) -> Vec<Finding> {
  let cells = frame.cells();
  let tokens = Tokens::on(None);
  let mut findings = colors(&cells, &palette(&tokens));
  for found in boxes(&cells, base(&tokens)) {
    findings.extend(found.findings());
  }
  findings.extend(slots(&cells));
  findings
}

/// The ground of the window.
fn base(tokens: &Tokens) -> Rgb {
  Rgb([tokens.base.r, tokens.base.g, tokens.base.b])
}

/// Each color that the tokens give, and each of them under the veil of a list part.
fn palette(tokens: &Tokens) -> Vec<Rgb> {
  let named = [
    tokens.base,
    tokens.panel,
    tokens.raised,
    tokens.lifted,
    tokens.border,
    tokens.text,
    tokens.hover,
    tokens.muted,
    tokens.accent,
    tokens.attention,
    tokens.danger,
  ];
  let named = named.into_iter().chain(tokens.hues).map(|color| Rgb([color.r, color.g, color.b]));
  let mut colors: Vec<Rgb> = named.chain(SYNTAX.map(|(_, value)| Rgb::hex(value))).collect();
  let (veil, alpha) = ([tokens.veil.r, tokens.veil.g, tokens.veil.b], u16::from(tokens.veil.a));
  let veiled = colors.iter().map(|color| {
    Rgb(array::from_fn(|at| {
      let mixed = (u16::from(color.0[at]) * (255 - alpha) + u16::from(veil[at]) * alpha) / 255;
      u8::try_from(mixed).unwrap_or(u8::MAX)
    }))
  });
  colors.extend(veiled.collect::<Vec<_>>());
  colors
}

/// Each color of a glyph or of a ground that no token gives, once, at its first cell, with how
/// many cells take it. A color one step from a token is that token, rounded by a blend.
fn colors(cells: &[Vec<Cell>], palette: &[Rgb]) -> Vec<Finding> {
  let near = |color: Rgb| {
    palette.iter().any(|token| token.0.iter().zip(color.0).all(|(a, b)| a.abs_diff(b) <= 1))
  };
  let mut seen: BTreeMap<(Rgb, &str), (usize, usize, usize)> = BTreeMap::new();
  for (row, line) in cells.iter().enumerate() {
    for (col, cell) in line.iter().enumerate() {
      let glyph = (!cell.glyph.trim().is_empty()).then_some(cell.fg);
      for (color, role) in [(glyph, "glyph"), (Some(cell.bg), "ground")] {
        let Some(color) = color.filter(|color| !near(*color)) else { continue };
        seen.entry((color, role)).and_modify(|found| found.2 += 1).or_insert((row, col, 1));
      }
    }
  }
  let finding =
    |((color, role), (row, col, count)): ((Rgb, &str), (usize, usize, usize))| Finding {
      row,
      col,
      says: format!("the color {} of a {role} is no token ({count} cells)", String::from(color)),
    };
  seen.into_iter().map(finding).collect()
}

/// A box: a rectangle of one ground other than the base, and the insets of its content, which is
/// each glyph and each cell of another ground in it. A bar on its edge is its border: the inset
/// holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Boxed {
  pub(crate) ground: Rgb,
  pub(crate) top: usize,
  pub(crate) left: usize,
  pub(crate) bottom: usize,
  pub(crate) right: usize,
  /// The insets of its content: left, right, above and below. None for an empty box.
  pub(crate) insets: Option<[usize; 4]>,
}

impl Boxed {
  fn findings(&self) -> Vec<Finding> {
    let Some([left, right, above, below]) = self.insets else { return Vec::new() };
    let at = |says: String| Finding { row: self.top, col: self.left, says };
    let name = format!(
      "the box {} of rows {} to {} and columns {} to {}",
      String::from(self.ground),
      self.top,
      self.bottom,
      self.left,
      self.right
    );
    let mut findings = Vec::new();
    if right <= PAD && left != right {
      findings.push(at(format!(
        "{name} is {left} cells wide inside on its left and {right} on its right"
      )));
    }
    if self.bottom > self.top + 1 && above != below && above <= PAD && below <= PAD {
      findings.push(at(format!("{name} is {above} rows high inside above and {below} below")));
    }
    let outside = [left, above, below].into_iter().chain((right <= PAD).then_some(right));
    let scale = scale();
    for inset in outside.filter(|inset| *inset <= PAD && !scale.contains(inset)) {
      findings.push(at(format!(
        "{name} has an inset of {inset}, which the scale {scale:?} does not name"
      )));
    }
    findings
  }
}

/// The boxes of a frame, each as wide and as high as its ground: a ground that does not fill the
/// rectangle that holds it, or that is one or two cells wide, is no box.
pub(crate) fn boxes(cells: &[Vec<Cell>], base: Rgb) -> Vec<Boxed> {
  let (rows, cols) = (cells.len(), cells.first().map_or(0, Vec::len));
  let mut seen = vec![vec![false; cols]; rows];
  let mut found = Vec::new();
  for row in 0..rows {
    for col in 0..cols {
      if seen[row][col] || cells[row][col].bg == base {
        continue;
      }
      let ground = cells[row][col].bg;
      let (mut stack, mut area) = (vec![(row, col)], Vec::new());
      seen[row][col] = true;
      while let Some((y, x)) = stack.pop() {
        area.push((y, x));
        let next = [(y + 1, x), (y.wrapping_sub(1), x), (y, x + 1), (y, x.wrapping_sub(1))];
        for (y, x) in next {
          if y < rows && x < cols && !seen[y][x] && cells[y][x].bg == ground {
            seen[y][x] = true;
            stack.push((y, x));
          }
        }
      }
      let (top, bottom) = bounds(area.iter().map(|at| at.0));
      let (left, right) = bounds(area.iter().map(|at| at.1));
      if right - left >= 2 && area.len() * 10 >= (bottom - top + 1) * (right - left + 1) * 9 {
        let mut one = Boxed { ground, top, left, bottom, right, insets: None };
        one.insets = insets(cells, &one);
        found.push(one);
      }
    }
  }
  found
}

fn bounds(values: impl Iterator<Item = usize> + Clone) -> (usize, usize) {
  (values.clone().min().unwrap_or_default(), values.max().unwrap_or_default())
}

fn insets(cells: &[Vec<Cell>], one: &Boxed) -> Option<[usize; 4]> {
  let edge = |col: usize, cell: &Cell| {
    (col == one.left || col == one.right) && BARS.contains(cell.glyph.as_str())
  };
  let content = (one.top..=one.bottom).flat_map(|row| {
    (one.left..=one.right).filter_map(move |col| {
      let cell = &cells[row][col];
      let shows = !cell.glyph.trim().is_empty() || cell.bg != one.ground;
      (shows && !edge(col, cell)).then_some((row, col))
    })
  });
  let content: Vec<(usize, usize)> = content.collect();
  let (first, last) = bounds(content.iter().map(|at| at.0));
  let (start, end) = bounds(content.iter().map(|at| at.1));
  (!content.is_empty())
    .then(|| [start - one.left, one.right - end, first - one.top, one.bottom - last])
}

/// The rows that repeat one part: rows of the same looks in the same order, in the same box or on
/// the base. Each slot of such rows starts in one column or ends in one column, else it is a
/// finding. Two slots split where the look changes, or at a blank that a glyph of another look
/// follows. A bar is the border of a part, and no slot.
fn slots(cells: &[Vec<Cell>]) -> Vec<Finding> {
  let mut parts: BTreeMap<Vec<Look>, Rows> = BTreeMap::new();
  for (row, line) in cells.iter().enumerate() {
    let pieces = pieces(line);
    if pieces.len() >= 2 {
      let looks = pieces.iter().map(|piece| piece.0).collect();
      parts
        .entry(looks)
        .or_default()
        .push((row, pieces.into_iter().map(|piece| (piece.1, piece.2)).collect()));
    }
  }
  let mut findings = Vec::new();
  for rows in parts.values().filter(|rows| rows.len() >= 2) {
    for slot in 0..rows[0].1.len() {
      let starts: Vec<usize> = rows.iter().map(|(_, pieces)| pieces[slot].0).collect();
      let ends: Vec<usize> = rows.iter().map(|(_, pieces)| pieces[slot].1).collect();
      if starts.iter().all(|start| *start == starts[0]) || ends.iter().all(|end| *end == ends[0]) {
        continue;
      }
      let places: Vec<String> = rows
        .iter()
        .map(|(row, pieces)| format!("row {row} {}-{}", pieces[slot].0, pieces[slot].1))
        .collect();
      let says = format!(
        "the slot {} of a repeated row neither starts nor ends in one column: {}",
        slot + 1,
        places.join(", ")
      );
      findings.push(Finding { row: rows[0].0, col: starts[0], says });
    }
  }
  findings
}

/// The pieces of a row: the look of each, and its first and its last column.
fn pieces(line: &[Cell]) -> Vec<Piece> {
  let look = |cell: &Cell| (cell.fg, cell.bg, cell.attributes);
  let shows = |cell: &Cell| {
    (!cell.glyph.trim().is_empty() && !BARS.contains(cell.glyph.as_str())) || cell.glyph.is_empty()
  };
  let mut pieces: Vec<Piece> = Vec::new();
  let mut open = false;
  for (col, cell) in line.iter().enumerate() {
    if !shows(cell) {
      let next = line.get(col + 1).filter(|next| shows(next));
      open =
        open && next.is_some_and(|next| pieces.last().is_some_and(|piece| piece.0 == look(next)));
      continue;
    }
    match pieces.last_mut() {
      Some(piece) if open && piece.0 == look(cell) => piece.2 = col,
      _ => pieces.push((look(cell), col, col)),
    }
    open = true;
  }
  pieces
}

/// The parts of the same step at each size whose insets differ. A part is a box of two rows or more
/// and ten columns or more, and a part at one size is the part of the same ground and the same rank,
/// from the top, at another. A side counts only where it is a padding at both sizes.
pub(crate) fn sizes(frames: &[(String, Frame)]) -> Vec<String> {
  let base = base(&Tokens::on(None));
  let mut ranks: Ranks = BTreeMap::new();
  for (size, frame) in frames {
    let parts = boxes(&frame.cells(), base)
      .into_iter()
      .filter(|one| one.bottom > one.top && one.right - one.left >= 9);
    let mut seen: BTreeMap<Rgb, usize> = BTreeMap::new();
    for one in parts {
      let rank = seen.entry(one.ground).and_modify(|rank| *rank += 1).or_insert(0);
      if let Some(insets) = one.insets {
        ranks.entry((one.ground, *rank)).or_default().push((size, insets));
      }
    }
  }
  let same = |one: [usize; 4], other: [usize; 4]| {
    one.iter().zip(other).all(|(a, b)| *a == b || *a > PAD || b > PAD)
  };
  let differ = ranks.into_iter().filter(|(_, seen)| seen.iter().any(|one| !same(one.1, seen[0].1)));
  differ
    .map(|((ground, rank), seen)| {
      let each: Vec<String> = seen
        .iter()
        .map(|(size, [left, right, above, below])| {
          format!("{size} left {left} right {right} above {above} below {below}")
        })
        .collect();
      format!(
        "the part {} number {} from the top has other insets at another size: {}",
        String::from(ground),
        rank + 1,
        each.join(", ")
      )
    })
    .collect()
}

#[cfg(test)]
#[path = "probe.test.rs"]
mod test;
