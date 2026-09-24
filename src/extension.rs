//! The extensions: what a host plays on a chain, and where it finds it.
//!
//! The python part of an extension is a file that a host plays as a rung, the World on every chain without a
//! source. The file is a word as it is, or a python module that imports what it uses from the engine and from the
//! extensions it requires, so that an editor, ruff and ty read it. The module of a chain binds every name of the
//! engine and of each word played before it already, so the host makes the word of a module by blanking those
//! imports, and the word reads every name through the globals of the chain, where a later rung may rebind it.
//!
//! Every host shares this module: the host in TypeScript, the host in python and a host in rust read the same
//! config, fetch into the same cache, read the same manifests, play the extensions in the same order, make the same
//! word of a module, and play a word on a chain by the same rule. What stays in each host is its own: the parts of
//! an extension for a World and for a TUI, and how it loads their code.
//!
//! The config is a json file, `{"extensions": {name: form}}`, in the config directory of the user and in the
//! directory `.furb` of the project, which names an extension by its key: `false` turns it off, `true` turns it on,
//! and a path, a git remote or an npm package says where it stands. The builtins, files, bash and grant, are on
//! unless a config turns them off. An extension stands in a directory whose `package.json` holds a field `furb`,
//! its manifest: its name, which is its key, the file of its python part, the word it plays in every life, the
//! files of its parts for a World and for a TUI, and the names it requires.

use std::{
  collections::HashSet,
  ffi::OsString,
  fmt, fs,
  path::{Component, Path, PathBuf},
  process::Command,
};

use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::value::Fault;

/// The package the engine and the extensions are imported from, whose imports a word leaves out.
const PACKAGE: &str = "furb";

/// The name of the builtins, in the order a host plays them.
const BUILTINS: [&str; 3] = ["files", "bash", "grant"];

/// What went wrong with an extension, which a host says once when it loads the extensions and plays nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
  /// A config that is no json, or whose extensions are no object.
  Config { file: PathBuf, why: String },
  /// An entry of a config of no known form.
  Entry { name: String, file: PathBuf, why: String },
  /// A fetch that failed, with what git, npm or the disk said.
  Fetch { name: String, why: String },
  /// A manifest that is missing, of no known form, or that names a file out of its directory.
  Manifest { root: PathBuf, why: String },
  /// An extension whose requirement is off, or that no config names.
  Requires { name: String, needs: String, off: bool },
  /// Extensions that require each other.
  Cycle { names: Vec<String> },
  /// A python part that python cannot parse, by the name of its extension, and the line of the fault.
  Word { name: String, line: usize, why: String },
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::Config { file, why } => write!(f, "the config {}: {why}", file.display()),
      Error::Entry { name, file, why } => {
        write!(f, "the extension {name} in the config {}: {why}", file.display())
      }
      Error::Fetch { name, why } => write!(f, "the extension {name} could not be fetched: {why}"),
      Error::Manifest { root, why } => write!(f, "the manifest in {}: {why}", root.display()),
      Error::Requires { name, needs, off: true } => write!(
        f,
        "the extension {name} requires {needs}, which is off: turn {name} off too, or turn {needs} on"
      ),
      Error::Requires { name, needs, off: false } => {
        write!(f, "the extension {name} requires {needs}, which no config names")
      }
      Error::Cycle { names } => write!(f, "the extensions {} require each other", names.join(", ")),
      Error::Word { name, line, why } if name.is_empty() => write!(f, "line {line}: {why}"),
      Error::Word { name, line, why } => write!(f, "the extension {name}: line {line}: {why}"),
    }
  }
}

impl std::error::Error for Error {}

impl From<Error> for Fault {
  fn from(error: Error) -> Self {
    Fault::refused(error.to_string())
  }
}

/// Where the extensions of a user stand: the config directory, the cache directory, and the home that a `~` of a
/// config expands to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
  pub config: PathBuf,
  pub cache: PathBuf,
  pub home: Option<PathBuf>,
}

impl Places {
  /// The places of this process, from its environment and its home.
  pub fn here() -> Places {
    #[allow(deprecated)]
    let home = std::env::home_dir();
    Places::of(|key| std::env::var_os(key), home, cfg!(windows))
  }

  /// The places for an environment, a home and a system: the config directory is `FURB_CONFIG_DIR`, or `furb`
  /// under `XDG_CONFIG_HOME`, under `APPDATA` on windows, or under `.config` of the home; the cache directory is
  /// `FURB_CACHE_DIR`, or `furb` under `XDG_CACHE_HOME`, under `LOCALAPPDATA` on windows, or under `.cache` of the
  /// home. A variable that is empty is unset.
  pub fn of(
    var: impl Fn(&str) -> Option<OsString>,
    home: Option<PathBuf>,
    windows: bool,
  ) -> Places {
    let set = |key: &str| var(key).filter(|one| !one.is_empty()).map(PathBuf::from);
    let under = |own: &str, xdg: &str, system: &str, dot: &str| {
      set(own).unwrap_or_else(|| {
        set(xdg)
          .or_else(|| windows.then(|| set(system)).flatten())
          .or_else(|| home.as_ref().map(|one| one.join(dot)))
          .unwrap_or_default()
          .join("furb")
      })
    };
    Places {
      config: under("FURB_CONFIG_DIR", "XDG_CONFIG_HOME", "APPDATA", ".config"),
      cache: under("FURB_CACHE_DIR", "XDG_CACHE_HOME", "LOCALAPPDATA", ".cache"),
      home: home.clone(),
    }
  }
}

/// Where an extension stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
  /// A builtin, which the crate carries.
  Builtin,
  /// A directory on the disk.
  Path(PathBuf),
  /// A git remote, at a branch or a tag, and a directory in it.
  Git { url: String, reference: Option<String>, path: Option<PathBuf> },
  /// An npm package, at a version.
  Npm { package: String, version: Option<String> },
}

/// What one config says of one name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Setting {
  Off,
  On,
  From(Source),
}

/// One extension the configs name, as they merge: its name, whether it is on, where it stands, and the config that
/// said so last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
  pub name: String,
  pub on: bool,
  pub source: Option<Source>,
  pub file: Option<PathBuf>,
}

/// The settings of one config, read from its text: each name of its extensions with what it says, in the order of
/// the file. A path, and a git remote or an npm package that starts with `./`, `../` or `~`, resolves against the
/// directory of the file, and a `~` expands to the home.
/// A path with each `.` dropped and each `..` taking the name before it away, as the path reads and before the disk
/// is asked, so the path a config names reads as the directory it is. A `..` at the root stays at the root.
pub fn tidy(path: &Path) -> PathBuf {
  use std::path::Component::{CurDir, Normal, ParentDir, Prefix, RootDir};
  let mut out = PathBuf::new();
  for part in path.components() {
    match (part, out.components().next_back()) {
      (CurDir, _) | (ParentDir, Some(RootDir | Prefix(_))) => {}
      (ParentDir, Some(Normal(_))) => {
        out.pop();
      }
      (other, _) => out.push(other),
    }
  }
  out
}

pub fn settings(
  text: &str,
  file: &Path,
  home: Option<&Path>,
) -> Result<Vec<(String, Setting)>, Error> {
  let refused = |why: String| Error::Config { file: file.to_owned(), why };
  let read: Value = serde_json::from_str(text).map_err(|why| refused(why.to_string()))?;
  let Some(extensions) = read.get("extensions") else {
    return Ok(Vec::new());
  };
  let Value::Object(extensions) = extensions else {
    return Err(refused("extensions is no object".to_owned()));
  };
  let at = file.parent().unwrap_or(Path::new(""));
  let local = |one: &str| {
    if one == "~" || one.starts_with("~/") || one.starts_with("~\\") {
      home.map_or_else(
        || PathBuf::from(one),
        |home| home.join(one[1..].trim_start_matches(['/', '\\'])),
      )
    } else {
      tidy(&at.join(one))
    }
  };
  let nearby = |one: &str| {
    if one.starts_with("./") || one.starts_with("../") || one.starts_with('~') {
      local(one).display().to_string()
    } else {
      one.to_owned()
    }
  };
  let mut out = Vec::new();
  for (name, form) in extensions {
    let wrong =
      |why: &str| Error::Entry { name: name.clone(), file: file.to_owned(), why: why.to_owned() };
    let text = |fields: &Map<String, Value>, key: &str| match fields.get(key) {
      None => Ok(None),
      Some(Value::String(one)) => Ok(Some(one.clone())),
      Some(_) => Err(wrong(&format!("{key} is no string"))),
    };
    let setting = match form {
      Value::Bool(false) => Setting::Off,
      Value::Bool(true) => Setting::On,
      Value::String(path) => Setting::From(Source::Path(local(path))),
      Value::Object(fields) if fields.contains_key("path") && fields.len() == 1 => {
        Setting::From(Source::Path(local(&text(fields, "path")?.unwrap_or_default())))
      }
      Value::Object(fields) if fields.contains_key("git") => {
        let Some(url) = text(fields, "git")? else { unreachable!() };
        Setting::From(Source::Git {
          url: nearby(&url),
          reference: text(fields, "ref")?,
          path: text(fields, "path")?.map(PathBuf::from),
        })
      }
      Value::Object(fields) if fields.contains_key("npm") => {
        let Some(package) = text(fields, "npm")? else { unreachable!() };
        Setting::From(Source::Npm { package: nearby(&package), version: text(fields, "version")? })
      }
      _ => {
        return Err(wrong("no form of an extension: false, true, a path, {path}, {git} or {npm}"));
      }
    };
    out.push((name.clone(), setting));
  }
  Ok(out)
}

/// The settings of one config file, and none when there is no such file.
pub fn read_settings(file: &Path, home: Option<&Path>) -> Result<Vec<(String, Setting)>, Error> {
  match fs::read_to_string(file) {
    Ok(text) => settings(&text, file, home),
    Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
    Err(why) => Err(Error::Config { file: file.to_owned(), why: why.to_string() }),
  }
}

/// The entries of the configs, merged by name: the builtins first, each on, then each name of each config in the
/// order of its file, the later config over the earlier. A name keeps its place; `false` turns it off and keeps its
/// source, `true` turns it on and keeps the source an earlier config gave, and a source replaces the source and
/// turns it on. `true` for a name that no config sources and no builtin is is refused.
pub fn merged(layers: &[(PathBuf, Vec<(String, Setting)>)]) -> Result<Vec<Entry>, Error> {
  let mut out: Vec<Entry> = BUILTINS
    .iter()
    .map(|&name| Entry {
      name: name.to_owned(),
      on: true,
      source: Some(Source::Builtin),
      file: None,
    })
    .collect();
  for (file, settings) in layers {
    for (name, setting) in settings {
      let at = match out.iter().position(|one| &one.name == name) {
        Some(at) => at,
        None => {
          out.push(Entry { name: name.clone(), on: false, source: None, file: None });
          out.len() - 1
        }
      };
      let entry = &mut out[at];
      entry.file = Some(file.clone());
      match setting {
        Setting::Off => entry.on = false,
        Setting::On if entry.source.is_none() => {
          return Err(Error::Entry {
            name: name.clone(),
            file: file.clone(),
            why: "true, and no config says where it stands".to_owned(),
          });
        }
        Setting::On => entry.on = true,
        Setting::From(source) => {
          entry.on = true;
          entry.source = Some(source.clone());
        }
      }
    }
  }
  Ok(out)
}

/// The entries of the config of the user and of the config of the project, merged.
pub fn config(places: &Places, project: &Path) -> Result<Vec<Entry>, Error> {
  let home = places.home.as_deref();
  let layers = [places.config.join("config.json"), project.join(".furb").join("config.json")]
    .into_iter()
    .map(|file| read_settings(&file, home).map(|read| (file, read)))
    .collect::<Result<Vec<_>, _>>()?;
  merged(&layers)
}

/// The hash of a text, as hex, which names a fetch in the cache.
fn hashed(text: &str) -> String {
  Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A program run to its end, and what it wrote on its stdout, or a refusal with what it wrote on its stderr.
fn ran(name: &str, program: &str, args: &[&str], at: Option<&Path>) -> Result<String, Error> {
  let mut command = Command::new(program);
  command.args(args).env("GIT_TERMINAL_PROMPT", "0");
  if let Some(at) = at {
    command.current_dir(at);
  }
  let refused = |why: String| Error::Fetch { name: name.to_owned(), why };
  let done = command.output().map_err(|why| refused(format!("{program}: {why}")))?;
  if done.status.success() {
    Ok(String::from_utf8_lossy(&done.stdout).into_owned())
  } else {
    Err(refused(format!(
      "{program} {}: {}",
      args.join(" "),
      String::from_utf8_lossy(&done.stderr).trim()
    )))
  }
}

/// The program of npm on this system.
fn npm() -> &'static str {
  if cfg!(windows) { "npm.cmd" } else { "npm" }
}

/// Whether a package is a name of the npm registry, which the cache keeps under its name and its version.
fn registered(package: &str) -> bool {
  let part = |one: &str| {
    !one.is_empty()
      && one
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '~')
      && one.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-._~".contains(c))
  };
  match package.strip_prefix('@') {
    Some(scoped) => scoped.split_once('/').is_some_and(|(scope, name)| part(scope) && part(name)),
    None => part(package),
  }
}

/// The directory of the cache an npm package is kept in.
pub fn npm_dir(places: &Places, package: &str, version: Option<&str>) -> PathBuf {
  let npm = places.cache.join("extensions").join("npm");
  if registered(package) {
    npm.join(format!("{package}@{}", version.unwrap_or("latest")))
  } else {
    npm.join(hashed(&format!("{package}@{}", version.unwrap_or(""))))
  }
}

/// A directory made in its place, from what `make` puts in a directory of its own: the fetch writes nowhere a
/// reader looks until it is whole, and a fetch another process made first stands.
fn placed(
  name: &str,
  dir: &Path,
  places: &Places,
  refresh: bool,
  make: impl FnOnce(&Path) -> Result<PathBuf, Error>,
) -> Result<(), Error> {
  if dir.is_dir() && !refresh {
    return Ok(());
  }
  let refused = |why: std::io::Error| Error::Fetch { name: name.to_owned(), why: why.to_string() };
  let spare = places.cache.join("extensions").join(".tmp");
  fs::create_dir_all(&spare).map_err(refused)?;
  let tmp = spare.join(format!("{}-{}", hashed(&dir.display().to_string()), std::process::id()));
  let _ = fs::remove_dir_all(&tmp);
  fs::create_dir_all(&tmp).map_err(refused)?;
  let made = make(&tmp);
  let made = match made {
    Ok(made) => made,
    Err(error) => {
      let _ = fs::remove_dir_all(&tmp);
      return Err(error);
    }
  };
  if refresh {
    let _ = fs::remove_dir_all(dir);
  }
  fs::create_dir_all(dir.parent().unwrap_or(dir)).map_err(refused)?;
  if fs::rename(&made, dir).is_err() && !dir.is_dir() {
    let _ = fs::remove_dir_all(&tmp);
    return Err(Error::Fetch {
      name: name.to_owned(),
      why: format!("no directory at {}", dir.display()),
    });
  }
  let _ = fs::remove_dir_all(&tmp);
  Ok(())
}

/// The directory an extension stands in, fetched into the cache once, and again on a refresh: a path stands where
/// it is; a git remote is cloned at its ref into a directory of the cache named by the hash of its url and its ref;
/// an npm package is packed by npm and unpacked into a directory of the cache named by its name and its version.
pub fn fetched(
  name: &str,
  source: &Source,
  places: &Places,
  refresh: bool,
) -> Result<PathBuf, Error> {
  let refused = |why: String| Error::Fetch { name: name.to_owned(), why };
  match source {
    Source::Builtin => Err(refused("a builtin stands in no directory".to_owned())),
    Source::Path(path) if path.is_dir() => Ok(path.clone()),
    Source::Path(path) => Err(refused(format!("no directory at {}", path.display()))),
    Source::Git { url, reference, path } => {
      let key = format!("{url}#{}", reference.as_deref().unwrap_or(""));
      let dir = places.cache.join("extensions").join("git").join(hashed(&key));
      placed(name, &dir, places, refresh, |tmp| {
        let into = tmp.join("clone");
        let into = into.display().to_string();
        let mut args = vec![
          "-c",
          "core.autocrlf=false",
          "-c",
          "advice.detachedHead=false",
          "clone",
          "--depth",
          "1",
        ];
        if let Some(reference) = reference {
          args.extend(["--branch", reference]);
        }
        args.extend([url.as_str(), into.as_str()]);
        ran(name, "git", &args, None)?;
        Ok(tmp.join("clone"))
      })?;
      let root = path.as_ref().map_or(dir.clone(), |sub| dir.join(sub));
      if root.is_dir() {
        Ok(root)
      } else {
        Err(refused(format!("no directory at {}", root.display())))
      }
    }
    Source::Npm { package, version } => {
      let dir = npm_dir(places, package, version.as_deref());
      placed(name, &dir, places, refresh, |tmp| {
        let spec =
          version.as_ref().map_or(package.clone(), |version| format!("{package}@{version}"));
        let at = tmp.display().to_string();
        let said = ran(
          name,
          npm(),
          &["pack", &spec, "--pack-destination", &at, "--json", "--ignore-scripts"],
          Some(tmp),
        )?;
        let packed: Value = serde_json::from_str(&said)
          .map_err(|why| refused(format!("npm pack said no json: {why}")))?;
        let Some(file) = packed.get(0).and_then(|one| one.get("filename")).and_then(Value::as_str)
        else {
          return Err(refused("npm pack named no file".to_owned()));
        };
        let tarball = fs::File::open(tmp.join(file.replace('/', "-").trim_start_matches('@')))
          .or_else(|_| fs::File::open(tmp.join(file)))
          .map_err(|why| refused(format!("{file}: {why}")))?;
        let unpacked = tmp.join("unpacked");
        tar::Archive::new(flate2::read::GzDecoder::new(tarball))
          .unpack(&unpacked)
          .map_err(|why| refused(format!("{file}: {why}")))?;
        Ok(unpacked.join("package"))
      })?;
      Ok(dir)
    }
  }
}

/// The dependencies of an extension installed, when its `package.json` names any and no `node_modules` stands:
/// what a host that loads the part of an extension for a World in TypeScript needs.
pub fn installed(name: &str, root: &Path) -> Result<(), Error> {
  let Ok(text) = fs::read_to_string(root.join("package.json")) else {
    return Ok(());
  };
  let read: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
  let needs =
    read.get("dependencies").and_then(Value::as_object).is_some_and(|one| !one.is_empty());
  if needs && !root.join("node_modules").is_dir() {
    ran(name, npm(), &["install", "--omit=dev", "--ignore-scripts"], Some(root))?;
  }
  Ok(())
}

/// The parts of an extension for a World, one file for each language a host writes its World in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Worlds {
  pub ts: Option<PathBuf>,
  pub py: Option<PathBuf>,
}

/// The manifest of an extension: its name, the file of its python part, the word it plays in every life, the files
/// of its parts for a World and for a TUI, and the names it requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
  pub name: String,
  pub python: Option<PathBuf>,
  pub life: Option<String>,
  pub world: Worlds,
  pub tui: Option<PathBuf>,
  pub requires: Vec<String>,
}

/// The manifest that the text of a `package.json` holds in its field `furb`, each file of it joined to the root.
/// Every part is optional and one at least is named, and a file out of the root is refused.
pub fn manifest_of(text: &str, root: &Path) -> Result<Manifest, Error> {
  let refused = |why: &str| Error::Manifest { root: root.to_owned(), why: why.to_owned() };
  let read: Value = serde_json::from_str(text)
    .map_err(|why| refused(&format!("package.json is no json: {why}")))?;
  let Some(Value::Object(furb)) = read.get("furb") else {
    return Err(refused("package.json has no object furb"));
  };
  let text = |key: &str| match furb.get(key) {
    None => Ok(None),
    Some(Value::String(one)) => Ok(Some(one.clone())),
    Some(_) => Err(refused(&format!("furb.{key} is no string"))),
  };
  let inside = |key: &str, one: Option<String>| -> Result<Option<PathBuf>, Error> {
    let Some(one) = one else { return Ok(None) };
    let mut depth = 0usize;
    for part in Path::new(&one).components() {
      match part {
        Component::Normal(_) => depth += 1,
        Component::CurDir => {}
        Component::ParentDir if depth > 0 => depth -= 1,
        _ => return Err(refused(&format!("furb.{key} leaves the directory of the extension"))),
      }
    }
    Ok(Some(root.join(one)))
  };
  let Some(name) = text("name")? else {
    return Err(refused("furb has no name"));
  };
  let world = match furb.get("world") {
    None => Worlds::default(),
    Some(Value::Object(worlds)) => {
      let of = |key: &str| match worlds.get(key) {
        None => Ok(None),
        Some(Value::String(one)) => inside("world", Some(one.clone())),
        Some(_) => Err(refused(&format!("furb.world.{key} is no string"))),
      };
      Worlds { ts: of("ts")?, py: of("py")? }
    }
    Some(_) => return Err(refused("furb.world is no object of a file for each language")),
  };
  let requires = match furb.get("requires") {
    None => Vec::new(),
    Some(Value::Array(names)) => names
      .iter()
      .map(|one| {
        one.as_str().map(str::to_owned).ok_or_else(|| refused("furb.requires holds no name"))
      })
      .collect::<Result<_, _>>()?,
    Some(_) => return Err(refused("furb.requires is no list of names")),
  };
  let manifest = Manifest {
    name,
    python: inside("python", text("python")?)?,
    life: text("life")?,
    world,
    tui: inside("tui", text("tui")?)?,
    requires,
  };
  if manifest.python.is_none()
    && manifest.life.is_none()
    && manifest.world == Worlds::default()
    && manifest.tui.is_none()
  {
    return Err(refused("furb names no part: python, life, world or tui"));
  }
  Ok(manifest)
}

/// The manifest of the extension in a directory, from its `package.json`.
pub fn manifest(root: &Path) -> Result<Manifest, Error> {
  let file = root.join("package.json");
  let text = fs::read_to_string(&file).map_err(|why| Error::Manifest {
    root: root.to_owned(),
    why: format!("no package.json: {why}"),
  })?;
  manifest_of(&text, root)
}

/// One extension as a host plays it: its name, the directory it stands in (none for a builtin), the word of its
/// python part, the word it plays in every life, the names it requires, and the files of its other parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
  pub name: String,
  pub root: Option<PathBuf>,
  pub word: Option<String>,
  pub life: Option<String>,
  pub requires: Vec<String>,
  pub world: Worlds,
  pub tui: Option<PathBuf>,
}

/// The builtin extensions, in the order a host plays them: files, bash, which requires files, and grant. Their
/// words are the words of the files the python package ships, and their other parts are each host's own.
pub fn builtins() -> Vec<Extension> {
  let builtin = |name: &str, source: &str, requires: &[&str]| Extension {
    name: name.to_owned(),
    root: None,
    word: Some(
      word(source).unwrap_or_else(|error| panic!("the builtin {name} does not parse: {error}")),
    ),
    life: None,
    requires: requires.iter().map(|&one| one.to_owned()).collect(),
    world: Worlds::default(),
    tui: None,
  };
  vec![
    builtin("files", include_str!("furb/builtin/files.py"), &[]),
    builtin("bash", include_str!("furb/builtin/bash.py"), &["files"]),
    builtin("grant", include_str!("furb/builtin/grant.py"), &[]),
  ]
}

/// The extension in a directory, loaded under the name its config gives it: its manifest, whose name must be that
/// name, and the word of its python part.
pub fn loaded(name: &str, root: &Path) -> Result<Extension, Error> {
  let read = manifest(root)?;
  if read.name != name {
    return Err(Error::Manifest {
      root: root.to_owned(),
      why: format!("the name {} is not the key {name} of its config", read.name),
    });
  }
  let word = match &read.python {
    None => None,
    Some(file) => {
      let text = fs::read_to_string(file).map_err(|why| Error::Manifest {
        root: root.to_owned(),
        why: format!("{}: {why}", file.display()),
      })?;
      Some(word(&text).map_err(|error| match error {
        Error::Word { line, why, .. } => Error::Word { name: name.to_owned(), line, why },
        other => other,
      })?)
    }
  };
  Ok(Extension {
    name: read.name,
    root: Some(root.to_owned()),
    word,
    life: read.life.map(|one| one.replace("\r\n", "\n")),
    requires: read.requires,
    world: read.world,
    tui: read.tui,
  })
}

/// The extensions in the order a host plays them: each after what it requires, and otherwise in the order of the
/// entries. A requirement that is off, or that no config names, is refused, and so are extensions that require each
/// other.
pub fn ordered(entries: &[Entry], extensions: Vec<Extension>) -> Result<Vec<Extension>, Error> {
  for one in &extensions {
    for needs in &one.requires {
      if !extensions.iter().any(|other| &other.name == needs) {
        let off = entries.iter().any(|entry| &entry.name == needs && !entry.on);
        return Err(Error::Requires { name: one.name.clone(), needs: needs.clone(), off });
      }
    }
  }
  let rank = |name: &str| entries.iter().position(|entry| entry.name == name).unwrap_or(usize::MAX);
  let mut left = extensions;
  left.sort_by_key(|one| rank(&one.name));
  let mut placed: HashSet<String> = HashSet::new();
  let mut out = Vec::new();
  while !left.is_empty() {
    let Some(at) =
      left.iter().position(|one| one.requires.iter().all(|needs| placed.contains(needs)))
    else {
      return Err(Error::Cycle { names: left.into_iter().map(|one| one.name).collect() });
    };
    let one = left.remove(at);
    placed.insert(one.name.clone());
    out.push(one);
  }
  Ok(out)
}

/// The extensions a host plays, from the configs of the user and of the project: each entry that is on, the builtin
/// or the one fetched into the cache, loaded, with its dependencies installed when `install` says so and it has a
/// part for a World in TypeScript, and ordered.
pub fn extensions(
  places: &Places,
  project: &Path,
  refresh: bool,
  install: bool,
) -> Result<Vec<Extension>, Error> {
  let entries = config(places, project)?;
  let builtin = builtins();
  let mut out = Vec::new();
  for entry in entries.iter().filter(|one| one.on) {
    match &entry.source {
      Some(Source::Builtin) | None => {
        out.extend(builtin.iter().filter(|one| one.name == entry.name).cloned());
      }
      Some(source) => {
        let root = fetched(&entry.name, source, places, refresh)?;
        let one = loaded(&entry.name, &root)?;
        if install && one.world.ts.is_some() {
          installed(&one.name, &root)?;
        }
        out.push(one);
      }
    }
  }
  ordered(&entries, out)
}

/// The words of the python parts of the extensions, in their order, which a host plays once on each chain without
/// a source.
pub fn words(extensions: &[Extension]) -> Vec<String> {
  extensions.iter().filter_map(|one| one.word.clone()).collect()
}

/// The life words of the extensions, in their order, which a host plays in every life on each chain without a
/// source, after the words.
pub fn lives(extensions: &[Extension]) -> Vec<String> {
  extensions.iter().filter_map(|one| one.life.clone()).collect()
}

/// The words that a program lacks, in their order: a word is missing when no word of the program is the same
/// string. This is the rule a host plays by, after boot on every chain without a source and at the birth of each.
pub fn missing<'a>(program: &[&str], words: &'a [String]) -> Vec<&'a str> {
  words.iter().map(String::as_str).filter(|one| !program.contains(one)).collect()
}

/// The word of a python part: the file with its line ends made LF, and every top-level `from furb... import` made
/// blank, and nothing else changed.
///
/// A line end is LF first, so a word is the same on every machine, and a clone that writes CRLF plays no word
/// again. Each line of such an import becomes an empty line, so every other line keeps its number and a finding of
/// the gate points at the line of the file. An import that shares its line with another statement leaves that
/// statement, and the semicolon between the two goes with the import. Every other byte stays as it is, the space at
/// the end of a line among them, since a string may hold it. A file with no such import is a word already. A file
/// python cannot parse is refused with the line of the fault, so a host says it once when it loads the extension
/// and no chain plays a broken word.
pub fn word(source: &str) -> Result<String, Error> {
  let source = source.replace("\r\n", "\n");
  let parsed = ruff_python_parser::parse_module(&source).map_err(|fault| Error::Word {
    name: String::new(),
    line: source[..usize::from(fault.location.start())].matches('\n').count() + 1,
    why: fault.error.to_string(),
  })?;
  let mut cuts: Vec<TextRange> = parsed
    .syntax()
    .body
    .iter()
    .filter_map(|statement| match statement {
      Stmt::ImportFrom(import)
        if import.level == 0
          && import.module.as_ref().is_some_and(|module| {
            let name = module.id.as_str();
            name == PACKAGE || name.strip_prefix(PACKAGE).is_some_and(|rest| rest.starts_with('.'))
          }) =>
      {
        Some(import.range())
      }
      _ => None,
    })
    .collect();
  cuts.reverse();
  let mut out = source.clone();
  for cut in cuts {
    let (start, end) = joined(&out, usize::from(cut.start()), usize::from(cut.end()));
    let kept: String = out[start..end].chars().filter(|&c| c == '\n').collect();
    out.replace_range(start..end, &kept);
  }
  Ok(out)
}

/// The span of an import with the semicolon that joins it to a statement on its line: the one after it, or else
/// the one before it.
fn joined(text: &str, start: usize, end: usize) -> (usize, usize) {
  let after = &text[end..];
  let gap = after.len() - after.trim_start_matches([' ', '\t']).len();
  if after[gap..].starts_with(';') {
    let rest = &after[gap + 1..];
    let more = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    return (start, end + gap + 1 + more);
  }
  let before = &text[..start];
  let gap = before.len() - before.trim_end_matches([' ', '\t']).len();
  if before[..before.len() - gap].ends_with(';') {
    return (start - gap - 1, end);
  }
  (start, end)
}

#[cfg(test)]
#[path = "extension.test.rs"]
mod tests;
