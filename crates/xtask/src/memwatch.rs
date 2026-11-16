//! The one guard of memory. No step of the work waits on memory: a hook, a build and a test run
//! start at once, since a wait on memory stalled them while the machine had room. This watcher runs
//! beside the work instead, and while the memory is tight it ends the largest test process: a test
//! binary of cargo, a pytest or a bun test.
//!
//! It ends one only when the work can be the cause, since a tight memory that the work does not
//! hold is the memory of other work, and each process that it ends is a run lost. The work can be
//! the cause when that process holds `FURB_WATCH_MIN` KB, 4 GB, as a test that runs away does, or
//! when the work holds `FURB_WATCH_LANE` KB, 8 GB. The work is each test process and each process
//! of this user whose command names a tree of the clone of the folder of the watch, as git worktree
//! list gives the trees. A daemon of furb is no part of the work, since no end of a test process
//! frees its memory. It ends no other process.
#![expect(clippy::string_slice, reason = "debt: a slice of a string that can cut a character")]

use std::{
  env, fs,
  io::Write,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode},
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use fancy_regex::Regex;

use crate::Fault;

/// The limits of the guard, from the environment: an empty value takes the default.
struct Guard {
  /// On Linux, the memory is tight under this share of available memory, in percent.
  floor: u64,
  /// The wait between two looks, and after an end, so the memory and the level of the kernel
  /// settle before the next look.
  poll: Duration,
  after: Duration,
  /// The size of a test process, and of the work, in KB, from which the work can be the cause.
  min: u64,
  cap: u64,
  /// The log of each process that it ends, as its time in seconds of the epoch, its pid and its
  /// command, so that `cargo xtask budget` names a kill as one of the guard only from it.
  log: PathBuf,
}

/// What a look at the processes gives: the memory of the work, and the largest test process.
struct Census {
  work: u64,
  largest: Option<(String, u64, String)>,
}

pub(crate) fn watch(once: bool) -> Result<ExitCode, Fault> {
  let guard = Guard::from_env();
  let folder =
    env::current_dir().map_err(|source| Fault::Read { path: PathBuf::from("."), source })?;
  let trees = trees(&folder);
  let mut said = false;
  loop {
    let ended = guard.look(&trees, &mut said)?;
    if once {
      return Ok(ExitCode::SUCCESS);
    }
    rest(if ended { guard.after } else { guard.poll });
  }
}

/// The wait to the next look: the system says no event of its memory, so the guard looks again.
#[expect(
  clippy::disallowed_methods,
  reason = "the guard looks at the memory each poll, as no event of the system says it"
)]
fn rest(wait: Duration) {
  thread::sleep(wait);
}

impl Guard {
  fn from_env() -> Self {
    let number = |name: &str, default: f64| {
      env::var(name)
        .ok()
        .filter(|one| !one.is_empty())
        .and_then(|one| one.parse::<f64>().ok())
        .unwrap_or(default)
    };
    let whole = |name: &str, default: u64| {
      env::var(name)
        .ok()
        .filter(|one| !one.is_empty())
        .and_then(|one| one.parse::<u64>().ok())
        .unwrap_or(default)
    };
    let log = env::var_os("FURB_WATCH_LOG")
      .filter(|one| !one.is_empty())
      .map_or_else(|| env::temp_dir().join("furb-memwatch.log"), PathBuf::from);
    Self {
      floor: whole("FURB_WATCH_FREE", 20),
      poll: Duration::from_secs_f64(number("FURB_WATCH_POLL", 2.0)),
      after: Duration::from_secs_f64(number("FURB_WATCH_AFTER", 30.0)),
      min: whole("FURB_WATCH_MIN", 4_194_304),
      cap: whole("FURB_WATCH_LANE", 8_388_608),
      log,
    }
  }

  /// One look: when the memory is tight and the work can be the cause, the end of the largest test
  /// process, and whether it ended one. A state that it said once is not said again until it moves.
  fn look(&self, trees: &[String], said: &mut bool) -> Result<bool, Fault> {
    let Some(strain) = self.tight() else {
      *said = false;
      return Ok(false);
    };
    let census = census(trees)?;
    let Some((pid, size, command)) = census.largest else {
      if !*said {
        println!("memwatch: {strain}, and no test process to end");
      }
      *said = true;
      return Ok(false);
    };
    if size < self.min && census.work < self.cap {
      if !*said {
        println!(
          "memwatch: {strain}, and the work is not the cause (it holds {} MB, its largest test process {} MB): none ended",
          census.work / 1024,
          size / 1024
        );
      }
      *said = true;
      return Ok(false);
    }
    if !Process::new("kill").args(["-9", &pid]).status().is_ok_and(|status| status.success()) {
      return Ok(false);
    }
    *said = false;
    println!("memwatch: {strain}, ended pid {pid} ({} MB): {command}", size / 1024);
    let when = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let failed = |source| Fault::Read { path: self.log.clone(), source };
    let mut log =
      fs::OpenOptions::new().create(true).append(true).open(&self.log).map_err(failed)?;
    writeln!(log, "{when} {pid} {command}").map_err(failed)?;
    Ok(true)
  }

  /// The reading that says the memory is tight, when it is: on macOS the pressure level of the
  /// kernel at warn or above, and on Linux a share of available memory under the floor. A machine
  /// that gives no reading is never tight.
  fn tight(&self) -> Option<String> {
    let level =
      Process::new("sysctl").args(["-n", "kern.memorystatus_vm_pressure_level"]).output().ok();
    let level = level
      .filter(|out| out.status.success())
      .and_then(|out| String::from_utf8_lossy(&out.stdout).trim().parse::<u64>().ok());
    if let Some(level) = level {
      return (level >= 2).then(|| format!("pressure level {level}"));
    }
    let info = fs::read_to_string("/proc/meminfo").ok()?;
    let field = |name: &str| {
      info
        .lines()
        .find_map(|line| line.strip_prefix(name)?.split_whitespace().next()?.parse::<u64>().ok())
    };
    let free = 100 * field("MemAvailable:")? / field("MemTotal:")?.max(1);
    (free < self.floor).then(|| format!("memory {free}% free"))
  }
}

/// Each tree of the clone of a folder, and the folder: a folder that is no clone gives itself
/// alone.
fn trees(folder: &Path) -> Vec<String> {
  let listed =
    Process::new("git").args(["worktree", "list", "--porcelain"]).current_dir(folder).output().ok();
  let text =
    listed.map(|out| String::from_utf8_lossy(&out.stdout).into_owned()).unwrap_or_default();
  let mut found: Vec<String> =
    text.lines().filter_map(|line| line.strip_prefix("worktree ")).map(str::to_owned).collect();
  found.push(
    fs::canonicalize(folder).unwrap_or_else(|_| folder.to_owned()).to_string_lossy().into_owned(),
  );
  found
}

/// The memory of the work, and the largest test process of this user that is no daemon of furb: a
/// test binary of cargo, a pytest and a bun test count as the program of their command, so a
/// linker, a shell or a hook that names one as an argument stays. The runner of python gives its
/// tests to workers whose command is the exec of a line of their input.
fn census(trees: &[String]) -> Result<Census, Fault> {
  let me = Process::new("id")
    .arg("-u")
    .output()
    .map_err(|source| Fault::Spawn { program: "id".to_owned(), source })?;
  let me = String::from_utf8_lossy(&me.stdout).trim().to_owned();
  let ps = Process::new("ps")
    .args(["-axo", "uid=,pid=,rss=,command="])
    .output()
    .map_err(|source| Fault::Spawn { program: "ps".to_owned(), source })?;
  counted(&me, &String::from_utf8_lossy(&ps.stdout), trees)
}

/// The census of the processes of a table of ps, as `uid pid rss command` on each line, that the
/// user `me` runs: the memory of the work, and the largest test process.
fn counted(me: &str, table: &str, trees: &[String]) -> Result<Census, Fault> {
  let binary = Regex::new(r"/target/.*/deps/[^/]+-[0-9a-f]+$")?;
  let python = Regex::new(r"(^|/)python[0-9.]*$")?;
  let host = Regex::new(r"[ /]furb host( |$)")?;
  let base = |word: &str, name: &str| word == name || word.ends_with(&format!("/{name}"));
  let mut census = Census { work: 0, largest: None };
  for line in table.lines() {
    let mut fields = line.split_whitespace();
    let (Some(uid), Some(pid), Some(rss)) = (fields.next(), fields.next(), fields.next()) else {
      continue;
    };
    let words: Vec<&str> = fields.collect();
    let command = words.join(" ");
    if uid != me || line.contains("memwatch") {
      continue;
    }
    let word = |n: usize| words.get(n).copied().unwrap_or("");
    let test = binary.is_match(word(0))?
      || base(word(0), "pytest")
      || python.is_match(word(0))?
        && (base(word(1), "pytest") || word(1) == "-m" && word(2) == "pytest")
      || base(word(0), "bun") && word(1) == "test"
      || line.contains("exec(eval(sys.stdin.readline");
    if host.is_match(line)? {
      continue;
    }
    let size = rss.parse::<u64>().unwrap_or(0);
    if test || names(&command, trees) {
      census.work += size;
    }
    if test && census.largest.as_ref().is_none_or(|(_, held, _)| size > *held) {
      census.largest = Some((pid.to_owned(), size, command.chars().take(150).collect()));
    }
  }
  Ok(census)
}

/// Whether a command names a tree as a whole path.
fn names(command: &str, trees: &[String]) -> bool {
  trees.iter().filter(|tree| !tree.is_empty()).any(|tree| {
    command
      .match_indices(tree.as_str())
      .any(|(at, _)| matches!(command[at + tree.len()..].chars().next(), None | Some('/' | ' ')))
  })
}
