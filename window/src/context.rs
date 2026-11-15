//! The box of the context of the open thread, under the list and in line with the input: how full
//! the window of its chain is at its last answer, a bar in a segment for each part of what that
//! answer read, and its counts, which stand in the row of the hints. A click opens one row for
//! each part, with its size and its share, and folds it again.

use furb::view::Context;
use opentui::style::{Color, Style};

use crate::{
  line::{Line, run, spread},
  model::Model,
  thread::Fold,
  tokens::{LIST, Role},
};

/// The columns of the text of the box, two in from each edge of the side of the list, as the head
/// of the resolved threads stands.
const INNER: usize = LIST as usize - 3;

/// A part of what an answer read, or the room that it left: its name, its tokens and its color.
struct Kind {
  name: &'static str,
  tokens: u64,
  color: Color,
}

impl Model {
  /// The rows of the box of the context: none while no thread is open, else its head, its bar,
  /// each part while it is open, and its counts.
  pub(crate) fn context_rows(&self) -> Vec<Line> {
    let Some(opened) = self.thread.as_ref().filter(|_| self.listed()) else { return Vec::new() };
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let Some(context) = &opened.context else {
      // The box keeps its rows before the first answer, so nothing moves when it comes.
      let free = [Kind { name: "Free", tokens: 1, color: t.border }];
      let head = row(vec![run("Context", prose)], vec![run("No answer yet", muted)]);
      return vec![head, bar(&free, 1), Vec::new()];
    };
    let open = opened.unfolded.contains(&Fold::Context);
    let fold = if open { " ▾" } else { " ▸" };
    let head = vec![run("Context", prose), run(fold, muted)];
    let read = &context.read;
    // The whole of the bar: the window, or what the answer read when no window is known.
    let whole = context.window.unwrap_or(read.tokens).max(1);
    let share = context.window.map(|_| format!("{}%", shared(read.tokens, whole, 100)));
    let mut rows = vec![row(head, vec![run(share.unwrap_or_else(|| count(read.tokens)), prose)])];
    let kinds = self.kinds(context);
    rows.push(bar(&kinds, whole));
    if open {
      rows.extend(kinds.iter().map(|kind| {
        // A part that holds tokens never reads as none of the window.
        let share = match shared(kind.tokens, whole, 100) {
          0 if kind.tokens > 0 => "<1%".to_owned(),
          share => format!("{share:>2}%"),
        };
        let left = vec![run("■ ", Style::new().fg(kind.color)), run(kind.name, muted)];
        row(left, vec![run(format!("{}  {share:>4}", count(kind.tokens)), muted)])
      }));
    }
    let of = context.window.map(|window| format!(" of {}", count(window))).unwrap_or_default();
    let numbers = format!("{}{of} · {} cached", count(read.tokens), count(read.cached));
    rows.push(row(vec![run(numbers, muted)], vec![run(format!("${:.2}", context.dollars), prose)]));
    rows
  }

  /// The ring of the context in the launcher, while the list and its box hide: how full the window
  /// of the chain of the open thread is, or what its last answer read when no window is known.
  pub(crate) fn ring(&self) -> Option<String> {
    let context = self.thread.as_ref().filter(|_| !self.listed())?.context.as_ref()?;
    let read = context.read.tokens;
    let Some(window) = context.window else { return Some(count(read)) };
    let share = shared(read, window.max(1), 100);
    // The glyph of the quarter nearest to the share: the default faces of the terminals hold no
    // ring that is half full.
    let ring = match share {
      0..13 => '○',
      13..50 => '◔',
      50..88 => '◕',
      _ => '●',
    };
    Some(format!("{ring} {share}%"))
  }

  /// The ring taken: the list shows, with each part of the box of the context.
  pub(crate) fn ringed(&mut self) {
    self.toggle();
    if let Some(opened) = self.thread.as_mut() {
      opened.unfolded.insert(Fold::Context);
    }
  }

  /// The parts of what the last answer read, in the order of the bar: what came from the cache,
  /// what it wrote to the cache, what was new, and the room that it left in the window.
  fn kinds(&self, context: &Context) -> Vec<Kind> {
    let t = &self.tokens;
    let [purple, blue, orange] = t.hues;
    let read = &context.read;
    let new = read.tokens.saturating_sub(read.cached + read.written);
    let mut kinds = vec![
      Kind { name: "From the cache", tokens: read.cached, color: blue },
      Kind { name: "Written to the cache", tokens: read.written, color: purple },
      Kind { name: "New", tokens: new, color: orange },
    ];
    if let Some(window) = context.window {
      kinds.push(Kind {
        name: "Free",
        tokens: window.saturating_sub(read.tokens),
        color: t.border,
      });
    }
    kinds
  }
}

/// The bar: one cell for each share of the whole, in the color of the part that it counts. A part
/// that holds tokens takes one cell at least, and the last part takes the cells that the rounding
/// left, so the bar fills its row.
fn bar(kinds: &[Kind], whole: u64) -> Line {
  let mut cells: Vec<usize> = kinds
    .iter()
    .map(|kind| if kind.tokens > 0 { shared(kind.tokens, whole, INNER).max(1) } else { 0 })
    .collect();
  let others: usize = cells.iter().rev().skip(1).sum();
  if let Some(last) = cells.last_mut() {
    *last = INNER.saturating_sub(others);
  }
  let mut line = vec![run(" ", Style::new())];
  line.extend(
    kinds
      .iter()
      .zip(cells)
      .map(|(kind, cells)| run("━".repeat(cells), Style::new().fg(kind.color))),
  );
  line
}

/// A count of tokens as a share of a whole, in a number of steps, rounded to the nearest.
fn shared(tokens: u64, whole: u64, steps: usize) -> usize {
  let steps = u64::try_from(steps).unwrap_or(u64::MAX);
  usize::try_from((tokens.saturating_mul(steps) + whole / 2) / whole.max(1)).unwrap_or(usize::MAX)
}

/// A row of the box: its text at its left and its value at its right.
fn row(left: Line, right: Line) -> Line {
  let mut line = vec![run(" ", Style::new())];
  line.extend(spread(left, right, INNER, Style::new()));
  line
}

/// A count of tokens as a person reads it: `840`, `8.4k`, `124k`, `1.2M`, `1M`, each cut and not
/// rounded up, so a count never shows more than it is.
pub(crate) fn count(tokens: u64) -> String {
  // The decimal shows unless it is 0, as a window of `1M`.
  let decimal = |unit: u64| match tokens / (unit / 10) % 10 {
    0 => (tokens / unit).to_string(),
    tenth => format!("{}.{tenth}", tokens / unit),
  };
  match tokens {
    0..1_000 => tokens.to_string(),
    1_000..10_000 => format!("{}k", decimal(1_000)),
    10_000..1_000_000 => format!("{}k", tokens / 1_000),
    _ => format!("{}M", decimal(1_000_000)),
  }
}

#[cfg(test)]
#[path = "context.test.rs"]
mod test;
