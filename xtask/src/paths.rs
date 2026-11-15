//! The task `paths`, which fails on each path of the repository that a tool of a developer or a
//! workflow names and that git does not hold.
//!
//! A move of a file leaves its old path in a tool that no test of that change runs, and the tool
//! fails later. So the task reads each file of the folders of `WATCHED`, and names each path whose
//! first part is a folder of the repository and that is neither a file nor a folder that git
//! holds. A path that a build makes and git ignores stands, a path that a tool makes outside the
//! repository is listed in `MADE`, and a path with a variable or a pattern in it is not a whole
//! path, so it is not checked. A path after `$root/`, which the scripts use for the root of the
//! repository, is checked as a path of the repository.

use std::{
  cmp::Reverse,
  collections::BTreeSet,
  fs,
  path::Path,
  process::{Command as Process, ExitCode},
};

use fancy_regex::{Regex, escape};

use crate::{Fault, git};

/// The folders of the tools of a developer, the scripts and the tasks, and of the workflows that
/// run them. The tests of xtask stay out, since a test names the paths of a yard of its own.
const WATCHED: [&str; 4] = ["script/", "xtask/src/", "xtask/python/", ".github/workflows/"];
/// The names that a script gives to the root of the repository.
const ROOTS: [&str; 2] = ["root", "ROOT"];
/// The paths that a tool makes in a folder that is not the repository.
const MADE: [&str; 1] = ["src/main.rs"];

/// A path that a file names at a line and that git does not hold.
struct Stale<'a> {
  file: &'a str,
  line: usize,
  path: String,
}

/// Print each path that git does not hold, as `<file>:<line>: <path> ...`, and fail when one
/// stands.
pub(crate) fn check(root: &Path) -> Result<ExitCode, Fault> {
  let files: BTreeSet<String> = git(root, &["ls-files", "-z"])?
    .split('\0')
    .filter(|file| !file.is_empty() && root.join(file).exists())
    .map(str::to_owned)
    .collect();
  let named = pattern(&files)?;
  let mut found = Vec::new();
  for file in files.iter().filter(|file| WATCHED.iter().any(|folder| file.starts_with(folder))) {
    let path = root.join(file);
    let bytes = fs::read(&path).map_err(|source| Fault::Read { path, source })?;
    for (at, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
      for one in named.captures_iter(line) {
        let path = one?[1].trim_end_matches(['.', '/']).to_owned();
        if !(path.contains(['*', '$', '?', '%'])
          || MADE.contains(&path.as_str())
          || stands(&path, &files))
        {
          found.push(Stale { file, line: at + 1, path });
        }
      }
    }
  }
  let made = ignored(root, &found)?;
  found.retain(|one| !made.contains(&one.path));
  for one in &found {
    println!("{}:{}: {} is no file or folder of git", one.file, one.line, one.path);
  }
  Ok(if found.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

/// The pattern of a path whose first part is a folder of `files`, after no character of a word or
/// of a longer path, or after a name of the root.
fn pattern(files: &BTreeSet<String>) -> Result<Regex, Fault> {
  let mut tops: Vec<&str> =
    files.iter().filter_map(|file| file.split_once('/')).map(|(top, _)| top).collect();
  tops.sort_by_key(|top| Reverse(top.len()));
  tops.dedup();
  let tops: Vec<_> = tops.into_iter().map(escape).collect();
  let roots: Vec<String> =
    ROOTS.iter().map(|name| format!(r"(?<=\${name}/)|(?<=\$\{{{name}\}}/)")).collect();
  let before = [r"(?<![\w./@$}-])".to_owned(), roots.join("|")].join("|");
  Ok(Regex::new(&format!(r#"(?:{before})((?:{})/[^\s"'`()=,;:|<>&\\\[\]{{}}]*)"#, tops.join("|")))?)
}

fn stands(path: &str, files: &BTreeSet<String>) -> bool {
  let folder = format!("{path}/");
  files.contains(path)
    || files.range(folder.clone()..).next().is_some_and(|file| file.starts_with(&folder))
}

/// The paths that git ignores. A folder that does not stand yet is ignored by a rule that ends in a
/// slash only when it is asked as a folder, so each path is asked in both forms.
fn ignored(root: &Path, found: &[Stale]) -> Result<BTreeSet<String>, Fault> {
  if found.is_empty() {
    return Ok(BTreeSet::new());
  }
  let asked: BTreeSet<&str> = found.iter().map(|one| one.path.as_str()).collect();
  let output = Process::new("git")
    .args(["check-ignore", "--"])
    .args(asked.iter().flat_map(|path| [(*path).to_owned(), format!("{path}/")]))
    .current_dir(root)
    .output()
    .map_err(|source| Fault::Spawn { program: "git".to_owned(), source })?;
  // check-ignore exits 1 when it ignores none of the paths.
  if !matches!(output.status.code(), Some(0 | 1)) {
    return Err(Fault::Git {
      args: "check-ignore".to_owned(),
      stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    });
  }
  Ok(
    String::from_utf8_lossy(&output.stdout)
      .lines()
      .map(|path| path.trim_end_matches('/').to_owned())
      .collect(),
  )
}
