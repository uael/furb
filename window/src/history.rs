//! The history of the input as the window shows it: the history itself and its walk stand in the
//! client. The up arrow on the first row of the input walks it from the newest, and the down arrow
//! on its last row walks back. While no walk runs, the input shows in grey the rest of the newest
//! text that starts with what he wrote, which the right arrow takes.

use unicode_width::UnicodeWidthStr;

use crate::{line::cut, model::Model};

impl Model {
  /// The rest that the input shows in grey, at the end of what he wrote: its column, its row, and
  /// its text on one row, cut at the end of the box of the input.
  pub(crate) fn ghost(&self) -> Option<(u16, u16, String)> {
    let rest = self.client.suggested()?;
    let area = self.area();
    let width = usize::from(area.width);
    if width == 0 {
      return None;
    }
    // The editor breaks a line that passes its columns at any character.
    let mut lines = self.client.draft.text.split('\n').map(UnicodeWidthStr::width);
    let last = lines.next_back()?;
    let above: usize = lines.map(|one| one.div_ceil(width).max(1)).sum();
    let (row, column) = (above + last / width, last % width);
    let rest = cut(&rest.replace('\n', "↵"), width - column);
    let number = |one: usize| u16::try_from(one).ok();
    let (x, y) = (area.x + number(column)?, area.y + number(row)?);
    (row < usize::from(area.height)).then_some((x, y, rest))
  }
}
