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
  fs::{self, File},
  io::{self, Read, Seek, SeekFrom},
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use furb::{
  Fact, Fault, Object,
  view::{self, Fold, Held, Standing},
  world,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  ears::json_of,
  protocol::{Card, Frame},
  serve::Outbox,
};

/// How many bytes of the end of a record its tip reads.
const TAIL: u64 = 4096;

/// The cards of one session, and the name of the session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Session {
  #[serde(default)]
  pub(crate) name: Option<String>,
  #[serde(default)]
  pub(crate) threads: Vec<Timed>,
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

/// The tip of a record: its length, and the sha256 of its last bytes. A record only grows, so
/// each entry that it takes changes both.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Tip {
  bytes: u64,
  tail: String,
}

impl Tip {
  fn of(record: &Path) -> io::Result<Self> {
    let mut file = File::open(record)?;
    let bytes = file.metadata()?.len();
    file.seek(SeekFrom::Start(bytes.saturating_sub(TAIL)))?;
    let mut tail = Vec::new();
    file.take(TAIL).read_to_end(&mut tail)?;
    let tail = format!("{:x}", Sha256::digest(&tail));
    Ok(Self { bytes, tail })
  }
}

/// The second of the epoch now.
pub(crate) fn now() -> u64 {
  seconds(SystemTime::now())
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

/// The session that the index of a record keeps, or none when the record has no index or its tip
/// is not the tip that the index names. It reads the index and the end of the record, and no
/// more of the record.
pub(crate) fn kept(record: &Path) -> Option<Session> {
  let indexed: Indexed = serde_json::from_slice(&fs::read(index(record)).ok()?).ok()?;
  (Tip::of(record).ok()? == indexed.tip).then_some(indexed.session)
}

/// The index of a record written from a session, at the tip of the record now. It is written
/// whole under another name and then moved, so no reader reads a part of it.
pub(crate) fn keep(record: &Path, session: &Session) -> io::Result<()> {
  let indexed = Indexed { tip: Tip::of(record)?, session: session.clone() };
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

/// The index of a record made from a fold of the record with no life, and kept. Each of its
/// threads stands since the record last changed, since a record tells no time.
pub(crate) fn folded(record: &Path) -> Result<(), Error> {
  let since = changed(record);
  let retold = world::retold(record).map_err(Error::Read)?;
  let (mut fold, mut held, mut at) = (Fold::default(), Held::default(), 0);
  let mut take = |one: Object| {
    let Some(fact) = Fact::of(one.as_ref()) else { return };
    let fact = json_of(&fact);
    at += 1;
    held.noted(&fact, at);
    for act in fold.recorded(&fact) {
      held.took(act);
    }
  };
  retold.each(&mut take).map_err(Error::Read)?;
  let name = retold.name().map(str::to_owned);
  let threads = held.cards().into_iter().map(|thread| Timed { since, thread }).collect();
  Ok(keep(record, &Session { name, threads })?)
}

/// The one card of a record whose threads no index tells yet: no prompt, a name, and at rest.
pub(crate) fn unfolded(name: String) -> view::Card {
  view::Card {
    prompt: String::new(),
    title: name,
    standing: Standing::Rests,
    model: None,
    agents: 0,
    readers: Vec::new(),
  }
}

/// The times of the cards of a life, which the life keeps as its cards change.
#[derive(Default)]
pub(crate) struct Times {
  /// The kind of the standing of each thread, and since when it stands so.
  since: HashMap<String, (u8, u64)>,
}

impl Times {
  /// The times that the index of a record keeps, and none for a record with no index.
  pub(crate) fn kept(record: Option<&str>) -> Self {
    let kept = record.and_then(|one| kept(Path::new(one)));
    let timed = |one: &Timed| (one.thread.prompt.clone(), (kind(&one.thread), one.since));
    Self { since: kept.iter().flat_map(|kept| &kept.threads).map(timed).collect() }
  }

  /// Each card with its time: the time that it had while the kind of its standing stays, and else
  /// `at`.
  pub(crate) fn timed(&mut self, cards: Vec<view::Card>, at: u64) -> Vec<Timed> {
    let timed = |thread: view::Card| {
      let now = kind(&thread);
      let stands = self.since.entry(thread.prompt.clone()).or_insert((now, at));
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

  /// The cards of a record as they stand now, told to each peer that hears them when they are not
  /// the cards told before: each card that is new or changed, and each prompt that is gone.
  pub(crate) fn tell(&mut self, life: &str, project: Option<String>, cards: Vec<Card>) {
    let before = self.shown.get(life).map(|one| one.1.as_slice()).unwrap_or_default();
    let changed: Vec<Card> = cards.iter().filter(|one| !before.contains(one)).cloned().collect();
    let stays = |was: &&Card| !cards.iter().any(|one| one.thread.prompt == was.thread.prompt);
    let gone: Vec<String> =
      before.iter().filter(stays).map(|was| was.thread.prompt.clone()).collect();
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
