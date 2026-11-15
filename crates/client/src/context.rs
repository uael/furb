//! The kinds of what the last answer of the chain of a thread read: the engine, which is its system
//! prompt, the doctrine, the code of the kits, the skills, the tools of MCP, the memory files
//! and the turns of the work, which the client parts from the turns of the chain while the box of
//! the context shows them.

use std::mem;

use furb_wire::protocol::{Answer, Verb};
use serde_json::Value;

use crate::{Asked, Client, Fold, Key};

/// A kind of content of what an answer read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
  Engine,
  Doctrine,
  Kits,
  Skills,
  Mcp,
  Memory,
  Turns,
}

impl Content {
  /// Each kind, in the order of the bar, which is the order in which the chain reads them: its
  /// system prompt first, then its setup, then the turns of the work at its end.
  pub const ALL: [Self; 7] =
    [Self::Engine, Self::Doctrine, Self::Kits, Self::Skills, Self::Mcp, Self::Memory, Self::Turns];

  /// The name of the kind, as the box writes it.
  pub const fn name(self) -> &'static str {
    match self {
      Self::Engine => "Engine",
      Self::Doctrine => "Doctrine",
      Self::Kits => "Kits",
      Self::Skills => "Skills",
      Self::Mcp => "MCP",
      Self::Memory => "Memory files",
      Self::Turns => "Turns",
    }
  }
}

/// How the box of the context splits what the last answer read: by its kinds, or by what came
/// from the cache, what the answer wrote to it and what was new.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Split {
  #[default]
  Kinds,
  Cache,
}

/// The kinds of what an answer read, each with its tokens in the order of the bar, and the tokens
/// of that answer, so the client asks them again once a later answer reads more. No kind stands
/// when the turns held no answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parts {
  pub read: u64,
  pub kinds: Vec<(Content, u64)>,
}

impl Client {
  /// The ask of the turns of the chain of the open thread, while its box shows the kinds and holds
  /// none for its last answer. A box that is folded asks nothing.
  pub(crate) fn parts_due(&self) -> Option<(usize, Verb, Asked)> {
    let opened = self.thread.as_ref().filter(|one| one.unfolded.contains(&Fold::Context))?;
    let thread = &opened.thread;
    let read = thread.context.as_ref()?.read.tokens;
    let held = thread.parts.as_ref().is_some_and(|one| one.read == read);
    let asked = Asked::Parts(thread.key.clone(), read);
    let asking = self.asked.values().any(|one| *one == asked);
    let (life, on) = (thread.key.1.clone(), Some(thread.key.2.clone()));
    (self.split == Split::Kinds && !held && !asking)
      .then(|| (thread.key.0, Verb::Turns { life, on }, asked))
  }

  /// The turns that a machine gave for the box of a thread, parted by kind; a refusal parts
  /// nothing, so the client does not ask again until a later answer.
  pub fn parted(&mut self, key: &Key, read: u64, answer: Option<Answer>) {
    let Some(opened) = self.thread.as_mut().filter(|one| one.thread.key == *key) else { return };
    let kinds = match answer {
      Some(Answer::Values(turns)) => kinds(&turns),
      _ => Vec::new(),
    };
    opened.thread.parts = Some(Parts { read, kinds });
  }

  /// The box shows the kinds when it showed the cache, and the cache when it showed the kinds.
  pub fn split_again(&mut self) {
    self.split = match self.split {
      Split::Kinds => Split::Cache,
      Split::Cache => Split::Kinds,
    };
  }
}

/// How many characters a token holds where the turns give no second answer to measure it: about
/// what the tokenizers of the models read in code and in English.
const CHARACTERS: f64 = 3.5;

/// The kinds of what the last answer of turns read, which sum to the tokens that it read. The
/// usage of an answer counts its system prompt and the turns before it, but no kind: so the
/// characters of each kind of turn before the last answer are counted, the first and the last
/// answers give the tokens of a character, and the engine takes what the first answer read beyond
/// its turns.
fn kinds(turns: &[Value]) -> Vec<(Content, u64)> {
  let answers: Vec<(usize, f64)> = turns
    .iter()
    .enumerate()
    .filter(|(_, turn)| turn[0] == "assistant")
    .filter_map(|(at, turn)| Some((at, turn[2][0].as_f64()?)))
    .collect();
  let (Some(&(first, opened)), Some(&(last, read))) = (answers.first(), answers.last()) else {
    return Vec::new();
  };
  let mut characters = [0.0; Content::ALL.len()];
  let mut before = 0.0;
  let mut kit = false;
  for (at, turn) in turns[..last].iter().enumerate() {
    let python = turn[1].as_str().unwrap_or_default();
    let parts = if turn[0] == "assistant" {
      // The setup reads the code of each kit in a rung of its own, and tells it whole.
      kit = python.starts_with("rung(read(") && python.ends_with(", HIDDEN).content)");
      vec![(Content::Turns, python)]
    } else if mem::take(&mut kit) {
      vec![(Content::Kits, python)]
    } else {
      paragraphs(python).into_iter().map(|paragraph| (kind(paragraph), paragraph)).collect()
    };
    for (kind, text) in parts {
      characters[slot(kind)] += count(text);
      before += if at < first { count(text) } else { 0.0 };
    }
  }
  let all: f64 = characters.iter().sum();
  let rate =
    if all > before && read > opened { (read - opened) / (all - before) } else { 1.0 / CHARACTERS };
  let engine = opened - rate * before;
  let (engine, rate) = if engine < 0.0 { (0.0, read / all.max(1.0)) } else { (engine, rate) };
  let mut kinds: Vec<(Content, u64)> =
    Content::ALL.into_iter().map(|kind| (kind, tokens(characters[slot(kind)] * rate))).collect();
  kinds[slot(Content::Engine)].1 = tokens(engine);
  // The turns take what the rounding left, so the kinds sum to what the answer read.
  let turns = slot(Content::Turns);
  let others: u64 = kinds.iter().enumerate().filter(|one| one.0 != turns).map(|one| one.1.1).sum();
  kinds[turns].1 = tokens(read).saturating_sub(others);
  kinds.retain(|one| one.1 > 0);
  kinds
}

/// The place of a kind in the order of the bar.
fn slot(kind: Content) -> usize {
  Content::ALL.iter().position(|one| *one == kind).unwrap_or_default()
}

/// The kind of a paragraph of a user turn, by the path that it tells or by its act.
fn kind(paragraph: &str) -> Content {
  if paragraph.starts_with("#mcp") {
    return Content::Mcp;
  }
  let path = paragraph
    .lines()
    .take(3)
    .find_map(|line| line.split_once("_path = '").map(|(_, path)| path.trim_end_matches('\'')));
  let name = path.map(|path| path.rsplit(['/', '\\']).next().unwrap_or(path));
  match (path, name) {
    (Some("skills://"), _) => Content::Skills,
    (_, Some("DOCTRINE.md")) => Content::Doctrine,
    (_, Some("AGENTS.md" | "CLAUDE.md")) => Content::Memory,
    _ => Content::Turns,
  }
}

/// The paragraphs of a user turn, each from the line of its head, as `#read4`. A heading of
/// markdown in a text that a paragraph quotes, as `# Title`, starts none.
fn paragraphs(python: &str) -> Vec<&str> {
  let heads = python.match_indices("\n\n#").map(|(at, _)| at + 2).filter(|at| {
    python[at + 1..].starts_with(|next: char| next.is_ascii_alphanumeric() || next == '_')
  });
  let starts: Vec<usize> = [0].into_iter().chain(heads).chain([python.len()]).collect();
  starts.windows(2).map(|pair| &python[pair[0]..pair[1]]).collect()
}

/// The characters of a text.
#[expect(clippy::cast_precision_loss, reason = "a turn holds far fewer characters than 2^52")]
fn count(text: &str) -> f64 {
  text.chars().count() as f64
}

/// A count of tokens, rounded to the nearest whole token.
#[expect(
  clippy::cast_possible_truncation,
  clippy::cast_sign_loss,
  reason = "the count is a share of what an answer read, a whole number of at least 0"
)]
fn tokens(count: f64) -> u64 {
  count.max(0.0).round() as u64
}
