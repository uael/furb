//! The cells that a text takes on the terminal, as the renderer of OpenTUI draws it. The renderer
//! measures by the method that the terminal speaks: a terminal that joins the characters of an
//! emoji draws a joined emoji in two cells, and one that adds up their code points, as tmux, in as
//! many cells as they add up to. The window lays out each row with this measure alone, so a row
//! never ends in another column than the cells that the renderer draws for it.

use std::{cell::RefCell, collections::HashMap};

use opentui::{
  App,
  native::{Context, ContextOptions, WidthMethod},
};
use unicode_segmentation::UnicodeSegmentation;

thread_local! {
  /// The measure of the thread of the app, which is the one thread that lays out rows.
  static MEASURE: RefCell<Measure> = RefCell::new(Measure::default());
}

/// The cells of a text.
pub(crate) trait Cells {
  /// The cells that the text takes in a row.
  fn width(&self) -> usize;
}

impl Cells for str {
  fn width(&self) -> usize {
    if printable(self) {
      self.len()
    } else {
      MEASURE.with_borrow_mut(|measure| measure.width(self))
    }
  }
}

/// Each character of a text, a grapheme cluster as the renderer splits a text, with its cells.
pub(crate) fn characters(text: &str) -> impl Iterator<Item = (&str, usize)> {
  text.graphemes(true).map(|one| (one, of(one)))
}

/// The method of the renderer of an app, which the terminal tells some time after the window
/// opens, so the window reads it again at each change of what it shows.
pub(crate) fn follow(app: &App) {
  let method = app.width_method();
  MEASURE.with_borrow_mut(|measure| {
    if measure.context.is_none() {
      measure.context = Some(app.session().context().clone());
    }
    if measure.method != method {
      measure.method = method;
      measure.known.clear();
      measure.widths.clear();
    }
  });
}

/// Whether each character of a text is one printable ASCII character, which takes one cell under
/// each method.
fn printable(text: &str) -> bool {
  text.bytes().all(|byte| (0x20..0x7f).contains(&byte))
}

/// The cells of one character.
fn of(character: &str) -> usize {
  if printable(character) {
    return character.len();
  }
  MEASURE.with_borrow_mut(|measure| measure.of(character))
}

/// How many texts the measure keeps the cells of before it forgets them all.
const WIDTHS: usize = 4096;

/// The context of the renderer, the method that it measures by, the cells of each character that
/// it measured and of each text, which a change of the method forgets. A layout measures the same
/// rows again at each change, so a text that it measured once costs a lookup, not a split into
/// characters.
#[derive(Default)]
struct Measure {
  context: Option<Context>,
  method: WidthMethod,
  known: HashMap<String, usize>,
  widths: HashMap<String, usize>,
}

impl Measure {
  fn width(&mut self, text: &str) -> usize {
    if let Some(known) = self.widths.get(text) {
      return *known;
    }
    let cells =
      text.graphemes(true).map(|one| if printable(one) { one.len() } else { self.of(one) }).sum();
    if self.widths.len() >= WIDTHS {
      self.widths.clear();
    }
    self.widths.insert(text.to_owned(), cells);
    cells
  }

  fn of(&mut self, character: &str) -> usize {
    if let Some(known) = self.known.get(character) {
      return *known;
    }
    // A layout before the window has an app, as a test of a row, measures in a context of its own.
    if self.context.is_none() {
      self.context = Context::new(ContextOptions::default()).ok();
    }
    // A character that no native context measures, which no renderer then draws either, takes one
    // cell, as the cell that the renderer leaves for what it cannot draw.
    let measured =
      self.context.as_ref().and_then(|one| one.text_width(character, self.method).ok());
    let cells = measured.map_or(1, |cells| usize::try_from(cells).unwrap_or(usize::MAX));
    self.known.insert(character.to_owned(), cells);
    cells
  }
}

#[cfg(test)]
#[path = "cells.test.rs"]
mod tests;
