//! The catalog of models: the models that this machine can ask, and of each its window, its efforts, its price, and
//! whether it takes an image.
//!
//! The crate carries a snapshot of the catalog, cut down to the providers that pi-ai serves, which
//! `script/catalog.py` makes again from models.dev and, for the two providers that models.dev does not list, from the
//! data of pi-ai. A life reads the models of a provider from the caches of furb when they hold them newer than the
//! snapshot, and a life updates the caches in the background when they are a day old, so no life waits on the network
//! and a life with no network reads the snapshot. [`update`] says how a source is fetched and kept. The claude command line is the provider
//! `claude-cli`, whose models are opus, sonnet, haiku and fable.
//!
//! A model is named `provider:id`. The catalog offers the models of a provider when its credential stands in the
//! environment, with every name that its address holds a place for, and the models of the claude command line when
//! it finds the program. An effort is one of [`LEVELS`], which each provider reads in its own words, an actor that
//! names none takes [`LEVEL`], and a level that a model does not take moves to the nearest one it takes.
//!
//! The models of the local server that the environment names are the provider `local`, which [`local`] lists. The
//! catalog asks that server only when a roster or an actor names a model of the provider local, or of a provider
//! that the catalog does not know, and when a host lists the models that the catalog offers, as a roster that the
//! host does not name does. It offers them after the claude command line and the providers of the network, so the
//! default actor is a model of the local server only when the catalog offers no other model.
//!
//! [`local`]: super::local

mod config;
mod update;

pub use update::{Source, Status, status, updated};

use std::{
  collections::{HashMap, HashSet},
  env, fs,
  path::PathBuf,
  sync::{Arc, OnceLock},
  time::Duration,
};

use indexmap::IndexMap;
use rig_core::completion::CompletionError;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{
  Hosted, Model, Provider, Streams,
  claude::{self, Claude},
  clients::{Client, Reach},
  local::{ENV_URL, Heard, LOCAL, Server},
  search,
};
use crate::world::cache;

/// The levels of effort, from least to most, which an actor names after its model, as `claude-cli:opus/low`.
pub const LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

/// The level of an actor that names none.
pub const LEVEL: &str = "high";

/// The doctrine of furb, which says how a model works in furb and holds no law: `DOCTRINE.md` at the root of the
/// repository. Every model reads it, once on each chain, in the paragraph that asks it for its first word.
pub const DOCTRINE: &str = include_str!("../../../DOCTRINE.md");

/// The snapshot of the catalog that the crate carries.
const SNAPSHOT: &str = include_str!("catalog.json");

/// One provider of the snapshot: the client of rig that asks it, the names of its credential, those whose credential
/// goes as a bearer, its address, the names of the environment that a place of an address reads before its own, and
/// its models.
#[derive(Clone, Deserialize)]
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
  #[serde(skip)]
  key: Option<String>,
}

/// One provider as the cache holds it, which is the catalog of models.dev whole: its models alone count.
#[derive(Deserialize)]
struct Cached {
  #[serde(default)]
  models: IndexMap<String, Listing>,
}

/// One model of the catalog, in the words of models.dev: `provider` names the package and the address of a model
/// that its provider serves apart.
#[derive(Clone, Deserialize, Default)]
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
#[derive(Clone, Deserialize, Default)]
#[serde(default)]
struct Own {
  npm: String,
  api: Option<String>,
  shape: Option<String>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
struct Reasoning {
  r#type: String,
  values: Vec<String>,
}

#[derive(Clone, Deserialize, Default)]
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

#[derive(Clone, Deserialize, Default)]
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

/// The models this machine can ask, each with whether the catalog offers it: those of the claude command line and of
/// the network, the local server that the environment names, which holds what it listed, and the most tokens of
/// thought of a model in one turn that the environment gives as `MAX_THINKING_TOKENS`.
pub struct Catalog {
  models: Vec<(Model, bool)>,
  /// Each provider, with the names of its credential and whether the settings enable it.
  providers: Vec<(String, Vec<String>, bool)>,
  local: Option<Arc<Server>>,
  thinking: Option<String>,
  /// The claude command line that the catalog found, which a first run asks whether it is signed in.
  claude: Option<Claude>,
}

/// What this machine offers through each way to a model, as a first run checks it: the claude command line and
/// whether it is signed in, each configured provider, and the local server that the environment names.
pub struct Setup {
  pub claude: bool,
  /// Whether the claude command line is signed in, which is false when the catalog found none.
  pub signed: bool,
  pub providers: Vec<Ready>,
  pub local: Option<Local>,
}

/// A provider as a first run checks it: its name, the names of its credential, whether the settings enable it, and
/// how many of its models the catalog offers.
pub struct Ready {
  pub provider: String,
  pub env: Vec<String>,
  pub offered: usize,
  /// Whether the provider settings enable this provider.
  pub enabled: bool,
}

/// The local server that the environment names, as a first run checks it: its address, how many models for chat the
/// catalog offers of it, and why it listed none, when it did not answer.
pub struct Local {
  pub address: String,
  pub offered: usize,
  pub failed: Option<String>,
}

impl Catalog {
  /// The catalog of this machine: the snapshot, with the models the cache holds of its providers, the claude command
  /// line when it finds one, and the local server that the environment of the process names, one for the process, so
  /// what it listed holds for every catalog after. A cache a day old is refreshed in the background, for the next
  /// life.
  pub fn load() -> Catalog {
    static SERVER: OnceLock<Option<Arc<Server>>> = OnceLock::new();
    let env = |name: &str| env::var(name).ok();
    let server = SERVER.get_or_init(|| Server::named(env).map(Arc::new)).clone();
    let cached = cache().map(|at| at.join(update::MODELS));
    let listed = listed(cached.as_ref());
    let mut catalog = Catalog::of(&listed, Claude::found(), server, env);
    catalog.providers.push((super::codex::NAME.to_owned(), Vec::new(), false));
    catalog
  }

  /// The catalog with the claude command line at a path, or the one of this machine when only a stall is given, which
  /// it offers, and whose turn may go with no progress for the stall; and the catalog as it stands when neither is
  /// given, or when it finds no program.
  pub fn claude(self, bin: Option<PathBuf>, stall: Option<Duration>) -> Catalog {
    let Some(bin) = bin.or_else(|| stall.and_then(|_| claude::located())) else { return self };
    let claude = Claude::with(Some(bin), stall);
    let found = Some(claude.clone());
    let prefix = format!("{}:", claude::CLAUDE);
    let network = self.models.into_iter().filter(|(model, _)| !model.name.starts_with(&prefix));
    let mut models: Vec<_> = claude.models().into_iter().map(|model| (model, true)).collect();
    models.extend(network);
    Catalog { models, claude: found, ..self }
  }

  /// The catalog of the claude command line, which it offers when it found the program, of these providers, each model
  /// of which it offers when the environment holds its credential and each name its address holds a place for, and of
  /// a local server, which it asks when it needs its models.
  pub(super) fn of(
    listed: &IndexMap<String, Listed>,
    claude: Option<Claude>,
    local: Option<Arc<Server>>,
    env: impl Fn(&str) -> Option<String>,
  ) -> Catalog {
    let env = |name: &str| env(name).filter(|one| !one.is_empty());
    let found = claude.is_some();
    let kept = claude.clone();
    let claude = match claude {
      Some(claude) => claude.models(),
      None => Claude::with(None, None).fixed(),
    };
    let mut models: Vec<_> = claude.into_iter().map(|model| (model, found)).collect();
    let mut providers = Vec::new();
    for (provider, one) in listed {
      providers.push((provider.clone(), one.env.clone(), true));
      let credential = one.credential(&env);
      for (id, listing) in &one.models {
        let Some(client) = listing.client(one.rig).filter(|_| listing.kept()) else { continue };
        let reach = credential.clone().and_then(|key| one.reach(id, listing, client, key, &env));
        let offered = reach.is_ok();
        models.push((model(provider, id, listing, client, reach), offered));
      }
    }
    Catalog { models, providers, local, thinking: env(super::ENV_THINKING), claude: kept }
  }

  /// What a first run checks: the claude command line, each provider of the network, and the local server, which it
  /// asks for what it lists, so it never runs on the thread of the reactor.
  pub fn setup(&self) -> Setup {
    let offered = |provider: &str| {
      let of = |model: &Model| model.name.split_once(':').is_some_and(|(one, _)| one == provider);
      self.models.iter().filter(|(model, offered)| *offered && of(model)).count()
    };
    let providers = self.providers.iter();
    Setup {
      claude: offered(claude::CLAUDE) > 0,
      signed: self.claude.as_ref().is_some_and(Claude::signed),
      providers: providers
        .map(|(provider, env, enabled)| Ready {
          provider: provider.clone(),
          env: env.clone(),
          offered: offered(provider),
          enabled: *enabled,
        })
        .collect(),
      local: self.local.as_ref().map(|server| {
        let heard = server.heard(None);
        Local { address: server.root.clone(), offered: heard.models.len(), failed: heard.failed }
      }),
    }
  }

  /// What the local server lists, for the name that the catalog wants of it, if any; and nothing when the environment
  /// names no server.
  fn heard(&self, wanted: Option<&str>) -> Option<Heard> {
    self.local.as_ref().map(|server| server.heard(wanted))
  }

  /// Every model the catalog offers, by its name: those of the claude command line, then those of the network, then
  /// those of the local server, so the first is the default actor.
  pub fn offered(&self) -> Vec<Model> {
    let offered =
      self.models.iter().filter(|(_, offered)| *offered).map(|(model, _)| model.clone());
    offered.chain(self.heard(None).into_iter().flat_map(|heard| heard.models)).collect()
  }

  /// The model a name names: its name, `provider:id`, or an id that one model alone holds, among the models of the
  /// claude command line and of the network, and then, for a name of the provider local or of a provider that the
  /// catalog does not know, among those of the local server.
  pub fn find(&self, name: &str) -> Option<Model> {
    named(&self.known(), name).cloned().or_else(|| self.served(name))
  }

  /// The models of the claude command line and of the network.
  fn known(&self) -> Vec<&Model> {
    self.models.iter().map(|(model, _)| model).collect()
  }

  /// Whether the local server may hold a model of a name: a name of the provider local, or of a provider that the
  /// catalog does not know.
  fn asks(&self, name: &str) -> bool {
    let Some((provider, _)) = name.split_once(':') else { return false };
    let known = |model: &&Model| model.name.split_once(':').is_some_and(|(one, _)| one == provider);
    provider == LOCAL || !self.known().iter().any(known)
  }

  /// The model of the local server that a name with a colon names: among what the server listed, which the catalog
  /// asks the server for only when the server may hold it, and among what it listed already for any other name, as an
  /// id of Ollama whose part before its colon is the name of a provider of the network.
  fn served(&self, name: &str) -> Option<Model> {
    let server = self.local.as_ref().filter(|_| name.contains(':'))?;
    let heard = if self.asks(name) { server.heard(Some(name)) } else { server.held() };
    named(&heard.models.iter().collect::<Vec<_>>(), name).cloned()
  }

  /// The model and the level an actor names: a model alone, or a model and a level after a slash, among the models of
  /// the claude command line and of the network first, and then among those of the local server.
  fn actor<'a>(&self, actor: &'a str) -> Option<(Model, Option<&'a str>)> {
    let split = actor.rsplit_once('/');
    let known = self.known();
    if let Some(model) = named(&known, actor) {
      return Some((model.clone(), None));
    }
    if let Some((name, level)) = split
      && let Some(model) = named(&known, name)
    {
      return Some((model.clone(), Some(level)));
    }
    if let Some(model) = self.served(actor) {
      return Some((model, None));
    }
    // A model of the local server is named by its id, so a word after its slash is a level of effort or no name of it.
    let (name, level) = split.filter(|(_, level)| LEVELS.contains(level))?;
    Some((self.served(name)?, Some(level)))
  }

  /// The models of a roster, and its default actor: the model of the actor first, then the models the host names, or,
  /// when the host names no roster, every model the catalog offers, in the order of [`Catalog::offered`], whose first
  /// stands for the actor when there is no actor. The actor is named as the catalog names its model, at its level as
  /// [`Model::at`] moves it.
  pub fn roster(
    &self,
    names: Option<&[String]>,
    actor: Option<&str>,
  ) -> Result<(Vec<Model>, Option<String>), String> {
    let unknown = |name: &str| {
      let unnamed = self.local.is_none() && name.starts_with(&format!("{LOCAL}:"));
      let why = unnamed.then(|| format!("no local server is named: set {ENV_URL}"));
      let heard = || self.asks(name).then(|| self.heard(Some(name))).flatten();
      let why = why.or_else(|| Some(heard()?.why(name)?.to_owned()));
      why.map_or_else(
        || format!("No model is {name}. Name one as provider:id."),
        |why| format!("No model is {name}: {why}."),
      )
    };
    let mut models = match names {
      Some(names) => names
        .iter()
        .map(|name| self.find(name).ok_or_else(|| unknown(name)))
        .collect::<Result<Vec<_>, _>>()?,
      None => self.offered(),
    };
    let actor = match actor {
      Some(actor) => {
        let (model, level) = self.actor(actor).ok_or_else(|| unknown(actor))?;
        let named = model.at(level);
        models.insert(0, model);
        Some(named)
      }
      None => None,
    };
    let mut seen = HashSet::new();
    models.retain(|model| seen.insert(model.name.clone()));
    Ok((models, actor))
  }

  /// The provider of a roster of the catalog on a directory, with the default actor of the roster, whose models a
  /// function of the host answers in place of the models when it gives one, and whose model thinks at most the tokens
  /// in one turn that the environment gives, else the config of the project in the directory, else 32768.
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
    let directory = directory.into();
    let thinking = super::thinking(self.thinking.as_deref(), std::path::Path::new(&directory));
    Ok(Provider::new(directory, models).actor(actor).thinking(thinking))
  }
}

/// The model that a name names among these models: the one of that name, or the one model of that id.
fn named<'a>(models: &[&'a Model], name: &str) -> Option<&'a Model> {
  if let Some(model) = models.iter().find(|model| model.name == name) {
    return Some(model);
  }
  let mut found =
    models.iter().filter(|model| model.name.split_once(':').is_some_and(|(_, id)| id == name));
  match (found.next(), found.next()) {
    (Some(model), None) => Some(model),
    _ => None,
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
    if let Some(key) = &self.key {
      return Ok((key.clone(), false));
    }
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
  /// before the name itself; or the first name that the environment does not hold. Each place is filled once, after
  /// the value of the place before it, so a value that names a place stands as it is.
  fn filled(&self, api: &str, env: &impl Fn(&str) -> Option<String>) -> Result<String, String> {
    let (mut filled, mut from) = (api.to_owned(), 0);
    while let Some(at) = filled[from..].find("${").map(|at| from + at) {
      let Some(end) = filled[at..].find('}').map(|end| at + end) else { break };
      let name = filled[at + 2..end].to_owned();
      let value = self.aliases.get(&name).and_then(|alias| env(alias)).or_else(|| env(&name));
      let value =
        value.ok_or_else(|| format!("its address needs {name}: set it in the environment"))?;
      filled.replace_range(at..=end, &value);
      from = at + value.len();
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

/// A model of a provider of the network. A model of Anthropic, of Amazon or of pi-ai counts what it read of the cache
/// apart, and is told the most it writes.
fn model(
  provider: &str,
  id: &str,
  listing: &Listing,
  client: Client,
  reach: Result<Reach, String>,
) -> Model {
  let dialect = client.dialect(id);
  let mut model = reached(format!("{provider}:{id}"), listing.limit.context, client, reach);
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

/// A model under a name, with the window it reads, whose client is made when it is first asked, or why it cannot be.
pub(super) fn reached(
  name: String,
  window: u64,
  client: Client,
  reach: Result<Reach, String>,
) -> Model {
  let made: Arc<OnceLock<Result<Streams, String>>> = Arc::default();
  let search = reach.as_ref().ok().and_then(|one| search::native(client, one));
  let model = Model::lazy(name, window, move || {
    let got = made.get_or_init(|| reach.clone().and_then(|reach| client.streams(&reach)));
    got.clone().map_err(CompletionError::ProviderError)
  });
  match search {
    Some(search) => model.searches(search),
    None => model,
  }
}

/// The providers of the snapshot, with the models of each that the caches hold, read again by each catalog so that a
/// life which stands again sees an update. The cache of models.dev is refreshed in the background when it is a day
/// old.
fn listed(cached: Option<&PathBuf>) -> IndexMap<String, Listed> {
  if let Some(at) = cached {
    update::background(at);
  }
  let text = cached.and_then(|at| fs::read_to_string(at).ok());
  let mut listed = parsed(SNAPSHOT, text.as_deref());
  let pi = cached.and_then(|at| fs::read_to_string(at.with_file_name(update::PI)).ok());
  update::applied(&mut listed, pi.as_deref());
  listed
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

#[cfg(test)]
#[path = "catalog.test.rs"]
mod test;
