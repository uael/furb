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
  sync::{Mutex, OnceLock, PoisonError, mpsc},
  time::{Duration, Instant},
};

use indexmap::IndexMap;
use jiff::Timestamp;
use rig_core::http_client::ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::{Cached, Listed};
use crate::{
  ear::{reactor, runtime},
  world::{
    cache,
    disk::{self, Access},
    provider::numbers,
  },
};

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
const STALE: Duration = Duration::from_hours(24);

/// How long a life waits after an attempt before it makes another one in the background.
const BACKOFF: Duration = Duration::from_hours(1);

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

/// Why a source gives no catalog. The text of each variant is what the status keeps of the failure.
#[derive(Debug, thiserror::Error)]
enum Error {
  #[error("this machine has no cache folder")]
  NoCache,
  #[error("{url}: {source}")]
  Fetch { url: String, source: reqwest::Error },
  #[error("models.dev gave invalid data: {source}")]
  ModelsDev { source: serde_json::Error },
  #[error("models.dev gave too few providers")]
  TooFew,
  #[error("{}: {source}", REGISTRY)]
  Registry { source: serde_json::Error },
  #[error("{} names no version", REGISTRY)]
  NoVersion,
  #[error("pi-ai {version} gave invalid data for {name}: {source}")]
  PiData { version: String, name: String, source: serde_json::Error },
  #[error("pi-ai {version} lists no model of {name}")]
  PiEmpty { version: String, name: String },
  #[error("the status is not JSON: {source}")]
  Status { source: serde_json::Error },
  #[error("{0}")]
  Write(#[from] disk::Error),
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

#[expect(clippy::expect_used, reason = "the snapshot is a file of the build, which a test reads")]
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
async fn update() -> Status {
  let Some(dir) = cache() else {
    let mut status = status();
    let why = Error::NoCache.to_string();
    (status.models_dev.error, status.pi_ai.error) = (Some(why.clone()), Some(why));
    return status;
  };
  let mut kept = stored(&dir);
  let fetched = stamp(Timestamp::now());
  match models_dev(&dir).await {
    Ok(()) => {
      kept.models_dev = Source { version: None, fetched: Some(fetched.clone()), error: None }
    }
    Err(why) => kept.models_dev.error = Some(why.to_string()),
  }
  match pi_ai(&dir).await {
    Ok(version) => {
      kept.pi_ai = Source { version: Some(version), fetched: Some(fetched), error: None }
    }
    Err(why) => kept.pi_ai.error = Some(why.to_string()),
  }
  let saved = serde_json::to_string_pretty(&kept)
    .map_err(|source| Error::Status { source })
    .and_then(|text| keep(&dir.join(STATUS), &text));
  let mut status = status();
  if let Err(why) = saved {
    // The status that stands is the old one, so it says that it could not be kept.
    for source in [&mut status.models_dev, &mut status.pi_ai] {
      source.error.get_or_insert_with(|| why.to_string());
    }
  }
  status
}

/// [`update`], for a host that does not run on the reactor.
pub fn updated() -> Status {
  let (send, got) = mpsc::channel();
  reactor().spawn(async move {
    // The caller waits on the other end until this send, so it cannot fail.
    drop(send.send(update().await));
  });
  got.recv().unwrap_or_else(|_| status())
}

/// What brings the cache up to date in the background, and the time of its last attempt.
struct Updater {
  last: Mutex<Option<Instant>>,
  run: fn(),
}

impl Updater {
  fn new(run: fn()) -> Self {
    Self { last: Mutex::new(None), run }
  }

  /// Run the update when the copy of models.dev at a path is a day old or absent, and the last
  /// attempt is an hour old, so no life waits on the network and a source that fails is not asked
  /// at each stand.
  fn background(&self, at: &Path) {
    let age = fs::metadata(at).ok().and_then(|made| made.modified().ok()?.elapsed().ok());
    if age.is_some_and(|age| age <= STALE) {
      return;
    }
    let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
    if last.is_some_and(|then| then.elapsed() < BACKOFF) {
      return;
    }
    *last = Some(Instant::now());
    (self.run)();
  }
}

/// The updater of the process, which a host injects with [`keep_fresh`]. A process that injects
/// none, as a test or a program that embeds the crate, never updates in the background.
static UPDATER: OnceLock<Updater> = OnceLock::new();

/// Update the models of this process in the background from now on: a catalog that stands on a
/// cache a day old starts an update for the next life. A host calls it once at its start.
pub fn keep_fresh() {
  UPDATER.get_or_init(|| {
    Updater::new(|| {
      runtime().spawn(async {
        update().await;
      });
    })
  });
}

/// The update that the injected updater makes when the copy at a path calls for it.
pub(super) fn background(at: &Path) {
  if let Some(updater) = UPDATER.get() {
    updater.background(at);
  }
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
      held.renew(one.models);
    }
  }
}

async fn get(url: &str) -> Result<String, Error> {
  let fetch = |source| Error::Fetch { url: url.to_owned(), source };
  let got = ReqwestClient::new()
    .get(url)
    .header("user-agent", "furb")
    .timeout(Duration::from_mins(1))
    .send()
    .await
    .and_then(reqwest::Response::error_for_status)
    .map_err(fetch)?;
  got.text().await.map_err(fetch)
}

fn keep(at: &Path, text: &str) -> Result<(), Error> {
  Ok(disk::write(at, text.as_bytes(), Access::Shared)?)
}

async fn models_dev(dir: &Path) -> Result<(), Error> {
  let text = get(URL).await?;
  let read = serde_json::from_str::<HashMap<String, Cached>>(&text)
    .map_err(|source| Error::ModelsDev { source })?;
  if read.values().filter(|one| !one.models.is_empty()).count() < 5 {
    return Err(Error::TooFew);
  }
  keep(&dir.join(MODELS), &text)
}

async fn pi_ai(dir: &Path) -> Result<String, Error> {
  let latest: Value =
    serde_json::from_str(&get(REGISTRY).await?).map_err(|source| Error::Registry { source })?;
  let version = latest.get("version").and_then(Value::as_str).ok_or(Error::NoVersion)?.to_owned();
  let mut providers = Map::new();
  for name in &bundled().pi_ai.providers {
    let text = get(&format!("{FILES}@{version}/dist/providers/data/{name}.json")).await?;
    let file: Value = serde_json::from_str(&text).map_err(|source| Error::PiData {
      version: version.clone(),
      name: name.clone(),
      source,
    })?;
    let mut models = Map::new();
    for api in file.as_object().into_iter().flat_map(Map::values) {
      for one in api.as_object().into_iter().flat_map(Map::values) {
        if let (Some(id), Some(model)) = (one.get("id").and_then(Value::as_str), pi(one)) {
          models.insert(id.to_owned(), model);
        }
      }
    }
    if models.is_empty() {
      return Err(Error::PiEmpty { version, name: name.clone() });
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

/// The time at an instant, to the second, as `2026-10-26T18:09:17Z`.
fn stamp(at: Timestamp) -> String {
  at.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
#[path = "update.test.rs"]
mod test;
