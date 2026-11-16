//! A row of text as its runs, and how a row fills its columns.
#![expect(clippy::string_slice, reason = "debt: a slice of a string that can cut a character")]

use std::{mem, ops::Range};

use crate::{
  cells::{Cells, characters},
  thread::columns,
};
use opentui::style::{Color, Span, Style};

/// A row of text, as its runs.
pub(crate) type Line = Vec<Span<'static>>;

/// One run of a row.
pub(crate) fn run(text: impl Into<String>, style: Style) -> Span<'static> {
  Span::styled(text.into(), style)
}

/// A text cut to a count of columns, with a mark that says so. The cut falls between two words
/// when a word stands before the one that it would split: the blank that pads the start of a text
/// stands before no word.
pub(crate) fn cut(text: &str, room: usize) -> String {
  if text.width() <= room {
    return text.to_owned();
  }
  let (mut kept, mut used) = (String::new(), 1);
  for (one, wide) in characters(text) {
    used += wide;
    if used > room {
      break;
    }
    kept.push_str(one);
  }
  let split = !text[kept.len()..].starts_with(' ');
  let pad = kept.len() - kept.trim_start().len();
  let kept = match kept[pad..].rfind(' ') {
    Some(at) if split => &kept[..pad + at],
    _ => kept.as_str(),
  };
  // A separator that the cut leaves at the end goes with the word after it.
  kept.trim_end_matches([' ', '·']).to_owned() + "…"
}

/// A row of a count of columns: the runs of its left, of which the one that crosses its room gives
/// way and those after it go, then the runs of its right at its end, two columns after the left at
/// least.
pub(crate) fn spread(left: Line, right: Line, room: usize, fill: Style) -> Line {
  let wide = |line: &Line| line.iter().map(|span| span.content.width()).sum::<usize>();
  let taken = match wide(&right) {
    0 => 0,
    wide => wide + 2,
  };
  let free = room.saturating_sub(taken);
  let mut left: Line = left
    .into_iter()
    .scan(0, |used, one| {
      let content =
        (*used < free || one.content.is_empty()).then(|| cut(&one.content, free - *used))?;
      let whole = content == one.content;
      // The run that gave way ends the left.
      *used = if whole { *used + content.width() } else { free };
      Some(run(content, one.style))
    })
    .collect();
  let used = wide(&left) + wide(&right);
  left.push(run(" ".repeat(room.saturating_sub(used)), fill));
  left.extend(right);
  left
}

/// A line in a box of a count of columns on a ground: one column of the ground on each side, and
/// each run on the ground.
/// The columns before the text of a box that a bar in a hue marks, a message of the operator or
/// the input: the margin of the feed, the bar, and one blank column of the ground of the box.
pub(crate) const BARRED: usize = 4;

/// The left of a row of a box that a bar in a hue marks: the margin of the feed, so the bar stands
/// in the column where each other box of the feed starts, the bar, and one blank column.
pub(crate) fn barred(hue: Color, ground: Color) -> Line {
  let ground = Style::new().bg(ground);
  vec![run("  ", Style::new()), run("┃", ground.fg(hue)), run(" ", ground)]
}

pub(crate) fn boxed(line: Line, room: usize, ground: Color) -> Line {
  let fill = Style::new().bg(ground);
  let inside = line.into_iter().map(|one| run(one.content, one.style.bg(ground))).collect();
  let mut boxed = vec![run(" ", fill)];
  boxed.extend(spread(inside, Vec::new(), room.saturating_sub(2), fill));
  boxed.push(run(" ", fill));
  boxed
}

/// A text as lines of a count of columns at most, broken between words. A word wider than a line,
/// as a path, breaks where the line ends.
pub(crate) fn wrapped(text: &str, room: usize) -> Vec<String> {
  let mut lines: Vec<String> = Vec::new();
  for word in text.split(' ') {
    match lines.last_mut() {
      Some(last) if last.width() + 1 + word.width() <= room => *last = format!("{last} {word}"),
      _ => lines.push(word.to_owned()),
    }
    if let Some(last) = lines.pop_if(|last| last.width() > room) {
      lines.extend(broken(&last, room));
    }
  }
  lines
}

/// A text wrapped to a count of columns at most, on as few rows as [`wrapped`] gives and each as
/// near the others as it can, so no word stands alone on its last row.
pub(crate) fn balanced(text: &str, room: usize) -> Vec<String> {
  let rows = wrapped(text, room).len();
  let least = (1..=room).rev().take_while(|one| wrapped(text, *one).len() == rows).last();
  wrapped(text, least.unwrap_or(room))
}

/// A word as parts of a count of columns at most, each of one character at least.
fn broken(word: &str, room: usize) -> Vec<String> {
  let mut parts = vec![String::new()];
  for (one, wide) in characters(word) {
    if parts.last().is_some_and(|last| !last.is_empty() && last.width() + wide > room) {
      parts.push(String::new());
    }
    if let Some(last) = parts.last_mut() {
      last.push_str(one);
    }
  }
  parts
}

/// The runs of a line of code cut to a count of columns: a cut line ends with `…+` and the count of
/// the characters that it hides, in a muted look, at the last column where the count leaves it.
pub(crate) fn clipped(line: Line, room: usize, muted: Style) -> Line {
  if line.iter().map(|one| one.content.width()).sum::<usize>() <= room {
    return line;
  }
  let chars: usize = line.iter().map(|one| one.content.chars().count()).sum();
  // The mark takes the room of the count of all the characters first, then of the count that it
  // hides, which has as many digits or fewer, so the cut ends at the column of the room.
  let mut mark = format!("…+{chars}").width();
  loop {
    let (out, kept) = kept(&line, room.saturating_sub(mark));
    let hidden = format!("…+{}", chars - kept);
    if hidden.width() == mark {
      return out.into_iter().chain([run(hidden, muted)]).collect();
    }
    mark = hidden.width();
  }
}

/// The runs of a line that fit a count of columns, and how many characters they keep. A cut never
/// ends on blanks, so the mark after it stands right after what it kept.
fn kept(line: &Line, room: usize) -> (Line, usize) {
  let (mut out, mut kept, mut left) = (Vec::new(), 0, room);
  for one in line {
    let fits = |(_, wide): &(&str, usize)| {
      let fits = *wide <= left;
      left = if fits { left - wide } else { 0 };
      fits
    };
    let part: String = characters(&one.content).take_while(fits).map(|(one, _)| one).collect();
    let whole = part.len() == one.content.len();
    kept += part.chars().count();
    out.push(run(part, one.style));
    if !whole {
      break;
    }
  }
  if let Some(last) = out.pop() {
    let trimmed = last.content.trim_end();
    kept -= last.content.chars().count() - trimmed.chars().count();
    out.push(run(trimmed.to_owned(), last.style));
  }
  (out, kept)
}

/// A line whose columns of a range take a style of their own, as a link under the pointer: the
/// runs that the range cuts split at its edges.
pub(crate) fn styled(line: Line, range: &Range<usize>, style: impl Fn(Style) -> Style) -> Line {
  let mut out: Line = Vec::new();
  let mut column = 0;
  for one in line {
    let mut part = (String::new(), None);
    for (character, wide) in characters(&one.content) {
      let inside = range.contains(&column);
      if part.1.is_some_and(|was| was != inside) {
        let (text, was) = mem::take(&mut part);
        out.push(run(text, if was == Some(true) { style(one.style) } else { one.style }));
      }
      part.0.push_str(character);
      part.1 = Some(inside);
      column += wide;
    }
    if let (text, Some(inside)) = part {
      out.push(run(text, if inside { style(one.style) } else { one.style }));
    }
  }
  out
}

/// The column of the mark that says how many characters a cut line lost, as `clipped` writes it.
pub(crate) fn marked(line: &Line) -> Option<Range<usize>> {
  let mut marks = columns(line).into_iter().zip(line);
  marks.rfind(|(_, one)| one.content.starts_with("…+")).map(|(at, _)| at)
}

/// A line as rows of a count of columns at most, each run in its style. A row breaks after the
/// last blank that it holds, else where it is full, as a text wraps.
pub(crate) fn sliced(line: &[Span<'static>], room: usize) -> Vec<Line> {
  let cells = line
    .iter()
    .flat_map(|one| characters(&one.content).map(|(character, wide)| (character, wide, one.style)));
  let wide = |row: &[(&str, usize, Style)]| -> usize { row.iter().map(|one| one.1).sum() };
  let (mut rows, mut row) = (Vec::new(), Vec::new());
  for cell in cells {
    if wide(&row) + cell.1 > room && !row.is_empty() {
      let blank = row.iter().rposition(|one| one.0 == " ").filter(|at| *at + 1 < row.len());
      let rest = blank.map_or_else(Vec::new, |at| row.split_off(at + 1));
      rows.push(mem::replace(&mut row, rest));
    }
    row.push(cell);
  }
  rows.push(row);
  let runs = |row: Vec<(&str, usize, Style)>| {
    let mut line: Line = Vec::new();
    for (character, _, style) in row {
      match line.last_mut() {
        Some(last) if last.style == style => last.content.to_mut().push_str(character),
        _ => line.push(run(character, style)),
      }
    }
    line
  };
  rows.into_iter().map(runs).collect()
}

/// A text with its first letter in upper case.
pub(crate) fn capital(text: &str) -> String {
  let mut chars = text.chars();
  chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// A row of a list between its two edges. The chosen row stands on a ground with a bar at its left,
/// in the right half of a column: the ground takes one column on each side of its text and the
/// left half of the column after it, so the row stands as far from each side of the list, and its
/// text as far from each side of the row. A row on no ground stands two columns in.
pub(crate) fn edged(bar: Color, ground: Option<Color>, line: Line) -> Line {
  let Some(ground) = ground else {
    let mut row = vec![run("  ", Style::new())];
    row.extend(line);
    return row;
  };
  let edge = || run(" ", Style::new().bg(ground));
  let mut row = vec![run("▐", Style::new().fg(bar)), edge()];
  row.extend(line);
  row.extend([edge(), run("▌", Style::new().fg(ground))]);
  row
}
