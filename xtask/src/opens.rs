//! Whether each record opens on a build of furb, with no model and no network: the proof that a
//! build still opens the records that an earlier build opened.

use std::{
  collections::BTreeMap,
  env, fs,
  io::{self, Write},
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode, Stdio, id},
  time::{Duration, Instant},
};

use wait_timeout::ChildExt;

use crate::Fault;

/// How often the watch reads the memory of the open.
const LOOK: Duration = Duration::from_secs(2);
/// The most memory of an open, in MB, before the watch ends it.
const MOST: u64 = 8000;
/// An address where no proxy listens, so a fetch of the network fails at once.
const NOWHERE: &str = "http://127.0.0.1:9";

/// A run of the opens: the build, the table that it writes, the table of a run before, the list of
/// records, and the bound of one open.
#[derive(clap::Args)]
pub(crate) struct Opens {
  /// The program furb of the build to prove.
  pub(crate) furb: PathBuf,
  /// The table that the run writes.
  pub(crate) out: PathBuf,
  /// The table of a run before.
  pub(crate) before: Option<PathBuf>,
  /// A file that names the records, one path on each line.
  #[arg(long, value_name = "FILE")]
  pub(crate) list: Option<PathBuf>,
  /// The most seconds of one open.
  #[arg(long, default_value_t = 1800)]
  pub(crate) bound: u64,
}

impl Opens {
  /// Open a copy of each record, one at a time, write a line for each to the table, and fail when
  /// a record that opened before opens no more.
  pub(crate) fn run(&self, root: &Path) -> Result<ExitCode, Fault> {
    let mut table = Vec::new();
    // The open runs in a folder of its own, so a path of the build from here would name nothing.
    let furb = fs::canonicalize(&self.furb)
      .map_err(|source| Fault::Read { path: self.furb.clone(), source })?;
    for record in self.records(root)? {
      let (verdict, seconds, peak, said) = opened(&furb, &record, Duration::from_secs(self.bound))?;
      println!(
        "{} {verdict} {seconds}s {peak}MB",
        record.file_name().unwrap_or_default().to_string_lossy()
      );
      table.push(format!("{}\t{verdict}\t{seconds}\t{peak}\t{said}\n", record.display()));
      fs::write(&self.out, table.concat())
        .map_err(|source| Fault::Read { path: self.out.clone(), source })?;
    }
    let Some(before) = &self.before else { return Ok(ExitCode::SUCCESS) };
    let read = |path: &Path| -> Result<BTreeMap<String, String>, Fault> {
      let text =
        fs::read_to_string(path).map_err(|source| Fault::Read { path: path.to_owned(), source })?;
      Ok(
        text
          .lines()
          .filter_map(|line| line.split('\t').next().zip(line.split('\t').nth(1)))
          .map(|(r, v)| (r.to_owned(), v.to_owned()))
          .collect(),
      )
    };
    let (was, now) = (read(before)?, read(&self.out)?);
    let lost: Vec<String> = was
      .iter()
      .filter(|(record, verdict)| {
        *verdict == "opens" && now.get(*record).is_none_or(|one| one != "opens")
      })
      .map(|(record, _)| {
        format!(
          "opened before and not now: {record} ({})",
          now.get(record).map_or("", String::as_str)
        )
      })
      .collect();
    if lost.is_empty() {
      println!("each record that opened before opens now");
      return Ok(ExitCode::SUCCESS);
    }
    for one in &lost {
      println!("{one}");
    }
    Ok(ExitCode::FAILURE)
  }

  /// The records of the list, one path on each line, or else those of tests/corpus and
  /// tests/replay.
  fn records(&self, root: &Path) -> Result<Vec<PathBuf>, Fault> {
    if let Some(list) = &self.list {
      let text =
        fs::read_to_string(list).map_err(|source| Fault::Read { path: list.clone(), source })?;
      return Ok(text.lines().filter(|line| !line.trim().is_empty()).map(PathBuf::from).collect());
    }
    let mut found = Vec::new();
    for (folder, deep) in [("tests/corpus", true), ("tests/replay", false), ("tests/replay", true)]
    {
      let entries =
        fs::read_dir(root.join(folder)).into_iter().flatten().flatten().map(|one| one.path());
      let folders: Vec<PathBuf> =
        if deep { entries.filter(|one| one.is_dir()).collect() } else { vec![root.join(folder)] };
      for one in folders {
        let mut records: Vec<PathBuf> = fs::read_dir(&one)
          .into_iter()
          .flatten()
          .flatten()
          .map(|entry| entry.path())
          .filter(|path| path.extension().is_some_and(|kind| kind == "jsonl"))
          .collect();
        records.sort();
        found.extend(records);
      }
    }
    Ok(found)
  }
}

/// The open of a copy of one record with `furb turns`, which makes the life again from the record
/// and only inspects it: its verdict, its seconds, its most memory in MB, and the first line of
/// furb that it wrote.
fn opened(
  furb: &Path,
  record: &Path,
  bound: Duration,
) -> Result<(String, u64, u64, String), Fault> {
  let work = env::temp_dir().join(format!("opens.{}", id()));
  let failed = |source| Fault::Read { path: work.clone(), source };
  if work.exists() {
    fs::remove_dir_all(&work).map_err(failed)?;
  }
  fs::create_dir_all(work.join("cwd")).map_err(failed)?;
  let copy = work.join("record.jsonl");
  fs::copy(record, &copy).map_err(failed)?;
  let log = work.join("log");
  let written = fs::File::create(&log).map_err(failed)?;
  let start = Instant::now();
  let mut child = Process::new(furb)
    .arg("turns")
    .arg("--record")
    .arg(&copy)
    .arg("--cwd")
    .arg(work.join("cwd"))
    .current_dir(work.join("cwd"))
    .envs([("HTTP_PROXY", NOWHERE), ("HTTPS_PROXY", NOWHERE), ("ALL_PROXY", NOWHERE)])
    .stdin(Stdio::null())
    .stdout(written.try_clone().map_err(failed)?)
    .stderr(written)
    .spawn()
    .map_err(|source| Fault::Spawn { program: furb.display().to_string(), source })?;
  let (mut peak, mut verdict) = (0, None);
  let status = loop {
    if let Some(status) = child.wait_timeout(LOOK).map_err(failed)? {
      break Some(status);
    }
    let used = resident(child.id());
    peak = peak.max(used);
    verdict = if used > MOST {
      Some("memory")
    } else if start.elapsed() >= bound {
      Some("bound")
    } else {
      None
    };
    if verdict.is_some() {
      child.kill().map_err(failed)?;
      child.wait().map_err(failed)?;
      break None;
    }
  };
  let verdict = match (verdict, status) {
    (Some(verdict), _) => verdict.to_owned(),
    (None, Some(status)) if status.success() => "opens".to_owned(),
    (None, status) => format!("refused:{}", status.and_then(|one| one.code()).unwrap_or(1)),
  };
  let text = fs::read_to_string(&log).unwrap_or_default();
  let said = text
    .lines()
    .find(|line| line.starts_with("furb:"))
    .or_else(|| text.lines().last())
    .unwrap_or_default();
  let said: String = said.chars().filter(|one| *one != '\r' && *one != '\t').take(240).collect();
  fs::remove_dir_all(&work).map_err(failed)?;
  io::stdout().flush().map_err(failed)?;
  Ok((verdict, start.elapsed().as_secs(), peak, said))
}

/// The resident memory of a process in MB, as ps reads it, or 0 when it reads none.
fn resident(pid: u32) -> u64 {
  let out = Process::new("ps").args(["-o", "rss=", "-p", &pid.to_string()]).output();
  out
    .ok()
    .and_then(|out| String::from_utf8_lossy(&out.stdout).trim().parse::<u64>().ok())
    .map_or(0, |kb| kb / 1024)
}

#[cfg(all(test, unix))]
#[path = "opens.test.rs"]
mod test;
