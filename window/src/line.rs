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
/// when a word stands before the one that it would split.
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
  let kept = match kept.rfind(' ') {
    Some(at) if split => &kept[..at],
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

/// A text as lines of a count of columns at most, broken between words.
pub(crate) fn wrapped(text: &str, room: usize) -> Vec<String> {
  let mut lines: Vec<String> = Vec::new();
  for word in text.split(' ') {
    match lines.last_mut() {
      Some(last) if last.width() + 1 + word.width() <= room => *last = format!("{last} {word}"),
      _ => lines.push(word.to_owned()),
    }
  }
  lines
}

#[cfg(test)]
#[path = "line.test.rs"]
mod test;
