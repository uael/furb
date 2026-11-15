//! The bar of the time of the open thread, under its head while the operator holds it: a play
//! button, the track of its moments with the one that the thread shows, and where it stands. A
//! click or a drag on the track takes the thread to the moment under the pointer.

use std::cmp::Ordering;

use opentui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::{
  keys::Does,
  line::{Line, cut, run},
  model::Model,
  thread::{Click, Hot, Row},
  tokens::Role,
};

/// The columns before the track: the margin, the play button, and a blank.
const BEFORE: usize = 5;

impl Model {
  /// The row of the bar of the time of the open thread while it shows, and under it the first line
  /// of the message in force at the moment under the pointer, while the pointer is on its track.
  pub(crate) fn time_rows(&self) -> Vec<Row> {
    let Some(time) = self.client.thread.as_ref().and_then(|one| one.thread.time.as_ref()) else {
      return Vec::new();
    };
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
    // click takes the thread there. The column of the moment that shows holds the mark, and the
    // column of each message a diamond, so the track reads as the history of the thread.
    let (last, span) = (count.saturating_sub(1), room.max(2) - 1);
    let place = |column: usize| (column * last + span / 2) / span;
    let column = |at: usize| (at * span + last / 2).checked_div(last).unwrap_or(room);
    let said: Vec<usize> = time
      .messages
      .keys()
      .map(|moment| column(time.moments.partition_point(|one| one < moment)))
      .collect();
    let mark = column(at);
    for one in 0..room {
      let past = one < mark;
      let (sign, style) = match (one.cmp(&mark), said.contains(&one)) {
        (Ordering::Equal, _) => ("●", accent),
        (_, true) => {
          (if past { "◆" } else { "◇" }, Style::new().fg(if past { t.accent } else { t.muted }))
        }
        (Ordering::Less, false) => ("━", accent),
        (Ordering::Greater, false) => ("─", Style::new().fg(t.border)),
      };
      line.push(run(sign, style));
      if count > 1 {
        hot.push((BEFORE + one..BEFORE + one + 1, Some(Click::Time(place(one)))));
      }
    }
    line.push(run(says, muted));
    let row = 1 + self.open_rows().len();
    let pointed = self.pointed().filter(|(_, y)| usize::from(*y) == row);
    let under = pointed
      .and_then(|(x, _)| usize::from(x).checked_sub(BEFORE).filter(|one| *one < room))
      .and_then(|one| time.message(place(one)))
      .map(|text| Row::of(vec![run(format!("{}{}", " ".repeat(BEFORE), cut(text, room)), muted)]));
    vec![Row { line, hot, ..Row::default() }, under.unwrap_or_default()]
  }

  /// The keys of the bar of the time while it shows, for the row of hints.
  pub(crate) fn time_hints(&self) -> Option<Line> {
    let thread = &self.client.thread.as_ref()?.thread;
    thread.time.as_ref()?;
    let send = if thread.past() {
      format!(" · {} send from here", self.key(Does::Send))
    } else {
      String::new()
    };
    let keys = format!("  ←→ a moment · Space play{send} · {} now", self.key(Does::Close));
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
