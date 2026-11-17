//! The transcript of the chain of the open thread over the whole window, as the review of the
//! changes stands: a head that names its thread and the kind that it shows, then the system prompt
//! and each turn that the chain reads, as Python, each under a row that says who wrote it and its
//! kind, and a foot of its keys. A piece longer than a screen folds to its first lines, and a click
//! on the row under them opens it whole or folds it again.

use furb_client::{Content, Transcript};
use opentui::style::Style;

use crate::{
  keys::Does,
  line::{Line, run, spread},
  model::Model,
  tokens::Role,
};

/// The rows of a piece past which it folds, and the rows that a folded piece shows.
const FOLDS: usize = 24;
const SHOWS: usize = 8;

/// The transcript as the window draws it: its head, its rows, its foot, and the piece that a click
/// on each row folds or opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Transcribed {
  pub(crate) head: Line,
  pub(crate) body: Vec<Line>,
  pub(crate) foot: Line,
  pub(crate) folds: Vec<Option<usize>>,
}

impl Model {
  /// The transcript, while one stands.
  pub(crate) fn transcribing(&mut self) -> Option<Transcribed> {
    let transcript = self.client.transcript.as_ref()?;
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let (mut body, mut folds) = (Vec::new(), Vec::new());
    let mut push = |line: Line, fold: Option<usize>| {
      body.push(line);
      folds.push(fold);
    };
    if transcript.pieces.is_none() {
      push(vec![run("  Reading the turns of the chain…", muted)], None);
    }
    let shown = self.client.transcribed();
    if transcript.pieces.is_some() && shown.is_empty() {
      push(vec![run("  No piece of this kind in the turns of the chain.", muted)], None);
    }
    for (at, piece) in &shown {
      let who = match (piece.model, piece.kind) {
        (_, Content::Engine) => "the system prompt",
        (true, _) => "the model",
        (false, _) => "furb",
      };
      let color = self.kind_color(piece.kind);
      push(
        vec![
          run("  ■ ", Style::new().fg(color)),
          run(format!("{who} · {}", piece.kind.name()), muted),
        ],
        None,
      );
      let lines = self.kept.colors.lines(&self.homed(&piece.text), prose);
      let open = transcript.unfolded.contains(at);
      let folds_here = lines.len() > FOLDS;
      let count = if folds_here && !open { SHOWS } else { lines.len() };
      for line in lines.iter().take(count) {
        let mut row = vec![run("    ", Style::new())];
        row.extend(line.iter().cloned());
        push(row, None);
      }
      if folds_here {
        let says = if open {
          "    ▴ fold".to_owned()
        } else {
          format!("    … {} more lines · a click opens them", lines.len() - count)
        };
        push(vec![run(says, muted)], Some(*at));
      }
      push(Vec::new(), None);
    }
    let (head, foot) = self.transcript_ends(transcript, shown.len());
    Some(Transcribed { head, body, foot, folds })
  }

  /// The head of the transcript, which names its thread, its kind and the count of its pieces, and
  /// its foot of keys.
  fn transcript_ends(&self, transcript: &Transcript, pieces: usize) -> (Line, Line) {
    let muted = self.tokens.style(Role::Muted);
    let thread = self.title().unwrap_or_default();
    let kind = transcript.only.map_or("the whole chain", Content::name);
    let left = vec![
      run("  Transcript · ", muted),
      run(thread, self.tokens.style(Role::Title)),
      run(format!(" · {kind}"), muted),
    ];
    let count = transcript.pieces.as_ref().map(|_| format!("{pieces} pieces"));
    let head = spread(left, vec![run(count.unwrap_or_default(), muted)], self.wide(), Style::new());
    let whole = if transcript.only.is_some() {
      format!("{} the whole chain · ", self.key(Does::Transcript))
    } else {
      String::new()
    };
    let keys = format!(
      "  {}{} scroll · {whole}{} close",
      self.key(Does::PageUp),
      self.key(Does::PageDown),
      self.key(Does::Close)
    );
    (head, vec![run(keys, muted)])
  }

  /// A click on a row of the transcript: the row under a long piece folds it or opens it whole.
  pub(crate) fn transcript_clicked(&mut self, row: usize) {
    let fold = self.transcribing().and_then(|one| one.folds.get(row).copied().flatten());
    if let Some(piece) = fold {
      self.client.unfold(piece);
    }
  }
}
