//! The acts of a life in time: the moments at which an act began or ended, and the acts as they
//! stood at one of them.

use std::collections::BTreeSet;

use super::{
  act::{Act, Phase, RunState},
  state::restate,
};

/// The acts of a life as they stood just after a moment: each act that began by then, and an act
/// that ended after it as one that runs, with no value and no pause, each in the state that this
/// gives.
pub fn as_of<'a>(acts: impl IntoIterator<Item = &'a Act>, at: u64) -> Vec<Act> {
  let mut then: Vec<Act> = acts
    .into_iter()
    .filter(|act| act.began <= at)
    .map(|act| match act.ended {
      Some(ended) if ended > at => Act {
        phase: Phase::Started,
        ended: None,
        pause: None,
        run: act.run.as_ref().map(|_| RunState::Running),
        ..act.clone()
      },
      _ => act.clone(),
    })
    .collect();
  restate(&mut then);
  then
}

/// The moments of a life, in order: each start and each end of an act.
pub fn moments_of<'a>(acts: impl IntoIterator<Item = &'a Act>) -> Vec<u64> {
  let moments: BTreeSet<u64> =
    acts.into_iter().flat_map(|act| [Some(act.began), act.ended]).flatten().collect();
  moments.into_iter().filter(|moment| *moment > 0).collect()
}
