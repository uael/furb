//! The files of the machine: a cd, a read and a write of a path that nobody of the engine serves.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  fmt, fs,
  io::{self, ErrorKind},
  path::{Path, PathBuf},
};

use super::{
  SCRATCH, Scratch,
  bash::door,
  disk::{self, Access},
  here, images, resolved,
  scratch::{Beneath, beneath},
};
use std::env;
use std::path;

/// The scheme of the paths of the config directory of the user.
pub const CONFIG: &str = "config://";
use crate::{
  ear::{Co, Ear, hear, hearing, say},
  fact::Fact,
  value::{Fault, Given, Object, Text},
};

/// The ear of the files: it answers a cd with the directory that the path names, a read with the
/// text at the path, or with one line for each entry of a folder, in order of name, with a slash
/// after a folder and no walk below it, and a write with the text as it stands after, but a write
/// of a door of a command, which the ear of the commands answers, each resolved against the working
/// directory of the chain, or with the refusal. The path of a cd, a read or a write that is `~` or
/// starts with `~/`, or with `~\\` on Windows, resolves against the home of the user, one of
/// `scratch://` against the scratch directory of the life, and one of `config://` against the
/// config directory of the user, in neither of which a chain stands. A file whose every line ends
/// with CRLF reads with LF, and a write to it writes CRLF. A path whose first part is the name of
/// an act of the life is a door that no ear of the engine served, and never a file, so the ear
/// refuses it with the chain of that act. The path of each text, of each refusal and of each cd
/// that it gives is as [`tilded`] writes it. A write puts the whole text in place in one step, as
/// `stored` says, so no reader sees a torn file. A read of an image keeps it in the directory of
/// the images of the life, the gallery, as `seen` says.
pub fn files(scratch: Scratch, config: PathBuf, gallery: Option<PathBuf>) -> Box<dyn Ear> {
  hearing(&["cd", "read", "write"], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if !a.question() || !matches!(a.kind(), "cd" | "read" | "write") {
        continue;
      }
      // A write of a door of a command of the life is the ear of the commands' to answer, wherever that ear stands.
      if a.kind() == "write"
        && let Some(at) = door(&a)
        && !co.call("get", vec![Object::string(&at.command)], vec![]).await?.as_ref().is_none()
      {
        continue;
      }
      let answer = match a.kind() {
        "cd" => moved(&mut co, &a).await.map(Object::string),
        _ => {
          served(&mut co, &a, &scratch, &config, gallery.as_deref()).await.map(|text| text.object())
        }
      };
      let answer = answer.unwrap_or_else(|fault| fault.object());
      say(&mut co, Fact::says("done", a.about(), [answer]));
    }
  })
}

/// The directory a cd came to: its path placed against the working directory of the chain, when a directory stands
/// there, as [`tilded`] writes it.
async fn moved(co: &mut Co, a: &Fact) -> Result<String, Fault> {
  let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  let path = native(&path, cfg!(windows));
  if let Some(scheme) =
    [SCRATCH, CONFIG].into_iter().find(|one| path.starts_with(&one[..one.len() - 2]))
  {
    return Err(Fault::refused(format!(
      "A chain cannot stand in {path}, since {scheme} has no path to stand in."
    )));
  }
  let at = placed(&here(co, a.on()).await?, &path)?;
  let shown = tilded(&at, env::home_dir().as_deref());
  if !at.is_dir() {
    return Err(Fault::refused(format!("There is no directory at {shown}.")));
  }
  Ok(shown)
}

/// What a read or a write came to.
async fn served(
  co: &mut Co,
  a: &Fact,
  scratch: &Scratch,
  config: &Path,
  gallery: Option<&Path>,
) -> Result<Text, Fault> {
  let (path, content) = if a.kind() == "read" {
    (a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default(), None)
  } else {
    let text =
      a.word(1).and_then(Text::of).ok_or_else(|| Fault::refused("a write gives a text"))?;
    (text.path, Some(text.content))
  };
  let path = native(&path, cfg!(windows));
  if let Some(path) = path.strip_prefix(SCRATCH) {
    return scratch.served(path, content, gallery);
  }
  if let Some(path) = path.strip_prefix(CONFIG) {
    let dir = Beneath { root: config, scheme: CONFIG, place: "the config directory" };
    return beneath(dir, path, content, gallery, || Ok(()));
  }
  if let Some(scheme) =
    [SCRATCH, CONFIG].into_iter().find(|one| path.starts_with(&one[..one.len() - 2]))
  {
    return Err(Fault::refused(format!(
      "{path} is no path of {scheme}, which starts with two slashes."
    )));
  }
  if path.contains("://") {
    return Err(Fault::refused(format!("No file at {path}.")));
  }
  let head = path.split('/').next().unwrap_or_default();
  if !co.call("get", vec![Object::string(head)], vec![]).await?.as_ref().is_none() {
    let chain = co.call("scope", vec![Object::string(head)], vec![]).await?;
    let chain = chain.as_ref().as_str().unwrap_or_default().to_owned();
    return Err(Fault::refused(format!(
      "{path} is no file, since {head} is an act of {chain}: {} it on {chain}.",
      a.kind()
    )));
  }
  let at = placed(&here(co, a.on()).await?, &path)?;
  if let Some(content) = content {
    filed(&at, content, at.display())?;
  }
  text(&at, tilded(&at, env::home_dir().as_deref()), gallery)
}

/// The content of a write put at a path, with the folders above it made, in one write as
/// [`disk::write`] says, and with the line ends that the file there has. `name` is the path as a
/// refusal names it.
pub(super) fn filed(at: &Path, content: String, name: impl fmt::Display) -> Result<(), Fault> {
  disk::write(at, ended(at, content).as_bytes(), Access::Shared)
    .map_err(|no| failed(&no.source, &name))
}

/// Whether bytes end each of their lines with CRLF: they hold a line break, and a CR comes before each one.
fn crlf(bytes: &[u8]) -> bool {
  let mut breaks = memchr::memchr_iter(b'\n', bytes).peekable();
  breaks.peek().is_some() && breaks.all(|at| at > 0 && bytes[at - 1] == b'\r')
}

/// The content of a write as the file at a path takes it: with CRLF at the end of each line when every line of the
/// file there ends with CRLF, so the file keeps its line ends, and else as it is given.
fn ended(at: &Path, content: String) -> String {
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
  text(at, tilded(at, env::home_dir().as_deref()), None)
}

/// The text at a path under a name, which the text and each refusal of it give. A file whose every line ends with
/// CRLF gives its lines with LF, so a text holds the same lines on every machine, and a write gives them their CRLF
/// back. A file that is no text of utf-8 and is an image gives what `seen` says.
pub(super) fn text(at: &Path, name: String, gallery: Option<&Path>) -> Result<Text, Fault> {
  let info = fs::metadata(at).map_err(|no| failed(&no, &name))?;
  if info.is_dir() {
    return listed(at, &name);
  }
  if !info.is_file() {
    return Err(Fault::refused(format!("Read needs a text file: {name}")));
  }
  let bytes = fs::read(at).map_err(|no| failed(&no, &name))?;
  let content = match String::from_utf8(bytes) {
    Ok(content) => content,
    Err(no) => return seen(at, &name, no.as_bytes(), gallery),
  };
  let content = if crlf(content.as_bytes()) { content.replace("\r\n", "\n") } else { content };
  Ok(Text::new(name, content))
}

/// The text of a file that is no text of utf-8 and is an image, a PNG, a JPEG, a GIF or a WebP: the image is copied
/// into the gallery under the digest of its bytes, and the text is one line that names it as a Markdown image,
/// `![name](furb-image://<digest>.<extension>)`, which the provider hands the model as an image. A life with no
/// gallery refuses it, and so does any other file that is no text of utf-8.
fn seen(at: &Path, name: &str, bytes: &[u8], gallery: Option<&Path>) -> Result<Text, Fault> {
  if images::kind(bytes).is_err() {
    return Err(Fault::refused(format!("Read needs a text file in utf-8: {name}")));
  }
  let Some(gallery) = gallery else {
    return Err(Fault::refused(format!(
      "{name} is an image, and this life keeps no directory of images to show it to a model."
    )));
  };
  let kept = images::attach(gallery, at).map_err(|why| Fault::refused(format!("{why} {name}")))?;
  Ok(Text::new(name, images::reference(&kept.name, &kept.uri)))
}

/// The text of a folder: one line for each entry, in order of name, with a slash after a folder,
/// and no walk below it. A link counts as what it points at, and a link that points at nothing
/// counts as a file.
fn listed(at: &Path, name: &str) -> Result<Text, Fault> {
  let mut names = Vec::new();
  for entry in fs::read_dir(at).map_err(|no| failed(&no, name))? {
    let entry = entry.map_err(|no| failed(&no, name))?;
    let mut one = entry.file_name().to_string_lossy().into_owned();
    if fs::metadata(entry.path()).is_ok_and(|info| info.is_dir()) {
      one.push('/');
    }
    names.push(one);
  }
  names.sort();
  Ok(Text::new(name, names.join("\n")))
}

/// A path that is `~` or starts with `~/`, or with `~\\` on Windows, resolved against the home of the user, as a shell
/// reads it, or the refusal when the user has no home; and nothing for any other path, which resolves against the
/// working directory.
pub(crate) fn homed(path: &str, home: Option<PathBuf>) -> Option<Result<PathBuf, Fault>> {
  let slash = |one: char| one == '/' || cfg!(windows) && one == '\\';
  let rest = path.strip_prefix('~').filter(|rest| rest.is_empty() || rest.starts_with(slash))?;
  let refused = || Fault::refused(format!("There is no home of the user for {path}."));
  Some(home.map(|home| resolved(&home, rest.trim_start_matches(slash))).ok_or_else(refused))
}

/// A path as the World writes it for a model, the reverse of [`homed`]: `~` in place of the home of the user when the
/// path is the home or stands under it, as `~/a.txt`, and the path as it stands otherwise, or when the home is the
/// root of the disk, under which every path stands.
pub(crate) fn tilded(path: &Path, home: Option<&Path>) -> String {
  let home = home.filter(|home| home.parent().is_some());
  match home.and_then(|home| path.strip_prefix(home).ok()) {
    Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
    Some(rest) => format!("~{}{}", path::MAIN_SEPARATOR, rest.display()),
    None => path.display().to_string(),
  }
}

/// The place of a path of the machine that a chain names: against the home of the user, as [`homed`] says, or else
/// against the directory.
pub(crate) fn placed(directory: &Path, path: &str) -> Result<PathBuf, Fault> {
  homed(path, env::home_dir()).unwrap_or_else(|| Ok(resolved(directory, path)))
}

/// What the machine refused at a path, under its name, in plain words, which the model and the operator both read.
pub(super) fn failed(no: &io::Error, name: impl fmt::Display) -> Fault {
  match no.kind() {
    ErrorKind::NotFound => Fault::refused(format!("There is no file at {name}.")),
    _ => Fault::refused(format!("{no}: {name}")),
  }
}

#[cfg(test)]
#[path = "files.test.rs"]
mod test;
