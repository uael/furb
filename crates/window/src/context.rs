//! The box of the context of the open thread, under the list and in line with the input: how full
//! the window of its chain is at its last answer, a bar in a segment for each part of what that
//! answer read, and its counts, which stand in the row of the hints. A click on the folded box opens
//! one row for each part, with its size and its share. The open box holds no one button: its head
//! holds the fold and two tabs, the kinds of content that the answer read, by default, and the
//! cache, which says what came from the cache, what it wrote there and what was new, and each row
//! of the kinds is a target of its own, which lights alone under the pointer.

use std::ops::Range;

use furb::view::{Context, Portion, percent, share, tokens};
use opentui::style::{Color, Span, Style};

use furb_client::{Content, Fold, Split};

use crate::cells::Cells;
use crate::{
  line::{Line, run, spread},
  model::Model,
  tokens::Role,
};

/// The names of the tabs of the open box, and the columns between the fold and them.
const KINDS: &str = "Kinds";
const CACHE: &str = "Cache";
const GAP: usize = 3;

/// What a click on a target of the box does: it folds or opens the box, it shows the kinds or the
/// cache, or it takes a part of the kinds, by its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Boxing {
  Fold,
  Kinds(bool),
  Part(usize),
}

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
    // The text of the box stands two columns in from each edge of the list, as the head of the
    // resolved threads stands.
    let inner = usize::from(self.list_width()) - 3;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let Some(context) = &opened.thread.context else {
      // The box keeps its rows before the first answer, so nothing moves when it comes: the
      // counts row says that none came, where the counts of the first answer will stand. Before
      // the first page of a thread that he did not start it says nothing: its chain may have
      // answered, which the page tells.
      let free = [Kind { name: "Free", tokens: 1, color: t.border }];
      let known = opened.thread.above.is_some() || opened.thread.started;
      let said = known.then(|| run("No answer yet", muted));
      let head = row(inner, vec![run("Context", prose)], Vec::new());
      return vec![head, bar(inner, &free, 1), row(inner, said.into_iter().collect(), Vec::new())];
    };
    let open = opened.unfolded.contains(&Fold::Context);
    let fold = if open { " ▾" } else { " ▸" };
    let mut head = vec![run("Context", prose), run(fold, muted)];
    if open {
      let kinds = self.client.split == Split::Kinds;
      let tab =
        |name: &'static str, on: bool| run(name, if on { prose.underline() } else { muted });
      let gap = run(" ".repeat(GAP), Style::new());
      head.extend([gap, tab(KINDS, kinds), run(" · ", muted), tab(CACHE, !kinds)]);
    }
    let read = &context.read;
    let whole = context.whole();
    let used = context.window.map(|_| share(read.tokens, whole));
    let mut rows =
      vec![row(inner, head, vec![run(used.unwrap_or_else(|| tokens(read.tokens)), prose)])];
    let kinds = self.kinds(context);
    rows.push(bar(inner, &kinds, whole));
    if open {
      rows.extend(kinds.iter().map(|kind| {
        let share = share(kind.tokens, whole);
        let left = vec![run("■ ", Style::new().fg(kind.color)), run(kind.name, muted)];
        row(inner, left, vec![run(format!("{}  {share:>4}", tokens(kind.tokens)), muted)])
      }));
    }
    let of = context.window.map(|window| format!(" of {}", tokens(window))).unwrap_or_default();
    let numbers = format!("{}{of} · {} cached", tokens(read.tokens), tokens(read.cached));
    rows.push(row(
      inner,
      vec![run(numbers, muted)],
      vec![run(format!("${:.2}", context.dollars), prose)],
    ));
    rows
  }

  /// The targets of a row of the box, by their columns in the list: the folded box is one target
  /// that opens it; the open box has its fold and its two tabs on its head, and each row of the
  /// kinds. A box that no answer told takes no click.
  pub(crate) fn context_hot(&self, at: usize) -> Vec<(Range<usize>, Boxing)> {
    let Some(opened) = self.client.thread.as_ref() else { return Vec::new() };
    let Some(context) = &opened.thread.context else { return Vec::new() };
    let inner = usize::from(self.list_width()) - 3;
    if !opened.unfolded.contains(&Fold::Context) {
      return vec![(1..1 + inner, Boxing::Fold)];
    }
    match at {
      0 => {
        let fold = 1..1 + "Context ▾".width();
        let kinds = fold.end + GAP..fold.end + GAP + KINDS.width();
        let cache = kinds.end + 3..kinds.end + 3 + CACHE.width();
        vec![(fold, Boxing::Fold), (kinds, Boxing::Kinds(true)), (cache, Boxing::Kinds(false))]
      }
      _ if self.parted() => match at.checked_sub(2) {
        Some(part) if part < self.kinds(context).len() => vec![(1..1 + inner, Boxing::Part(part))],
        _ => Vec::new(),
      },
      _ => Vec::new(),
    }
  }

  /// A row of the box as the list draws it: the folded box lights whole under the pointer, as the
  /// one thing that takes a click, and in the open box only the target under the pointer lights.
  /// The colored cells keep their colors.
  pub(crate) fn context_line(&self, at: usize, (on_box, here): (bool, bool)) -> Line {
    let line = self.context_rows().into_iter().nth(at).unwrap_or_default();
    let hot = self.context_hot(at);
    let open = self.client.thread.as_ref().is_some_and(|one| one.unfolded.contains(&Fold::Context));
    let lit = if open {
      let column = self.pointer.and_then(|(x, _)| self.list_column(x)).filter(|_| here);
      column.and_then(|x| hot.into_iter().find(|one| one.0.contains(&x)).map(|one| one.0))
    } else {
      (on_box && !hot.is_empty()).then_some(0..usize::MAX)
    };
    let Some(lit) = lit else { return line };
    let mut x = 0;
    let light = |one: Span<'static>| {
      let start = x;
      x += one.content.width();
      let cells = one.content.starts_with(['━', '■']);
      if lit.contains(&start) && !cells {
        run(one.content, one.style.fg(self.tokens.hover))
      } else {
        one
      }
    };
    line.into_iter().map(light).collect()
  }

  /// A click on a row of the box, where the pointer stands.
  pub(crate) fn context_clicked(&mut self, at: usize) {
    let Some(column) = self.pointer.and_then(|(x, _)| self.list_column(x)) else { return };
    let Some(does) = self.context_hot(at).into_iter().find(|one| one.0.contains(&column)) else {
      return;
    };
    match does.1 {
      Boxing::Fold => self.fold_context(),
      Boxing::Kinds(kinds) => self.client.split = if kinds { Split::Kinds } else { Split::Cache },
      Boxing::Part(part) => self.client.notice = self.part_said(part),
    }
  }

  /// What the status row says of a part of the kinds that he took: its size and its share, and that
  /// the turns of that part open from it once their view stands.
  fn part_said(&self, part: usize) -> Option<String> {
    let context = self.client.thread.as_ref()?.thread.context.as_ref()?;
    let kind = self.kinds(context).into_iter().nth(part)?;
    let share = share(kind.tokens, context.whole());
    let said = "a click here opens the transcript of this part, once its view stands";
    Some(format!("{} · {} · {share} of the window · {said}", kind.name, tokens(kind.tokens)))
  }

  /// Whether the open box shows the kinds of content, once the client parted the turns.
  fn parted(&self) -> bool {
    let parts = self.client.thread.as_ref().and_then(|one| one.thread.parts.as_ref());
    self.client.split == Split::Kinds && parts.is_some_and(|one| !one.kinds.is_empty())
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
    let [purple, rose, gold] = t.hues;
    let parts = self.client.thread.as_ref().and_then(|one| one.thread.parts.as_ref());
    if let Some(parts) =
      parts.filter(|one| self.client.split == Split::Kinds && !one.kinds.is_empty())
    {
      let kind = |&(content, tokens): &(Content, u64)| {
        let color = match content {
          Content::Turns => t.accent,
          Content::Kits => purple,
          Content::Skills => gold,
          Content::Mcp => t.projects[0].letters,
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
    // The view of the kinds shows kinds alone: until the machine parts the turns, the read stands
    // as one part that says so, and never as the parts of the cache under the tab of the kinds.
    if self.client.split == Split::Kinds {
      let name = if parts.is_some() { "Turns (not parted)" } else { "Reading the parts…" };
      let free = context.window.map(|window| window.saturating_sub(context.read.tokens));
      let read = Kind { name, tokens: context.read.tokens, color: t.muted };
      return [read]
        .into_iter()
        .chain(free.map(|tokens| Kind { name: "Free", tokens, color: t.border }))
        .collect();
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
fn bar(inner: usize, kinds: &[Kind], whole: u64) -> Line {
  let mut cells: Vec<usize> = kinds
    .iter()
    .map(|kind| if kind.tokens > 0 { shared(kind.tokens, whole, inner).max(1) } else { 0 })
    .collect();
  let others: usize = cells.iter().rev().skip(1).sum();
  if let Some(last) = cells.last_mut() {
    *last = inner.saturating_sub(others);
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
fn row(inner: usize, left: Line, right: Line) -> Line {
  let mut line = vec![run(" ", Style::new())];
  line.extend(spread(left, right, inner, Style::new()));
  line
}
