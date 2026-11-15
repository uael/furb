//! The box of the context of the open thread, under the list and in line with the input: how full
//! the window of its chain is at its last answer, a bar in a segment for each part of what that
//! answer read, and its counts, which stand in the row of the hints. A click opens one row for
//! each part, with its size and its share, and folds it again. The parts are the kinds of content
//! that the answer read, or what came from the cache, what it wrote there and what was new: a
//! click on the bar, or the key of the context, shows the other.

use furb::view::{Context, Portion, percent, share, tokens};
use opentui::style::{Color, Style};

use furb_client::{Content, Fold, Split};

use crate::{
  line::{Line, run, spread},
  model::Model,
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
    let Some(opened) = self.client.thread.as_ref().filter(|_| self.listed()) else {
      return Vec::new();
    };
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let Some(context) = &opened.thread.context else {
      // The box keeps its rows before the first answer, so nothing moves when it comes.
      let free = [Kind { name: "Free", tokens: 1, color: t.border }];
      let head = row(vec![run("Context", prose)], vec![run("No answer yet", muted)]);
      return vec![head, bar(&free, 1), Vec::new()];
    };
    let open = opened.unfolded.contains(&Fold::Context);
    let fold = if open { " ▾" } else { " ▸" };
    let head = vec![run("Context", prose), run(fold, muted)];
    let read = &context.read;
    let whole = context.whole();
    let used = context.window.map(|_| share(read.tokens, whole));
    let mut rows = vec![row(head, vec![run(used.unwrap_or_else(|| tokens(read.tokens)), prose)])];
    let kinds = self.kinds(context);
    rows.push(bar(&kinds, whole));
    if open {
      rows.extend(kinds.iter().map(|kind| {
        let share = share(kind.tokens, whole);
        let left = vec![run("■ ", Style::new().fg(kind.color)), run(kind.name, muted)];
        row(left, vec![run(format!("{}  {share:>4}", tokens(kind.tokens)), muted)])
      }));
    }
    let of = context.window.map(|window| format!(" of {}", tokens(window))).unwrap_or_default();
    let numbers = format!("{}{of} · {} cached", tokens(read.tokens), tokens(read.cached));
    rows.push(row(vec![run(numbers, muted)], vec![run(format!("${:.2}", context.dollars), prose)]));
    rows
  }

  /// The ring of the context in the launcher, while the list and its box hide: how full the window
  /// of the chain of the open thread is, or what its last answer read when no window is known.
  pub(crate) fn ring(&self) -> Option<String> {
    let context =
      self.client.thread.as_ref().filter(|_| !self.listed())?.thread.context.as_ref()?;
    let read = context.read.tokens;
    let Some(window) = context.window else { return Some(tokens(read)) };
    let share = percent(read, window);
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
  fn ringed(&mut self) {
    self.toggle();
    if let Some(opened) = self.client.thread.as_mut() {
      opened.unfolded.insert(Fold::Context);
    }
    self.client.split = Split::Kinds;
  }

  /// The key of the context, the command `/context` and the ring step the box: it opens on the
  /// kinds of content, then shows the cache, then folds.
  pub(crate) fn context_step(&mut self) {
    if !self.listed() {
      return self.ringed();
    }
    let open = self.client.thread.as_ref().is_some_and(|one| one.unfolded.contains(&Fold::Context));
    if open && self.client.split == Split::Kinds {
      self.client.split = Split::Cache;
    } else {
      self.fold_context();
    }
  }

  /// The box folded when it was open, and open on the kinds of content when it was folded.
  pub(crate) fn fold_context(&mut self) {
    if let Some(opened) = self.client.thread.as_mut() {
      opened.fold(Fold::Context);
    }
    self.client.split = Split::Kinds;
  }

  /// The parts of what the last answer read, in the order of the bar, each with its name and its
  /// color: its kinds of content once the client parted its turns, else what came from the cache.
  fn kinds(&self, context: &Context) -> Vec<Kind> {
    let t = &self.tokens;
    let [purple, rose, gold, slate] = t.hues;
    let parts = self.client.thread.as_ref().and_then(|one| one.thread.parts.as_ref());
    if let Some(parts) =
      parts.filter(|one| self.client.split == Split::Kinds && !one.kinds.is_empty())
    {
      let kind = |&(content, tokens): &(Content, u64)| {
        let color = match content {
          Content::Turns => t.accent,
          Content::Kits => purple,
          Content::Skills => gold,
          Content::Mcp => slate,
          Content::Memory => rose,
          Content::Doctrine => t.text,
          Content::Engine => t.muted,
        };
        Kind { name: content.name(), tokens, color }
      };
      let mut kinds: Vec<Kind> = parts.kinds.iter().map(kind).collect();
      let free = context.window.map(|window| window.saturating_sub(context.read.tokens));
      kinds.extend(free.map(|tokens| Kind { name: "Free", tokens, color: t.border }));
      return kinds;
    }
    let kind = |(portion, tokens)| {
      let (name, color) = match portion {
        Portion::Cached => ("From the cache", gold),
        Portion::Written => ("Written to the cache", purple),
        Portion::New => ("New", rose),
        Portion::Free => ("Free", t.border),
      };
      Kind { name, tokens, color }
    };
    context.portions().into_iter().map(kind).collect()
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
