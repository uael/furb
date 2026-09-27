//! The catalog of models: the models that this machine can ask, and of each its window, its efforts, its price, and
//! whether it takes an image.
//!
//! The crate carries a snapshot of the catalog, cut down to the providers that pi-ai serves, which
//! `script/catalog.py` makes again from models.dev and, for the two providers that models.dev does not list, from the
//! data of pi-ai. A life reads the models of a provider of models.dev from the cache of furb when the cache holds
//! them newer than the snapshot, and a life refreshes the cache in the background when it is a day old, so no life
//! waits on the network and a life with no network reads the snapshot. The claude command line is the provider
//! `claude-cli`, whose models are opus, sonnet, haiku and fable.
//!
//! A model is named `provider:id`. The catalog offers the models of a provider when its credential stands in the
//! environment, with every name that its address holds a place for, and the models of the claude command line when
//! it finds the program. An effort is one of [`LEVELS`], which each provider reads in its own words, and a level that
//! a model does not take moves to the nearest one it takes.

use std::{
  collections::{HashMap, HashSet},
  env, fs,
  path::PathBuf,
  sync::{Arc, OnceLock},
  time::Duration,
};

use indexmap::IndexMap;
use rig_core::{completion::CompletionError, http_client::ReqwestClient};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{
  Hosted, Model, Provider, Streams,
  claude::{self, Claude},
  clients::{Client, Reach},
  runtime,
};

/// The levels of effort, from least to most, which an actor names after its model, as `claude-cli:opus/low`.
pub const LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

/// The snapshot of the catalog that the crate carries.
const SNAPSHOT: &str = include_str!("catalog.json");

/// Where models.dev serves its catalog.
const URL: &str = "https://models.dev/api.json";

/// How old the cache grows before a life refreshes it.
const STALE: Duration = Duration::from_secs(24 * 60 * 60);

/// One provider of the snapshot: the client of rig that asks it, the names of its credential, those whose credential
/// goes as a bearer, its address, the names of the environment that a place of an address reads before its own, and
/// its models.
#[derive(Deserialize)]
pub(super) struct Listed {
  rig: Client,
  env: Vec<String>,
  #[serde(default)]
  bearer: Vec<String>,
  #[serde(default)]
  api: Option<String>,
  #[serde(default)]
  aliases: HashMap<String, String>,
  models: IndexMap<String, Listing>,
}

/// One provider as the cache holds it, which is the catalog of models.dev whole: its models alone count.
#[derive(Deserialize)]
struct Cached {
  #[serde(default)]
  models: IndexMap<String, Listing>,
}

/// One model of the catalog, in the words of models.dev: `provider` names the package and the address of a model
/// that its provider serves apart.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Listing {
  reasoning: bool,
  reasoning_options: Vec<Reasoning>,
  limit: Limit,
  cost: Option<Cost>,
  modalities: Modalities,
  status: Option<String>,
  last_updated: Option<String>,
  provider: Option<Own>,
}

/// The package that asks a model apart from its provider, the address it asks, and the shape of its request.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Own {
  npm: String,
  api: Option<String>,
  shape: Option<String>,
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

/// How a provider reads an effort: the words of its request that think, and whether it can turn thought off. A
/// provider of the silent dialect reads no effort.
#[derive(Clone, Copy, PartialEq)]
enum Dialect {
  Anthropic,
  Responses,
  Gemini,
  OpenRouter,
  Chat,
  Pi,
  Silent,
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

  /// The catalog with the claude command line at a path, or the one of this machine when only a stall is given, which
  /// it offers, and whose turn may go with no progress for the stall; and the catalog as it stands when neither is
  /// given, or when it finds no program.
  pub fn claude(self, bin: Option<PathBuf>, stall: Option<Duration>) -> Catalog {
    let Some(bin) = bin.or_else(|| stall.and_then(|_| claude::located())) else { return self };
    let claude = Claude::with(Some(bin), stall);
    let prefix = format!("{}:", claude::CLAUDE);
    let network = self.models.into_iter().filter(|(model, _)| !model.name.starts_with(&prefix));
    let mut models: Vec<_> = claude.models().into_iter().map(|model| (model, true)).collect();
    models.extend(network);
    Catalog { models }
  }

  /// The catalog of these providers and of the claude command line, which it offers when it found the program, and
  /// each model offered when the environment holds its credential and each name its address holds a place for.
  pub(super) fn of(
    listed: &IndexMap<String, Listed>,
    claude: Option<Claude>,
    env: impl Fn(&str) -> Option<String>,
  ) -> Catalog {
    let env = |name: &str| env(name).filter(|one| !one.is_empty());
    let found = claude.is_some();
    let claude = claude.unwrap_or_default().models().into_iter();
    let mut models: Vec<_> = claude.map(|model| (model, found)).collect();
    for (provider, one) in listed {
      let credential = one.credential(&env);
      for (id, listing) in &one.models {
        let Some(client) = listing.client(one.rig).filter(|_| listing.kept()) else { continue };
        let reach = credential.clone().and_then(|key| one.reach(id, listing, client, key, &env));
        let offered = reach.is_ok();
        models.push((model(provider, id, listing, client, reach), offered));
      }
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

  /// The models of a roster, and its default actor: the model of the actor, and the models the host names; or, when
  /// the host names no roster, the model of the actor alone, and the first model the catalog offers when there is no
  /// actor either. The actor is named as the catalog names its model, with its level moved to the nearest one the
  /// model takes.
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
      None if actor.is_none() => self.offered().into_iter().take(1).cloned().collect(),
      None => Vec::new(),
    };
    let actor = match actor {
      Some(actor) => {
        let (model, level) = self.actor(actor).ok_or_else(|| unknown(actor))?;
        models.insert(0, model.clone());
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

  /// The client of rig that asks the model where its provider stands: the client of its provider, or that of its own
  /// package; and none for a package that no client of rig serves there. The gateway of Cloudflare asks every model
  /// itself.
  fn client(&self, provider: Client) -> Option<Client> {
    let Some(own) = &self.provider else { return Some(provider) };
    match (provider, own.npm.as_str()) {
      (Client::Gateway, _) => Some(Client::Gateway),
      (Client::Bedrock | Client::Vertex, _) => None,
      _ if own.shape.as_deref() == Some("completions") => None,
      (_, "@ai-sdk/anthropic") => Some(Client::Anthropic),
      (_, "@ai-sdk/openai") => Some(Client::Openai),
      (_, "@ai-sdk/openai-compatible") => Some(Client::Chat),
      _ => None,
    }
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
    if dialect == Dialect::Silent {
      return Vec::new();
    }
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
      Dialect::Pi if level == "off" => json!({}),
      Dialect::Pi => json!({"reasoning": level}),
      Dialect::Silent => json!({}),
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

impl Listed {
  /// The credential of the provider and whether it goes as a bearer, or why the environment holds none. The clients
  /// of Amazon and Google read their credentials themselves: those of Google are its key of an API, or its
  /// credentials of an application where its client finds them, under the application data on Windows and under
  /// the home elsewhere.
  fn credential(&self, env: &impl Fn(&str) -> Option<String>) -> Result<(String, bool), String> {
    let named = self.env.iter().find_map(|name| env(name).map(|key| (name, key)));
    let adc = || {
      let (root, gcloud) =
        if cfg!(windows) { ("APPDATA", "gcloud") } else { ("HOME", ".config/gcloud") };
      let root = env(root).map(PathBuf::from);
      root.is_some_and(|root| {
        root.join(gcloud).join("application_default_credentials.json").is_file()
      })
    };
    match (self.rig, named) {
      (Client::Bedrock, Some(_)) => Ok((String::new(), false)),
      (Client::Vertex, Some(_)) => Ok((env("GOOGLE_CLOUD_API_KEY").unwrap_or_default(), false)),
      (Client::Vertex, None) if adc() => Ok((String::new(), false)),
      (_, Some((name, key))) => Ok((key, self.bearer.contains(name))),
      (_, None) => {
        Err(format!("no credential of it stands in the environment: set {}", self.env.join(" or ")))
      }
    }
  }

  /// Where a model of the provider is asked, with its credential: its address, with each place filled from the
  /// environment, the name that Azure knows it by, and the project and the location of Google; or why the
  /// environment does not say it.
  fn reach(
    &self,
    id: &str,
    listing: &Listing,
    client: Client,
    (key, bearer): (String, bool),
    env: &impl Fn(&str) -> Option<String>,
  ) -> Result<Reach, String> {
    let own = listing.provider.as_ref().and_then(|own| own.api.clone());
    let based = env("AZURE_OPENAI_BASE_URL").filter(|_| client == Client::Azure);
    let api = match based {
      Some(base) => Some(
        base.trim_end_matches('/').trim_end_matches("/v1").trim_end_matches("/openai").to_owned(),
      ),
      None => own.or_else(|| self.api.clone()).map(|api| self.filled(&api, env)).transpose()?,
    };
    // Azure knows a model by the name of its deployment, which the map of the environment gives as `id=name`.
    let deployments = env("AZURE_OPENAI_DEPLOYMENT_NAME_MAP").filter(|_| client == Client::Azure);
    let deployed = deployments.unwrap_or_default().split(',').find_map(|one| {
      let (model, name) = one.split_once('=')?;
      (model.trim() == id).then(|| name.trim().to_owned())
    });
    let project = match client {
      Client::Vertex => {
        let project = env("GOOGLE_CLOUD_PROJECT").or_else(|| env("GCLOUD_PROJECT"));
        let project = project
          .ok_or("no project of Google stands in the environment: set GOOGLE_CLOUD_PROJECT")?;
        Some((project, env("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|| "global".to_owned())))
      }
      _ => None,
    };
    let version = env("AZURE_OPENAI_API_VERSION").filter(|_| client == Client::Azure);
    let id = deployed.unwrap_or_else(|| id.to_owned());
    Ok(Reach { key, bearer, api, id, version, project })
  }

  /// An address with each place `${NAME}` filled from the environment, which reads the name of pi-ai for the place
  /// before the name itself; or the first name that the environment does not hold.
  fn filled(&self, api: &str, env: &impl Fn(&str) -> Option<String>) -> Result<String, String> {
    let mut filled = api.to_owned();
    while let Some(at) = filled.find("${") {
      let Some(end) = filled[at..].find('}').map(|end| at + end) else { break };
      let name = filled[at + 2..end].to_owned();
      let value = self.aliases.get(&name).and_then(|alias| env(alias)).or_else(|| env(&name));
      let value =
        value.ok_or_else(|| format!("its address needs {name}: set it in the environment"))?;
      filled.replace_range(at..=end, &value);
    }
    Ok(filled)
  }
}

impl Client {
  /// How the provider of this client reads an effort for a model: the models of Anthropic on Amazon read it as
  /// Anthropic does, and GitHub Copilot and the other models of Amazon read none.
  fn dialect(self, id: &str) -> Dialect {
    match self {
      Client::Anthropic => Dialect::Anthropic,
      Client::Bedrock if id.contains("anthropic.") => Dialect::Anthropic,
      Client::Openai | Client::Xai => Dialect::Responses,
      Client::Gemini | Client::Vertex => Dialect::Gemini,
      Client::Openrouter => Dialect::OpenRouter,
      Client::Pi => Dialect::Pi,
      Client::Bedrock | Client::Copilot => Dialect::Silent,
      _ => Dialect::Chat,
    }
  }
}

/// A model of a provider of the network, whose client is made when it is first asked, or why it cannot be. A model
/// of Anthropic, of Amazon or of pi-ai counts what it read of the cache apart, and is told the most it writes.
fn model(
  provider: &str,
  id: &str,
  listing: &Listing,
  client: Client,
  reach: Result<Reach, String>,
) -> Model {
  let dialect = client.dialect(id);
  let made: Arc<OnceLock<Result<Streams, String>>> = Arc::default();
  let mut model = Model::lazy(format!("{provider}:{id}"), listing.limit.context, move || {
    let got = made.get_or_init(|| reach.clone().and_then(|reach| client.streams(&reach)));
    got.clone().map_err(CompletionError::ProviderError)
  });
  model.efforts = listing.efforts(dialect);
  model.images = listing.modalities.input.iter().any(|one| one == "image");
  model.price =
    listing.cost.map(|cost| [cost.input, cost.output, cost.cache_read, cost.cache_write]);
  model.apart = matches!(client, Client::Anthropic | Client::Bedrock | Client::Pi);
  let told = matches!(client, Client::Anthropic | Client::Bedrock) && listing.limit.output > 0;
  model.tokens = told.then_some(listing.limit.output);
  match client {
    Client::Pi => model.conversation("sessionId"),
    _ => model,
  }
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

/// The providers of a snapshot, each with the models of the cache when the cache holds models of it as new as those
/// of the snapshot or newer. The last day that a model of a list changed says how new the list is.
pub(super) fn parsed(snapshot: &str, cached: Option<&str>) -> IndexMap<String, Listed> {
  let mut listed: IndexMap<String, Listed> = serde_json::from_str(snapshot)
    .expect("the snapshot of the catalog is the JSON of its providers");
  let cached = cached.and_then(|text| serde_json::from_str::<HashMap<String, Cached>>(text).ok());
  let newest = |models: &IndexMap<String, Listing>| {
    models.values().filter_map(|one| one.last_updated.clone()).max()
  };
  for (provider, one) in cached.into_iter().flatten() {
    let Some(held) = listed.get_mut(&provider) else { continue };
    if !one.models.is_empty() && newest(&one.models) >= newest(&held.models) {
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
pub(super) fn cache() -> Option<PathBuf> {
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
