//! The bar of the time of the open thread, under its head while the operator holds it: a play
//! button, the track of its moments with the one that the thread shows, and where it stands. A
//! click or a drag on the track takes the thread to the moment under the pointer.

use std::cmp::Ordering;

use furb_client::Time;
use opentui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::{
  keys::Does,
  line::{Line, run},
  model::Model,
  thread::{Click, Hot, Row},
  tokens::Role,
};

/// The columns before the track: the margin, the play button, and a blank.
const BEFORE: usize = 5;

impl Model {
  /// The row of the bar of the time of the open thread, and a blank under it, while it shows.
  pub(crate) fn time_rows(&self) -> Vec<Row> {
    let Some(time) = self.client.thread.as_ref().and_then(|one| one.thread.time.as_ref()) else {
      return Vec::new();
    };
    vec![self.time_row(time), Row::default()]
  }

  fn time_row(&self, time: &Time) -> Row {
    let t = &self.tokens;
    let (accent, muted) = (Style::new().fg(t.accent), self.tokens.style(Role::Muted));
    let count = time.moments.len();
    let at = time.at.unwrap_or(count.saturating_sub(1));
    let says = match (count, time.at) {
      (0, _) if time.stale => " reading".to_owned(),
      (0 | 1, _) => " nothing before now".to_owned(),
      (_, None) => " now".to_owned(),
      (_, Some(at)) => format!(" {} of {count}", at + 1),
    };
    let play = if time.played.is_some() { "❚❚" } else { "▶ " };
    let room = self.wide().saturating_sub(BEFORE + says.width() + 2);
    let mut line: Line = vec![run("  ", muted), run(play, accent), run(" ", muted)];
    let mut hot: Vec<Hot> = vec![(2..4, Some(Click::Play))];
    // Each column of the track stands for the moment nearest to its share of the track, so a
    // click takes the thread there; the column of the moment that shows holds the mark.
    let last = count.saturating_sub(1);
    let place = |column: usize| (column * last + (room.max(2) - 1) / 2) / (room.max(2) - 1);
    let mark = if count > 1 { (at * (room.max(2) - 1) + last / 2) / last } else { room };
    for column in 0..room {
      let (sign, style) = match column.cmp(&mark) {
        Ordering::Less => ("━", accent),
        Ordering::Equal => ("●", accent),
        Ordering::Greater => ("─", Style::new().fg(t.border)),
      };
      line.push(run(sign, style));
      if count > 1 {
        hot.push((BEFORE + column..BEFORE + column + 1, Some(Click::Time(place(column)))));
      }
    }
    line.push(run(says, muted));
    Row { line, hot, ..Row::default() }
  }

  /// The keys of the bar of the time while it shows, for the row of hints.
  pub(crate) fn time_hints(&self) -> Option<Line> {
    self.client.thread.as_ref().and_then(|one| one.thread.time.as_ref())?;
    let keys = format!("  ←→ a moment · Space play · {} now", self.key(Does::Close));
    Some(vec![run(keys, self.tokens.style(Role::Muted))])
  }

  /// A drag on the left side: on the track of the bar of the time, the thread goes to the moment
  /// under the pointer.
  pub(crate) fn dragged_left(&mut self, x: u16, y: usize) {
    let Some(x) = x.checked_sub(self.margin()).map(usize::from) else { return };
    if let Some(Click::Time(at)) = self.left_rows().into_iter().nth(y).and_then(|row| row.click(x))
    {
      self.client.reach(at);
    }
  }
}
