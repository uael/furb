//! The journal of the changes of files of a life, beside its record, which a window reads to show
//! and to undo what each write changed. As an ear that comes before the files of the crate, it
//! reads the text that a write replaces before the files take the write, and it keeps the change
//! once the write is done. It says nothing, so each write goes on to the ear that takes it.

use std::{
  collections::HashMap,
  fs::{self, OpenOptions},
  io::{self, Write},
  path::{Path, PathBuf},
};

use furb::{
  Ear, Object,
  ear::{Co, ear, hear},
};
use serde_json::{Value, json};
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::{ears::json_of, event::Event};

/// The journal of the changes of a record, as a window finds it.
pub(crate) fn journal(record: &str) -> PathBuf {
  PathBuf::from(format!("{record}.changes.jsonl"))
}

/// The ear of the changes of a life that stands on a folder and keeps its journal at a path. A
/// task of the runtime of the life appends each change, off the thread of the life, in order, and
/// tells a fault of the disk as a notice.
pub(crate) fn changes(folder: PathBuf, at: PathBuf, told: UnboundedSender<Event>) -> Box<dyn Ear> {
  let (lines, mut heard) = mpsc::unbounded_channel::<String>();
  tokio::spawn(async move {
    while let Some(line) = heard.recv().await {
      let path = at.clone();
      let written = tokio::task::spawn_blocking(move || appended(&path, &line)).await;
      let fault = match written {
        Ok(Ok(())) => continue,
        Ok(Err(no)) => no.to_string(),
        Err(no) => no.to_string(),
      };
      let notice =
        format!("the journal of file changes at {} took no change: {fault}", at.display());
      told.send(Event::Notice(notice)).ok();
    }
  });
  ear(move |mut co| async move {
    let mut before: HashMap<String, Value> = HashMap::new();
    loop {
      let a = hear(&mut co).await;
      let fact = json_of(&a);
      if a.question() && a.kind() == "write" {
        if let Some(change) = replaced(&mut co, &folder, &fact).await? {
          before.insert(a.about().to_owned(), change);
        }
      } else if a.kind() == "done"
        && let Some(mut change) = before.remove(a.about())
        && fact[3]["is"] == "Text"
        && let Some(after) = fact[3]["content"].as_str()
      {
        change["after"] = json!(after);
        change["act"] = json!(a.about());
        if lines.send(change.to_string()).is_err() {
          return Ok(());
        }
      }
    }
  })
}

/// The change that a write begins: the path of its file where its chain stands, and the text that
/// the file held, or that the write makes it. A path of a door, as `scratch://`, is no file.
async fn replaced<W: 'static>(
  co: &mut Co<W>,
  folder: &Path,
  fact: &Value,
) -> Result<Option<Value>, furb::Fault> {
  let (Some(on), Some(path)) = (fact[3].as_str(), fact[4]["path"].as_str()) else {
    return Ok(None);
  };
  if path.contains("://") {
    return Ok(None);
  }
  let here = co.call("cwd", vec![], vec![("on", Object::string(on))]).await?;
  let here = here.as_ref().as_str().unwrap_or_default().to_owned();
  let file = folder.join(homed(&here)).join(homed(path));
  let mut change = json!({ "path": file.display().to_string(), "before": "" });
  match fs::read_to_string(&file) {
    Ok(text) => change["before"] = json!(text),
    Err(_) if !file.exists() => change["made"] = json!(true),
    Err(_) => {}
  }
  Ok(Some(change))
}

/// A path that the World writes with `~` for the home of the user, as a path of the machine.
fn homed(path: &str) -> PathBuf {
  match (path.strip_prefix('~'), std::env::home_dir()) {
    (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
      home.join(rest.trim_start_matches('/'))
    }
    _ => PathBuf::from(path),
  }
}

/// One change appended to the journal as one line, on the disk before the next.
fn appended(at: &Path, line: &str) -> io::Result<()> {
  if let Some(folder) = at.parent() {
    fs::create_dir_all(folder)?;
  }
  let mut options = OpenOptions::new();
  options.create(true).append(true);
  #[cfg(unix)]
  std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
  let mut file = options.open(at)?;
  file.write_all(format!("{line}\n").as_bytes())?;
  file.sync_all()
}
