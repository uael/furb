//! The update of the catalog: the one path by which every host brings the models of its sources up to date.
//!
//! Two sources feed it. models.dev lists the providers of the network, and the maintained package
//! `@earendil-works/pi-ai` lists the providers that models.dev does not. An update fetches both, checks that each
//! reads as a catalog, and only then replaces the copy in the cache of furb, so a source that is offline or gives
//! invalid data leaves the last valid copy in place. `catalog.status.json` beside the copies keeps the version of
//! each source, the time of its last good fetch, and the error of its last failed one. Each catalog is read again from
//! the caches when a life stands again, so a running life uses an update with no restart.

use std::{
  collections::HashMap,
  fs,
  path::Path,
  sync::{Mutex, OnceLock, mpsc},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use indexmap::IndexMap;
use rig_core::http_client::ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::{Cached, Listed};
use crate::{ear::reactor, world::cache};

/// Where models.dev serves its catalog.
const URL: &str = "https://models.dev/api.json";

/// The registry entry of the latest release of pi-ai.
const REGISTRY: &str = "https://registry.npmjs.org/@earendil-works/pi-ai/latest";

/// Where the files of one exact release of pi-ai are served.
const FILES: &str = "https://cdn.jsdelivr.net/npm/@earendil-works/pi-ai";

/// What the snapshot says of its sources, which `script/catalog.py` writes.
const SOURCES: &str = include_str!("../catalog.sources.json");

/// The copy of models.dev in the cache.
pub(super) const MODELS: &str = "models.json";

/// The copy of the providers of pi-ai in the cache.
pub(super) const PI: &str = "pi.json";

/// The status in the cache.
const STATUS: &str = "catalog.status.json";

/// How old the copy of models.dev grows before a life updates it.
const STALE: Duration = Duration::from_secs(24 * 60 * 60);

/// How long a life waits after an attempt before it makes another one in the background.
const BACKOFF: Duration = Duration::from_secs(60 * 60);

/// One source of the catalog: its version when it has one, the time of its last good fetch, and the error of its last
/// failed fetch, which a good fetch clears.
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Source {
  pub version: Option<String>,
  pub fetched: Option<String>,
  pub error: Option<String>,
}

/// Where the models of this machine came from.
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Status {
  #[serde(rename = "models.dev")]
  pub models_dev: Source,
  #[serde(rename = "pi-ai")]
  pub pi_ai: Source,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Bundled {
  #[serde(rename = "models.dev")]
  models_dev: Source,
  #[serde(rename = "pi-ai")]
  pi_ai: BundledPi,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BundledPi {
  version: Option<String>,
  fetched: Option<String>,
  providers: Vec<String>,
}

#[derive(Deserialize)]
struct PiCache {
  version: String,
  providers: HashMap<String, Cached>,
}

fn bundled() -> &'static Bundled {
  static BUNDLED: OnceLock<Bundled> = OnceLock::new();
  BUNDLED
    .get_or_init(|| serde_json::from_str(SOURCES).expect("the sources of the snapshot are JSON"))
}

fn stored(dir: &Path) -> Status {
  fs::read_to_string(dir.join(STATUS))
    .ok()
    .and_then(|text| serde_json::from_str(&text).ok())
    .unwrap_or_default()
}

/// Where the models of this machine came from: for each source, the newer of its last fetch into the cache and the
/// fetch that made the snapshot, and the error of the last failed fetch.
pub fn status() -> Status {
  let kept = cache().map(|dir| stored(&dir)).unwrap_or_default();
  let held = bundled();
  let newer = |kept: Source, version: Option<String>, fetched: Option<String>| {
    if kept.fetched.is_some() && kept.fetched >= fetched {
      kept
    } else {
      Source { version, fetched, error: kept.error }
    }
  };
  Status {
    models_dev: newer(kept.models_dev, None, held.models_dev.fetched.clone()),
    pi_ai: newer(kept.pi_ai, held.pi_ai.version.clone(), held.pi_ai.fetched.clone()),
  }
}

/// Fetch both sources now, keep what reads as a catalog, and say where the models stand after it.
pub async fn update() -> Status {
  let Some(dir) = cache() else {
    let mut status = status();
    let why = "this machine has no cache folder".to_owned();
    (status.models_dev.error, status.pi_ai.error) = (Some(why.clone()), Some(why));
    return status;
  };
  let mut kept = stored(&dir);
  let stamp = now();
  match models_dev(&dir).await {
    Ok(()) => kept.models_dev = Source { version: None, fetched: Some(stamp.clone()), error: None },
    Err(why) => kept.models_dev.error = Some(why),
  }
  match pi_ai(&dir).await {
    Ok(version) => {
      kept.pi_ai = Source { version: Some(version), fetched: Some(stamp), error: None }
    }
    Err(why) => kept.pi_ai.error = Some(why),
  }
  if let Ok(text) = serde_json::to_string_pretty(&kept) {
    let _ = keep(&dir.join(STATUS), &text);
  }
  status()
}

/// [`update`], for a host that does not run on the reactor.
pub fn updated() -> Status {
  let (send, got) = mpsc::channel();
  reactor().spawn(async move {
    let _ = send.send(update().await);
  });
  got.recv().unwrap_or_else(|_| status())
}

/// Update in the background when the copy of models.dev at a path is a day old or absent, and the last attempt of the
/// process is an hour old, so no life waits on the network and a source that fails is not asked at each stand.
pub(super) fn background(at: &Path) {
  static LAST: Mutex<Option<Instant>> = Mutex::new(None);
  let age = fs::metadata(at).ok().and_then(|made| made.modified().ok()?.elapsed().ok());
  if cfg!(test) || age.is_some_and(|age| age <= STALE) {
    return;
  }
  let Ok(mut last) = LAST.lock() else { return };
  if last.is_some_and(|then| then.elapsed() < BACKOFF) {
    return;
  }
  *last = Some(Instant::now());
  reactor().spawn(async {
    update().await;
  });
}

/// The providers that the copy of pi-ai holds, in place of those of the snapshot, when the copy is of a release as new
/// as the one of the snapshot.
pub(super) fn applied(listed: &mut IndexMap<String, Listed>, text: Option<&str>) {
  let Some(copy) = text.and_then(|text| serde_json::from_str::<PiCache>(text).ok()) else { return };
  if numbers(&copy.version) < numbers(bundled().pi_ai.version.as_deref().unwrap_or_default()) {
    return;
  }
  for (name, one) in copy.providers {
    if let Some(held) = listed.get_mut(&name).filter(|_| !one.models.is_empty()) {
      held.models = one.models;
    }
  }
}

fn numbers(version: &str) -> Vec<u64> {
  version.split(|one: char| !one.is_ascii_digit()).filter_map(|part| part.parse().ok()).collect()
}

async fn get(url: &str) -> Result<String, String> {
  let asked =
    ReqwestClient::new().get(url).header("user-agent", "furb").timeout(Duration::from_secs(60));
  let got = asked
    .send()
    .await
    .and_then(|got| got.error_for_status())
    .map_err(|no| format!("{url}: {no}"))?;
  got.text().await.map_err(|no| format!("{url}: {no}"))
}

fn keep(at: &Path, text: &str) -> Result<(), String> {
  let partial = at.with_extension("part");
  let wrote = at.parent().is_some_and(|parent| fs::create_dir_all(parent).is_ok())
    && fs::write(&partial, text).is_ok()
    && fs::rename(&partial, at).is_ok();
  if wrote { Ok(()) } else { Err(format!("no write of {}", at.display())) }
}

async fn models_dev(dir: &Path) -> Result<(), String> {
  let text = get(URL).await?;
  let read = serde_json::from_str::<HashMap<String, Cached>>(&text)
    .map_err(|no| format!("models.dev gave invalid data: {no}"))?;
  if read.values().filter(|one| !one.models.is_empty()).count() < 5 {
    return Err("models.dev gave too few providers".to_owned());
  }
  keep(&dir.join(MODELS), &text)
}

async fn pi_ai(dir: &Path) -> Result<String, String> {
  let latest: Value =
    serde_json::from_str(&get(REGISTRY).await?).map_err(|no| format!("{REGISTRY}: {no}"))?;
  let version = latest
    .get("version")
    .and_then(Value::as_str)
    .ok_or_else(|| format!("{REGISTRY} names no version"))?
    .to_owned();
  let mut providers = Map::new();
  for name in &bundled().pi_ai.providers {
    let text = get(&format!("{FILES}@{version}/dist/providers/data/{name}.json")).await?;
    let file: Value = serde_json::from_str(&text)
      .map_err(|no| format!("pi-ai {version} gave invalid data for {name}: {no}"))?;
    let mut models = Map::new();
    for api in file.as_object().into_iter().flat_map(Map::values) {
      for one in api.as_object().into_iter().flat_map(Map::values) {
        if let (Some(id), Some(model)) = (one.get("id").and_then(Value::as_str), pi(one)) {
          models.insert(id.to_owned(), model);
        }
      }
    }
    if models.is_empty() {
      return Err(format!("pi-ai {version} lists no model of {name}"));
    }
    providers.insert(name.clone(), json!({ "models": models }));
  }
  keep(&dir.join(PI), &json!({ "version": version, "providers": providers }).to_string())?;
  Ok(version)
}

/// A model of the data of pi-ai in the words of models.dev, as `script/catalog.py` makes it.
fn pi(listing: &Value) -> Option<Value> {
  let window = listing.get("contextWindow")?.as_u64()?;
  let output = listing.get("maxTokens")?.as_u64()?;
  let levels: Vec<Value> = listing
    .get("thinkingLevelMap")
    .and_then(Value::as_object)
    .map(|map| {
      map
        .iter()
        .filter(|(_, word)| !word.is_null())
        .map(|(level, _)| json!(if level == "off" { "none" } else { level }))
        .collect()
    })
    .unwrap_or_default();
  let price =
    |name: &str| listing.get("cost").and_then(|cost| cost.get(name)).cloned().unwrap_or(json!(0));
  Some(json!({
    "reasoning": listing.get("reasoning").and_then(Value::as_bool).unwrap_or(false),
    "reasoning_options": if levels.is_empty() { json!([]) } else { json!([{ "type": "effort", "values": levels }]) },
    "limit": { "context": window, "output": output },
    "cost": {
      "input": price("input"),
      "output": price("output"),
      "cache_read": price("cacheRead"),
      "cache_write": price("cacheWrite"),
    },
    "modalities": { "input": listing.get("input").cloned().unwrap_or(json!(["text"])), "output": ["text"] },
  }))
}

/// The time now, as `2026-10-26T18:09:17Z`.
fn now() -> String {
  stamp(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs()))
}

/// The time at a count of seconds since the epoch, as `2026-10-26T18:09:17Z`.
fn stamp(secs: u64) -> String {
  let (days, rest) = (i64::try_from(secs / 86400).unwrap_or(0), secs % 86400);
  let z = days + 719_468;
  let (era, day) = (z.div_euclid(146_097), z.rem_euclid(146_097));
  let years = (day - day / 1460 + day / 36_524 - day / 146_096) / 365;
  let in_year = day - (365 * years + years / 4 - years / 100);
  let month = (5 * in_year + 2) / 153;
  let date = in_year - (153 * month + 2) / 5 + 1;
  let month = if month < 10 { month + 3 } else { month - 9 };
  let year = years + era * 400 + i64::from(month <= 2);
  format!(
    "{year:04}-{month:02}-{date:02}T{:02}:{:02}:{:02}Z",
    rest / 3600,
    rest % 3600 / 60,
    rest % 60
  )
}

#[cfg(test)]
#[path = "update.test.rs"]
mod test;
