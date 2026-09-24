//! The extensions: what a host adds to the engine, and where it finds it.
//!
//! The system prompt of a life is the text the life runs: the engine, less the definitions of each builtin, files,
//! bash or grant, that the life does not take, then the words of the other extensions. The python part of an
//! extension is a word that the module of the engine runs after the engine, so every chain binds its names from its
//! birth. The file is a word as it is, or a python module that imports what it uses from the engine and from the
//! extensions it requires, so that an editor, ruff and ty read it. The module of the engine binds every name of the
//! engine and of each word before it already, so the host makes the word of a module by cutting those imports out.
//! A life word is a word that a host plays as a rung, as the World, in every life on each chain without a source.
//!
//! Every host shares this module, so every host reads the same config, fetches into the same cache, reads the same
//! manifests, orders the extensions the same, and makes the same words and the same system prompt. What stays in
//! each host is its own: the parts of an extension for a World and for a TUI, and how it loads their code.
//!
//! The config is a json file, `{"extensions": {name: form}}`, in the config directory of the user and in the
//! directory `.furb` of the project, which names an extension by its key: `false` turns it off, `true` turns it on,
//! and a path, a git remote or an npm package says where it stands. The builtins are on unless a config turns them
//! off. An extension stands in a directory whose `package.json` holds a field `furb`, its manifest: its name, which
//! is its key, the file of its python part, the word it plays in every life, the files of its parts for a World and
//! for a TUI, and the names it requires.

use std::{
  collections::HashSet,
  ffi::OsString,
  fmt, fs,
  path::{Component, Path, PathBuf},
  process::Command,
};

use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::{Object, life::WORLD, value::Fault};

/// The package the engine and the extensions are imported from, whose imports a word leaves out.
const PACKAGE: &str = "furb";

/// The builtins, in the order a host takes them: the name of each, the builtins it requires, and the top-level names
/// of the engine it defines, which leave the system prompt when it is off.
const BUILTINS: [(&str, &[&str], &[&str]); 3] = [
  (
    "files",
    &[],
    &[
      "span", "differs", "HEAD", "TAIL", "HIDDEN", "read", "write", "cd", "cwd", "landed",
      "showing",
    ],
  ),
  ("bash", &["files"], &["TIMEOUT", "bash", "Exit"]),
  ("grant", &[], &["WINDOW", "grant"]),
];

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
    Places::of(|key| std::env::var_os(key), std::env::home_dir(), cfg!(windows))
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
      home,
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

/// The settings of one config, read from its text: each name of its extensions with what it says, in the order of
/// the file. A path, and a git remote or an npm package that starts with `./`, `../` or `~`, resolves against the
/// directory of the file, and a `~` expands to the home.
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
      Value::Object(fields) if fields.contains_key("git") => Setting::From(Source::Git {
        url: nearby(&text(fields, "git")?.unwrap_or_default()),
        reference: text(fields, "ref")?,
        path: text(fields, "path")?.map(PathBuf::from),
      }),
      Value::Object(fields) if fields.contains_key("npm") => Setting::From(Source::Npm {
        package: nearby(&text(fields, "npm")?.unwrap_or_default()),
        version: text(fields, "version")?,
      }),
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
    .map(|&(name, ..)| Entry {
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
      && !one.starts_with(['.', '_'])
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
  let got = make(&tmp).and_then(|made| {
    if refresh {
      let _ = fs::remove_dir_all(dir);
    }
    fs::create_dir_all(dir.parent().unwrap_or(dir)).map_err(refused)?;
    if fs::rename(&made, dir).is_err() && !dir.is_dir() {
      return Err(Error::Fetch {
        name: name.to_owned(),
        why: format!("no directory at {}", dir.display()),
      });
    }
    Ok(())
  });
  let _ = fs::remove_dir_all(&tmp);
  got
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
        let clone = tmp.join("clone");
        let into = clone.display().to_string();
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
        Ok(clone)
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

/// The builtin extensions, in the order a host takes them: files, bash, which requires files, and grant. Each is
/// definitions of the engine, so none has a word, and their parts for a World and for a TUI are each host's own.
pub fn builtins() -> Vec<Extension> {
  BUILTINS
    .iter()
    .map(|&(name, requires, _)| Extension {
      name: name.to_owned(),
      root: None,
      word: None,
      life: None,
      requires: requires.iter().map(|&one| one.to_owned()).collect(),
      world: Worlds::default(),
      tui: None,
    })
    .collect()
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

/// The words of the python parts of the extensions, in their order, which the module of the engine runs after the
/// engine.
pub fn words(extensions: &[Extension]) -> Vec<String> {
  extensions.iter().filter_map(|one| one.word.clone()).collect()
}

/// The life words of the extensions, in their order, which a host plays as rungs in every life on each chain without
/// a source: once boot stands on its record, and at the birth of each such chain after.
pub fn lives(extensions: &[Extension]) -> Vec<String> {
  extensions.iter().filter_map(|one| one.life.clone()).collect()
}

/// The text, then each word after an empty line.
pub(crate) fn appended(text: &str, words: &[String]) -> String {
  let mut out = text.to_owned();
  for one in words {
    if !out.is_empty() && !out.ends_with('\n') {
      out.push('\n');
    }
    out.push('\n');
    out.push_str(one);
  }
  out
}

/// The kind of the fact by which the World pins what a life runs: the builtins it takes and the words.
pub const PINNED: &str = "extensions";

/// What a life on a record runs, the builtins it takes and the words, and whether it pins them: what the record pins,
/// or every builtin and no word for a record that pins nothing; a life on an empty record runs what it is given, and
/// pins it unless it is every builtin and no word.
pub fn pinned(
  record: &[Object],
  taken: &[String],
  words: &[String],
) -> (Vec<String>, Vec<String>, bool) {
  let every: Vec<String> = BUILTINS.iter().map(|&(name, ..)| name.to_owned()).collect();
  if let Some((taken, words)) = record.iter().find_map(pin) {
    return (taken, words, false);
  }
  if !record.is_empty() {
    return (every, Vec::new(), false);
  }
  let taken: Vec<String> = every.iter().filter(|one| taken.contains(one)).cloned().collect();
  let pins = taken != every || !words.is_empty();
  (taken, words.to_vec(), pins)
}

/// The builtins and the words of an entry that is a pin of the World.
fn pin(entry: &Object) -> Option<(Vec<String>, Vec<String>)> {
  let entry = entry.as_ref();
  let fact = crate::value::entry(&entry, 0)?.items()?;
  if fact.first()?.as_str()? != PINNED || fact.get(2)?.as_str()? != WORLD {
    return None;
  }
  let texts = |at: usize| -> Option<Vec<String>> {
    Some(fact.get(at)?.items()?.iter().filter_map(|one| one.as_str().map(str::to_owned)).collect())
  };
  Some((texts(3)?, texts(4)?))
}

/// The system prompt of a life, which is the text the life runs: the engine less the top-level statements that
/// define only names of a builtin that `taken` does not name, then the words.
pub fn system(engine: &str, taken: &[String], words: &[String]) -> Result<String, Error> {
  let off = cut_names(taken);
  let kept = cut(engine, |statement| {
    let names = binds(statement);
    !names.is_empty() && names.iter().all(|name| off.contains(name))
  })
  .map_err(|(line, why)| Error::Word { name: String::new(), line, why })?;
  Ok(appended(&kept, words))
}

/// The top-level names of the engine that each builtin not in `taken` defines.
pub fn cut_names(taken: &[String]) -> HashSet<&'static str> {
  BUILTINS
    .iter()
    .filter(|&&(name, ..)| !taken.iter().any(|one| one == name))
    .flat_map(|&(_, _, names)| names.iter().copied())
    .collect()
}

/// The names a top-level statement binds: a function, a class, the targets of an assignment, the names of an import
/// and a type alias, and none for any other statement.
fn binds(statement: &Stmt) -> Vec<&str> {
  fn targets(target: &Expr) -> Vec<&str> {
    match target {
      Expr::Name(one) => vec![one.id.as_str()],
      Expr::Tuple(one) => one.elts.iter().flat_map(targets).collect(),
      Expr::List(one) => one.elts.iter().flat_map(targets).collect(),
      _ => Vec::new(),
    }
  }
  match statement {
    Stmt::FunctionDef(one) => vec![one.name.as_str()],
    Stmt::ClassDef(one) => vec![one.name.as_str()],
    Stmt::Assign(one) => one.targets.iter().flat_map(targets).collect(),
    Stmt::AnnAssign(one) => targets(&one.target),
    Stmt::TypeAlias(one) => targets(&one.name),
    Stmt::ImportFrom(one) => {
      one.names.iter().map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str()).collect()
    }
    Stmt::Import(one) => one
      .names
      .iter()
      .map(|alias| {
        alias.asname.as_ref().map_or_else(
          || alias.name.as_str().split('.').next().unwrap_or_default(),
          |named| named.as_str(),
        )
      })
      .collect(),
    _ => Vec::new(),
  }
}

/// The word of a python part: the file with its line ends made LF, so a word is the same on every machine, less every
/// top-level `from furb... import`, which `cut` takes out with no trace, since a model reads the word. A file python
/// cannot parse is refused with the line of the fault, so a host says it once when it loads the extension, and no
/// life runs it.
pub fn word(source: &str) -> Result<String, Error> {
  cut(&source.replace("\r\n", "\n"), |statement| match statement {
    Stmt::ImportFrom(import) if import.level == 0 => import.module.as_ref().is_some_and(|module| {
      let name = module.id.as_str();
      name == PACKAGE || name.strip_prefix(PACKAGE).is_some_and(|rest| rest.starts_with('.'))
    }),
    _ => false,
  })
  .map_err(|(line, why)| Error::Word { name: String::new(), line, why })
}

/// The source less each top-level statement that `which` picks, and nothing else changed: the lines of a statement
/// go, and the empty lines around the place it stood keep the most of those before it and those after it, so two
/// top-level statements stand apart as they did, and the text starts and ends with its code. A statement that shares
/// its line with another leaves that statement, and the semicolon between the two goes with it. Every other byte
/// stays, the space at the end of a line among them, since a string may hold it; an empty line next to a top-level
/// statement is in no string. A source python cannot parse gives the line of the fault and what it is.
fn cut(source: &str, which: impl Fn(&Stmt) -> bool) -> Result<String, (usize, String)> {
  let parsed = ruff_python_parser::parse_module(source).map_err(|fault| {
    (
      source[..usize::from(fault.location.start())].matches('\n').count() + 1,
      fault.error.to_string(),
    )
  })?;
  let mut cuts: Vec<TextRange> =
    parsed.syntax().body.iter().filter(|&one| which(one)).map(Ranged::range).collect();
  cuts.reverse();
  let mut out = source.to_owned();
  for cut in cuts {
    let (start, end) = (usize::from(cut.start()), usize::from(cut.end()));
    let (first, rest) = (out[..start].rfind('\n').map_or(0, |at| at + 1), out[end..].find('\n'));
    let tail = rest.map_or(&out[end..], |at| &out[end..end + at]).trim_start();
    if out[first..start].trim().is_empty() && (tail.is_empty() || tail.starts_with('#')) {
      out.replace_range(first..rest.map_or(out.len(), |at| end + at + 1), "");
      spaced(&mut out, first);
    } else {
      let (start, end) = joined(&out, start, end);
      out.replace_range(start..end, "");
    }
  }
  Ok(out)
}

/// The empty lines around the place a statement was cut out at, made the most of those before it and those after it,
/// and none at the start or at the end of the text.
fn spaced(out: &mut String, at: usize) {
  let empty = |line: &str| line.trim().is_empty();
  let (mut from, mut before) = (at, 0);
  while from > 0 {
    let line = out[..from - 1].rfind('\n').map_or(0, |one| one + 1);
    if !empty(&out[line..from - 1]) {
      break;
    }
    (from, before) = (line, before + 1);
  }
  let (mut to, mut after) = (at, 0);
  while to < out.len() {
    let line = out[to..].find('\n').map_or(out.len(), |one| to + one);
    if !empty(&out[to..line]) {
      break;
    }
    (to, after) = ((line + 1).min(out.len()), after + 1);
  }
  let kept = if from == 0 || to == out.len() { 0 } else { before.max(after) };
  out.replace_range(from..to, &"\n".repeat(kept));
}

/// The span of a statement with the semicolon that joins it to another on its line: the one after it, or else
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
