//! The part of the changes of the open thread, over the box of its context at the foot of the
//! list: one card of the pull request of its branch, with its title, its number, the lines that it
//! adds and removes and how its checks stand, or, with no pull request, its branch and the lines
//! that its checkout adds and removes against its base. A click on the title opens the pull
//! request, on the counts the review of the changes, and on the checks the checks of the pull
//! request: each thing of the card is its own target, so it holds no menu.

use std::{mem, ops::Range};

use crate::cells::Cells;
use opentui::style::Style;

use crate::{
  keys::Does,
  line::{Line, cut, run, spread, styled},
  model::Model,
  tokens::{Loader, Role},
};

/// What a click on the card of the changes does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Changing {
  Pull,
  Diff,
  Checks,
}

/// A row of the part: its runs, and the columns of the list that take a click.
pub(crate) type Piece = (Line, Vec<(Range<usize>, Changing)>);

impl Model {
  /// The rows of the part of the changes: its head, the two lines of its card and a blank row,
  /// while a thread is open in a checkout with a branch; none else.
  pub(crate) fn changes_rows(&self) -> Vec<Piece> {
    if self.client.thread.is_none() || !self.listed() {
      return Vec::new();
    }
    let Some((at, folder)) = self.client.target_folder() else { return Vec::new() };
    let Some(changes) = self.client.changes(at, &folder) else { return Vec::new() };
    let Some(branch) = changes.branch.as_deref() else { return Vec::new() };
    let t = &self.tokens;
    let inner = usize::from(self.list_width()) - 3;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let sums =
      || changes.files.iter().fold((0, 0), |sum, one| (sum.0 + one.added, sum.1 + one.removed));
    let based = changes.base.map(|one| (one.added, one.removed));
    let (title, number, (added, removed), opens) = match &changes.pull {
      Some(pull) => (
        pull.title.as_str(),
        Some(format!("#{}", pull.number)),
        (pull.added, pull.removed),
        Changing::Pull,
      ),
      None => (branch, None, based.unwrap_or_else(sums), Changing::Diff),
    };
    let title = cut(title, inner.saturating_sub(2));
    let left = vec![run("⑂ ", Style::new().fg(t.accent)), run(title.clone(), prose)];
    let first = spread(left, Vec::new(), inner, Style::new());
    let mut hot = vec![(1..3 + title.width(), opens)];
    let first = (indented(first), mem::take(&mut hot));
    let mut left = Vec::new();
    let mut column = 1;
    if let Some(number) = number {
      hot.push((column..column + number.width(), Changing::Pull));
      column += number.width() + 1;
      left.extend([run(number, muted), run(" ", Style::new())]);
    }
    let counts = [
      run(format!("+{}", grouped(added)), Style::new().fg(t.added)),
      run(" ", Style::new()),
      run(format!("−{}", grouped(removed)), Style::new().fg(t.danger)),
    ];
    let wide: usize = counts.iter().map(|one| one.content.width()).sum();
    hot.push((column..column + wide, Changing::Diff));
    left.extend(counts);
    let checks =
      changes.pull.as_ref().and_then(|pull| match (pull.failing, pull.pending, pull.passing) {
        (0, 0, 0) => None,
        (0, 0, _) => Some(vec![run("✓ passing", Style::new().fg(t.added))]),
        (0, _, _) => {
          let frame = Loader::Command.frame(self.client.moved);
          Some(vec![run(format!("{frame} pending"), Style::new().fg(t.attention))])
        }
        (failing, ..) => Some(vec![run(format!("✕ {failing} failing"), Style::new().fg(t.danger))]),
      });
    let right = checks.unwrap_or_default();
    let shown: usize = right.iter().map(|one| one.content.width()).sum();
    if shown > 0 {
      hot.push((1 + inner - shown..1 + inner, Changing::Checks));
    }
    let second = (indented(spread(left, right, inner, Style::new())), hot);
    let head = (indented(vec![run("Changes", muted)]), Vec::new());
    vec![head, first, second, (Vec::new(), Vec::new())]
  }

  /// A row of the part as the list draws it: the thing under the pointer that takes a click goes
  /// bright.
  pub(crate) fn changes_line(&self, at: usize, pointed: bool) -> Line {
    let Some((line, hot)) = self.changes_rows().into_iter().nth(at) else { return Vec::new() };
    let column = self.pointer.and_then(|(x, _)| self.list_column(x)).filter(|_| pointed);
    let range =
      column.and_then(|x| hot.iter().find(|one| one.0.contains(&x)).map(|one| one.0.clone()));
    match range {
      Some(range) => styled(line, &range, |one| one.fg(self.tokens.hover)),
      None => line,
    }
  }

  /// A click on a row of the part, where the pointer stands: what it does, and the key whose thing
  /// it does, as the review of the changes.
  pub(crate) fn changes_clicked(&mut self, row: usize) -> Option<Does> {
    let column = self.list_column(self.pointer?.0)?;
    let rows = self.changes_rows();
    let does = rows.get(row)?.1.iter().find(|one| one.0.contains(&column))?.1;
    let pull = self.client.target_folder().and_then(|(at, folder)| {
      self.client.changes(at, &folder)?.pull.as_ref().map(|one| one.url.clone())
    });
    let to = match (does, pull) {
      (Changing::Diff, _) | (_, None) => return Some(Does::Changes),
      (Changing::Checks, Some(url)) => format!("{url}/checks"),
      (Changing::Pull, Some(url)) => url,
    };
    self.client.notice = Some(match (self.browse)(&to) {
      Ok(()) => format!("Opened {to} in the browser."),
      Err(no) => format!("The browser did not open {to}: {no}"),
    });
    None
  }

  /// Whether the checks of the pull request of the open thread wait, so their mark moves.
  pub(crate) fn checks_wait(&self) -> bool {
    let Some((at, folder)) = self.client.target_folder() else { return false };
    let pull = self.client.changes(at, &folder).and_then(|one| one.pull.as_ref());
    pull.is_some_and(|one| one.failing == 0 && one.pending > 0) && !self.changes_rows().is_empty()
  }

  /// The column of the list at a column of the window, past the padding of its panel.
  fn list_column(&self, x: u16) -> Option<usize> {
    usize::from(x).checked_sub(usize::from(self.size.0.saturating_sub(self.list_width())) + 1)
  }
}

/// A line of the part, one column in from the edge of the list, as the box of the context.
fn indented(line: Line) -> Line {
  let mut indented = vec![run(" ", Style::new())];
  indented.extend(line);
  indented
}

/// A count with a comma between each group of three digits, as `2,061`.
fn grouped(count: u32) -> String {
  let digits = count.to_string();
  let mut out = String::new();
  for (at, digit) in digits.chars().enumerate() {
    if at > 0 && (digits.len() - at) % 3 == 0 {
      out.push(',');
    }
    out.push(digit);
  }
  out
}
