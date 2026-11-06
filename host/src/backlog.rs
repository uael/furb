//! The facts that the record of a life does not hold yet, as on a full disk, which the life holds
//! in memory and never drops: the one home of the bound on them, and of the rule that a life whose
//! record lacks some of its facts does not end. Past the bound, the life pauses each chain that
//! lives, and each chain that opens while it holds, and tells the operator once, with the folder of
//! the record and the space left on its disk. Once the record holds each fact, the life wakes each
//! chain that it paused and that no later pause or wake of the operator holds, and a pause that the
//! operator set, before or after its own, stays.

use std::path::Path;

use furb::{Engine, Fault, Object, verbs, wire::outward};

/// How many facts the record of a life may lack before the life pauses its work, which bounds the
/// memory that holds them: twice the facts after which a life is born again, so a life that only
/// waits for its rebirth never meets it.
pub(crate) const UNKEPT: usize = 20_000;

/// Why the life paused a chain.
const WHY: &str = "the record of this life does not hold its facts yet";

/// Whether the life holds its work for its record, and each chain that it paused for it.
#[derive(Default)]
pub(crate) struct Backlog {
  holding: bool,
  paused: Vec<String>,
}

impl Backlog {
  /// Whether the life holds its work until its record takes each fact.
  pub(crate) fn holds(&self) -> bool {
    self.holding
  }

  /// The notice of a hold or a wake of the work of a life whose record lacks some of its facts,
  /// when one comes now: a hold past [`UNKEPT`], which pauses each chain that lives and no pause
  /// covers, then each chain that opens while it holds, and a wake of each chain that it paused
  /// once the record holds each fact.
  pub(crate) fn weighed(
    &mut self,
    engine: &mut Engine,
    record: &str,
  ) -> Result<Option<String>, Fault> {
    let count = engine.unkept();
    if self.holding && count == 0 {
      for chain in std::mem::take(&mut self.paused) {
        if last_ours(engine, &chain)? {
          engine.wake(&chain)?;
        }
      }
      self.holding = false;
      return Ok(Some(
        "The record of this life holds each of its facts again, so the life wakes the work that it \
         paused."
          .to_owned(),
      ));
    }
    if !self.holding && count <= UNKEPT {
      return Ok(None);
    }
    let told = !self.holding;
    self.holding = true;
    self.paused_each(engine)?;
    if !told {
      return Ok(None);
    }
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

  /// Each chain that lives and that no pause covers, paused for the record.
  fn paused_each(&mut self, engine: &mut Engine) -> Result<(), Fault> {
    for n in 1.. {
      let chain = format!("chain{n}");
      if engine.get(&chain)?.is_none() {
        return Ok(());
      }
      if self.paused.contains(&chain) || engine.outcome(&chain)?.is_some() {
        continue;
      }
      let paused = engine.verb("paused", vec![Object::string(&chain)], vec![])?;
      if outward(paused.as_ref()).as_bool() == Some(true) {
        continue;
      }
      engine.pause(&chain, verbs::Pause { why: Some(Object::string(WHY)) })?;
      self.paused.push(chain);
    }
    Ok(())
  }
}

/// Whether the last pause or wake of a chain is the pause of the life for its record, so a pause or
/// a wake that the operator said after it stays.
fn last_ours(engine: &mut Engine, chain: &str) -> Result<bool, Fault> {
  let of = verbs::Transcript { on: Some(chain.to_owned()), of: Some("pause".to_owned()) };
  let facts = engine.transcript(of)?;
  let last = facts.iter().map(|one| outward(one.0.as_ref())).rfind(|fact| fact[1] == chain);
  Ok(last.is_some_and(|fact| fact[0] == "pause" && fact[3] == WHY))
}

/// Why a life does not end yet while its record does not hold a number of the facts that it said.
pub(crate) fn unkept(count: usize) -> String {
  format!(
    "The life does not end yet: its record does not hold {count} facts, which an end now loses. \
     Give the disk of the record room. The life ends once the record holds them."
  )
}
