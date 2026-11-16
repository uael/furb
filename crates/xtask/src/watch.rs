//! The task `watch`, which checks the head of a branch whole on this machine each time it moved.
//!
//! The check runs in a tree of its own, detached at the head that it checks, so no edit of a
//! person or of an agent changes the revision under it, and it runs at a low priority, so it slows
//! no other work. It blocks nothing: a green writes one line in its table, a red one entry in its
//! inbox, with the tests that failed, the owner of each, and the commits since the last green, or
//! since the head checked before.

use std::{
  fmt::Write as _,
  fs,
  io::Write as _,
  num::NonZero,
  path::{Path, PathBuf},
  process::{self, Command as Process, ExitCode, Stdio},
  sync::mpsc::{self, RecvTimeoutError},
  thread,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use notify::{RecursiveMode, Watcher as _};

use crate::{Fault, git};

/// What the task takes, from its options.
pub(crate) struct Watch {
  pub(crate) branch: String,
  pub(crate) tree: PathBuf,
  pub(crate) every: Duration,
  pub(crate) table: PathBuf,
  pub(crate) inbox: PathBuf,
  pub(crate) owners: Option<PathBuf>,
  pub(crate) pid: Option<PathBuf>,
  pub(crate) once: bool,
}

/// The parts of a check, cheap first: the hooks of a commit on the files that changed since the
/// head checked before, which take seconds and which a commit with --no-verify skipped, then each
/// suite, with the files that the published crates read against their packs once rust built
/// them, then the two hooks of the manual stage that are no suite, then clippy on Linux and
/// Windows, which this machine does not build. They run one after the other,
/// since the suites at once took 137 processes and moved the load of 14 cores from 5 to 18.
fn parts(before: Option<&str>, head: &str) -> Vec<(&'static str, Vec<String>)> {
  let words = |all: &[&str]| all.iter().map(|one| (*one).to_owned()).collect::<Vec<_>>();
  let hook = ["sh", "script/hooks/pre-commit", "run"];
  let changed = before.map_or_else(
    || words(&["--all-files"]),
    |before| words(&["--from-ref", before, "--to-ref", head]),
  );
  vec![
    ("commit", [words(&hook), changed].concat()),
    ("rust", words(&["cargo", "xtask", "test", "rust"])),
    ("pack", words(&["cargo", "xtask", "pack"])),
    ("python", words(&["cargo", "xtask", "test", "python"])),
    ("typescript", words(&["cargo", "xtask", "test", "typescript"])),
    ("clippy", words(&[&hook[..], &["clippy", "--hook-stage", "manual", "--all-files"]].concat())),
    ("biome", words(&[&hook[..], &["biome", "--hook-stage", "manual", "--all-files"]].concat())),
    ("targets", words(&["cargo", "xtask", "targets"])),
  ]
}

impl Watch {
  /// The watch reads the branch and its commits through its tree, a worktree of the clone, so it
  /// runs from any folder.
  pub(crate) fn run(&self) -> Result<ExitCode, Fault> {
    if let Some(pid) = &self.pid {
      held(pid)?;
    }
    // A move of the branch writes its ref in this folder, so the watch wakes on the write itself.
    let common = git(&self.tree, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
    let common = PathBuf::from(common.trim());
    let refs = Path::new(&self.branch).parent().map_or_else(
      || common.join("refs").join("heads"),
      |folder| common.join("refs").join("heads").join(folder),
    );
    let (sender, moves) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| drop(sender.send(event)))?;
    watcher.watch(&refs, RecursiveMode::NonRecursive)?;
    let mut next = Instant::now();
    loop {
      let head = self.head()?;
      if last(&self.table).is_some_and(|checked| checked == head) {
        if self.once {
          return Ok(ExitCode::SUCCESS);
        }
        heard(Ok(moves.recv().map_err(|_ended| Fault::Unwatched)?))?;
        continue;
      }
      // The checks start at most once in each interval: the moves that come before its end wait for
      // it, and the check takes the head that stands then. The loop reads the head again after a
      // check, since the waits of the check take the event of a move that comes during it.
      while let Some(left) = next.checked_duration_since(Instant::now()) {
        heard(moves.recv_timeout(left))?;
      }
      next = Instant::now() + self.every;
      let head = self.head()?;
      // A check that cannot run, as a part that a move of a folder left with no place to start,
      // is a red of its head: the watch tells it and goes on to the next head, and never ends on
      // it.
      if let Err(fault) = self.check(&head, &moves) {
        append(&self.table, &format!("{}\t{head}\tred\t0\t1\n", now()))?;
        self.tell(&format!("the check of {head} could not run: {fault}"))?;
      }
      if self.once {
        return Ok(ExitCode::SUCCESS);
      }
    }
  }

  fn head(&self) -> Result<String, Fault> {
    Ok(git(&self.tree, &["rev-parse", "--short=9", &self.branch])?.trim().to_owned())
  }

  /// One check of a head: the tree goes to it, each part runs, and the result is written.
  fn check(&self, head: &str, moves: &Moves) -> Result<(), Fault> {
    let tree = self.tree.as_path();
    if !git(tree, &["status", "--porcelain"])?.trim().is_empty() {
      return self
        .tell(&format!("the tree {} holds changes, so {head} was not checked", tree.display()));
    }
    git(tree, &["checkout", "--quiet", "--detach", head])?;
    let began = Instant::now();
    let mut failed = Vec::new();
    let before = last(&self.table);
    for (name, words) in parts(before.as_deref(), head) {
      calm(moves)?;
      let log = tree.join("target").join("xtask").join(format!("watch-{name}.log"));
      if !part(tree, &words, &log)? {
        failed.extend(named(tree, name, &log));
      }
    }
    // A hook that formats writes its fix in the tree, which the next check would refuse as a change
    // of a person: the failure names it, and the tree goes back to the head.
    git(tree, &["checkout", "--quiet", "--", "."])?;
    let seconds = began.elapsed().as_secs();
    let state = if failed.is_empty() { "green" } else { "red" };
    let line = format!("{}\t{head}\t{state}\t{seconds}\t{}\n", now(), failed.len());
    append(&self.table, &line)?;
    if failed.is_empty() {
      return Ok(());
    }
    // The range starts at the last green, or at the head checked before when none was green, or
    // 20 commits back at the first check.
    let since = green(&self.table, head).or(before);
    let range =
      since.map_or_else(|| format!("{head}~20..{head}"), |since| format!("{since}..{head}"));
    let commits = git(tree, &["log", "--oneline", "--no-decorate", &range]).unwrap_or_default();
    let owners = self.owners.as_deref().map(owners).unwrap_or_default();
    let mut entry = format!("RED of the carrier at {head}, {seconds} s, commits {range}:\n");
    for test in &failed {
      let owner = owners.iter().find(|(prefix, _)| test.starts_with(prefix.as_str()));
      let owner = owner.map_or("no owner", |one| one.1.as_str());
      writeln!(entry, "- {test}: {owner}").unwrap_or_default();
    }
    entry.push_str("```\n");
    entry.push_str(&commits);
    entry.push_str("```\n");
    self.tell(&entry)
  }

  fn tell(&self, text: &str) -> Result<(), Fault> {
    append(&self.inbox, &format!("\n## {} WATCH {text}\n", stamp()))
  }
}

/// Whether a part passed, run at a low priority into its log.
fn part(tree: &Path, words: &[String], log: &Path) -> Result<bool, Fault> {
  let folder = log.parent().unwrap_or(tree);
  fs::create_dir_all(folder).map_err(|source| Fault::Read { path: folder.to_owned(), source })?;
  let file =
    fs::File::create(log).map_err(|source| Fault::Read { path: log.to_owned(), source })?;
  let out = || {
    file.try_clone().map(Stdio::from).map_err(|source| Fault::Read { path: log.to_owned(), source })
  };
  let Some((program, args)) = words.split_first() else { return Ok(true) };
  let status = Process::new(program)
    .args(args)
    .current_dir(tree)
    .stdout(out()?)
    .stderr(out()?)
    .status()
    .map_err(|source| Fault::Spawn { program: program.clone(), source })?;
  Ok(status.success())
}

/// The tests that a failed part names, by the lines of each runner, or the part itself when no
/// line names a test, as a build that fails.
fn named(tree: &Path, name: &str, log: &Path) -> Vec<String> {
  let text = fs::read_to_string(log).unwrap_or_default();
  let mut found: Vec<String> = Vec::new();
  if name == "rust" {
    for line in text.lines().map(str::trim_start) {
      if let Some(rest) = line.strip_prefix("FAIL ").or_else(|| line.strip_prefix("TIMEOUT ")) {
        found.extend(rest.split_once(") ").map(|(_, test)| test.trim().to_owned()));
      }
    }
  } else if name == "python" {
    for line in text.lines() {
      found.extend(
        line
          .strip_prefix("FAILED ")
          .map(|rest| rest.split(" - ").next().unwrap_or(rest).to_owned()),
      );
    }
  } else if name == "typescript" {
    for line in text.lines() {
      if let Some((_, files)) = line.split_once(" runs failed: ") {
        found.extend(files.split(", ").map(|file| file.trim_start_matches("./").to_owned()));
      }
    }
  } else {
    // A failed hook is named by each file that it names under its line, so the owner of the folder
    // of that file reads it, and by itself when it names none.
    let mut failed: Option<(String, bool)> = None;
    for line in text.lines().chain(["end of the hooks Passed"]) {
      if ["Passed", "Failed", "Skipped"].iter().any(|end| line.ends_with(end)) {
        if let Some((hook, false)) = failed.take() {
          found.push(format!("hook {hook}"));
        }
        let hook = line.strip_suffix("Failed").map(|hook| hook.trim_end_matches('.').trim());
        failed = hook.map(|hook| (hook.to_owned(), false));
      } else if let Some((hook, named)) = &mut failed {
        // A checker names a file as `path:line`, ty after `--> `, rustfmt after `Diff in ` and from
        // the root of the file system.
        let named_file = line.trim().trim_start_matches("--> ").trim_start_matches("Diff in ");
        let path = named_file.split(':').next().unwrap_or_default();
        let path =
          Path::new(path).strip_prefix(tree).map_or(path, |inside| inside.to_str().unwrap_or(path));
        if !path.is_empty() && tree.join(path).is_file() {
          found.push(format!("{path} (hook {hook})"));
          *named = true;
        }
      }
    }
  }
  // A suite past its limits fails with each test green: it is named by its budget, which the owner
  // of the suite reads.
  let over =
    |line: &str| line.ends_with("over its limit.") || line.contains("passed its wall limit by");
  for line in text.lines().filter(|line| line.starts_with("budget: ") && over(line)) {
    found.push(format!("budget of {name}: {}", line.trim_start_matches("budget: ")));
  }
  found.sort();
  found.dedup();
  if found.is_empty() {
    found.push(format!("{name} (no test named: {})", log.display()));
  }
  found
}

/// The events of the refs folder, by which the watch wakes when the branch moves.
type Moves = mpsc::Receiver<notify::Result<notify::Event>>;

/// An event of the refs folder, or the fault that ended their watch.
fn heard(event: Result<notify::Result<notify::Event>, RecvTimeoutError>) -> Result<(), Fault> {
  match event {
    Err(RecvTimeoutError::Disconnected) => Err(Fault::Unwatched),
    Ok(Err(fault)) => Err(fault.into()),
    Ok(Ok(_)) | Err(RecvTimeoutError::Timeout) => Ok(()),
  }
}

/// Wait while the load of the machine is at its count of cores or more, so a part never adds to a
/// machine that is already full. The load has no event, so it is read again each minute.
fn calm(moves: &Moves) -> Result<(), Fault> {
  let cores = thread::available_parallelism().map_or(1, NonZero::get);
  #[expect(clippy::cast_precision_loss, reason = "a count of cores is far below 2^52")]
  while load().is_some_and(|load| load >= cores as f64) {
    heard(moves.recv_timeout(Duration::from_secs(60)))?;
  }
  Ok(())
}

/// The load of the last minute, from Linux or from the sysctl of macOS; none on Windows.
fn load() -> Option<f64> {
  let text = fs::read_to_string("/proc/loadavg").ok().or_else(|| {
    let out = Process::new("sysctl").args(["-n", "vm.loadavg"]).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
  })?;
  text.split_whitespace().find_map(|word| word.parse().ok())
}

/// The owner of each prefix of a test, from a file of lines `<prefix> <owner>`.
fn owners(path: &Path) -> Vec<(String, String)> {
  let text = fs::read_to_string(path).unwrap_or_default();
  let pairs = text.lines().filter(|line| !line.starts_with('#'));
  pairs
    .filter_map(|line| line.split_once(char::is_whitespace))
    .map(|(a, b)| (a.to_owned(), b.trim().to_owned()))
    .collect()
}

/// The head of the last check that the table holds.
fn last(table: &Path) -> Option<String> {
  let text = fs::read_to_string(table).ok()?;
  text.lines().last().and_then(|line| line.split('\t').nth(1)).map(str::to_owned)
}

/// The head of the last green check of the table before this one.
fn green(table: &Path, head: &str) -> Option<String> {
  let text = fs::read_to_string(table).ok()?;
  let mut rows = text.lines().map(|line| line.split('\t').collect::<Vec<_>>());
  rows
    .rfind(|row| row.get(2) == Some(&"green") && row.get(1) != Some(&head))
    .and_then(|row| row.get(1).map(|one| (*one).to_owned()))
}

fn append(path: &Path, text: &str) -> Result<(), Fault> {
  let mut file = fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(path)
    .map_err(|source| Fault::Read { path: path.to_owned(), source })?;
  file.write_all(text.as_bytes()).map_err(|source| Fault::Read { path: path.to_owned(), source })
}

/// Refuse to start while the process that the pid file names lives, then write this pid there.
fn held(pid: &Path) -> Result<(), Fault> {
  if let Ok(text) = fs::read_to_string(pid)
    && let Ok(number) = text.trim().parse::<u32>()
    && Process::new("kill")
      .args(["-0", &number.to_string()])
      .stderr(Stdio::null())
      .status()
      .is_ok_and(|status| status.success())
  {
    return Err(Fault::Running { pid: number, file: pid.to_owned() });
  }
  fs::write(pid, process::id().to_string())
    .map_err(|source| Fault::Read { path: pid.to_owned(), source })
}

fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

fn stamp() -> String {
  Process::new("date")
    .arg("+%m-%d %H:%M")
    .output()
    .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    .unwrap_or_default()
}

#[cfg(test)]
#[path = "watch.test.rs"]
mod test;
