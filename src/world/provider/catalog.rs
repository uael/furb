//! The catalog of models: the models that this machine can ask, and of each its window, its efforts, its price, and
//! whether it takes an image.
//!
//! The crate carries a snapshot of the catalog of models.dev, cut down to the providers that rig serves, which
//! `script/catalog.py` makes again. A life reads the models of those providers from the cache of furb when a life
//! before it refreshed the cache, and a life refreshes the cache in the background when it is a day old, so no life
//! waits on the network and a life with no network reads the snapshot. The claude command line is the provider
//! `claude-cli`, whose models are opus, sonnet, haiku and fable.
//!
//! A model is named `provider:id`. The catalog offers the models of a provider when its credentials stand in the
//! environment, and the models of the claude command line when it finds the program. An effort is one of [`LEVELS`],
//! which each provider reads in its own words, and a level that a model does not take moves to the nearest one it
//! takes.

use std::{
  collections::{HashMap, HashSet},
  env, fs,
  path::PathBuf,
  sync::{Arc, OnceLock},
  time::Duration,
};

use indexmap::IndexMap;
use rig_core::{
  client::CompletionClient,
  completion::CompletionError,
  http_client::ReqwestClient,
  providers::{
    anthropic, deepseek, gemini, groq, huggingface, minimax, mistral, moonshot, openai, openrouter,
    together, xai, xiaomimimo, zai,
  },
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{
  Hosted, Model, Provider, Streams,
  claude::{self, Claude},
  runtime, streams,
};

/// The levels of effort, from least to most, which an actor names after its model, as `claude-cli:opus/low`.
pub const LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

/// The snapshot of the catalog that the crate carries.
const SNAPSHOT: &str = include_str!("catalog.json");

/// Where models.dev serves its catalog.
const URL: &str = "https://models.dev/api.json";

/// How old the cache grows before a life refreshes it.
const STALE: Duration = Duration::from_secs(24 * 60 * 60);

/// One provider of the snapshot: the client of rig that asks it, the names of its credentials, its address when the
/// client asks one, and its models.
#[derive(Deserialize)]
pub(super) struct Listed {
  rig: String,
  env: Vec<String>,
  #[serde(default)]
  api: Option<String>,
  models: IndexMap<String, Listing>,
}

/// One provider as the cache holds it, which is the catalog of models.dev whole: its models alone count.
#[derive(Deserialize)]
struct Cached {
  #[serde(default)]
  models: IndexMap<String, Listing>,
}

/// One model of the catalog, in the words of models.dev.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Listing {
  reasoning: bool,
  reasoning_options: Vec<Reasoning>,
  limit: Limit,
  cost: Option<Cost>,
  modalities: Modalities,
  status: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Reasoning {
  r#type: String,
  values: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Limit {
  context: u64,
  output: u64,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(default)]
struct Cost {
  input: f64,
  output: f64,
  cache_read: f64,
  cache_write: f64,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Modalities {
  input: Vec<String>,
  output: Vec<String>,
}

/// How a provider reads an effort: the words of its request that think, and whether it can turn thought off.
#[derive(Clone, Copy, PartialEq)]
enum Dialect {
  Anthropic,
  Responses,
  Gemini,
  OpenRouter,
  Chat,
}

/// The models this machine can ask, each with whether the catalog offers it.
pub struct Catalog {
  models: Vec<(Model, bool)>,
}

impl Catalog {
  /// The catalog of this machine: the snapshot, with the models the cache holds of its providers, and the claude
  /// command line when it finds one. A cache a day old is refreshed in the background, for the next life.
  pub fn load() -> Catalog {
    let cached = cache().map(|at| at.join("models.json"));
    Catalog::of(listed(cached.as_ref()), Claude::found(), |name| env::var(name).ok())
  }

  /// The catalog with this claude command line, which it offers.
  pub fn with_claude(self, claude: Claude) -> Catalog {
    let prefix = format!("{}:", claude::CLAUDE);
    let network = self.models.into_iter().filter(|(model, _)| !model.name.starts_with(&prefix));
    let mut models: Vec<_> = claude.models().into_iter().map(|model| (model, true)).collect();
    models.extend(network);
    Catalog { models }
  }

  /// The catalog of these providers and of the claude command line, which it offers when it found the program, and
  /// each provider offered when a credential of it stands under one of its names.
  pub(super) fn of(
    listed: &IndexMap<String, Listed>,
    claude: Option<Claude>,
    credential: impl Fn(&str) -> Option<String>,
  ) -> Catalog {
    let found = claude.is_some();
    let claude = claude.unwrap_or_default().models().into_iter();
    let mut models: Vec<_> = claude.map(|model| (model, found)).collect();
    for (provider, one) in listed {
      let key = one.env.iter().find_map(|name| credential(name).filter(|key| !key.is_empty()));
      let source = Arc::new(Source {
        rig: one.rig.clone(),
        env: one.env.clone(),
        api: one.api.clone(),
        key: key.clone(),
      });
      let each = one.models.iter().filter(|(_, listing)| listing.kept());
      models
        .extend(each.map(|(id, listing)| (model(provider, id, listing, &source), key.is_some())));
    }
    Catalog { models }
  }

  /// Every model the catalog offers, by its name.
  pub fn offered(&self) -> Vec<&Model> {
    self.models.iter().filter(|(_, offered)| *offered).map(|(model, _)| model).collect()
  }

  /// The model a name names: its name, `provider:id`, or an id that one model of the catalog alone holds.
  pub fn find(&self, name: &str) -> Option<&Model> {
    let models = || self.models.iter().map(|(model, _)| model);
    if let Some(model) = models().find(|model| model.name == name) {
      return Some(model);
    }
    let mut found =
      models().filter(|model| model.name.split_once(':').is_some_and(|(_, id)| id == name));
    match (found.next(), found.next()) {
      (Some(model), None) => Some(model),
      _ => None,
    }
  }

  /// The model and the level an actor names: a model alone, or a model and a level after a slash.
  fn actor<'a>(&self, actor: &'a str) -> Option<(&Model, Option<&'a str>)> {
    if let Some(model) = self.find(actor) {
      return Some((model, None));
    }
    let (name, level) = actor.rsplit_once('/')?;
    Some((self.find(name)?, Some(level)))
  }

  /// The models of a roster, and its default actor: the models the host names, or every model the catalog offers
  /// when it names none, with the model of the actor first when that one is not among them; and the actor, named
  /// as the catalog names its model, with its level moved to the nearest one the model takes.
  pub fn roster(
    &self,
    names: Option<&[String]>,
    actor: Option<&str>,
  ) -> Result<(Vec<Model>, Option<String>), String> {
    let unknown = |name: &str| format!("No model is {name}. Name one as provider:id.");
    let mut models = match names {
      Some(names) => names
        .iter()
        .map(|name| self.find(name).cloned().ok_or_else(|| unknown(name)))
        .collect::<Result<Vec<_>, _>>()?,
      None => self.offered().into_iter().cloned().collect(),
    };
    let actor = match actor {
      Some(actor) => {
        let (model, level) = self.actor(actor).ok_or_else(|| unknown(actor))?;
        if !models.iter().any(|one| one.name == model.name) {
          models.insert(0, model.clone());
        }
        let efforts: Vec<&str> = model.efforts.iter().map(|(name, _)| name.as_str()).collect();
        Some(match level.and_then(|level| clamp(&efforts, level)) {
          Some(level) => format!("{}/{level}", model.name),
          None => model.name.clone(),
        })
      }
      None => None,
    };
    let mut seen = HashSet::new();
    models.retain(|model| seen.insert(model.name.clone()));
    Ok((models, actor))
  }

  /// The provider of a roster of the catalog on a directory, with the default actor of the roster, whose models a
  /// function of the host answers in place of the models when it gives one.
  pub fn provider(
    &self,
    directory: impl Into<String>,
    names: Option<&[String]>,
    actor: Option<&str>,
    host: Option<Hosted>,
  ) -> Result<Provider, String> {
    let (models, actor) = self.roster(names, actor)?;
    let models = match host {
      Some(host) => models.into_iter().map(|model| model.hosted(Arc::clone(&host))).collect(),
      None => models,
    };
    Ok(Provider::new(directory, models).actor(actor))
  }
}

/// The level of these efforts nearest a level: the level itself when a model takes it, the next one up that it
/// takes, or the next one down; and nothing for a model that takes no effort.
pub fn clamp<'a>(efforts: &[&'a str], level: &str) -> Option<&'a str> {
  if let Some(found) = efforts.iter().find(|one| **one == level) {
    return Some(found);
  }
  let Some(at) = LEVELS.iter().position(|one| *one == level) else {
    return efforts.first().copied();
  };
  let mut near = LEVELS[at..].iter().chain(LEVELS[..at].iter().rev());
  near.find_map(|one| efforts.iter().find(|taken| *taken == one).copied())
}

impl Listing {
  /// Whether the catalog keeps a model: one that reads and writes text, and is not deprecated.
  fn kept(&self) -> bool {
    let text = |sides: &[String]| sides.iter().any(|one| one == "text");
    self.status.as_deref() != Some("deprecated")
      && text(&self.modalities.input)
      && text(&self.modalities.output)
  }

  /// Whether its options of reasoning hold one of this type.
  fn reasons(&self, kind: &str) -> bool {
    self.reasoning && self.reasoning_options.iter().any(|one| one.r#type == kind)
  }

  /// The levels the model takes by their names, `none` being `off`.
  fn named(&self) -> Vec<&'static str> {
    let words =
      self.reasoning_options.iter().filter(|one| self.reasoning && one.r#type == "effort");
    let words: Vec<&str> = words
      .flat_map(|one| &one.values)
      .map(|word| if word == "none" { "off" } else { word })
      .collect();
    LEVELS.into_iter().filter(|level| words.contains(level)).collect()
  }

  /// The efforts it takes, each with the settings that a provider of this dialect reads it by. The levels are those
  /// the model names; a model that counts its thought by a budget takes the levels of a budget where the provider
  /// reads a budget; and a model that can think or not takes `off` too where the provider can turn thought off.
  fn efforts(&self, dialect: Dialect) -> Vec<(String, Map<String, Value>)> {
    let named = self.named();
    let budgets = matches!(dialect, Dialect::Anthropic | Dialect::Gemini | Dialect::OpenRouter)
      && self.reasons("budget_tokens");
    let quiets = matches!(dialect, Dialect::Anthropic | Dialect::OpenRouter)
      && (self.reasons("toggle") || budgets)
      && (budgets || !named.is_empty());
    let taken = LEVELS.into_iter().filter(|level| {
      named.contains(level)
        || (named.is_empty() && budgets && ["minimal", "low", "medium", "high"].contains(level))
        || (*level == "off" && quiets)
    });
    let each = taken.map(|level| (level, self.settings(dialect, level, named.contains(&level))));
    each.map(|(level, settings)| (level.to_owned(), settings)).collect()
  }

  /// The settings of a request that make the model think at a level, in the words of a dialect: `named` when the
  /// model takes the level by its name, and by a budget of thought otherwise.
  fn settings(&self, dialect: Dialect, level: &str, named: bool) -> Map<String, Value> {
    let word = if level == "off" { "none" } else { level };
    let thought = json!({"type": "enabled", "budget_tokens": budget(level)});
    let settings = match dialect {
      Dialect::Anthropic if level == "off" && self.reasons("toggle") => {
        json!({"thinking": {"type": "disabled"}})
      }
      Dialect::Anthropic if level == "off" => json!({}),
      Dialect::Anthropic if named && self.reasons("budget_tokens") => {
        json!({"output_config": {"effort": level}, "thinking": thought})
      }
      Dialect::Anthropic if named => {
        json!({"output_config": {"effort": level}, "thinking": {"type": "adaptive"}})
      }
      Dialect::Anthropic => json!({"thinking": thought}),
      Dialect::Responses => json!({"reasoning": {"effort": word}}),
      Dialect::Gemini => {
        let config = if named {
          json!({"thinkingLevel": level, "includeThoughts": true})
        } else {
          json!({"thinkingBudget": budget(level), "includeThoughts": true})
        };
        json!({"generationConfig": {"thinkingConfig": config}})
      }
      Dialect::OpenRouter if named => json!({"reasoning": {"effort": word}}),
      Dialect::OpenRouter if level == "off" => json!({"reasoning": {"enabled": false}}),
      Dialect::OpenRouter => json!({"reasoning": {"max_tokens": budget(level)}}),
      Dialect::Chat => json!({"reasoning_effort": word}),
    };
    match settings {
      Value::Object(settings) => settings,
      _ => Map::new(),
    }
  }
}

/// The tokens of thought a level spends on a model that counts its thought by a budget: a budget for each level from
/// minimal to high, and the budget of high for a level past it.
fn budget(level: &str) -> u64 {
  match level {
    "minimal" => 1024,
    "low" => 2048,
    "medium" => 8192,
    _ => 16_384,
  }
}

/// A provider of the network: the client of rig that asks it, the names of its credentials, its address, and the
/// credential that stands in the environment.
struct Source {
  rig: String,
  env: Vec<String>,
  api: Option<String>,
  key: Option<String>,
}

impl Source {
  fn dialect(&self) -> Dialect {
    match self.rig.as_str() {
      "anthropic" => Dialect::Anthropic,
      "openai" | "xai" => Dialect::Responses,
      "gemini" => Dialect::Gemini,
      "openrouter" => Dialect::OpenRouter,
      _ => Dialect::Chat,
    }
  }

  /// The model of rig that asks this provider for a model, made from its credential.
  fn streams(&self, id: &str) -> Result<Streams, String> {
    let Some(key) = self.key.clone() else {
      return Err(format!(
        "no credential of it stands in the environment: set {}",
        self.env.join(" or ")
      ));
    };
    macro_rules! asked {
      ($client:ty) => {
        asked!($client, None::<String>)
      };
      ($client:ty, $api:expr) => {{
        let mut builder = <$client>::builder().api_key(key);
        if let Some(api) = $api {
          builder = builder.base_url(api);
        }
        builder.build().map_err(|no| no.to_string())?.completion_model(id)
      }};
    }
    Ok(match self.rig.as_str() {
      "anthropic" => {
        streams(asked!(anthropic::Client, self.api.as_deref()).with_automatic_caching())
      }
      "openai" => streams(asked!(openai::Client)),
      "gemini" => streams(asked!(gemini::Client)),
      "openrouter" => streams(asked!(openrouter::Client)),
      "groq" => streams(asked!(groq::Client)),
      "xai" => streams(asked!(xai::Client)),
      "mistral" => streams(asked!(mistral::Client)),
      "deepseek" => streams(asked!(deepseek::Client)),
      "together" => streams(asked!(together::Client)),
      "moonshot" => streams(asked!(moonshot::Client)),
      "zai" => streams(asked!(zai::Client)),
      "minimax" => streams(asked!(minimax::Client)),
      "huggingface" => streams(asked!(huggingface::Client)),
      "xiaomi" => streams(asked!(xiaomimimo::Client)),
      "chat" => streams(asked!(openai::CompletionsClient, self.api.as_deref())),
      other => return Err(format!("rig serves no provider {other}")),
    })
  }
}

/// A model of a provider of the network, whose client is made when it is first asked.
fn model(provider: &str, id: &str, listing: &Listing, source: &Arc<Source>) -> Model {
  let dialect = source.dialect();
  let made: Arc<OnceLock<Result<Streams, String>>> = Arc::default();
  let (asked, id) = (Arc::clone(source), id.to_owned());
  let name = format!("{provider}:{id}");
  let mut model = Model::lazy(name, listing.limit.context, move || {
    let got = made.get_or_init(|| asked.streams(&id));
    got.clone().map_err(|no| CompletionError::ProviderError(no.clone()))
  });
  model.efforts = listing.efforts(dialect);
  model.images = listing.modalities.input.iter().any(|one| one == "image");
  model.price =
    listing.cost.map(|cost| [cost.input, cost.output, cost.cache_read, cost.cache_write]);
  model.apart = dialect == Dialect::Anthropic;
  model.tokens =
    (dialect == Dialect::Anthropic && listing.limit.output > 0).then_some(listing.limit.output);
  model
}

/// The providers of the snapshot, with the models of each that the cache at a path holds, read once in a process;
/// and the cache refreshed in the background when it is a day old.
fn listed(cached: Option<&PathBuf>) -> &'static IndexMap<String, Listed> {
  static LISTED: OnceLock<IndexMap<String, Listed>> = OnceLock::new();
  LISTED.get_or_init(|| {
    let age = cached.and_then(|at| fs::metadata(at).ok()?.modified().ok()?.elapsed().ok());
    if let Some(at) = cached.filter(|_| !cfg!(test) && age.is_none_or(|age| age > STALE)) {
      runtime().spawn(refreshed(at.clone()));
    }
    let text = cached.and_then(|at| fs::read_to_string(at).ok());
    parsed(SNAPSHOT, text.as_deref())
  })
}

/// The providers of a snapshot, each with the models the cache holds of it when the cache holds it.
pub(super) fn parsed(snapshot: &str, cached: Option<&str>) -> IndexMap<String, Listed> {
  let mut listed: IndexMap<String, Listed> = serde_json::from_str(snapshot)
    .expect("the snapshot of the catalog is the JSON of its providers");
  let cached = cached.and_then(|text| serde_json::from_str::<HashMap<String, Cached>>(text).ok());
  for (provider, one) in cached.into_iter().flatten() {
    if let Some(held) = listed.get_mut(&provider).filter(|_| !one.models.is_empty()) {
      held.models = one.models;
    }
  }
  listed
}

/// The catalog of models.dev, fetched and kept in the cache at a path, when it reads as one.
async fn refreshed(at: PathBuf) {
  let asked = ReqwestClient::new().get(URL).header("user-agent", "furb").send();
  let Ok(Ok(got)) = asked.await.map(|got| got.error_for_status()) else { return };
  let Ok(text) = got.text().await else { return };
  if serde_json::from_str::<HashMap<String, Cached>>(&text).is_err() {
    return;
  }
  let partial = at.with_extension("json.part");
  let wrote = at.parent().is_some_and(|parent| fs::create_dir_all(parent).is_ok())
    && fs::write(&partial, text).is_ok();
  if wrote {
    let _ = fs::rename(&partial, &at);
  }
}

/// The cache of furb on this machine: under `XDG_CACHE_HOME`, `LOCALAPPDATA` on Windows, `Library/Caches` of the
/// home on macOS, and `.cache` of the home elsewhere.
fn cache() -> Option<PathBuf> {
  let named = |name: &str| env::var_os(name).filter(|one| !one.is_empty()).map(PathBuf::from);
  let base = named("XDG_CACHE_HOME").or_else(|| {
    if cfg!(windows) {
      named("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
      env::home_dir().map(|home| home.join("Library/Caches"))
    } else {
      env::home_dir().map(|home| home.join(".cache"))
    }
  });
  base.map(|base| base.join("furb"))
}

#[cfg(test)]
#[path = "catalog.test.rs"]
mod test;
