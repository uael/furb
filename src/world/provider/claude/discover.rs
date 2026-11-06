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
  process::Stdio,
  sync::{Mutex, OnceLock, PoisonError},
  time::{Duration, Instant, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
  process::{ChildStdin, ChildStdout},
};

use super::{EFFORTS, Program};
use crate::world;
use crate::world::{
  child,
  disk::{self, Access},
};
use std::path;
use tokio::time;

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

fn cached() -> Option<path::PathBuf> {
  world::cache().map(|at| at.join(FILE))
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
  // A cache that cannot be written costs the next life one probe more, so the failure ends here.
  drop(disk::write(&file, Value::Object(map).to_string().as_bytes(), Access::Shared));
}

/// The models that the program says, from memory, from the cache, or from the program itself;
/// none when it does not answer. No lock is held while the program is asked, so two catalogs
/// that ask at once may each ask it.
pub(super) async fn models(program: &Program) -> Option<Vec<Found>> {
  let stamp = stamp(program);
  let held = match seen().lock().unwrap_or_else(PoisonError::into_inner).get(&stamp) {
    Some(Seen::Found(found)) => Some(Some(found.clone())),
    Some(Seen::Failed(at)) if at.elapsed() < RETRY => Some(None),
    _ => None,
  };
  if let Some(held) = held {
    return held;
  }
  let found = if let Some(found) = stored(&stamp) {
    Some(found)
  } else {
    let found = probe(program).await;
    if let Some(found) = &found {
      store(&stamp, found);
    }
    found
  };
  let kept = found.clone().map_or(Seen::Failed(Instant::now()), Seen::Found);
  seen().lock().unwrap_or_else(PoisonError::into_inner).insert(stamp, kept);
  found
}

/// Whether a turn may spend this effort on this alias: any effort of furb, or one that the command line reported.
pub(super) async fn takes(program: &Program, alias: &str, effort: &str) -> bool {
  if EFFORTS.contains(&effort) {
    return true;
  }
  let Some(found) = models(program).await else { return false };
  found.iter().any(|one| one.alias == alias && one.efforts.iter().any(|one| one == effort))
}

/// Whether the program lists a flag in its help, asked once for each stamp of the program. A program that does not
/// answer in time lists none, so furb gives it no flag that it may not know.
pub(super) async fn lists(program: &Program, flag: &str) -> bool {
  static LISTED: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
  let held = LISTED.get_or_init(Default::default);
  let key = format!("{}\0{flag}", stamp(program));
  let known = held.lock().unwrap_or_else(PoisonError::into_inner).get(&key).copied();
  if let Some(known) = known {
    return known;
  }
  let listed = help(program).await.is_some_and(|text| text.contains(flag));
  held.lock().unwrap_or_else(PoisonError::into_inner).insert(key, listed);
  listed
}

/// What the program says for `--help`, none when it does not run or does not end within
/// [`WITHIN`], when it is killed.
async fn help(program: &Program) -> Option<String> {
  let mut command = program.command().ok()?;
  command.arg("--help").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
  let out = child::output(&mut command, WITHIN).await?;
  Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Forget each discovery, so the next models ask the command line again.
pub fn refresh() {
  seen().lock().unwrap_or_else(PoisonError::into_inner).clear();
  // A cache that cannot go stays, and the next models read it until the program makes a new stamp.
  if let Some(file) = cached() {
    fs::remove_file(file).unwrap_or_default();
  }
}

/// An alias that a turn can name: a plain word, as `opus`, and no `default` or `opus[1m]`.
fn plain(alias: &str) -> bool {
  alias != "default"
    && !alias.is_empty()
    && alias.chars().all(|one| one.is_ascii_alphanumeric() || one == '-' || one == '_')
}

/// Ask the program of its models, with one process that spends no model prompt and that is
/// killed once it answered, or once [`WITHIN`] passed.
async fn probe(program: &Program) -> Option<Vec<Found>> {
  let print =
    ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"];
  let mut command = program.base_command(&print).ok()?;
  command.args(["--tools", ""]);
  command.env("FURB_CLAUDE_DISCOVER", "1");
  command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
  // The group ends the child as it drops, and the runtime reaps it.
  let (mut child, _group) = child::spawn(&mut command).ok()?;
  let (stdin, stdout) = child.stdin.take().zip(child.stdout.take())?;
  let found = time::timeout(WITHIN, asked(stdin, BufReader::new(stdout).lines())).await;
  found.ok().flatten()
}

/// The models that the program says over its control requests.
async fn asked(
  mut stdin: ChildStdin,
  mut lines: Lines<BufReader<ChildStdout>>,
) -> Option<Vec<Found>> {
  let mut count = 0;
  let mut ask = async |request: Value| -> Option<Value> {
    count += 1;
    let id = format!("furb-{count}");
    let line = json!({"type": "control_request", "request_id": id, "request": request});
    stdin.write_all(format!("{line}\n").as_bytes()).await.ok()?;
    stdin.flush().await.ok()?;
    loop {
      let line = lines.next_line().await.ok()??;
      let Ok(event) = serde_json::from_str::<Value>(&line) else { continue };
      let response = &event["response"];
      if event["type"] == "control_response" && response["request_id"] == id.as_str() {
        return (response["subtype"] == "success").then(|| response["response"].clone());
      }
    }
  };
  let init = ask(json!({"subtype": "initialize"})).await?;
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
    if ask(json!({"subtype": "set_model", "model": alias})).await.is_none() {
      continue;
    }
    let Some(window) =
      ask(json!({"subtype": "get_context_usage"})).await.and_then(|one| one["maxTokens"].as_u64())
    else {
      continue;
    };
    found.push(Found { alias: alias.to_owned(), window, efforts });
  }
  (!found.is_empty()).then_some(found)
}
