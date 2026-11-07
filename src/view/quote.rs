//! The quotes of a word: `<s:name>` at the start of a line, its text, and `</s:name>` at the end of
//! a line, which bind the name to the text as one string. A window reads each quote of a word that
//! ran, and of a word that a model still writes, from here.

use std::fmt::Write as _;

use serde::Serialize;

/// A quote of a word: the line where it opens, its name, its text, the line of its close mark, or
/// the last line of what came of a quote with no close mark yet, whether its close mark came, and
/// the binding that it makes, as python writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Quote {
  pub start: usize,
  pub name: String,
  pub body: String,
  pub end: usize,
  pub closed: bool,
  pub binding: String,
}

/// The name of a quote that opens a line, and what follows its mark on that line.
fn opening(line: &str) -> Option<(&str, &str)> {
  let rest = line.strip_prefix("<s:")?;
  let size = rest.bytes().take_while(|b| b.is_ascii_alphanumeric() || *b == b'_').count();
  let (name, after) = rest.split_at(size);
  (size > 0).then_some(())?;
  Some((name, after.strip_prefix('>')?))
}

/// The quote that opens at a line of a word. A quote with no close mark is none, but in a word that
/// a model still writes, `open`, where it runs to the end of what came, less the part of its close
/// mark that came.
pub fn quote_at(source: &[&str], index: usize, open: bool) -> Option<Quote> {
  let (name, head) = opening(source.get(index)?)?;
  let close = format!("</s:{name}>");
  let found = source.iter().skip(index).position(|one| one.ends_with(&close)).map(|at| at + index);
  if found.is_none() && !open {
    return None;
  }
  let end = found.unwrap_or(source.len() - 1);
  // The word binds the text between the marks, less the line break that follows the opening mark.
  let mut body = std::iter::once(head)
    .chain(source[index + 1..=end].iter().copied())
    .collect::<Vec<_>>()
    .join("\n");
  if body.starts_with('\n') {
    body.remove(0);
  }
  if found.is_some() {
    body.truncate(body.len() - close.len());
  } else if let Some(size) = (2..close.len()).rev().find(|size| body.ends_with(&close[..*size])) {
    body.truncate(body.len() - size);
  }
  let binding = format!("{name} = {}", python_string(&body));
  Some(Quote { start: index, name: name.to_owned(), body, end, closed: found.is_some(), binding })
}

/// Each quote of a text, in order, and none inside another: of a word that ran, each that closes,
/// and of a word that a model still writes, `open`, the last one too while its close mark has not
/// come.
pub fn quotes(text: &str, open: bool) -> Vec<Quote> {
  let source: Vec<&str> = text.split('\n').collect();
  let mut out = Vec::new();
  let mut index = 0;
  while index < source.len() {
    match quote_at(&source, index, open) {
      Some(quote) => {
        index = quote.end + 1;
        out.push(quote);
      }
      None => index += 1,
    }
  }
  out
}

/// What a model writes while its word has not come yet: the lines of the word that it finished,
/// which are python, and the line that it still writes. While it writes no word, the tail is the
/// last line that it thinks. A quote stands as its binding from its opening mark on, and its text
/// grows in the quotes of the string as it comes, the line that it still writes too. `numbers`
/// gives the line of the word where each line of `lines` starts, and `count` the lines of the word
/// that `lines` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Writing {
  pub lines: String,
  pub tail: String,
  pub numbers: Vec<usize>,
  pub count: usize,
}

/// What a model writes, from the text of its word that came and the text that it thinks.
pub fn writing(text: &str, thinking: &str) -> Writing {
  let source: Vec<&str> = text.split('\n').collect();
  let mut rows: Vec<(String, usize, usize)> = Vec::new();
  let mut tail = source.last().copied().unwrap_or_default();
  let mut index = 0;
  while index < source.len() {
    if let Some(quote) = quote_at(&source, index, true) {
      rows.push((quote.binding, index + 1, quote.end + 1));
      if quote.end == source.len() - 1 {
        tail = "";
      }
      index = quote.end;
    } else if index < source.len() - 1 {
      rows.push((source[index].to_owned(), index + 1, index + 1));
    }
    index += 1;
  }
  while rows.last().is_some_and(|row| row.0.is_empty()) {
    rows.pop();
  }
  let lines = rows.iter().map(|row| row.0.as_str()).collect::<Vec<_>>().join("\n");
  let numbers = rows.iter().map(|row| row.1).collect();
  let count = rows.last().map_or(0, |row| row.2);
  if !lines.is_empty() {
    return Writing { lines, tail: tail.trim().to_owned(), numbers, count };
  }
  let said = if text.is_empty() { thinking } else { text };
  let last = said.trim_end_matches('\n').split('\n').next_back().unwrap_or_default();
  Writing { lines, tail: last.trim().to_owned(), numbers, count }
}

/// A text as a string of python, as repr writes it: single marks, or double marks for a text that
/// holds a single mark and no double mark.
pub fn python_string(text: &str) -> String {
  let mark = if text.contains('\'') && !text.contains('"') { '"' } else { '\'' };
  let mut out = String::with_capacity(text.len() + 2);
  out.push(mark);
  for one in text.chars() {
    match one {
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      one if one == mark => {
        out.push('\\');
        out.push(one);
      }
      one if one < ' ' || one == '\x7f' => {
        write!(out, "\\x{:02x}", u32::from(one)).ok();
      }
      one => out.push(one),
    }
  }
  out.push(mark);
  out
}

#[cfg(test)]
#[path = "quote.test.rs"]
mod tests;
