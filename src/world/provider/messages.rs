//! The turns of a chain as the messages that a model reads.

use std::collections::HashSet;

use rig_core::{
  completion::CompletionError,
  message::{AssistantContent, Message, Reasoning, Text, UserContent},
};

use super::{images::Images, model::Model, watch};
use crate::fact;
use crate::{
  value::{ObjectRef, entry},
  wire,
};
use std::mem;

/// The id of a thought that the provider cut at the most of its turn, or that ended with no word,
/// and gave back to its model to write its word with no more thought.
pub(super) const CUT: &str = "cut";

/// The most characters of a thought that the provider cut that a model that forgets reads again in
/// its next turn.
pub(super) const TAIL: usize = 16_384;

/// The blocks of a turn of a model that forgets its thought, less that thought: but for the last
/// turn, which keeps the end of a thought that the provider cut, [`TAIL`] characters at most, since
/// the model wrote a word from that thought alone, and with no thought of it in the next turn, it
/// thinks again to the most of that turn.
fn forgotten(blocks: Vec<AssistantContent>, latest: bool) -> Vec<AssistantContent> {
  blocks
    .into_iter()
    .filter_map(|one| match one {
      AssistantContent::Reasoning(thought) if latest && thought.id.as_deref() == Some(CUT) => {
        let text = thought.display_text();
        Some(AssistantContent::Reasoning(Reasoning::new(tail(&text, TAIL)).with_id(CUT.to_owned())))
      }
      AssistantContent::Reasoning(_) => None,
      other => Some(other),
    })
    .collect()
}

/// The last characters of a text, this many at most.
pub(super) fn tail(text: &str, most: usize) -> &str {
  let skip = text.chars().count().saturating_sub(most);
  text.char_indices().nth(skip).map_or("", |(at, _)| &text[at..])
}

/// The turns of a chain as messages of rig. The engine phrases every turn as python and the
/// provider renders nothing: a user turn goes as its python and the images that the prompts it
/// opens name, each after the text that names it and once in the request, and one that holds
/// nothing goes not at all; an assistant turn goes as the blocks its provider gave, less its
/// thought for a model that forgets it, as [`forgotten`] says, or as its python when a provider of
/// another kind gave them. A turn of the model whose word the gate refused goes on the user side,
/// as a quote under the finding of its rung, so the model reads it as a word that the chain refused
/// and never as one that it would write again. A model that takes no image refuses a turn that
/// names one.
pub(super) fn messages(
  turns: ObjectRef<'_>,
  model: &Model,
  images: &mut Images,
) -> Result<Vec<Message>, CompletionError> {
  let mut messages = Vec::new();
  // Whether the last turn of the model held nothing but marks, so the next user turn joins the one
  // before it.
  let mut joins = false;
  // The images that an earlier turn gave, which no later turn gives again.
  let mut given = HashSet::new();
  // The word of the turn of the model that the gate refused, which the next user turn holds under
  // its finding.
  let mut quoted: Option<(String, usize)> = None;
  let all = turns.items().unwrap_or_default();
  // The last turn that a model wrote, whose thought that the provider cut a model that forgets
  // still reads. A word that the chain placed in the turns, as a stand, holds no blocks, so it is
  // no turn of the model.
  let last = all.iter().rposition(|turn| {
    entry(turn, 0).and_then(|one| one.as_str()) == Some("assistant")
      && entry(turn, 3).and_then(|one| one.items()).is_some()
  });
  for (at, turn) in all.iter().enumerate() {
    let Some(role) = entry(turn, 0).and_then(|one| one.as_str()) else { continue };
    let python = entry(turn, 1).and_then(|one| one.as_str()).unwrap_or_default();
    if role == "assistant" {
      let next = all.get(at + 1).and_then(|one| entry(one, 1)?.as_str().map(str::to_owned));
      if let Some(place) = next.as_deref().and_then(refusal) {
        // The quote holds the word less the marks of a fence or of a tool call, which the model
        // would write again.
        let word = watch::bare(python).map_or_else(|| python.to_owned(), |(word, _)| word);
        quoted = (!word.trim().is_empty()).then_some((word, place));
        joins = true;
        continue;
      }
      let latest = Some(at) == last;
      let blocks = entry(turn, 3)
        .and_then(|one| serde_json::from_value::<Vec<AssistantContent>>(wire::record(one)).ok())
        .map(|blocks| if model.forgets { forgotten(blocks, latest) } else { blocks })
        .filter(|blocks| !blocks.is_empty())
        .unwrap_or_else(|| vec![AssistantContent::text(python)]);
      let (blocks, bared) = unmarked(blocks);
      joins = bared && !blocks.iter().any(|one| matches!(one, AssistantContent::Text(_)));
      if !joins {
        messages.push(Message::Assistant { id: None, content: blocks });
      }
      continue;
    }
    let held = quoted.take().map(|(word, place)| refused(python, place, &word));
    let python = held.as_deref().unwrap_or(python);
    if python.is_empty() {
      continue;
    }
    let seen = images
      .named(python, &mut given)
      .map_err(|no| CompletionError::ProviderError(no.to_string()))?;
    if !seen.is_empty() && !model.images {
      return Err(CompletionError::ProviderError(format!(
        "{} does not accept images.",
        model.name
      )));
    }
    spoken(&mut messages, python, seen, mem::take(&mut joins));
  }
  Ok(messages)
}

/// A user turn as a message of rig, with the images that it names: joined to the last user turn
/// when the turn of the model between them held nothing but marks, and else a message of its own.
pub(super) fn spoken(
  messages: &mut Vec<Message>,
  python: &str,
  seen: Vec<UserContent>,
  joins: bool,
) {
  if joins
    && let Some(Message::User { content }) = messages.last_mut()
    && let Some(UserContent::Text(was)) =
      content.iter_mut().rev().find(|one| matches!(one, UserContent::Text(_)))
  {
    was.text = format!("{}\n\n{python}", was.text);
    content.extend(seen);
    return;
  }
  // The open of a chain after the setup that it holds goes as a block of its own, so the setup
  // that every chain with no source shares ends a block, which a provider can keep in its cache.
  let texts = match banner(python) {
    Some(at) => vec![&python[..at - 2], &python[at..]],
    None => vec![python],
  };
  let content = texts.into_iter().map(|one| UserContent::Text(Text::new(one))).chain(seen);
  messages.push(Message::User { content: content.collect() });
}

/// Where the open of a chain starts in a user turn, as `#chain3` alone on the line that starts a
/// paragraph after another, which the setup of the chain stands before.
pub(crate) fn banner(python: &str) -> Option<usize> {
  python.match_indices("\n\n#chain").map(|(at, _)| at + 2).find(|&at| {
    let rest = &python[at + "#chain".len()..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && rest[digits..].starts_with('\n')
  })
}

/// Where the finding of a refused rung ends in a user turn that holds one, `#rungN refused` at the
/// start of a paragraph with its findings after it: the end of the line of its findings, or of the
/// quote that holds them.
pub(super) fn refusal(python: &str) -> Option<usize> {
  let mut from = 0;
  let head = loop {
    let at = from + python[from..].find(" refused\n")?;
    let start = python[..at].rfind('\n').map_or(0, |one| one + 1);
    let name = python[start..at].strip_prefix('#').unwrap_or_default();
    if fact::named(name, "rung") && (start == 0 || python[..start].ends_with("\n\n")) {
      break (name, at + " refused\n".len());
    }
    from = at + 1;
  };
  let (name, body) = head;
  let rest = &python[body..];
  let line = |from: usize| rest[from..].find('\n').map_or(rest.len(), |one| from + one);
  let end = if rest.starts_with(&format!("{name}_findings = ")) {
    line(0)
  } else {
    let tag = rest.strip_prefix("<s:")?.split_once('>')?.0;
    line(rest.find(&format!("</s:{tag}>"))?)
  };
  Some(body + end)
}

/// A user turn with the word that the gate refused quoted under the finding that ends at this
/// place, under the name `rungN_word` that a rung tells its word by, so the model reads it as the
/// word of that rung.
pub(super) fn refused(python: &str, place: usize, word: &str) -> String {
  let start = python[..place].rfind("\n\n").map_or(0, |one| one + 2);
  let rung = python[start + 1..].split(' ').next().unwrap_or_default();
  let mut name = format!("{rung}_word");
  while format!("{word}\n").contains(&format!("</s:{name}>\n")) {
    name.push('_');
  }
  let word = word.trim_end_matches('\n');
  format!("{}\n<s:{name}>\n{word}\n</s:{name}>{}", &python[..place], &python[place..])
}

/// The blocks of a turn of a model less the marks of a fence or of a tool call in each text, which
/// [`watch::bare`] drops, with each text that held only marks gone, and whether a mark went. A turn
/// of a record that an older provider made can hold them, as a word that the gate refused for them,
/// and the model reads the finding of the gate in the next turn, and never the marks, which it
/// would write again.
fn unmarked(blocks: Vec<AssistantContent>) -> (Vec<AssistantContent>, bool) {
  let mut bared = false;
  let blocks = blocks
    .into_iter()
    .filter_map(|one| match one {
      AssistantContent::Text(text) => match watch::bare(&text.text) {
        Some((word, _)) => {
          bared = true;
          (!word.trim().is_empty()).then(|| AssistantContent::text(word))
        }
        None => Some(AssistantContent::Text(text)),
      },
      other => Some(other),
    })
    .collect();
  (blocks, bared)
}
