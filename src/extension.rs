//! The extensions: what an extension is, and where a host finds the ones that a life runs.
//!
//! An extension stands in a folder that holds a manifest, `furb.json`, which gives its name, the file of its word,
//! its life word, where it plays and the file of its part for a TUI, and that file: a python module that imports what it uses from `furb.engine`, so an editor
//! reads it. The word is that file less those imports. The official extensions are the crate's own, which it
//! carries in the same form.
//!
//! A host finds the extensions in two configs, `config.json` in the config directory of the user and
//! `.furb/config.json` in the directory of the life, each `{"extensions": {name: true, false, or a path}}`. The
//! config of the project comes after the config of the user, so it wins for a name.
//!
//! An extension enters a life as rungs, and never enters the module of the engine or the system prompt, so a chain
//! that takes one grows at its end, and what a provider caches of it stands. The ear [`extensions`] enables each
//! extension by a fact that the record keeps, and plays the word and the life word as a rung on each chain where the
//! extension plays, and the definitions alone of a new word of an extension that the life runs, which reloads it.

pub mod compaction;
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
};

/// The kind of the fact that enables an extension in a life, which [`extensions`] says and reads.
pub const ENABLE: &str = "enable";

/// The journal, which says the record again in a later life, so a done it says answers no stand of this life.
const JOURNAL: &str = "journal";

/// What the manifest gives under `plays` for an extension that plays on every chain, which is the default.
const EVERY: &str = "every";

/// What the manifest gives under `plays` for an extension that plays on the root alone, and what the fact enable
/// carries after the life word for it.
const ROOTED: &str = "root";

/// The official extensions, each as its manifest, the file of its word, and the file of the part for a TUI that its
/// manifest names, in the order a life runs them.
const OFFICIAL: [(&str, &str, Option<&str>); 5] = [
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

  /// The saying that enables the extension, about the root.
  fn enable(&self) -> Fact {
    let words = [&self.name, &self.word, &self.life].map(|one| Object::string(one.as_str()));
    let rooted = self.root.then(|| Object::string(ROOTED));
    Fact::says(ENABLE, ROOT, words.into_iter().chain(rooted))
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

/// The official extensions, in the order a life runs them.
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
  Ok(Extension { name: name.to_owned(), word: word(read(file)?.as_ref()), life, root })
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
  for file in [places.config.join("config.json"), project.join(".furb").join("config.json")] {
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
  let Ok(parsed) = parse_module(word) else { return word.to_owned() };
  let capital = |target: &Expr| match target {
    Expr::Name(name) => {
      let id = name.id.as_str();
      id.chars().any(char::is_uppercase) && !id.chars().any(char::is_lowercase)
    }
    _ => false,
  };
  let defines = |one: &&Stmt| match one {
    Stmt::FunctionDef(_)
    | Stmt::ClassDef(_)
    | Stmt::TypeAlias(_)
    | Stmt::Import(_)
    | Stmt::ImportFrom(_) => true,
    Stmt::Assign(one) => one.targets.iter().all(capital),
    Stmt::AnnAssign(one) => one.value.is_some() && capital(&one.target),
    _ => false,
  };
  let kept: Vec<&str> =
    parsed.syntax().body.iter().filter(defines).map(|one| &word[one.range()]).collect();
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
/// reloads it: each chain plays the definitions of its new word alone, so each name of state keeps its value, and its
/// life word does not run again. The ear plays the rungs from the fact alone, so a later life plays the same rungs
/// where the journal says the fact again.
pub fn extensions(given: Vec<Extension>) -> Box<dyn Ear> {
  hearing(&["chain", "stand", "write", ENABLE], move |mut co| async move {
    // What the life enabled, in order, and each chain in the order of its birth, with the extensions it has.
    let (mut enabled, mut chains) = (Vec::<Extension>::new(), Vec::<(String, Vec<String>)>::new());
    loop {
      let a = hear(&mut co).await;
      let runs_name = |name: &str| enabled.iter().any(|two| two.name == name);
      let runs = |one: &Extension| runs_name(&one.name);
      let new: Vec<Extension> = match a.kind() {
        // The ear hears a chain at its birth, before the call that made it returns, and the chain has what its
        // origin had, which it made again.
        "chain" if a.question() => {
          let source = a.word(2).and_then(|one| one.as_str().map(str::to_owned));
          let had =
            chains.iter().find(|one| Some(&one.0) == source.as_ref()).map(|one| one.1.clone());
          chains.push((a.about().to_owned(), had.unwrap_or_default()));
          let born = chains.len() - 1;
          for one in &enabled {
            play(&mut co, one, &mut chains[born]).await?;
          }
          continue;
        }
        "done" if named(a.about(), "stand") && a.by() != JOURNAL => {
          given.iter().filter(|one| !enabled.contains(one)).cloned().collect()
        }
        // A write of the config of the project that lands turns on each extension that it gives by its path and
        // that the life does not run, as the configs do at the tip.
        // A write that lands of the config of the project turns on each extension that it gives by its path and that
        // the life does not run, as the configs do at the tip. A write in the folder of an extension that the life
        // runs and that a config turns on reloads it when its word changed.
        "done" if named(a.about(), "write") && a.by() != JOURNAL => {
          let Some(text) = a.word(0).and_then(Text::of) else { continue };
          let file = PathBuf::from(&text.path);
          let config = file.ends_with(Path::new(".furb").join("config.json"));
          if !config && !file.parent().is_some_and(|one| one.join("furb.json").is_file()) {
            continue;
          }
          let got = co.call("standing", vec![], vec![]).await?;
          let Some(project) =
            entry(&got.as_ref(), 1).and_then(|one| one.as_str().map(PathBuf::from))
          else {
            continue;
          };
          let places = Places::here();
          if file == project.join(".furb").join("config.json") {
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
          let word = definitions(&one.word);
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
    format!("{}\n\n{}", one.word, one.life).trim_matches('\n').to_owned()
  };
  ring(co, chain, &word).await
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
