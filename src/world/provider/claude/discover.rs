//! What the installed claude command line says of its models, read from its own answer and not from a table of furb.
//!
//! The command line answers control requests with no model prompt: `initialize` lists its aliases with the efforts that
//! each takes, and `set_model` then `get_context_usage` give the window of the model that an alias resolves to now.
//! A process of its own asks them. Furb keeps the answer in memory and in its cache, and gives what
//! they hold with no wait: an answer that is [`LASTS`] seconds old is asked again behind the models
//! that it gives, for the next ones. The models wait for the command line only while furb holds
//! nothing of its program: before its first answer, after the program changes, and after
//! [`refresh`]. A command line that does not answer, as an older one does not, leaves the fixed
//! table of [`fallback`], and one that left an ask with no answer for [`WITHIN`] is in the cache as
//! such, so no later process waits for it again.

use std::{
  collections::HashMap,
  env, fs,
  process::{self, Stdio},
  sync::{Mutex, MutexGuard, OnceLock, PoisonError},
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
  process::{ChildStdin, ChildStdout},
};

use super::{
  EFFORTS, Program,
  protocol::{Control, Event},
};
use crate::ear::runtime;
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

/// How many seconds an ask that gave no models holds before the next models ask the command line
/// again.
const RETRY: u64 = 300;

/// How many seconds a discovery holds, in memory and in the cache, before the next models ask
/// the command line again, behind what they give: the models that an account may use change while
/// the program of its command line does not.
pub(super) const LASTS: u64 = 3600;

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

/// What furb holds of a program, in memory and in the cache: the models that it said and the second
/// that it said them, none when it never answered, and the second of the last ask that gave no
/// models.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Told {
  at: u64,
  found: Vec<Found>,
  #[serde(default)]
  asked: Option<u64>,
}

impl Told {
  /// Whether the program is asked again at a second: it said no models, or said them [`LASTS`]
  /// seconds before, and no ask of the [`RETRY`] seconds before gave none.
  fn old(&self, now: u64) -> bool {
    let since = |then: u64| now.saturating_sub(then);
    (self.found.is_empty() || since(self.at) >= LASTS)
      && self.asked.is_none_or(|then| since(then) >= RETRY)
  }
}

fn seen() -> &'static Mutex<HashMap<String, Told>> {
  static SEEN: OnceLock<Mutex<HashMap<String, Told>>> = OnceLock::new();
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

/// The file of the cache. A test run owns one of its own, so no other process of the machine, as
/// another test binary or a furb that runs, writes back an entry that a refresh of the test forgot.
fn cached() -> Option<path::PathBuf> {
  let at = if cfg!(test) {
    Some(env::temp_dir().join(format!("furb-test-{}", process::id())))
  } else {
    world::cache()
  };
  at.map(|at| at.join(FILE))
}

fn stored(file: &path::Path, stamp: &str) -> Option<Told> {
  let text = fs::read_to_string(file).ok()?;
  let map: serde_json::Map<String, Value> = serde_json::from_str(&text).ok()?;
  serde_json::from_value(map.get(stamp)?.clone()).ok()
}

/// Holds a store of the cache and a refresh of it apart: the file is read, changed and written
/// whole, so a store that read it before a refresh removed it would write back what the refresh
/// forgot.
fn exclusive() -> MutexGuard<'static, ()> {
  static LOCK: Mutex<()> = Mutex::new(());
  LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

fn store(file: &path::Path, stamp: &str, told: &Told) {
  // A cache that cannot be written costs the next life one probe more, so the failure ends here.
  stored_by(file, stamp, told, |file, bytes| {
    drop(disk::write(file, bytes, Access::Shared));
  });
}

/// [`store`] with the write of the file given, so a test of the race writes with no sync to the
/// disk, whose time grows with the load of the machine.
fn stored_by(file: &path::Path, stamp: &str, told: &Told, write: impl FnOnce(&path::Path, &[u8])) {
  let _held = exclusive();
  let mut map: serde_json::Map<String, Value> = fs::read_to_string(file)
    .ok()
    .and_then(|text| serde_json::from_str(&text).ok())
    .unwrap_or_default();
  // One entry for each program: an older stamp of it goes.
  let program = stamp.splitn(3, '\0').take(2).collect::<Vec<_>>().join("\0");
  map.retain(|key, _| !key.starts_with(&program));
  map.insert(stamp.to_owned(), json!(told));
  while map.len() > 16 {
    let Some(oldest) = map.keys().next().cloned() else { break };
    map.shift_remove(&oldest);
  }
  write(file, Value::Object(map).to_string().as_bytes());
}

/// The entries of the cache whose program is named so gone, so its next models ask it again.
fn forget(file: &path::Path, named: &str) {
  let _held = exclusive();
  let Some(mut map) = fs::read_to_string(file)
    .ok()
    .and_then(|text| serde_json::from_str::<serde_json::Map<String, Value>>(&text).ok())
  else {
    fs::remove_file(file).unwrap_or_default();
    return;
  };
  map.retain(|key, _| !key.starts_with(named));
  drop(disk::write(file, Value::Object(map).to_string().as_bytes(), Access::Shared));
}

/// The models that the program said last, from memory or from the cache; none when it said none.
/// They wait for the program only while furb holds nothing of it. An answer that is old is asked
/// again as a task of the runtime of the ears, which keeps what the program says for the next
/// models. No lock is held while the program is asked, so two catalogs that ask at once may each
/// ask it.
pub(super) async fn models(program: &Program) -> Option<Vec<Found>> {
  let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |one| one.as_secs());
  models_at(program, now).await
}

/// [`models`] as of a second.
pub(super) async fn models_at(program: &Program, now: u64) -> Option<Vec<Found>> {
  let stamp = stamp(program);
  let told = match held(&stamp, now) {
    Some(told) if told.old(now) => {
      let (program, was) = (program.clone(), told.clone());
      runtime().spawn(async move {
        ask(&program, stamp, was, now).await;
      });
      told
    }
    Some(told) => told,
    None => ask(program, stamp, Told::default(), now).await,
  };
  (!told.found.is_empty()).then_some(told.found)
}

/// What furb holds of the program of a stamp: what the memory holds while it is not old, and else
/// what the cache holds, since another process may have asked the program after this one did. One
/// that is old comes to one caller, which asks the program: the memory then holds it as asked at
/// this second, which holds the next ask of this process off while the program answers this one.
fn held(stamp: &str, now: u64) -> Option<Told> {
  let mut seen = seen().lock().unwrap_or_else(PoisonError::into_inner);
  let told = match seen.get(stamp).cloned() {
    Some(told) if !told.old(now) => told,
    kept => cached().and_then(|file| stored(&file, stamp)).or(kept)?,
  };
  if told.old(now) {
    seen.insert(stamp.to_owned(), Told { asked: Some(now), ..told.clone() });
  }
  Some(told)
}

/// Ask the program, and keep what it says: its models as of this second, or, when it gives none,
/// what it said before with the second of this ask.
async fn ask(program: &Program, stamp: String, was: Told, now: u64) -> Told {
  let said = time::timeout(WITHIN, probe(program)).await;
  // A program that says at once that it gives no models costs the next process no wait, so only
  // the memory keeps that ask. One that left the ask with no answer is in the cache too.
  let lasts = !matches!(said, Ok(None));
  let told = match said {
    Ok(Some(found)) => Told { at: now, found, asked: None },
    _ => Told { asked: Some(now), ..was },
  };
  if lasts && let Some(file) = cached() {
    store(&file, &stamp, &told);
  }
  seen().lock().unwrap_or_else(PoisonError::into_inner).insert(stamp, told.clone());
  told
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

/// Forget each discovery of the program at `bin`, or of the command line of this machine, so its
/// next models ask it again. The discoveries of another program stay, as those of another life of
/// the same process.
pub fn refresh(bin: Option<&path::Path>) {
  let Some(bin) = bin.map(path::Path::to_path_buf).or_else(super::located) else { return };
  let named = format!("{}\0", bin.display());
  seen().lock().unwrap_or_else(PoisonError::into_inner).retain(|key, _| !key.starts_with(&named));
  // A cache that cannot be written stays, and the next models read it until the program makes a new
  // stamp.
  if let Some(file) = cached() {
    forget(&file, &named);
  }
}

/// An alias that a turn can name: a plain word, as `opus`, and no `default` or `opus[1m]`.
fn plain(alias: &str) -> bool {
  alias != "default"
    && !alias.is_empty()
    && alias.chars().all(|one| one.is_ascii_alphanumeric() || one == '-' || one == '_')
}

/// Ask the program of its models, with one process that spends no model prompt and that is
/// killed once it answered, or once the future goes, as when [`WITHIN`] passed.
async fn probe(program: &Program) -> Option<Vec<Found>> {
  let print =
    ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"];
  let mut command = program.base_command(&print, false).ok()?;
  command.args(["--tools", ""]);
  command.env("FURB_CLAUDE_DISCOVER", "1");
  command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
  // The group ends the child as it drops, and the runtime reaps it.
  let (mut child, _group) = child::spawn(&mut command).ok()?;
  let (stdin, stdout) = child.stdin.take().zip(child.stdout.take())?;
  asked(stdin, BufReader::new(stdout).lines()).await
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
      let Ok(Event::ControlResponse { response }) =
        serde_json::from_str(&line).and_then(Event::read)
      else {
        continue;
      };
      match response {
        Control::Success { request_id, response } if request_id == id => return Some(response),
        Control::Error { request_id } if request_id == id => return None,
        _ => {}
      }
    }
  };
  let Init { models } =
    serde_json::from_value(ask(json!({"subtype": "initialize"})).await?).ok()?;
  let mut found: Vec<Found> = Vec::new();
  for one in models {
    if !plain(&one.value) || found.iter().any(|known| known.alias == one.value) {
      continue;
    }
    let efforts = match (one.supports_effort, one.supported_effort_levels) {
      (Some(false), _) => Vec::new(),
      (_, Some(levels)) => levels,
      _ => EFFORTS.iter().map(|one| (*one).to_owned()).collect(),
    };
    if ask(json!({"subtype": "set_model", "model": one.value.as_str()})).await.is_none() {
      continue;
    }
    let usage = ask(json!({"subtype": "get_context_usage"})).await;
    let Some(Usage { max_tokens }) = usage.and_then(|one| serde_json::from_value(one).ok()) else {
      continue;
    };
    found.push(Found { alias: one.value, window: max_tokens, efforts });
  }
  (!found.is_empty()).then_some(found)
}

/// The answer of the request `initialize`: the models that the command line offers.
#[derive(Deserialize)]
struct Init {
  models: Vec<Offered>,
}

/// A model that the command line offers, by its alias, with the efforts that it takes.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Offered {
  value: String,
  supports_effort: Option<bool>,
  supported_effort_levels: Option<Vec<String>>,
}

/// The answer of the request `get_context_usage`: the window of the model that is set.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Usage {
  max_tokens: u64,
}

#[cfg(test)]
#[path = "discover.test.rs"]
mod test;
