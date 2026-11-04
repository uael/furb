//! The stream-json that the claude command line writes to its stdout, one event on each line, as
//! the types that serde reads. A field that the protocol gives is required here, so an event that
//! lacks it fails the turn. A kind of event, of block or of delta that this does not read is
//! `Other`, and it is ignored.

use serde::Deserialize;
use serde_json::Value;

/// One line of the stdout.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Event {
  /// An event of the message that streams.
  #[serde(rename = "stream_event")]
  Stream { event: Streamed },
  /// A message whole, once the command line has it.
  Assistant { message: Said },
  /// The end of the turn.
  Result(Finished),
  #[serde(other)]
  Other,
}

impl Event {
  /// The event that a JSON value is. A value with no type is none of the events, and reads as
  /// `Other`.
  pub(super) fn read(value: Value) -> Result<Event, serde_json::Error> {
    if value.get("type").is_none() {
      return Ok(Event::Other);
    }
    serde_json::from_value(value)
  }
}

/// An event of the message that streams.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Streamed {
  MessageStart,
  ContentBlockStart {
    index: u64,
    content_block: Start,
  },
  ContentBlockDelta {
    index: u64,
    delta: Delta,
  },
  ContentBlockStop {
    index: u64,
  },
  #[serde(other)]
  Other,
}

/// The block that a stream opens.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Start {
  Text {
    text: String,
  },
  Thinking {
    thinking: String,
  },
  #[serde(other)]
  Other,
}

/// What a block gained.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Delta {
  #[serde(rename = "text_delta")]
  Text { text: String },
  #[serde(rename = "thinking_delta")]
  Thinking { thinking: String },
  #[serde(rename = "signature_delta")]
  Signature { signature: String },
  #[serde(other)]
  Other,
}

/// A message whole: its blocks, and what its request spent when the command line says it.
#[derive(Deserialize)]
pub(super) struct Said {
  pub(super) content: Vec<Whole>,
  pub(super) usage: Option<Spent>,
}

/// A block whole.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Whole {
  Text {
    text: String,
  },
  Thinking {
    thinking: String,
    signature: Option<String>,
  },
  #[serde(other)]
  Other,
}

/// What a request read and wrote. A count that the command line leaves out is none, since it says
/// `{}` of a turn that spent nothing.
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub(super) struct Spent {
  #[serde(rename = "input_tokens")]
  pub(super) input: u64,
  #[serde(rename = "output_tokens")]
  pub(super) output: u64,
  #[serde(rename = "cache_read_input_tokens")]
  pub(super) cache_read: u64,
  #[serde(rename = "cache_creation_input_tokens")]
  pub(super) cache_creation: u64,
}

/// The end of a turn. Only a turn that fails has an error, a subtype and a status, so the fields
/// of an error are optional.
#[derive(Deserialize)]
pub(super) struct Finished {
  pub(super) session_id: Option<String>,
  #[serde(default)]
  pub(super) is_error: bool,
  pub(super) result: Option<String>,
  pub(super) subtype: Option<String>,
  pub(super) api_error_status: Option<Value>,
  pub(super) errors: Option<Vec<Value>>,
  pub(super) usage: Option<Spent>,
  pub(super) total_cost_usd: Option<f64>,
  pub(super) stop_reason: Option<Stop>,
}

/// Why a turn stopped, when it stopped on a limit or on a refusal.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Stop {
  MaxTokens,
  Refusal,
  #[serde(other)]
  Other,
}

#[cfg(test)]
#[path = "protocol.test.rs"]
mod test;
