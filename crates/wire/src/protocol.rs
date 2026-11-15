//! The one protocol of the host: what a peer says to the daemon, and what the daemon says to a
//! peer, each frame one line of JSON.

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
  Act, ChainTrack, Desk, Page, Rung, Shown, Stage, ThreadNode, Threads, card,
  error::ProtocolError,
  event::{Event, Seq},
};

/// The version of the protocol. A peer of another version is refused at its hello, but for a
/// skill of a version from [`KITS`].
pub const PROTOCOL: u32 = 64;

/// The first version of the protocol whose skills each later daemon takes, since a kit works from
/// the moment that it is written: what the process of a skill says keeps its meaning from that
/// version on, or the daemon maps it, and `host/tests/kits` holds what a skill of each version
/// says and reads.
pub const KITS: u32 = 44;

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

/// A peer as the daemon knows it after its hello: its operator, the login that its transport
/// proved, its role, its kind, and the name of a skill. No window is named apart from its
/// operator, so two windows of one operator are one operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Peer {
  pub operator: String,
  pub role: Role,
  pub kind: Kind,
  pub name: Option<String>,
}

/// An operator whose window holds a life now: the operator, the greatest role of his windows,
/// the draft that he writes, when one of his windows writes one, and how many of his windows hold
/// the life, so a window knows whether it is the last view of the life.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Present {
  pub operator: String,
  pub role: Role,
  pub writes: Option<Draft>,
  #[serde(default = "one")]
  pub windows: usize,
}

/// One window, which a peer of a version before the count of the windows of an operator means.
fn one() -> usize {
  1
}

/// Where a person writes a draft: in a thread, or for a new thread when it names none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Draft {
  pub on: Option<String>,
}

/// The folders of a life that a kit reads: the folder of its project, the config folder of its
/// user, and the folder of its images, which a kit writes an image to as a turn names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Folders {
  pub project: String,
  pub config: String,
  pub images: Option<String>,
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
#[serde(tag = "verb", rename_all = "snake_case", deny_unknown_fields)]
pub enum Verb {
  Hello(Hello),
  /// The records of the machine: each record of the sessions of each project that
  /// `workspaces.json` in the folder `config` names, the config folder of the daemon when the peer
  /// names none, as a window keeps its list there, and each record that a life of the daemon holds.
  List {
    #[serde(default)]
    config: Option<String>,
  },
  /// The projects of the records that `list` gives with the same config, as [`Project::of`] folds
  /// them, then the folder of the sessions with no project, `sessions` in that config folder.
  Projects {
    #[serde(default)]
    config: Option<String>,
  },
  /// The files of the folder `cwd` and of each folder in it, which a mention of the input names:
  /// dot files too, less what an ignore file of git leaves out, the folder `.git` and the records
  /// of the sessions of furb, `.furb/sessions`. A `cwd` that starts with `~` names a folder of the
  /// home of the user of the machine.
  Files {
    cwd: String,
  },
  /// The changes of the checkout of git that holds the folder `cwd`, against its last commit: its
  /// branch, each file that differs, by its path from the top of the checkout, with the lines that
  /// it adds and removes, and the diff of the file at `path` when the peer names one. A folder in
  /// no checkout has no branch and no change.
  Changes {
    cwd: String,
    #[serde(default)]
    path: Option<String>,
  },
  /// The models that the catalog of the machine offers to a life in the folder `cwd`, with the
  /// settings of the folder `config` and the claude command line at `claude`, each as `open`
  /// takes it, with no life: the ones of the daemon when the peer names none.
  Models {
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    config: Option<String>,
    #[serde(default)]
    claude: Option<String>,
  },
  /// The cards of the machine: each thread that an operator started in each record that `list`
  /// gives with no config. The peer then hears a frame `cards` at each change of them, until it
  /// leaves.
  Cards,
  /// What the peer sent in the project of a record, the newest first and each text once: what it
  /// sent to that record, then to each other record of the project, the record that changed last
  /// first. A record whose index does not agree with it gives nothing until the daemon made its
  /// index.
  History {
    life: String,
  },
  /// The skills that the root of a life read last, by a read of `skills://`, in the order of their
  /// names: those of the record, else those of each other record of its project, the record that
  /// changed last first, so a new session offers the skills of the last session of its project.
  /// A record whose index does not agree with it gives none until the daemon made its index.
  Skills {
    life: String,
  },
  /// The skills of a life as its root finds them now, in the order of their names, each with how
  /// it plays: each skill that the folders of skills of its working directory, the configs and
  /// furb give.
  Plays {
    life: String,
  },
  /// The life of a record opened, in a folder, else in the checkout whose `.furb/sessions` holds
  /// it, else in the folder of its record. A life that this open makes stands on the actor, the
  /// roster and the skills that the peer names, reads the settings of the user in the folder
  /// `config` and runs the claude command line `claude` when the peer names them, as the window
  /// of the peer reads them, and the peer answers each request of its models in their place when
  /// it asks to, as a demo does.
  /// With `attach`, the peer attaches the life in the same step, from that place in its log, so
  /// the life has a holder before its grace can pass.
  Open {
    life: String,
    cwd: Option<String>,
    actor: Option<String>,
    roster: Option<Vec<String>>,
    skills: Option<bool>,
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
  /// The end of a life now, whatever pins hold it, as the grace ends a life with no holder: it
  /// writes its image, each of its windows hears its end, and its record opens again at its tip,
  /// where the life takes its open work again, so an end is a pause. With `cancel`, the end is a
  /// kill: the operator first cancels each open work of the life, so the next life takes none.
  /// It is refused while a window of another peer holds the life, so no view ends a life that
  /// another view shows.
  End {
    life: String,
    #[serde(default)]
    cancel: bool,
  },
  /// Each operator whose window holds a life now.
  Presence {
    life: String,
  },
  /// The draft that the person of the window writes now, or none once it is empty, which each
  /// window of the life hears in the presence. It is no fact, and it goes with the window.
  Writes {
    life: String,
    draft: Option<Draft>,
  },
  Get {
    life: String,
    act: String,
  },
  Peek {
    life: String,
    act: String,
  },
  /// An act as the life folds it once it is done: at once when it is done, else when its done
  /// comes. The window that ran a word asks it of the rung of that word, and asks it again after a
  /// reconnect, since the record keeps the end.
  Ended {
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
  /// A word that runs on a chain as a turn of the operator: each act of a person goes this way, as
  /// the word that `Door` gives, so the transcript shows it as it shows the act of a model.
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
  /// A fact that the process of a kit says of its own accord, as news that it learned: its kind,
  /// the chain or the act that it is about, and its words, each written as the record writes a
  /// value. A life that names `fact` in its welcome takes it.
  Fact {
    life: String,
    kind: String,
    about: String,
    words: Vec<String>,
  },
  /// The names that the module of a chain binds.
  Names {
    life: String,
    on: Option<String>,
  },
  /// The models that a life offers, in place of the roster of the life, which no word can say,
  /// since the roster is the World's: the life then stands on them, as after a change of its
  /// configs, and each later birth of it keeps them.
  Roster {
    life: String,
    roster: Vec<String>,
  },
  /// The configs of the folder of the life, in their layers: what each key holds, a change of a
  /// key, or a stand on them as the disk holds them now.
  Config {
    life: String,
    config: Config,
  },
  /// An image kept in the gallery of the life, beside its record, under the digest of its bytes:
  /// a PNG, a JPEG, a GIF or a WebP, in base64. It answers the text by which a message names it
  /// under its name, `![name](uri)`, so that the model sees it.
  Image {
    life: String,
    name: String,
    data: String,
  },
  /// The sign-in of codex of the user of the machine, which the daemon answers once it ends: its
  /// state is pushed at each change, and once it connects the user config turns codex on, checked
  /// against the catalog of the folder `cwd`, each life stands again, so each project offers its
  /// models, and the answer gives the models that the catalog of `cwd` offers then. A sign-in from
  /// a `device` gives a code that the operator enters on a page in the browser of any device; else
  /// the page of the sign-in calls back the machine of the daemon, so a window on another machine
  /// asks a sign-in from a device.
  SignIn {
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    device: bool,
  },
  /// A cancel of the sign-in of codex that runs on the machine, which refuses the sign-in or the
  /// config that asked it. It writes no config.
  CancelSignIn,
  /// The fact that the operator read a chain or a thread up to now, which each window and each
  /// later life reads in the record.
  Seen {
    life: String,
    about: String,
  },
  /// The resolve of a thread of the operator: a close of each act that keeps it open, the last
  /// first, with the text `resolved` or with None, as its shape takes. A message on the thread
  /// opens it again.
  Resolve {
    life: String,
    thread: String,
  },
  /// A word of the operator on the thread `on`, or on the root, that writes back at each path that
  /// the writes that `writes` names wrote the text that the life knew there before the first of
  /// them, in their order. A path that the life did not know before stays as it is, and a revert of
  /// writes that wrote no such path is refused. A revert of the word of a revert writes again what
  /// it reverted.
  Revert {
    life: String,
    writes: Vec<String>,
    on: Option<String>,
  },
  /// A follow-up that the operator queued on a chain, or on a thread, under an entry that the
  /// window chose. The life sends it as a prompt of its shape and its message once the chain rests,
  /// on the chain, or on the thread as a message of it once the chain of the thread rests, and a
  /// second queue of the same entry, from any window, changes nothing.
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
  /// The follow-ups that wait on a chain, or on a thread, in order.
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
  /// The threads of every chain of a life but those it names, as one tree in the order of the
  /// list of the threads, now or as they stood just after a moment.
  Rail {
    life: String,
    #[serde(default)]
    hidden: Vec<String>,
    at: Option<u64>,
  },
  /// The ladder of a chain, now or as it stood just after a moment: the steps that show and the
  /// folds of the others, with the folds that the window opened, the prompts that the operator
  /// owes, the only acts at its top when `taken` names them, and how many last steps show.
  Ladder {
    life: String,
    on: Option<String>,
    at: Option<u64>,
    #[serde(default)]
    opened: Vec<String>,
    #[serde(default)]
    owed: Vec<String>,
    #[serde(default)]
    taken: Option<Vec<String>>,
    #[serde(default)]
    shown: Option<Shown>,
  },
  /// The acts of a life as they stood just after a moment, each with its meaning then.
  AsOf {
    life: String,
    at: u64,
  },
  /// A page of a thread of a life, its prompt: the exchanges before the one at `before`, or
  /// before the end, at most `limit` of them, the newest page first, now or as they stood just
  /// after the moment `at`. The page of the end of now also makes the peer hear a frame `thread`
  /// at each change of that thread, until it reads the end of another thread or leaves: a peer
  /// reads one thread at a time.
  Thread {
    life: String,
    thread: String,
    #[serde(default)]
    before: Option<usize>,
    limit: usize,
    #[serde(default)]
    at: Option<u64>,
  },
  /// Each moment of a life at which an act began or ended, in order.
  Moments {
    life: String,
    /// The chain whose acts give the moments, or the thread whose page shows the acts that give
    /// them; none gives those of the whole life.
    #[serde(default)]
    on: Option<String>,
  },
  /// Each chain of a life in time, in the order the chains began.
  Tracks {
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
  /// The place of the daemon given to the daemon of a new build: each life written to its image
  /// and ended, and the lives with their pins in the answer, once the daemon stopped to
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
  /// The record of the life that the verb names, which the daemon reads as its one key. Each verb
  /// is named here, so a new verb says whether it names a record.
  #[expect(clippy::too_many_lines, reason = "the match names each verb, one on each line")]
  pub fn life_mut(&mut self) -> Option<&mut String> {
    match self {
      Self::Open { life, .. }
      | Self::Attach { life, .. }
      | Self::Detach { life }
      | Self::Pin { life, .. }
      | Self::Unpin { life, .. }
      | Self::End { life, .. }
      | Self::Presence { life }
      | Self::Writes { life, .. }
      | Self::Get { life, .. }
      | Self::Peek { life, .. }
      | Self::Ended { life, .. }
      | Self::Transcript { life, .. }
      | Self::Turns { life, .. }
      | Self::Program { life, .. }
      | Self::Standing { life }
      | Self::Filled { life, .. }
      | Self::Cwd { life, .. }
      | Self::Inspect { life, .. }
      | Self::Pending { life }
      | Self::Prompt { life, .. }
      | Self::Answer { life, .. }
      | Self::Pause { life, .. }
      | Self::Wake { life, .. }
      | Self::Cancel { life, .. }
      | Self::Run { life, .. }
      | Self::Close { life, .. }
      | Self::Fact { life, .. }
      | Self::Names { life, .. }
      | Self::Roster { life, .. }
      | Self::Config { life, .. }
      | Self::Image { life, .. }
      | Self::Seen { life, .. }
      | Self::Resolve { life, .. }
      | Self::Revert { life, .. }
      | Self::Queue { life, .. }
      | Self::Unqueue { life, .. }
      | Self::Queued { life, .. }
      | Self::Desk { life, .. }
      | Self::Threads { life, .. }
      | Self::Rail { life, .. }
      | Self::Ladder { life, .. }
      | Self::AsOf { life, .. }
      | Self::Thread { life, .. }
      | Self::Moments { life, .. }
      | Self::Tracks { life }
      | Self::History { life }
      | Self::Skills { life }
      | Self::Plays { life }
      | Self::Answered { life, .. }
      | Self::Wrote { life, .. }
      | Self::ViewMount(Mount { life, .. }) => Some(life),
      Self::Hello(_)
      | Self::List { .. }
      | Self::Projects { .. }
      | Self::Models { .. }
      | Self::SignIn { .. }
      | Self::CancelSignIn
      | Self::Files { .. }
      | Self::Changes { .. }
      | Self::Cards
      | Self::Handover
      | Self::ViewTree(_)
      | Self::ViewEvent(_)
      | Self::ViewEffect(_)
      | Self::ViewDispose(_) => None,
    }
  }

  /// The least role that says the verb. A verb that this list does not name needs the role to
  /// write.
  pub fn needs(&self) -> Role {
    let reads = matches!(
      self,
      Self::Hello(_)
        | Self::List { .. }
        | Self::Projects { .. }
        | Self::Models { .. }
        | Self::Changes { .. }
        | Self::Cards
        | Self::History { .. }
        | Self::Skills { .. }
        | Self::Plays { .. }
        | Self::Attach { .. }
        | Self::Detach { .. }
        | Self::Presence { .. }
        | Self::Get { .. }
        | Self::Peek { .. }
        | Self::Ended { .. }
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
        | Self::Rail { .. }
        | Self::Ladder { .. }
        | Self::AsOf { .. }
        | Self::Thread { .. }
        | Self::Moments { .. }
        | Self::Tracks { .. }
        | Self::Names { .. }
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
  /// A key of a layer, the layer of the scope of the key when none is named, set to a value, or
  /// taken out with no value, then a stand of the life on the configs. A change that the catalog
  /// refuses puts its config back as it was, and the life does not stand.
  Set { key: Vec<String>, value: Option<Value>, layer: Option<Layer> },
}

pub use crate::config::Layer;

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
  /// Each operator that a window of a life holds now, pushed to each of its windows after each
  /// attach, detach and draft.
  Presence { life: String, operators: Vec<Present> },
  /// The role that the peer holds now, pushed when a change of the access list moved it, so a
  /// window shows what its login may do for its whole life, not only at its welcome.
  Granted { role: Role },
  /// A change of the cards of one record, pushed to each peer that asked the cards: each card that
  /// is new or that changed, and the prompt of each card that the record holds no more.
  Cards { life: String, changed: Vec<Card>, gone: Vec<String> },
  /// How far the daemon stands in the index of the cards of a record that no life holds, pushed to
  /// each peer that asked the cards while the daemon reads the record: the thousandths of the read
  /// done, of 1000, which never go back. The frame with `done` equal to `of` comes before the frame
  /// `cards` of the record.
  Indexing { life: String, done: u64, of: u64 },
  /// A change of the thread that the peer reads: each exchange that is new or that changed, how
  /// many exchanges stand before the first of the page of the end now, and the agents of the
  /// thread.
  Thread { life: String, thread: String, page: Page },
  /// How far the open of a life stands, pushed to each of its windows while the life opens: the
  /// stage, its work done and its work known, each in the unit of the stage, and both 0 for a
  /// stage that counts nothing. The stages of one open come in their order, the numbers of a stage
  /// never go back, and `done` never passes `of`. A window hears a frame at once for each stage
  /// that the open reaches, and at most four each second for a stage that goes on. The frame of
  /// `log` with `done` equal to `of` ends the open, and comes before the first event that the open
  /// tells. It is no event of the log and no fact of the record.
  Opening { life: String, stage: Stage, done: u64, of: u64 },
  /// A life that holds open acts and said no fact for the bound of the daemon, pushed once to each
  /// of its windows by the watch of the daemon, which no word of the life runs: each open act, and
  /// the notice that a window shows as it stands.
  Stalled { life: String, acts: Vec<Open>, notice: String },
  /// The state of the sign-in of codex that this peer asked, pushed at each change of the state,
  /// so no window asks it again: the word of the state, the browser address while it waits, the
  /// code to enter there for a sign-in from a device, and what to tell the operator when it needs
  /// him or failed.
  SignIn {
    state: String,
    url: Option<String>,
    #[serde(default)]
    code: Option<String>,
    message: Option<String>,
  },
  /// A request of a model of a life that the peer answers: its number, and the request with the
  /// actor, the chain, the messages and the settings.
  Ask { life: String, ask: u64, request: Value },
  /// The skills attached to a life, each by its name, which a window mounts a view of: pushed to
  /// each window of the life when it attaches, and after each attach and each end of a skill.
  Served { life: String, skills: Vec<String> },
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
    Self::Refused { id, why: error.to_string(), error }
  }
}

/// What a verb comes to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "answer", content = "data", rename_all = "snake_case")]
pub enum Answer {
  Done,
  /// The answer to a hello: the protocol of the daemon, the peer as the daemon knows it, the name
  /// of the machine of the daemon, and its build. A life that answers the process of a kit names
  /// too each verb that it takes past those of the protocol 44 of [`KITS`], and the folders of the
  /// life, as `initialize` gives them in LSP.
  Welcome {
    protocol: u32,
    peer: Peer,
    machine: String,
    build: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    folders: Option<Folders>,
  },
  Lives(Vec<Listed>),
  Projects(Vec<Project>),
  Models(Vec<Model>),
  Files(Files),
  Changes(Changes),
  /// The cards of the machine.
  Cards(Vec<Card>),
  /// What the peer sent, the newest first.
  History(Vec<card::Sent>),
  /// The skills of a life, each with what it is for, and with how it plays when the verb asks.
  Skills(Vec<card::Skill>),
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
  Presence(Vec<Present>),
  /// The lives that a daemon gave at its handover.
  Handed(Vec<Handed>),
  /// What waits for the operator.
  Desk(Desk),
  /// The threads of a chain.
  Threads(Threads),
  /// The threads of every chain of a life, as one tree.
  Rail(Vec<ThreadNode>),
  /// The ladder of a chain.
  Ladder(Vec<Rung>),
  /// The acts as they stood at a moment, or the one act that `ended` names.
  Acts(Vec<Act>),
  /// Each moment at which an act began or ended.
  Moments(Vec<u64>),
  /// Each chain of a life in time.
  Tracks(Vec<ChainTrack>),
  /// A page of a thread.
  Page(Page),
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

/// A record of the machine: the main checkout of the project whose sessions hold it, when the
/// list names one, else the checkout whose `.furb/sessions` holds a record that a life holds,
/// the second of the epoch of the last write of the record, when the disk holds it,
/// whether a life of the daemon holds it now, whether that life lives and does not
/// end, whether a window holds that life, and the name of its session while a life holds it: the
/// one that the operator said last on the root, which wins, else the one that the skill naming
/// gave. An empty name takes the default name again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Listed {
  pub life: String,
  pub project: Option<String>,
  pub used: Option<u64>,
  pub held: bool,
  pub open: bool,
  /// Whether a window of any connection holds the life now: a pin holds it with no window.
  #[serde(default)]
  pub watched: bool,
  /// The holder of each pin of the life, so a life that only a pin holds stands in the background,
  /// and a life with no holder, which ends at its grace, does not.
  #[serde(default)]
  pub pins: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
}

/// A card of the machine: one thread that an operator started, in the record of its session. It
/// holds the name of the session when it has one, the main checkout of the project whose sessions
/// hold the record, when the list of the machine names one, else the checkout whose
/// `.furb/sessions` holds a record that a life holds, whether a life holds the record now,
/// each operator whose window holds that life, and the second of the epoch since which the thread
/// stands as it does. A record that has no index of its threads yet gives no card until
/// the daemon made its index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Card {
  pub life: String,
  pub session: Option<String>,
  pub project: Option<String>,
  /// Why no life of the machine opens the record of the card, whose thread then only reads.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub refused: Option<String>,
  pub held: bool,
  pub viewers: Vec<String>,
  /// The holder of each pin of the life, as `list` gives it.
  pub pins: Vec<String>,
  pub since: u64,
  #[serde(flatten)]
  pub thread: card::Card,
}

impl Card {
  /// Where a member finds the card in a list: its part, its rank in that part, and its time. What
  /// needs him waits the longest first, and each other part shows the newest first. A thread that
  /// only reads is resolved, whatever its record says of it.
  pub fn place(&self, operator: &str) -> (card::Part, u8, i64) {
    let (part, rank) =
      if self.refused.is_some() { (card::Part::Resolved, 0) } else { self.thread.part(operator) };
    let since = i64::try_from(self.since).unwrap_or(i64::MAX);
    (part, rank, if part == card::Part::NeedsYou { since } else { -since })
  }

  /// Whether the card waits for a member: it stands in the part that needs him.
  pub fn needs(&self, operator: &str) -> bool {
    self.place(operator).0 == card::Part::NeedsYou
  }
}

/// A view that a window mounts in a slot of a life, which the process of a skill serves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Mount {
  pub viewer: String,
  pub life: String,
  pub slot: Slot,
  pub skill: String,
}

/// The tree of a view, which takes the place of each tree of a lower version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tree {
  pub viewer: String,
  pub slot: Slot,
  pub version: u64,
  pub tree: Node,
}

/// What a person did to a node of a view, with the value of an input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewEvent {
  pub viewer: String,
  pub node: String,
  pub kind: Gesture,
  pub value: Option<String>,
}

/// What a skill asks of the one window of a view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewEffect {
  pub viewer: String,
  pub effect: Effect,
}

/// The end of a view in its window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
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
  pub fn inert(self) -> Self {
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
    Self { shape, ..self }
  }
}

fn inerts(nodes: Vec<Node>) -> Vec<Node> {
  nodes.into_iter().map(Node::inert).collect()
}

/// The closed set of the nodes of a view, which each window draws in its own way.
/// The files of a folder: the path of each from the folder, in the order of a walk by name, and
/// whether more files stand there than the daemon gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Files {
  pub paths: Vec<String>,
  pub more: bool,
}

/// The changes of a checkout of git against its last commit: its branch, none when its head names
/// no branch, each file that differs, and the diff of the file that the peer named, as git writes
/// a unified diff, a file that git does not track yet added whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Changes {
  pub branch: Option<String>,
  pub files: Vec<Changed>,
  #[serde(default)]
  pub diff: Option<String>,
}

/// A file of a checkout that differs from the last commit: its path from the top of the checkout,
/// the lines that it adds and removes, and how it differs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Changed {
  pub path: String,
  pub added: u32,
  pub removed: u32,
  pub state: Change,
}

/// How a file differs from the last commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Change {
  Added,
  Modified,
  Deleted,
  Renamed,
  /// A file that git does not track yet.
  Untracked,
}

/// A project of the machine: its main checkout, the count of the records of its sessions, the
/// second of the epoch of the last write of one of them, and whether the folder holds instead the
/// sessions that stand in no project, each in a folder of its own that a window names in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
  pub folder: String,
  pub sessions: usize,
  pub used: Option<u64>,
  #[serde(default)]
  pub detached: bool,
}

impl Project {
  /// The projects of the records of a list, each once, the one used last first. A record of no
  /// project stands in none.
  pub fn of(lives: &[Listed]) -> Vec<Self> {
    let mut projects: Vec<Self> = Vec::new();
    for listed in lives {
      let Some(folder) = &listed.project else { continue };
      match projects.iter_mut().find(|one| &one.folder == folder) {
        Some(one) => (one.sessions, one.used) = (one.sessions + 1, one.used.max(listed.used)),
        None => projects.push(Self {
          folder: folder.clone(),
          sessions: 1,
          used: listed.used,
          detached: false,
        }),
      }
    }
    projects.sort_by(|one, other| other.used.cmp(&one.used).then(one.folder.cmp(&other.folder)));
    projects
  }
}

/// A model that the catalog of a machine offers: the name that a stand takes, the levels of effort
/// that it takes, the level that a life takes when its actor names none, the tokens of its window,
/// and whether it sees images.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Model {
  pub name: String,
  pub efforts: Vec<String>,
  #[serde(default)]
  pub effort: Option<String>,
  pub window: u64,
  pub images: bool,
}

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
  let generator = SchemaSettings::draft07().into_generator();
  let mut schema = generator.into_root_schema_for::<Wire>();
  schema.insert("x-protocol".to_owned(), PROTOCOL.into());
  schema.to_value()
}
