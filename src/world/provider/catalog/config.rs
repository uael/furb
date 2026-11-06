//! The provider settings of the user and the project, applied to a catalog before a life stands on it.

use std::{
  collections::HashMap,
  env, fs, io,
  path::{Path, PathBuf},
};

use indexmap::IndexMap;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use super::{
  super::codex, Catalog, Client, Cost, Entry, Error, LOCAL, Limit, Listed, Listing, Modalities,
  Model, Offer, cache, claude, listed, switched, update::MODELS,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderConfig {
  #[serde(rename = "enabled")]
  _enabled: Option<bool>,
  protocol: Option<Client>,
  url: Option<String>,
  key_env: Option<String>,
  key: Option<String>,
  models: Option<IndexMap<String, ConfiguredModel>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaudeConfig {
  #[serde(rename = "enabled")]
  _enabled: Option<bool>,
  command: Option<CommandPrefix>,
  accounts: Option<Vec<AccountConfig>>,
  switch: Option<claude::Switch>,
}

/// An account of the claude command line, which names exactly one place for its credential.
#[derive(Deserialize)]
#[serde(
  untagged,
  deny_unknown_fields,
  expecting = "an account with a name and exactly one of token_file, token_env and config_dir"
)]
enum AccountConfig {
  File { name: String, token_file: String },
  Env { name: String, token_env: String },
  Dir { name: String, config_dir: String },
}

impl AccountConfig {
  fn name(&self) -> &str {
    match self {
      Self::File { name, .. } | Self::Env { name, .. } | Self::Dir { name, .. } => name,
    }
  }

  /// The account, with the rules of its name and of its credential checked.
  /// A path is absolute or starts with `~`, since a project file has no folder to count from.
  fn into_account(self) -> Result<claude::Account, Error> {
    let name = self.name();
    if name.is_empty() || name.chars().any(|one| one.is_whitespace() || one.is_control()) {
      return Err(Error::AccountName(name.to_owned()));
    }
    let (name, source) = match self {
      Self::File { name, token_file } => {
        let path = absolute(&name, &token_file)?;
        (name, claude::Source::TokenFile(path))
      }
      Self::Env { name, token_env } => {
        if token_env.is_empty() || token_env.contains(['=', '\0']) {
          return Err(Error::AccountVariable(name));
        }
        (name, claude::Source::TokenEnv(token_env))
      }
      Self::Dir { name, config_dir } => {
        let path = absolute(&name, &config_dir)?;
        (name, claude::Source::ConfigDir(path))
      }
    };
    Ok(claude::Account { name, source })
  }
}

/// The accounts of the claude command line and the policy that moves among them.
fn account_list(
  accounts: Option<Vec<AccountConfig>>,
  switch: Option<claude::Switch>,
) -> Result<(Vec<claude::Account>, claude::Switch), Error> {
  let Some(list) = accounts else {
    return match switch {
      Some(_) => Err(Error::SwitchWithoutAccounts),
      None => Ok((Vec::new(), claude::Switch::default())),
    };
  };
  if list.is_empty() {
    return Err(Error::NoAccounts);
  }
  let mut made: Vec<claude::Account> = Vec::new();
  for one in list {
    let account = one.into_account()?;
    if made.iter().any(|held| held.name == account.name) {
      return Err(Error::AccountTwice(account.name));
    }
    made.push(account);
  }
  Ok((made, switch.unwrap_or_default()))
}

/// A path of an account, with a leading `~` taken for the home of the user.
fn absolute(name: &str, path: &str) -> Result<PathBuf, Error> {
  let home = || env::home_dir().ok_or_else(|| Error::NoHome(name.to_owned()));
  let full = match path.strip_prefix("~/") {
    Some(rest) => home()?.join(rest),
    None if path == "~" => home()?,
    None => path.into(),
  };
  if full.is_absolute() && !path.contains('\0') {
    Ok(full)
  } else {
    Err(Error::Path { name: name.to_owned(), path: path.to_owned() })
  }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CommandPrefix {
  Executable(String),
  Words(Vec<String>),
}

impl CommandPrefix {
  fn parts(self) -> Result<(PathBuf, Vec<String>), Error> {
    let mut words = match self {
      CommandPrefix::Executable(bin) => vec![bin],
      CommandPrefix::Words(words) => words,
    };
    if words.first().is_none_or(|bin| bin.trim().is_empty()) {
      return Err(Error::CommandEmpty);
    }
    if words.iter().any(|word| word.contains('\0')) {
      return Err(Error::CommandNull);
    }
    let bin = words.remove(0);
    Ok((bin.into(), words))
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredModel {
  window: u64,
  output: Option<u64>,
  cost: Option<Cost>,
  #[serde(default)]
  reasoning: bool,
  #[serde(default)]
  images: bool,
  /// The levels of effort that a model of the protocol `chat` takes, as a model of a local server takes them.
  efforts: Option<Vec<String>>,
}

/// The levels of effort that a model of the protocol `chat` can take in the settings.
const CHAT_LEVELS: [&str; 4] = ["off", "low", "medium", "high"];

impl ConfiguredModel {
  /// The listing of the model `id` of the provider `provider`, whose protocol is chat or not.
  fn listing(self, provider: &str, id: &str, chat: bool) -> Result<Listing, Error> {
    let prices = |cost: Cost| [cost.input, cost.output, cost.cache_read, cost.cache_write];
    if self.cost.is_some_and(|cost| prices(cost).iter().any(|price| *price < 0.0)) {
      return Err(Error::Prices(provider.to_owned()));
    }
    if let Some(levels) = &self.efforts
      && (!chat
        || levels.is_empty()
        || levels.iter().any(|level| !CHAT_LEVELS.contains(&level.as_str())))
    {
      return Err(Error::Efforts(provider.to_owned()));
    }
    if id.is_empty() || self.window == 0 || self.output == Some(0) {
      return Err(Error::Model(provider.to_owned()));
    }
    let mut input = vec!["text".to_owned()];
    if self.images {
      input.push("image".to_owned());
    }
    Ok(Listing {
      reasoning: self.reasoning,
      cost: self.cost,
      limit: Limit { context: self.window, output: self.output.unwrap_or(self.window / 4).max(1) },
      modalities: Modalities { input, output: vec!["text".to_owned()] },
      levels: self.efforts,
      ..Listing::default()
    })
  }
}

impl ProviderConfig {
  /// The provider that these settings make of the one that the catalog lists, or of a blank one.
  fn into_listed(self, name: &str, original: Option<&Listed>) -> Result<Listed, Error> {
    let mut listed = original.cloned().unwrap_or_else(blank);
    if let Some(protocol) = self.protocol {
      listed.rig = protocol;
      for model in listed.models.values_mut() {
        model.provider = None;
      }
    }
    if let Some(url) = self.url {
      checked_url(name, &url)?;
      listed.api = Some(url);
      for own in listed.models.values_mut().filter_map(|model| model.provider.as_mut()) {
        own.api = None;
      }
    }
    if let Some(key) = self.key {
      listed.key = Some(key);
    }
    if let Some(key_env) = self.key_env {
      listed.env = vec![key_env];
      listed.key = None;
    }
    if let Some(models) = self.models {
      let chat = matches!(listed.rig, Client::Chat);
      let one = |(id, one): (String, ConfiguredModel)| {
        one.listing(name, &id, chat).map(|listing| (id, listing))
      };
      listed.models = models.into_iter().map(one).collect::<Result<_, _>>()?;
    }
    if listed.models.is_empty() || (original.is_none() && listed.api.is_none()) {
      return Err(Error::Custom(name.to_owned()));
    }
    Ok(listed)
  }
}

/// A provider that the catalog does not list, which the settings fill in.
fn blank() -> Listed {
  Listed {
    rig: Client::Chat,
    env: Vec::new(),
    bearer: Vec::new(),
    api: None,
    aliases: HashMap::new(),
    models: IndexMap::new(),
    key: Some(String::new()),
  }
}

fn checked_url(name: &str, url: &str) -> Result<(), Error> {
  let parsed =
    url::Url::parse(url).map_err(|source| Error::Url { name: name.to_owned(), source })?;
  if !matches!(parsed.scheme(), "http" | "https")
    || !parsed.username().is_empty()
    || parsed.password().is_some()
  {
    return Err(Error::UrlScheme(name.to_owned()));
  }
  Ok(())
}

fn parse<T: DeserializeOwned>(name: &str, form: Value) -> Result<T, Error> {
  serde_json::from_value(form).map_err(|source| Error::Invalid { name: name.to_owned(), source })
}

/// The `providers` of one config file, or none when the file is absent or has none.
fn providers_of(file: &Path) -> Result<Option<Map<String, Value>>, Error> {
  let text = match fs::read_to_string(file) {
    Err(no) if no.kind() == io::ErrorKind::NotFound => return Ok(None),
    got => got.map_err(|source| Error::Read { path: file.to_owned(), source })?,
  };
  let read =
    serde_json::from_str(&text).map_err(|source| Error::Parse { path: file.to_owned(), source })?;
  let Value::Object(mut read) = read else {
    return Err(Error::NotObject { path: file.to_owned() });
  };
  match read.remove("providers") {
    None => Ok(None),
    Some(Value::Object(providers)) => Ok(Some(providers)),
    Some(_) => Err(Error::ProvidersNotObject { path: file.to_owned() }),
  }
}

/// The providers of the config files in order.
/// A later field wins over an earlier one for each provider.
fn merged(files: impl IntoIterator<Item = PathBuf>) -> Result<IndexMap<String, Value>, Error> {
  let mut entries: IndexMap<String, Value> = IndexMap::new();
  for file in files {
    let Some(providers) = providers_of(&file)? else { continue };
    for (name, form) in providers {
      match (entries.get_mut(&name), form) {
        (Some(Value::Object(held)), Value::Object(fields)) => held.extend(fields),
        (Some(Value::Object(held)), Value::Bool(on)) => {
          switched(held, on);
        }
        (_, form) => {
          entries.insert(name, form);
        }
      }
    }
  }
  Ok(entries)
}

fn valid_name(name: &str) -> Result<(), Error> {
  if name.is_empty() || name.contains([':', '/']) || name.chars().any(char::is_whitespace) {
    return Err(Error::ProviderName);
  }
  Ok(())
}

fn is_of(model: &Model, provider: &str) -> bool {
  model.name.split_once(':').is_some_and(|(id, _)| id == provider)
}

impl Catalog {
  /// The catalog with the provider settings of the configs, in the order of [`crate::extension::configs`]: the user,
  /// the project, then the local config of the project. A false entry removes a provider.
  /// Invalid settings give an error. A running provider keeps its previous settings when a reload fails.
  pub fn configured(self, directory: &Path, config: Option<&Path>) -> Result<Catalog, Error> {
    super::blocking(self.configure(directory, config))
  }

  /// The catalog with the provider settings of the configs, as [`Catalog::configured`] gives it,
  /// for a caller that awaits.
  pub async fn configure(
    mut self,
    directory: &Path,
    config: Option<&Path>,
  ) -> Result<Catalog, Error> {
    let places = crate::extension::Places::here();
    let files = crate::extension::configs(config.unwrap_or(&places.config), directory);
    let entries = merged(files)?;
    let cached = cache().map(|at| at.join(MODELS));
    let defaults = listed(cached.as_ref());
    for (name, form) in entries {
      valid_name(&name)?;
      self.apply(name, form, config, &defaults).await?;
    }
    Ok(self)
  }

  async fn apply(
    &mut self,
    name: String,
    form: Value,
    config: Option<&Path>,
    defaults: &IndexMap<String, Listed>,
  ) -> Result<(), Error> {
    let enabled =
      form.as_bool().or_else(|| form.get("enabled").and_then(Value::as_bool)).unwrap_or(true);
    if !enabled {
      self.disable(&name);
      return Ok(());
    }
    if name == codex::NAME {
      return self.set_codex(name, config, &form);
    }
    if form == Value::Bool(true) {
      let built_in = defaults.contains_key(&name) || name == LOCAL || name == claude::CLAUDE;
      return if built_in { Ok(()) } else { Err(Error::Builtin(name)) };
    }
    if name == claude::CLAUDE {
      return self.set_claude(form).await;
    }
    self.set_custom(&name, form, defaults)
  }

  /// The place of the first model of a provider.
  fn first_of(&self, provider: &str) -> Option<usize> {
    self.models.iter().position(|offer| is_of(&offer.model, provider))
  }

  fn drop_models(&mut self, provider: &str) {
    self.models.retain(|offer| !is_of(&offer.model, provider));
  }

  fn disable(&mut self, name: &str) {
    self.drop_models(name);
    match self.providers.iter_mut().find(|held| held.name == name) {
      Some(held) => held.enabled = false,
      None => {
        self.providers.push(Entry { name: name.to_owned(), env: Vec::new(), enabled: false });
      }
    }
    if name == LOCAL {
      self.local = None;
    }
  }

  fn set_codex(&mut self, name: String, config: Option<&Path>, form: &Value) -> Result<(), Error> {
    let models = codex::offered(config, form).map_err(Error::Codex)?;
    self.drop_models(&name);
    self.models.extend(models.into_iter().map(|model| Offer::new(model, true)));
    self.providers.retain(|held| held.name != name);
    self.providers.push(Entry { name, env: Vec::new(), enabled: true });
    Ok(())
  }

  async fn set_claude(&mut self, form: Value) -> Result<(), Error> {
    let settings: ClaudeConfig = parse(claude::CLAUDE, form)?;
    if settings.command.is_none() && settings.accounts.is_none() && settings.switch.is_none() {
      return Ok(());
    }
    let (bin, args) = match settings.command {
      Some(command) => {
        let (bin, args) = command.parts()?;
        (Some(bin), args)
      }
      None => (None, Vec::new()),
    };
    let (accounts, switch) = account_list(settings.accounts, settings.switch)?;
    let command = claude::Claude::with_accounts(bin, args, accounts, switch, None);
    let at = self.first_of(claude::CLAUDE).unwrap_or(0);
    self.drop_models(claude::CLAUDE);
    self
      .models
      .splice(at..at, command.models().await.into_iter().map(|model| Offer::new(model, true)));
    self.providers.retain(|held| held.name != claude::CLAUDE);
    Ok(())
  }

  fn set_custom(
    &mut self,
    name: &str,
    form: Value,
    defaults: &IndexMap<String, Listed>,
  ) -> Result<(), Error> {
    if name == LOCAL {
      self.local = None;
    }
    let settings: ProviderConfig = parse(name, form)?;
    let listed = settings.into_listed(name, defaults.get(name))?;
    let at = self.first_of(name).unwrap_or(self.models.len());
    let replacement =
      Catalog::of(&IndexMap::from([(name.to_owned(), listed)]), None, None, |key| {
        env::var(key).ok()
      });
    let added: Vec<Offer> =
      replacement.models.into_iter().filter(|offer| is_of(&offer.model, name)).collect();
    self.drop_models(name);
    self.models.splice(at..at, added);
    self.providers.retain(|held| held.name != name);
    self.providers.extend(replacement.providers);
    Ok(())
  }
}
