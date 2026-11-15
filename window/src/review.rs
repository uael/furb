//! The review of the changes of a project as the window draws it, in the frame of the thread: the
//! diff of the file that he reads where the feed stands, under a head that names the project, its
//! branch and that file, and the changed files where the list stands, on its panel, the chosen one
//! on the ground of a selected card. The diff of OpenTUI draws its line numbers, its grounds and
//! the colors of its grammar. What the review holds stands in the client.

use opentui::style::Style;
use unicode_width::UnicodeWidthStr;

use furb_client::Review;
use furb_host::protocol::{Change, Changed, Changes};

use crate::{
  input::named,
  keys::Does,
  line::{Line, cut, edged, run, spread},
  model::Model,
  tokens::{BESIDE, Role},
};

/// The rows of the panel above its first file: its head and a blank row.
const ABOVE: u16 = 2;

/// The review as the window draws it: the head of the diff, the rows of the panel, which shows
/// only where the list would, the foot, the diff of the file that he reads with the kind of its
/// text, and whether its old and new lines stand side by side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shown {
  pub(crate) head: Line,
  pub(crate) side: Option<Vec<Line>>,
  pub(crate) foot: Line,
  pub(crate) diff: String,
  pub(crate) kind: Option<&'static str>,
  pub(crate) split: bool,
}

impl Model {
  /// The review, while one stands.
  pub(crate) fn reviewing(&self) -> Option<Shown> {
    let review = self.client.review.as_ref()?;
    let changes = self.client.changes(review.at, &review.folder);
    let read = self.client.reviewed();
    let diff = review.diff.as_ref().filter(|one| Some(&one.0) == read.map(|one| &one.path));
    let diff = diff.map(|one| one.1.clone()).unwrap_or_default();
    let mut side = vec![self.review_sums(changes), Line::new()];
    side.extend(self.review_files(review.chosen, changes));
    Some(Shown {
      head: self.review_head(review),
      side: self.sided().then_some(side),
      foot: self.review_foot(),
      split: self.splits(&diff),
      kind: read.and_then(|one| kind(&one.path)),
      diff,
    })
  }

  /// The head of the diff, as the head of a thread: the project and its branch muted, the file
  /// that he reads, and the lines that it adds and removes at the right. With no panel, it says
  /// which file of how many he reads.
  fn review_head(&self, review: &Review) -> Line {
    let (changes, read) = (self.client.changes(review.at, &review.folder), self.client.reviewed());
    let muted = self.tokens.style(Role::Muted);
    let branch = changes.and_then(|one| one.branch.as_deref()).map(|one| format!(" · ⑂ {one}"));
    let project = format!(" {}{} · ", named(&review.folder), branch.unwrap_or_default());
    let mut left = vec![run(project, muted)];
    left.extend(read.map(|one| run(one.path.clone(), self.tokens.style(Role::Title))));
    let count = changes.map_or(0, |one| one.files.len());
    if !self.sided() && count > 1 {
      left.push(run(format!("  {} of {count}", review.chosen + 1), muted));
    }
    let counts = read.map(|one| self.counts(one.added, one.removed)).unwrap_or_default();
    spread(left, counts, self.diff_width(), Style::new())
  }

  /// The head of the panel, as the head of the list: what it holds, and the lines that all the
  /// changes add and remove.
  fn review_sums(&self, changes: Option<&Changes>) -> Line {
    let files = changes.map(|one| &one.files[..]).unwrap_or_default();
    let (added, removed) =
      files.iter().fold((0, 0), |sum, one| (sum.0 + one.added, sum.1 + one.removed));
    let count = match files.len() {
      1 => "1 file".to_owned(),
      many => format!("{many} files"),
    };
    // The head stands in the columns of the first row of the list, whose panel this is.
    let left = vec![
      run(" ", Style::new()),
      run("Changes", self.tokens.style(Role::Title)),
      run(format!("  {count}"), self.tokens.style(Role::Muted)),
    ];
    spread(left, self.counts(added, removed), usize::from(self.list_width()) - 2, Style::new())
  }

  /// A row for each changed file, as a card of the list: how it differs, its name and its folder,
  /// and the lines that it adds and removes. The chosen one stands on the lifted ground with a bar
  /// in the color of how it differs, and the name of the one under the pointer goes bright. With
  /// no file, one row says why.
  fn review_files(&self, chosen: usize, changes: Option<&Changes>) -> Vec<Line> {
    let t = &self.tokens;
    let files = changes.map(|one| &one.files[..]).unwrap_or_default();
    let pointed = self.pointer.filter(|(x, _)| self.on_side(*x)).map(|(_, y)| y);
    let room = usize::from(self.list_width()) - 5;
    let row = |(place, slot): (usize, Slot<'_>)| {
      let at = match slot {
        Slot::Folder(folder) => {
          let folder = if folder.is_empty() { "./".to_owned() } else { format!("{folder}/") };
          return vec![run(format!("  {}", cut(&folder, room)), t.style(Role::Muted))];
        }
        Slot::File(at) => at,
      };
      let one = &files[at];
      let ground = (at == chosen).then_some(t.lifted);
      let on = |style: Style| ground.map_or(style, |bg| style.bg(bg));
      let lit = pointed.and_then(|y| y.checked_sub(ABOVE)).map(usize::from) == Some(place);
      let (mark, color) = match one.state {
        Change::Added => ("A", t.added),
        Change::Modified => ("M", t.accent),
        Change::Deleted => ("D", t.danger),
        Change::Renamed => ("R", t.accent),
        Change::Untracked => ("U", t.added),
      };
      let name = one.path.rsplit_once('/').map_or(one.path.as_str(), |(_, name)| name);
      let name_style = if lit { t.style(Role::Prose).fg(t.hover) } else { t.style(Role::Prose) };
      let counts = self.counts(one.added, one.removed).into_iter();
      let counts: Line = counts.map(|one| run(one.content, on(one.style))).collect();
      let used = 6 + counts.iter().map(|one| one.content.width()).sum::<usize>();
      let left = vec![
        run(format!("  {mark} "), on(Style::new().fg(color))),
        run(cut(name, room.saturating_sub(used)), on(name_style)),
      ];
      edged(color, ground, spread(left, counts, room, on(Style::new())))
    };
    let mut rows: Vec<Line> = slots(files).into_iter().enumerate().map(row).collect();
    if rows.is_empty() {
      let says = match changes {
        None => "Reading the changes…",
        Some(one) if one.branch.is_none() => "No change, or no checkout of git here.",
        Some(_) => "No change since the last commit.",
      };
      rows.push(vec![run(format!("  {says}"), t.style(Role::Muted))]);
    }
    rows
  }

  /// The foot, as the row of hints: the keys of the review.
  fn review_foot(&self) -> Line {
    let foot = format!(
      "  {}{} file · {} {} change · {}{} scroll · {} close",
      self.key(Does::Up),
      self.key(Does::Down),
      PREVIOUS,
      NEXT,
      self.key(Does::PageUp),
      self.key(Does::PageDown),
      self.key(Does::Close),
    );
    vec![run(foot, self.tokens.style(Role::Muted))]
  }

  /// The lines that a change adds and removes, each in its color, and none of a count of zero.
  fn counts(&self, added: u32, removed: u32) -> Line {
    let t = &self.tokens;
    let added = (added > 0).then(|| run(format!("+{added}"), Style::new().fg(t.added)));
    let removed = (removed > 0).then(|| run(format!("−{removed}"), Style::new().fg(t.danger)));
    let gap = (added.is_some() && removed.is_some()).then(|| run(" ", Style::new()));
    added.into_iter().chain(gap).chain(removed).collect()
  }

  /// The columns of the diff: those that the panel leaves, less one on each side.
  pub(crate) fn diff_width(&self) -> usize {
    usize::from(self.size.0.saturating_sub(if self.sided() { self.panel_width() } else { 0 } + 2))
  }

  /// Whether the panel of the files shows: where the list shows beside a thread.
  fn sided(&self) -> bool {
    self.size.0 >= BESIDE
  }

  /// Whether a diff stands with its old lines beside its new ones: only when it removes and adds,
  /// and each side holds its longest line beside its numbers with no cut.
  fn splits(&self, diff: &str) -> bool {
    let body = || diff.lines().filter(|one| !one.starts_with("+++") && !one.starts_with("---"));
    let changed = |sign: char| body().any(|one| one.starts_with(sign));
    let longest = body().map(|one| one.get(1..).unwrap_or_default().width()).max();
    let numbers = diff.lines().count().to_string().len();
    // A side holds the numbers with one cell before them and the sign after them.
    let side = longest.unwrap_or_default() + numbers + 3;
    changed('-') && changed('+') && 2 * side <= self.diff_width()
  }

  /// Whether a column of the window stands on the panel of the review.
  fn on_side(&self, x: u16) -> bool {
    x >= self.size.0.saturating_sub(self.list_width())
  }

  /// The file of the review at a cell of the window, which a click reads.
  pub(crate) fn review_at(&self, x: u16, y: u16) -> Option<usize> {
    let shown = self.reviewing()?;
    let row = usize::from(y.checked_sub(ABOVE)?);
    shown.side?;
    let review = self.client.review.as_ref()?;
    let files = self.client.changes(review.at, &review.folder).map(|one| &one.files[..]);
    match slots(files.unwrap_or_default()).get(row) {
      Some(Slot::File(at)) if self.on_side(x) => Some(*at),
      _ => None,
    }
  }
}

/// A row of the files of a review: the folder of the files under it, or a file by its place among
/// the files.
enum Slot<'a> {
  Folder(&'a str),
  File(usize),
}

/// The rows of the files of a review: each file under its folder, and each folder once, where it
/// changes, as the tree of the files of a review of GitHub. A file at the top stands under `./`
/// once a folder stood before it.
fn slots(files: &[Changed]) -> Vec<Slot<'_>> {
  let mut slots = Vec::new();
  let mut last = "";
  for (at, one) in files.iter().enumerate() {
    let folder = one.path.rsplit_once('/').map_or("", |(folder, _)| folder);
    if folder != last {
      slots.push(Slot::Folder(folder));
    }
    last = folder;
    slots.push(Slot::File(at));
  }
  slots
}

/// The keys that move the diff to the change before and after the one at its top.
pub(crate) const PREVIOUS: &str = "[";
pub(crate) const NEXT: &str = "]";

/// The kind of the text of a file, by its extension, as the diff colors it.
fn kind(path: &str) -> Option<&'static str> {
  let suffix = path.rsplit_once('.')?.1;
  let kind = match suffix {
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
