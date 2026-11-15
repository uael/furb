//! The colors of the code of a thread: the grammar of python reads each block whole, so each
//! statement takes its colors from its whole text and not from the line that a window shows.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use opentui::{
  prelude::Span,
  style::{Color, Style},
  syntax::{Grammar, SyntaxTheme, highlight, register},
};

use crate::{
  line::{Line, run},
  tokens::SYNTAX,
};

/// The numbers of a block of code, by its last number: their columns, two at least, so each block
/// of the window starts its code in one column and the code of a block that grows moves no cell
/// past its ninth line, and how many of those columns no digit takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gutter {
  pub(crate) width: usize,
  spare: usize,
}

impl Gutter {
  pub(crate) fn of(last: usize) -> Self {
    let digits = last.to_string().len();
    let width = digits.max(2);
    Self { width, spare: width - digits }
  }

  /// The columns of a line of the block inside a box of a count of columns: the box keeps blank on
  /// its right as many columns as the numbers leave blank on its left, so the text of the block
  /// stands as far from each side.
  pub(crate) fn inside(self, room: usize) -> usize {
    room.saturating_sub(self.spare)
  }
}

/// How many texts the colors keep before they forget them all.
const KEPT: usize = 256;

/// The colors of each text of python that the window read, so a frame that shows a block again
/// reads no grammar.
pub(crate) struct Colors {
  theme: SyntaxTheme,
  kept: RefCell<HashMap<String, Rc<Vec<Line>>>>,
}

impl Colors {
  /// The colors, with the grammar of python that each widget of the window reads too, as the diff
  /// of a review.
  #[expect(
    clippy::expect_used,
    reason = "the grammar of python of the build compiles, as each colored block of a test shows"
  )]
  pub(crate) fn new() -> Self {
    let python = Grammar::new(tree_sitter_python::LANGUAGE, tree_sitter_python::HIGHLIGHTS_QUERY);
    register("python", python).expect("the grammar of python that the build holds compiles");
    Self { theme: syntax(), kept: RefCell::default() }
  }

  /// The lines of a text of python, each as its runs on a ground. A text that the grammar cannot
  /// read shows in the color of the text.
  pub(crate) fn lines(&self, text: &str, plain: Style) -> Rc<Vec<Line>> {
    let key = format!("{plain:?}{text}");
    if let Some(kept) = self.kept.borrow().get(&key) {
      return Rc::clone(kept);
    }
    let lines = Rc::new(self.read(text, plain));
    let mut kept = self.kept.borrow_mut();
    if kept.len() >= KEPT {
      kept.clear();
    }
    kept.insert(key, Rc::clone(&lines));
    lines
  }

  fn read(&self, text: &str, plain: Style) -> Vec<Line> {
    let spans =
      highlight(text, "python", &self.theme).unwrap_or_else(|| vec![Span::raw(text.to_owned())]);
    let mut lines: Vec<Line> = vec![Vec::new()];
    for span in spans {
      let style = plain.patch(span.style);
      for (at, part) in span.content.split('\n').enumerate() {
        if at > 0 {
          lines.push(Vec::new());
        }
        if !part.is_empty()
          && let Some(last) = lines.last_mut()
        {
          last.push(run(part, style));
        }
      }
    }
    lines
  }
}

/// The colors of code by the name of a capture of its grammar, from [`SYNTAX`]: a name that it
/// does not hold takes the color of its longest part that it holds, or the text.
pub(crate) fn syntax() -> SyntaxTheme {
  SYNTAX.iter().map(|&(name, color)| (name, Style::new().fg(Color::hex(color)))).collect()
}
