//! The call of a model for a reply, and the turn that its answer gives.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  collections::HashSet,
  time::{Duration, SystemTime},
};

use futures::{Stream, StreamExt, stream};
use jiff::{Timestamp, Zoned};
use rig_core::{
  completion::{CompletionError, CompletionRequest, CompletionResponse, FinishReason},
  message::{AssistantContent, Message, Reasoning},
  streaming::{StreamFinal, StreamedAssistantContent, StreamingCompletionResponse},
};
use serde_json::{Map, Value};
use tokio::sync::{mpsc, watch as holding};

use super::{
  Told, claude,
  hearer::{Came, Replied, Retried, released, reply_gone},
  messages::CUT,
  model::{Answer, Asked, Model, Streams},
  retry::{Backoff, LIMIT_POLL, LIMIT_SLACK, Limit, Retry, SIGN_POLL, limited, passing},
  thinking::merged,
  watch,
};
use crate::value;
use crate::{
  value::{Object, ObjectRef, entry},
  wire,
};
use std::future;
use std::mem;
use tokio::time;

/// One response of a model of rig, streamed: what it writes is told as it comes, and the response
/// holds it whole, with the raw record of its end, where a model says what the turn cost. A model
/// whose thought passes its budget is asked again, as [`rethought`] says, and so is a model with a
/// budget whose turn ended with a thought and no word, so the chain never reads an empty word from
/// it. A turn that the most it writes ended is not asked again.
pub(super) async fn streamed(
  streams: Streams,
  asked: Asked,
) -> Result<CompletionResponse, CompletionError> {
  let Asked { request, told, thought: budget, .. } = asked;
  let mut stream = streams(request.clone()).await?;
  // A thought whose parts streamed is told already, and a thought that came whole is told as it
  // comes.
  let mut thought = HashSet::new();
  let (mut thinking, mut parts) = (String::new(), 0);
  while let Some(part) = stream.next().await {
    match part? {
      StreamedAssistantContent::Text(text) => told(&text.text, ""),
      StreamedAssistantContent::ReasoningDelta { id, reasoning, .. } => {
        told("", &reasoning);
        thought.insert(id);
        thinking.push_str(&reasoning);
        parts += 1;
        if let Some((most, again)) = &budget
          && parts > *most
        {
          // The call of this stream ends here.
          drop(stream);
          return rethought(&streams, request, again, thinking, parts, &told).await;
        }
      }
      StreamedAssistantContent::Reasoning { reasoning, id } if !thought.contains(&id) => {
        told("", &reasoning.display_text());
      }
      _ => {}
    }
  }
  let response = whole(stream);
  let Some((_, again)) = &budget else { return Ok(response) };
  let blocks = untagged(response.choice.clone());
  if thinking.is_empty() {
    thinking = blocks
      .iter()
      .filter_map(|one| match one {
        AssistantContent::Reasoning(one) => Some(one.display_text()),
        _ => None,
      })
      .collect();
  }
  let ended = !matches!(response.finish_reason(), None | Some(FinishReason::Length));
  if !ended || thinking.trim().is_empty() || !texts(&blocks).trim().is_empty() {
    return Ok(response);
  }
  let parts = response.usage.output_tokens.max(parts);
  rethought(&streams, request, again, thinking, parts, &told).await
}

/// The response of a stream that ended, with the raw record of its end. A model that says each
/// block whole once it streamed it, as the claude command line does, gives those blocks in that
/// record as `settled`, and they are the choice of the response, since a part of the stream may be
/// lost where the whole block holds it.
pub(super) fn whole(stream: StreamingCompletionResponse) -> CompletionResponse {
  let raw = stream.response.as_ref().map(|end| end.raw.clone()).unwrap_or_default();
  let settled = raw.get("settled").and_then(|one| serde_json::from_value(one.clone()).ok());
  let mut response = CompletionResponse::from(stream).with_raw(raw);
  if let Some(settled) = settled {
    response.choice = settled;
  }
  response
}

/// The turn of a model whose thought passed its budget. The provider asks it again with the same
/// turns, its thought given back as a turn of its own that holds that thought and no word, and with
/// the settings of its budget, which turn its thought off, so it writes its word at once. The turn
/// holds that thought, under the id [`CUT`], and the word, and what the model wrote counts each
/// part of that thought.
pub(super) async fn rethought(
  streams: &Streams,
  mut request: CompletionRequest,
  again: &Map<String, Value>,
  thinking: String,
  parts: u64,
  told: &Told,
) -> Result<CompletionResponse, CompletionError> {
  let mut settings = match request.additional_params.take() {
    Some(Value::Object(settings)) => settings,
    _ => Map::new(),
  };
  merged(&mut settings, again);
  request.additional_params = (!settings.is_empty()).then_some(Value::Object(settings));
  let thought = AssistantContent::Reasoning(Reasoning::new(&thinking).with_id(CUT.to_owned()));
  // rig sends no turn that holds no text, so the turn of the thought holds an empty text too.
  let given = vec![thought.clone(), AssistantContent::text("")];
  request.chat_history.push(Message::Assistant { id: None, content: given });
  let mut stream = streams(request).await?;
  while let Some(part) = stream.next().await {
    match part? {
      StreamedAssistantContent::Text(text) => told(&text.text, ""),
      StreamedAssistantContent::ReasoningDelta { reasoning, .. } => told("", &reasoning),
      _ => {}
    }
  }
  let mut response = whole(stream);
  let word = texts(&response.choice);
  response.choice = vec![thought, AssistantContent::text(word)];
  response.usage.output_tokens += parts;
  response.usage.total_tokens += parts;
  Ok(response)
}

/// A text less the marks that wrap it: an opening line of three backticks, alone or with `python`,
/// at its start, and a closing line of three backticks at its end when it has one; or a tag of
/// markup alone on its first line, as `<answer>` or a bare `<`, and each line that is that tag
/// alone, as it opens or closes, since a model of a local server wraps each part of its word in it.
/// A text that no such mark opens gets nothing, so a text before the mark keeps the text as it is,
/// and the mark of a quote is no tag.
pub(super) fn unfenced(text: &str) -> Option<String> {
  let text = text.trim();
  let (opening, rest) = text.split_once('\n').unwrap_or((text, ""));
  let (opening, rest) = (opening.trim_end(), rest.trim_end());
  if matches!(opening, "```" | "```python") {
    let closed =
      rest.strip_suffix("```").filter(|inside| inside.is_empty() || inside.ends_with('\n'));
    return Some(closed.unwrap_or(rest).trim_end().to_owned());
  }
  let name = opening.strip_prefix('<')?;
  let name = name.strip_suffix('>').unwrap_or(name);
  if !name.chars().all(|one| one.is_ascii_alphanumeric() || one == '_') {
    return None;
  }
  let tag = |line: &str| {
    let line = line.trim();
    let bare = line.strip_prefix("</").or_else(|| line.strip_prefix('<'));
    bare.is_some_and(|inside| inside.strip_suffix('>').unwrap_or(inside) == name)
  };
  let kept: Vec<&str> = rest.lines().filter(|line| !tag(line)).collect();
  Some(kept.join("\n").trim_end().to_owned())
}

/// What the chain is told after the rung of a turn whose word lost the line of three backticks at
/// its end.
const UNCLOSED: &str = "wrote a line of three backticks alone at the end of its word, and the provider dropped that \
  line: a reply is python and quotes only, with no fence";

/// A word less the line of three backticks alone at its end, which a model writes out of the habit
/// of a fence that no line opened. A word gets nothing when its last line is not that fence, when
/// its first line opens a fence, which the gate refuses whole, when a quote stands open before that
/// line, whose text the line may end, or when nothing stands before that line.
pub(super) fn unclosed(text: &str) -> Option<String> {
  let (word, last) = text.trim_end().rsplit_once('\n')?;
  let opens = word.trim_start().starts_with("```");
  let fence = last.trim() == "```" && !opens && !watch::open(word) && !word.trim().is_empty();
  fence.then(|| word.trim_end().to_owned())
}

/// The terminal record of a stream of a provider, from a response of it that came whole.
pub(super) fn ended(provider: &str, response: CompletionResponse) -> StreamFinal {
  let mut record = StreamFinal::new(provider, response.usage);
  record.finish_reason = response.finish_reason();
  record.response_id = response.response_id;
  record.model = response.model;
  record.raw = response.raw;
  record
}

/// The work of a reply: each part its model writes, each retry of the call of its model, then the
/// turn of the model or why it answered nothing, in the order they came; and nothing more once the
/// reply went, which ends the call of the model. A call that failed for a cause that passes is made
/// again after the wait of its backoff, until its attempts are spent. A call that a usage limit
/// refused is made again at the reset that the limit names, however long that takes, and each such
/// wait is no attempt. A call that a failed sign in refused is made again every minute until it
/// succeeds or a stand asks it at once, since a sign in outside furb ends the failure.
pub(super) fn answered(call: Answering) -> impl Stream<Item = Replied> {
  let Answering { id, asked, backoff, tells, mut told, hold, again } = call;
  let came = async move {
    let mut waits = Waits { id, backoff, tells, hold, attempt: 1, limited: false, signed: false };
    let got = match asked {
      Ok((model, asked)) => waits.attempted(&model, &asked, again).await,
      Err(no) => Err(no),
    };
    let (got, dropped) = match got {
      Ok((turn, dropped)) => (Ok(turn), dropped),
      Err(no) => (Err(no), None),
    };
    let Waits { id, tells, .. } = waits;
    // The stream holds the receiver, so a send that fails means the stream is going.
    // The stream then gives the reply itself.
    tells.send(Replied::Came(Came { id, got, dropped })).err().map(|no| no.0)
  };
  // The reply comes through the channel alone, so it comes after all that its model wrote before
  // it.
  let told = stream::poll_fn(move |cx| told.poll_recv(cx));
  let came = stream::once(came).filter_map(future::ready);
  stream::select(told, came)
}

/// The work of the call of the model of a reply: the id of the reply, its model and what it asks,
/// the backoff of its retries, the two ends of the channel through which it tells what the model
/// writes, whether a pause holds the reply, and whether it asks again a call that waited for a
/// retry.
pub(super) struct Answering {
  pub(super) id: String,
  pub(super) asked: Result<(Model, Asked), CompletionError>,
  pub(super) backoff: Backoff,
  pub(super) tells: mpsc::UnboundedSender<Replied>,
  pub(super) told: mpsc::UnboundedReceiver<Replied>,
  pub(super) hold: holding::Receiver<bool>,
  pub(super) again: bool,
}

/// The waits of the call of the model of a reply between its attempts: the id of the reply, the
/// backoff of its retries, where it tells each retry, whether a pause holds the reply, the attempt
/// that runs, and whether a usage limit or a failed sign in refused an earlier attempt.
struct Waits {
  id: String,
  backoff: Backoff,
  tells: mpsc::UnboundedSender<Replied>,
  hold: holding::Receiver<bool>,
  attempt: u32,
  limited: bool,
  signed: bool,
}

impl Waits {
  /// The turn of the model, asked again after the wait of each retry, until an attempt answers or
  /// a failure ends the call.
  async fn attempted(
    &mut self,
    model: &Model,
    asked: &Asked,
    again: bool,
  ) -> Result<(Object, Option<String>), CompletionError> {
    if again {
      released(&mut self.hold).await?;
    }
    loop {
      let no = match (model.answers)(asked.clone()).await {
        Ok(Answer::Response(response)) => return turn(model, &response),
        Ok(Answer::Turn(turn)) => return hosted(turn.as_ref()).map(|one| (one, None)),
        Err(no) => no,
      };
      let (retry, limit) = self.retry(no)?;
      let wait = retry.wait;
      let retried = Retried { id: self.id.clone(), retry, limit };
      self.tells.send(Replied::Retried(retried)).map_err(reply_gone)?;
      time::sleep(wait).await;
      released(&mut self.hold).await?;
    }
  }

  /// The retry after a failed attempt, and whether the chain is told of its wait, which it is of
  /// the first wait of a usage limit and of a failed sign in; or the failure, which ends the call.
  fn retry(&mut self, no: CompletionError) -> Result<(Retry, bool), CompletionError> {
    let most = self.backoff.most;
    let now = Zoned::now();
    if let Some(Limit { until }) = limited(&no, SystemTime::from(now.timestamp())) {
      let reset = until
        .and_then(|at| Timestamp::try_from(at).ok())
        .map(|at| at.to_zoned(now.time_zone().clone()));
      let wait = reset.as_ref().map_or(LIMIT_POLL, |at| {
        let seconds = at.timestamp().as_second() - now.timestamp().as_second();
        Duration::from_secs(u64::try_from(seconds).unwrap_or(0)) + LIMIT_SLACK
      });
      // The call that follows the end of a limit and meets it again waits a short time, not the
      // next day.
      let wait = if self.limited { wait.min(LIMIT_POLL) } else { wait };
      let until =
        reset.map_or(String::new(), |at| at.strftime(" until %Y-%m-%d %H:%M (%Q)").to_string());
      let why = format!("usage limit{until}");
      let retry = Retry { attempt: self.attempt + 1, most, wait, why };
      return Ok((retry, !mem::replace(&mut self.limited, true)));
    }
    if let Some(
      claude::Error::SignedOut(_) | claude::Error::NoToken(_) | claude::Error::Exhausted,
    ) = claude::error(&no)
    {
      let why = "signed out, sign in again".to_owned();
      let retry = Retry { attempt: self.attempt + 1, most, wait: SIGN_POLL, why };
      return Ok((retry, !mem::replace(&mut self.signed, true)));
    }
    let Some(why) = passing(&no).filter(|_| self.attempt < most) else { return Err(no) };
    self.attempt += 1;
    let wait = self.backoff.wait(self.attempt);
    Ok((Retry { attempt: self.attempt, most, wait, why: why.to_string() }, false))
  }
}

/// The turn of a model: its text, which is the word of the rung, what it read and wrote, and the
/// blocks it gave.
///
/// A model speaks python alone, so prose around the code stays in the word, for the gate to refuse,
/// but for the marks of a fence, of a tool call or of a break of HTML around it, which
/// [`watch::bare`] drops with the name that it gives a first binding of no name, and a line of
/// three backticks alone at its end, which [`unclosed`] drops, since they hold no code that runs
/// and a model writes them out of habit; a turn of a model that may wrap its word in a fence, as a
/// model of a local server, loses instead the fence that opens its text, and the closing fence at
/// its end when it has one. What it read is every token of its input, the cache among them, and
/// what it wrote is the rest of its total, its thought among it. A model that says why each turn
/// ended, and says nothing of the end of this one or says that the most it writes ended it, was
/// cut, so its turn is refused, and no more of its word runs than the parts that ran as it wrote
/// them, whose word the ear gives in place of the refusal. The thought that a model writes in its
/// text, before its word, is a block of reasoning of its turn.
pub(super) fn turn(
  model: &Model,
  response: &CompletionResponse,
) -> Result<(Object, Option<String>), CompletionError> {
  let cut = match response.finish_reason() {
    _ if !model.ends => None,
    None => Some(format!("the stream of {} stopped before its end", model.name)),
    Some(FinishReason::Length) => {
      Some(format!("the turn of {} reached the most it writes before its end", model.name))
    }
    Some(_) => None,
  };
  if let Some(why) = cut {
    return Err(CompletionError::ResponseError(format!("{why}: {}", wrote(model, response))));
  }
  let mut choice: Vec<AssistantContent> = response.choice.clone();
  if model.tagged {
    choice = untagged(choice);
  }
  let mut text = texts(&choice);
  let mut dropped = None;
  if model.fenced
    && let Some(word) = unfenced(&text)
  {
    (choice, text) = (worded(choice, &word), word);
  } else if let Some((word, note)) = watch::bare(&text) {
    (choice, text, dropped) = (worded(choice, &word), word, Some(note));
  } else if let Some(word) = unclosed(&text) {
    (choice, text, dropped) = (worded(choice, &word), word, Some(UNCLOSED.to_owned()));
  }
  let spent = response.usage;
  let (read, wrote) = (spent.cached_input_tokens, spent.cache_creation_input_tokens);
  let input = spent.input_tokens + if model.apart { read + wrote } else { 0 };
  let output =
    spent.total_tokens.checked_sub(input).filter(|one| *one > 0).unwrap_or(spent.output_tokens);
  let priced = model.price.map(|[fresh, out, cached, cache]| {
    let fresh = counted(input.saturating_sub(read + wrote)) * fresh;
    (fresh + counted(output) * out + counted(read) * cached + counted(wrote) * cache) / 1e6
  });
  let dollars = response.raw.get("cost").and_then(Value::as_f64).or(priced).unwrap_or_default();
  let failed = |why: String| CompletionError::ResponseError(why);
  let blocks = serde_json::to_value(&choice).map_err(|no| failed(no.to_string()))?;
  let blocks = wire::inward(&blocks).map_err(|fault| failed(fault.message()))?;
  let count = |n: u64| Object::int(i64::try_from(n).unwrap_or(i64::MAX));
  let usage =
    Object::tuple([count(input), count(output), count(read), count(wrote), Object::float(dollars)]);
  let turn =
    Object::tuple([Object::string("assistant"), Object::string(text.trim()), usage, blocks]);
  Ok((turn, dropped))
}

/// These blocks with the word as their one text, at the place of their first text.
pub(super) fn worded(choice: Vec<AssistantContent>, word: &str) -> Vec<AssistantContent> {
  let mut worded = Vec::new();
  for one in choice {
    match one {
      AssistantContent::Text(_)
        if worded.iter().any(|one| matches!(one, AssistantContent::Text(_))) => {}
      AssistantContent::Text(_) => worded.push(AssistantContent::text(word)),
      other => worded.push(other),
    }
  }
  worded
}

/// The turn of a reply that ended at the first statement of its word that cannot be python: the
/// word as far as that statement, as the turn of a model that wrote nothing more. It counts
/// nothing, since the usage of a stream comes with its end.
pub(super) fn cut(word: &str) -> Result<Object, CompletionError> {
  let word = word.trim();
  let failed = |why: String| CompletionError::ResponseError(why);
  let blocks =
    serde_json::to_value([AssistantContent::text(word)]).map_err(|no| failed(no.to_string()))?;
  let blocks = wire::inward(&blocks).map_err(|fault| failed(fault.message()))?;
  let zero = || Object::int(0);
  let usage = Object::tuple([zero(), zero(), zero(), zero(), Object::float(0.0)]);
  Ok(Object::tuple([Object::string("assistant"), Object::string(word), usage, blocks]))
}

/// How many characters of the end of a cut turn its refusal shows.
const END: usize = 400;

/// What a cut turn wrote, which its refusal says, since the record keeps the refusal and no part of
/// the turn: the tokens it wrote, the characters of its thought and of its text, and the end of the
/// last of the two that it wrote. A thought in the text whose close tag never came is thought.
pub(super) fn wrote(model: &Model, response: &CompletionResponse) -> String {
  let mut choice: Vec<AssistantContent> = response.choice.clone();
  if model.tagged {
    choice = untagged(choice);
  }
  let mut thought: String = choice
    .iter()
    .filter_map(|one| match one {
      AssistantContent::Reasoning(one) => Some(one.display_text()),
      _ => None,
    })
    .collect();
  let mut text = texts(&choice);
  if model.tagged && text.trim_start().starts_with("<think>") && !text.contains("</think>") {
    thought.push_str(&text);
    text.clear();
  }
  let last: Vec<char> =
    if text.trim().is_empty() { &thought } else { &text }.trim_end().chars().collect();
  let end: String = last[last.len().saturating_sub(END)..].iter().collect();
  let (thought, text) = (thought.chars().count(), text.chars().count());
  let tokens = response.usage.output_tokens;
  format!(
    "it wrote {tokens} tokens, {thought} characters of thought and {text} of text, which end with {end:?}"
  )
}

/// The text of these blocks, whole.
pub(super) fn texts(choice: &[AssistantContent]) -> String {
  let text = choice.iter().filter_map(|one| match one {
    AssistantContent::Text(text) => Some(text.text.as_str()),
    _ => None,
  });
  text.collect()
}

/// The blocks of a turn whose text starts with a thought between `<think>` and `</think>`, as a
/// server with no reader of thought writes it: at the place of its first text, the thought as a
/// block of reasoning when it holds any, and the text after it.
fn untagged(choice: Vec<AssistantContent>) -> Vec<AssistantContent> {
  let text = texts(&choice);
  let Some((thought, word)) =
    text.trim_start().strip_prefix("<think>").and_then(|rest| rest.split_once("</think>"))
  else {
    return choice;
  };
  let (thought, word) = (thought.trim().to_owned(), word.trim().to_owned());
  let mut blocks = Vec::new();
  for one in choice {
    match one {
      AssistantContent::Text(_)
        if blocks.iter().any(|one| matches!(one, AssistantContent::Text(_))) => {}
      AssistantContent::Text(_) => {
        if !thought.is_empty() {
          blocks.push(AssistantContent::Reasoning(Reasoning::new(&thought)));
        }
        blocks.push(AssistantContent::text(&word));
      }
      other => blocks.push(other),
    }
  }
  blocks
}

/// A count of tokens as a float, which holds it whole up to `u32::MAX`, more than a turn counts.
fn counted(tokens: u64) -> f64 {
  f64::from(u32::try_from(tokens).unwrap_or(u32::MAX))
}

/// The turn that a function of the host gave, as a turn of a model: its role, its python, its
/// usage, whose dollars are a float however the host wrote them, and its blocks.
pub(super) fn hosted(turn: ObjectRef<'_>) -> Result<Object, CompletionError> {
  let part = |at: usize| entry(&turn, at).map_or_else(Object::none, |one| one.to_owned());
  let (role, python) = (entry(&turn, 0), entry(&turn, 1));
  let whole = turn.items().is_some_and(|items| items.len() == 4)
    && role.and_then(|one| one.as_str()) == Some("assistant")
    && python.is_some_and(|one| one.as_str().is_some());
  if !whole {
    let why = format!(
      "a host answers with a turn: ('assistant', python, usage, blocks), not {}",
      turn.py_repr()
    );
    return Err(CompletionError::ResponseError(why));
  }
  let usage = entry(&turn, 2).and_then(|one| one.items()).map(|usage| {
    let each = usage.into_iter().enumerate().map(|(at, one)| match (at, one.as_int()) {
      (4, Some(whole)) => Object::float(
        i32::try_from(whole)
          .map_or(if whole < 0 { f64::from(i32::MIN) } else { f64::from(i32::MAX) }, f64::from),
      ),
      _ => one.to_owned(),
    });
    Object::tuple(each)
  });
  Ok(Object::tuple([part(0), part(1), usage.unwrap_or_else(Object::none), part(3)]))
}

/// A new random id, a uuid of version 4, as the claude command line takes one for a conversation.
/// A machine that gives no random bytes fails the call that needs the id.
pub(super) fn uuid() -> Result<String, CompletionError> {
  uuid_from(getrandom::fill)
}

/// The uuid that the bytes of `fill` make.
pub(super) fn uuid_from(
  fill: impl FnOnce(&mut [u8]) -> Result<(), getrandom::Error>,
) -> Result<String, CompletionError> {
  let mut bytes = [0u8; 16];
  fill(&mut bytes).map_err(|no| {
    CompletionError::ProviderError(format!("the machine gave no random bytes: {no}"))
  })?;
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  let digits = value::hex(&bytes);
  Ok(format!(
    "{}-{}-{}-{}-{}",
    &digits[..8],
    &digits[8..12],
    &digits[12..16],
    &digits[16..20],
    &digits[20..]
  ))
}

/// The numbers of a text, in order, which sort as a version does: `1.10.0` is greater than `1.4.0`.
pub(super) fn numbers(text: &str) -> Vec<u64> {
  text.split(|one: char| !one.is_ascii_digit()).filter_map(|part| part.parse().ok()).collect()
}
