//! The width of the comments of Rust. rustfmt sets the width of the code at 100 and wraps no
//! comment, so a comment goes past it with no gate that says so: this check fails on a comment
//! line that a change adds past it.

use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::{Path, PathBuf},
  process::ExitCode,
};

use ra_ap_rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};

use crate::{Fault, git};

const WIDTH: usize = 100;

pub(crate) fn check(root: &Path, base: Option<&str>, files: &[PathBuf]) -> Result<ExitCode, Fault> {
  let found = failures(root, base, files)?;
  for line in &found {
    println!("{line}");
  }
  Ok(if found.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

/// Each line that the tree adds to the base and that holds only a comment past `WIDTH`, as
/// `file:line: ...`.
fn failures(root: &Path, base: Option<&str>, files: &[PathBuf]) -> Result<Vec<String>, Fault> {
  let base = base.map_or_else(|| merged(root), str::to_owned);
  let named: Vec<String> = if files.is_empty() {
    vec!["*.rs".to_owned()]
  } else {
    files.iter().map(|one| one.to_string_lossy().into_owned()).collect()
  };
  let named: Vec<&str> = named.iter().map(String::as_str).collect();
  let diff = git(root, &[&["diff", "-U0", &base, "--"][..], &named].concat())?;
  let mut found = Vec::new();
  for (file, lines) in added(&diff) {
    // A file that a change deletes, or that holds no text, holds no comment to read.
    let Ok(text) = fs::read_to_string(root.join(&file)) else { continue };
    for line in long(&text).into_iter().filter(|line| lines.contains(line)) {
      found.push(format!(
        "{file}:{line}: a comment line that this change adds goes past {WIDTH} characters"
      ));
    }
  }
  Ok(found)
}

/// The merge base of the head with main, or the head when the clone has no main.
fn merged(root: &Path) -> String {
  ["origin/main", "main"]
    .iter()
    .find_map(|main| git(root, &["merge-base", "HEAD", main]).ok())
    .map_or_else(|| "HEAD".to_owned(), |base| base.trim().to_owned())
}

/// The numbers of the lines that a diff with no context adds, for each file.
fn added(diff: &str) -> BTreeMap<String, BTreeSet<usize>> {
  let (mut file, mut added) = (None, BTreeMap::<String, BTreeSet<usize>>::new());
  for row in diff.lines() {
    if let Some(name) = row.strip_prefix("+++ ") {
      file = name.strip_prefix("b/").map(str::to_owned);
    } else if let (Some(file), Some(hunk)) = (&file, row.strip_prefix("@@ -")) {
      let new = hunk.split_whitespace().nth(1).unwrap_or_default().trim_start_matches('+');
      let (first, count) = new.split_once(',').unwrap_or((new, "1"));
      let (Ok(first), Ok(count)) = (first.parse::<usize>(), count.parse::<usize>()) else {
        continue;
      };
      added.entry(file.clone()).or_default().extend(first..first + count);
    }
  }
  added
}

/// The numbers, from 1, of the lines that hold only a line comment longer than `WIDTH`, as the
/// lexer of rustc reads them, so a comment inside a string is none.
fn long(text: &str) -> Vec<usize> {
  let (mut at, mut line, mut found) = (0, 1, Vec::new());
  for token in tokenize(text, FrontmatterAllowed::No) {
    let end = at + token.len as usize;
    if matches!(token.kind, TokenKind::LineComment { .. }) {
      let start = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
      if text[start..at].trim().is_empty() && text[start..end].chars().count() > WIDTH {
        found.push(line);
      }
    }
    line += text[at..end].matches('\n').count();
    at = end;
  }
  found
}
