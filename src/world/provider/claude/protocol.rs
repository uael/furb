//! The stream-json that the claude command line writes to its stdout, one event on each line, as
//! the types that serde reads. A field that the protocol gives is required here, so an event that
//! lacks it fails the turn. A kind of event, of block or of delta that this does not read is
//! `Other`, and it is ignored. The reply of a turn is made of these events as they come.

use std::collections::HashMap;

use rig_core::{
  completion::{CompletionResponse, FinishReason, Usage},
  message::{AssistantContent, Reasoning},
  streaming::RawStreamingChoice,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{CLAUDE, Deltas};

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
  /// The answer of a control request, which the probe of the models sends.
  ControlResponse { response: Control },
  #[serde(other)]
  Other,
}

impl Event {
  /// The event that a JSON value is. A value with no type is none of the events, and reads as
  /// `Other`.
  pub(super) fn read(value: Value) -> Result<Self, serde_json::Error> {
    if value.get("type").is_none() {
      return Ok(Self::Other);
    }
    serde_json::from_value(value)
  }
}

/// The answer of a control request, by the id of the request: what it gave, or that it failed.
#[derive(Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub(super) enum Control {
  Success {
    request_id: String,
    #[serde(default)]
    response: Value,
  },
  Error {
    request_id: String,
  },
  #[serde(other)]
  Other,
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
  RedactedThinking {
    data: String,
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
  RedactedThinking {
    data: String,
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

/// One block of a reply as the command line streams it: a text, a thought and its signature, or a
/// thought that the API redacted, as its opaque data, which the next turn gives back whole.
enum Block {
  Text(String),
  Thinking(String, Option<String>),
  Redacted(String),
}

impl Block {
  fn content(&self) -> AssistantContent {
    match self {
      Self::Text(text) => AssistantContent::text(text.clone()),
      Self::Thinking(text, signature) => {
        AssistantContent::Reasoning(Reasoning::new_with_signature(text, signature.clone()))
      }
      Self::Redacted(data) => AssistantContent::Reasoning(Reasoning::redacted(data.clone())),
    }
  }
}

/// The reply of a turn as it is written: its blocks, and where the stream puts each.
#[derive(Default)]
pub(super) struct Reply<'a> {
  blocks: Vec<Block>,
  /// The place in the reply of each block the current message streams, by its index in the stream.
  /// A block of a kind this does not read has none, and nothing streamed means the command line
  /// says each message whole.
  streamed: Option<HashMap<u64, usize>>,
  /// How many blocks of the current message the command line has settled, which is the index of the
  /// next one.
  settled: u64,
  /// The usage of the last message the command line settled, which is the one request of the turn
  /// whose answer the reply holds.
  pub(super) spent: Option<Spent>,
  deltas: Option<&'a Deltas>,
}

impl<'a> Reply<'a> {
  /// A reply with no block yet, whose parts go where the turn streams.
  pub(super) fn new(deltas: Option<&'a Deltas>) -> Self {
    Reply { deltas, ..Reply::default() }
  }

  /// One part of the stream, told to whoever streams the turn.
  fn told(&self, part: RawStreamingChoice) {
    if let Some(deltas) = self.deltas {
      drop(deltas.send(Ok(part)));
    }
  }

  /// A block that begins, which a stream opens.
  fn opened(&mut self, block: Block) {
    let id = self.blocks.len().to_string();
    self.told(match block {
      Block::Text(_) => RawStreamingChoice::TextStart { id: id.into(), additional_params: None },
      Block::Thinking(..) | Block::Redacted(_) => {
        RawStreamingChoice::ReasoningStart { id: id.into(), provider_id: None }
      }
    });
    self.blocks.push(block);
  }

  /// What the block at a place gained, which a stream tells as it comes.
  fn grew(&mut self, at: usize, text: &str) {
    let id = at.to_string();
    match self.blocks.get_mut(at) {
      Some(Block::Text(held)) => {
        held.push_str(text);
        self.told(RawStreamingChoice::Message(text.to_owned()));
      }
      Some(Block::Thinking(held, _)) => {
        held.push_str(text);
        let reasoning = text.to_owned();
        self.told(RawStreamingChoice::ReasoningDelta {
          id: id.into(),
          provider_id: None,
          reasoning,
        });
      }
      Some(Block::Redacted(_)) | None => {}
    }
  }

  /// The block at a place is whole, which a stream closes.
  fn closed(&self, at: usize) {
    let id = at.to_string().into();
    match self.blocks.get(at) {
      Some(Block::Text(_)) => self.told(RawStreamingChoice::TextEnd { id }),
      Some(block @ (Block::Thinking(..) | Block::Redacted(_))) => {
        let AssistantContent::Reasoning(reasoning) = block.content() else { return };
        let reasoning = Some(reasoning);
        self.told(RawStreamingChoice::ReasoningEnd {
          id,
          reasoning,
          signature: None,
          wire_sent: true,
        });
      }
      None => {}
    }
  }

  /// One event of a message as it streams.
  pub(super) fn streamed(&mut self, update: &Streamed) {
    if matches!(update, Streamed::MessageStart) {
      self.streamed = Some(HashMap::new());
      self.settled = 0;
    }
    let places = self.streamed.get_or_insert_default();
    match update {
      Streamed::ContentBlockStart { index, content_block } => {
        let block = match content_block {
          Start::Text { text } => Block::Text(text.clone()),
          Start::Thinking { thinking } => Block::Thinking(thinking.clone(), None),
          Start::RedactedThinking { data } => Block::Redacted(data.clone()),
          Start::Other => return,
        };
        places.insert(*index, self.blocks.len());
        self.opened(block);
      }
      Streamed::ContentBlockDelta { index, delta } => {
        let Some(at) = places.get(index).copied() else { return };
        match (delta, self.blocks.get(at)) {
          (Delta::Text { text }, Some(Block::Text(_))) => self.grew(at, text),
          (Delta::Thinking { thinking }, Some(Block::Thinking(..))) => {
            self.grew(at, thinking);
          }
          (Delta::Signature { signature }, _) => {
            if let Some(Block::Thinking(_, held)) = self.blocks.get_mut(at) {
              held.get_or_insert_default().push_str(signature);
            }
          }
          _ => {}
        }
      }
      Streamed::ContentBlockStop { index } => {
        if let Some(at) = places.get(index).copied() {
          self.closed(at);
        }
      }
      Streamed::MessageStart | Streamed::Other => {}
    }
  }

  /// The blocks of a message that the command line says whole. It says each block again, once it is
  /// whole, in the order it streamed them: the whole block stands over the one streamed at its
  /// place, and never twice. A message that did not stream is told as it comes.
  pub(super) fn settled(&mut self, content: &[Whole]) {
    for one in content {
      let block = match one {
        Whole::Text { text } => Some(Block::Text(text.clone())),
        Whole::Thinking { thinking, signature } => {
          Some(Block::Thinking(thinking.clone(), signature.clone()))
        }
        Whole::RedactedThinking { data } => Some(Block::Redacted(data.clone())),
        Whole::Other => None,
      };
      if let Some(streamed) = &self.streamed {
        let at = streamed.get(&self.settled).copied();
        self.settled += 1;
        if let (Some(at), Some(block)) = (at, block) {
          self.blocks[at] = block;
        }
      } else {
        let Some(block) = block else { continue };
        let whole = match &block {
          Block::Text(text) | Block::Thinking(text, _) => text.clone(),
          Block::Redacted(_) => String::new(),
        };
        let at = self.blocks.len();
        self.opened(match block {
          Block::Text(_) => Block::Text(String::new()),
          Block::Thinking(_, signature) => Block::Thinking(String::new(), signature),
          redacted @ Block::Redacted(_) => redacted,
        });
        self.grew(at, &whole);
        self.closed(at);
      }
    }
  }

  /// The response the result of a turn ends the reply with: its blocks, or the text of the result
  /// when the reply holds none, what its last request read and wrote, and its part of the cost the
  /// process says of all its turns. The usage of the result is the sum of every request of the
  /// turn, and the command line makes many when the model calls a tool, which it refuses, or stops
  /// at its limit of output, so only the last request says what the answer read of the window. A
  /// result that no message with a usage came before gives its own usage.
  pub(super) fn response(
    mut self,
    result: &Finished,
    name: &str,
    id: &str,
    paid: &mut f64,
  ) -> CompletionResponse {
    if self.blocks.is_empty()
      && let Some(text) = result.result.as_deref().filter(|one| !one.is_empty())
    {
      self.blocks.push(Block::Text(text.to_owned()));
    }
    let spent = self.spent.take().or_else(|| result.usage.clone()).unwrap_or_default();
    let mut usage = Usage::new();
    usage.input_tokens = spent.input;
    usage.output_tokens = spent.output;
    usage.cached_input_tokens = spent.cache_read;
    usage.cache_creation_input_tokens = spent.cache_creation;
    usage.total_tokens = usage.input_tokens
      + usage.output_tokens
      + usage.cached_input_tokens
      + usage.cache_creation_input_tokens;
    let total = result.total_cost_usd.unwrap_or(*paid);
    let cost = (total - *paid).max(0.0);
    *paid = total;
    let finish = match result.stop_reason {
      Some(Stop::MaxTokens) => FinishReason::Length,
      Some(Stop::Refusal) => FinishReason::ContentFilter,
      Some(Stop::Other) | None => FinishReason::Stop,
    };
    let choice = self.blocks.iter().map(Block::content).collect();
    CompletionResponse::new(choice, usage, CLAUDE)
      .with_finish_reason(finish)
      .with_model(name)
      .with_response_id(id)
      .with_raw(json!({"cost": cost}))
  }
}

/// What an error result tells when its text is empty: its subtype, its status and its errors, so
/// that a login error shows apart from a limit.
pub(super) fn unexplained(event: &Finished) -> String {
  let mut facts = Vec::new();
  if let Some(subtype) = event.subtype.as_deref().filter(|one| !one.is_empty()) {
    facts.push(format!("subtype {subtype}"));
  }
  if let Some(status) = &event.api_error_status {
    facts.push(format!("api_error_status {status}"));
  }
  let errors = event
    .errors
    .iter()
    .flatten()
    .map(|one| one.as_str().map_or_else(|| one.to_string(), str::to_owned))
    .collect::<Vec<_>>()
    .join("; ");
  let head = if facts.is_empty() { String::new() } else { format!(" ({})", facts.join(", ")) };
  let tail = if errors.is_empty() { String::new() } else { format!(": {errors}") };
  format!("Claude reported an error{head}{tail}.")
}

#[cfg(test)]
#[path = "protocol.test.rs"]
mod test;
