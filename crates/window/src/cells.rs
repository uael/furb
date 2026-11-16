//! The cells that a text takes on the terminal, as the renderer of OpenTUI draws it. The renderer
//! measures by the method that the terminal speaks: a terminal that joins the characters of an
//! emoji draws a joined emoji in two cells, and one that adds up their code points, as tmux, in as
//! many cells as they add up to. The window lays out each row with this measure alone, so a row
//! never ends in another column than the cells that the renderer draws for it.

use std::cell::{Cell, OnceCell};

use opentui::{
  App,
  native::{Context, ContextOptions, WidthMethod},
};
use unicode_segmentation::UnicodeSegmentation;

thread_local! {
  /// The method that the terminal of the app measures by, which the app tells once it knows it.
  static METHOD: Cell<WidthMethod> = const { Cell::new(WidthMethod::Unicode) };
  /// The native context of the renderer, which measures each character: the context of the app,
  /// or, for a layout before the window has an app, as a test of a row, a context of its own. A
  /// character that no context measures, which no renderer then draws either, takes one cell, as
  /// the cell that the renderer leaves for what it cannot draw.
  static CONTEXT: OnceCell<Option<Context>> = const { OnceCell::new() };
}

/// The cells of a text.
pub(crate) trait Cells {
  /// The cells that the text takes in a row.
  fn width(&self) -> usize;
}

impl Cells for str {
  fn width(&self) -> usize {
    if printable(self) { self.len() } else { self.graphemes(true).map(of).sum() }
  }
}

/// Each character of a text, a grapheme cluster as the renderer splits a text, with its cells.
pub(crate) fn characters(text: &str) -> impl Iterator<Item = (&str, usize)> {
  text.graphemes(true).map(|one| (one, of(one)))
}

/// The method of the renderer of an app, which the terminal tells some time after the window
/// opens, so the window reads it again at each change of what it shows.
pub(crate) fn follow(app: &App) {
  CONTEXT.with(|context| {
    context.get_or_init(|| Some(app.session().context().clone()));
  });
  METHOD.with(|method| method.set(app.width_method()));
}

/// Whether each character of a text is one printable ASCII character, which takes one cell under
/// each method.
fn printable(text: &str) -> bool {
  text.bytes().all(|byte| (0x20..0x7f).contains(&byte))
}

/// The cells of one character, as the renderer measures it by the method of the terminal.
fn of(character: &str) -> usize {
  if printable(character) {
    return character.len();
  }
  let method = METHOD.with(Cell::get);
  let measured = CONTEXT.with(|context| {
    let context = context.get_or_init(|| Context::new(ContextOptions::default()).ok());
    context.as_ref().and_then(|one| one.text_width(character, method).ok())
  });
  measured.map_or(1, |cells| usize::try_from(cells).unwrap_or(usize::MAX))
}

#[cfg(test)]
#[path = "cells.test.rs"]
mod tests;
