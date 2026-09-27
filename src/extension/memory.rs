//! The ear of the memory extension, which answers a memory question from the CLAUDE.md and AGENTS.md files on the
//! disk.

use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
};

use crate::{
  ear::{Co, Ear, call, ear, hear, say},
  fact::{Fact, named},
  value::{Fault, Object, ObjectRef, Text},
  world::{files::read, here, resolved},
};

/// The files that hold the memory of a folder: the first of them that the folder holds.
const NAMES: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

/// The ear of the memory: it answers a memory question with the texts of the memory files that apply to its path
/// and that its chain does not hold as they stand, which are the memory of the config directory of the user, then
/// of each folder from the root down to the working directory of the chain, then of each folder under it down to the
/// folder of the path. What each chain holds it reads off the facts alone, so it holds the same in every life: the
/// texts of the answer of each memory question, the text of each read and each write of a memory file, and what the
/// prefix of a chain holds of them, each when an act that tells made the question, since nothing else is told.
pub fn memory(config: PathBuf) -> Box<dyn Ear> {
  ear(move |co, _| async move {
    // What each chain holds of the memory files, by path, and each question of one that an act that tells made and
    // whose done the ear has not heard yet, with its chain.
    let mut held: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut unheard: Vec<(String, String)> = Vec::new();
    loop {
      let a = hear(&co).await;
      let (about, on) = (a.about().to_owned(), a.on().to_owned());
      match a.kind() {
        "memory" | "read" | "write" if a.question() => {
          let (asks, path) = (a.kind() == "memory", a.word(1).map(named_path).unwrap_or_default());
          if (asks || remembered(&path)) && tells(&co, a.by()).await? {
            unheard.push((about.clone(), on.clone()));
          }
          if !asks {
            continue;
          }
          // A done said while the ear was not hearing stands in the life already, so the ear reads it now, and holds it
          // when it hears it.
          let early =
            unheard.iter().filter(|one| one.1 == on && one.0 != about).map(|one| one.0.clone());
          for one in early.collect::<Vec<_>>() {
            let got = call(&co, "peek", vec![Object::string(&one)], vec![]).await?;
            held.entry(on.clone()).or_default().extend(given(&one, Some(got.as_ref())));
          }
          let here = here(&co, &on).await?;
          // A path of a scheme adds no folder of its own.
          let target = if path.contains("://") { here.clone() } else { resolved(&here, &path) };
          let chain = held.get(&on);
          let mut told: Vec<Text> = Vec::new();
          for one in
            [config.clone()].into_iter().chain(folders(&here, &target)).filter_map(|one| of(&one))
          {
            let fresh = chain.and_then(|chain| chain.get(&one.path)) != Some(&one.content);
            if fresh && !told.iter().any(|two| two.path == one.path) {
              told.push(one);
            }
          }
          say(&co, Fact::says("done", &about, [Object::list(told.iter().map(Text::object))])).await;
        }
        "done" => {
          let got = given(&about, a.word(0));
          if got.is_empty() {
            continue;
          }
          let known = unheard.extract_if(.., |one| one.0 == about).next().map(|one| one.1);
          if let Some(chain) = known.or(heard(&co, &about).await?) {
            held.entry(chain).or_default().extend(got);
          }
        }
        "prefix" => {
          let facts = a.word(0).and_then(|one| one.items()).unwrap_or_default();
          for one in facts.into_iter().filter_map(Fact::of).filter(|one| one.kind() == "done") {
            let got = given(one.about(), one.word(0));
            if !got.is_empty() && heard(&co, one.about()).await?.is_some() {
              held.entry(about.clone()).or_default().extend(got);
            }
          }
        }
        _ => {}
      }
    }
  })
}

/// Whether an act made something where it tells, since nothing is told from outside an act.
async fn tells(co: &Co, maker: &str) -> Result<bool, Fault> {
  let made = call(co, "get", vec![Object::string(maker)], vec![]).await?;
  if made.as_ref().type_name() == "NoneType" {
    return Ok(false);
  }
  Ok(call(co, "tells", vec![Object::string(maker)], vec![]).await?.as_ref().as_bool() == Some(true))
}

/// The chain of a question, when an act that tells made it, so the chain heard what it came to.
async fn heard(co: &Co, question: &str) -> Result<Option<String>, Fault> {
  let made = call(co, "get", vec![Object::string(question)], vec![]).await?;
  let Some(made) = Fact::of(made.as_ref()) else { return Ok(None) };
  Ok(tells(co, made.by()).await?.then(|| made.on().to_owned()))
}

/// The path and the content of each memory file that the done of a question gives: the answer of a memory question,
/// and the text of a read or of a write, when it is a memory file.
fn given(question: &str, value: Option<ObjectRef<'_>>) -> Vec<(String, String)> {
  let texts: Vec<Text> = match value {
    Some(one) if named(question, "memory") => {
      one.items().unwrap_or_default().into_iter().filter_map(Text::of).collect()
    }
    Some(one) if named(question, "read") || named(question, "write") => {
      Text::of(one).into_iter().collect()
    }
    _ => Vec::new(),
  };
  texts.into_iter().filter(|one| remembered(&one.path)).map(|one| (one.path, one.content)).collect()
}

/// The path a question names: the path of a read or of a memory question, and the path of the text of a write.
fn named_path(word: ObjectRef<'_>) -> String {
  Text::of(word).map_or_else(|| word.as_str().unwrap_or_default().to_owned(), |text| text.path)
}

/// Whether a path names a memory file, by the name of its file.
fn remembered(path: &str) -> bool {
  Path::new(path).file_name().is_some_and(|name| NAMES.iter().any(|one| name == *one))
}

/// The folders whose memory applies to a path: each folder from the root down to the working directory, then each
/// folder under it down to the folder of the path, which is the path when it is a folder, and else its parent.
fn folders(here: &Path, target: &Path) -> Vec<PathBuf> {
  let mut out: Vec<PathBuf> = here.ancestors().map(Path::to_path_buf).collect();
  out.reverse();
  let folder = if target.is_dir() { target } else { target.parent().unwrap_or(target) };
  let mut at = here.to_path_buf();
  for part in folder.strip_prefix(here).into_iter().flat_map(Path::components) {
    at.push(part);
    out.push(at.clone());
  }
  out
}

/// The memory of a folder: its CLAUDE.md file, or its AGENTS.md file when it holds no CLAUDE.md, when the World reads
/// it as a text. A file holds the memory under that name exactly, on a disk that ignores the case of a name too, so a
/// folder has the same memory on every machine.
fn of(folder: &Path) -> Option<Text> {
  let exact = |path: &Path| {
    let held = fs::read_dir(folder).into_iter().flatten().flatten();
    held.map(|one| one.file_name()).any(|name| Some(name.as_os_str()) == path.file_name())
  };
  read(&NAMES.iter().map(|one| folder.join(one)).find(|one| one.is_file() && exact(one))?).ok()
}
