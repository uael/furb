//! The colors of the code of a thread: the grammar of python reads each block whole, so each
//! statement takes its colors from its whole text and not from the line that a window shows.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use opentui::style::{Color, Style};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{
  line::{Line, run},
  tokens::SYNTAX,
};

/// How many texts the colors keep before they forget them all.
const KEPT: usize = 256;

/// The grammar of python and the colors of each text that it read, so a frame that shows a block
/// again reads no grammar.
pub(crate) struct Colors {
  config: Option<HighlightConfiguration>,
  kept: RefCell<HashMap<String, Rc<Vec<Line>>>>,
}

impl Colors {
  pub(crate) fn new() -> Self {
    let python = tree_sitter_python::LANGUAGE.into();
    let config =
      HighlightConfiguration::new(python, "python", tree_sitter_python::HIGHLIGHTS_QUERY, "", "");
    let config = config.ok().map(|mut config| {
      config.configure(&SYNTAX.map(|(name, _)| name));
      config
    });
    Self { config, kept: RefCell::default() }
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
    let mut lines: Vec<Line> = vec![Vec::new()];
    let mut styles = vec![plain];
    let mut put = |piece: &str, style: Style| {
      for (at, part) in piece.split('\n').enumerate() {
        if at > 0 {
          lines.push(Vec::new());
        }
        if !part.is_empty()
          && let Some(last) = lines.last_mut()
        {
          last.push(run(part, style));
        }
      }
    };
    let mut highlighter = Highlighter::new();
    let events = self
      .config
      .as_ref()
      .and_then(|config| highlighter.highlight(config, text.as_bytes(), None, |_| None).ok());
    let Some(events) = events else {
      put(text, plain);
      return lines;
    };
    let mut done = 0;
    for event in events {
      match event {
        Ok(HighlightEvent::HighlightStart(one)) => {
          let color = SYNTAX[one.0].1;
          styles.push(plain.fg(Color::hex(color)));
        }
        Ok(HighlightEvent::HighlightEnd) => {
          styles.pop();
        }
        Ok(HighlightEvent::Source { start, end }) => {
          put(&text[start..end], styles.last().copied().unwrap_or(plain));
          done = end;
        }
        // A text that the grammar stops on shows the rest in the color of the text.
        Err(_) => {
          put(&text[done..], plain);
          break;
        }
      }
    }
    lines
  }
}
