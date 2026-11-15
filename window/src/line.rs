//! A row of text as its runs, and how a row fills its columns.

use opentui::style::{Color, Span, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
  for one in text.chars() {
    used += one.width().unwrap_or_default();
    if used > room {
      break;
    }
    kept.push(one);
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

/// A row of a count of columns: the runs of its left, of which the last gives way, then the runs
/// of its right at its end, two columns after the left at least.
pub(crate) fn spread(mut left: Line, right: Line, room: usize, fill: Style) -> Line {
  let wide = |line: &Line| line.iter().map(|span| span.content.width()).sum::<usize>();
  let taken = match wide(&right) {
    0 => 0,
    wide => wide + 2,
  };
  if let Some(last) = left.pop() {
    let content = cut(&last.content, room.saturating_sub(taken + wide(&left)));
    left.push(run(content, last.style));
  }
  let used = wide(&left) + wide(&right);
  left.push(run(" ".repeat(room.saturating_sub(used)), fill));
  left.extend(right);
  left
}

/// A line in a box of a count of columns on a ground: one column of the ground on each side, and
/// each run on the ground.
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

/// A word as parts of a count of columns at most, each of one character at least.
fn broken(word: &str, room: usize) -> Vec<String> {
  let mut parts = vec![String::new()];
  for one in word.chars() {
    let wide = one.width().unwrap_or_default();
    if parts.last().is_some_and(|last| !last.is_empty() && last.width() + wide > room) {
      parts.push(String::new());
    }
    if let Some(last) = parts.last_mut() {
      last.push(one);
    }
  }
  parts
}

/// The runs of a line of code cut to a count of columns: a cut line ends with `…+` and the count of
/// the characters that it hides, in a muted look.
pub(crate) fn clipped(line: Line, room: usize, muted: Style) -> Line {
  if line.iter().map(|one| one.content.width()).sum::<usize>() <= room {
    return line;
  }
  let chars: usize = line.iter().map(|one| one.content.chars().count()).sum();
  // The mark is as wide as the count of all the characters at most, so the cut leaves it room.
  let mut left = room.saturating_sub(format!("…+{chars}").width());
  let (mut out, mut kept) = (Vec::new(), 0);
  for one in line {
    let fits = |char: &char| {
      let wide = char.width().unwrap_or_default();
      let fits = wide <= left;
      left = if fits { left - wide } else { 0 };
      fits
    };
    let part: String = one.content.chars().take_while(fits).collect();
    let whole = part.len() == one.content.len();
    kept += part.chars().count();
    out.push(run(part, one.style));
    if !whole {
      break;
    }
  }
  out.push(run(format!("…+{}", chars - kept), muted));
  out
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
