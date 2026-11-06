//! The name of a session, folded from the acts of its root: the value of the last `rename_session`
//! that is done, which the operator said and which may be empty to take the default name again,
//! else the value of the last `name_session` that is done with a name, which the extension naming
//! gave.

use std::collections::HashMap;

use serde_json::Value;

use super::{act::Act, operator};

/// The root chain of a life, which the session names.
const ROOT: &str = "chain1";

/// The kind of the act by which the operator names a session.
pub const RENAME: &str = "rename_session";
/// The kind of the act by which the extension naming names a session.
pub const NAME: &str = "name_session";

/// The word of a rung of the operator that names a session, with the name in JSON between its two
/// parts.
const WORD: (&str, &str) = ("act(\"rename_session\", \"\", lambda id: (yield \"done\", id, ", "))");

/// The name that a session has, from the acts of its root as they come: the name that the operator
/// said and the name that the extension naming gave, each the last of its kind by the place that
/// its act began.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Named {
  said: Option<(u64, String)>,
  given: Option<(u64, String)>,
  /// The kind and the place of each act on the root that names the session and is not done, for a
  /// fold of facts.
  asked: HashMap<String, (bool, u64)>,
  /// How many facts the fold of facts heard.
  heard: u64,
}

impl Named {
  /// The name of the session: the one that the operator said, else the one that the extension gave,
  /// and nothing when the root holds neither. An empty name says that the operator took the default
  /// name again.
  pub fn name(&self) -> Option<&str> {
    self.said().or_else(|| self.given())
  }

  /// The name that the operator said last.
  pub fn said(&self) -> Option<&str> {
    self.said.as_ref().map(|one| one.1.as_str())
  }

  /// The name that the extension naming gave last.
  pub fn given(&self) -> Option<&str> {
    self.given.as_ref().map(|one| one.1.as_str())
  }

  /// The name that the acts of a chain give, as the fold of facts gives it: the last
  /// `rename_session` that is done with a text, and the last `name_session` that is done with a
  /// name, each by the place where its act began.
  pub fn of_acts(on: &[&Act]) -> Self {
    let mut named = Self::default();
    for act in on {
      let said = match act.kind.as_str() {
        RENAME => true,
        NAME => false,
        _ => continue,
      };
      if let Some(value) = act.value().and_then(Value::as_str) {
        named.take(said, act.began, value);
      }
    }
    named
  }

  /// A fact of a life, in the order of its log: an act on the root that names the session, and its
  /// done. With `words`, the word of a rung of the operator on the root that names the session says
  /// the name too, as the record keeps it when it keeps no done of the act that the word makes.
  pub fn hear(&mut self, fact: &Value, words: bool) -> bool {
    self.heard += 1;
    let text = |at: usize| fact.get(at).and_then(Value::as_str).unwrap_or_default();
    let (kind, about) = (text(0), text(1));
    match kind {
      "done" => match (self.asked.remove(about), fact.get(3)) {
        (Some((said, began)), Some(Value::String(value))) => self.take(said, began, value),
        _ => false,
      },
      RENAME | NAME if text(3) == ROOT && crate::fact::named(about, kind) => {
        self.asked.insert(about.to_owned(), (kind == RENAME, self.heard));
        false
      }
      "rung" if words && operator(text(2)) && text(3) == ROOT => match said(text(4)) {
        Some(value) => self.take(true, self.heard, &value),
        None => false,
      },
      _ => false,
    }
  }

  /// A name of a kind, which takes the place of the one before it unless that one began later.
  fn take(&mut self, said: bool, began: u64, value: &str) -> bool {
    if !said && value.is_empty() {
      return false;
    }
    let held = if said { &mut self.said } else { &mut self.given };
    if held.as_ref().is_some_and(|(at, one)| *at > began || (*at == began && one == value)) {
      return false;
    }
    *held = Some((began, value.to_owned()));
    true
  }
}

/// The name that the word of a rung says, when it is the word that names a session.
pub fn said(word: &str) -> Option<String> {
  let inner = word.strip_prefix(WORD.0)?.strip_suffix(WORD.1)?;
  serde_json::from_str::<String>(inner).ok()
}

/// The word that names a session with a name, as the operator says it: the word of `renameWord` in
/// tui/src/names.ts, which the TUI sends. The crate builds it for its tests alone.
#[cfg(test)]
pub fn word(name: &str) -> String {
  format!("{}{}{}", WORD.0, Value::String(name.to_owned()), WORD.1)
}

#[cfg(test)]
#[path = "named.test.rs"]
mod tests;
