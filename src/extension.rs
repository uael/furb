//! The extensions: what an extension is, and where a host finds the ones that a life runs.
//!
//! An extension stands in a folder that holds a manifest, `furb.json`, which gives its name, the file of its word
//! and its life word, and that file: a python module that imports what it uses from `furb.engine`, so an editor
//! reads it. The word is that file less those imports. The official extensions are the crate's own, which it
//! carries in the same form.
//!
//! A host finds the extensions in two configs, `config.json` in the config directory of the user and
//! `.furb/config.json` in the directory of the life, each `{"extensions": {name: true, false, or a path}}`. The
//! config of the project comes after the config of the user, so it wins for a name.
//!
//! An extension enters a life as rungs, and never enters the module of the engine or the system prompt, so a chain
//! that takes one grows at its end, and what a provider caches of it stands. The ear [`extensions`] enables each
//! extension by a fact that the record keeps, and plays the word and the life word as a rung on each chain.

pub mod skills;

use std::{
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{
  ear::{Co, Ear, call, ear, hear, say},
  fact::{Fact, named},
  value::{Fault, Object, entry},
};

/// The kind of the fact that enables an extension in a life: it carries the name, the word and the life word.
pub const ENABLE: &str = "enable";

/// The root, whose transcript holds each fact that enables an extension, as it holds the standing.
const ROOT: &str = "chain1";

/// The journal, which says the record again in a later life, so a done it says answers no stand of this life.
const JOURNAL: &str = "journal";

/// The official extensions, each as its manifest and the file of its word, in the order a life runs them.
const OFFICIAL: [(&str, &str); 1] = [(
  include_str!("../extensions/skills/furb.json"),
  include_str!("../extensions/skills/skills.py"),
)];

/// The start of each line that imports furb, which the word of a file leaves out.
const IMPORTS: [&str; 3] = ["from furb ", "from furb.", "import furb"];

/// An extension: its name, its word, and its life word, which is empty when it has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
  pub name: String,
  pub word: String,
  pub life: String,
}

impl Extension {
  /// The extension that a fact which enables one carries.
  fn of(a: &Fact) -> Option<Extension> {
    let text = |at| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
    Some(Extension { name: text(0)?, word: text(1)?, life: text(2)? })
  }

  /// The saying that enables the extension, about the root.
  fn enable(&self) -> Fact {
    let words = [&self.name, &self.word, &self.life].map(|one| Object::string(one.as_str()));
    Fact::says(ENABLE, ROOT, words)
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
  /// under `APPDATA` on Windows, or under `.config` in the home, which is where the TUI keeps its preferences.
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
  OFFICIAL.iter().map(|(manifest, word)| manifested(manifest, |_| Ok(word)).expect(whole)).collect()
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
  let field = |key: &str| match manifest.get(key) {
    None => Ok(None),
    Some(Value::String(one)) => Ok(Some(one.as_str())),
    Some(_) => Err(format!("The {key} of the manifest is no text")),
  };
  let (Some(name), Some(file)) = (field("name")?, field("word")?) else {
    return Err("A manifest gives a name and a word".to_owned());
  };
  let life = field("life")?.unwrap_or_default().to_owned();
  Ok(Extension { name: name.to_owned(), word: word(read(file)?.as_ref()), life })
}

/// The extensions that the configs of the user and of the project turn on, in the order a life runs them: each
/// official one unless a config turns it off, then each other one in the order a config first names it. A config
/// gives the folder of an extension that is not official by its path, which resolves against the folder of the
/// config, `~` being the home, and `true` keeps the path that an earlier config gave.
pub fn configured(places: &Places, project: &Path) -> Result<Vec<Extension>, Fault> {
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
  let on = named.into_iter().enumerate().filter(|(_, one)| one.2);
  on.map(|(at, (name, path, _))| {
    path.map_or_else(|| Ok(official[at].clone()), |root| load(&name, &root))
  })
  .collect()
}

/// A path that a config gives, resolved against the folder of the config, with `~` as the home.
fn beside(file: &Path, path: &str, places: &Places) -> PathBuf {
  match (path.strip_prefix('~'), &places.home) {
    (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
      home.join(rest.trim_start_matches(['/', '\\']))
    }
    _ => file.parent().unwrap_or(Path::new("")).join(path),
  }
}

/// The word of a file: the file with its line ends made LF, less each top level import of furb and the lines up to
/// the `)` of one that opens a `(`, with no empty line at its start or at its end, and no run of more than two
/// empty lines.
pub fn word(source: &str) -> String {
  let (mut out, mut empty, mut open) = (String::new(), 0, false);
  for line in source.split('\n').map(|one| one.strip_suffix('\r').unwrap_or(one)) {
    if open || IMPORTS.iter().any(|one| line.starts_with(one)) {
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

/// The ear of the extensions, which plays each extension that the life enables as a rung on each chain: on every
/// chain there is when the life enables it, and after that on each chain at its birth. The rung holds the word and
/// then the life word, and the life word alone on a chain whose origin had the extension, since that chain made the
/// rung of its origin again. At each stand that this life answers, the first of which is at the tip, the ear
/// enables each extension it is given that the life does not run yet, by a fact that the record keeps. A later
/// life says that fact again at its place, and the ear plays the same rungs there, from the fact alone.
pub fn extensions(given: Vec<Extension>) -> Box<dyn Ear> {
  ear(move |co, _| async move {
    // What the life enabled, in order, and each chain in the order of its birth, with the extensions it has.
    let (mut enabled, mut chains) = (Vec::<Extension>::new(), Vec::<(String, Vec<String>)>::new());
    loop {
      let a = hear(&co).await;
      let runs = |one: &Extension| enabled.iter().any(|two| two.name == one.name);
      let new: Vec<Extension> = match a.kind() {
        // A chain says its started at its birth, and it has what its origin had, which it made again.
        "started" if named(a.about(), "chain") => {
          let got = call(&co, "get", vec![Object::string(a.about())], vec![]).await?;
          let source = entry(&got.as_ref(), 5).and_then(|one| one.as_str().map(str::to_owned));
          let had =
            chains.iter().find(|one| Some(&one.0) == source.as_ref()).map(|one| one.1.clone());
          chains.push((a.about().to_owned(), had.unwrap_or_default()));
          let born = chains.len() - 1;
          for one in &enabled {
            play(&co, one, &mut chains[born]).await?;
          }
          continue;
        }
        "done" if named(a.about(), "stand") && a.by() != JOURNAL => {
          given.iter().filter(|one| !runs(one)).cloned().collect()
        }
        ENABLE => Extension::of(&a).filter(|one| !runs(one)).into_iter().collect(),
        _ => continue,
      };
      for one in new {
        if a.kind() != ENABLE {
          say(&co, one.enable()).await;
        }
        for chain in &mut chains {
          play(&co, &one, chain).await?;
        }
        enabled.push(one);
      }
    }
  })
}

/// One extension played on a chain as a rung: its word then its life word, or its life word alone on a chain that
/// has the extension already.
async fn play(
  co: &Co,
  one: &Extension,
  (chain, had): &mut (String, Vec<String>),
) -> Result<(), Fault> {
  let word = if had.contains(&one.name) {
    one.life.clone()
  } else {
    had.push(one.name.clone());
    format!("{}\n\n{}", one.word, one.life).trim_matches('\n').to_owned()
  };
  if !word.is_empty() {
    let on = vec![("on", Object::string(chain.as_str()))];
    call(co, "rung", vec![Object::string(word)], on).await?;
  }
  Ok(())
}

#[cfg(test)]
#[path = "extension.test.rs"]
mod test;
