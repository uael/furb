//! The history of the input as the window shows it: the history itself and its walk stand in the
//! client. The up arrow on the first row of the input walks it from the newest, and the down arrow
//! on its last row walks back. While no walk runs, the input shows in grey the rest of the newest
//! text that starts with what he wrote, which the right arrow takes.

use unicode_width::UnicodeWidthStr;

use crate::{line::cut, model::Model, pick::Pick};

impl Model {
  /// The text of the editor, which the draft of the client takes. A slash that starts an empty
  /// input of a message opens the commands, as the key of the commands does; a close of that list
  /// gives the slash back, so a message can start with one.
  pub(crate) fn edited(&mut self, text: String) {
    let slash = self.client.draft.text.is_empty()
      && text == "/"
      && !self.client.draft.python
      && self.form().is_none()
      && self.picker.is_none();
    self.client.edited(text);
    if slash {
      self.client.draft.text.clear();
      self.client.restore = Some(String::new());
      self.pick(Pick::Commands);
      if let Some(picker) = self.picker.as_mut() {
        picker.slashed = true;
      }
    }
  }

  /// The list part closed: a list that a slash opened gives the slash back to the input.
  pub(crate) fn unpicked(&mut self) {
    if self.picker.take().is_some_and(|one| one.slashed) {
      self.client.draft.text.clear();
      self.client.draft.text.push('/');
      self.client.restore = Some("/".to_owned());
    }
  }

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
