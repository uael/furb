//! The ear of the skills extension, which answers a read of `skills://` from the folders of skills on the disk.

use std::{fs, path::PathBuf};

use crate::{
  ear::{Ear, ear, hear, say},
  fact::Fact,
  value::{Fault, Text},
  world::{files::read, here},
};

/// The scheme of the paths that the ear serves.
const SCHEME: &str = "skills://";

/// The marks that open a block of lines as the value of a key of a frontmatter.
const BLOCKS: [&str; 6] = ["|", "|-", "|+", ">", ">-", ">+"];

/// One skill that the ear found: its name, what it is for, and its SKILL.md file.
struct Skill {
  name: String,
  description: String,
  path: PathBuf,
}

/// The ear of the skills: it answers a read of `skills://` with the list of the skills of the chain, one line for
/// each, and a read of `skills://` and a name with the SKILL.md file of that skill, or with the refusal. It finds the
/// skills in `.furb/skills` and `.claude/skills` of the working directory of the chain and of each folder above it,
/// nearest first, and then in `skills` of the config directory of the user.
pub fn skills(config: PathBuf) -> Box<dyn Ear> {
  ear(move |co, _| async move {
    loop {
      let a = hear(&co).await;
      if a.kind() != "read" || !a.question() {
        continue;
      }
      let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      let Some(name) = path.strip_prefix(SCHEME) else { continue };
      let here = here(&co, a.on()).await?;
      let roots = here.ancestors().flat_map(|one| [one.join(".furb"), one.join(".claude")]);
      let found = found(roots.map(|one| one.join("skills")).chain([config.join("skills")]));
      let answer = match found.iter().find(|one| one.name == name) {
        _ if name.is_empty() => {
          let lines: Vec<_> =
            found.iter().map(|one| format!("{}: {}", one.name, one.description)).collect();
          Ok(Text::new(SCHEME, lines.join("\n")))
        }
        Some(one) => read(&one.path),
        None => Err(Fault::refused(format!("There is no skill {name}."))),
      };
      let answer = answer.map_or_else(|no| no.object(), |one| one.object());
      say(&co, Fact::says("done", a.about(), [answer])).await;
    }
  })
}

/// The skills in these folders, in the order of their names: each folder in them that holds a SKILL.md file that
/// the World reads as a text, under the name its frontmatter gives or else the name of the folder, and a name that
/// an earlier folder holds wins.
fn found(roots: impl Iterator<Item = PathBuf>) -> Vec<Skill> {
  let mut out: Vec<Skill> = Vec::new();
  for root in roots {
    let Ok(held) = fs::read_dir(&root) else { continue };
    let mut folders = held.filter_map(|one| one.ok().map(|one| one.path())).collect::<Vec<_>>();
    folders.sort();
    for folder in folders {
      let path = folder.join("SKILL.md");
      let Ok(text) = read(&path) else { continue };
      let (name, description) = frontmatter(&text.content);
      let name = name.unwrap_or_else(|| {
        folder.file_name().map(|one| one.to_string_lossy().into()).unwrap_or_default()
      });
      if !out.iter().any(|one| one.name == name) {
        out.push(Skill { name, description: description.unwrap_or_default(), path });
      }
    }
  }
  out.sort_by(|one, two| one.name.cmp(&two.name));
  out
}

/// The name and the description that the frontmatter of a SKILL.md file gives, each on one line: the text after its
/// key and the lines under the key that are indented, less the mark of a block and the quotes around it.
fn frontmatter(text: &str) -> (Option<String>, Option<String>) {
  let mut lines = text.lines();
  if lines.next().map(str::trim_end) != Some("---") {
    return (None, None);
  }
  let mut fields: Vec<(&str, String)> = Vec::new();
  for line in lines.take_while(|one| one.trim_end() != "---") {
    match fields.last_mut() {
      Some((_, value)) if line.starts_with([' ', '\t']) => {
        value.push(' ');
        value.push_str(line.trim());
      }
      _ => {
        if let Some((key, value)) = line.split_once(':') {
          let value = value.trim();
          fields.push((
            key.trim(),
            if BLOCKS.contains(&value) { String::new() } else { value.to_owned() },
          ));
        }
      }
    }
  }
  let field = |key: &str| {
    let value = fields.iter().find(|one| one.0 == key)?.1.trim();
    let bare = ['"', '\''].iter().find_map(|mark| value.strip_prefix(*mark)?.strip_suffix(*mark));
    Some(bare.unwrap_or(value).split_whitespace().collect::<Vec<_>>().join(" "))
      .filter(|one| !one.is_empty())
  };
  (field("name"), field("description"))
}

#[cfg(test)]
mod tests {
  use super::frontmatter as read;

  #[test]
  fn a_frontmatter_gives_a_name_and_a_description_each_on_one_line() {
    let some =
      |name: &str, description: &str| (Some(name.to_owned()), Some(description.to_owned()));
    assert_eq!(
      read("---\nname: brew\ndescription: Make tea.\n---\nBody."),
      some("brew", "Make tea.")
    );
    assert_eq!(
      read("---\r\nname: 'brew'\r\ndescription: \"Make: tea.\"\r\n---\r\n"),
      some("brew", "Make: tea.")
    );
    assert_eq!(
      read("---\nname: brew\ndescription: |\n  Make\n  tea.\nlicense: none\n---\n"),
      some("brew", "Make tea.")
    );
    assert_eq!(
      read("---\nname: brew\ndescription: >-\n  Make\n\n  tea.\n---\n"),
      some("brew", "Make tea.")
    );
    assert_eq!(read("---\ndescription: Make tea.\n---\n"), (None, Some("Make tea.".to_owned())));
    assert_eq!(
      read("# Brew\n\nname: brew\n"),
      (None, None),
      "a file that opens with no --- has no frontmatter"
    );
  }
}
