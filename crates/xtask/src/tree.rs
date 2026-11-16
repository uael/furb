//! The life of a worktree of this clone: `new` makes it with a warm build, `land` moves a branch
//! to its head, and `close` removes it once nothing in it can be lost.

use std::{
  fs, io,
  path::{self, Path, PathBuf},
  process::{Command as Process, ExitCode, Stdio},
  thread,
  time::{Duration, SystemTime},
};

use clap::Subcommand;
#[cfg(target_vendor = "apple")]
use rustix::fs::{CloneFlags, fclonefileat};

use crate::{Fault, git};

/// The build folders that a new tree takes from the nearest tree: the target of cargo, the
/// packages of bun, the build of the TypeScript package and the configs of its tests.
const WARM: [&str; 5] = [
  "target",
  "node_modules",
  "crates/typescript/node_modules",
  "crates/typescript/dist",
  ".furb/tests",
];
/// How long git does nothing in a tree before `close` takes it as unused.
const IDLE: Duration = Duration::from_hours(2);

#[derive(Subcommand)]
pub(crate) enum Verb {
  /// Make a worktree whose build starts warm: a copy on write of the build of the tree of this
  /// clone whose commit stands nearest.
  ///
  /// A branch that stands in this clone or on origin is checked out, and a local branch that lags
  /// behind origin moves up to it; a new branch starts from the base. The tree then makes its own
  /// .venv with uv sync, and builds the TypeScript package on the warm target. A file system with
  /// no copy on write gives a plain tree, which builds from nothing.
  New {
    /// The folder of the new tree.
    path: PathBuf,
    /// The branch that the tree holds.
    branch: String,
    /// Where a new branch starts; origin/main when unsaid.
    #[arg(long)]
    base: Option<String>,
  },
  /// Rebase the head of this tree on a branch of the clone, then move that branch to the head.
  ///
  /// The move is a compare and swap of the ref, so two trees that land at once lose no commit. A
  /// conflict aborts the rebase and leaves the tree as it was.
  Land {
    /// The branch that takes the head, as w/integrate.
    branch: String,
  },
  /// Remove a worktree and its build once the removal can lose nothing, then its local branch.
  ///
  /// The tree holds no change, each of its commits stands on --into or on a remote, and, unless
  /// --own, no process stands in it and git did nothing in it for two hours.
  Close {
    /// The folder of the tree.
    path: PathBuf,
    /// The branch where each commit of the tree must stand, as w/integrate.
    #[arg(long, default_value = "origin/main")]
    into: String,
    /// The caller closes its own tree, which it alone uses.
    #[arg(long)]
    own: bool,
  },
}

/// Why `close` keeps a tree.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Kept {
  #[error("it holds changes:\n{0}")]
  Changed(String),
  #[error("{count} of its commits stand neither on {into} nor on a remote")]
  Unlanded { count: String, into: String },
  #[error("the processes {0} stand in it")]
  Used(String),
  #[error("git worked in it {0} minutes ago, under the two hours of an unused tree")]
  Recent(u64),
}

pub(crate) fn run(root: &Path, verb: Verb) -> Result<ExitCode, Fault> {
  match verb {
    Verb::New { path, branch, base } => new(root, &path, &branch, base.as_deref()),
    Verb::Land { branch } => land(root, &branch),
    Verb::Close { path, into, own } => close(root, &path, &into, own),
  }
  .map(|()| ExitCode::SUCCESS)
}

fn new(root: &Path, path: &Path, branch: &str, base: Option<&str>) -> Result<(), Fault> {
  let path =
    path::absolute(path).map_err(|source| Fault::Read { path: path.to_owned(), source })?;
  let main = checkout(root)?;
  fetched(&main)?;
  // The hooks print only what fails. The setting is shared by each tree of the clone, and one
  // that the owner set stays.
  if said(&main, &["config", "--get", "core.hooksPath"]).is_none() {
    git(&main, &["config", "core.hooksPath", "script/hooks"])?;
  }
  let at = path.to_string_lossy();
  let remote = format!("origin/{branch}");
  let local = said(&main, &["show-ref", "--verify", "-q", &format!("refs/heads/{branch}")]);
  let held = said(&main, &["show-ref", "--verify", "-q", &format!("refs/remotes/{remote}")]);
  match (&local, &held) {
    (Some(_), _) => git(&main, &["worktree", "add", "-q", &at, branch])?,
    (None, Some(_)) => git(&main, &["worktree", "add", "-q", "-b", branch, &at, &remote])?,
    (None, None) => {
      git(&main, &["worktree", "add", "-q", "-b", branch, &at, base.unwrap_or("origin/main")])?
    }
  };
  if local.is_some() || held.is_some() {
    if base.is_some() {
      println!("The branch {branch} stands already, so the base is not used.");
    }
    if held.is_some() {
      followed(&path, branch, &remote, local.is_some())?;
    }
  }
  warmed(&main, &path)?;
  println!("{} is ready on {branch}.", path.display());
  Ok(())
}

/// The folder of the main checkout of the clone, or its git folder when the clone is bare.
fn checkout(root: &Path) -> Result<PathBuf, Fault> {
  let common = git(root, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
  let common = PathBuf::from(common.trim());
  let bare =
    git(root, &["--git-dir", &common.to_string_lossy(), "rev-parse", "--is-bare-repository"])?;
  Ok(match common.parent() {
    Some(parent) if bare.trim() != "true" => parent.to_owned(),
    _ => common,
  })
}

/// A fetch of origin. Other workers fetch in the same clone, so git waits for a lock of a ref that
/// another fetch holds, up to 5 s, as its settings of the timeouts of a lock say.
fn fetched(main: &Path) -> Result<(), Fault> {
  let waits = ["-c", "core.filesRefLockTimeout=5000", "-c", "core.packedRefsTimeout=5000"];
  git(main, &[&waits[..], &["fetch", "-q", "origin"]].concat()).map(drop)
}

/// The branch of a tree follows origin, and a local branch that only lags behind it moves up.
fn followed(path: &Path, branch: &str, remote: &str, local: bool) -> Result<(), Fault> {
  git(path, &["branch", "-q", &format!("--set-upstream-to={remote}"), branch])?;
  if !local {
    return Ok(());
  }
  if said(path, &["merge-base", "--is-ancestor", "HEAD", remote]).is_some() {
    git(path, &["merge", "-q", "--ff-only", remote])?;
  } else if said(path, &["merge-base", "--is-ancestor", remote, "HEAD"]).is_none() {
    println!(
      "{branch} and {remote} each hold commits that the other lacks: look at both before you push."
    );
  }
  Ok(())
}

/// The build of the nearest tree, copied on write into the new tree, then the .venv and the
/// TypeScript package that the tree builds on it.
fn warmed(main: &Path, path: &Path) -> Result<(), Fault> {
  let source = nearest(main, path)?;
  if source != main {
    println!("The build is copied from {}.", source.display());
  }
  let natives = fs::read_dir(source.join("crates/typescript")).into_iter().flatten().flatten();
  let natives = natives.map(|entry| entry.file_name().to_string_lossy().into_owned());
  let natives = natives
    .filter(|name| {
      name == "index.cjs"
        || Path::new(name).extension().is_some_and(|one| one.eq_ignore_ascii_case("node"))
    })
    .map(|name| format!("crates/typescript/{name}"));
  for part in WARM.map(str::to_owned).into_iter().chain(natives) {
    let (from, to) = (source.join(&part), path.join(&part));
    // A source whose target is a link to a target that trees shared gives a copy of what the
    // link names, which becomes a target of this tree alone.
    let Ok(from) = fs::canonicalize(&from) else { continue };
    if to.exists() {
      continue;
    }
    let folder = to.parent().unwrap_or(path);
    fs::create_dir_all(folder).map_err(|source| Fault::Read { path: folder.to_owned(), source })?;
    if let Err(no) = clone(&from, &to) {
      println!(
        "This file system has no copy on write ({no}): {} builds from nothing.",
        path.display()
      );
      break;
    }
  }
  // The TypeScript package and the .venv build the crate in two folders of the target, so they
  // build at once. The .venv is the tree's own, made on the warm target, so its python imports
  // this tree.
  let (built, synced) = thread::scope(|scope| {
    let built = scope.spawn(|| {
      !path.join("node_modules").is_dir()
        || ran(path, "bun", &["install", "--frozen-lockfile", "--offline"])
          && ran(path, "bun", &["run", "build"])
    });
    let synced = scope.spawn(|| {
      path.join(".venv").exists()
        || !path.join("uv.lock").is_file()
        || ran(path, "uv", &["sync", "--locked"])
    });
    (built.join().unwrap_or(false), synced.join().unwrap_or(false))
  });
  if !built {
    println!("The TypeScript side did not build: run bun install and bun run build there.");
  }
  if !synced {
    println!("uv sync did not end there: run it before you trust a test of python.");
  }
  Ok(())
}

/// Of the main checkout and each tree of the clone where uv built the crate into a .venv, the one
/// whose commit stands nearest to the commit of the new tree; the main checkout wins a tie.
fn nearest(main: &Path, path: &Path) -> Result<PathBuf, Fault> {
  let here = git(path, &["rev-parse", "HEAD"])?;
  let listed = git(main, &["worktree", "list", "--porcelain"])?;
  let trees = listed.lines().filter_map(|line| line.strip_prefix("worktree ")).map(PathBuf::from);
  let built = trees.filter(|tree| tree != path && tree.join("target").is_dir() && wheeled(tree));
  let far = |tree: &PathBuf| {
    let count = said(tree, &["rev-list", "--count", &format!("{}...HEAD", here.trim())])?;
    count.trim().parse::<u64>().ok()
  };
  let mut best: Option<(u64, PathBuf)> = None;
  for tree in built {
    if let Some(far) = far(&tree)
      && best.as_ref().is_none_or(|(least, _)| far < *least)
    {
      best = Some((far, tree));
    }
  }
  Ok(best.map_or_else(|| main.to_owned(), |(_, tree)| tree))
}

/// Whether the .venv of a tree holds the wheel of the crate, which uv sync built.
fn wheeled(tree: &Path) -> bool {
  let pythons = fs::read_dir(tree.join(".venv/lib")).into_iter().flatten().flatten();
  pythons
    .flat_map(|python| {
      fs::read_dir(python.path().join("site-packages")).into_iter().flatten().flatten()
    })
    .any(|entry| {
      let name = entry.file_name();
      let name = name.to_string_lossy();
      name.starts_with("furb_monty-") && name.ends_with(".dist-info")
    })
}

/// A copy on write of a file or of a whole folder in one call, which keeps the times that cargo
/// compares.
#[cfg(target_vendor = "apple")]
fn clone(from: &Path, to: &Path) -> io::Result<()> {
  let (Some(folder), Some(name)) = (to.parent(), to.file_name()) else {
    return Err(io::Error::other("no folder to copy into"));
  };
  let (source, into) = (fs::File::open(from)?, fs::File::open(folder)?);
  Ok(fclonefileat(&source, &into, name, CloneFlags::empty())?)
}

/// A copy on write of a file or of a folder, by a reflink of each file.
#[cfg(not(target_vendor = "apple"))]
fn clone(from: &Path, to: &Path) -> io::Result<()> {
  let mut copy = Process::new("cp");
  let copied = copy.args(["-a", "--reflink=always"]).arg(from).arg(to).stderr(Stdio::null());
  if copied.status()?.success() {
    return Ok(());
  }
  if to.is_dir() {
    fs::remove_dir_all(to)?;
  } else if to.exists() {
    fs::remove_file(to)?;
  }
  Err(io::Error::other("cp --reflink=always refused"))
}

fn land(tree: &Path, branch: &str) -> Result<(), Fault> {
  let changed = git(tree, &["status", "--porcelain", "--untracked-files=no"])?;
  if !changed.trim().is_empty() {
    return Err(Fault::Kept { path: tree.to_owned(), why: Kept::Changed(changed) });
  }
  let named = format!("refs/heads/{branch}");
  for _ in 0..5 {
    let carrier = git(tree, &["rev-parse", &named])?.trim().to_owned();
    if let Err(failed) = git(tree, &["rebase", "-q", &carrier]) {
      let unmerged = git(tree, &["diff", "--name-only", "--diff-filter=U"])?;
      // A rebase that git refused before its first step left nothing to abort, so the abort fails
      // there and the tree stands as it was.
      said(tree, &["rebase", "--abort"]);
      return Err(if unmerged.trim().is_empty() {
        failed
      } else {
        Fault::Conflict(branch.to_owned())
      });
    }
    let head = git(tree, &["rev-parse", "HEAD"])?.trim().to_owned();
    if head == carrier {
      println!("Nothing to land: the head holds no commit that {branch} lacks.");
      return Ok(());
    }
    if said(tree, &["update-ref", &named, &head, &carrier]).is_some() {
      let count = git(tree, &["rev-list", "--count", &format!("{carrier}..{head}")])?;
      println!("landed: {branch} is {} ({} commits)", &head[..9], count.trim());
      return Ok(());
    }
  }
  Err(Fault::Moved(branch.to_owned()))
}

fn close(root: &Path, path: &Path, into: &str, own: bool) -> Result<(), Fault> {
  let path =
    dunce::canonicalize(path).map_err(|source| Fault::Read { path: path.to_owned(), source })?;
  let kept = |why| Err(Fault::Kept { path: path.clone(), why });
  let changed = git(&path, &["status", "--porcelain"])?;
  if !changed.trim().is_empty() {
    return kept(Kept::Changed(changed));
  }
  let count = git(&path, &["rev-list", "--count", "HEAD", "--not", "--remotes", into])?;
  if count.trim() != "0" {
    return kept(Kept::Unlanded { count: count.trim().to_owned(), into: into.to_owned() });
  }
  if !own {
    if let Some(pids) = used(&path)? {
      return kept(Kept::Used(pids));
    }
    if let Some(minutes) = recent(&path)? {
      return kept(Kept::Recent(minutes));
    }
  }
  let branch = said(&path, &["symbolic-ref", "-q", "--short", "HEAD"]);
  let main = checkout(root)?;
  git(&main, &["worktree", "remove", &path.to_string_lossy()])?;
  if let Some(branch) = &branch {
    git(&main, &["branch", "-q", "-D", branch.trim()])?;
  }
  println!(
    "{} is closed, with its build{}.",
    path.display(),
    branch.map_or(String::new(), |one| format!(" and the branch {}", one.trim()))
  );
  Ok(())
}

/// The processes whose folder stands in the tree, as lsof names them. Windows has no lsof, and it
/// refuses to remove a folder that a process stands in, so there the removal of the tree fails.
fn used(path: &Path) -> Result<Option<String>, Fault> {
  if cfg!(windows) {
    return Ok(None);
  }
  // lsof ends with 1 when it cannot read a process of another user, and still names the others.
  let listed = Process::new("lsof")
    .args(["-d", "cwd", "-Fpn"])
    .stderr(Stdio::null())
    .output()
    .map_err(|source| Fault::Spawn { program: "lsof".to_owned(), source })?;
  let (mut pid, mut pids) = ("", Vec::new());
  let text = String::from_utf8_lossy(&listed.stdout);
  for line in text.lines() {
    if let Some(one) = line.strip_prefix('p') {
      pid = one;
    } else if let Some(folder) = line.strip_prefix('n')
      && Path::new(folder).starts_with(path)
    {
      pids.push(pid);
    }
  }
  Ok((!pids.is_empty()).then(|| pids.join(" ")))
}

/// The minutes since git last worked in the tree, when that is under two hours: a checkout, a
/// commit, an add and a rebase each write its index or its log.
fn recent(path: &Path) -> Result<Option<u64>, Fault> {
  let folder = PathBuf::from(git(path, &["rev-parse", "--absolute-git-dir"])?.trim());
  let last = ["index", "HEAD", "logs/HEAD"]
    .iter()
    .filter_map(|name| fs::metadata(folder.join(name)).and_then(|info| info.modified()).ok())
    .max();
  let since = last.and_then(|last| SystemTime::now().duration_since(last).ok());
  Ok(since.filter(|since| *since < IDLE).map(|since| since.as_secs() / 60))
}

/// What a git command printed, or nothing when it failed.
fn said(dir: &Path, args: &[&str]) -> Option<String> {
  let output =
    Process::new("git").args(args).current_dir(dir).stderr(Stdio::null()).output().ok()?;
  output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Whether a program ended well in the tree, with its output kept out of the way.
fn ran(dir: &Path, program: &str, args: &[&str]) -> bool {
  let mut run = Process::new(program);
  run.args(args).current_dir(dir).stdout(Stdio::null()).stderr(Stdio::null());
  run.status().is_ok_and(|status| status.success())
}
