//! Who else is on a session: each other operator whose window holds its life, and the thread where
//! he writes, as the machine pushes the presence of the life; and where the operator of this
//! client writes, which the client tells the life so each other window shows it.

use furb_wire::protocol::{Draft, Present, Verb};

use crate::{Asked, Client, Target};

impl Client {
  /// Each other operator whose window holds a life of a machine, by his login.
  pub fn others(&self, at: usize, life: &str) -> Vec<&Present> {
    let Some(told) = self.machines.get(at) else { return Vec::new() };
    let all = told.presence.get(life).into_iter().flatten();
    all.filter(|one| one.operator != told.login).collect()
  }

  /// Where the operator writes now: the life of the target of the input, and the thread of it,
  /// none for a new thread, while the input holds a message. A word of Python and a new session
  /// are no draft that another window shows.
  fn writing(&self) -> Option<(usize, String, Option<String>)> {
    if self.draft.python || self.draft.text.trim().is_empty() {
      return None;
    }
    match self.target() {
      Target::Thread(key) => Some((key.0, key.1, Some(key.2))),
      Target::Session(at, life) => Some((at, life, None)),
      Target::New(..) => None,
    }
  }

  /// The tell of where the operator writes, when it changed since the client last told it: the
  /// end of the draft in the life that it left first, then the draft in the life of the target.
  pub(crate) fn writes_due(&mut self) -> Option<(usize, Verb, Asked)> {
    let now = self.writing();
    let told = self.wrote.clone();
    let said = match (&told, &now) {
      (Some((at, life, _)), _) if now.as_ref().is_none_or(|one| one.0 != *at || one.1 != *life) => {
        (*at, life.clone(), None)
      }
      (_, Some((at, life, on))) if told.as_ref().is_none_or(|one| one.2 != *on) => {
        (*at, life.clone(), Some(on.clone()))
      }
      _ => return None,
    };
    let (at, life, on) = said;
    self.wrote = on.clone().map(|on| (at, life.clone(), on));
    let draft = on.map(|on| Draft { on });
    Some((at, Verb::Writes { life, draft }, Asked::Writes))
  }
}
