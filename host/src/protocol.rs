//! The one protocol of the host: what a peer says to the daemon, and what the daemon says to a
//! peer, each frame one line of JSON.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
  error::ProtocolError,
  event::{Event, Seq},
};

/// The version of the protocol. A peer of another version is refused at its hello.
pub const PROTOCOL: u32 = 1;

/// The most bytes that one frame holds. A frame past it closes the peer.
pub const BOUND: usize = 16 << 20;

/// What a peer may do: read a life, or read and write it. Write holds each right of read.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Role {
  Read,
  Write,
}

/// What a peer is: a window of an operator, or the process of a kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
  Window,
  Kit,
}

/// The first frame of a peer: the protocol that it speaks, its kind, and the name of a kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Hello {
  pub protocol: u32,
  pub kind: Kind,
  pub name: Option<String>,
}

/// A peer as the daemon knows it after its hello: the login that its transport proved, its role,
/// its kind, and the name of a kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Peer {
  pub login: String,
  pub role: Role,
  pub kind: Kind,
  pub name: Option<String>,
}

/// One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
/// `act`, since `id` is the key of the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Request {
  pub id: u64,
  #[serde(flatten)]
  pub verb: Verb,
}

/// Each thing that a peer may ask the daemon. A life is named by the path of its record, and a
/// verb that names no chain says it on the root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum Verb {
  Hello(Hello),
  /// The lives and the records of the machine.
  List,
  /// The life of a record opened, in a folder or in the folder of its record.
  Open {
    life: String,
    cwd: Option<String>,
  },
  /// The events of a life from a place in its log, which the daemon pushes to the peer.
  Attach {
    life: String,
    from: Seq,
  },
  Detach {
    life: String,
  },
  End {
    life: String,
  },
  Pin {
    life: String,
    holder: String,
  },
  Unpin {
    life: String,
    holder: String,
  },
  /// Who is attached to a life now.
  Presence {
    life: String,
  },
  Get {
    life: String,
    act: String,
  },
  Peek {
    life: String,
    act: String,
  },
  Transcript {
    life: String,
    on: Option<String>,
    of: Option<String>,
  },
  Turns {
    life: String,
    on: Option<String>,
  },
  Program {
    life: String,
    on: Option<String>,
  },
  Standing {
    life: String,
  },
  Cwd {
    life: String,
    on: Option<String>,
  },
  Inspect {
    life: String,
    name: String,
    on: Option<String>,
  },
  /// The prompts of a life that wait for an operator.
  Pending {
    life: String,
  },
  Prompt {
    life: String,
    shape: String,
    message: String,
    to: Option<String>,
    on: Option<String>,
  },
  /// The close of a prompt to the operator, with a line that the operator wrote.
  Answer {
    life: String,
    act: String,
    line: String,
  },
  Pause {
    life: String,
    act: String,
  },
  Wake {
    life: String,
    act: String,
  },
  Cancel {
    life: String,
    act: String,
  },
  Chain {
    life: String,
    label: Option<String>,
    on: Option<String>,
  },
  /// A word that runs on a chain as a turn of the operator.
  Run {
    life: String,
    word: String,
    on: Option<String>,
  },
  #[serde(rename = "view.mount")]
  ViewMount(Mount),
  #[serde(rename = "view.tree")]
  ViewTree(Tree),
  #[serde(rename = "view.event")]
  ViewEvent(ViewEvent),
  #[serde(rename = "view.effect")]
  ViewEffect(ViewEffect),
  #[serde(rename = "view.dispose")]
  ViewDispose(Dispose),
}

impl Verb {
  /// The least role that says the verb. A verb that this list does not name needs the role to
  /// write.
  pub fn needs(&self) -> Role {
    let reads = matches!(
      self,
      Self::Hello(_)
        | Self::List
        | Self::Attach { .. }
        | Self::Detach { .. }
        | Self::Presence { .. }
        | Self::Get { .. }
        | Self::Peek { .. }
        | Self::Transcript { .. }
        | Self::Turns { .. }
        | Self::Program { .. }
        | Self::Standing { .. }
        | Self::Cwd { .. }
        | Self::Inspect { .. }
        | Self::Pending { .. }
        | Self::ViewMount(_)
        | Self::ViewTree(_)
        | Self::ViewEvent(_)
        | Self::ViewEffect(_)
        | Self::ViewDispose(_)
    );
    if reads { Role::Read } else { Role::Write }
  }
}

/// What the daemon says to a peer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum Frame {
  /// The answer to the request of an id.
  Reply { id: u64, answer: Answer },
  /// A refusal of the request of an id, or of a frame with no id that could be read, with the
  /// text that a person reads.
  Refused { id: Option<u64>, error: ProtocolError, why: String },
  /// One event of a life that the peer attached, under its place in the log of the life.
  Event { life: String, seq: Seq, event: Event },
  #[serde(rename = "view.mount")]
  ViewMount(Mount),
  #[serde(rename = "view.tree")]
  ViewTree(Tree),
  #[serde(rename = "view.event")]
  ViewEvent(ViewEvent),
  #[serde(rename = "view.effect")]
  ViewEffect(ViewEffect),
  #[serde(rename = "view.dispose")]
  ViewDispose(Dispose),
}

impl Frame {
  /// The refusal of a request, or of a frame with no id.
  pub fn refused(id: Option<u64>, error: ProtocolError) -> Self {
    Frame::Refused { id, why: error.to_string(), error }
  }
}

/// What a verb comes to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "answer", content = "data", rename_all = "snake_case")]
pub enum Answer {
  Done,
  /// The answer to a hello: the protocol of the daemon, and the peer as the daemon knows it.
  Welcome {
    protocol: u32,
    peer: Peer,
  },
  Lives(Vec<Listed>),
  /// The name of the act that the verb made.
  Act(String),
  Value(Value),
  Values(Vec<Value>),
  /// A name of a module: the name of its type, its representation, and its value as JSON.
  Inspected {
    kind: String,
    repr: String,
    value: Value,
  },
  Presence(Vec<Peer>),
}

/// A record of the machine, and whether a life of the daemon holds it now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Listed {
  pub life: String,
  pub open: bool,
}

/// A view that a window mounts in a slot, which the process of a kit serves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Mount {
  pub viewer: String,
  pub slot: Slot,
  pub kit: String,
}

/// The tree of a view, which takes the place of each tree of a lower version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tree {
  pub viewer: String,
  pub slot: Slot,
  pub version: u64,
  pub tree: Node,
}

/// What a person did to a node of a view, with the value of an input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ViewEvent {
  pub viewer: String,
  pub node: String,
  pub kind: Gesture,
  pub value: Option<String>,
}

/// What a kit asks of the one window of a view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ViewEffect {
  pub viewer: String,
  pub effect: Effect,
}

/// The end of a view in its window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Dispose {
  pub viewer: String,
}

/// Where a window shows a view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "slot", rename_all = "snake_case")]
pub enum Slot {
  Dock,
  Sidebar,
  /// The row of each act of a kind.
  Row {
    kind: String,
  },
  /// The detail of each act of a kind.
  Detail {
    kind: String,
  },
  Mark,
  Footer,
  Screen,
  Overlay,
}

/// What a person does to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
  Press,
  Change,
  Submit,
}

/// What a kit asks of a window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum Effect {
  Open { url: String },
  Draft { text: String },
  Notice { text: String },
}

/// One node of a view, under an id that an event names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Node {
  pub id: String,
  #[serde(flatten)]
  pub shape: Shape,
}

/// The closed set of the nodes of a view, which each window draws in its own way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "node", rename_all = "snake_case")]
pub enum Shape {
  Box {
    axis: Axis,
    gap: u16,
    border: bool,
    children: Vec<Node>,
  },
  Text {
    runs: Vec<Run>,
  },
  List {
    items: Vec<Node>,
  },
  Table {
    header: Vec<String>,
    rows: Vec<Vec<Node>>,
  },
  Input {
    value: String,
    hint: Option<String>,
  },
  Button {
    label: String,
  },
  /// The share of the work that is done, from 0 to 1, or none while it is not known.
  Progress {
    done: Option<f64>,
  },
  Code {
    text: String,
    language: Option<String>,
  },
  /// A diff in the unified form.
  Diff {
    text: String,
  },
  Markdown {
    text: String,
  },
}

/// The way a box lays out its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
  Row,
  Column,
}

/// A run of a text, in a tone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Run {
  pub text: String,
  #[serde(default)]
  pub tone: Tone,
}

/// The tone of a run, which each window gives its own color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
  #[default]
  Text,
  Muted,
  Faint,
  Accent,
  Success,
  Warning,
  Danger,
}

/// Both ways of the protocol, which its schema holds.
#[derive(JsonSchema)]
#[schemars(title = "Protocol")]
#[expect(dead_code, reason = "only its schema is read")]
struct Wire {
  request: Request,
  frame: Frame,
}

/// The JSON Schema of the protocol: a request and a frame, with the version of the protocol under
/// the key `x-protocol`.
pub fn schema() -> Value {
  let generator = schemars::generate::SchemaSettings::draft07().into_generator();
  let mut schema = generator.into_root_schema_for::<Wire>();
  schema.insert("x-protocol".to_owned(), PROTOCOL.into());
  schema.to_value()
}
