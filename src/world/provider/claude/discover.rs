//! What the installed claude command line says of its models, read from its own answer and not from a table of furb.
//!
//! The command line answers control requests with no model prompt: `initialize` lists its aliases with the efforts that
//! each takes, and `set_model` then `get_context_usage` give the window of the model that an alias resolves to now.
//! A process of its own asks them once for each program. Furb keeps the answer in memory and in its cache until the
//! program changes or [`refresh`] is called. A command line that does not answer, as an older one does not, leaves the
//! fixed table of [`fallback`].

use std::{
  collections::HashMap,
  fs,
  io::{BufRead, BufReader, Write},
  process::Stdio,
  sync::{Mutex, OnceLock, mpsc},
  thread,
  time::{Duration, Instant, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{EFFORTS, Program};

/// One model of the command line as it said itself: the alias, the window of the model that it resolves to, and the
/// efforts that it takes, none when it takes no effort.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Found {
  pub alias: String,
  pub window: u64,
  pub efforts: Vec<String>,
}

/// The aliases and windows that furb offers when it cannot ask the command line, each with every effort.
const FIXED: [(&str, u64); 4] =
  [("opus", 1_000_000), ("sonnet", 1_000_000), ("haiku", 200_000), ("fable", 1_000_000)];

/// How long the whole conversation of a discovery may take.
const WITHIN: Duration = Duration::from_secs(20);

/// How long a failed discovery holds before the next models ask the command line again.
const RETRY: Duration = Duration::from_secs(300);

/// The file of the cache of furb that holds the discoveries.
const FILE: &str = "claude-models.json";

/// The models that furb offers when the command line cannot be asked.
pub(super) fn fallback() -> Vec<Found> {
  let efforts = || EFFORTS.iter().map(|one| (*one).to_owned()).collect();
  FIXED
    .iter()
    .map(|(alias, window)| Found {
      alias: (*alias).to_owned(),
      window: *window,
      efforts: efforts(),
    })
    .collect()
}

enum Seen {
  Found(Vec<Found>),
  Failed(Instant),
}

fn seen() -> &'static Mutex<HashMap<String, Seen>> {
  static SEEN: OnceLock<Mutex<HashMap<String, Seen>>> = OnceLock::new();
  SEEN.get_or_init(Default::default)
}

/// What names a program and its state: its file, its words, and the time and the size of its file, so an update of
/// the program makes a new stamp.
fn stamp(program: &Program) -> String {
  let meta = fs::metadata(&*program.bin).ok();
  let time = meta
    .as_ref()
    .and_then(|one| one.modified().ok())
    .and_then(|one| one.duration_since(UNIX_EPOCH).ok())
    .map_or(0, |one| one.as_nanos());
  let size = meta.map_or(0, |one| one.len());
  format!("{}\0{}\0{time}\0{size}", program.bin.display(), program.args.join("\u{1f}"))
}

fn cached() -> Option<std::path::PathBuf> {
  crate::world::cache().map(|at| at.join(FILE))
}

fn stored(stamp: &str) -> Option<Vec<Found>> {
  let text = fs::read_to_string(cached()?).ok()?;
  let map: serde_json::Map<String, Value> = serde_json::from_str(&text).ok()?;
  let found: Vec<Found> = serde_json::from_value(map.get(stamp)?.clone()).ok()?;
  (!found.is_empty()).then_some(found)
}

fn store(stamp: &str, found: &[Found]) {
  let Some(file) = cached() else { return };
  let mut map: serde_json::Map<String, Value> = fs::read_to_string(&file)
    .ok()
    .and_then(|text| serde_json::from_str(&text).ok())
    .unwrap_or_default();
  // One entry for each program: an older stamp of it goes.
  let program = stamp.splitn(3, '\0').take(2).collect::<Vec<_>>().join("\0");
  map.retain(|key, _| !key.starts_with(&program));
  map.insert(stamp.to_owned(), json!(found));
  while map.len() > 16 {
    let Some(oldest) = map.keys().next().cloned() else { break };
    map.shift_remove(&oldest);
  }
  let part = file.with_extension(format!("{}.part", std::process::id()));
  let wrote = file.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| {
    fs::write(&part, Value::Object(map).to_string()).and_then(|()| fs::rename(&part, &file))
  });
  if wrote.is_err() {
    let _ = fs::remove_file(&part);
  }
}

/// The models that the program says, from memory, from the cache, or from the program itself; none when it does not
/// answer.
pub(super) fn models(program: &Program) -> Option<Vec<Found>> {
  let stamp = stamp(program);
  let mut seen = seen().lock().unwrap_or_else(|one| one.into_inner());
  match seen.get(&stamp) {
    Some(Seen::Found(found)) => return Some(found.clone()),
    Some(Seen::Failed(at)) if at.elapsed() < RETRY => return None,
    _ => {}
  }
  let found = stored(&stamp).or_else(|| {
    let found = probe(program)?;
    store(&stamp, &found);
    Some(found)
  });
  let kept = found.clone().map_or(Seen::Failed(Instant::now()), Seen::Found);
  seen.insert(stamp, kept);
  found
}

/// Whether a turn may spend this effort on this alias: any effort of furb, or one that the command line reported.
pub(super) fn takes(program: &Program, alias: &str, effort: &str) -> bool {
  if EFFORTS.contains(&effort) {
    return true;
  }
  let Some(found) = models(program) else { return false };
  found.iter().any(|one| one.alias == alias && one.efforts.iter().any(|one| one == effort))
}

/// Forget each discovery, so the next models ask the command line again.
pub fn refresh() {
  seen().lock().unwrap_or_else(|one| one.into_inner()).clear();
  if let Some(file) = cached() {
    let _ = fs::remove_file(file);
  }
}

/// An alias that a turn can name: a plain word, as `opus`, and no `default` or `opus[1m]`.
fn plain(alias: &str) -> bool {
  alias != "default"
    && !alias.is_empty()
    && alias.chars().all(|one| one.is_ascii_alphanumeric() || one == '-' || one == '_')
}

/// Ask the program of its models, with one process that spends no model prompt.
fn probe(program: &Program) -> Option<Vec<Found>> {
  let mut tokio = program.command();
  let command = tokio.as_std_mut();
  command.args([
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
  ]);
  command.args([
    "--tools",
    "",
    "--strict-mcp-config",
    "--disable-slash-commands",
    "--setting-sources",
    "",
  ]);
  command.env("FURB_CLAUDE_DISCOVER", "1");
  command.env("CLAUDE_CODE_DISABLE_BUNDLED_SKILLS", "1");
  command.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
  command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
  let mut child = command.spawn().ok()?;
  let found = child.stdin.take().zip(child.stdout.take()).and_then(|(mut stdin, stdout)| {
    let (lines, heard) = mpsc::channel();
    thread::spawn(move || {
      for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        if lines.send(line).is_err() {
          break;
        }
      }
    });
    asked(&mut stdin, &heard)
  });
  let _ = child.kill();
  let _ = child.wait();
  found
}

fn asked(stdin: &mut impl Write, heard: &mpsc::Receiver<String>) -> Option<Vec<Found>> {
  let deadline = Instant::now() + WITHIN;
  let mut count = 0;
  let mut ask = |request: Value| -> Option<Value> {
    count += 1;
    let id = format!("furb-{count}");
    writeln!(stdin, "{}", json!({"type": "control_request", "request_id": id, "request": request}))
      .ok()?;
    stdin.flush().ok()?;
    loop {
      let line = heard.recv_timeout(deadline.checked_duration_since(Instant::now())?).ok()?;
      let Ok(event) = serde_json::from_str::<Value>(&line) else { continue };
      let response = &event["response"];
      if event["type"] == "control_response" && response["request_id"] == id.as_str() {
        return (response["subtype"] == "success").then(|| response["response"].clone());
      }
    }
  };
  let init = ask(json!({"subtype": "initialize"}))?;
  let mut found: Vec<Found> = Vec::new();
  for one in init["models"].as_array()? {
    let Some(alias) = one["value"].as_str().filter(|one| plain(one)) else { continue };
    if found.iter().any(|one| one.alias == alias) {
      continue;
    }
    let efforts = match (one["supportsEffort"].as_bool(), one["supportedEffortLevels"].as_array()) {
      (Some(false), _) => Vec::new(),
      (_, Some(levels)) => {
        levels.iter().filter_map(|one| one.as_str().map(str::to_owned)).collect()
      }
      _ => EFFORTS.iter().map(|one| (*one).to_owned()).collect(),
    };
    if ask(json!({"subtype": "set_model", "model": alias})).is_none() {
      continue;
    }
    let Some(window) =
      ask(json!({"subtype": "get_context_usage"})).and_then(|one| one["maxTokens"].as_u64())
    else {
      continue;
    };
    found.push(Found { alias: alias.to_owned(), window, efforts });
  }
  (!found.is_empty()).then_some(found)
}
