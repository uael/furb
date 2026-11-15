//! The review of the changes of a project as the window draws it: over the whole window, a head
//! that names the project, its branch and the count of its changes, the changed files at the left
//! with how each differs and the lines that it adds and removes, the file that he reads lifted, and
//! its diff at the right, which the diff of OpenTUI draws with its line numbers and its colors.
//! What the review holds stands in the client.

use opentui::style::Style;

use furb_host::protocol::{Change, Changed, Changes};

use crate::{
  input::named,
  keys::Does,
  line::{Line, run, spread},
  model::Model,
  tokens::Role,
};

/// The columns of the files at the left of the review.
pub(crate) const FILES: u16 = 40;

/// The review as the window draws it: its head, the rows of its files, its foot, the diff of the
/// file that he reads with the kind of its text, and the place of the chosen file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shown {
  pub(crate) head: Line,
  pub(crate) files: Vec<Line>,
  pub(crate) foot: Line,
  pub(crate) diff: String,
  pub(crate) kind: Option<&'static str>,
}

impl Model {
  /// The review, while one stands.
  pub(crate) fn reviewing(&self) -> Option<Shown> {
    let review = self.client.review.as_ref()?;
    let muted = self.tokens.style(Role::Muted);
    let changes = self.client.changes(review.at, &review.folder);
    let foot = format!(
      "{}{} file · {}{} scroll · {} close",
      self.key(Does::Up),
      self.key(Does::Down),
      self.key(Does::PageUp),
      self.key(Does::PageDown),
      self.key(Does::Close),
    );
    let read = self.client.reviewed().map(|one| one.path.clone());
    let diff =
      review.diff.as_ref().filter(|one| Some(&one.0) == read.as_ref()).map(|one| one.1.clone());
    Some(Shown {
      head: self.review_head(&review.folder, changes),
      files: self.review_files(review.chosen, changes),
      foot: vec![run(foot, muted)],
      diff: diff.unwrap_or_default(),
      kind: read.as_deref().and_then(kind),
    })
  }

  /// The head of the review: the project, its branch and its count of files at the left, and the
  /// lines that its changes add and remove at the right.
  fn review_head(&self, folder: &str, changes: Option<&Changes>) -> Line {
    let t = &self.tokens;
    let files = changes.map(|one| &one.files[..]).unwrap_or_default();
    let (added, removed) =
      files.iter().fold((0, 0), |sum, one| (sum.0 + one.added, sum.1 + one.removed));
    let branch = changes.and_then(|one| one.branch.as_deref()).map(|one| format!(" · ⎇ {one}"));
    let count = match files.len() {
      1 => "1 file".to_owned(),
      many => format!("{many} files"),
    };
    let title = format!("Changes of {}{}", named(folder), branch.unwrap_or_default());
    let left =
      vec![run(title, t.style(Role::Title)), run(format!("  {count}"), t.style(Role::Muted))];
    let sums = vec![
      run(format!("+{added}"), Style::new().fg(t.hues[3])),
      run(format!(" −{removed}"), Style::new().fg(t.danger)),
    ];
    spread(left, sums, usize::from(self.size.0).saturating_sub(4), Style::new())
  }

  /// A row for each changed file: how it differs, its path, and the lines that it adds and
  /// removes, the chosen one on the lifted ground; or one row that says why none shows.
  fn review_files(&self, chosen: usize, changes: Option<&Changes>) -> Vec<Line> {
    let t = &self.tokens;
    let (muted, prose) = (t.style(Role::Muted), t.style(Role::Prose));
    let room = usize::from(FILES).saturating_sub(3);
    let files = changes.map(|one| &one.files[..]).unwrap_or_default();
    let row = |(at, one): (usize, &Changed)| {
      let ground = |style: Style| if at == chosen { style.bg(t.lifted) } else { style };
      let (mark, color) = match one.state {
        Change::Added => ("A", t.hues[3]),
        Change::Modified => ("M", t.attention),
        Change::Deleted => ("D", t.danger),
        Change::Renamed => ("R", t.accent),
        Change::Untracked => ("?", t.hues[3]),
      };
      let left = vec![
        run(format!(" {mark} "), ground(Style::new().fg(color))),
        run(one.path.clone(), ground(prose)),
      ];
      let counts = vec![
        run(format!("+{} −{}", one.added, one.removed), ground(muted)),
        run(" ", ground(Style::new())),
      ];
      spread(left, counts, room, ground(Style::new()))
    };
    let mut rows: Vec<Line> = files.iter().enumerate().map(row).collect();
    if rows.is_empty() {
      let says = match changes {
        None => "Reading the changes…",
        Some(one) if one.branch.is_none() => "No change, or no checkout of git here.",
        Some(_) => "No change since the last commit.",
      };
      rows.push(vec![run(format!(" {says}"), muted)]);
    }
    rows
  }

  /// The file of the review at a row of the window, which a click reads.
  pub(crate) fn review_at(&self, x: u16, y: u16) -> Option<usize> {
    let shown = self.reviewing()?;
    let row = usize::from(y.checked_sub(2)?);
    (x < FILES && row < shown.files.len()).then_some(row)
  }
}

/// The kind of the text of a file, by its extension, as the diff colors it.
fn kind(path: &str) -> Option<&'static str> {
  let extension = path.rsplit_once('.')?.1;
  let kind = match extension {
    "rs" => "rust",
    "py" | "pyi" => "python",
    "ts" | "tsx" => "typescript",
    "js" | "mjs" | "cjs" => "javascript",
    "md" => "markdown",
    "json" => "json",
    "toml" => "toml",
    "sh" => "bash",
    "zig" => "zig",
    _ => return None,
  };
  Some(kind)
}
