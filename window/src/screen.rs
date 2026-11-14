//! The rule from what the daemon told to the widgets.
//!
//! A screen keeps what the daemon told the window, and nothing that the window made. A part of the
//! screen is two functions: one reads from that the value that the part shows, and one puts a
//! value on the widgets of the part. The second runs only when the value changes. So a frame of
//! the daemon that a part does not show costs that part one comparison: it touches no widget, and
//! OpenTUI paints only the cells of the widgets that changed. A loader moves by its own tick and
//! is no part.

use furb_host::protocol::{Answer, Frame};

use crate::link::Heard;

/// What the daemon of a machine told the window.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum Told {
  /// No welcome yet: the window dials the daemon, which starts when none listens.
  #[default]
  Dialing,
  /// The welcome of the daemon, with its protocol, and the count of the records of the machine once
  /// the daemon listed them.
  Welcomed { protocol: u32, records: Option<usize> },
  /// Why the connection ended, or why the daemon refused the window.
  Lost(String),
}

impl Told {
  /// One frame of the daemon, or the end of the connection. The first cause of a loss stays: the
  /// daemon refuses each later request of a peer whose hello it refused, and then closes it.
  pub(crate) fn hear(&mut self, heard: Heard) {
    if matches!(self, Self::Lost(_)) {
      return;
    }
    match heard {
      Ok(Frame::Reply { answer: Answer::Welcome { protocol, .. }, .. }) => {
        *self = Self::Welcomed { protocol, records: None };
      }
      Ok(Frame::Reply { answer: Answer::Lives(lives), .. }) => {
        if let Self::Welcomed { records, .. } = self {
          *records = Some(lives.len());
        }
      }
      Ok(Frame::Refused { why, .. }) => *self = Self::Lost(why),
      Err(fault) => *self = Self::Lost(fault.to_string()),
      // A frame that no part of the window shows yet.
      Ok(_) => {}
    }
  }
}

/// A part of a screen as the screen keeps it: it hears what the daemon told, and draws when the
/// value that it shows changed.
type Part = Box<dyn FnMut(&Told)>;

/// What the daemon told, and the parts that show it.
#[derive(Default)]
pub(crate) struct Screen {
  told: Told,
  parts: Vec<Part>,
}

impl Screen {
  /// A part of the screen: `shown` reads the value that the part shows from what the daemon told,
  /// and `draw` puts a value on the widgets of the part. `draw` runs now, for the first value,
  /// then only for a value that differs from the one before.
  pub(crate) fn part<S: PartialEq + 'static>(
    &mut self,
    shown: impl Fn(&Told) -> S + 'static,
    draw: impl Fn(&S) + 'static,
  ) {
    let mut last = None;
    let mut part = move |told: &Told| {
      let now = shown(told);
      if last.as_ref() != Some(&now) {
        draw(&now);
        last = Some(now);
      }
    };
    part(&self.told);
    self.parts.push(Box::new(part));
  }

  /// A change of what the daemon told, which each part hears.
  pub(crate) fn tell(&mut self, change: impl FnOnce(&mut Told)) {
    change(&mut self.told);
    for part in &mut self.parts {
      part(&self.told);
    }
  }
}

#[cfg(test)]
#[path = "screen.test.rs"]
mod test;
