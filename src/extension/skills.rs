//! The ear of the skills, which answers a read of `skills://` with one finder for each skill, with
//! code or with none: the folders of skills on the disk, the extensions of the project and of the
//! configs, and the official extensions that the crate carries.

use std::{
  fs,
  path::{Component, Path, PathBuf},
};

use crate::{
  ear::{Co, Ear, hear, hearing, say},
  engine::ROOT,
  extension::{
    DEFAULT, ENABLE, Extension, MANIFEST, Places, WORD, enabled, frontmatter, manifests, shipped,
    turned_on, word,
  },
  fact::Fact,
  value::{Fault, Object, Text, entry},
  world::{
    files::{self, read},
    here,
  },
};

/// The scheme of the paths that the ear serves.
const SCHEME: &str = "skills://";

/// The first line of the index, which says how a chain reads a skill.
const USE: &str = "skill(name) reads a skill: the word of a skill with code, else its SKILL.md. \
                   read(\"skills://<name>/<file>\") reads a file of its folder.";

/// The skill that the extension carries, which says how a model extends furb, and which a skill of its name in a
/// folder replaces.
const EXTENDING: &str = include_str!("../../extensions/skills/extending/SKILL.md");

/// Where the files of a skill stand: a folder on the disk, the files that the crate carries of an
/// official extension, or the one text of the skill that the extension carries.
enum Place {
  Folder(PathBuf),
  Shipped(Vec<(String, &'static str)>),
  Carried,
}

/// One skill that the ear found: its name, what it is for, the file of its word when it has code,
/// whether it plays by default, and where its files stand.
struct Skill {
  name: String,
  description: String,
  word: Option<String>,
  default: bool,
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
    Skill { name, description, word, default: front.meta(DEFAULT) == Some("true"), place }
  }

  /// The line of the index of the skill: its name, the marks that hold of code, on and default, and
  /// what it is for.
  fn line(&self, on: bool) -> String {
    let marks: Vec<&str> = [(self.word.is_some(), "code"), (on, "on"), (self.default, "default")]
      .into_iter()
      .filter_map(|(holds, mark)| holds.then_some(mark))
      .collect();
    let marked = if marks.is_empty() { String::new() } else { format!(" ({})", marks.join(", ")) };
    format!("{}{marked}: {}", self.name, self.description)
  }
}

/// The ear of the skills: it answers a read of `skills://` with the index of the skills of the
/// chain, a read of `skills://` and a name with the word of a skill with code, which the life last
/// enabled or else its file holds, or with the SKILL.md file of a skill with none, and a read of
/// `skills://`, a name, a slash and a file with that file of the folder of the skill, or with the
/// refusal. It finds the skills in
/// `.furb/skills`, `.agents/skills` and `.claude/skills` of the working directory of the chain and
/// of each folder above it, nearest first, then in
/// `skills` of the config directory of the user, then in the folders of the extensions of the
/// project in
/// `.furb/extensions` and of each folder that a config turns on, then in the official extensions,
/// and then the skill
/// `extending` that the extension carries.
pub fn skills(config: PathBuf) -> Box<dyn Ear> {
  hearing(&["read"], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() != "read" || !a.question() {
        continue;
      }
      let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      let Some(asked) = path.strip_prefix(SCHEME) else { continue };
      let here = here(&mut co, a.on()).await?;
      let project = project(&mut co).await?.unwrap_or_else(|| here.clone());
      let places = Places { config: config.clone(), home: std::env::home_dir() };
      let found = found(&here, &project, &places);
      let runs = enables(&mut co).await?;
      let on =
        |one: &Skill| runs.iter().any(|two| two.name == one.name && (!two.root || a.on() == ROOT));
      let (name, file) = asked.split_once('/').map_or((asked, None), |(one, two)| (one, Some(two)));
      let answer = match (found.iter().find(|one| one.name == name), file) {
        _ if asked.is_empty() => {
          let lines = found.iter().map(|one| one.line(on(one)));
          Ok(Text::new(
            SCHEME,
            [USE.to_owned()].into_iter().chain(lines).collect::<Vec<_>>().join("\n"),
          ))
        }
        (Some(one), Some(file)) => filed(one, file, &path),
        (Some(one @ Skill { word: Some(_), .. }), None) => worded(one, &runs, &path),
        (Some(Skill { place: Place::Folder(folder), .. }), None) => read(&folder.join(MANIFEST)),
        (Some(_), None) => Ok(Text::new(path.clone(), EXTENDING)),
        (None, _) => Err(Fault::refused(format!("There is no skill {name}."))),
      };
      let answer = answer.map_or_else(|no| no.object(), |one| one.object());
      say(&mut co, Fact::says("done", a.about(), [answer]));
    }
  })
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
    Place::Carried if file == MANIFEST => Ok(Text::new(path, EXTENDING)),
    Place::Carried => Err(missing()),
  }
}

/// The skills of a chain that stands in `here`, in the order of their names: each folder that holds
/// a SKILL.md file in the folders of skills, then the extensions of the project and of the configs,
/// then the official ones, and then the skill that the extension carries. A name that an earlier
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
  keep(Skill::of(EXTENDING, "", Place::Carried));
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
