//! A row of text as its runs, and how a row fills its columns.

use opentui::style::{Span, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A row of text, as its runs.
pub(crate) type Line = Vec<Span<'static>>;

/// One run of a row.
pub(crate) fn run(text: impl Into<String>, style: Style) -> Span<'static> {
  Span::styled(text.into(), style)
}

/// A text cut to a count of columns, with a mark that says so.
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
  kept.trim_end().to_owned() + "…"
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
