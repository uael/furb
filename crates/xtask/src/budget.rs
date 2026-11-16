//! The limit of time of a part of the suite. The run of its command fails when it passes the limit
//! of that part, which `crates/xtask/budgets.toml` holds: the table `local` for this machine and
//! `ci` for a runner of the gates.

use std::{
  env, fs, hint,
  io::{self, BufRead, BufReader, Write},
  num::NonZero,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode},
  thread,
  time::{Instant, SystemTime, UNIX_EPOCH},
};

use fancy_regex::{Regex, escape};
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use nix::sys::{
  resource::{UsageWho, getrusage},
  time::TimeVal,
};

use crate::Fault;

/// Where and how a part runs: the limits that it reads, the load at its start, and the log of the
/// guard of memory.
struct Machine {
  /// The file of the limits.
  home: PathBuf,
  /// The table of the limits, `local` or `ci`.
  table: &'static str,
  /// The load average of the last minute, and the cores of the machine.
  load: f64,
  cores: f64,
  /// The log of the guard of memory: a line for each process that it ended, as its time in
  /// seconds of the epoch, its pid and its command. It is the one evidence that the guard ended a
  /// process, and not the kernel, a timeout or a crash, which end it by the same signal.
  log: PathBuf,
  /// The folder of the run, under which a program of the run stands.
  folder: PathBuf,
}

/// The limits of a part, in seconds of wall and of CPU, and the CPU seconds of the probe of the
/// speed on this machine when it was quiet; none is 0.
struct Limits {
  wall: f64,
  cpu: f64,
  quiet: f64,
}

impl Limits {
  /// The first line of a verdict: the times of the run against the limits that it has.
  fn heading(&self, part: &str, took: f64, worked: Option<f64>) -> String {
    let mut pieces = vec![format!("budget: {part} took {took:.0} s")];
    if self.wall > 0.0 {
      pieces.push(format!(" of {} s", self.wall));
    }
    if let Some(worked) = worked {
      pieces.push(format!(" and {worked:.0} s of CPU"));
      if self.cpu > 0.0 {
        pieces.push(format!(" of {} s", self.cpu));
      }
    }
    if self.wall <= 0.0 && self.cpu <= 0.0 {
      pieces.push(", with no limit yet".to_owned());
    }
    pieces.concat()
  }
}

/// What a run that ended well took: its wall and CPU seconds, its lines, and its limits.
struct Ran {
  took: f64,
  worked: f64,
  lines: Vec<String>,
  limits: Limits,
}

/// What a run came to: the code to exit with, and the lines that judge it.
struct Verdict {
  code: u8,
  said: Vec<String>,
}

pub(crate) fn check(root: &Path, part: &str, command: &[String]) -> Result<ExitCode, Fault> {
  let verdict = Machine::this(root)?.run(part, command)?;
  for line in &verdict.said {
    println!("{line}");
  }
  Ok(ExitCode::from(verdict.code))
}

impl Machine {
  /// This machine, as its environment says: CI picks the table of a runner, `FURB_BUDGETS` moves
  /// the limits, `FURB_BUDGET_LOAD` gives the load and `FURB_WATCH_LOG` moves the log.
  fn this(root: &Path) -> Result<Self, Fault> {
    let home = env::var_os("FURB_BUDGETS")
      .map_or_else(|| root.join("crates/xtask/budgets.toml"), PathBuf::from);
    let log = env::var_os("FURB_WATCH_LOG")
      .map_or_else(|| env::temp_dir().join("furb-memwatch.log"), PathBuf::from);
    let load =
      env::var("FURB_BUDGET_LOAD").ok().and_then(|one| one.parse().ok()).unwrap_or_else(load);
    let cores = thread::available_parallelism()
      .map_or(1.0, |one| f64::from(u32::try_from(one.get()).unwrap_or(1)));
    let folder =
      env::current_dir().map_err(|source| Fault::Read { path: PathBuf::from("."), source })?;
    let table = if env::var_os("CI").is_some() { "ci" } else { "local" };
    Ok(Self { home, table, load, cores, log, folder })
  }

  /// The run of the command of a part, judged by its limits: by its wall time, or by its CPU time
  /// when the load at its start was over the cores. A command that fails gives its own code.
  fn run(&self, part: &str, command: &[String]) -> Result<Verdict, Fault> {
    let limits = self.limits(part)?;
    let first = probed();
    let (began, start, used) = (epoch(), Instant::now(), cpu(Of::Children));
    let (status, lines) = streamed(command)?;
    let took = start.elapsed().as_secs_f64();
    let worked = cpu(Of::Children).zip(used).map(|(now, before)| now - before);
    let speed = first.zip(probed()).map(|(one, other)| f64::midpoint(one, other));
    // With no count of CPU, as on Windows, the wall time alone judges.
    let mut limits = Limits { cpu: if worked.is_some() { limits.cpu } else { 0.0 }, ..limits };
    let mut scaled = None;
    if let Some(probe) = speed.filter(|_| limits.cpu > 0.0 && limits.quiet > 0.0) {
      scaled = Some(format!(
        "budget: the probe of the speed took {:.0} ms of CPU, against {:.0} ms on this machine when quiet, so the CPU limit of {part} is {:.0} s.",
        probe * 1000.0,
        limits.quiet * 1000.0,
        limits.cpu * probe / limits.quiet
      ));
      limits.cpu *= probe / limits.quiet;
    }
    let mut said = vec![limits.heading(part, took, worked)];
    said.extend(scaled);
    if status != Some(0) {
      let ended = self.watched(began, epoch(), &lines.join("\n"))?;
      if !ended.is_empty() {
        said.push(format!(
          "budget: the guard of memory ended a test process of {part}, by its log {}:",
          self.log.display()
        ));
        said.extend(ended.into_iter().map(|one| format!("  {one}")));
      }
      let code = status.and_then(|code| u8::try_from(code).ok()).filter(|code| *code != 0);
      return Ok(Verdict { code: code.unwrap_or(1), said });
    }
    let ran = Ran { took, worked: worked.unwrap_or(0.0), lines, limits };
    let code = self.judged(part, &ran, &mut said)?;
    Ok(Verdict { code, said })
  }

  /// The limits of a part in its table, none when the table holds none.
  fn limits(&self, part: &str) -> Result<Limits, Fault> {
    let text = fs::read_to_string(&self.home)
      .map_err(|source| Fault::Read { path: self.home.clone(), source })?;
    let table =
      toml::from_str::<toml::Table>(&text).ok().and_then(|all| all.get(self.table).cloned());
    let limit = |key: &str| {
      let seconds = table.as_ref().and_then(|one| one.get(key)).and_then(toml::Value::as_integer);
      seconds.and_then(|one| u32::try_from(one).ok()).map_or(0.0, f64::from)
    };
    let quiet = limit("probe_ms") / 1000.0;
    Ok(Limits { wall: limit(part), cpu: limit(&format!("{part}_cpu")), quiet })
  }

  /// Whether a run that ended well passed its limits, with what says so: a run past its CPU limit,
  /// which the speed of the machine during the run scales, fails whatever the load, and a run past
  /// its wall limit fails unless the load at its start was over the cores and its CPU time fits.
  fn judged(&self, part: &str, ran: &Ran, said: &mut Vec<String>) -> Result<u8, Fault> {
    let over_cpu = ran.limits.cpu > 0.0 && ran.worked > ran.limits.cpu;
    let mut over_wall = ran.limits.wall > 0.0 && ran.took > ran.limits.wall;
    if over_wall && ran.limits.cpu > 0.0 && !over_cpu && self.load > self.cores {
      said.push(format!(
        "budget: {part} passed its wall limit while the load was {:.0} on {:.0} cores, and its CPU time fits.",
        self.load, self.cores
      ));
      over_wall = false;
    }
    if !(over_cpu || over_wall) {
      return Ok(0);
    }
    if over_cpu {
      said.push(format!(
        "budget: {part} used {:.0} s of CPU, {:.0} s over its limit.",
        ran.worked,
        ran.worked - ran.limits.cpu
      ));
    }
    if over_wall {
      said.push(format!(
        "budget: {part} passed its wall limit by {:.0} s.",
        ran.took - ran.limits.wall
      ));
    }
    said
      .push("budget: make the run faster, and never raise a limit. Its slowest tests:".to_owned());
    let slow = slowest(&ran.lines)?;
    if slow.is_empty() {
      said.push("     0.00 s  none named: run the part with the times of its tests".to_owned());
    }
    said.extend(slow.into_iter().map(|(seconds, name)| format!("  {seconds:>7.2} s  {name}")));
    Ok(1)
  }

  /// The processes of this run that the log of the guard says it ended between two times: each
  /// whose program lies under the folder of the run, or whose absolute path the output of the run
  /// names as a whole word, as cargo names a test binary that a signal ended. A bare name, as bun,
  /// is no evidence.
  fn watched(&self, since: u64, until: u64, said: &str) -> Result<Vec<String>, Fault> {
    let Ok(text) = fs::read_to_string(&self.log) else { return Ok(Vec::new()) };
    let mut found = Vec::new();
    for line in text.lines() {
      let mut parts = line.splitn(3, ' ');
      let (when, pid, command) =
        (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
      let program = command.split(' ').next().unwrap_or("");
      let named = Path::new(program).is_absolute()
        && Regex::new(&format!(r"(?<![\w/.-]){}(?![\w/.-])", escape(program)))?.is_match(said)?;
      let ours = Path::new(program).starts_with(&self.folder) || named;
      if let Ok(when) = when.parse::<u64>()
        && since.saturating_sub(1) <= when
        && when <= until + 1
        && ours
      {
        found.push(format!("pid {pid}: {command}"));
      }
    }
    Ok(found)
  }
}

/// The run of a command whose output, its errors with it, goes to this output line by line, and
/// its code and lines; no code when a signal ended it.
fn streamed(command: &[String]) -> Result<(Option<i32>, Vec<String>), Fault> {
  let program = command.first().map_or("", String::as_str);
  let failed = |source| Fault::Spawn { program: program.to_owned(), source };
  let (reader, writer) = io::pipe().map_err(failed)?;
  let mut child = Process::new(program)
    .args(command.iter().skip(1))
    .stdout(writer.try_clone().map_err(failed)?)
    .stderr(writer)
    .spawn()
    .map_err(failed)?;
  let mut lines = Vec::new();
  let mut out = io::stdout();
  for line in BufReader::new(reader).split(b'\n') {
    let line = String::from_utf8_lossy(&line.map_err(failed)?).into_owned();
    writeln!(out, "{line}").map_err(failed)?;
    lines.push(line);
  }
  let status = child.wait().map_err(failed)?;
  Ok((status.code(), lines))
}

/// The tests of a run with their times, the slowest ten first: pytest gives `12.34s call
/// path::name`, cargo nextest `PASS [   1.234s] (  1/9) crate path::name`, and again at its end
/// for each test that was slow or failed.
fn slowest(lines: &[String]) -> Result<Vec<(f64, String)>, Fault> {
  let time = Regex::new(
    r"^\s*(?P<lead>\d+\.\d+)s\s+\w+\s+(?P<a>\S+)|^\s*[A-Z]+ +\[ *(?P<trail>\d+\.\d+)s\] +\( *\d+/\d+\) (?P<b>\S+ \S+)",
  )?;
  let mut found = Vec::new();
  for line in lines {
    if let Some(hit) = time.captures(line)? {
      let seconds = hit.name("lead").or_else(|| hit.name("trail")).map_or("0", |one| one.as_str());
      let name = hit.name("a").or_else(|| hit.name("b")).map_or("", |one| one.as_str());
      found.push((seconds.parse::<f64>().unwrap_or(0.0), name.to_owned()));
    }
  }
  found.sort_by(|one, other| other.0.total_cmp(&one.0).then_with(|| other.1.cmp(&one.1)));
  found.dedup();
  found.truncate(10);
  Ok(found)
}

fn epoch() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// Whose CPU time a count is: this process, or its waited children.
#[derive(Clone, Copy)]
enum Of {
  Own,
  Children,
}

/// The CPU seconds that this process, or its waited children, have used.
#[cfg(unix)]
fn cpu(of: Of) -> Option<f64> {
  let who = match of {
    Of::Own => UsageWho::RUSAGE_SELF,
    Of::Children => UsageWho::RUSAGE_CHILDREN,
  };
  let used = getrusage(who).ok()?;
  let seconds = |one: TimeVal| {
    let micros = u32::try_from(one.tv_usec()).unwrap_or(0);
    Duration::new(u64::try_from(one.tv_sec()).unwrap_or(0), micros * 1000).as_secs_f64()
  };
  Some(seconds(used.user_time()) + seconds(used.system_time()))
}

/// Windows counts no CPU time of waited children, so a run there is judged by its wall time.
#[cfg(not(unix))]
fn cpu(_: Of) -> Option<f64> {
  None
}

/// The steps of the fixed work of the probe on each core, and the entries of the table that it
/// walks.
const PROBE: u64 = 1_000_000;
const TABLE: u32 = 1 << 20;

/// The CPU seconds of a fixed work on each core at once. The same work costs more CPU seconds when
/// the load moves it to slower cores, as the cores of efficiency of a Mac, so the probe measures
/// the speed that the machine gives a run, which scales its CPU limit.
fn probed() -> Option<f64> {
  let cores = thread::available_parallelism().map_or(1, NonZero::get);
  let before = cpu(Of::Own)?;
  thread::scope(|scope| {
    for _ in 0..cores {
      scope.spawn(|| {
        // A walk of a table larger than the caches of a core, which waits on memory and on the
        // branches as the work of an interpreter does, and so slows as that work does on a core
        // that is slower or shared.
        let table: Vec<u32> =
          (0..TABLE).map(|at| (at.wrapping_mul(5).wrapping_add(1)) % TABLE).collect();
        (0..PROBE)
          .fold(0_u32, |at, _| hint::black_box(table.get(at as usize).copied().unwrap_or_default()))
      });
    }
  });
  Some(cpu(Of::Own)? - before)
}

/// The load average of the last minute: Linux gives it in a file and macOS by sysctl; no other
/// system gives one here.
fn load() -> f64 {
  let text = fs::read_to_string("/proc/loadavg").ok().or_else(|| {
    let out = Process::new("sysctl").args(["-n", "vm.loadavg"]).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).replace(['{', '}'], ""))
  });
  text.and_then(|one| one.split_whitespace().next()?.parse().ok()).unwrap_or(0.0)
}
