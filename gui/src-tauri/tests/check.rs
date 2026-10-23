//! The check of the app on a record: `furb-gui --check` opens the record through its host, as the TUI opens it, and
//! the counts that it prints are those that `furb facts` shows of the same record, while the web view of the app stays
//! under [`MOST`] megabytes. The record is a copy of the life of the demo in `tests/fixtures`, or the one that
//! `FURB_RECORD` names, which must be a copy, since a life writes beside its record. Run it after `bun run build` at
//! the root and in `gui`:
//!
//! `cargo test --features tauri/custom-protocol --test check -- --nocapture`
//!
//! The command `furb` comes from `FURB_CLI`, else from the target of the repository, else from PATH. The web view is
//! the WebContent of WebKit, a process of its own, whose id the app prints, so the test runs on macOS. A check ends
//! once its window drew and its stdin ended, so the test reads the web view after the window drew, while it stands.

#![cfg(target_os = "macos")]

use std::{
  collections::HashSet,
  io::{BufRead, BufReader},
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::mpsc,
  thread,
  time::Duration,
};

/// The most megabytes that the WebContent of the app may take on a record: its footprint, as Activity Monitor shows
/// it.
const MOST: f64 = 300.0;

/// The questions that the engine asks on the way to what an act does, which the model of a life, and so the app,
/// shows as no act: those of `STEPS` in `bind/typescript/src/activity.ts`.
const STEPS: [&str; 9] =
  ["stand", "clock", "chance", "gate", "cd", "merged", "run", "wants", "reply"];

/// The name of a process and its footprint in megabytes, as one answer of `footprint` says them, so the footprint is
/// that of the process that the name names, even when its id names another process later.
fn footprint(pid: u32) -> Option<(String, f64)> {
  let out = Command::new("footprint").args(["-p", &pid.to_string()]).output().ok()?;
  let text = String::from_utf8_lossy(&out.stdout);
  let (head, said) = text.split_once("Footprint: ")?;
  let (name, _) = head.lines().last()?.split_once(&format!(" [{pid}]"))?;
  let mut words = said.split_whitespace();
  let number: f64 = words.next()?.parse().ok()?;
  let scale = match words.next()? {
    "B" => 1.0 / 1024.0 / 1024.0,
    "KB" => 1.0 / 1024.0,
    "MB" => 1.0,
    "GB" => 1024.0,
    _ => return None,
  };
  Some((name.trim().to_owned(), number * scale))
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

/// The chains, the threads and the acts of the questions of a life, each given as its kind, its name, who made it and
/// the chain that it stands on. A chain is each chain that a question makes or stands on, a thread is each prompt on a
/// chain,
/// and an act is each question that is no step of the engine, each chain, and each rung that a question other than a
/// prompt names as its maker, which the reader of the crate makes when the record does not keep it.
fn tally<'a>(questions: impl Iterator<Item = [&'a str; 4]>) -> (usize, usize, usize) {
  let (mut chains, mut acts, mut threads) = (HashSet::new(), HashSet::new(), 0);
  for [kind, about, by, on] in questions {
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
    // A prompt on a thread is a message of that thread, and opens no thread of its own.
    threads += usize::from(kind == "prompt" && !named(on, "prompt"));
  }
  (chains.len(), threads, acts.len())
}

/// The chains, the threads and the acts that a record kept, as `furb facts` shows its questions.
fn counted(record: &Path) -> (usize, usize, usize) {
  let out =
    Command::new(furb()).args(["facts", "--record"]).arg(record).output().expect("furb facts runs");
  assert!(out.status.success(), "furb facts fails: {}", String::from_utf8_lossy(&out.stderr));
  let text = String::from_utf8_lossy(&out.stdout);
  tally(text.lines().filter_map(|line| words(line).try_into().ok()))
}

/// The chains, the threads and the acts of the log of a life, beside its record: each fact that the life said, one
/// list of JSON on each line after the first.
fn logged(record: &Path) -> (usize, usize, usize) {
  let mut path = record.as_os_str().to_owned();
  path.push(".log");
  let text = std::fs::read_to_string(&path).expect("the log of the life is read");
  let facts: Vec<Vec<serde_json::Value>> =
    text.lines().skip(1).filter_map(|line| serde_json::from_str(line).ok()).collect();
  tally(facts.iter().filter_map(|fact| {
    let text = |at: usize| fact.get(at).and_then(serde_json::Value::as_str).unwrap_or_default();
    (fact.len() >= 4).then(|| [text(0), text(1), text(2), text(3)])
  }))
}

/// The record that the check opens: the one that `FURB_RECORD` names, else a copy of the record of a life of the demo
/// that the repository holds, in a folder of its own, since a life that opens on a record writes beside it.
fn record() -> PathBuf {
  if let Ok(named) = std::env::var("FURB_RECORD") {
    return named.into();
  }
  let folder = std::env::temp_dir().join(format!("furb-gui-check-fixture-{}", std::process::id()));
  std::fs::create_dir_all(&folder).expect("the folder of the copy is made");
  let copy = folder.join("demo.jsonl");
  std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo.jsonl"), &copy)
    .expect("the record of the demo is copied");
  copy
}

#[test]
fn the_check_draws_the_whole_record_and_its_web_view_stays_light() {
  let record = record();
  let mut app = Command::new(env!("CARGO_BIN_EXE_furb-gui"))
    .args(["--check", "--record"])
    .arg(&record)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    .spawn()
    .expect("furb-gui starts");
  let mut input = app.stdin.take();
  // The app prints the process of its web view, then what its window drew, so the test reads its lines as they come
  // and measures that process alone, and no other WebContent of the machine.
  let (lines, heard) = mpsc::channel();
  let out = BufReader::new(app.stdout.take().expect("the output of the check"));
  thread::spawn(move || {
    for line in out.lines().map_while(Result::ok) {
      let _ = lines.send(line);
    }
  });
  let (mut printed, mut content, mut most, mut names, mut last) =
    (Vec::new(), None, 0.0_f64, HashSet::new(), None);
  let status = loop {
    // A line wakes the test at once, so it reads the process of the web view as soon as the app names it.
    let line = heard.recv_timeout(Duration::from_millis(100)).ok();
    if let Some(line) = &line {
      content = content.or_else(|| {
        line.strip_prefix("furb-gui: the web view runs in process ")?.parse::<u32>().ok()
      });
    }
    let read = content.and_then(footprint);
    if let Some((name, megabytes)) = &read {
      names.insert(name.clone());
      most = most.max(*megabytes);
    }
    // The app waits for the end of its stdin once its window drew, so this read finds its web view still there.
    if line.as_deref().is_some_and(|one| one.starts_with("furb-gui: the window drew ")) {
      last = read;
      drop(input.take());
    }
    printed.extend(line);
    if let Some(status) = app.try_wait().expect("furb-gui is watched") {
      thread::sleep(Duration::from_millis(100));
      printed.extend(heard.try_iter());
      break status;
    }
  };
  println!("{}\nthe WebContent took {most:.0} MB at most", printed.join("\n"));
  assert!(status.success(), "the check failed");
  let drew = printed
    .iter()
    .find_map(|line| line.strip_prefix("furb-gui: the window drew "))
    .expect("the check says what the window drew");
  let numbers: Vec<usize> =
    drew.split_whitespace().filter_map(|one| one.trim_end_matches(',').parse().ok()).collect();
  let (chains, threads, acts) =
    if drew.ends_with("of its log") { logged(&record) } else { counted(&record) };
  assert_eq!(numbers, vec![chains, threads, acts]);
  assert!(content.is_some(), "the app said no process of its web view");
  assert!(last.is_some(), "no footprint of the web view of the app was read once its window drew");
  assert!(
    names.iter().all(|name| name.contains("WebContent")),
    "the process that the app said is no WebContent of WebKit: {names:?}"
  );
  assert!(most > 0.0, "no footprint of the web view of the app was read");
  assert!(most < MOST, "the WebContent took {most:.0} MB");
}

#[test]
fn a_check_ends_in_a_window_that_is_hidden_as_a_window_behind_others() {
  let record = record();
  let mut app = Command::new(env!("CARGO_BIN_EXE_furb-gui"))
    .args(["--check", "--record"])
    .arg(&record)
    .env("FURB_GUI_HIDDEN", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    .spawn()
    .expect("furb-gui starts");
  // WebKit runs no callback of a frame in a hidden window, so a check that waits for a frame alone never ends.
  let mut waited = Duration::ZERO;
  let status = loop {
    if let Some(status) = app.try_wait().expect("furb-gui is watched") {
      break status;
    }
    if waited > Duration::from_secs(120) {
      let _ = app.kill();
      panic!("the check of a hidden window did not end in 120 seconds");
    }
    thread::sleep(Duration::from_millis(100));
    waited += Duration::from_millis(100);
  };
  let printed = std::io::read_to_string(app.stdout.take().expect("the output of the check"))
    .unwrap_or_default();
  assert!(status.success(), "the check failed: {printed}");
  assert!(printed.contains("furb-gui: the window drew "), "the check drew nothing: {printed}");
}
