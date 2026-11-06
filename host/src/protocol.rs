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

/// What a peer is: a window of an operator, the process of a skill, or the daemon of a new build,
/// which asks the handover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
  Window,
  Skill,
  Daemon,
}

/// The first frame of a peer: the protocol that it speaks, its kind, the name of a skill, and the
/// role that it asks. The daemon gives the peer the lesser of that role and the role that its
/// transport proved, so a peer asks `read` to be a reader on any transport. With no role, the peer
/// has the role that its transport proved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Hello {
  pub protocol: u32,
  pub kind: Kind,
  pub name: Option<String>,
  #[serde(default)]
  pub role: Option<Role>,
}

/// A peer as the daemon knows it after its hello: the login that its transport proved, its role,
/// its kind, and the name of a skill.
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
  /// The records of the machine: each record of the sessions of each workspace that
  /// `workspaces.json` in the folder `config` names, the config folder of the daemon when the peer
  /// names none, as a window keeps its list there, and each record that a life of the daemon holds.
  List {
    #[serde(default)]
    config: Option<String>,
  },
  /// The life of a record opened, in a folder or in the folder of its record. A life that this
  /// open makes stands on the actor, the roster and the extensions that the peer names, reads the
  /// settings of the user in the folder `config` and runs the claude command line `claude` when
  /// the peer names them, as the window of the peer reads them, and the peer answers each request
  /// of its models in their place when it asks to, as a demo does.
  /// With `attach`, the peer attaches the life in the same step, from that place in its log, so
  /// the life has a holder before its grace can pass.
  Open {
    life: String,
    cwd: Option<String>,
    actor: Option<String>,
    roster: Option<Vec<String>>,
    extensions: Option<bool>,
    #[serde(default)]
    config: Option<String>,
    #[serde(default)]
    claude: Option<String>,
    #[serde(default)]
    answers: bool,
    #[serde(default)]
    attach: Option<Seq>,
  },
  /// The events of a life from a place in its log, which the daemon pushes to the peer.
  Attach {
    life: String,
    from: Seq,
  },
  Detach {
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
  /// The share of the window of its model that the prompt of an answer filled, as `filled` of
  /// the engine gives it from the reply that took that answer, and nothing before an answer.
  Filled {
    life: String,
    act: String,
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
  /// A pause of an act or of a chain, with the cause that the operator gives.
  Pause {
    life: String,
    act: String,
    why: Option<String>,
  },
  Wake {
    life: String,
    act: String,
  },
  /// A cancel of an act or of a chain, with the cause that the operator gives.
  Cancel {
    life: String,
    act: String,
    why: Option<String>,
  },
  /// A chain made on a chain, under a label, or a fork of the chain `source` with its transcript
  /// and its program.
  Chain {
    life: String,
    label: Option<String>,
    source: Option<String>,
    on: Option<String>,
  },
  /// A word that runs on a chain as a turn of the operator.
  Run {
    life: String,
    word: String,
    on: Option<String>,
  },
  /// A close of an act with a value, written as the record writes a value: `2.0` stays a float,
  /// and `{"is": "Refused", "args": ["why"]}` is a refusal.
  Close {
    life: String,
    act: String,
    value: String,
  },
  /// A ceiling of the spend of a chain, in dollars or in a share of its window.
  Grant {
    life: String,
    usd: Option<f64>,
    share: Option<f64>,
    on: Option<String>,
  },
  /// A command that runs on a chain, with the text of its stdin, or with its stdin kept open for
  /// the lines that a write to `<act>/stdin` feeds it.
  Bash {
    life: String,
    command: String,
    stdin: Option<String>,
    #[serde(default)]
    feed: bool,
    on: Option<String>,
  },
  /// The text at a path, read where a chain stands, as an act of the operator.
  Read {
    life: String,
    path: String,
    on: Option<String>,
  },
  /// A text written at a path, where a chain stands, as an act of the operator.
  Write {
    life: String,
    path: String,
    content: String,
    on: Option<String>,
  },
  /// The names that the module of a chain binds.
  Names {
    life: String,
    on: Option<String>,
  },
  /// What the gate finds in a word on a chain, which runs nothing.
  Gate {
    life: String,
    word: String,
    on: Option<String>,
  },
  /// A stand of the life, which reads the roster and the actor again.
  Stand {
    life: String,
  },
  /// The configs of the folder of the life, in their layers: what each key holds, a change of a
  /// key, or a stand on them as the disk holds them now.
  Config {
    life: String,
    config: Config,
  },
  /// The fact that the operator read a chain or a thread up to now, which each window and each
  /// later life reads in the record.
  Seen {
    life: String,
    about: String,
  },
  /// A follow-up that the operator queued on a chain, under an entry that the window chose. The
  /// life sends it as a prompt of its shape and its message once the chain rests, and a second
  /// queue of the same entry, from any window, changes nothing.
  Queue {
    life: String,
    entry: String,
    shape: String,
    message: String,
    on: Option<String>,
  },
  /// A follow-up taken out of its queue before the life sent it.
  Unqueue {
    life: String,
    entry: String,
  },
  /// The follow-ups that wait on a chain, in order.
  Queued {
    life: String,
    on: Option<String>,
  },
  /// What waits for the operator in a life, with each notice that the window still shows.
  Desk {
    life: String,
    #[serde(default)]
    notices: Vec<String>,
  },
  /// The threads of a chain with their meaning, now or as they stood just after a moment. `read`
  /// is the place up to which the window read the chain, from which the runs of notes count the
  /// ones not yet read; none reads nothing.
  Threads {
    life: String,
    on: Option<String>,
    at: Option<u64>,
    #[serde(default)]
    read: Option<u64>,
  },
  /// The acts of a life as they stood just after a moment, each with its meaning then.
  AsOf {
    life: String,
    at: u64,
  },
  /// Each moment of a life at which an act began or ended, in order.
  Moments {
    life: String,
  },
  /// The turn that the peer gave for a request of a model of a life that it answers, written as
  /// the record writes a value, or why it gives none.
  Answered {
    life: String,
    ask: u64,
    turn: Option<Value>,
    why: Option<String>,
  },
  /// What the peer added to the text and to the thought of the turn of a request, as it writes.
  Wrote {
    life: String,
    ask: u64,
    text: String,
    thinking: String,
  },
  /// The place of the daemon given to the daemon of a new build: each life written to its
  /// checkpoint and ended, and the lives with their pins in the answer, once the daemon stopped to
  /// listen.
  Handover,
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
        | Self::List { .. }
        | Self::Attach { .. }
        | Self::Detach { .. }
        | Self::Presence { .. }
        | Self::Get { .. }
        | Self::Peek { .. }
        | Self::Transcript { .. }
        | Self::Turns { .. }
        | Self::Program { .. }
        | Self::Standing { .. }
        | Self::Filled { .. }
        | Self::Cwd { .. }
        | Self::Inspect { .. }
        | Self::Pending { .. }
        | Self::Queued { .. }
        | Self::Desk { .. }
        | Self::Threads { .. }
        | Self::AsOf { .. }
        | Self::Moments { .. }
        | Self::Names { .. }
        | Self::Gate { .. }
        | Self::Config { config: Config::Get, .. }
        | Self::ViewMount(_)
        | Self::ViewTree(_)
        | Self::ViewEffect(_)
        | Self::ViewDispose(_)
    );
    if reads { Role::Read } else { Role::Write }
  }

  /// The kind of peer that says the verb, when it is a frame of a view: a window mounts, says what
  /// a person did and disposes, and a skill serves the trees and the effects. Any other peer is
  /// refused it.
  pub fn speaker(&self) -> Option<Kind> {
    match self {
      Self::ViewMount(_) | Self::ViewEvent(_) | Self::ViewDispose(_) => Some(Kind::Window),
      Self::ViewTree(_) | Self::ViewEffect(_) => Some(Kind::Skill),
      _ => None,
    }
  }
}

/// What a peer asks of the configs of a life, which the World writes, so no window writes a disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Config {
  /// Each key of the configs with the value that wins for it and its layer.
  Get,
  /// A key of a layer, the project when none is named, set to a value, or taken out with no
  /// value, then a stand of the life on the configs. A change that the catalog refuses puts its
  /// config back as it was, and the life does not stand.
  Set { key: Vec<String>, value: Option<Value>, layer: Option<Layer> },
  /// A stand of the life on the configs as the disk holds them, which asks again each model that
  /// waits to retry.
  Reload,
}

/// A layer of the configs, from the one that wins least to the one that wins most: the config of
/// the user, the config of the project, and its local config, which git ignores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
  User,
  Project,
  Local,
}

impl From<Layer> for furb::world::config::Layer {
  fn from(layer: Layer) -> Self {
    match layer {
      Layer::User => Self::User,
      Layer::Project => Self::Project,
      Layer::Local => Self::Local,
    }
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
  /// Who is attached to a life now, pushed to each of its windows after each attach and detach.
  Presence { life: String, peers: Vec<Peer> },
  /// A life that holds open acts and said no fact for the bound of the daemon, pushed once to each
  /// of its windows by the watch of the daemon, which no word of the life runs: each open act, and
  /// the notice that a window shows as it stands.
  Stalled { life: String, acts: Vec<Open>, notice: String },
  /// A request of a model of a life that the peer answers: its number, and the request with the
  /// actor, the chain, the messages and the settings.
  Ask { life: String, ask: u64, request: Value },
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
  /// The lives that a daemon gave at its handover.
  Handed(Vec<Handed>),
}

/// An act that stays open in a life that stalled: its name, its kind, and the chain that it stands
/// in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Open {
  pub act: String,
  pub kind: String,
  pub chain: String,
}

/// A life that a daemon gave at its handover: its record, its folder, and the holder of each pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Handed {
  pub life: String,
  pub cwd: String,
  pub pins: Vec<String>,
}

/// A record of the machine: the main checkout of the workspace whose sessions hold it, when the
/// list names one, whether a life of the daemon holds it now, and whether that life lives and does
/// not end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Listed {
  pub life: String,
  pub workspace: Option<String>,
  pub held: bool,
  pub open: bool,
}

/// A view that a window mounts in a slot of a life, which the process of a skill serves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Mount {
  pub viewer: String,
  pub life: String,
  pub slot: Slot,
  pub skill: String,
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

/// What a skill asks of the one window of a view.
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

/// Where a window shows a view. The dock follows the chain that it names, and the row, the detail
/// and the mark follow the chain of the act that they name. The sidebar, the footer, a screen and
/// an overlay follow the life.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "slot", rename_all = "snake_case")]
pub enum Slot {
  Dock {
    on: String,
  },
  /// A section of the sidebar.
  Sidebar,
  /// The row of one act, of a kind that the skill draws.
  Row {
    act: String,
  },
  /// The detail of one act, of a kind that the skill draws.
  Detail {
    act: String,
  },
  /// A mark in the feed, after one act, of a kind that the skill marks.
  Mark {
    act: String,
  },
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

/// What a skill asks of a window.
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

impl Node {
  /// The node as a window that only reads gets it: each node that acts, inert.
  #[must_use]
  pub fn inert(self) -> Node {
    let shape = match self.shape {
      Shape::Input { value, hint, .. } => Shape::Input { value, hint, inert: true },
      Shape::Button { label, .. } => Shape::Button { label, inert: true },
      Shape::Box { axis, gap, border, children } => {
        Shape::Box { axis, gap, border, children: inerts(children) }
      }
      Shape::List { items } => Shape::List { items: inerts(items) },
      Shape::Table { header, rows } => {
        Shape::Table { header, rows: rows.into_iter().map(inerts).collect() }
      }
      shape @ (Shape::Text { .. }
      | Shape::Progress { .. }
      | Shape::Code { .. }
      | Shape::Diff { .. }
      | Shape::Markdown { .. }) => shape,
    };
    Node { shape, ..self }
  }
}

fn inerts(nodes: Vec<Node>) -> Vec<Node> {
  nodes.into_iter().map(Node::inert).collect()
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
    /// A node that acts is inert in the window of a peer that only reads: it draws and takes no
    /// gesture.
    #[serde(default)]
    inert: bool,
  },
  Button {
    label: String,
    #[serde(default)]
    inert: bool,
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
