//! A card of a thread that the operator started, as a list of threads shows it, with the rule of
//! its part and of its place in that list.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
  act::{OPERATOR, operator},
  state::State,
};

/// One thread that the operator started, as a card of a list shows it: its prompt, its title, how
/// it stands, the model that wrote its last word, how many agents work under it, and the site of
/// each operator who read it to its end, `operator` for each of them and `operator/<login>` for
/// one member. A question of an agent under it is a question of the card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Card {
  pub prompt: String,
  pub title: String,
  pub standing: Standing,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub model: Option<String>,
  #[serde(default)]
  pub agents: usize,
  #[serde(default)]
  pub readers: Vec<String>,
}

/// What an operator sent to a life: a message, or a word of Python, with the site that sent it,
/// `operator` or `operator/<login>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Sent {
  pub by: String,
  pub text: String,
  #[serde(default)]
  pub python: bool,
}

/// A skill that the root of a life was told of, by the last read of `skills://` there: its name,
/// and what it is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Skill {
  pub name: String,
  pub description: String,
}

/// How the thread of a card stands. It asks one member by his login, or each operator. A pause
/// names who said it. A thread whose model closed its prompt is resolved, and one that a cancel
/// ended is stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "is", rename_all = "snake_case")]
pub enum Standing {
  Asks { of: Option<String> },
  Failed,
  Paused { by: String },
  Thinks,
  Runs,
  Delegates,
  Stopped,
  Rests,
  Resolved,
}

/// The part of a list of cards that a card stands in, in the order of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Part {
  NeedsYou,
  Working,
  Resting,
  Resolved,
}

impl Standing {
  /// How a thread stands from its state, whether it is over, and who said the pause that holds
  /// it. A question asks each operator.
  pub fn of(state: State, over: bool, paused_by: String) -> Self {
    match state {
      State::Asks => Self::Asks { of: None },
      State::Failed => Self::Failed,
      State::Paused => Self::Paused { by: paused_by },
      State::Thinks => Self::Thinks,
      State::Runs => Self::Runs,
      State::Delegates => Self::Delegates,
      State::Cancelled => Self::Stopped,
      State::Done | State::Unread | State::Idle if over => Self::Resolved,
      State::Done | State::Unread | State::Idle => Self::Rests,
    }
  }
}

impl Card {
  /// Whether a member read the thread to its end.
  pub fn read_by(&self, member: &str) -> bool {
    let reads = |site: &String| {
      site == OPERATOR
        || site.strip_prefix(OPERATOR).and_then(|one| one.strip_prefix('/')) == Some(member)
    };
    self.readers.iter().any(reads)
  }

  /// Whether the thread works.
  pub fn works(&self) -> bool {
    matches!(self.standing, Standing::Thinks | Standing::Runs | Standing::Delegates)
  }

  /// Whether the thread ended: it is resolved, stopped or failed, and a message of the operator
  /// opens it again.
  pub fn ended(&self) -> bool {
    matches!(self.standing, Standing::Resolved | Standing::Stopped | Standing::Failed)
  }

  /// The card of a thread whose record no life holds: nothing works there, so a thread that worked
  /// rests, and no agent of it works.
  #[must_use]
  pub fn at_rest(mut self) -> Self {
    if self.works() {
      self.standing = Standing::Rests;
    }
    self.agents = 0;
    self
  }

  /// Whether the thread failed or ended after what a member read of it.
  pub fn unread(&self, member: &str) -> bool {
    matches!(self.standing, Standing::Failed | Standing::Resolved) && !self.read_by(member)
  }

  /// Whether the thread waits for a member: it asks him or each operator, it failed and he did not
  /// read it, or a pause holds it that no operator said. An end that he did not read waits for
  /// nobody: it rests, with its mark, whether a life holds the session or not.
  pub fn needs(&self, member: &str) -> bool {
    match &self.standing {
      Standing::Asks { of } => of.as_deref().is_none_or(|of| of == member),
      Standing::Paused { by } => !operator(by),
      Standing::Failed => self.unread(member),
      _ => false,
    }
  }

  /// The part of the list where a member finds the card, and its rank in that part: in what needs
  /// him, a question stands before a failure and a pause. A thread that ended and that he did not
  /// read rests.
  pub fn part(&self, member: &str) -> (Part, u8) {
    let needs = self.needs(member);
    match &self.standing {
      Standing::Asks { .. } if needs => (Part::NeedsYou, 0),
      Standing::Failed | Standing::Paused { .. } if needs => (Part::NeedsYou, 1),
      Standing::Resolved if self.unread(member) => (Part::Resting, 0),
      Standing::Resolved => (Part::Resolved, 0),
      _ if self.works() => (Part::Working, 0),
      _ => (Part::Resting, 0),
    }
  }
}
