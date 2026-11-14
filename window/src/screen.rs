//! The rule from what the window knows to the widgets.
//!
//! A screen keeps a model: what the daemons told the window, and where the hand of the operator
//! is. A part of the screen is two functions: one reads from the model the value that the part
//! shows, and one puts a value on the widgets of the part. The second runs only when the value
//! changes. So a frame of a daemon that a part does not show costs that part one comparison: it
//! touches no widget, and OpenTUI paints only the cells of the widgets that changed.

/// A part of a screen as the screen keeps it: it reads the model, and draws when the value that
/// it shows changed.
type Part<M> = Box<dyn FnMut(&M)>;

/// A model, and the parts that show it.
pub(crate) struct Screen<M> {
  pub(crate) model: M,
  parts: Vec<Part<M>>,
}

impl<M: 'static> Screen<M> {
  pub(crate) fn new(model: M) -> Self {
    Self { model, parts: Vec::new() }
  }

  /// A part of the screen: `shown` reads the value that the part shows from the model, and `draw`
  /// puts a value on the widgets of the part. `draw` runs now, for the first value, then only for
  /// a value that differs from the one before.
  pub(crate) fn part<S: PartialEq + 'static>(
    &mut self,
    shown: impl Fn(&M) -> S + 'static,
    draw: impl Fn(&S) + 'static,
  ) {
    let mut last = None;
    let mut part = move |model: &M| {
      let now = shown(model);
      if last.as_ref() != Some(&now) {
        draw(&now);
        last = Some(now);
      }
    };
    part(&self.model);
    self.parts.push(Box::new(part));
  }

  /// A change of the model, which each part hears, and what the change gives.
  pub(crate) fn tell<T>(&mut self, change: impl FnOnce(&mut M) -> T) -> T {
    let given = change(&mut self.model);
    for part in &mut self.parts {
      part(&self.model);
    }
    given
  }
}

#[cfg(test)]
#[path = "screen.test.rs"]
mod test;
