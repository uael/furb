//! Why the daemon refuses a frame of a peer, which crosses the protocol as data beside its text.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::protocol::{Kind, Role};

/// Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum ProtocolError {
  /// The hello names another protocol, and the daemon closes the peer.
  #[error("the daemon speaks protocol {ours}, and the peer speaks protocol {theirs}")]
  Protocol { ours: u32, theirs: u32 },
  /// The first frame of the peer was no hello, and the daemon closes the peer.
  #[error("the first frame of a peer is its hello")]
  First,
  /// A second hello of the same peer.
  #[error("a peer says its hello once")]
  Again,
  #[error("the verb needs the role {needs:?}, and the peer holds the role {has:?}")]
  Role { needs: Role, has: Role },
  /// A frame that is no frame of the protocol, with what the reader found.
  #[error("the frame is no frame of the protocol: {why}")]
  Unread { why: String },
  /// A frame past the bound, and the daemon closes the peer.
  #[error("a frame holds more than {bound} bytes")]
  Large { bound: usize },
  /// The daemon or the life refused the verb, with why.
  #[error("{why}")]
  Refused { why: String },
  /// The daemon gave its place to the daemon of a new build, or stops.
  #[error("the daemon stops")]
  Stopped,
  #[error("no life of the daemon lives on the record {life}")]
  Closed { life: String },
  /// A life is named by the absolute path of its record, the same for each peer.
  #[error("the record {life} is not an absolute path")]
  Relative { life: String },
  #[error("the peer did not attach the life on the record {life}")]
  Unattached { life: String },
  /// A peer may not end a life that the window of another peer shows.
  #[error("another window shows the life on the record {life}")]
  Shown { life: String },
  #[error("no pin of the holder {holder} holds the life on the record {life}")]
  Unpinned { life: String, holder: String },
  /// A turn or a write for a request of a model that the life does not wait for.
  #[error("no model of the life on the record {life} waits for the turn of the request {ask}")]
  Unasked { life: String, ask: u64 },
  /// A turn or a write for a request of a model that another peer answers: only the peer that
  /// opened the life with `answers` speaks for its requests.
  #[error("another peer answers the request {ask} of a model of the life on the record {life}")]
  Foreign { life: String, ask: u64 },
  /// A turn for a life whose models no peer answers.
  #[error("no peer answers the models of the life on the record {life}")]
  Answerless { life: String },
  /// An open with `answers` of a life whose models a peer that lives answers already.
  #[error("a peer that lives answers the models of the life on the record {life}")]
  Answering { life: String },
  /// A frame of a view that only a window says, or only a skill, and the peer is of another kind.
  #[error("the verb is said by a peer of the kind {needs:?}, and the peer is of the kind {has:?}")]
  Speaker { needs: Kind, has: Kind },
  /// A skill says its name at its hello, since a window mounts the views of a skill by it.
  #[error("a skill says its name at its hello")]
  Nameless,
  #[error("no skill {skill} is attached to the life on the record {life}")]
  NoSkill { life: String, skill: String },
  /// A view belongs to the window that mounted it and to the skill that serves it, and to no other
  /// peer.
  #[error("the peer holds no view {viewer}")]
  Unmounted { viewer: String },
  /// A viewer names one view of the daemon.
  #[error("a view {viewer} is mounted already")]
  Mounted { viewer: String },
}
