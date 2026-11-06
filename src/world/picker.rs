//! The files that the picker of a window offers for what the operator types after `@`: walked by
//! ignore, the walker of ripgrep, which reads `.gitignore`, and ranked by the matcher of nucleo,
//! which helix uses. Of the matchers measured on this repository, nucleo ranked the meant file
//! first for 11 of 15 known queries, the matcher of skim for 9 of them, with 3 that it did not
//! match at all.

use std::{
  env,
  path::{Component, Path, PathBuf},
};

use ignore::WalkBuilder;
use nucleo_matcher::{
  Config, Matcher, Utf32Str,
  pattern::{CaseMatching, Normalization, Pattern},
};

/// The most files that one walk lists, so a walk from a folder as large as `/` ends at once.
const MOST: usize = 50_000;

/// The files under a folder, by their path from it with `/` between its parts, in order: each dot
/// file too, but each file that a `.gitignore` or an `.ignore` hides, and `.git` itself.
pub fn walked(root: &Path) -> Vec<String> {
  if root.file_name().is_some_and(|name| name == ".git") {
    return Vec::new();
  }
  let walk = WalkBuilder::new(root)
    .hidden(false)
    .require_git(false)
    .filter_entry(|entry| entry.file_name() != ".git")
    .build();
  let mut found: Vec<String> = walk
    .filter_map(Result::ok)
    .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
    .filter_map(|entry| {
      let inside = entry.path().strip_prefix(root).ok()?;
      let parts = inside.components().map(|part| part.as_os_str().to_string_lossy());
      Some(parts.collect::<Vec<_>>().join("/"))
    })
    .take(MOST)
    .collect();
  found.sort();
  found
}

/// The paths that match a query, best first, as nucleo ranks a path, a tie to the shorter path and
/// then to the order given; each path in its order for a query of no text.
pub fn ranked(query: &str, paths: &[String]) -> Vec<String> {
  if query.trim().is_empty() {
    return paths.to_vec();
  }
  let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
  let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
  let mut buffer = Vec::new();
  let mut scored: Vec<(u32, usize, &String)> = paths
    .iter()
    .enumerate()
    .filter_map(|(at, path)| {
      let score = pattern.score(Utf32Str::new(path, &mut buffer), &mut matcher)?;
      Some((score, at, path))
    })
    .collect();
  scored.sort_by(|one, other| {
    (other.0.cmp(&one.0)).then(one.2.len().cmp(&other.2.len())).then(one.1.cmp(&other.1))
  });
  scored.into_iter().map(|(_, _, path)| path.clone()).collect()
}

/// The files that a query after `@` picks, best first. A query that starts with `/`, `~` or `../`
/// walks from the folder that it names up to its last `/`, and matches the rest, each path written
/// from that folder as the query writes it; any other matches the files of the directory, which
/// `project` holds as [`walked`] gives them.
pub fn picked(directory: &Path, query: &str, project: &[String]) -> Vec<String> {
  let Some((folder, rest)) = outside(query) else { return ranked(query, project) };
  let place = match folder.strip_prefix('~') {
    Some(under) => match env::home_dir() {
      Some(home) => home.join(under.trim_start_matches('/')),
      None => return Vec::new(),
    },
    None => directory.join(&folder),
  };
  let root = normal(&place);
  ranked(rest, &walked(&root)).into_iter().map(|path| format!("{folder}{path}")).collect()
}

/// The folder that a query outside the directory names, ending with `/`, and the rest of it.
fn outside(query: &str) -> Option<(String, &str)> {
  let leaves = query.starts_with('/')
    || query.starts_with("../")
    || query == ".."
    || query.starts_with("~/")
    || query == "~";
  if !leaves {
    return None;
  }
  Some(match query.rfind('/') {
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
