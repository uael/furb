//! The changes of a checkout of git as a window shows them: its branch, the files that differ from
//! the last commit with the lines that each adds and removes, and the diff of one of them. Each
//! asks the program git of the machine, which knows each form of a checkout, a worktree among them,
//! and runs off the loop of the daemon under a bound of time, so a slow disk holds back no verb.
//! Each read writes nothing in the checkout: a status takes no optional lock, and each diff is the
//! plumbing `diff-index`, which never refreshes the index, so a git command that the operator runs
//! there at the same moment never meets the lock of the index.

use std::{
  fs,
  path::{Path, PathBuf},
  process::Stdio,
  time::Duration,
};

use serde::Deserialize;
use tokio::{join, process::Command, time::timeout};

use crate::protocol::{Change, Changed, Changes, Lines, Pull};

/// How long git may take to answer, past which the daemon says that it did not.
const BOUND: Duration = Duration::from_secs(10);

/// The changes of the checkout that holds a folder, in that folder alone, with the diff of the file
/// at a path from its top when one is named: none in a folder that no checkout holds. A folder deep
/// in a large checkout names none of the files of the rest of it, as a project in a folder that a
/// checkout of many other projects holds.
pub(crate) async fn changes(folder: &Path, path: Option<&str>) -> Result<Changes, String> {
  let status = ["status", "--porcelain=v1", "-z", "--branch", "--untracked-files=all", "--", "."];
  let Some(status) = git(folder, &status).await? else {
    return Ok(Changes::default());
  };
  let (branch, mut files) = statused(&status);
  // A checkout with no commit has no head to count against: each file is new there.
  let counts = git(folder, &["diff-index", "-M", "--numstat", "-z", "HEAD", "--", "."])
    .await?
    .unwrap_or_default();
  let top = git(folder, &["rev-parse", "--show-toplevel"]).await?.unwrap_or_default();
  let top = PathBuf::from(top.trim());
  for (path, added, removed) in counted(&counts) {
    if let Some(one) = files.iter_mut().find(|one| one.path == path) {
      (one.added, one.removed) = (added, removed);
    }
  }
  for one in files.iter_mut().filter(|one| one.state == Change::Untracked) {
    one.added = lines(&top.join(&one.path));
  }
  let (diff, base, pull) = if let Some(path) = path {
    (Some(diff(folder, path).await?), None, None)
  } else {
    let (base, pull) = join!(based(folder), pulled(folder));
    (None, base, pull)
  };
  Ok(Changes { branch, files, diff, base, pull })
}

/// The lines that the checkout adds and removes in the folder against the point where its branch
/// leaves the default branch of its remote, its tree included: none with no such remote, as git
/// then has nothing to count against.
async fn based(folder: &Path) -> Option<Lines> {
  let remote = git(folder, &["rev-parse", "--abbrev-ref", "origin/HEAD"]).await.ok()??;
  let base = git(folder, &["merge-base", "HEAD", remote.trim()]).await.ok()??;
  let counts =
    git(folder, &["diff-index", "-M", "--numstat", "-z", base.trim(), "--", "."]).await.ok()??;
  let sums = counted(&counts).into_iter().fold((0, 0), |sum, one| (sum.0 + one.1, sum.1 + one.2));
  Some(Lines { added: sums.0, removed: sums.1 })
}

/// A pull request as `gh pr view --json` writes it.
#[derive(Deserialize)]
struct Viewed {
  number: u64,
  title: String,
  url: String,
  additions: u32,
  deletions: u32,
  #[serde(default, rename = "statusCheckRollup")]
  checks: Vec<Check>,
}

/// A check of a pull request as gh writes it: a run of a workflow with its conclusion once it
/// ended, or a status with its state.
#[derive(Deserialize)]
struct Check {
  #[serde(default)]
  conclusion: Option<String>,
  #[serde(default)]
  state: Option<String>,
}

/// The pull request of the branch of the checkout, as the program gh of the machine gives it: none
/// with no gh, no pull request or no answer in time, which the part of the changes shows as a
/// branch with no pull request.
async fn pulled(folder: &Path) -> Option<Pull> {
  let fields = "number,title,url,additions,deletions,statusCheckRollup";
  let mut command = Command::new("gh");
  command.current_dir(folder).args(["pr", "view", "--json", fields]);
  command.stdin(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true);
  let out = timeout(BOUND, command.output()).await.ok()?.ok()?;
  let viewed: Viewed = serde_json::from_slice(&out.stdout).ok()?;
  let mut pull = Pull {
    number: viewed.number,
    title: viewed.title,
    url: viewed.url,
    added: viewed.additions,
    removed: viewed.deletions,
    failing: 0,
    pending: 0,
    passing: 0,
  };
  for check in &viewed.checks {
    let word = check.conclusion.as_deref().filter(|one| !one.is_empty()).or(check.state.as_deref());
    match word.unwrap_or_default() {
      "SUCCESS" | "NEUTRAL" | "SKIPPED" => pull.passing += 1,
      "FAILURE" | "ERROR" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "STARTUP_FAILURE" => {
        pull.failing += 1;
      }
      _ => pull.pending += 1,
    }
  }
  Some(pull)
}

/// The diff of one file of the checkout that holds a folder, by its path from its top.
async fn diff(folder: &Path, path: &str) -> Result<String, String> {
  let top = format!(":(top){path}");
  let tracked = git(folder, &["ls-files", "--error-unmatch", "--", &top]).await?.is_some();
  let diff = if tracked {
    git(folder, &["diff-index", "-p", "HEAD", "--", &top]).await?
  } else {
    let root = git(folder, &["rev-parse", "--show-toplevel"]).await?.unwrap_or_default();
    let file = Path::new(root.trim()).join(path).display().to_string();
    // A diff of no index says by its status 1 that the files differ, which is its answer here.
    git_any(folder, &["diff", "--no-index", "--", "/dev/null", &file]).await?
  };
  Ok(diff.unwrap_or_default())
}

/// What git writes for its arguments in a folder, or none when it fails, as in a folder that no
/// checkout holds. Why git did not run or did not answer in time is the error.
async fn git(folder: &Path, args: &[&str]) -> Result<Option<String>, String> {
  ran(folder, args, |code| code == 0).await
}

/// What git writes for its arguments in a folder when it ends with 0 or 1.
async fn git_any(folder: &Path, args: &[&str]) -> Result<Option<String>, String> {
  ran(folder, args, |code| code <= 1).await
}

async fn ran(
  folder: &Path,
  args: &[&str],
  fine: fn(i32) -> bool,
) -> Result<Option<String>, String> {
  let mut command = Command::new("git");
  command.arg("--no-optional-locks").arg("-C").arg(folder).args(args);
  command.stdin(Stdio::null()).kill_on_drop(true);
  let run = command.output();
  let out = match timeout(BOUND, run).await {
    Ok(Ok(out)) => out,
    Ok(Err(no)) => return Err(format!("git did not run: {no}")),
    Err(_) => return Err(format!("git did not answer in {} s", BOUND.as_secs())),
  };
  let done = out.status.code().is_some_and(fine);
  Ok(done.then(|| String::from_utf8_lossy(&out.stdout).into_owned()))
}

/// The branch and the files of a status of git in its form for programs, with `-z`: a head line
/// that starts with `## `, then each file as two letters of state, a blank and its path, and the
/// path that a renamed file had after it. The records of the sessions of furb stay out.
fn statused(status: &str) -> (Option<String>, Vec<Changed>) {
  let mut parts = status.split('\0').filter(|one| !one.is_empty());
  let mut branch = None;
  let mut files = Vec::new();
  while let Some(part) = parts.next() {
    if let Some(head) = part.strip_prefix("## ") {
      branch = branched(head);
      continue;
    }
    let (Some(code), Some(path)) = (part.get(..2), part.get(3..)) else { continue };
    // The records of the sessions of furb are no change of the project.
    if path.starts_with(".furb/sessions/") {
      continue;
    }
    let state = match code {
      "??" => Change::Untracked,
      _ if code.contains('R') => {
        // The path that the file had before comes next.
        parts.next();
        Change::Renamed
      }
      _ if code.contains('A') => Change::Added,
      _ if code.contains('D') => Change::Deleted,
      _ => Change::Modified,
    };
    files.push(Changed { path: path.to_owned(), added: 0, removed: 0, state });
  }
  (branch, files)
}

/// The branch of the head line of a status: `main...origin/main [ahead 1]`, `No commits yet on
/// main`, or none for a head that names no branch.
fn branched(head: &str) -> Option<String> {
  let head = head.strip_prefix("No commits yet on ").unwrap_or(head);
  if head.starts_with("HEAD (no branch)") {
    return None;
  }
  let name = head.split("...").next()?.split_whitespace().next()?;
  Some(name.to_owned())
}

/// The lines that each file adds and removes, from the counts of git with `-z`: a binary file
/// counts none, and a renamed file names its new path last.
fn counted(counts: &str) -> Vec<(String, u32, u32)> {
  let mut parts = counts.split('\0');
  let mut all = Vec::new();
  while let Some(part) = parts.next() {
    let mut fields = part.splitn(3, '\t');
    let (Some(added), Some(removed), Some(path)) = (fields.next(), fields.next(), fields.next())
    else {
      continue;
    };
    let path = if path.is_empty() {
      parts.next();
      parts.next().unwrap_or_default()
    } else {
      path
    };
    let count = |one: &str| one.parse().unwrap_or_default();
    all.push((path.to_owned(), count(added), count(removed)));
  }
  all
}

/// The lines of a file that git does not track yet, all of them added: none for a file that is
/// not text or that cannot be read.
fn lines(file: &Path) -> u32 {
  let text = fs::read_to_string(file).unwrap_or_default();
  u32::try_from(text.lines().count()).unwrap_or(u32::MAX)
}
