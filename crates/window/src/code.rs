//! The colors of the code of a thread: the grammar of python reads each block whole, so each
//! statement takes its colors from its whole text and not from the line that a window shows.

use std::{collections::HashMap, rc::Rc, sync::LazyLock};

use opentui::{
  prelude::Span,
  style::{Color, Style},
  syntax::{Grammar, SyntaxTheme, highlight, register},
};

use crate::{
  line::{Line, run},
  tokens::{PROMPT, SYNTAX},
};

/// The numbers of a block of code, by its last number: their columns, two at least, so each block
/// of the window starts its code in one column and the code of a block that grows moves no cell
/// past its ninth line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gutter {
  pub(crate) width: usize,
}

impl Gutter {
  pub(crate) fn of(last: usize) -> Self {
    Self { width: last.to_string().len().max(2) }
  }

  /// The columns of a line of the block inside a box of a count of columns: the box keeps one
  /// blank column on its right, so what stands at the right of a line ends in the column where
  /// each label at the right of the feed ends, whatever the count of digits of its numbers.
  pub(crate) const fn inside(room: usize) -> usize {
    room.saturating_sub(1)
  }
}

/// How many texts the colors keep before they forget them all.
const KEPT: usize = 256;

/// The colors of code by the name of a capture of its grammar, from [`SYNTAX`]: a name that it
/// does not hold takes the color of its longest part that it holds, or the text. The grammar of
/// python, which each widget of the window reads too, as the diff of a review, stands first.
static THEME: LazyLock<SyntaxTheme> = LazyLock::new(|| {
  let python = Grammar::new(tree_sitter_python::LANGUAGE, tree_sitter_python::HIGHLIGHTS_QUERY);
  // A grammar that does not compile leaves each text of python in the color of the text, as a
  // text that the grammar cannot read.
  register("python", python).ok();
  SYNTAX.iter().map(|&(name, color)| (name, Style::new().fg(Color::hex(color)))).collect()
});

/// The theme of the code of the window, with the grammar of python registered.
pub(crate) fn theme() -> &'static SyntaxTheme {
  &THEME
}

/// The colors of each text of python that the window read, so a frame that shows a block again
/// reads no grammar.
#[derive(Default)]
pub(crate) struct Colors {
  kept: HashMap<String, Rc<Vec<Line>>>,
}

impl Colors {
  /// The lines of a text of python, each as its runs on a ground. A text that the grammar cannot
  /// read shows in the color of the text.
  pub(crate) fn lines(&mut self, text: &str, plain: Style) -> Rc<Vec<Line>> {
    self.coded(text, "python", plain)
  }

  /// The lines of a text of a language that the grammars of the window read, as a name of a file
  /// type, `python` or `rust`, each as its runs on a ground. A text of a language that no grammar
  /// reads shows in the color of the text.
  pub(crate) fn coded(&mut self, text: &str, language: &str, plain: Style) -> Rc<Vec<Line>> {
    let key = format!("{language}{plain:?}{text}");
    if let Some(kept) = self.kept.get(&key) {
      return Rc::clone(kept);
    }
    let lines = Rc::new(Self::read(text, language, plain));
    if self.kept.len() >= KEPT {
      self.kept.clear();
    }
    self.kept.insert(key, Rc::clone(&lines));
    lines
  }

  fn read(text: &str, language: &str, plain: Style) -> Vec<Line> {
    let spans =
      highlight(text, language, theme()).unwrap_or_else(|| vec![Span::raw(text.to_owned())]);
    let mut lines: Vec<Line> = vec![Vec::new()];
    for span in spans {
      // `prompt` is the one call that a model answers, where all else runs as written, so it
      // stands out of the code at rest as its light does while it runs.
      let style = plain.patch(span.style);
      let style = if span.content == PROMPT { style.bold() } else { style };
      for (at, part) in span.content.split('\n').enumerate() {
        if at > 0 {
          lines.push(Vec::new());
        }
        if !part.is_empty()
          && let Some(last) = lines.last_mut()
        {
          last.push(run(part.to_owned(), style));
        }
      }
    }
    lines
  }
}
