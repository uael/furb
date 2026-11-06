//! The facts that the record of a life does not hold yet, as on a full disk, which the life holds
//! in memory and never drops: the one home of the bound on them, and of the rule that a life whose
//! record lacks some of its facts does not end. Past the bound, the life pauses its work on the
//! root, which covers each chain, and tells the operator once, with the folder of the record and
//! the space left on its disk. Once the record holds each fact, the life wakes the work that it
//! paused, and a pause that the operator set stays.

use std::path::Path;

use furb::{Engine, Fault, Object, verbs, wire::outward};

/// How many facts the record of a life may lack before the life pauses its work, which bounds the
/// memory that holds them: twice the facts after which a life is born again, so a life that only
/// waits for its rebirth never meets it.
pub(crate) const UNKEPT: usize = 20_000;

/// Why the life paused its root.
const WHY: &str = "the record of this life does not hold its facts yet";

/// Whether the life paused its root for its record.
#[derive(Default)]
pub(crate) struct Backlog {
  paused: bool,
}

impl Backlog {
  /// Whether the life holds its work until its record takes each fact.
  pub(crate) fn holds(&self) -> bool {
    self.paused
  }

  /// The notice of a pause or a wake of the root of a life whose record lacks some of its facts,
  /// when one comes now: a pause past [`UNKEPT`], unless the root is paused already, and a wake of
  /// that pause once the record holds each fact.
  pub(crate) fn weighed(
    &mut self,
    engine: &mut Engine,
    record: &str,
  ) -> Result<Option<String>, Fault> {
    let (count, root) = (engine.unkept(), engine.root().to_owned());
    if self.paused && count == 0 {
      engine.wake(&root)?;
      self.paused = false;
      return Ok(Some(
        "The record of this life holds each of its facts again, so the life wakes the work that it \
         paused."
          .to_owned(),
      ));
    }
    if self.paused || count <= UNKEPT {
      return Ok(None);
    }
    let paused = engine.verb("paused", vec![Object::string(&root)], vec![])?;
    if outward(paused.as_ref()).as_bool() == Some(true) {
      return Ok(None);
    }
    engine.pause(&root, verbs::Pause { why: Some(Object::string(WHY)) })?;
    self.paused = true;
    let folder = Path::new(record).parent().unwrap_or(Path::new("."));
    let left = furb::world::free(folder)
      .map_or_else(|| "an unknown space".to_owned(), |free| format!("{} MB", free / 1_000_000));
    Ok(Some(format!(
      "The record of this life does not hold {count} facts, more than {UNKEPT}, so the life pauses \
       its work and holds each fact in memory: {} has {left} left. Give the disk room. The life \
       wakes its work once the record holds them.",
      folder.display()
    )))
  }
}

/// Why a life does not end yet while its record does not hold a number of the facts that it said.
pub(crate) fn unkept(count: usize) -> String {
  format!(
    "The life does not end yet: its record does not hold {count} facts, which an end now loses. \
     Give the disk of the record room. The life ends once the record holds them."
  )
}
