//! The files of the machine: a cd, a read and a write of a path that nobody of the engine serves.

use std::{
  fs,
  io::{self, ErrorKind, Write},
  path::{Path, PathBuf},
  sync::atomic::{AtomicU32, Ordering},
};

use super::{SCRATCH, Scratch, bash::door, here, images, resolved};
use crate::{
  ear::{Co, Ear, hear, hearing, say},
  fact::Fact,
  value::{Fault, Object, Text},
};

/// The ear of the files: it answers a cd with the directory that the path names, a read with the text at the path
/// and a write with the text as it stands after, but a write of a door of a command, which the ear of the commands
/// answers, each resolved against the working directory of the chain, or with
/// the refusal. The path of a read or a write that is `~` or starts with `~/`, or with `~\\` on Windows, resolves
/// against the home of the user,
/// and one of `scratch://` against the scratch directory of the life, which no chain stands in. A file whose every
/// line ends with CRLF reads with LF, and a write to it writes CRLF. A path whose first part is the name of an act
/// of the life is a door that no ear of the engine served, and never a file, so the ear refuses it with the chain of
/// that act. A write puts the whole text in place in one step, as `stored` says, so no reader sees a torn file. A read
/// of an image keeps it in the directory of the images of the life, the gallery, as `seen` says.
pub fn files(scratch: Scratch, gallery: Option<PathBuf>) -> Box<dyn Ear> {
  hearing(&["cd", "read", "write"], move |mut co| async move {
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
        _ => served(&mut co, &a, &scratch, gallery.as_deref()).await.map(|text| text.object()),
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
async fn served(
  co: &mut Co,
  a: &Fact,
  scratch: &Scratch,
  gallery: Option<&Path>,
) -> Result<Text, Fault> {
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
    return scratch.served(path, content, gallery);
  }
  if path.starts_with("scratch:") {
    return Err(Fault::refused(format!(
      "{path} is no path of {SCRATCH}, which starts with two slashes."
    )));
  }
  if path.contains("://") {
    return Err(Fault::refused(format!("No file at {path}.")));
  }
  let head = path.split('/').next().unwrap_or_default();
  if co.call("get", vec![Object::string(head)], vec![]).await?.as_ref().type_name() != "NoneType" {
    let chain = co.call("scope", vec![Object::string(head)], vec![]).await?;
    let chain = chain.as_ref().as_str().unwrap_or_default().to_owned();
    return Err(Fault::refused(format!(
      "{path} is no file, since {head} is an act of {chain}: {} it on {chain}.",
      a.kind()
    )));
  }
  let at = match homed(&path, std::env::home_dir()) {
    Some(at) => at?,
    None => resolved(&here(co, a.on()).await?, &path),
  };
  if let Some(content) = content {
    if let Some(parent) = at.parent() {
      fs::create_dir_all(parent).map_err(|no| failed(&no, at.display()))?;
    }
    stored(&at, ended(&at, content).as_bytes()).map_err(|no| failed(&no, at.display()))?;
  }
  text(&at, at.display().to_string(), gallery)
}

/// Writes bytes to the file at a path in one step, so a crash or a failure midway leaves the old file whole and a
/// reader sees the old bytes or the new, and never a part. The bytes go to a temporary file in the same folder, which
/// is synced, takes the mode of the old file, and is renamed over the target. A link is resolved to its target first,
/// so it stays a link. A file that cannot be opened for writing is refused as a plain write refuses it. Where a
/// temporary file cannot be made, or the rename fails, as on Windows while another program holds the file open, the
/// write is the plain write in place, which is not atomic. A hard link to the file does not see the new bytes, and a
/// crash may leave a temporary file `.<name>.<pid>.<n>.tmp` in the folder.
pub(super) fn stored(at: &Path, bytes: &[u8]) -> io::Result<()> {
  replace(at, bytes, |file, bytes| file.write_all(bytes))
}

/// `stored` with the filling of the temporary file given, so a test can make it fail midway.
fn replace(
  at: &Path,
  bytes: &[u8],
  fill: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
  static TEMPS: AtomicU32 = AtomicU32::new(0);
  let Some(target) = linked(at) else { return fs::write(at, bytes) };
  let mode = match fs::metadata(&target) {
    Ok(info) if info.is_file() => {
      fs::OpenOptions::new().write(true).open(&target)?;
      Some(info.permissions())
    }
    Ok(_) => return fs::write(&target, bytes),
    Err(no) if no.kind() == ErrorKind::NotFound => None,
    Err(_) => return fs::write(&target, bytes),
  };
  let Some(name) = target.file_name() else { return fs::write(&target, bytes) };
  let temp = target.with_file_name(format!(
    ".{}.{}.{}.tmp",
    name.to_string_lossy(),
    std::process::id(),
    TEMPS.fetch_add(1, Ordering::Relaxed)
  ));
  let Ok(mut file) = fs::OpenOptions::new().write(true).create_new(true).open(&temp) else {
    return fs::write(&target, bytes);
  };
  let done = fill(&mut file, bytes)
    .and_then(|()| file.sync_all())
    .and_then(|()| mode.map_or(Ok(()), |mode| file.set_permissions(mode)));
  drop(file);
  if let Err(no) = done {
    let _ = fs::remove_file(&temp);
    return Err(no);
  }
  if fs::rename(&temp, &target).is_err() {
    let _ = fs::remove_file(&temp);
    return fs::write(&target, bytes);
  }
  Ok(())
}

/// The file that a path names after its links: a link is followed to its target, each relative link from the folder
/// that holds it, and nothing for a chain of more than 40 links, which a plain write refuses.
fn linked(at: &Path) -> Option<PathBuf> {
  let mut at = at.to_path_buf();
  for _ in 0..40 {
    match fs::read_link(&at) {
      Ok(to) => at = at.parent().unwrap_or(Path::new("")).join(to),
      Err(_) => return Some(at),
    }
  }
  None
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
  text(at, at.display().to_string(), None)
}

/// The text at a path under a name, which the text and each refusal of it give. A file whose every line ends with
/// CRLF gives its lines with LF, so a text holds the same lines on every machine, and a write gives them their CRLF
/// back. A file that is no text of utf-8 and is an image gives what `seen` says.
pub(super) fn text(at: &Path, name: String, gallery: Option<&Path>) -> Result<Text, Fault> {
  let info = fs::metadata(at).map_err(|no| failed(&no, &name))?;
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

/// A path that is `~` or starts with `~/`, or with `~\\` on Windows, resolved against the home of the user, as a shell
/// reads it, or the refusal when the user has no home; and nothing for any other path, which resolves against the
/// working directory.
fn homed(path: &str, home: Option<PathBuf>) -> Option<Result<PathBuf, Fault>> {
  let slash = |one: char| one == '/' || cfg!(windows) && one == '\\';
  let rest = path.strip_prefix('~').filter(|rest| rest.is_empty() || rest.starts_with(slash))?;
  let refused = || Fault::refused(format!("There is no home of the user for {path}."));
  Some(home.map(|home| resolved(&home, rest.trim_start_matches(slash))).ok_or_else(refused))
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
