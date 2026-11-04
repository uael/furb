//! The one set of states of the work of a life, which an act, a thread, a chain and a session each
//! take.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::act::{Act, Kind, RunState};

/// The state of work: it asks the operator, it ended while the operator looked away, it failed, a
/// pause holds it, its model thinks or writes its word, a word or a command of it runs, its word
/// awaits a prompt on another chain, it ended well, a cancel ended it, or it rests. The order is
/// the order of the roll-up. Only a window says unread, since only it knows what its operator saw.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum State {
  Asks,
  Unread,
  Failed,
  Paused,
  Thinks,
  Runs,
  Delegates,
  Done,
  Cancelled,
  Idle,
}

impl State {
  /// Each state, in the order of the roll-up.
  pub const ALL: [State; 10] = [
    State::Asks,
    State::Unread,
    State::Failed,
    State::Paused,
    State::Thinks,
    State::Runs,
    State::Delegates,
    State::Done,
    State::Cancelled,
    State::Idle,
  ];

  /// The state of a whole: the first state that any of its parts holds, and idle for no part.
  pub fn roll_up(states: impl IntoIterator<Item = State>) -> State {
    states.into_iter().min().unwrap_or(State::Idle)
  }

  /// Whether the state is work that is not done: a model that thinks, a word or a command that
  /// runs, or a word that awaits another chain.
  pub fn busy(self) -> bool {
    matches!(self, State::Thinks | State::Runs | State::Delegates)
  }

  /// The state of one act. A rung whose word has not come yet is its model that thinks, and an act
  /// that is no work and lives, as a chain or a grant, is work that waits on another chain.
  pub fn of(act: &Act) -> State {
    let done = act.done();
    if act.owed() {
      State::Asks
    } else if act.pause.is_some() && !done {
      State::Paused
    } else if act.cancelled() {
      State::Cancelled
    } else if act.failed() {
      State::Failed
    } else if !done && !act.kind.work() {
      State::Delegates
    } else if done && (act.kind != Kind::Rung || act.run != Some(RunState::Running)) {
      State::Done
    } else if act.kind == Kind::Rung && !done && act.word.is_none() {
      State::Thinks
    } else {
      State::Runs
    }
  }
}

/// What the work of a chain that is not done does, from its own acts and the prompts that its words
/// made on other chains: a model that writes a word of it thinks, a command or a wait of it runs,
/// and a word of it that awaits only another chain delegates.
pub fn working(local: &[&Act], remote: &[&Act]) -> State {
  let mut open = local.iter().filter(|act| !act.done());
  if open.clone().any(|act| act.kind == Kind::Rung && act.word.is_none()) {
    State::Thinks
  } else if open.any(|act| matches!(act.kind, Kind::Bash | Kind::Wait)) {
    State::Runs
  } else if remote.iter().any(|act| !act.done()) {
    State::Delegates
  } else {
    State::Runs
  }
}

/// The state of a chain, from the acts that stand in it, in the order they began, and the prompts
/// that its words made on other chains. It asks while the operator owes a prompt of it, a pause
/// holds it while it holds a live act of work, it works while one lives, and else its last act of
/// work says whether a cancel or a fault ended it. A chain whose prompts all closed is done, and
/// one with no prompt rests.
pub fn chain_state(on: &[&Act], remote: &[&Act]) -> State {
  if on.iter().any(|act| act.owed()) {
    return State::Asks;
  }
  let live: Vec<&Act> = on.iter().copied().filter(|act| act.kind.work()).collect();
  if live.iter().any(|act| act.pause.is_some() && !act.done()) {
    return State::Paused;
  }
  if live.iter().any(|act| !act.done()) {
    return working(&live, remote);
  }
  match live.last() {
    Some(last) if last.cancelled() => State::Cancelled,
    Some(last) if last.failed() => State::Failed,
    _ if live.iter().any(|act| act.kind == Kind::Prompt) => State::Done,
    _ => State::Idle,
  }
}

/// The state of each act, and of each chain from the acts that stand in it.
pub fn restate(acts: &mut [Act]) {
  let ids: HashMap<&str, &Act> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
  let states: Vec<State> = acts
    .iter()
    .map(|act| {
      if act.kind != Kind::Chain {
        return State::of(act);
      }
      let on: Vec<&Act> =
        acts.iter().filter(|one| one.chain == act.id && one.id != act.id).collect();
      let remote: Vec<&Act> = acts
        .iter()
        .filter(|one| one.kind == Kind::Prompt && one.chain != act.id)
        .filter(|one| ids.get(one.by.as_str()).is_some_and(|maker| maker.chain == act.id))
        .collect();
      chain_state(&on, &remote)
    })
    .collect();
  for (act, state) in acts.iter_mut().zip(states) {
    act.state = state;
  }
}
