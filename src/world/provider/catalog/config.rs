//! The provider settings of the user and the project, applied to a catalog before a life stands on it.

use std::{collections::HashMap, env, fs};

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::{
  Catalog, Client, Cost, LOCAL, Limit, Listed, Listing, Modalities, cache, claude, listed,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderConfig {
  enabled: Option<bool>,
  protocol: Option<Client>,
  url: Option<String>,
  key_env: Option<String>,
  key: Option<String>,
  models: Option<IndexMap<String, ConfiguredModel>>,
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
}

impl Catalog {
  /// The catalog with the provider settings of the user and then the project. A false entry removes a provider.
  /// Invalid settings give an error. A running provider keeps its previous settings when a reload fails.
  pub fn configured(
    mut self,
    directory: &std::path::Path,
    config: Option<&std::path::Path>,
  ) -> Result<Catalog, String> {
    let places = crate::extension::Places::here();
    let files =
      [config.unwrap_or(&places.config).join("config.json"), directory.join(".furb/config.json")];
    let mut entries: IndexMap<String, Value> = IndexMap::new();
    for file in files {
      let text = match fs::read_to_string(&file) {
        Err(no) if no.kind() == std::io::ErrorKind::NotFound => continue,
        got => got.map_err(|no| format!("Could not read {}: {no}", file.display()))?,
      };
      let read: Value = serde_json::from_str(&text)
        .map_err(|no| format!("Invalid config at {}: {no}", file.display()))?;
      if !read.is_object() {
        return Err(format!("The config at {} must be an object", file.display()));
      }
      let Some(providers) = read.get("providers") else { continue };
      let providers = providers
        .as_object()
        .ok_or_else(|| format!("The providers at {} must be an object", file.display()))?;
      for (name, form) in providers {
        match (entries.get_mut(name), form.as_object()) {
          (Some(Value::Object(held)), Some(fields)) => held.extend(fields.clone()),
          (Some(Value::Object(held)), None) if form.is_boolean() => {
            held.insert("enabled".to_owned(), form.clone());
          }
          _ => {
            entries.insert(name.clone(), form.clone());
          }
        }
      }
    }
    let cached = cache().map(|at| at.join("models.json"));
    let defaults = listed(cached.as_ref());
    for (name, form) in entries {
      if name.is_empty() || name.contains([':', '/']) || name.chars().any(char::is_whitespace) {
        return Err(
          "A provider name must be nonempty and contain no space, colon or slash".to_owned(),
        );
      }
      let enabled =
        form.as_bool().or_else(|| form.get("enabled").and_then(Value::as_bool)).unwrap_or(true);
      if !enabled {
        self
          .models
          .retain(|(model, _)| model.name.split_once(':').is_none_or(|(id, _)| id != name));
        if let Some(held) = self.providers.iter_mut().find(|(id, _, _)| id == &name) {
          held.2 = false;
        } else {
          self.providers.push((name.clone(), Vec::new(), false));
        }
        if name == LOCAL {
          self.local = None;
        }
        continue;
      }
      if name == super::super::codex::NAME {
        self.models.retain(|(model, _)| !model.name.starts_with("codex:"));
        self.models.extend(
          super::super::codex::offered(config, &form)?.into_iter().map(|model| (model, true)),
        );
        self.providers.retain(|(id, _, _)| id != &name);
        self.providers.push((name, Vec::new(), true));
        continue;
      }
      if form == Value::Bool(true) {
        if defaults.contains_key(&name) || name == LOCAL || name == claude::CLAUDE {
          continue;
        }
        return Err(format!("No built-in provider is {name}"));
      }
      if name == claude::CLAUDE {
        return Err("The claude-cli provider accepts true or false".to_owned());
      }
      if name == LOCAL {
        self.local = None;
      }
      let settings: ProviderConfig =
        serde_json::from_value(form).map_err(|no| format!("Invalid provider {name}: {no}"))?;
      let _ = settings.enabled;
      let original = defaults.get(&name);
      let mut listed = original.cloned().unwrap_or(Listed {
        rig: Client::Chat,
        env: Vec::new(),
        bearer: Vec::new(),
        api: None,
        aliases: HashMap::new(),
        models: IndexMap::new(),
        key: Some(String::new()),
      });
      if let Some(protocol) = settings.protocol {
        listed.rig = protocol;
        for model in listed.models.values_mut() {
          model.provider = None;
        }
      }
      if let Some(url) = settings.url {
        let parsed = url::Url::parse(&url).map_err(|_| format!("The URL of {name} is invalid"))?;
        if !matches!(parsed.scheme(), "http" | "https")
          || !parsed.username().is_empty()
          || parsed.password().is_some()
        {
          return Err(format!("The URL of {name} must use HTTP or HTTPS with no credential"));
        }
        listed.api = Some(url);
        for model in listed.models.values_mut() {
          if let Some(own) = &mut model.provider {
            own.api = None;
          }
        }
      }
      if let Some(key) = settings.key {
        listed.key = Some(key);
      }
      if let Some(key_env) = settings.key_env {
        listed.env = vec![key_env];
        listed.key = None;
      }
      if let Some(models) = settings.models {
        listed.models = models
          .into_iter()
          .map(|(id, one)| {
            if one.cost.is_some_and(|cost| {
              [cost.input, cost.output, cost.cache_read, cost.cache_write]
                .iter()
                .any(|price| *price < 0.0)
            }) {
              return Err(format!("The prices of each model of {name} must be nonnegative"));
            }
            if id.is_empty() || one.window == 0 || one.output == Some(0) {
              return Err(format!(
                "Each model of {name} needs an id, a positive window and a positive output limit"
              ));
            }
            Ok((
              id,
              Listing {
                reasoning: one.reasoning,
                cost: one.cost,
                limit: Limit {
                  context: one.window,
                  output: one.output.unwrap_or(one.window / 4).max(1),
                },
                modalities: Modalities {
                  input: if one.images {
                    vec!["text".to_owned(), "image".to_owned()]
                  } else {
                    vec!["text".to_owned()]
                  },
                  output: vec!["text".to_owned()],
                },
                ..Listing::default()
              },
            ))
          })
          .collect::<Result<_, _>>()?;
      }
      if listed.models.is_empty() || (original.is_none() && listed.api.is_none()) {
        return Err(format!("A custom provider {name} needs a URL and at least one model"));
      }
      let at = self
        .models
        .iter()
        .position(|(model, _)| model.name.starts_with(&format!("{name}:")))
        .unwrap_or(self.models.len());
      let replacement = Catalog::of(&IndexMap::from([(name.clone(), listed)]), None, None, |key| {
        env::var(key).ok()
      });
      let added = replacement
        .models
        .into_iter()
        .filter(|(model, _)| model.name.starts_with(&format!("{name}:")));
      self.models.retain(|(model, _)| !model.name.starts_with(&format!("{name}:")));
      self.models.splice(at..at, added);
      self.providers.retain(|(id, _, _)| id != &name);
      self.providers.extend(replacement.providers);
    }
    Ok(self)
  }
}
