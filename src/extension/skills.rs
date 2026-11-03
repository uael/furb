//! The ear of the skills extension, which answers a read of `skills://` from the folders of skills on the disk.

use std::{fs, path::PathBuf};

use crate::{
  ear::{Ear, hear, hearing, say},
  extension::frontmatter,
  fact::Fact,
  value::{Fault, Text},
  world::{files::read, here},
};

/// The scheme of the paths that the ear serves.
const SCHEME: &str = "skills://";

/// The skill that the extension carries, which says how a model extends furb, and which a skill of its name in a
/// folder replaces.
const EXTENDING: &str = include_str!("../../extensions/skills/extending/SKILL.md");

/// One skill that the ear found: its name, what it is for, and its SKILL.md file, which the skill that the extension
/// carries has not.
struct Skill {
  name: String,
  description: String,
  path: Option<PathBuf>,
}

/// The ear of the skills: it answers a read of `skills://` with the list of the skills of the chain, one line for
/// each, and a read of `skills://` and a name with the SKILL.md file of that skill, or with the refusal. It finds the
/// skills in `.furb/skills`, `.agents/skills` and `.claude/skills` of the working directory of the chain and of each
/// folder above it, nearest first, then in `skills` of the config directory of the user, and then the skill `extending` that the
/// extension carries.
pub fn skills(config: PathBuf) -> Box<dyn Ear> {
  hearing(&["read"], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() != "read" || !a.question() {
        continue;
      }
      let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      let Some(name) = path.strip_prefix(SCHEME) else { continue };
      let here = here(&mut co, a.on()).await?;
      let roots = here
        .ancestors()
        .flat_map(|one| [one.join(".furb"), one.join(".agents"), one.join(".claude")]);
      let found = found(roots.map(|one| one.join("skills")).chain([config.join("skills")]));
      let answer = match found.iter().find(|one| one.name == name) {
        _ if name.is_empty() => {
          let lines: Vec<_> =
            found.iter().map(|one| format!("{}: {}", one.name, one.description)).collect();
          Ok(Text::new(SCHEME, lines.join("\n")))
        }
        Some(Skill { path: Some(path), .. }) => read(path),
        Some(_) => Ok(Text::new(format!("{SCHEME}{name}"), EXTENDING)),
        None => Err(Fault::refused(format!("There is no skill {name}."))),
      };
      let answer = answer.map_or_else(|no| no.object(), |one| one.object());
      say(&mut co, Fact::says("done", a.about(), [answer]));
    }
  })
}

/// The skills in these folders, in the order of their names: each folder in them that holds a SKILL.md file that
/// the World reads as a text, under the name its frontmatter gives or else the name of the folder, and then the skill
/// that the extension carries, and a name that an earlier folder holds wins.
fn found(roots: impl Iterator<Item = PathBuf>) -> Vec<Skill> {
  let mut out: Vec<Skill> = Vec::new();
  for root in roots {
    let Ok(held) = fs::read_dir(&root) else { continue };
    let mut folders = held.filter_map(|one| one.ok().map(|one| one.path())).collect::<Vec<_>>();
    folders.sort();
    for folder in folders {
      let path = folder.join("SKILL.md");
      let Ok(text) = read(&path) else { continue };
      let front = frontmatter(&text.content);
      let name = front.field("name").map_or_else(
        || folder.file_name().map(|one| one.to_string_lossy().into()).unwrap_or_default(),
        str::to_owned,
      );
      if !out.iter().any(|one| one.name == name) {
        let description = front.field("description").unwrap_or_default().to_owned();
        out.push(Skill { name, description, path: Some(path) });
      }
    }
  }
  let front = frontmatter(EXTENDING);
  let name = front.field("name").unwrap_or_default().to_owned();
  if !out.iter().any(|one| one.name == name) {
    let description = front.field("description").unwrap_or_default().to_owned();
    out.push(Skill { name, description, path: None });
  }
  out.sort_by(|one, two| one.name.cmp(&two.name));
  out
}
