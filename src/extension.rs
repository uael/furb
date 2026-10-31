//! The extensions: what an extension is, and where a host finds the ones that a life runs.
//!
//! An extension stands in a folder that holds a manifest, `furb.json`, which gives its name, the file of its word,
//! its life word, where it plays and the file of its part for a TUI, and that file: a python module that imports what it uses from `furb.engine`, so an editor
//! reads it. The word is that file less those imports. The official extensions are the crate's own, which it
//! carries in the same form.
//!
//! A host finds the extensions in the configs that [`configs`] gives, in its order, each `{"extensions": {name:
//! true, false, or a path}}`: `config.json` in the config directory of the user, `.furb/config.json` in the directory
//! of the life, then `.furb/config.local.json` beside it, which git ignores. A later config wins for a name.
//!
//! An extension enters a life as rungs, and never enters the module of the engine or the system prompt, so a chain
//! that takes one grows at its end, and what a provider caches of it stands. The ear [`extensions`] enables each
//! extension by a fact that the record keeps, and plays the word and the life word as a rung on each chain where the
//! extension plays, and the definitions alone of a new word of an extension that the life runs, which reloads it.

pub mod compaction;
pub mod mcp;
pub mod memory;
pub mod skills;
pub mod web;

use std::{
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
};

use ruff_python_ast::{Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use serde_json::Value;

use crate::{
  ear::{Co, Ear, hear, hearing, say},
  engine::ROOT,
  fact::{Fact, named},
  value::{Fault, Object, Text, entry},
  world::files,
};

/// The kind of the fact that enables an extension in a life, which [`extensions`] says and reads.
pub const ENABLE: &str = "enable";

/// The journal, which says the record again in a later life, so a done it says answers no stand of this life.
const JOURNAL: &str = "journal";

/// The kind of the fact by which the ear teaches the doctrine to a chain that lives when a stand of this life lands,
/// `("teach", chain, "extensions", actor, path)`, which the record keeps, so a later life reads the doctrine again
/// where the journal says the fact. A chain born later reads it at its birth, from the standing alone.
const TEACH: &str = "teach";

/// The kind of the act `setup1`, `("setup", "setup1", "extensions", "chain1")`, which the ear makes on the root as it
/// plays its setup, after the rungs of the setup that every chain shares and before those of the extensions of the root
/// alone, and answers with nothing, so a chain with no source holds none of the acts that those make. A later life
/// makes it again at the same place, as each rung of the setup.
const SETUP: &str = "setup";

/// What the manifest gives under `plays` for an extension that plays on every chain, which is the default.
const EVERY: &str = "every";

/// What the manifest gives under `plays` for an extension that plays on the root alone, and what the fact enable
/// carries after the life word for it.
const ROOTED: &str = "root";

/// The official extensions, each as its manifest, the file of its word, and the file of the part for a TUI that its
/// manifest names, in the order a life runs them.
const OFFICIAL: [(&str, &str, Option<&str>); 7] = [
  (
    include_str!("../extensions/catchup/furb.json"),
    include_str!("../extensions/catchup/catchup.py"),
    None,
  ),
  (
    include_str!("../extensions/memory/furb.json"),
    include_str!("../extensions/memory/memory.py"),
    None,
  ),
  (
    include_str!("../extensions/skills/furb.json"),
    include_str!("../extensions/skills/skills.py"),
    None,
  ),
  (
    include_str!("../extensions/compaction/furb.json"),
    include_str!("../extensions/compaction/compaction.py"),
    Some(include_str!("../extensions/compaction/tui.ts")),
  ),
  (
    include_str!("../extensions/ouroboros/furb.json"),
    include_str!("../extensions/ouroboros/ouroboros.py"),
    None,
  ),
  (include_str!("../extensions/web/furb.json"), include_str!("../extensions/web/web.py"), None),
  (
    include_str!("../extensions/mcp/furb.json"),
    include_str!("../extensions/mcp/mcp.py"),
    Some(include_str!("../extensions/mcp/tui.ts")),
  ),
];

/// Whether a line imports furb or a module of it, as `from furb import`, `from furb.engine import` or `import
/// furb`, which the word of a file leaves out. A module whose name only starts with furb is another module.
fn imports_furb(line: &str) -> bool {
  let module = line.strip_prefix("from ").or_else(|| line.strip_prefix("import "));
  let after = module.and_then(|one| one.strip_prefix("furb"));
  after.is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '.', ',']))
}

/// An extension: its name, its word, its life word, which is empty when it has none, and whether it plays on the root
/// alone. An extension that plays on the root alone plays on no other chain but one whose source had it, since that
/// chain makes the rung of its source again; any other extension plays on every chain.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", napi_derive::napi(object))]
#[cfg_attr(feature = "python", derive(pyo3::FromPyObject, pyo3::IntoPyObject), pyo3(from_item_all))]
pub struct Extension {
  pub name: String,
  pub word: String,
  pub life: String,
  pub root: bool,
}

impl Extension {
  /// The extension that a fact which enables one carries.
  fn of(a: &Fact) -> Option<Extension> {
    let text = |at| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
    let root = text(3).as_deref() == Some(ROOTED);
    Some(Extension { name: text(0)?, word: text(1)?, life: text(2)?, root })
  }

  /// The saying that enables the extension, about the root: its name, its word, its life word, whether it plays on the
  /// root alone. An enable of an earlier build holds the file of the word after these, which no life reads.
  fn enable(&self) -> Fact {
    let rooted = if self.root { ROOTED } else { "" };
    let words = [self.name.as_str(), self.word.as_str(), self.life.as_str(), rooted];
    Fact::says(ENABLE, ROOT, words.map(Object::string))
  }
}

/// Where the extensions of a user stand: the config directory, and the home that a `~` of a config names.
#[derive(Clone, Debug)]
pub struct Places {
  pub config: PathBuf,
  pub home: Option<PathBuf>,
}

impl Places {
  /// The places of this process: the config directory is `FURB_CONFIG_DIR`, else `furb` under `XDG_CONFIG_HOME`,
  /// under `APPDATA` on Windows, or under `.config` in the home, where the TUI keeps its preferences too.
  pub fn here() -> Places {
    let var = |key: &str| std::env::var_os(key).map(PathBuf::from);
    let home = std::env::home_dir();
    let config = var("FURB_CONFIG_DIR").unwrap_or_else(|| {
      var("XDG_CONFIG_HOME")
        .or_else(|| var("APPDATA").filter(|_| cfg!(windows)))
        .or_else(|| home.as_ref().map(|one| one.join(".config")))
        .unwrap_or_default()
        .join("furb")
    });
    Places { config, home }
  }
}

/// The configs that each reader reads, in this order, a later one winning over an earlier one for a name:
/// `config.json` in the config directory of the user, then the configs of the project that [`projected`] gives.
pub fn configs(user: &Path, project: &Path) -> [PathBuf; 3] {
  let [shared, local] = projected(project);
  [user.join("config.json"), shared, local]
}

/// The configs of a project, in the order of [`configs`]: `.furb/config.json`, which git keeps, then
/// `.furb/config.local.json`, which git ignores, so a project keeps there what serves this machine alone, as its local
/// models and its local extensions.
pub fn projected(project: &Path) -> [PathBuf; 2] {
  let furb = project.join(".furb");
  [furb.join("config.json"), furb.join("config.local.json")]
}

/// The official extensions, in the order a life runs them, from the text of each that the crate carries.
pub fn official() -> Vec<Extension> {
  let whole = "an official extension is whole";
  OFFICIAL
    .iter()
    .map(|(manifest, word, _)| manifested(manifest, |_| Ok(word)).expect(whole))
    .collect()
}

/// The extension in a folder, under the name that its config gives it: its manifest, whose name must be that
/// name, and the word of the file that the manifest names.
pub fn load(name: &str, root: &Path) -> Result<Extension, Fault> {
  let file = root.join("furb.json");
  let refused = |why: String| Fault::refused(format!("{why}: {}", file.display()));
  let text = fs::read_to_string(&file).map_err(|no| refused(no.to_string()))?;
  let read = |word: &str| fs::read_to_string(root.join(word)).map_err(|no| format!("{word}: {no}"));
  let one = manifested(&text, read).map_err(refused)?;
  let lawful = name.starts_with(|one: char| one.is_ascii_lowercase())
    && name.chars().all(|one| one.is_ascii_lowercase() || one.is_ascii_digit() || one == '-');
  if !lawful || one.name != name {
    let why =
      format!("The manifest must give the name {name}, a letter then letters, digits and -");
    return Err(refused(why));
  }
  Ok(one)
}

/// An extension from the text of its manifest, with the text of its word as `read` gives it for the file named.
fn manifested<T: AsRef<str>>(
  text: &str,
  read: impl FnOnce(&str) -> Result<T, String>,
) -> Result<Extension, String> {
  let manifest: Value = serde_json::from_str(text).map_err(|no| no.to_string())?;
  let (Some(name), Some(file)) = (field(&manifest, "name")?, field(&manifest, "word")?) else {
    return Err("A manifest gives a name and a word".to_owned());
  };
  let life = field(&manifest, "life")?.unwrap_or_default().to_owned();
  let root = match field(&manifest, "plays")? {
    None | Some(EVERY) => false,
    Some(ROOTED) => true,
    Some(other) => {
      return Err(format!(
        "The manifest plays {other:?}, and an extension plays \"{EVERY}\" or \"{ROOTED}\""
      ));
    }
  };
  field(&manifest, "tui")?;
  let word = word(read(file)?.as_ref());
  Ok(Extension { name: name.to_owned(), word, life, root })
}

/// The text that a manifest gives under a key, or none when it gives none, and a refusal when it is not text.
fn field<'a>(manifest: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
  match manifest.get(key) {
    None => Ok(None),
    Some(Value::String(one)) => Ok(Some(one.as_str())),
    Some(_) => Err(format!("The {key} of the manifest is no text")),
  }
}

/// The extensions that the configs of the user and of the project turn on, in the order a life runs them: each
/// official one unless a config turns it off, then each other one in the order a config first names it. A config
/// gives the folder of an extension that is not official by its path, which resolves against the folder of the
/// config, `~` being the home, and `true` keeps the path that an earlier config gave.
pub fn configured(places: &Places, project: &Path) -> Result<Vec<Extension>, Fault> {
  let official = official();
  let on = turned_on(places, project)?.into_iter();
  on.map(|(name, path)| match path {
    Some(root) => load(&name, &root),
    // A copy of an official extension in the folder of the extensions of the project prevails over the one that furb
    // carries.
    None if project.join(".furb").join("extensions").join(&name).join("furb.json").is_file() => {
      load(&name, &project.join(".furb").join("extensions").join(&name))
    }
    None => {
      Ok(official.iter().find(|one| one.name == name).cloned().expect("an official extension"))
    }
  })
  .collect()
}

/// The name of each extension that the configs turn on, in the order a life runs them, with the path of its folder,
/// which an official extension has not.
fn turned_on(places: &Places, project: &Path) -> Result<Vec<(String, Option<PathBuf>)>, Fault> {
  let official = official();
  let mut named: Vec<(String, Option<PathBuf>, bool)> =
    official.iter().map(|one| (one.name.clone(), None, true)).collect();
  for file in configs(&places.config, project) {
    let refused = |why: String| Fault::refused(format!("{why}: {}", file.display()));
    let text = match fs::read_to_string(&file) {
      Err(no) if no.kind() == ErrorKind::NotFound => continue,
      got => got.map_err(|no| refused(no.to_string()))?,
    };
    let read: Value = serde_json::from_str(&text).map_err(|no| refused(no.to_string()))?;
    let extensions = match read.get("extensions") {
      Some(Value::Object(extensions)) => extensions,
      None if read.is_object() => continue,
      _ => return Err(refused("A config is a map, whose extensions is a map".to_owned())),
    };
    for (name, form) in extensions {
      let at = named.iter().position(|one| &one.0 == name).unwrap_or_else(|| {
        named.push((name.clone(), None, false));
        named.len() - 1
      });
      let one = &mut named[at];
      let theirs = at >= official.len();
      match form {
        Value::Bool(true) if theirs && one.1.is_none() => {
          return Err(refused(format!("No config gives the folder of the extension {name}")));
        }
        Value::Bool(on) => one.2 = *on,
        Value::String(path) if theirs => (one.1, one.2) = (Some(beside(&file, path, places)), true),
        _ => {
          let form = "is true, false, or the path of its folder when it is not official";
          return Err(refused(format!("The extension {name} {form}")));
        }
      }
    }
  }
  // The official extensions stand first, and each other one that is on has the path of its folder.
  Ok(named.into_iter().filter(|one| one.2).map(|(name, path, _)| (name, path)).collect())
}

/// The part of an extension that a TUI loads: the name of the extension, the path of its module of TypeScript, which
/// the manifest names under `tui`, beside the word, and the text of that module when the crate carries it. The crate
/// carries the part of an official extension, so a TUI loads it from the text where no checkout of furb stands, as in
/// an installed package, and the path is then its place in the repository of furb.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", napi_derive::napi(object))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct Part {
  pub name: String,
  pub file: String,
  pub text: Option<String>,
}

/// The part for a TUI of each extension that the configs turn on and whose manifest names one, in the order a life
/// runs them. The part of an official extension is the one that the crate carries, and the manifest of each other
/// folder is read again, so a part that a later write names is found.
pub fn parts(places: &Places, project: &Path) -> Result<Vec<Part>, Fault> {
  let mut out = Vec::new();
  for (name, root) in turned_on(places, project)? {
    let Some(root) = root else {
      out.extend(carried(&name));
      continue;
    };
    let file = root.join("furb.json");
    let refused = |why: String| Fault::refused(format!("{why}: {}", file.display()));
    let text = fs::read_to_string(&file).map_err(|no| refused(no.to_string()))?;
    let manifest: Value = serde_json::from_str(&text).map_err(|no| refused(no.to_string()))?;
    if let Some(tui) = field(&manifest, "tui").map_err(refused)? {
      out.push(Part { name, file: root.join(tui).display().to_string(), text: None });
    }
  }
  Ok(out)
}

/// The part that the crate carries for an official extension, when its manifest names one.
fn carried(name: &str) -> Option<Part> {
  OFFICIAL.iter().find_map(|(manifest, _, text)| {
    let manifest: Value = serde_json::from_str(manifest).expect("an official extension is whole");
    let tui = field(&manifest, "tui").ok()??;
    let file = format!("extensions/{name}/{tui}");
    (manifest["name"] == name).then(|| Part {
      name: name.to_owned(),
      file,
      text: text.map(str::to_owned),
    })
  })
}

/// A path that a config gives, resolved against the folder of the config, with `~` as the home. The path is joined
/// part by part, so each separator is that of the system, as `extensions/todo` gives `extensions\\todo` on Windows.
fn beside(file: &Path, path: &str, places: &Places) -> PathBuf {
  let native = |one: &str| Path::new(one).components().collect::<PathBuf>();
  match (path.strip_prefix('~'), &places.home) {
    (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
      home.join(native(rest.trim_start_matches(['/', '\\'])))
    }
    _ => file.parent().unwrap_or(Path::new("")).join(native(path)),
  }
}

/// The extensions that a config of the project turns on by the path of their folder, which a write of that config
/// gives in the middle of a life. A config that is not of the form of a config, and a folder that holds no
/// extension, turn on nothing.
fn turned(file: &Path, text: &str, places: &Places) -> Vec<Extension> {
  let Some(Value::Object(extensions)) =
    serde_json::from_str::<Value>(text).ok().and_then(|one| one.get("extensions").cloned())
  else {
    return Vec::new();
  };
  let folder = |form: &Value| form.as_str().map(|path| beside(file, path, places));
  let found = extensions.iter().filter_map(|(name, form)| load(name, &folder(form)?).ok());
  found.collect()
}

/// The word of a file: the file with its line ends made LF, less each top level import of furb and the lines up to
/// the `)` of one that opens a `(`, with no empty line at its start or at its end, and no run of more than two
/// empty lines.
pub fn word(source: &str) -> String {
  let (mut out, mut empty, mut open) = (String::new(), 0, false);
  for line in source.split('\n').map(|one| one.strip_suffix('\r').unwrap_or(one)) {
    if open || imports_furb(line) {
      open = !line.contains(')') && (open || line.contains('('));
    } else if line.is_empty() {
      empty += 1;
    } else {
      if !out.is_empty() {
        out.push_str(&"\n".repeat(empty.min(2) + 1));
      }
      out.push_str(line);
      empty = 0;
    }
  }
  out
}

/// The definitions of a word: each statement at its top that defines a function, a class, a type or a constant, an
/// import, and an assignment to names in capitals, in its order. A word that does not parse is its own definitions,
/// so the gate says why it is refused.
pub fn definitions(word: &str) -> String {
  shaped(word, false)
}

/// What a chain plays when it reloads a word: its [`definitions`] and, in the same order, each name of state that it
/// assigns, as `name = globals().setdefault("name", value)`. So a name that the module lacks takes its value, and a
/// name that the module holds keeps the value that it holds.
pub fn reloads(word: &str) -> String {
  shaped(word, true)
}

fn shaped(word: &str, state: bool) -> String {
  let Ok(parsed) = parse_module(word) else { return word.to_owned() };
  let capital = |target: &Expr| match target {
    Expr::Name(name) => {
      let id = name.id.as_str();
      id.chars().any(char::is_uppercase) && !id.chars().any(char::is_lowercase)
    }
    _ => false,
  };
  let defines = |one: &Stmt| match one {
    Stmt::FunctionDef(_)
    | Stmt::ClassDef(_)
    | Stmt::TypeAlias(_)
    | Stmt::Import(_)
    | Stmt::ImportFrom(_) => true,
    Stmt::Assign(one) => one.targets.iter().all(capital),
    Stmt::AnnAssign(one) => one.value.is_some() && capital(&one.target),
    _ => false,
  };
  let held = |id: &str, annotation: &str, value: &Expr, whole: &str| {
    let value = &word[value.range()];
    if value.starts_with("globals().setdefault(") {
      whole.to_owned()
    } else {
      format!("{id}{annotation} = globals().setdefault({id:?}, {value})")
    }
  };
  let kept: Vec<String> = parsed
    .syntax()
    .body
    .iter()
    .filter_map(|one| {
      let whole = &word[one.range()];
      if defines(one) {
        return Some(whole.to_owned());
      }
      match one {
        Stmt::Assign(two) if state => match two.targets.as_slice() {
          [Expr::Name(name)] => Some(held(name.id.as_str(), "", &two.value, whole)),
          _ => None,
        },
        Stmt::AnnAssign(two) if state => match (&*two.target, &two.value) {
          (Expr::Name(name), Some(value)) => {
            let annotation = format!(": {}", &word[two.annotation.range()]);
            Some(held(name.id.as_str(), &annotation, value, whole))
          }
          _ => None,
        },
        _ => None,
      }
    })
    .collect();
  kept.join("\n\n")
}

/// The extensions that a life runs, in the order it first enabled them, each with the word and the life word that
/// it last enabled, as the transcript of its root holds them.
pub fn enabled(root: &[Fact]) -> Vec<Extension> {
  let mut out: Vec<Extension> = Vec::new();
  for one in root.iter().filter(|a| a.kind() == ENABLE).filter_map(Extension::of) {
    match out.iter_mut().find(|two| two.name == one.name) {
      Some(two) => *two = one,
      None => out.push(one),
    }
  }
  out
}

/// The ear of the extensions, which says and hears the fact enable. `src/furb/AGENTS.md` says what the fact means and
/// when a life says it, under the names enable and life word and in its laws.
///
/// The fact is `("enable", "chain1", "extensions", name, word, life)`, with `"root"` after the life word for an
/// extension that plays on the root alone. No contract holds it, so this is the one home of its shape. The ear says it for each extension it is given that the life does not run as it is given, at each
/// stand that this life answers, the first of which is at the tip, and for an extension of a folder that a config
/// turns on, when a write of the life in that folder lands. The fact of an extension that the life runs already
/// reloads it: each chain plays the definitions of its new word and each name of state that it lacks, so each name of
/// state that it holds keeps its value, and its life word does not run again. The ear plays the rungs from the fact alone, so a later life plays the same rungs
/// where the journal says the fact again.
pub fn extensions(given: Vec<Extension>) -> Box<dyn Ear> {
  hearing(&["chain", "stand", "rung", "write", ENABLE, TEACH, SETUP], move |mut co| async move {
    // What the life enabled, in order, and each chain in the order of its birth, with the extensions it has.
    let (mut enabled, mut chains) = (Vec::<Extension>::new(), Vec::<(String, Vec<String>)>::new());
    // The setup of the root: the actor and the doctrine that the fact teach names with the enables still to hear
    // before it plays, and whether the root took it.
    let (mut teach, mut taught) = (None::<(String, String, i64)>, false);
    // Whether this life says the fact teach, and with it the fact setup, which the journal of a later life says again.
    let mut fresh = true;
    loop {
      let a = hear(&mut co).await;
      let runs_name = |name: &str| enabled.iter().any(|two| two.name == name);
      let runs = |one: &Extension| runs_name(&one.name);
      let new: Vec<Extension> = match a.kind() {
        // The ear hears a chain at its birth, before the call that made it returns, and the chain has what its
        // origin had, which it made again. A chain with no source in a life whose root took its setup forks that
        // setup, which holds each extension of every chain, so the ear plays nothing on it.
        "chain" if a.question() => {
          let source = a.word(2).and_then(|one| one.as_str().map(str::to_owned));
          let had =
            chains.iter().find(|one| Some(&one.0) == source.as_ref()).map(|one| one.1.clone());
          if taught {
            let every = enabled.iter().filter(|one| !one.root).map(|one| one.name.clone());
            chains.push((a.about().to_owned(), had.unwrap_or_else(|| every.collect())));
            continue;
          }
          chains.push((a.about().to_owned(), had.unwrap_or_default()));
          let born = chains.len() - 1;
          for one in &enabled {
            play(&mut co, one, &mut chains[born]).await?;
          }
          continue;
        }
        // The first stand that this life answers sets up the root of a new record when the standing names a
        // doctrine: the ear says the fact teach, with the count of the enables that follow it, then each enable, and
        // it plays the setup once it heard them, as the journal of a later life says them again.
        "done" if named(a.about(), "stand") && a.by() != JOURNAL => {
          let new: Vec<Extension> =
            given.iter().filter(|one| !enabled.contains(one)).cloned().collect();
          if !taught
            && teach.is_none()
            && let Some((actor, path)) = doctrine(&mut co).await?
            && bare(&mut co).await?
          {
            let count = Object::int(i64::try_from(new.len()).unwrap_or(i64::MAX));
            say(
              &mut co,
              Fact::says(TEACH, ROOT, [Object::string(&actor), Object::string(&path), count]),
            );
            for one in &new {
              say(&mut co, one.enable());
            }
            continue;
          }
          new
        }
        SETUP if a.question() && fresh => {
          say(&mut co, Fact::says("done", a.about(), [Object::none()]));
          continue;
        }
        TEACH => {
          let text = |at| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
          let count = a.word(2).and_then(|one| one.as_int()).unwrap_or(0);
          if let (Some(actor), Some(path)) = (text(0), text(1)) {
            teach = Some((actor, path, count));
            fresh = a.by() != JOURNAL;
          }
          if count > 0 {
            continue;
          }
          Vec::new()
        }
        ENABLE if teach.is_some() => {
          if let Some(one) = Extension::of(&a) {
            match enabled.iter().position(|two| two.name == one.name) {
              Some(at) => enabled[at] = one,
              None => enabled.push(one),
            }
          }
          if let Some(left) = teach.as_mut().map(|one| &mut one.2) {
            *left -= 1;
            if *left > 0 {
              continue;
            }
          }
          Vec::new()
        }
        // A write of a config of the project that lands turns on each extension that it gives by its path and
        // that the life does not run, as the configs do at the tip.
        // A write that lands of a config of the project turns on each extension that it gives by its path and that
        // the life does not run, as the configs do at the tip. A write in the folder of an extension that the life
        // runs and that a config turns on reloads it when its word changed.
        "done" if named(a.about(), "write") && a.by() != JOURNAL => {
          let Some(text) = a.word(0).and_then(Text::of) else { continue };
          let Ok(file) = files::placed(Path::new(""), &text.path) else { continue };
          let config = projected(Path::new("")).iter().any(|one| file.ends_with(one));
          if !config && !file.parent().is_some_and(|one| one.join("furb.json").is_file()) {
            continue;
          }
          let got = co.call("standing", vec![], vec![]).await?;
          let Some(Ok(project)) = entry(&got.as_ref(), 1)
            .and_then(|one| one.as_str().map(|one| files::placed(Path::new(""), one)))
          else {
            continue;
          };
          let places = Places::here();
          if projected(&project).contains(&file) {
            turned(&file, &text.content, &places).into_iter().filter(|one| !runs(one)).collect()
          } else {
            let folders = turned_on(&places, &project).unwrap_or_default().into_iter();
            let mine =
              folders.filter(|(name, root)| root.as_deref() == file.parent() && runs_name(name));
            let found = mine.filter_map(|(name, root)| load(&name, &root?).ok());
            found.filter(|one| !enabled.contains(one)).collect()
          }
        }
        ENABLE => Extension::of(&a).filter(|one| !enabled.contains(one)).into_iter().collect(),
        _ => continue,
      };
      // The setup of the root plays once the fact teach and each enable after it were heard.
      if let Some((actor, path, 0)) = teach.take() {
        setup(&mut co, &enabled, &actor, &path).await?;
        taught = true;
        let every = enabled.iter().filter(|one| !one.root).map(|one| one.name.clone()).collect();
        if let Some(root) = chains.iter_mut().find(|one| one.0 == ROOT) {
          root.1 = every;
        }
        continue;
      }
      // Each enable is said before any rung plays, so the journal of a later life says them again as one burst and
      // the rungs play in the order of the first life.
      if a.kind() != ENABLE {
        for one in &new {
          say(&mut co, one.enable());
        }
      }
      for one in new {
        let Some(at) = enabled.iter().position(|two| two.name == one.name) else {
          for chain in &mut chains {
            play(&mut co, &one, chain).await?;
          }
          enabled.push(one);
          continue;
        };
        if enabled[at].word != one.word {
          let word = reloads(&one.word);
          for (chain, had) in &chains {
            if had.contains(&one.name) {
              ring(&mut co, chain, &word).await?;
            }
          }
        }
        enabled[at] = one;
      }
    }
  })
}

/// One extension played on a chain as a rung: its word then its life word, or its life word alone on a chain that
/// has the extension already, and nothing on a chain other than the root that lacks an extension of the root alone.
async fn play(
  co: &mut Co,
  one: &Extension,
  (chain, had): &mut (String, Vec<String>),
) -> Result<(), Fault> {
  let word = if had.contains(&one.name) {
    one.life.clone()
  } else if one.root && chain != ROOT {
    return Ok(());
  } else {
    had.push(one.name.clone());
    let word = if one.word.trim().is_empty() { "" } else { one.word.as_str() };
    format!("{word}\n\n{}", one.life).trim_matches('\n').to_owned()
  };
  ring(co, chain, &word).await
}

/// The setup of the root, as turns of its default actor whose words furb writes: the read of the doctrine, then one
/// turn of the word of each extension of every chain, in the order of the life, and then of the life word of each that
/// has one, then the fact setup, then the same turn for the extensions of the root alone. It makes each rung at once,
/// and the chain gives them their turns in the order it made them, so the fact setup stands before the turn of the root
/// alone.
async fn setup(co: &mut Co, enabled: &[Extension], actor: &str, path: &str) -> Result<(), Fault> {
  written(co, ROOT, &reading(path), actor).await?;
  for root in [false, true] {
    if root {
      let words = [SETUP, ROOT].map(Object::string).into_iter().chain([Object::none()]);
      co.call("act", words.collect(), vec![]).await?;
    }
    loaded(co, enabled.iter().filter(|one| one.root == root), actor).await?;
  }
  Ok(())
}

/// The words of extensions on the root as one turn of an actor, each word then each life word, so the model reads in
/// its context the code that ran in that turn, the module holds what it defines at the same place, and the turn after it
/// holds what each life word told. A word that is blank, as that of an extension that holds only a part for the TUI,
/// stands in no turn: a run of no word would stand in the program of the chain as a word that a replay asks a model for.
/// It plays no turn when it is given no word.
async fn loaded(
  co: &mut Co,
  given: impl Iterator<Item = &Extension>,
  actor: &str,
) -> Result<(), Fault> {
  let given: Vec<&Extension> = given.collect();
  let joined = |part: fn(&Extension) -> &str, by: &str| {
    given
      .iter()
      .map(|one| part(one))
      .filter(|one| !one.trim().is_empty())
      .collect::<Vec<_>>()
      .join(by)
  };
  let words = [joined(|one| &one.word, "\n\n"), joined(|one| &one.life, "\n")];
  let word = words.into_iter().filter(|one| !one.is_empty()).collect::<Vec<_>>().join("\n\n");
  if !word.is_empty() {
    written(co, ROOT, &word, actor).await?;
  }
  Ok(())
}

/// The word of a read of a path, as python writes it.
fn reading(path: &str) -> String {
  format!("read({})", Object::string(path).py_repr())
}

/// A turn of an actor on a chain whose word furb writes: a rung of that word and that actor, by its name.
async fn written(
  co: &mut Co,
  chain: &str,
  word: &str,
  actor: &str,
) -> Result<Option<String>, Fault> {
  let words = vec![("to", Object::string(actor)), ("on", Object::string(chain))];
  let made = co.call("rung", vec![Object::string(word)], words).await?;
  Ok(made.as_ref().as_str().map(str::to_owned))
}

/// The default actor and the path of the doctrine that the standing names, or none while it names no doctrine, as in a
/// record of a build before the doctrine was read.
async fn doctrine(co: &mut Co) -> Result<Option<(String, String)>, Fault> {
  let got = co.call("standing", vec![], vec![]).await?;
  let text = |at| entry(&got.as_ref(), at).and_then(|one| one.as_str().map(str::to_owned));
  Ok(text(2).zip(text(3)))
}

/// Whether the root holds no reply, as the root of a new record, since a root that a checkpoint carries holds the
/// replies that read what it needs.
async fn bare(co: &mut Co) -> Result<bool, Fault> {
  let of = [ROOT, "reply"].map(Object::string).to_vec();
  Ok(entry(&co.call("transcript", of, vec![]).await?.as_ref(), 0).is_none())
}

/// A word played on a chain as a rung, unless it is empty.
async fn ring(co: &mut Co, chain: &str, word: &str) -> Result<(), Fault> {
  if !word.is_empty() {
    let on = vec![("on", Object::string(chain))];
    co.call("rung", vec![Object::string(word)], on).await?;
  }
  Ok(())
}

#[cfg(test)]
#[path = "extension.test.rs"]
mod test;
