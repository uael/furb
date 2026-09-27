//! The ear of the memory extension, which answers a memory question from the CLAUDE.md and AGENTS.md files on the
//! disk.

use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use crate::{
  ear::{Ear, call, ear, hear, say},
  fact::{Fact, named},
  value::{Object, Text},
  world::{files::read, here, resolved},
};

/// The files that hold the memory of a folder: the first of them that the folder holds.
const NAMES: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

/// The ear of the memory: it answers a memory question with the texts of the memory files that apply to its path
/// and that its chain does not hold as they stand, which are the memory of the config directory of the user, then
/// of each folder from the root down to the working directory of the chain, then of each folder under it down to the
/// folder of the path. What each chain holds it reads off the facts alone, so it holds the same in every life: the
/// texts of the answer of each memory question, the text of each read and each write of a memory file, and what the
/// prefix of a chain holds of them.
pub fn memory(config: PathBuf) -> Box<dyn Ear> {
  ear(move |co, _| async move {
    let mut held: HashMap<String, HashMap<String, String>> = HashMap::new();
    loop {
      let a = hear(&co).await;
      match a.kind() {
        "memory" if a.question() => {
          let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
          let here = here(&co, a.on()).await?;
          // A path of a scheme adds no folder of its own.
          let target = if path.contains("://") { here.clone() } else { resolved(&here, &path) };
          let chain = held.entry(a.on().to_owned()).or_default();
          let mut told = Vec::new();
          for one in
            [config.clone()].into_iter().chain(folders(&here, &target)).filter_map(|one| of(&one))
          {
            if chain.get(&one.path) != Some(&one.content) {
              chain.insert(one.path.clone(), one.content.clone());
              told.push(one.object());
            }
          }
          say(&co, Fact::says("done", a.about(), [Object::list(told)])).await;
        }
        "done" => {
          let texts = kept(&a);
          if !texts.is_empty() {
            let chain = call(&co, "scope", vec![Object::string(a.about())], vec![]).await?;
            let chain =
              held.entry(chain.as_ref().as_str().unwrap_or_default().to_owned()).or_default();
            chain.extend(texts.into_iter().map(|one| (one.path, one.content)));
          }
        }
        "prefix" => {
          let facts = a.word(0).and_then(|one| one.items()).unwrap_or_default();
          let dones = facts.into_iter().filter_map(Fact::of).filter(|one| one.kind() == "done");
          let chain = held.entry(a.about().to_owned()).or_default();
          chain.extend(dones.flat_map(|one| kept(&one)).map(|one| (one.path, one.content)));
        }
        _ => {}
      }
    }
  })
}

/// The texts of memory files that a done holds: the answer of a memory question, and the text of a read or of a
/// write when it is a memory file.
fn kept(a: &Fact) -> Vec<Text> {
  let value = a.word(0);
  if named(a.about(), "memory") {
    return value
      .and_then(|one| one.items())
      .unwrap_or_default()
      .into_iter()
      .filter_map(Text::of)
      .collect();
  }
  let memory = |one: &Text| {
    Path::new(&one.path).file_name().is_some_and(|name| NAMES.iter().any(|two| name == *two))
  };
  let done = named(a.about(), "read") || named(a.about(), "write");
  value.and_then(Text::of).filter(|one| done && memory(one)).into_iter().collect()
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
/// it as a text.
fn of(folder: &Path) -> Option<Text> {
  read(&NAMES.iter().map(|one| folder.join(one)).find(|one| one.is_file())?).ok()
}
