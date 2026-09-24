//! The extensions, for a host in TypeScript: what the crate reads for every host, the config, the cache, the
//! manifests, the order and the words, given as plain objects. The host keeps what is its own: the parts of an
//! extension for a World and for a TUI, and the import of their code.

use std::{
  ffi::OsString,
  path::{Path, PathBuf},
};

use napi::{
  Env,
  bindgen_prelude::{JsObjectValue, Object as JsObject},
};
use napi_derive::napi;

use crate::{Fault, extension};

/// The files of the parts of an extension for a World, one for each language a host writes its World in.
#[napi(object)]
pub struct ExtensionWorld {
  pub ts: Option<String>,
  pub py: Option<String>,
}

/// One extension as a host plays it: its name, whether the crate carries it, the directory it stands in, the word
/// of its python part and its life word, the names it requires, and the files of its parts for a World and for a
/// TUI.
#[napi(object, js_name = "Extension")]
pub struct JsExtension {
  pub name: String,
  pub builtin: bool,
  pub root: Option<String>,
  pub word: Option<String>,
  pub life: Option<String>,
  pub requires: Vec<String>,
  pub world: ExtensionWorld,
  pub tui: Option<String>,
}

/// What a resolution of the extensions of a project does beside reading: fetch each one again, and install the
/// dependencies of an extension whose part for a World is TypeScript, which it does unless `install` is false.
#[napi(object)]
#[derive(Default)]
pub struct ResolveOptions {
  pub refresh: Option<bool>,
  pub install: Option<bool>,
}

fn shown(path: Option<PathBuf>) -> Option<String> {
  path.map(|one| one.display().to_string())
}

impl From<extension::Extension> for JsExtension {
  fn from(one: extension::Extension) -> Self {
    JsExtension {
      name: one.name,
      builtin: one.root.is_none(),
      root: shown(one.root),
      word: one.word,
      life: one.life,
      requires: one.requires,
      world: ExtensionWorld { ts: shown(one.world.ts), py: shown(one.world.py) },
      tui: shown(one.tui),
    }
  }
}

fn refused(error: extension::Error) -> napi::Error {
  napi::Error::from_reason(Fault::from(error).message())
}

/// The places of this process, from its environment as JavaScript holds it, `process.env`, which a runtime may keep
/// apart from the environment of the system, and from its home.
fn places(env: &Env) -> napi::Result<extension::Places> {
  let variables: JsObject =
    env.get_global()?.get_named_property::<JsObject>("process")?.get_named_property("env")?;
  let mut held = std::collections::HashMap::new();
  for key in [
    "FURB_CONFIG_DIR",
    "FURB_CACHE_DIR",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "APPDATA",
    "LOCALAPPDATA",
  ] {
    if let Some(value) = variables.get::<String>(key)? {
      held.insert(key, OsString::from(value));
    }
  }
  Ok(extension::Places::of(|key| held.get(key).cloned(), std::env::home_dir(), cfg!(windows)))
}

/// The config directory of the user, as the environment of JavaScript says it.
#[napi]
pub fn config_directory(env: Env) -> napi::Result<String> {
  Ok(places(&env)?.config.display().to_string())
}

/// The cache directory of the user, as the environment of JavaScript says it.
#[napi]
pub fn cache_directory(env: Env) -> napi::Result<String> {
  Ok(places(&env)?.cache.display().to_string())
}

/// The builtin extensions, files, bash and grant, in the order a host takes them.
#[napi]
pub fn builtin_extensions() -> Vec<JsExtension> {
  extension::builtins().into_iter().map(JsExtension::from).collect()
}

/// The extensions a host plays for a project: the builtins and what the config of the user and the config of the
/// project name, fetched into the cache once, and again on a refresh, loaded, and ordered by what each requires. It
/// throws with what failed: a config, a fetch, a manifest, a requirement or a word.
#[napi]
pub fn resolve_extensions(
  env: Env,
  project: String,
  options: Option<ResolveOptions>,
) -> napi::Result<Vec<JsExtension>> {
  let options = options.unwrap_or_default();
  extension::extensions(
    &places(&env)?,
    Path::new(&project),
    options.refresh.unwrap_or(false),
    options.install.unwrap_or(true),
  )
  .map(|got| got.into_iter().map(JsExtension::from).collect())
  .map_err(refused)
}

/// The word of the python part of an extension, which the module of the engine runs after the engine: the file with
/// its line ends made LF and less every top-level import from `furb`, which leaves no line of its own. It throws for
/// a file python cannot parse.
#[napi]
pub fn word_of(source: String) -> napi::Result<String> {
  extension::word(&source).map_err(refused)
}
