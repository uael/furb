//! An act with its meaning, and what its models cost.

use std::fmt;

use crate::engine::OPERATOR;
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
  Other(String),
}

impl Kind {
  /// The kind as the fact writes it.
  pub fn as_str(&self) -> &str {
    match self {
      Kind::Chain => "chain",
      Kind::Rung => "rung",
      Kind::Prompt => "prompt",
      Kind::Reply => "reply",
      Kind::Run => "run",
      Kind::Bash => "bash",
      Kind::Wait => "wait",
      Kind::Other(kind) => kind,
    }
  }

  /// Whether an act of the kind is work which ends later.
  pub fn work(&self) -> bool {
    matches!(self, Kind::Prompt | Kind::Rung | Kind::Bash | Kind::Wait)
  }
}

impl From<&str> for Kind {
  fn from(kind: &str) -> Kind {
    match kind {
      "chain" => Kind::Chain,
      "rung" => Kind::Rung,
      "prompt" => Kind::Prompt,
      "reply" => Kind::Reply,
      "run" => Kind::Run,
      "bash" => Kind::Bash,
      "wait" => Kind::Wait,
      other => Kind::Other(other.to_owned()),
    }
  }
}

impl From<String> for Kind {
  fn from(kind: String) -> Kind {
    Kind::from(kind.as_str())
  }
}

impl From<Kind> for String {
  fn from(kind: Kind) -> String {
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

/// Whether a name is the operator: [`OPERATOR`] itself, or one of his sessions under it.
pub fn operator(name: &str) -> bool {
  name.strip_prefix(OPERATOR).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// One act of a life with its meaning, as it stands after the last fact about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Act {
  #[schemars(with = "String")]
  pub kind: Kind,
  pub id: String,
  pub by: String,
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
}

/// What the answers of models cost: their dollars, how many they are, and the actor of the last.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Spend {
  pub dollars: f64,
  pub answers: u32,
  pub actor: Option<String>,
}

impl Spend {
  /// What answers cost, in order.
  pub fn of<'a>(answers: impl IntoIterator<Item = &'a Answer>) -> Spend {
    let mut spend = Spend::default();
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
  }
}

impl Pause {
  /// The pause that a fact of a pause says, and none for any other fact.
  pub fn of(fact: &Value) -> Option<Pause> {
    (fact[0] == "pause").then(|| Pause {
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
