//! The cards of a session: each thread that the operator started, with the time since it stands as
//! it does, and the index that keeps them beside the record of a session at rest.
//!
//! The record is the one truth. The index, `<record>.threads.json`, is a cache: it names the tip
//! of the record that it was made from, and a reader drops an index whose tip is not the tip of
//! the record. A life writes the index of its record at its end, from the fold that it holds. The
//! daemon makes the index that a record lacks from a fold of the record with no life, off the
//! path of the list.

use std::{
  collections::HashMap,
  fs, io,
  path::{Path, PathBuf},
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};

use furb::{
  Fault,
  view::{self, Fold, Held, Name, Sent, Skill, Standing},
  world::{self, Tip},
};
use serde::{Deserialize, Serialize};

use crate::{
  ears::json_of_line,
  opens::{self, thousandths},
  protocol::{Card, Frame},
  serve::Outbox,
};

/// The cards of one session, the name of the session, what each operator sent to it, the skills
/// that its root read, and why no life of this build opens its record, whose threads then only
/// read. An index of a build before any of them holds none of them, so it reads as no index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Session {
  #[serde(default)]
  pub(crate) name: Option<String>,
  #[serde(default)]
  pub(crate) threads: Vec<Timed>,
  pub(crate) sent: Vec<Sent>,
  pub(crate) skills: Vec<Skill>,
  // A missing field is no index, not a record that opens.
  #[serde(deserialize_with = "Option::deserialize")]
  pub(crate) refused: Option<String>,
}

/// A card with the second of the epoch since which its thread stands as it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Timed {
  pub(crate) since: u64,
  #[serde(flatten)]
  pub(crate) thread: view::Card,
}

/// The index of a record as its file holds it.
#[derive(Serialize, Deserialize)]
struct Indexed {
  tip: Tip,
  #[serde(flatten)]
  session: Session,
}

/// The second of the epoch now.
pub(crate) fn now() -> u64 {
  seconds(SystemTime::now())
}

/// The millisecond of the epoch now.
pub(crate) fn millis() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}

fn seconds(time: SystemTime) -> u64 {
  time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// The second of the epoch at which a record last changed, and now for a record that the disk
/// does not hold yet.
pub(crate) fn changed(record: &Path) -> u64 {
  fs::metadata(record).and_then(|one| one.modified()).map_or_else(|_| now(), seconds)
}

fn index(record: &Path) -> PathBuf {
  let mut name = record.as_os_str().to_owned();
  name.push(".threads.json");
  PathBuf::from(name)
}

/// The session that the index of a record keeps, and whether the index names the tip of the record
/// now. An index of an older tip says threads that the record still holds, since a record only
/// grows. It reads the index and the end of the record, and no more of the record.
pub(crate) fn kept(record: &Path) -> Option<(Session, bool)> {
  let indexed: Indexed = serde_json::from_slice(&fs::read(index(record)).ok()?).ok()?;
  let fresh = Tip::of(record, None).is_ok_and(|tip| tip == indexed.tip);
  Some((indexed.session, fresh))
}

/// The index of a record written from a session, at the tip of the record now. It is written
/// whole under another name and then moved, so no reader reads a part of it.
pub(crate) fn keep(record: &Path, session: &Session) -> io::Result<()> {
  let indexed = Indexed { tip: Tip::of(record, None)?, session: session.clone() };
  let fresh = index(record).with_extension("json.new");
  fs::write(&fresh, serde_json::to_vec(&indexed)?)?;
  fs::rename(fresh, index(record))
}

/// Why a record gives no index.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
  #[error("{0}")]
  Read(Fault),
  #[error("the index cannot be written: {0}")]
  Write(#[from] io::Error),
}

/// The index of a record made from a fold of the record with no life, and kept, with how far it
/// stands in thousandths, as [`thousandths`] counts them. Each of its threads stands since the
/// record last changed, since a record tells no time.
pub(crate) fn folded(record: &Path, told: &mut dyn FnMut(u64)) -> Result<(), Error> {
  let since = changed(record);
  let reads = &mut |done, of| told(thousandths((done, of), (0, 1)));
  let retold = world::retold_with(record, reads).map_err(Error::Read)?;
  let size = fs::metadata(&retold.file).map_or(0, |meta| meta.len());
  let (mut fold, mut held, mut at, mut read) = (Fold::default(), Held::default(), 0, 0);
  let mut take = |line: &str| {
    read += line.len() as u64 + 1;
    told(thousandths((1, 1), (read, size)));
    let Some(fact) = json_of_line(line) else { return };
    at += 1;
    held.noted(&fact, at);
    for act in fold.recorded(&fact) {
      held.took(act);
    }
  };
  retold.each(&mut take).map_err(Error::Read)?;
  let name = retold.name().map(str::to_owned);
  let threads = held.cards().into_iter().map(|thread| Timed { since, thread }).collect();
  let (sent, skills) = (held.sent(), held.skills());
  let refused = world::unopened(record).map(|no| no.to_string());
  Ok(keep(record, &Session { name, threads, sent, skills, refused })?)
}

/// The times of the cards of a life, which the life keeps as its cards change.
#[derive(Default)]
pub(crate) struct Times {
  /// The kind of the standing of each thread, and since when it stands so.
  since: HashMap<Name, (u8, u64)>,
}

impl Times {
  /// The times that the index of a record keeps, at whatever tip it names, and none for a record
  /// with no index: a thread whose standing changed since takes a new time.
  pub(crate) fn kept(record: Option<&str>) -> Self {
    let kept = record.and_then(|one| kept(Path::new(one)));
    let timed = |one: &Timed| (Arc::clone(&one.thread.prompt), (kind(&one.thread), one.since));
    Self { since: kept.iter().flat_map(|(kept, _)| &kept.threads).map(timed).collect() }
  }

  /// Each card with its time: the time that it had while the kind of its standing stays, and else
  /// `at`.
  pub(crate) fn timed(&mut self, cards: Vec<view::Card>, at: u64) -> Vec<Timed> {
    let timed = |thread: view::Card| {
      let now = kind(&thread);
      let stands = self.since.entry(Arc::clone(&thread.prompt)).or_insert((now, at));
      if stands.0 != now {
        *stands = (now, at);
      }
      Timed { since: stands.1, thread }
    };
    cards.into_iter().map(timed).collect()
  }
}

/// The kind of a standing whose time a card shows: a thread that works keeps the time of the start
/// of its work while its model and its commands take turns.
fn kind(card: &view::Card) -> u8 {
  match card.standing {
    Standing::Asks { .. } => 0,
    Standing::Failed => 1,
    Standing::Paused { .. } => 2,
    Standing::Thinks | Standing::Runs | Standing::Delegates => 3,
    Standing::Stopped => 4,
    Standing::Rests => 5,
    Standing::Resolved => 6,
  }
}

/// The cards that the daemon told: each peer that asked them, and the cards of each record as the
/// daemon last told them, with the project of the record.
#[derive(Default)]
pub(crate) struct Told {
  watchers: Vec<Outbox>,
  shown: HashMap<String, (Option<String>, Vec<Card>)>,
}

impl Told {
  /// Whether a peer hears the cards. Once none does, the daemon keeps no card.
  pub(crate) fn watched(&mut self) -> bool {
    self.watchers.retain(|one| !one.is_closed());
    if self.watchers.is_empty() {
      self.shown.clear();
    }
    !self.watchers.is_empty()
  }

  /// One more peer that hears each change from now on.
  pub(crate) fn watch(&mut self, peer: Outbox) {
    self.watchers.push(peer);
  }

  /// The project of a record whose cards the daemon told.
  pub(crate) fn project(&self, life: &str) -> Option<&Option<String>> {
    self.shown.get(life).map(|one| &one.0)
  }

  /// How far the index of a record stands, told to each peer that hears the cards.
  pub(crate) fn indexing(&self, life: &str, done: u64) {
    let frame = Frame::Indexing { life: life.to_owned(), done, of: opens::WHOLE };
    for one in &self.watchers {
      one.send(frame.clone()).ok();
    }
  }

  /// The cards of a record as they stand now, told to each peer that hears them when they are not
  /// the cards told before: each card that is new or changed, and each prompt that is gone.
  pub(crate) fn tell(&mut self, life: &str, project: Option<String>, cards: Vec<Card>) {
    let before = self.shown.get(life).map(|one| one.1.as_slice()).unwrap_or_default();
    let changed: Vec<Card> = cards.iter().filter(|one| !before.contains(one)).cloned().collect();
    let stays = |was: &&Card| !cards.iter().any(|one| one.thread.prompt == was.thread.prompt);
    let gone: Vec<Name> =
      before.iter().filter(stays).map(|was| Arc::clone(&was.thread.prompt)).collect();
    if !changed.is_empty() || !gone.is_empty() {
      let frame = Frame::Cards { life: life.to_owned(), changed, gone };
      for one in &self.watchers {
        // A peer that is gone takes no frame, and the next look at the watchers lets it go.
        one.send(frame.clone()).ok();
      }
    }
    self.shown.insert(life.to_owned(), (project, cards));
  }
}
