//! The stamp of a dump: what makes the session of a life valid to restore.
//!
//! A dump holds the engine and the stand-in as they ran, so it is valid only for the engine and the build of the
//! crate that made it. It is the life that `boot` would replay from one record, so it is valid only for that record:
//! as many entries, and the same last one. The stamp says these parts as lines of text before the session, and a
//! restore reads them back and refuses a dump whose parts are not the ones it stands on now.

use sha2::{Digest, Sha256};

use crate::{
  ENGINE,
  value::{Fault, ObjectRef, entry},
};

/// The build of the crate, which build.rs hashes from every file the crate is compiled from but the engine.
const BUILD: &str = env!("FURB_BUILD");

/// The first line of every dump.
const MARK: &str = "furb dump";

/// Where a record stands: how many entries it holds, and the kind and the name of the fact of its last entry.
///
/// The kind and the name are text on every side of the wire, where the rest of an entry may come back as another
/// shape of the same value, so they are what the stamp keeps of the last entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
  entries: u64,
  last: Option<(String, String)>,
}

impl Tally {
  /// One more entry, which the World was told to keep.
  pub(crate) fn kept(&mut self, one: ObjectRef<'_>) {
    self.entries += 1;
    let fact = entry(&one, 0);
    let text = |at: usize| {
      fact.as_ref().and_then(|fact| entry(fact, at)).and_then(|one| one.as_str().map(str::to_owned))
    };
    self.last = Some((text(0).unwrap_or_default(), text(1).unwrap_or_default()));
  }

  fn said(&self) -> String {
    match &self.last {
      Some((kind, about)) if self.entries == 1 => format!("1 entry, {kind} {about}"),
      Some((kind, about)) => format!("{} entries that end with {kind} {about}", self.entries),
      None => "no entry".to_owned(),
    }
  }
}

/// What a dump is stamped with: the engine it holds, the build that made it, and the record it matches.
#[derive(Debug)]
pub(crate) struct Stamp {
  engine: String,
  build: String,
  record: Tally,
}

impl Stamp {
  /// The stamp a dump takes here and now, of a life whose record stands as the tally says.
  pub(crate) fn now(record: Tally) -> Self {
    Stamp { engine: format!("{:x}", Sha256::digest(ENGINE)), build: BUILD.to_owned(), record }
  }

  /// The session, stamped: the lines of the stamp, a blank line, and the session.
  pub(crate) fn stamped(&self, session: &[u8]) -> Result<Vec<u8>, Fault> {
    let (kind, about) = self.record.last.clone().unwrap_or_default();
    if kind.contains('\n') || about.contains('\n') {
      return Err(Fault::refused(
        "the last entry of the record names a line break, which no stamp holds",
      ));
    }
    let head = format!(
      "{MARK}\nengine {}\nbuild {}\nentries {}\nkind {kind}\nabout {about}\n\n",
      self.engine, self.build, self.record.entries
    );
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(session);
    Ok(bytes)
  }

  /// The stamp of a dump, and the session after it.
  pub(crate) fn read(dump: &[u8]) -> Result<(Stamp, &[u8]), Fault> {
    let no = |why: &str| Fault::refused(format!("no dump of a life: {why}"));
    let end =
      dump.windows(2).position(|two| two == b"\n\n").ok_or_else(|| no("it holds no stamp"))?;
    let head = std::str::from_utf8(&dump[..end]).map_err(|_| no("its stamp is no text"))?;
    let mut lines = head.split('\n');
    if lines.next() != Some(MARK) {
      return Err(no("it does not begin with the mark of a dump"));
    }
    let mut field = |name: &str| {
      lines
        .next()
        .and_then(|line| line.strip_prefix(name))
        .and_then(|rest| rest.strip_prefix(' '))
        .map(str::to_owned)
        .ok_or_else(|| no(&format!("its stamp says no {name}")))
    };
    let (engine, build, entries) = (field("engine")?, field("build")?, field("entries")?);
    let (kind, about) = (field("kind")?, field("about")?);
    let entries = entries.parse().map_err(|_| no("its stamp counts no entries"))?;
    let last = (entries > 0).then_some((kind, about));
    Ok((Stamp { engine, build, record: Tally { entries, last } }, &dump[end + 2..]))
  }

  /// Each part of this stamp that is not the part of the stamp a dump takes now, said as it differs.
  pub(crate) fn differs(&self, now: &Stamp) -> Vec<String> {
    let mut parts = Vec::new();
    if self.engine != now.engine {
      parts.push("it holds another engine".to_owned());
    }
    if self.build != now.build {
      parts.push("another build of the crate made it".to_owned());
    }
    if self.record != now.record {
      parts.push(format!(
        "it is of a record of {}, and the record holds {}",
        self.record.said(),
        now.record.said()
      ));
    }
    parts
  }
}

#[cfg(test)]
#[path = "stamp.test.rs"]
mod test;
