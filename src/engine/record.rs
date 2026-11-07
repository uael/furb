//! What the entries of a record show of the work that the outside took in earlier lives.

use std::collections::HashSet;

use crate::{
  fact::{self, Fact},
  value::{Object, entry},
};

/// How many lives may end while an act of the outside runs before a later life parks it. Each of
/// those lives ended while the act ran and before it was done, so the act may be what ended them.
pub const CRASHES: usize = 5;

/// The kinds of act that a life parks, which are those that can end the process that runs them
/// again: the run of a word and a reply. A command, a fetch, a search and an act of MCP are none
/// of them, since no later life runs one again: its ear closes it as interrupted at the wake.
const PARKS: [&str; 2] = ["run", "reply"];

/// The acts that the entries of a record show started and not done, by name, in the order of the
/// record, once for each life that started it: work that the outside took in an earlier life. The
/// record keeps the started of the outside alone, since a later life says again what an act said.
pub(crate) fn left(record: &[Object]) -> Vec<String> {
  let facts: Vec<Fact> =
    record.iter().filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of)).collect();
  let done: HashSet<&str> = facts.iter().filter(|a| a.kind() == "done").map(Fact::about).collect();
  let started = facts.iter().filter(|a| a.kind() == "started" && !done.contains(a.about()));
  started.map(|a| a.about().to_owned()).collect()
}

/// The questions that the entries of a record hold, by name: the acts that an earlier life made.
/// The word of such a run ran within the bound of a turn when its life made it, or the record
/// keeps the end that the bound gave it, so a replay steps it with no bound, and a machine under
/// load never ends it otherwise than the record does.
pub(crate) fn settled(record: &[Object]) -> HashSet<String> {
  let facts = record.iter().filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of));
  facts.filter(Fact::question).map(|a| a.about().to_owned()).collect()
}

/// The acts that the entries of a record park, each by its name and its kind, in the order of the
/// record: an act of a kind that [`PARKS`] names, which the record shows started in [`CRASHES`]
/// lives or more and not done. The outside says an act started once in each life that takes it, and
/// the journal keeps each such started.
pub(crate) fn looping(record: &[Object]) -> Vec<(String, String)> {
  let mut lives: Vec<(String, usize)> = Vec::new();
  for id in left(record) {
    match lives.iter_mut().find(|(one, _)| *one == id) {
      Some((_, n)) => *n += 1,
      None => lives.push((id, 1)),
    }
  }
  let parks = |id: String| {
    PARKS.iter().find(|kind| fact::named(&id, kind)).map(|kind| (id, (*kind).to_owned()))
  };
  lives.into_iter().filter(|(_, n)| *n >= CRASHES).filter_map(|(id, _)| parks(id)).collect()
}
