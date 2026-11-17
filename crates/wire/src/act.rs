//! An act with its meaning, and what its models cost.

use std::{fmt, sync::Arc};

use schemars::JsonSchema;
use serde::{
  Deserialize, Deserializer, Serialize,
  de::{self, Visitor},
};
use serde_json::Value;

use crate::{page::Viewed, state::State};

/// The name of an act or of a chain. A name is made once from its fact and shared: each place that
/// holds it holds the same text.
pub type Name = Arc<str>;

/// A name read from the text that a frame holds, made in one allocation: serde reads an `Arc<str>`
/// through a `String` that it then copies.
pub(crate) fn read_name<'de, D: Deserializer<'de>>(from: D) -> Result<Name, D::Error> {
  struct Text;
  impl Visitor<'_> for Text {
    type Value = Name;
    fn expecting(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
      out.write_str("a name")
    }
    fn visit_str<E: de::Error>(self, text: &str) -> Result<Name, E> {
      Ok(Name::from(text))
    }
  }
  from.deserialize_str(Text)
}

/// A name in a list or an option, read as [`read_name`] reads it.
#[repr(transparent)]
struct Named(Name);

impl<'de> Deserialize<'de> for Named {
  fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
    read_name(from).map(Self)
  }
}

/// A list of names, each read in one allocation; the list keeps the place that it was read into.
pub(crate) fn read_names<'de, D: Deserializer<'de>>(from: D) -> Result<Vec<Name>, D::Error> {
  Vec::<Named>::deserialize(from).map(|names| names.into_iter().map(|one| one.0).collect())
}

/// A name that may be absent, read in one allocation.
pub(crate) fn read_some_name<'de, D: Deserializer<'de>>(from: D) -> Result<Option<Name>, D::Error> {
  Option::<Named>::deserialize(from).map(|name| name.map(|one| one.0))
}

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

/// The name of the operator in the engine, which `engine::OPERATOR` of the crate furb holds too,
/// as a test of that crate proves.
pub const OPERATOR: &str = "operator";

/// The refusal of an act that a life ended before the act did.
pub fn interruption(id: &str) -> String {
  format!("{id} was interrupted: the life that ran it ended before it did")
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
    #[serde(alias = "login", alias = "profile")]
    operator: String,
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
  /// The maker as a person reads it at the head of a block: a model by its name with no provider
  /// and its effort, as `opus/high`, else its role.
  pub fn shown(&self) -> String {
    match self {
      Self::Model { model, effort } => {
        let name = named(model);
        effort.as_ref().map_or_else(|| name.to_owned(), |effort| format!("{name}/{effort}"))
      }
      Self::Operator | Self::Member { .. } => "operator".to_owned(),
      Self::Skill => "skill".to_owned(),
      Self::Engine => "furb".to_owned(),
    }
  }
}

/// One act of a life with its meaning, as it stands after the last fact about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Act {
  #[schemars(with = "String")]
  pub kind: Kind,
  #[serde(deserialize_with = "read_name")]
  pub id: Name,
  #[serde(deserialize_with = "read_name")]
  pub by: Name,
  /// Who made the act: for a rung that waits for the word of an actor, that actor; for any
  /// other act, the role of the act or the person that made it.
  #[serde(default)]
  pub maker: Maker,
  /// What the act is on: a chain, or the prompt of a thread for a prompt on a thread.
  #[serde(deserialize_with = "read_name")]
  pub on: Name,
  /// The chain that the act stands in.
  #[serde(deserialize_with = "read_name")]
  pub chain: Name,
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
  /// Who closed or cancelled the act, as a window names who made an act, once a close or a cancel
  /// of it was said.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub closer: Option<Maker>,
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

/// A pause, as its fact holds it: the act or the chain that it names, which a wake names to end it,
/// who said it, and the cause that it gave.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Pause {
  #[serde(default)]
  pub of: String,
  pub by: String,
  pub why: Option<Value>,
}

/// How the word of a rung ran: it runs or waits for its word, it ran, or a fault or a cancel ended
/// it, each with what a person reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "run", content = "reason", rename_all = "snake_case")]
pub enum RunState {
  Running,
  Done,
  Failed(String),
  Cancelled(String),
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

/// What the answers of models cost: their dollars, how many they are, and the actor of the last
/// and what it read.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Spend {
  pub dollars: f64,
  pub answers: u32,
  pub actor: Option<String>,
  /// The account of the last answer, none when that answer named none.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub account: Option<String>,
  /// What the last answer read.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub read: Option<Read>,
}

/// What an answer of a model read: the tokens of its whole prompt, and of them those that it read
/// from the cache and those that it wrote to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Read {
  pub tokens: u64,
  pub cached: u64,
  pub written: u64,
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
    self.read = Some(Read::of(&answer.usage));
  }
}

impl Read {
  /// What the usage of an answer says that it read.
  #[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the usage holds each count of tokens, a whole number of at least 0, as a float"
  )]
  fn of(usage: &[f64; 5]) -> Self {
    let [tokens, _, cached, written, _] = *usage;
    Self { tokens: tokens as u64, cached: cached as u64, written: written as u64 }
  }
}

impl Pause {
  /// The pause that a fact of a pause says, and none for any other fact.
  pub fn of(fact: &Value) -> Option<Self> {
    (fact[0] == "pause").then(|| Self {
      of: fact[1].as_str().unwrap_or_default().to_owned(),
      by: fact[2].as_str().unwrap_or_default().to_owned(),
      why: Some(fact[3].clone()).filter(|why| !why.is_null()),
    })
  }
}

/// A model by its name with no provider, as `opus` for `claude-cli:opus`.
fn named(model: &str) -> &str {
  model.rsplit(':').next().unwrap_or(model)
}

/// Why a word failed, as a person reads it: a refusal of furb says its reason alone, with no name
/// of its class, and a model that answered nothing says it as the head of its block names it, then
/// the cause with no name of the class of the error of its provider.
pub fn reason(why: &str) -> String {
  let why = why.strip_prefix("Refused: ").unwrap_or(why);
  match why.split_once(" answered nothing: ") {
    Some((actor, cause)) => {
      let cause = cause.strip_prefix("ProviderError: ").unwrap_or(cause);
      format!("{} answered nothing: {cause}", named(actor))
    }
    None => why.to_owned(),
  }
}

/// The class and the arguments of a value that is a fault, as the engine gives one.
pub fn fault(value: &Value) -> Option<(&str, &[Value])> {
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
    (self.kind == Kind::Prompt && self.on != self.chain).then_some(self.on.as_ref())
  }

  /// Whether the act is a prompt to the operator: a question, or a view that a word draws.
  pub fn to_operator(&self) -> bool {
    self.kind == Kind::Prompt && self.words.get(2).and_then(Value::as_str).is_some_and(operator)
  }

  /// Whether the act asks the operator a question: a prompt to him that draws no view.
  pub fn asks_operator(&self) -> bool {
    self.to_operator() && self.view().is_none()
  }

  /// The view that the act draws: a prompt to the operator whose message is one.
  pub fn view(&self) -> Option<Viewed> {
    let message = self.words.get(1).and_then(Value::as_str).filter(|_| self.to_operator())?;
    Viewed::of(&self.id, message)
  }

  /// Whether the operator owes the act an answer: a question that his console took, and that is
  /// not done. A view waits for a gesture that no one owes.
  pub fn owed(&self) -> bool {
    self.operator && !self.done() && self.view().is_none()
  }

  /// Whether the act failed: a rung whose run raised, or an act done with a fault that is no
  /// cancel. An act that the end of the life that ran it cut, as a command, did not fail: its ear
  /// closed it with the refusal that says so, and the operator has nothing to act on.
  pub fn failed(&self) -> bool {
    match &self.run {
      Some(run) => matches!(run, RunState::Failed(_)),
      None => self.value().and_then(fault).is_some_and(|(is, args)| {
        let cut = matches!(args, [Value::String(why)] if *why == interruption(&self.id));
        is != "CancelledError" && !cut
      }),
    }
  }

  /// Whether a cancel ended the act. A cancel is what the operator asked for, so it is no failure.
  pub fn cancelled(&self) -> bool {
    match &self.run {
      Some(run) => matches!(run, RunState::Cancelled(_)),
      None => self.value().and_then(fault).is_some_and(|(is, _)| is == "CancelledError"),
    }
  }

  /// Whether an operator took the act back: a cancel that he said of the act itself ended it, and
  /// not a cancel of a prompt above it, as the stop of its thread.
  pub fn taken_back(&self) -> bool {
    self.cancelled() && matches!(self.closer, Some(Maker::Operator | Maker::Member { .. }))
  }
}
