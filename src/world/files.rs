//! The files of the machine: a cd, a read and a write of a path that nobody of the engine serves.

use std::{
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
};

use super::{SCRATCH, Scratch, bash::door, here, resolved};
use crate::{
  ear::{Co, Ear, ear, hear, say},
  fact::Fact,
  value::{Fault, Object, Text},
};

/// The ear of the files: it answers a cd with the directory that the path names, a read with the text at the path
/// and a write with the text as it stands after, but a write of a door of a command, which the ear of the commands
/// answers, each resolved against the working directory of the chain, or with
/// the refusal. The path of a read or a write that is `~` or starts with `~/` resolves against the home of the user,
/// and one of `scratch://` against the scratch directory of the life, which no chain stands in. A file whose every
/// line ends with CRLF reads with LF, and a write to it writes CRLF.
pub fn files(scratch: Scratch) -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if !a.question() || !matches!(a.kind(), "cd" | "read" | "write") {
        continue;
      }
      // A write of a door of a command of the life is the ear of the commands' to answer, wherever that ear stands.
      if a.kind() == "write"
        && let Some((_, command, _)) = door(&a)
        && co.call("get", vec![Object::string(&command)], vec![]).await?.as_ref().type_name()
          != "NoneType"
      {
        continue;
      }
      let answer = match a.kind() {
        "cd" => moved(&mut co, &a).await.map(Object::string),
        _ => served(&mut co, &a, &scratch).await.map(|text| text.object()),
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
  let path = native(&path, cfg!(windows));
  if path.starts_with("scratch:") {
    return Err(Fault::refused(format!(
      "A chain cannot stand in {path}, since {SCRATCH} has no path to stand in."
    )));
  }
  let at = resolved(&here(co, a.on()).await?, &path);
  if !at.is_dir() {
    return Err(Fault::refused(format!("There is no directory at {}.", at.display())));
  }
  Ok(at.display().to_string())
}

/// What a read or a write came to.
async fn served(co: &mut Co, a: &Fact, scratch: &Scratch) -> Result<Text, Fault> {
  let (path, content) = match a.kind() {
    "read" => (a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default(), None),
    _ => {
      let text =
        a.word(1).and_then(Text::of).ok_or_else(|| Fault::refused("a write gives a text"))?;
      (text.path, Some(text.content))
    }
  };
  let path = native(&path, cfg!(windows));
  if let Some(path) = path.strip_prefix(SCRATCH) {
    return scratch.served(path, content);
  }
  if path.starts_with("scratch:") {
    return Err(Fault::refused(format!(
      "{path} is no path of {SCRATCH}, which starts with two slashes."
    )));
  }
  if path.contains("://") {
    return Err(Fault::refused(format!("No file at {path}.")));
  }
  let at = match homed(&path, std::env::home_dir()) {
    Some(at) => at?,
    None => resolved(&here(co, a.on()).await?, &path),
  };
  if let Some(content) = content {
    if let Some(parent) = at.parent() {
      fs::create_dir_all(parent).map_err(|no| failed(&no, at.display()))?;
    }
    fs::write(&at, ended(&at, content)).map_err(|no| failed(&no, at.display()))?;
  }
  read(&at)
}

/// Whether bytes end each of their lines with CRLF: they hold a line break, and a CR comes before each one.
fn crlf(bytes: &[u8]) -> bool {
  let mut breaks = memchr::memchr_iter(b'\n', bytes).peekable();
  breaks.peek().is_some() && breaks.all(|at| at > 0 && bytes[at - 1] == b'\r')
}

/// The content of a write as the file at a path takes it: with CRLF at the end of each line when every line of the
/// file there ends with CRLF, so the file keeps its line ends, and else as it is given.
pub(super) fn ended(at: &Path, content: String) -> String {
  if fs::read(at).is_ok_and(|held| crlf(&held)) {
    content.replace("\r\n", "\n").replace('\n', "\r\n")
  } else {
    content
  }
}

/// A path as Windows names it, from the form of the sh of Git for Windows, which names `C:\\x` as `/c/x` and the
/// scratch directory, its temporary directory, as `/tmp`: on Windows, `/c` and `/c/x` are `C:/` and `C:/x`, and `/tmp`
/// and `/tmp/x` are `scratch://` and `scratch://x`. Any other path, and every path on another machine, stands as it is.
pub(super) fn native(path: &str, windows: bool) -> String {
  if !windows {
    return path.to_owned();
  }
  if let Some(rest) =
    path.strip_prefix("/tmp").filter(|rest| rest.is_empty() || rest.starts_with('/'))
  {
    return format!("{SCRATCH}{}", rest.trim_start_matches('/'));
  }
  match path.as_bytes() {
    [b'/', drive, rest @ ..]
      if drive.is_ascii_alphabetic() && (rest.is_empty() || rest[0] == b'/') =>
    {
      format!("{}:/{}", char::from(*drive).to_ascii_uppercase(), path[2..].trim_start_matches('/'))
    }
    _ => path.to_owned(),
  }
}

/// The text at a path: a file of utf-8 of any size, since the engine bounds what a read tells.
pub(crate) fn read(at: &Path) -> Result<Text, Fault> {
  text(at, at.display().to_string())
}

/// The text at a path under a name, which the text and each refusal of it give. A file whose every line ends with
/// CRLF gives its lines with LF, so a text holds the same lines on every machine, and a write gives them their CRLF
/// back.
pub(super) fn text(at: &Path, name: String) -> Result<Text, Fault> {
  let info = fs::metadata(at).map_err(|no| failed(&no, &name))?;
  if !info.is_file() {
    return Err(Fault::refused(format!("Read needs a text file: {name}")));
  }
  let bytes = fs::read(at).map_err(|no| failed(&no, &name))?;
  let content = String::from_utf8(bytes)
    .map_err(|_| Fault::refused(format!("Read needs a text file in utf-8: {name}")))?;
  let content = if crlf(content.as_bytes()) { content.replace("\r\n", "\n") } else { content };
  Ok(Text::new(name, content))
}

/// A path that is `~` or starts with `~/`, resolved against the home of the user, as a shell reads it, or the refusal
/// when the user has no home; and nothing for any other path, which resolves against the working directory.
fn homed(path: &str, home: Option<PathBuf>) -> Option<Result<PathBuf, Fault>> {
  let rest = path.strip_prefix('~').filter(|rest| rest.is_empty() || rest.starts_with('/'))?;
  let refused = || Fault::refused(format!("There is no home of the user for {path}."));
  Some(home.map(|home| resolved(&home, rest.trim_start_matches('/'))).ok_or_else(refused))
}

/// What the machine refused at a path, under its name, in plain words, which the model and the operator both read.
pub(super) fn failed(no: &std::io::Error, name: impl std::fmt::Display) -> Fault {
  match no.kind() {
    ErrorKind::NotFound => Fault::refused(format!("There is no file at {name}.")),
    _ => Fault::refused(format!("{no}: {name}")),
  }
}

#[cfg(test)]
#[path = "files.test.rs"]
mod test;
