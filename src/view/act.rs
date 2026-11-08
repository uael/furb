//! An act with its meaning, and what its models cost.

use std::fmt;

use crate::{engine::OPERATOR, world::catalog::leveled};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::state::State;

/// The kind of an act, read once from its fact: each kind that the view reads by name, and any
/// other as its text. It serializes to that text, so a window reads the string of the fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Kind {
  Chain,
  Rung,
  Prompt,
  Reply,
  Run,
  Bash,
  Wait,
  Stand,
  Other(String),
}

impl Kind {
  /// The kind as the fact writes it.
  pub fn as_str(&self) -> &str {
    match self {
      Self::Chain => "chain",
      Self::Rung => "rung",
      Self::Prompt => "prompt",
      Self::Reply => "reply",
      Self::Run => "run",
      Self::Bash => "bash",
      Self::Wait => "wait",
      Self::Stand => "stand",
      Self::Other(kind) => kind,
    }
  }

  /// Whether an act of the kind is work which ends later.
  pub fn work(&self) -> bool {
    matches!(self, Self::Prompt | Self::Rung | Self::Bash | Self::Wait)
  }
}

impl From<&str> for Kind {
  fn from(kind: &str) -> Self {
    match kind {
      "chain" => Self::Chain,
      "rung" => Self::Rung,
      "prompt" => Self::Prompt,
      "reply" => Self::Reply,
      "run" => Self::Run,
      "bash" => Self::Bash,
      "wait" => Self::Wait,
      "stand" => Self::Stand,
      other => Self::Other(other.to_owned()),
    }
  }
}

impl From<String> for Kind {
  fn from(kind: String) -> Self {
    Self::from(kind.as_str())
  }
}

impl From<Kind> for String {
  fn from(kind: Kind) -> Self {
    match kind {
      Kind::Other(kind) => kind,
      known => known.as_str().to_owned(),
    }
  }
}

impl fmt::Display for Kind {
  fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
    out.write_str(self.as_str())
  }
}

/// The speaker of the watch of the daemon, which tells a chain that stalled: a notice that no one
/// of the operator made.
pub const WATCH: &str = "watch";

/// Whether a name is the operator: [`OPERATOR`] itself, or one of his sessions under it.
pub fn operator(name: &str) -> bool {
  name.strip_prefix(OPERATOR).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// Who made an act, as a window names it: the operator, a member who answers as him by his login,
/// a model with its effort when its actor names one, a skill, or the engine itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Maker {
  Operator,
  Member {
    login: String,
  },
  Model {
    model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
  },
  Skill,
  #[default]
  Engine,
}

impl Maker {
  /// The maker that an actor names, as a chain or a reply names it: the operator, a member when
  /// the login after `operator/` is one of these members, else the operator in one of his
  /// sessions; a model else, read as [`leveled`] reads it, unless the models hold the whole
  /// name. No actor is the engine.
  pub fn of(actor: &str, models: &[String], members: &[String]) -> Self {
    let model = |model: &str, effort: Option<&str>| Self::Model {
      model: model.to_owned(),
      effort: effort.map(str::to_owned),
    };
    match actor {
      "" => Self::Engine,
      _ if operator(actor) => {
        match actor.strip_prefix(OPERATOR).and_then(|rest| rest.strip_prefix('/')) {
          Some(login) if members.iter().any(|one| one == login) => {
            Self::Member { login: login.to_owned() }
          }
          _ => Self::Operator,
        }
      }
      _ if models.iter().any(|one| one == actor) => model(actor, None),
      _ => {
        let (name, effort) = leveled(actor);
        model(name, effort)
      }
    }
  }
}

/// One act of a life with its meaning, as it stands after the last fact about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Act {
  #[schemars(with = "String")]
  pub kind: Kind,
  pub id: String,
  pub by: String,
  /// Who made the act: for a rung that waits for the word of an actor, that actor; for any
  /// other act, the role of the act or the person that made it.
  #[serde(default)]
  pub maker: Maker,
  /// What the act is on: a chain, or the prompt of a thread for a prompt on a thread.
  pub on: String,
  /// The chain that the act stands in.
  pub chain: String,
  /// The words of the act after its kind, its name, its maker and what it is on.
  pub words: Vec<Value>,
  pub phase: Phase,
  /// Whether the console of the operator took the act: a question to him, or a notice.
  pub operator: bool,
  /// The pause that holds the act while it lives.
  pub pause: Option<Pause>,
  /// How the word of a rung ran, for a rung alone.
  pub run: Option<RunState>,
  /// The word of a rung, once its chain took it to run or to refuse.
  pub word: Option<String>,
  /// Whether a reply of a rung runs, while its model writes its word. A rung with no word and no
  /// reply that runs waits for its turn.
  #[serde(default)]
  pub asking: bool,
  /// The messages of the operator that a chain with a source took from it, in their order: each
  /// prompt of the operator that the prefix of the chain carries, with the id it has on the source.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub taken: Vec<String>,
  /// How many facts the life had said when the act was made, which is its place in time.
  pub began: u64,
  /// How many facts the life had said when the act was done.
  pub ended: Option<u64>,
  /// What the models cost: the answers that wrote the word of a rung, or each answer on a chain.
  pub spend: Option<Spend>,
  /// The state of the act, or of the work of a chain for a chain.
  pub state: State,
}

/// Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "phase", content = "value", rename_all = "snake_case")]
pub enum Phase {
  Asked,
  Started,
  Done(Value),
}

/// A pause, as its fact holds it: who said it, and the cause that it gave.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Pause {
  pub by: String,
  pub why: Option<Value>,
}

/// How the word of a rung ran: it runs or waits for its word, it ran, a fault or a cancel ended it,
/// or the gate refused it, each with what a person reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "run", content = "reason", rename_all = "snake_case")]
pub enum RunState {
  Running,
  Done,
  Failed(String),
  Cancelled(String),
  /// The findings of the gate, one for each line.
  Refused(String),
}

/// One answer of a model: the actor whose reply it answered, and its usage, which is the tokens
/// that it read and wrote, the tokens read from and written to the cache, and its dollars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Answer {
  pub actor: String,
  pub usage: [f64; 5],
  /// The account that served the reply, by its name alone, when the provider names one.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub account: Option<String>,
}

/// What the answers of models cost: their dollars, how many they are, and the actor of the last.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Spend {
  pub dollars: f64,
  pub answers: u32,
  pub actor: Option<String>,
  /// The account of the last answer, none when that answer named none.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub account: Option<String>,
}

impl Spend {
  /// What answers cost, in order.
  pub fn of<'a>(answers: impl IntoIterator<Item = &'a Answer>) -> Self {
    let mut spend = Self::default();
    for one in answers {
      spend.add(one);
    }
    spend
  }

  /// One more answer.
  pub fn add(&mut self, answer: &Answer) {
    self.dollars += answer.usage[4];
    self.answers += 1;
    self.actor = Some(answer.actor.clone());
    self.account.clone_from(&answer.account);
  }
}

impl Pause {
  /// The pause that a fact of a pause says, and none for any other fact.
  pub fn of(fact: &Value) -> Option<Self> {
    (fact[0] == "pause").then(|| Self {
      by: fact[2].as_str().unwrap_or_default().to_owned(),
      why: Some(fact[3].clone()).filter(|why| !why.is_null()),
    })
  }
}

/// The class and the arguments of a value that is a fault, as the engine gives one.
pub(crate) fn fault(value: &Value) -> Option<(&str, &[Value])> {
  Some((value.get("is")?.as_str()?, value.get("args")?.as_array()?.as_slice()))
}

impl Act {
  /// Whether the act is done.
  pub fn done(&self) -> bool {
    matches!(self.phase, Phase::Done(_))
  }

  /// What the act came to, once it is done.
  pub fn value(&self) -> Option<&Value> {
    match &self.phase {
      Phase::Done(value) => Some(value),
      _ => None,
    }
  }

  /// The thread that a prompt on a thread is on: the prompt that it is on.
  pub fn thread(&self) -> Option<&str> {
    (self.kind == Kind::Prompt && self.on != self.chain).then_some(self.on.as_str())
  }

  /// Whether the act asks the operator: a prompt to him.
  pub fn asks_operator(&self) -> bool {
    self.kind == Kind::Prompt && self.words.get(2).and_then(Value::as_str).is_some_and(operator)
  }

  /// Whether the operator owes the act an answer: a prompt that his console took, and that is not
  /// done.
  pub fn owed(&self) -> bool {
    self.operator && !self.done()
  }

  /// Whether the act failed: a rung that the gate refused or whose run raised, or an act done with
  /// a fault that is no cancel.
  pub fn failed(&self) -> bool {
    match &self.run {
      Some(run) => matches!(run, RunState::Failed(_) | RunState::Refused(_)),
      None => self.value().and_then(fault).is_some_and(|(is, _)| is != "CancelledError"),
    }
  }

  /// Whether a cancel ended the act. A cancel is what the operator asked for, so it is no failure.
  pub fn cancelled(&self) -> bool {
    match &self.run {
      Some(run) => matches!(run, RunState::Cancelled(_)),
      None => self.value().and_then(fault).is_some_and(|(is, _)| is == "CancelledError"),
    }
  }
}
