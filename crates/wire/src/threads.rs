//! The threads of a chain as a window draws them: each thread with its agents, its asides, the
//! runs of its notes, and what they say to the operator.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::State;

/// One thread of a chain: its prompt, its name, its messages, the prompts to the operator that it
/// made and the questions of them that wait, and its state. A thread is over once its prompt
/// and each of its messages is done, and a message to a thread that is over opens it again until
/// its model closes the message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChainThread {
  pub prompt: String,
  /// The name of the thread: the label of its chain for the first thread that no operator made on
  /// a chain under the root, the name of the session for the first thread of the operator on the
  /// root, and else the first line of its message that is not empty.
  pub title: String,
  pub messages: Vec<String>,
  pub asked: Vec<String>,
  pub asks: Vec<String>,
  pub state: State,
  pub over: bool,
  pub origin: Origin,
  /// Whether the operator started it, himself or by a word of his own.
  pub yours: bool,
}

/// Who made the prompt of a thread: the operator, a rung of a word, or an ear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
  Operator,
  Rung,
  Ear,
}

/// A thread with the threads that its ladder made, its agents. Its state is its own, or asks while
/// a question waits in an agent under it: a failure or an unread in an agent does not roll up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ThreadNode {
  pub thread: ChainThread,
  pub agents: Vec<Self>,
  pub state: State,
}

/// A chain of a life in time: its name, its label, the chain whose word made it, none for the
/// root, the state of its work now, each moment at which an act of it began or ended, and each
/// thread that it holds with the moment that thread began, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChainTrack {
  pub chain: String,
  pub label: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parent: Option<String>,
  pub state: State,
  pub moments: Vec<u64>,
  pub threads: Vec<Began>,
}

/// A thread of a chain and the moment its prompt began.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Began {
  pub prompt: String,
  pub at: u64,
}

/// How a thread concerns the operator: a question waits for him in it, or he started it and it
/// failed or a pause holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Concern {
  Asks,
  Failed,
  Paused,
  /// A thread that he started ended after the place up to which he saw it.
  Unread,
}

/// What a tree of threads says to the operator, counted by thread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Attention {
  pub asks: usize,
  pub failed: usize,
  pub paused: usize,
  pub unread: usize,
}

/// What a prompt of a chain is that is no conversation, which the feed of its chain draws as what
/// it is and the list of its threads leaves out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "aside", content = "from", rename_all = "snake_case")]
pub enum Aside {
  /// A notice of the round: a prompt of None that a rung or a chain made, which no one answers, as
  /// the acknowledgment of an act that stands in no thread of the chain, a stall that the watch of
  /// the daemon tells, or the question of the act that names the session.
  Notice,
  /// What a word of the operator told the chain: a prompt of None that a rung of his made, or the
  /// acknowledgment of a rung that stands in no thread.
  Rung,
  /// A wake of the chain: a prompt of None that an ear or a verb of the round made.
  Wake,
  /// A question of another chain to this one, a worker that asks its coordinator, with the chain
  /// that asks.
  Question(String),
  /// The work of an agent: a prompt that a rung made on a chain under its own, when that rung
  /// stands in no thread, as a command of the operator runs its word. It is no thread of the
  /// operator and never takes an unread mark of his.
  Agent,
}

/// A prompt of a chain that is no thread, with what it is and the prompts on it, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChainAside {
  pub prompt: String,
  pub aside: Aside,
  pub messages: Vec<String>,
}

/// A run of notes: the asides of a chain that are no question and follow one another in its feed
/// with no card of another kind between them, which a window draws as one block. `unread` counts
/// the notes that began or ended after the place up to which the operator read the chain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NoteRun {
  pub prompts: Vec<String>,
  pub count: usize,
  pub unread: usize,
}

/// The threads of one chain as a window draws them: the tree of its threads, its asides, the runs
/// of its notes, what the tree says to the operator, and the state of the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Threads {
  pub threads: Vec<ThreadNode>,
  pub asides: Vec<ChainAside>,
  pub notes: Vec<NoteRun>,
  pub attention: Attention,
  pub state: State,
}
