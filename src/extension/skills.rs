//! The ear of the skills, which answers a read of `skills://` with one finder for each skill, with
//! code or with none: the folders of skills on the disk, the extensions of the project and of the
//! configs, and the official extensions that the crate carries. It answers the act `skill`, which
//! loads a skill on a chain, and a write of `skills://` and a name, which writes the word of a
//! skill, and a read of `config://`, which gives the keys of a skill in the configs, so its word
//! reads them. [`process`] gives the program of the process of a skill, which a daemon runs.

use std::{
  fs,
  path::{Component, Path, PathBuf},
};

use serde_json::{Map, Value};

use crate::{
  ear::{Co, Ear, hear, hearing, say},
  engine::ROOT,
  extension::{
    CONFIG, DEFAULT, ENABLE, Extension, INTERFACE, MANIFEST, PROCESS, Places, WORD, copy, enabled,
    frontmatter, imports_furb, load, manifested, manifests, shipped, turned_on, word,
  },
  fact::Fact,
  value::{Fault, Object, Text, entry},
  world::{
    config::{self, Layers},
    files::{self, read},
    here,
  },
};

/// The scheme of the paths that the ear serves.
const SCHEME: &str = "skills://";

/// The scheme of the values of the configs, which a word reads: `config://` gives each key with
/// the value that wins for it, and `config://` and a name the value of that key, as the object of
/// the keys of a skill.
const CONFIGS: &str = "config://";

/// The first line of the index, which says how a chain loads and reads a skill.
const USE: &str = "skill(name) loads a skill: a skill with code plays its word and tells its \
                   interface, and any other tells its SKILL.md. skill() plays each default skill. \
                   read(\"skills://<name>\") gives the word of a skill with code, and \
                   read(\"skills://<name>/<file>\") a file of its folder. \
                   read(\"config://<name>\") gives the keys of a skill in the configs, as JSON.";

/// Where the files of a skill stand: a folder on the disk, the files that the crate carries of an
/// official extension.
enum Place {
  Folder(PathBuf),
  Shipped(Vec<(String, &'static str)>),
}

/// One skill that the ear found: its name, what it is for, the file of its word when it has code,
/// whether it plays by default, the text of its manifest, and where its files stand.
struct Skill {
  name: String,
  description: String,
  word: Option<String>,
  default: bool,
  manifest: String,
  place: Place,
}

impl Skill {
  /// The skill that the text of a manifest gives, under the name of its folder when the frontmatter
  /// gives none.
  fn of(text: &str, folder: &str, place: Place) -> Skill {
    let front = frontmatter(text);
    let name = front.field("name").unwrap_or(folder).to_owned();
    let description = front.field("description").unwrap_or_default().to_owned();
    let word = front.meta(WORD).map(str::to_owned);
    let default = front.meta(DEFAULT) == Some("true");
    Skill { name, description, word, default, manifest: text.to_owned(), place }
  }

  /// The line of the index of the skill: its name, the marks that hold of code, on and default,
  /// and what it is for.
  fn line(&self, on: bool) -> String {
    let marks: Vec<&str> = [(self.word.is_some(), "code"), (on, "on"), (self.default, "default")]
      .into_iter()
      .filter_map(|(holds, mark)| holds.then_some(mark))
      .collect();
    let marked = if marks.is_empty() { String::new() } else { format!(" ({})", marks.join(", ")) };
    format!("{}{marked}: {}", self.name, self.description)
  }

  /// The extension of a skill with code, as its manifest and the files of its folder give it, with
  /// the copy of its word in the config directory of these places.
  fn extension(&self, places: &Places) -> Result<Extension, Fault> {
    let read = |file: &str| filed(self, file, "").map(|got| got.content).map_err(|no| no.message());
    let mut one = manifested(&self.manifest, read).map_err(Fault::refused)?;
    one.file = copy(&one.name, &one.file, &one.word, places);
    Ok(one)
  }

  /// The program of the process of the skill, as the fact enable names it: under `~` when it
  /// stands in the home of these places, and empty when the skill names none.
  fn program(&self, places: &Places) -> String {
    let Place::Folder(folder) = &self.place else { return String::new() };
    programmed(folder, &self.manifest).map(|at| tilded(&at, places)).unwrap_or_default()
  }
}

/// What the ear holds to answer one act: the chain that asked, the skills that it finds, the
/// extensions that the life runs, and the places of the user and of the project.
struct Seen {
  chain: String,
  found: Vec<Skill>,
  runs: Vec<Extension>,
  places: Places,
  project: PathBuf,
}

impl Seen {
  /// Whether the life runs a skill on the chain.
  fn on(&self, one: &Skill) -> bool {
    self.runs.iter().any(|two| two.name == one.name && (!two.root || self.chain == ROOT))
  }

  /// The skill of a name, or the refusal that names it.
  fn named(&self, name: &str) -> Result<&Skill, Fault> {
    let found = self.found.iter().find(|one| one.name == name);
    found.ok_or_else(|| Fault::refused(format!("There is no skill {name}.")))
  }
}

/// The ear of the skills. It answers a read of `skills://` with the index of the skills of the
/// chain, a read of `skills://` and a name with the word of a skill with code, which the life last
/// enabled or else its file holds, or with the SKILL.md file of a skill with none, and a read of
/// `skills://`, a name, a slash and a file with that file of the folder of the skill. It answers
/// the act `skill` of a name with the prose and the interface of a skill with code, or its code
/// when it ships no interface, after it says the fact enable the first time that the life plays
/// that word, and with the SKILL.md file of a skill with none. The act with no name plays each
/// default skill that the life does not run. A write of `skills://` and a name writes the word of a
/// skill with code in `.furb/extensions` of the project, and says the fact enable again when the
/// life runs it, which reloads it. A read of `config://` gives the values of the configs, as
/// [`configs`] says. It finds the skills in `.furb/skills`, `.agents/skills` and
/// `.claude/skills` of the working directory of the chain and of each folder above it, nearest
/// first, then in `skills` of the config directory of the user, then in the folders of the
/// extensions of the project in `.furb/extensions` and of each folder that a config turns on, then
/// in the official extensions.
pub fn skills(config: PathBuf) -> Box<dyn Ear> {
  hearing(&["read", "write", "skill"], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if !a.question() || !matches!(a.kind(), "read" | "write" | "skill") {
        continue;
      }
      let path = match a.kind() {
        "read" => a.word(1).and_then(|one| one.as_str().map(str::to_owned)),
        "write" => a.word(1).and_then(Text::of).map(|one| one.path),
        _ => Some(SCHEME.to_owned()),
      };
      let configured =
        a.kind() == "read" && path.as_deref().is_some_and(|one| one.starts_with(CONFIGS));
      let scheme = if configured { CONFIGS } else { SCHEME };
      let Some(asked) = path.as_deref().and_then(|one| one.strip_prefix(scheme)) else { continue };
      let asked = asked.to_owned();
      let here = here(&mut co, a.on()).await?;
      let project = project(&mut co).await?.unwrap_or_else(|| here.clone());
      let places = Places { config: config.clone(), home: std::env::home_dir() };
      let found = found(&here, &project, &places);
      let runs = enables(&mut co).await?;
      let seen = Seen { chain: a.on().to_owned(), found, runs, places, project };
      let answer = match a.kind() {
        "read" if configured => configs(&seen, &asked),
        "read" => answered(&seen, &asked),
        "skill" => {
          let name = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
          loaded(&mut co, &seen, &name)
        }
        _ => {
          let content = a.word(1).and_then(Text::of).map(|one| one.content).unwrap_or_default();
          written(&mut co, &seen, &asked, &content)
        }
      };
      let answer = answer.map_or_else(|no| no.object(), |one| one.object());
      say(&mut co, Fact::says("done", a.about(), [answer]));
    }
  })
}

/// The answer of a read of `skills://` and what follows it: the index, the word or the manifest of
/// a skill, or a file of its folder.
fn answered(seen: &Seen, asked: &str) -> Result<Text, Fault> {
  let path = format!("{SCHEME}{asked}");
  if asked.is_empty() {
    let lines = seen.found.iter().map(|one| one.line(seen.on(one)));
    return Ok(Text::new(
      SCHEME,
      [USE.to_owned()].into_iter().chain(lines).collect::<Vec<_>>().join("\n"),
    ));
  }
  let (name, file) = asked.split_once('/').map_or((asked, None), |(one, two)| (one, Some(two)));
  let one = seen.named(name)?;
  match (file, &one.place) {
    (Some(file), _) => filed(one, file, &path),
    (None, _) if one.word.is_some() => worded(one, &seen.runs, &path),
    (None, Place::Folder(folder)) => read(&folder.join(MANIFEST)),
    (None, _) => Ok(Text::new(path, one.manifest.clone())),
  }
}

/// The answer of a read of `config://` and what follows it: the value of each key of the
/// configs as one JSON object, or the value of the key of a name. The keys of a skill whose
/// manifest names a file of keys under `furb-config` come with the default of each key that no
/// config gives, whether the configs turn the skill on or a folder of skills holds it, and a skill
/// whose keys no config gives comes as an empty object. Any other name that no config gives and no
/// schema names is refused.
fn configs(seen: &Seen, asked: &str) -> Result<Text, Fault> {
  let path = format!("{CONFIGS}{asked}");
  let refused = |no: config::Error| Fault::refused(no.to_string());
  let mut values =
    Layers::new(&seen.project, Some(seen.places.config.clone())).values().map_err(refused)?;
  if asked.is_empty() {
    return Ok(Text::new(path, json(&Value::Object(values))));
  }
  let mut value = values.remove(asked);
  if let Some(one) = seen.found.iter().find(|one| one.name == asked) {
    let within = value.get_or_insert_with(|| Value::Object(Map::new()));
    if let Some(file) = frontmatter(&one.manifest).meta(CONFIG)
      && let Value::Object(within) = within
    {
      let text = filed(one, file, &path)?.content;
      for (key, default) in config::defaults(asked, &text).map_err(refused)? {
        within.entry(key).or_insert(default);
      }
    }
  }
  let value = value
    .ok_or_else(|| Fault::refused(format!("No config gives {path}, and no schema names it.")))?;
  Ok(Text::new(path, json(&value)))
}

/// A value as the JSON text that a word reads, one key on each line.
fn json(value: &Value) -> String {
  serde_json::to_string_pretty(value).unwrap_or_default()
}

/// The answer of the act `skill`: the prose and the interface or the code of a skill with code,
/// once the fact enable plays its word, the SKILL.md file of a skill with none, or, with no name,
/// the line of each default skill, once each that the life does not run plays.
fn loaded(co: &mut Co, seen: &Seen, name: &str) -> Result<Text, Fault> {
  if name.is_empty() {
    let defaults: Vec<&Skill> =
      seen.found.iter().filter(|one| one.default && one.word.is_some()).collect();
    for one in &defaults {
      if !seen.runs.iter().any(|two| two.name == one.name) {
        say(co, one.extension(&seen.places)?.played(&one.program(&seen.places)));
      }
    }
    let lines = defaults.iter().map(|one| one.line(true)).collect::<Vec<_>>().join("\n");
    return Ok(Text::new(SCHEME, lines));
  }
  let one = seen.named(name)?;
  let path = format!("{SCHEME}{name}");
  if one.word.is_none() {
    return answered(seen, name);
  }
  let extension = one.extension(&seen.places)?;
  if !seen.runs.iter().any(|two| two.name == name && two.word == extension.word) {
    say(co, extension.played(&one.program(&seen.places)));
  }
  let shown = match frontmatter(&one.manifest).meta(INTERFACE) {
    Some(file) => filed(one, file, &path)?.content,
    None => extension.word,
  };
  let prose = body(&one.manifest).trim().to_owned();
  Ok(Text::new(
    path,
    [prose, shown].into_iter().filter(|one| !one.is_empty()).collect::<Vec<_>>().join("\n\n"),
  ))
}

/// The answer of a write of `skills://` and a name: the word of a skill with code written in the
/// folder of the skill in `.furb/extensions` of the project, after a copy of its folder there when
/// it stands elsewhere, with the imports of furb that the top of its file holds. The fact enable
/// with the new word reloads the skill when the life runs it. A word that the skill refuses, as one
/// that binds a name of the engine at its top, changes no file, and no copy of the folder stands.
fn written(co: &mut Co, seen: &Seen, name: &str, content: &str) -> Result<Text, Fault> {
  let one = seen.named(name)?;
  let Some(file) = one.word.as_deref() else {
    return Err(Fault::refused(format!("The skill {name} has no code to write.")));
  };
  let given = content.trim_matches('\n');
  manifested(&one.manifest, |_| Ok(given)).map_err(Fault::refused)?;
  let target = seen.project.join(".furb").join("extensions").join(name);
  let stays = matches!(&one.place, Place::Folder(folder) if *folder == target);
  if !stays {
    copied(one, &target)?;
  }
  let at = target.join(file);
  let before = fs::read_to_string(&at).unwrap_or_default();
  let imports: Vec<&str> = before
    .lines()
    .take_while(|line| line.is_empty() || imports_furb(line) || line.starts_with([' ', ')']))
    .collect();
  let imports = imports.join("\n").trim_end().to_owned();
  let whole =
    if imports.is_empty() { format!("{given}\n") } else { format!("{imports}\n\n\n{given}\n") };
  fs::write(&at, whole).map_err(|no| Fault::refused(format!("{no}: {}", at.display())))?;
  if seen.runs.iter().any(|two| two.name == name) {
    let program = programmed(&target, &one.manifest).map(|at| tilded(&at, &seen.places));
    say(co, load(name, &target, &seen.places)?.played(&program.unwrap_or_default()));
  }
  Ok(Text::new(format!("{SCHEME}{name}"), given))
}

/// Each file of the folder of a skill, and the files that the crate carries of an official one,
/// copied in a folder.
fn copied(one: &Skill, target: &Path) -> Result<(), Fault> {
  let failed = |no: std::io::Error| Fault::refused(format!("{no}: {}", target.display()));
  fs::create_dir_all(target).map_err(failed)?;
  match &one.place {
    Place::Folder(folder) => {
      for file in fs::read_dir(folder).map_err(failed)?.filter_map(Result::ok) {
        if file.path().is_file() {
          fs::copy(file.path(), target.join(file.file_name())).map_err(failed)?;
        }
      }
    }
    Place::Shipped(files) => {
      for (file, text) in files {
        fs::write(target.join(file), text).map_err(failed)?;
      }
    }
  }
  Ok(())
}

/// The body of a manifest: its text after its frontmatter.
fn body(text: &str) -> &str {
  let Some(rest) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) else {
    return text;
  };
  rest.split_once("\n---").map_or("", |(_, after)| after.split_once('\n').map_or("", |two| two.1))
}

/// The folder of the project that the standing names, or none while it names none.
async fn project(co: &mut Co) -> Result<Option<PathBuf>, Fault> {
  let got = co.call("standing", vec![], vec![]).await?;
  let named = entry(&got.as_ref(), 1).and_then(|one| one.as_str().map(str::to_owned));
  Ok(named.and_then(|one| files::placed(Path::new(""), &one).ok()))
}

/// The extensions that the life runs, as the enable facts of the root say them.
async fn enables(co: &mut Co) -> Result<Vec<Extension>, Fault> {
  let of = [ROOT, ENABLE].map(Object::string).to_vec();
  let got = co.call("transcript", of, vec![]).await?;
  let facts: Vec<Fact> =
    (0..).map_while(|at| entry(&got.as_ref(), at)).filter_map(Fact::of).collect();
  Ok(enabled(&facts))
}

/// The word of a skill with code: the word that the life last enabled for it, or else the word of
/// its file.
fn worded(one: &Skill, runs: &[Extension], path: &str) -> Result<Text, Fault> {
  if let Some(two) = runs.iter().rev().find(|two| two.name == one.name) {
    return Ok(Text::new(path, two.word.clone()));
  }
  let file = one.word.as_deref().unwrap_or_default();
  Ok(Text::new(path, word(&filed(one, file, path)?.content)))
}

/// A file of the folder of a skill, at the path that the chain read, and a refusal for a file that
/// leaves the folder or that the folder does not hold.
fn filed(one: &Skill, file: &str, path: &str) -> Result<Text, Fault> {
  let missing = || Fault::refused(format!("The skill {} holds no file {file}.", one.name));
  if !Path::new(file).components().all(|part| matches!(part, Component::Normal(_))) {
    return Err(missing());
  }
  match &one.place {
    Place::Folder(folder) => read(&folder.join(file)).map(|got| Text::new(path, got.content)),
    Place::Shipped(files) => {
      let text = files.iter().find(|two| two.0 == file).map(|two| two.1).ok_or_else(missing)?;
      Ok(Text::new(path, text))
    }
  }
}

/// The program of the process of the skill of a name, as a life whose folder is `here` finds the
/// skill: the file of its folder that its manifest names under `metadata` as `furb-process`. It is
/// none when the manifest names none, when the file leaves the folder, and when the skill is
/// official, since the crate carries no process yet.
/// A fact enable of a build before the program names none, so a life finds the process of such a
/// fact by the name of its skill, as it finds it now.
pub fn process(name: &str, here: &Path, places: &Places) -> Option<PathBuf> {
  let one = found(here, here, places).into_iter().find(|one| one.name == name)?;
  let Place::Folder(folder) = &one.place else { return None };
  programmed(folder, &one.manifest)
}

/// The program that a fact enable names, as a path of the machine, or none when it names none.
pub fn program(named: &str) -> Option<PathBuf> {
  (!named.is_empty()).then(|| files::placed(Path::new(""), named).ok()).flatten()
}

/// The program of the process that the manifest of a skill in a folder names under `metadata` as
/// `furb-process`, unless the file leaves the folder.
fn programmed(folder: &Path, manifest: &str) -> Option<PathBuf> {
  let file = frontmatter(manifest).meta(PROCESS)?.to_owned();
  let inside = Path::new(&file).components().all(|part| matches!(part, Component::Normal(_)));
  inside.then(|| folder.join(file))
}

/// A path of the machine under `~` when it stands in the home of these places.
fn tilded(path: &Path, places: &Places) -> String {
  files::tilded(path, places.home.as_deref())
}

/// The skills of a chain that stands in `here`, in the order of their names: each folder that holds
/// a SKILL.md file in the folders of skills, then the extensions of the project and of the configs,
/// then the official ones. A name that an earlier
/// place holds wins.
fn found(here: &Path, project: &Path, places: &Places) -> Vec<Skill> {
  let roots =
    here.ancestors().flat_map(|one| [one.join(".furb"), one.join(".agents"), one.join(".claude")]);
  let roots = roots.map(|one| one.join("skills")).chain([places.config.join("skills")]);
  let mut folders: Vec<PathBuf> = roots.flat_map(|root| inside(&root)).collect();
  folders.extend(inside(&project.join(".furb").join("extensions")));
  let configured = turned_on(places, project).unwrap_or_default().into_iter();
  folders.extend(configured.filter_map(|(_, root)| root));
  let mut out: Vec<Skill> = Vec::new();
  let mut keep = |one: Skill| {
    if !out.iter().any(|two| two.name == one.name) {
      out.push(one);
    }
  };
  for folder in folders {
    let Ok(text) = read(&folder.join(MANIFEST)) else { continue };
    let name = folder.file_name().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
    keep(Skill::of(&text.content, &name, Place::Folder(folder)));
  }
  for manifest in manifests() {
    let name = frontmatter(manifest).field("name").unwrap_or_default().to_owned();
    keep(Skill::of(manifest, &name, Place::Shipped(shipped(&name).unwrap_or_default())));
  }
  out.sort_by(|one, two| one.name.cmp(&two.name));
  out
}

/// The folders in a folder, in the order of their names, or none when it cannot be read.
fn inside(root: &Path) -> Vec<PathBuf> {
  let Ok(held) = fs::read_dir(root) else { return Vec::new() };
  let mut folders: Vec<PathBuf> = held.filter_map(|one| one.ok().map(|one| one.path())).collect();
  folders.sort();
  folders
}
