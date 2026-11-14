//! A loader: one cell that moves while something works. The package of OpenTUI has no such widget.

use std::rc::Rc;

use opentui::{
  App,
  prelude::{AnimationTick, Color, Text},
  widget::{Element, Widget},
};

use crate::tokens::Loader;

/// A loader of one cell, which shows the next frame of its mark at each step. It moves on the
/// animation clock of the app, which all the loaders share, so two loaders of one step move in the
/// same frame. Each step sets the text of that one cell and nothing else, so nothing else is
/// painted. A loader that is hidden, or that no frame draws, asks no frame and wakes no loop.
#[derive(Clone)]
pub(crate) struct Spinner {
  mark: Text,
  /// The steps of the loader, which end when its last handle goes.
  _steps: Rc<AnimationTick>,
}

impl Spinner {
  pub(crate) fn new(app: &App, loader: Loader, color: Color) -> Self {
    let frames = loader.frames();
    let mark = Text::new(app, frames[0]).fg(color);
    let mut at = 0;
    let steps = mark.on_animation_tick(1.0 / loader.step().as_secs_f64(), {
      let mark = mark.clone();
      move |_, tick| {
        // A frame that came late covers more than one step.
        at = (at + usize::try_from(tick.ticks).unwrap_or(1)) % frames.len();
        mark.set_text(frames[at]);
      }
    });
    Self { mark, _steps: Rc::new(steps) }
  }
}

impl Widget for Spinner {
  fn element(&self) -> &Element {
    self.mark.element()
  }
}

#[cfg(test)]
#[path = "spinner.test.rs"]
mod test;
