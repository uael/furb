//! The files of the machine: a cd, a read and a write of a path that nobody of the engine serves.

use std::{fs, io::ErrorKind, path::Path};

use super::{here, resolved};
use crate::{
  ear::{Co, Ear, ear, hear, say},
  fact::Fact,
  value::{Fault, Object, Text},
};

/// The largest text a read gives, in bytes.
const LARGEST: u64 = 524_288;

/// The ear of the files: it answers a cd with the directory that the path names, a read with the text at the path
/// and a write with the text as it stands after, each resolved against the working directory of the chain, or with
/// the refusal.
pub fn files() -> Box<dyn Ear> {
  ear(|mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if !a.question() || !matches!(a.kind(), "cd" | "read" | "write") {
        continue;
      }
      let answer = match a.kind() {
        "cd" => moved(&mut co, &a).await.map(Object::string),
        _ => served(&mut co, &a).await.map(|text| text.object()),
      };
      let answer = answer.unwrap_or_else(|fault| fault.object());
      say(&mut co, Fact::says("done", a.about(), [answer]));
    }
  })
}

/// The directory a cd came to: its path resolved against the working directory of the chain, when a directory
/// stands there.
async fn moved(co: &mut Co, a: &Fact) -> Result<String, Fault> {
  let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  let at = resolved(&here(co, a.on()).await?, &path);
  if !at.is_dir() {
    return Err(Fault::refused(format!("There is no directory at {}.", at.display())));
  }
  Ok(at.display().to_string())
}

/// What a read or a write came to.
async fn served(co: &mut Co, a: &Fact) -> Result<Text, Fault> {
  let (path, content) = match a.kind() {
    "read" => (a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default(), None),
    _ => {
      let text =
        a.word(1).and_then(Text::of).ok_or_else(|| Fault::refused("a write gives a text"))?;
      (text.path, Some(text.content))
    }
  };
  if path.contains("://") {
    return Err(Fault::refused(format!("No file at {path}.")));
  }
  let at = resolved(&here(co, a.on()).await?, &path);
  if let Some(content) = content {
    if let Some(parent) = at.parent() {
      fs::create_dir_all(parent).map_err(|no| failed(&no, &at))?;
    }
    fs::write(&at, content).map_err(|no| failed(&no, &at))?;
  }
  read(&at)
}

/// The text at a path: a file of utf-8 no larger than a read gives.
pub(crate) fn read(at: &Path) -> Result<Text, Fault> {
  let shown = at.display();
  let info = fs::metadata(at).map_err(|no| failed(&no, at))?;
  if !info.is_file() || info.len() > LARGEST {
    return Err(Fault::refused(format!("Read needs a text file at most {LARGEST} bytes: {shown}")));
  }
  let bytes = fs::read(at).map_err(|no| failed(&no, at))?;
  let content = String::from_utf8(bytes)
    .map_err(|_| Fault::refused(format!("Read needs a text file in utf-8: {shown}")))?;
  Ok(Text::new(shown.to_string(), content))
}

/// What the machine refused, in plain words, which the model and the operator both read.
fn failed(no: &std::io::Error, at: &Path) -> Fault {
  match no.kind() {
    ErrorKind::NotFound => Fault::refused(format!("There is no file at {}.", at.display())),
    _ => Fault::refused(format!("{no}: {}", at.display())),
  }
}
