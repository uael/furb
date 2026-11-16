//! The transcript of the chain of the open thread as the operator reads it: the system prompt,
//! then each turn that its model reads, each piece with its kind, so a view shows the whole of
//! it or the pieces of one kind alone, as the box of the context counts them.

use std::{collections::BTreeSet, iter};

use furb_wire::protocol::{Answer, Frame, Verb};

use crate::{
  Asked, Client, Key,
  context::{Content, pieces},
};

/// The transcript that the operator reads: its thread, the kind that it shows alone, none for the
/// whole, the pieces of its chain once its machine gave them, and the pieces that he unfolded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
  pub key: Key,
  pub only: Option<Content>,
  pub pieces: Option<Vec<Piece>>,
  pub unfolded: BTreeSet<usize>,
}

/// A piece of the transcript: whether the model wrote it, its kind and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
  pub model: bool,
  pub kind: Content,
  pub text: String,
}

impl Client {
  /// The transcript of the chain of the open thread opened, whole or of one kind alone, or closed
  /// when it shows already as it is asked.
  pub fn transcript(&mut self, only: Option<Content>) {
    let Some(key) = self.thread.as_ref().map(|one| one.thread.key.clone()) else { return };
    if self.transcript.as_ref().is_some_and(|one| one.key == key && one.only == only) {
      self.transcript = None;
      return;
    }
    let pieces = self.transcript.take().filter(|one| one.key == key).and_then(|one| one.pieces);
    self.transcript = Some(Transcript { key, only, pieces, unfolded: BTreeSet::new() });
  }

  /// The pieces of the transcript that it shows: each of its kind, or all of them.
  pub fn transcribed(&self) -> Vec<(usize, &Piece)> {
    let Some(transcript) = self.transcript.as_ref() else { return Vec::new() };
    let all = transcript.pieces.iter().flatten().enumerate();
    all.filter(|(_, one)| transcript.only.is_none_or(|only| one.kind == only)).collect()
  }

  /// The ask of the turns of the chain of the transcript, while it holds none.
  pub(crate) fn transcript_due(&self) -> Option<(usize, Verb, Asked)> {
    let transcript = self.transcript.as_ref().filter(|one| one.pieces.is_none())?;
    let asked = Asked::Transcript(transcript.key.clone());
    if self.asked.values().any(|one| *one == asked) {
      return None;
    }
    let (at, life, on) = transcript.key.clone();
    Some((at, Verb::Turns { life, on: Some(on) }, asked))
  }

  /// What a machine gave for the transcript of a thread: the system prompt, then each piece of
  /// its turns. A refusal or a lost machine gives the system prompt alone.
  pub fn transcribe(&mut self, key: &Key, heard: Option<Frame>) {
    let system = self.system;
    let Some(transcript) = self.transcript.as_mut().filter(|one| one.key == *key) else { return };
    let turns = match heard {
      Some(Frame::Reply { answer: Answer::Values(turns), .. }) => turns,
      _ => Vec::new(),
    };
    let first = Piece { model: false, kind: Content::Engine, text: system.to_owned() };
    let rest = pieces(&turns).into_iter().map(|(_, model, kind, text)| Piece {
      model,
      kind,
      text: text.trim_end().to_owned(),
    });
    transcript.pieces = Some(iter::once(first).chain(rest).collect());
  }

  /// A piece of the transcript unfolded whole, or folded again.
  pub fn unfold(&mut self, piece: usize) {
    if let Some(transcript) = self.transcript.as_mut()
      && !transcript.unfolded.remove(&piece)
    {
      transcript.unfolded.insert(piece);
    }
  }
}
