//! The views of the kits of the life of the open thread as the window draws them: each tree that
//! a skill serves, under the dock of the agents and over the input, in the closed set of nodes of
//! the protocol. A button takes a press, and an input takes the input of the window, whose next
//! send goes to that node.

use crate::cells::Cells;
use furb_client::View;
use furb_host::protocol::{Axis, Node, Run, Shape, Tone};
use opentui::style::{Span, Style};

use crate::{
  keys::Does,
  line::{Line, run},
  model::Model,
  thread::{Click, Row},
  tokens::Role,
};

/// The cells of the bar of a progress.
const BAR: u32 = 20;

/// The fewest cells of the buttons and the inputs of a column of a view.
const FIELD: usize = 40;

/// What Enter does in an input of a view: it gives its text to the view.
const SUBMIT: &str = "send";

impl Model {
  /// The rows of the views of the life of the target of the input: each tree after a blank row of
  /// its own, and a blank row under the last, so no view touches the input; none while no view
  /// shows.
  pub(crate) fn kit_rows(&self) -> Vec<Row> {
    // A view stands at the left edge, as the bands of the threads and the input do.
    let room = self.wide().saturating_sub(2);
    let mut rows = Vec::new();
    for view in self.client.shown_views() {
      let Some(tree) = &view.tree else { continue };
      rows.push(Row::default());
      rows.extend(self.node_rows(&view, tree, room));
    }
    // While the keys reach a control, the row under the views says which keys move and use it,
    // in the column of the text of the views.
    if self.client.aimed_control().is_some() {
      let keys = [Does::Up, Does::Down].map(|one| self.key(one)).concat();
      let (send, close) = (self.key(Does::Send), self.key(Does::Close));
      let said = format!("  {keys} move · {send} use · {close} back to the input");
      rows.push(Row::of(vec![run(said, self.tokens.style(Role::Muted))]));
    }
    if !rows.is_empty() {
      rows.push(Row::default());
    }
    rows
  }

  /// Whether the keys reach a node of a view.
  fn reached(&self, view: &View, node: &Node) -> bool {
    let aimed = self.client.aimed_control();
    aimed.is_some_and(|one| one.viewer == view.viewer && one.node == node.id)
  }

  /// The keys turned to the controls of the views, or back to the input, and said when no view
  /// shows.
  pub(crate) fn reach_views(&mut self) {
    if !self.client.aim_controls() && self.client.controls().is_empty() {
      self.client.notice = Some("No view shows here.".to_owned());
    }
  }

  /// The rows of one node of a view in a count of columns.
  fn node_rows(&self, view: &View, node: &Node, room: usize) -> Vec<Row> {
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    match &node.shape {
      Shape::Box { axis, gap, border, children } => {
        let inner = if *border { room.saturating_sub(2) } else { room };
        let rows = match axis {
          Axis::Column => under(self.column(view, children, inner), usize::from(*gap)),
          Axis::Row => {
            let parts: Vec<Vec<Row>> =
              children.iter().map(|child| self.node_rows(view, child, inner)).collect();
            beside(&parts, usize::from(*gap))
          }
        };
        if *border { self.bordered(rows) } else { rows }
      }
      Shape::Text { runs } => vec![Row::of(runs.iter().map(|one| self.toned(one)).collect())],
      Shape::List { items } => items
        .iter()
        .flat_map(|item| {
          let rows = self.node_rows(view, item, room.saturating_sub(2));
          rows.into_iter().enumerate().map(|(at, row)| {
            let mark = if at == 0 { "• " } else { "  " };
            row.shifted(2).with_head(vec![run(mark, muted)])
          })
        })
        .collect(),
      Shape::Table { header, rows } => self.table(view, (header, rows), room),
      Shape::Input { value, hint, inert } => {
        vec![self.view_input((view, node), (value, hint, *inert), 0)]
      }
      Shape::Button { label, inert } => vec![self.button((view, node), (label, *inert), 0)],
      Shape::Progress { done } => vec![Row::of(self.progress(*done))],
      Shape::Code { text, .. } | Shape::Markdown { text } => {
        text.lines().map(|line| Row::of(vec![run(line.to_owned(), prose)])).collect()
      }
      Shape::Diff { text } => text.lines().map(|line| Row::of(vec![self.diffed(line)])).collect(),
    }
  }

  /// The rows of the children of a column, whose buttons and inputs take one width: the widest
  /// row of the column, at least [`FIELD`] cells, within its room, as the rows of a list do.
  fn column(&self, view: &View, children: &[Node], room: usize) -> Vec<Vec<Row>> {
    let parts: Vec<Vec<Row>> =
      children.iter().map(|child| self.node_rows(view, child, room)).collect();
    let widest = parts.iter().flatten().map(Row::width).max().unwrap_or_default();
    let wide = widest.max(FIELD).min(room);
    let one = |(child, part): (&Node, Vec<Row>)| match &child.shape {
      Shape::Button { label, inert } => vec![self.button((view, child), (label, *inert), wide)],
      Shape::Input { value, hint, inert } => {
        vec![self.view_input((view, child), (value, hint, *inert), wide)]
      }
      _ => part,
    };
    children.iter().zip(parts).map(one).collect()
  }

  /// The row of a button of a view, as wide as its label or as a width that it fills. A click
  /// presses it.
  fn button(&self, (view, node): (&View, &Node), (label, inert): (&str, bool), wide: usize) -> Row {
    let t = &self.tokens;
    let style = match (inert, self.reached(view, node)) {
      (true, _) => t.style(Role::Muted),
      (false, true) => t.style(Role::Button).fg(t.accent).bold(),
      (false, false) => t.style(Role::Button),
    };
    let text = format!(" {label} ");
    let pad = wide.saturating_sub(text.width());
    let line = vec![run(format!("{text}{}", " ".repeat(pad)), style)];
    let press = Click::Press { viewer: view.viewer.clone(), node: node.id.clone() };
    if inert { Row::of(line) } else { Row::clicked(line, press) }
  }

  /// The row of an input of a view: its value, or its hint while it holds none, then what Enter
  /// does there, on the ground of a field as wide as its text or as a width that it fills, in the
  /// accent while the input of the window fills it. A click there turns the input of the window
  /// to it.
  fn view_input(
    &self,
    (view, node): (&View, &Node),
    (value, hint, inert): (&str, &Option<String>, bool),
    wide: usize,
  ) -> Row {
    let t = &self.tokens;
    let filled = self.client.filled.as_ref();
    let filled = filled.is_some_and(|one| one.viewer == view.viewer && one.node == node.id)
      || self.reached(view, node);
    let ground = Style::new().bg(t.panel);
    let (text, style) = match hint {
      Some(hint) if value.is_empty() => (hint.clone(), t.style(Role::Muted)),
      _ => (value.to_owned(), t.style(Role::Prose)),
    };
    let enter = if inert { String::new() } else { format!("{} {SUBMIT}", self.key(Does::Send)) };
    let pad = wide.saturating_sub(text.width() + enter.width() + 3);
    let edge = Style::new().fg(if filled { t.accent } else { t.border }).bg(t.panel);
    let line = vec![
      run("▎", edge),
      run(format!("{text}{}", " ".repeat(pad + 1)), style.bg(t.panel)),
      run(enter, t.style(Role::Muted).bg(t.panel)),
      run(" ", ground),
    ];
    let (viewer, node, value) = (view.viewer.clone(), node.id.clone(), value.to_owned());
    if inert { Row::of(line) } else { Row::clicked(line, Click::Fill { viewer, node, value }) }
  }

  /// The bar of a progress: the share done in the accent, the rest faint, and the percent; or the
  /// whole bar faint while the share is not known.
  fn progress(&self, done: Option<f64>) -> Line {
    let t = &self.tokens;
    let muted = t.style(Role::Muted);
    let all = usize::try_from(BAR).unwrap_or_default();
    let Some(done) = done else {
      return vec![run("━".repeat(all), Style::new().fg(t.border)), run(" …", muted)];
    };
    let cells = (0..BAR).filter(|cell| f64::from(*cell) + 0.5 <= done * f64::from(BAR)).count();
    let rest = all.saturating_sub(cells);
    vec![
      run("━".repeat(cells), Style::new().fg(t.accent)),
      run("━".repeat(rest), Style::new().fg(t.border)),
      run(format!(" {:.0}%", done.clamp(0.0, 1.0) * 100.0), muted),
    ]
  }

  /// A line of a diff of a view, in the color of what it does.
  fn diffed(&self, line: &str) -> Span<'static> {
    let t = &self.tokens;
    let color = match line.as_bytes().first() {
      Some(b'+') => t.added,
      Some(b'-') => t.danger,
      _ => t.muted,
    };
    run(line.to_owned(), Style::new().fg(color))
  }

  /// A run of a text of a view in the color of its tone.
  fn toned(&self, one: &Run) -> Span<'static> {
    let t = &self.tokens;
    let color = match one.tone {
      Tone::Text => t.text,
      Tone::Muted => t.muted,
      Tone::Faint => t.border,
      Tone::Accent => t.accent,
      Tone::Success => t.added,
      Tone::Warning => t.attention,
      Tone::Danger => t.danger,
    };
    run(one.text.clone(), Style::new().fg(color))
  }

  /// The rows of a box with a border: a bar at the left of each of its rows.
  fn bordered(&self, rows: Vec<Row>) -> Vec<Row> {
    let edge = Style::new().fg(self.tokens.border);
    rows.into_iter().map(|row| row.shifted(2).with_head(vec![run("│ ", edge)])).collect()
  }

  /// The rows of a table: its header, muted, then each row with its cells in columns as wide as
  /// the widest cell of each, each cell its first row.
  fn table(&self, view: &View, (header, rows): (&[String], &[Vec<Node>]), room: usize) -> Vec<Row> {
    let muted = self.tokens.style(Role::Muted);
    let cells: Vec<Vec<Row>> = rows
      .iter()
      .map(|cells| {
        let first = |cell: &Node| self.node_rows(view, cell, room).into_iter().next();
        cells.iter().map(|cell| first(cell).unwrap_or_default()).collect()
      })
      .collect();
    let head: Vec<Row> = header.iter().map(|one| Row::of(vec![run(one.clone(), muted)])).collect();
    let all = [head].into_iter().chain(cells).filter(|row| !row.is_empty());
    let all: Vec<Vec<Row>> = all.collect();
    let columns = all.iter().map(Vec::len).max().unwrap_or_default();
    let widths: Vec<usize> = (0..columns)
      .map(|at| all.iter().filter_map(|row| row.get(at)).map(Row::width).max().unwrap_or_default())
      .collect();
    all
      .into_iter()
      .map(|cells| {
        let (mut row, mut column) = (Row::default(), 0);
        for (at, cell) in cells.into_iter().enumerate() {
          row = row.at(column, cell);
          column += widths.get(at).copied().unwrap_or_default() + 2;
        }
        row
      })
      .collect()
  }
}

/// Parts laid out one under the other, with a gap of blank rows between.
fn under(parts: Vec<Vec<Row>>, gap: usize) -> Vec<Row> {
  let mut rows = Vec::new();
  for (at, part) in parts.into_iter().enumerate() {
    if at > 0 {
      rows.extend((0..gap).map(|_| Row::default()));
    }
    rows.extend(part);
  }
  rows
}

/// Parts laid out side by side, each in a column as wide as its widest row, with a gap between.
fn beside(parts: &[Vec<Row>], gap: usize) -> Vec<Row> {
  let height = parts.iter().map(Vec::len).max().unwrap_or_default();
  let widths: Vec<usize> =
    parts.iter().map(|rows| rows.iter().map(Row::width).max().unwrap_or_default()).collect();
  (0..height)
    .map(|y| {
      let (mut row, mut column) = (Row::default(), 0);
      for (part, width) in parts.iter().zip(&widths) {
        row = row.at(column, part.get(y).cloned().unwrap_or_default());
        column += width + gap;
      }
      row
    })
    .collect()
}

/// The columns of a line.
fn wide(line: &Line) -> usize {
  line.iter().map(|span| span.content.width()).sum()
}

impl Row {
  /// The columns of the row.
  pub(crate) fn width(&self) -> usize {
    wide(&self.line)
  }

  /// The row with its hot columns moved right by a count of columns, as a head of that width
  /// will stand before it.
  pub(crate) fn shifted(mut self, by: usize) -> Self {
    for hot in &mut self.hot {
      hot.0 = hot.0.start + by..hot.0.end + by;
    }
    for loader in &mut self.loaders {
      loader.0 += by;
    }
    self
  }

  /// The row after a head of runs, whose columns the hot columns already count.
  fn with_head(mut self, head: Line) -> Self {
    let mut line = head;
    line.append(&mut self.line);
    self.line = line;
    self
  }

  /// The row with another row placed at a column, after blanks up to it.
  fn at(mut self, column: usize, next: Self) -> Self {
    let pad = column.saturating_sub(self.width());
    self.line.push(run(" ".repeat(pad), Style::new()));
    let next = next.shifted(self.width());
    self.line.extend(next.line);
    self.hot.extend(next.hot);
    self.loaders.extend(next.loaders);
    self
  }
}
