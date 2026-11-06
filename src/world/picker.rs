//! The files that the picker of a window offers for what the operator types after `@`: walked by
//! ignore, the walker of ripgrep, which reads `.gitignore`, and ranked by nucleo, the matcher of
//! helix. Of the matchers measured on this repository, nucleo ranked the meant file first for 11 of
//! 15 known queries, the matcher of skim for 9 of them, with 3 that it did not match at all.
//!
//! A [`Picker`] walks its folder once, when it opens, on a thread of its own, and nucleo matches
//! the files as the walk gives them, on threads of its own. A key gives the picker its query and
//! reads the rows that stand at that moment, so no key waits for a walk or a match.

use std::{
  env,
  path::{self, Component, Path, PathBuf},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
};

use ignore::WalkBuilder;
use nucleo::{
  Config, Nucleo,
  pattern::{CaseMatching, Normalization},
};

/// The most files that one walk lists, so a walk from a folder as large as `/` ends at once.
const MOST: usize = 50_000;

/// The files under a folder, by their path from it with `/` between its parts, in order: each dot
/// file too, but each file that a `.gitignore` or an `.ignore` hides, and `.git` itself.
pub fn walked(root: &Path) -> Vec<String> {
  let mut found = Vec::new();
  walk(root, |path| {
    found.push(path);
    true
  });
  found.sort();
  found
}

/// Each file of [`walked`], given to `each` as the walk finds it, until `each` says to stop or
/// [`MOST`] files came.
fn walk(root: &Path, mut each: impl FnMut(String) -> bool) {
  if root.file_name().is_some_and(|name| name == ".git") {
    return;
  }
  let walk = WalkBuilder::new(root)
    .hidden(false)
    .require_git(false)
    .filter_entry(|entry| entry.file_name() != ".git")
    .sort_by_file_name(Ord::cmp)
    .build();
  let files = walk
    .filter_map(Result::ok)
    .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
    .filter_map(|entry| {
      let inside = entry.path().strip_prefix(root).ok()?;
      let parts = inside.components().map(|part| part.as_os_str().to_string_lossy());
      Some(parts.collect::<Vec<_>>().join("/"))
    })
    .take(MOST);
  for path in files {
    if !each(path) {
      return;
    }
  }
}

/// The picker of a window: the files of the directory, and of the last folder outside it that a
/// query named, each walked once, and the rows that the last query picks from them.
pub struct Picker {
  directory: PathBuf,
  home: Option<PathBuf>,
  project: Index,
  outside: Option<Index>,
  /// Whether the last query named a folder outside the directory.
  away: bool,
}

/// What a key of the picker shows: the first rows that the query picks, best first, and whether the
/// walk or the match still runs, so later rows may come.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", napi_derive::napi(object, object_from_js = false))]
pub struct Rows {
  pub paths: Vec<String>,
  pub running: bool,
}

impl Picker {
  /// A picker of a directory, whose walk starts now.
  pub fn new(directory: &Path) -> Self {
    Self {
      directory: directory.to_owned(),
      home: env::home_dir(),
      project: Index::open(Some(directory.to_owned()), String::new()),
      outside: None,
      away: false,
    }
  }

  /// Takes the query that the operator typed after `@`. A query that names an absolute path or
  /// starts with `/`, `~` or `../` matches the files of the folder that it names up to its last
  /// separator, which a walk lists the first time that the folder is named, and its rows are
  /// written from that folder as the query writes it; any other matches the files of the directory.
  pub fn find(&mut self, query: &str) {
    let Some((folder, rest)) = outside(query) else {
      self.away = false;
      self.project.find(query);
      return;
    };
    self.away = true;
    let index = match &mut self.outside {
      Some(index) if index.folder == folder => index,
      held => {
        let root = placed(&self.directory, &folder, self.home.clone());
        held.insert(Index::open(root, folder))
      }
    };
    index.find(rest);
  }

  /// The first rows of the last query as they stand now, with no wait for the walk or the match.
  pub fn rows(&mut self, most: u32) -> Rows {
    let index = match (&mut self.outside, self.away) {
      (Some(outside), true) => outside,
      _ => &mut self.project,
    };
    let running = index.nucleo.tick(0).running || index.nucleo.active_injectors() > 0;
    let snapshot = index.nucleo.snapshot();
    let first = snapshot.matched_items(..most.min(snapshot.matched_item_count()));
    let paths = first.map(|item| format!("{}{}", index.folder, item.data)).collect();
    Rows { paths, running }
  }
}

/// The files of one folder, which a walk gives nucleo as it finds them, and the text that nucleo
/// matches now.
struct Index {
  /// The folder as the query writes it, which each row starts with: empty for the directory.
  folder: String,
  nucleo: Nucleo<String>,
  text: String,
  /// Whether the walk should stop, as the index is dropped.
  stop: Arc<AtomicBool>,
}

impl Index {
  /// The index of a folder, whose walk starts now on a thread of its own; an index of nothing when
  /// the folder is not known.
  fn open(root: Option<PathBuf>, folder: String) -> Self {
    let nucleo = Nucleo::new(Config::DEFAULT.match_paths(), Arc::new(|| {}), None, 1);
    let stop = Arc::new(AtomicBool::new(false));
    if let Some(root) = root {
      let (injector, stopped) = (nucleo.injector(), Arc::clone(&stop));
      thread::spawn(move || {
        walk(&root, |path| {
          injector.push(path, |one, columns| columns[0] = one.as_str().into());
          !stopped.load(Ordering::Relaxed)
        });
      });
    }
    Self { folder, nucleo, text: String::new(), stop }
  }

  /// Matches this text from now on: a text that only grows matches among the files that the last
  /// one matched.
  fn find(&mut self, text: &str) {
    let grows = text.starts_with(self.text.as_str());
    let pattern = &mut self.nucleo.pattern;
    pattern.reparse(0, text, CaseMatching::Smart, Normalization::Smart, grows);
    text.clone_into(&mut self.text);
  }
}

impl Drop for Index {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Relaxed);
  }
}

/// The folder that a query outside the directory names, from the directory or from the home
/// folder for a `~`; none when the home folder is not known.
fn placed(directory: &Path, folder: &str, home: Option<PathBuf>) -> Option<PathBuf> {
  let place = match folder.strip_prefix('~') {
    Some(under) => home?.join(under.trim_start_matches(path::is_separator)),
    None => directory.join(folder),
  };
  Some(normal(&place))
}

/// The folder that a query outside the directory names, with its separator, and the rest of it.
fn outside(query: &str) -> Option<(String, &str)> {
  let names = |dots: &str| {
    query
      .strip_prefix(dots)
      .is_some_and(|after| after.is_empty() || after.starts_with(path::is_separator))
  };
  if !(query.starts_with(path::is_separator)
    || Path::new(query).is_absolute()
    || names("..")
    || names("~"))
  {
    return None;
  }
  Some(match query.rfind(path::is_separator) {
    Some(at) => (query[..=at].to_owned(), &query[at + 1..]),
    None => (format!("{query}/"), ""),
  })
}

/// A path with each `..` taken against the part before it, so a walk lists its files from the
/// folder itself.
fn normal(path: &Path) -> PathBuf {
  let mut made = PathBuf::new();
  for part in path.components() {
    match part {
      Component::ParentDir if made.file_name().is_some() => {
        made.pop();
      }
      Component::CurDir => {}
      other => made.push(other),
    }
  }
  made
}

#[cfg(test)]
#[path = "picker.test.rs"]
mod test;
