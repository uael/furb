//! The check of the app on a real record, which `FURB_RECORD` names: `furb-gui --check` reads the whole record, and
//! the counts that it prints are those that `furb facts` shows of the same record, while the web view stays under
//! [`MOST`] megabytes. Run it on a copy of a record, after `bun run build` in `gui`:
//!
//! `FURB_RECORD=<copy> cargo test --features tauri/custom-protocol --test check -- --ignored --nocapture`
//!
//! The command `furb` comes from `FURB_CLI`, else from the target of the repository, else from PATH. The web view is
//! the WebContent of WebKit, a process of its own, so the test runs on macOS.

#![cfg(target_os = "macos")]

use std::{
  collections::HashSet,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  thread,
  time::Duration,
};

/// The most megabytes that the WebContent of the app may take on a record: its footprint, as Activity Monitor shows
/// it.
const MOST: f64 = 300.0;

/// The questions that the engine asks on the way to what an act does, which the app shows as no act.
const STEPS: [&str; 8] = ["stand", "clock", "chance", "gate", "merged", "run", "wants", "reply"];

/// The processes of WebContent that stand now.
fn webcontents() -> HashSet<u32> {
  let found = Command::new("pgrep").args(["-f", "com.apple.WebKit.WebContent"]).output();
  let text = found.map(|one| String::from_utf8_lossy(&one.stdout).into_owned()).unwrap_or_default();
  text.lines().filter_map(|line| line.trim().parse().ok()).collect()
}

/// The footprint of a process in megabytes, as `footprint` says it.
fn footprint(pid: u32) -> Option<f64> {
  let out = Command::new("footprint").args(["-p", &pid.to_string()]).output().ok()?;
  let text = String::from_utf8_lossy(&out.stdout);
  let said = text.split("Footprint: ").nth(1)?;
  let mut words = said.split_whitespace();
  let number: f64 = words.next()?.parse().ok()?;
  let scale = match words.next()? {
    "B" => 1.0 / 1024.0 / 1024.0,
    "KB" => 1.0 / 1024.0,
    "MB" => 1.0,
    "GB" => 1024.0,
    _ => return None,
  };
  Some(number * scale)
}

/// The command `furb` that prints the facts of a record.
fn furb() -> PathBuf {
  if let Ok(path) = std::env::var("FURB_CLI") {
    return path.into();
  }
  let built = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/furb");
  if built.is_file() { built } else { "furb".into() }
}

/// The words of a fact that `furb facts` prints, as `12: ['read', 'read3', 'rung4', 'chain1', ...]`: its first texts
/// in quotes, up to four.
fn words(line: &str) -> Vec<&str> {
  let mut rest = line.split_once(": [").map_or("", |(_, rest)| rest);
  let mut out = Vec::new();
  while out.len() < 4 {
    let Some(inner) = rest.strip_prefix('\'') else { break };
    let Some(end) = inner.find('\'') else { break };
    out.push(&inner[..end]);
    rest = inner[end + 1..].trim_start_matches(", ");
  }
  out
}

/// Whether a name is the name of an act of a kind: the kind, then a number.
fn named(name: &str, kind: &str) -> bool {
  name.strip_prefix(kind).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The chains, the threads and the acts of a record, as `furb facts` shows its questions. A chain is each chain that a
/// question makes or stands on, a thread is each prompt, and an act is each question that is no step of the engine,
/// each chain, and each rung that a question other than a prompt names as its maker, which the reader of the crate
/// makes when the record does not keep it.
fn counted(record: &Path) -> (usize, usize, usize) {
  let out =
    Command::new(furb()).args(["facts", "--record"]).arg(record).output().expect("furb facts runs");
  assert!(out.status.success(), "furb facts fails: {}", String::from_utf8_lossy(&out.stderr));
  let (mut chains, mut acts, mut threads) = (HashSet::new(), HashSet::new(), 0);
  for line in String::from_utf8_lossy(&out.stdout).lines() {
    let [kind, about, by, on, ..] = words(line)[..] else { continue };
    if !named(about, kind) {
      continue;
    }
    for chain in [about, on].into_iter().filter(|one| named(one, "chain")) {
      chains.insert(chain.to_owned());
      acts.insert(chain.to_owned());
    }
    if !STEPS.contains(&kind) {
      acts.insert(about.to_owned());
    }
    if kind != "prompt" && named(by, "rung") {
      acts.insert(by.to_owned());
    }
    threads += usize::from(kind == "prompt");
  }
  (chains.len(), threads, acts.len())
}

#[test]
#[ignore = "a check of the app on the real record that FURB_RECORD names, with a window"]
fn the_check_draws_the_whole_record_and_its_web_view_stays_light() {
  let Ok(record) = std::env::var("FURB_RECORD") else { return };
  let record = PathBuf::from(record);
  let before = webcontents();
  let mut app = Command::new(env!("CARGO_BIN_EXE_furb-gui"))
    .args(["--check", "--record"])
    .arg(&record)
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    .spawn()
    .expect("furb-gui starts");
  let mut most = 0.0_f64;
  let status = loop {
    if let Some(status) = app.try_wait().expect("furb-gui is watched") {
      break status;
    }
    for pid in webcontents().difference(&before) {
      most = most.max(footprint(*pid).unwrap_or_default());
    }
    thread::sleep(Duration::from_millis(500));
  };
  let printed = std::io::read_to_string(app.stdout.take().expect("the output of the check"))
    .unwrap_or_default();
  println!("{}the WebContent took {most:.0} MB at most", printed);
  assert!(status.success(), "the check failed");
  let drew = printed
    .lines()
    .find_map(|line| line.strip_prefix("furb-gui: the window drew "))
    .expect("the check says what the window drew");
  let numbers: Vec<usize> =
    drew.split_whitespace().filter_map(|one| one.trim_end_matches(',').parse().ok()).collect();
  let (chains, threads, acts) = counted(&record);
  assert_eq!(numbers, vec![chains, threads, acts]);
  assert!(most > 0.0, "no WebContent of the app was seen");
  assert!(most < MOST, "the WebContent took {most:.0} MB");
}
