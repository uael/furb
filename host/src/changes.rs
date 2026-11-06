//! The journal of the changes of files of a life, beside its record, which a window reads to show
//! and to undo what each write changed. As an ear that comes before the files of the crate, it
//! reads the text that a write replaces before the files take the write, and it keeps the change
//! once the write is done. It says nothing, so each write goes on to the ear that takes it.

use std::{
  collections::HashMap,
  fs::{self, OpenOptions},
  io::{self, Write},
  path::{Path, PathBuf},
  sync::Arc,
};

use furb::{
  Ear, Object,
  ear::{Co, ear, hear},
};
use serde_json::{Value, json};
use tokio::sync::{
  mpsc::{self, UnboundedReceiver, UnboundedSender},
  oneshot,
};

use crate::{ears::json_of, error::JournalError, event::Event};

/// The journal of the changes of a record, as a window finds it.
pub(crate) fn journal(record: &str) -> PathBuf {
  PathBuf::from(format!("{record}.changes.jsonl"))
}

/// The journal of the changes of a life that stands on a folder, which the thread of the life owns
/// across each of its births. One task appends each change off the thread of the life, in the
/// order of the writes, so a slow disk holds no fact of the life. A change is on the disk only
/// once [`Journal::flushed`] gives: the life waits for it before it ends.
pub(crate) struct Journal {
  folder: PathBuf,
  lines: UnboundedSender<Line>,
}

/// What the task of a journal takes, in order: a change to append, or a flush, which hears what
/// the disk lost since the last flush once each change before it is done.
enum Line {
  /// A change, and the chain whose write made it, which hears that it changed once the change is
  /// on the disk.
  Change {
    chain: String,
    change: String,
  },
  Flush(oneshot::Sender<Result<(), JournalError>>),
}

/// How a journal appends one change at a path. The tests give one that a test holds.
type Append = Arc<dyn Fn(&Path, &str) -> io::Result<()> + Send + Sync>;

impl Journal {
  /// The journal of a life that stands on a folder, appended at a path, which tells each fault of
  /// the disk as a notice at once.
  pub(crate) fn spawn(folder: PathBuf, at: PathBuf, told: UnboundedSender<Event>) -> Self {
    Self::appending(folder, at, told, Arc::new(appended))
  }

  fn appending(folder: PathBuf, at: PathBuf, told: UnboundedSender<Event>, append: Append) -> Self {
    let (lines, heard) = mpsc::unbounded_channel();
    tokio::spawn(written(at, heard, told, append));
    Journal { folder, lines }
  }

  /// Each change said before it on the disk, or what the disk lost since the last flush.
  pub(crate) async fn flushed(&self) -> Result<(), JournalError> {
    let (done, flushed) = oneshot::channel();
    self.lines.send(Line::Flush(done)).map_err(|_stopped| JournalError::Gone)?;
    flushed.await.map_err(|_stopped| JournalError::Gone)?
  }

  /// The ear of the changes for one birth of the life, which keeps each change in this journal.
  pub(crate) fn ear(&self) -> Box<dyn Ear> {
    let (folder, lines) = (self.folder.clone(), self.lines.clone());
    ear(move |mut co| async move {
      let mut before: HashMap<String, (String, Value)> = HashMap::new();
      loop {
        let a = hear(&mut co).await;
        let fact = json_of(&a);
        if a.question() && a.kind() == "write" {
          if let Some(change) = replaced(&mut co, &folder, &fact).await? {
            let chain = fact[3].as_str().unwrap_or_default().to_owned();
            before.insert(a.about().to_owned(), (chain, change));
          }
        } else if a.kind() == "done"
          && let Some((chain, mut change)) = before.remove(a.about())
          && fact[3]["is"] == "Text"
          && let Some(after) = fact[3]["content"].as_str()
        {
          change["after"] = json!(after);
          change["act"] = json!(a.about());
          if lines.send(Line::Change { chain, change: change.to_string() }).is_err() {
            return Ok(());
          }
        }
      }
    })
  }
}

/// The task of a journal: each line taken in order, until the journal and each of its ears are
/// gone.
async fn written(
  at: PathBuf,
  mut heard: UnboundedReceiver<Line>,
  told: UnboundedSender<Event>,
  append: Append,
) {
  let mut lost: Option<JournalError> = None;
  // The journal tells the operator when it begins to lose changes and when it takes changes again,
  // not each change.
  let mut failing = false;
  while let Some(line) = heard.recv().await {
    let (chain, change) = match line {
      Line::Change { chain, change } => (chain, change),
      Line::Flush(done) => {
        // A flush whose life no longer waits for it hears nothing, and what was lost stays told.
        done.send(lost.take().map_or(Ok(()), Err)).ok();
        continue;
      }
    };
    let (path, append) = (at.clone(), Arc::clone(&append));
    let fault = match tokio::task::spawn_blocking(move || append(&path, &change)).await {
      // The chain of the write hears that its changes changed only now: a window that read the
      // journal at the fact of the write could find it without the change, and hear nothing after.
      Ok(Ok(())) => {
        if failing {
          failing = false;
          let notice =
            format!("the journal of file changes at {} takes changes again", at.display());
          told.send(Event::Notice(notice)).ok();
        }
        if !chain.is_empty() {
          told.send(Event::Changed(chain)).ok();
        }
        continue;
      }
      Ok(Err(no)) => no,
      Err(no) => io::Error::other(no),
    };
    if !failing {
      failing = true;
      let notice =
        format!("the journal of file changes at {} took no change: {fault}", at.display());
      told.send(Event::Notice(notice)).ok();
    }
    match &mut lost {
      Some(JournalError::Lost { count, .. }) => *count += 1,
      _ => lost = Some(JournalError::Lost { at: at.clone(), count: 1, first: fault }),
    }
  }
}

/// The change that a write begins: the path of its file where its chain stands, and the text that
/// the file held, or that the write makes it. A path of a door, as `scratch://`, is no file. The
/// file is read on the thread of the life, since the files take the write once this ear yields.
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
  Ok(Some(before(&file)))
}

/// The change that a write of a file begins, with what the file held. A file that does not stand is
/// made by the write. A file that cannot be read keeps `before` empty for the readers that do not
/// know `unread`, and `unread` says why, so no undo puts back a text that the file never held.
fn before(file: &Path) -> Value {
  let mut change = json!({ "path": file.display().to_string(), "before": "" });
  match fs::read_to_string(file) {
    Ok(text) => change["before"] = json!(text),
    Err(no) if no.kind() == io::ErrorKind::NotFound => change["made"] = json!(true),
    Err(no) => change["unread"] = json!(no.to_string()),
  }
  change
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

#[cfg(test)]
#[path = "changes.test.rs"]
mod tests;
