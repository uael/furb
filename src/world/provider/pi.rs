//! The protocol of pi-ai itself, which the gateway Radius speaks: a request is the model, the context of a turn and
//! its options, which one post to `<address>/messages` carries, and the reply streams as the events of pi-ai, each a
//! line of data of its own.

#![expect(
  clippy::redundant_closure_for_method_calls,
  reason = "debt: a closure where a method path serves"
)]

use std::collections::VecDeque;

use futures::StreamExt;
use rig_core::{
  completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse, Usage,
    message::{AssistantContent, DocumentSourceKind, Message, MimeType, Reasoning, UserContent},
  },
  http_client::{HttpClientExt, Request, ReqwestClient, sse::BoxedStream},
  streaming::{RawStreamingChoice, StreamFinal, StreamingCompletionResponse},
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// The name that pi-ai gives the provider of each turn it keeps.
const NAME: &str = "radius";

/// One model of a gateway that speaks the protocol of pi-ai: the address of the gateway, the credential, and the
/// name of the model.
#[derive(Clone)]
pub(super) struct Pi {
  client: ReqwestClient,
  api: String,
  key: String,
  id: String,
}

impl Pi {
  pub(super) fn new(api: String, key: String, id: String) -> Pi {
    Pi { client: ReqwestClient::default(), api, key, id }
  }

  /// The context of a turn in the words of pi-ai: the system prompt, and each message as pi-ai keeps it.
  fn context(&self, request: &CompletionRequest) -> Value {
    let messages = request.chat_history.iter().filter_map(|message| match message {
      Message::User { content } => {
        let parts = content.iter().filter_map(|part| match part {
          UserContent::Text(text) => Some(json!({"type": "text", "text": text.text})),
          UserContent::Image(image) => match &image.data {
            DocumentSourceKind::Base64(data) => {
              let media = image.media_type.as_ref().map(|one| one.to_mime_type());
              Some(json!({"type": "image", "data": data, "mimeType": media}))
            }
            _ => None,
          },
          _ => None,
        });
        Some(json!({"role": "user", "content": parts.collect::<Vec<_>>(), "timestamp": 0}))
      }
      Message::Assistant { content, .. } => {
        let parts = content.iter().filter_map(|part| match part {
          AssistantContent::Text(text) => Some(json!({"type": "text", "text": text.text})),
          AssistantContent::Reasoning(reasoning) => Some(json!({
            "type": "thinking",
            "thinking": reasoning.display_text(),
            "thinkingSignature": reasoning.first_signature(),
          })),
          _ => None,
        });
        let usage = json!({"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 0,
          "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}});
        Some(json!({
          "role": "assistant", "content": parts.collect::<Vec<_>>(), "api": "pi-messages", "provider": NAME,
          "model": self.id, "usage": usage, "stopReason": "stop", "timestamp": 0,
        }))
      }
      Message::System { .. } => None,
    });
    json!({"systemPrompt": request.preamble, "messages": messages.collect::<Vec<_>>()})
  }
}

impl CompletionModel for Pi {
  async fn completion(
    &self,
    request: CompletionRequest,
  ) -> Result<CompletionResponse, CompletionError> {
    let mut stream = self.stream(request).await?;
    while let Some(part) = stream.next().await {
      part?;
    }
    let raw = stream.response.as_ref().map(|end| end.raw.clone()).unwrap_or_default();
    Ok(CompletionResponse::from(stream).with_raw(raw))
  }

  async fn stream(
    &self,
    request: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let mut options = match &request.additional_params {
      Some(Value::Object(options)) => options.clone(),
      _ => Map::new(),
    };
    if let Some(tokens) = request.max_tokens {
      options.insert("maxTokens".to_owned(), json!(tokens));
    }
    let payload = json!({"model": self.id, "context": self.context(&request), "options": options});
    let asked = Request::post(format!("{}/messages", self.api.trim_end_matches('/')))
      .header("authorization", format!("Bearer {}", self.key))
      .header("accept", "text/event-stream")
      .header("content-type", "application/json")
      .body(payload.to_string())
      .map_err(|no| CompletionError::RequestError(no.into()))?;
    let got = self.client.send_streaming(asked).await?;
    let heard =
      Heard { body: got.into_body(), buffer: Vec::new(), parts: VecDeque::new(), over: false };
    let parts = futures::stream::unfold(heard, |mut heard| async move {
      heard.next().await.map(|part| (part, heard))
    });
    Ok(StreamingCompletionResponse::stream(NAME, Box::pin(parts)))
  }
}

/// A reply as it is heard: the body, what of it is not read yet, the parts it made that are not told yet, and whether
/// it is over.
struct Heard {
  body: BoxedStream,
  buffer: Vec<u8>,
  parts: VecDeque<Result<RawStreamingChoice, CompletionError>>,
  over: bool,
}

impl Heard {
  /// The next part of the reply, and nothing once it is over.
  async fn next(&mut self) -> Option<Result<RawStreamingChoice, CompletionError>> {
    loop {
      if let Some(part) = self.parts.pop_front() {
        return Some(part);
      }
      if self.over {
        return None;
      }
      match self.body.next().await {
        Some(Ok(bytes)) => self.buffer.extend_from_slice(&bytes),
        Some(Err(no)) => return Some(Err(CompletionError::ProviderError(no.to_string()))),
        None => {
          self.over = true;
          let ended =
            CompletionError::ProviderError(format!("{NAME} ended with no end of its reply"));
          return Some(Err(ended));
        }
      }
      // A line ends in a return and a new line, or in a new line; the data of an event holds no return. A chunk may
      // end within a letter, so only the whole events are read as text, and the bytes after them wait.
      self.buffer.retain(|one| *one != b'\r');
      let Some(at) = self.buffer.windows(2).rposition(|two| two == b"\n\n") else { continue };
      let rest = self.buffer.split_off(at + 2);
      let whole = std::mem::replace(&mut self.buffer, rest);
      for block in String::from_utf8_lossy(&whole[..at]).split("\n\n") {
        let data = block.lines().find_map(|line| line.strip_prefix("data:")).map(str::trim);
        let Some(data) = data.filter(|one| *one != "[DONE]") else { continue };
        match serde_json::from_str(data) {
          Ok(event) => self.heard(event),
          Err(no) => {
            self.over = true;
            let why = format!("{NAME} sent an event its protocol forbids: {no}");
            self.parts.push_back(Err(CompletionError::ProviderError(why)));
            break;
          }
        }
      }
    }
  }

  /// The parts that one event of pi-ai makes.
  fn heard(&mut self, event: Event) {
    let part = match event {
      Event::TextStart { content_index } => RawStreamingChoice::TextStart {
        id: content_index.to_string().into(),
        additional_params: None,
      },
      Event::TextDelta { delta } => RawStreamingChoice::Message(delta),
      Event::TextEnd { content_index } => {
        RawStreamingChoice::TextEnd { id: content_index.to_string().into() }
      }
      Event::ThinkingStart { content_index } => RawStreamingChoice::ReasoningStart {
        id: content_index.to_string().into(),
        provider_id: None,
      },
      Event::ThinkingDelta { content_index, delta } => RawStreamingChoice::ReasoningDelta {
        id: content_index.to_string().into(),
        provider_id: None,
        reasoning: delta,
      },
      Event::ThinkingEnd { content_index, content, content_signature } => {
        let reasoning = Some(Reasoning::new_with_signature(&content, content_signature));
        RawStreamingChoice::ReasoningEnd {
          id: content_index.to_string().into(),
          reasoning,
          signature: None,
          wire_sent: true,
        }
      }
      Event::Done { usage: spent } => {
        self.over = true;
        let mut usage = Usage::new();
        (usage.input_tokens, usage.output_tokens) = (spent.input, spent.output);
        (usage.cached_input_tokens, usage.cache_creation_input_tokens) =
          (spent.cache_read, spent.cache_write);
        usage.total_tokens = spent.total_tokens;
        let mut end = StreamFinal::new(NAME, usage);
        end.raw = json!({"cost": spent.cost.map(|one| one.total)});
        RawStreamingChoice::FinalResponse(end)
      }
      Event::Error { error } => {
        self.over = true;
        let why = error.error_message.unwrap_or_else(|| "the gateway failed".to_owned());
        self.parts.push_back(Err(CompletionError::ProviderError(why)));
        return;
      }
      Event::Other => return,
    };
    self.parts.push_back(Ok(part));
  }
}

/// One event of pi-ai. The field that an event gives is required, so an event that lacks it fails
/// the reply. An event of a kind that this does not read is `Other`, and it is ignored.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
enum Event {
  TextStart {
    content_index: u64,
  },
  TextDelta {
    delta: String,
  },
  TextEnd {
    content_index: u64,
  },
  ThinkingStart {
    content_index: u64,
  },
  ThinkingDelta {
    content_index: u64,
    delta: String,
  },
  ThinkingEnd {
    content_index: u64,
    content: String,
    content_signature: Option<String>,
  },
  Done {
    usage: Spent,
  },
  Error {
    error: Failure,
  },
  #[serde(other)]
  Other,
}

/// What a reply spent. A count that the gateway leaves out is none, since it says `{}` of a reply
/// that spent nothing.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Spent {
  input: u64,
  output: u64,
  cache_read: u64,
  cache_write: u64,
  total_tokens: u64,
  cost: Option<Cost>,
}

/// What a reply cost, in dollars.
#[derive(Deserialize)]
struct Cost {
  total: f64,
}

/// The message of a reply that failed, which a gateway may leave out.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Failure {
  error_message: Option<String>,
}

#[cfg(test)]
#[path = "pi.test.rs"]
mod test;
