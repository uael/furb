//! The window gives its place to a new build of itself: while it runs, it watches its program, and
//! once a build replaced it, it writes where it stands in a file beside its cards, ends, and starts
//! the new program, which opens where this one stood: on the same thread, at the same place of its
//! feed, with the same draft. A person who works on furb from its window sees each build at once.

use std::{
  fs,
  path::{Path, PathBuf},
  process,
};

use furb_client::{Key, Opened, Place, Time};
use furb_host::built;
use jiff::{Timestamp, tz::TimeZone};
use serde::{Deserialize, Serialize};

use crate::model::Model;

/// The variable by which a window tells the program that it starts that it opens where it stood.
pub(crate) const RELOADED: &str = "FURB_RELOADED";

/// Where a window stood: the thread that the operator opened from the list, the agent or the sub
/// thread of it that he read, the place of the feed, the draft, and the moment of the time of the
/// thread that he held, none for now. A window that gives its place writes it, and so does each
/// send, as [`Seen`] says.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Stood {
  pub thread: Option<Key>,
  pub agent: Option<String>,
  pub top: Option<(usize, isize)>,
  pub draft: String,
  pub python: bool,
  #[serde(default)]
  pub moment: Option<u64>,
}

/// What the operator saw when he sent a message: the size of the window and where it stood, which
/// `furb-frames render --seen` draws again as text, cells and an image. The window writes it at
/// each send in `seen-<pid>.json` beside its cards, the file of its process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seen {
  pub cols: u16,
  pub rows: u16,
  pub stood: Stood,
}

/// The file where the window of a process writes what the operator saw, beside its cards.
pub(crate) fn seen_of(kept: &Path) -> PathBuf {
  kept.with_file_name(format!("seen-{}.json", process::id()))
}

/// The file where a window writes where it stood, beside the file of its cards.
pub(crate) fn file_of(kept: &Path) -> PathBuf {
  kept.with_file_name("reload.json")
}

/// The time at which a build wrote a program, as a person reads it on this computer, which names
/// the build that a window runs after it gave its place.
pub(crate) fn built_at(program: &Path) -> Option<String> {
  let written = fs::metadata(program).ok()?.modified().ok()?;
  let at = Timestamp::try_from(written).ok()?.to_zoned(TimeZone::system());
  Some(at.strftime("%H:%M").to_string())
}

/// The watch of the program of a window: its path, its build when the window opened, and its
/// build at the last look, as [`built`] gives them.
pub(crate) struct Watch {
  program: PathBuf,
  born: Option<String>,
  seen: Option<String>,
}

impl Watch {
  pub(crate) fn new(program: PathBuf) -> Self {
    let born = built(&program);
    Self { program, seen: born.clone(), born }
  }

  /// Whether a build replaced the program and it held still since the last look, so the build
  /// wrote it whole.
  pub(crate) fn replaced(&mut self) -> bool {
    let now = built(&self.program);
    let still = now.is_some() && now != self.born && now == self.seen;
    self.seen = now;
    still
  }
}

impl Model {
  /// Where the window stands now.
  pub(crate) fn stood(&self) -> Stood {
    let opened = self.client.thread.as_ref();
    let root = opened.map(Opened::root);
    let agent = opened.filter(|one| one.from.is_some()).map(|one| one.thread.key.2.clone());
    let top = opened.and_then(|one| one.top).map(|one| (one.at, one.row));
    let draft = &self.client.draft;
    let moment = opened.and_then(|one| one.thread.time.as_ref()).and_then(|time| time.at);
    Stood {
      thread: root.map(|one| one.thread.key.clone()).filter(|key| !key.2.is_empty()),
      agent,
      top,
      draft: draft.text.clone(),
      python: draft.python,
      moment,
    }
  }

  /// The window opened where the window before it stood, which gave it its place.
  pub(crate) fn resume(&mut self, stood: Stood) {
    if let Some(key) = stood.thread {
      self.client.selected = Some(key);
      self.open_thread();
      if let Some(agent) = stood.agent {
        self.open_agent(agent);
      }
      if let Some(opened) = self.client.thread.as_mut() {
        opened.top = stood.top.map(|(at, row)| Place { at, row });
        // The thread shows each page as it stood at the moment that he held.
        if let Some(at) = stood.moment {
          let since = self.client.moved;
          opened.thread.time = Some(Time { at: Some(at), stale: true, since, ..Time::default() });
        }
      }
    }
    // The editor takes the draft back, and the draft holds it, as a recall of the history does.
    self.client.draft.python = stood.python;
    if !stood.draft.is_empty() {
      self.client.restore = Some(stood.draft.clone());
      self.client.draft.text = stood.draft;
    }
  }
}
