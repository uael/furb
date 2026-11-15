//! The bar of the time of the open thread, under its head while the operator holds it: a play
//! button, the track of its moments with the one that the thread shows, and how many moments it
//! stands back from now. A click or a drag on the track takes the thread to the moment under the
//! pointer. Under it, while
//! the operator holds them open, the chains of the life stand on the same moments, one row each,
//! so a column is one moment in every row; a click on a chain opens its thread at that moment.
//! A prompt that a word of one chain asked of another flows down its column from the track of
//! the asker to the track of the chain that it asks, and its answer flows back, as the flow events
//! of a profiler join two tracks.

use std::{cmp::Ordering, collections::HashMap, ptr};

use furb_client::{Chains, Flow, Time, resting};
use opentui::style::{Color, Style};
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

/// The flows between the chains under the bar, and their cells by row and column: the sign, and the
/// place of the flow among the flows.
#[derive(Default)]
struct Flows {
  all: Vec<Flow>,
  cells: HashMap<(usize, usize), (&'static str, usize)>,
}

impl Flows {
  /// The sign and the flow at a cell of the rows of the chains.
  fn at(&self, row: usize, column: usize) -> Option<(&'static str, &Flow)> {
    self.cells.get(&(row, column)).map(|(sign, at)| (*sign, &self.all[*at]))
  }
}

/// The columns of the track of a bar, each the moment nearest to its share of the track.
struct Axis {
  last: usize,
  span: usize,
  room: usize,
}

impl Axis {
  /// The place among the moments that a column of the track stands for.
  fn place(&self, column: usize) -> usize {
    (column * self.last + self.span / 2) / self.span
  }

  /// The column of the track that a place among the moments takes.
  fn column(&self, at: usize) -> usize {
    (at * self.span + self.last / 2).checked_div(self.last).unwrap_or(self.room)
  }
}

impl Model {
  /// The row of the bar of the time of the open thread while it shows, under it the first line of
  /// the message in force at the moment under the pointer while the pointer is on its track, and
  /// the chains while they show.
  pub(crate) fn time_rows(&self) -> Vec<Row> {
    let Some(time) = self.client.thread.as_ref().and_then(|one| one.thread.time.as_ref()) else {
      return Vec::new();
    };
    let t = &self.tokens;
    let key = &self.client.thread.as_ref().map(|one| one.thread.key.clone()).unwrap_or_default();
    // The track is the history of this thread, so what it passed takes the hue of the bar of its
    // messages, and the knob the bright text, as the hand on a scale.
    let (hue, muted) = (Style::new().fg(self.hue(key)), self.tokens.style(Role::Muted));
    let count = time.moments.len();
    let last = count.saturating_sub(1);
    let at = time.place();
    let says = match count {
      0 if time.stale => " reading".to_owned(),
      0 | 1 => " nothing before now".to_owned(),
      _ => back(last - at),
    };
    // The words take the room of the farthest moment, so the track keeps its columns as the
    // thread steps through its time.
    let said = says.width().max(back(last).width());
    let play = if time.played.is_some() { "❚❚" } else { "▶ " };
    // The words end at the column of the text, as each label at the right of the feed.
    let room = self.wide().saturating_sub(BEFORE + said + 4);
    let axis = Axis { last, span: room.max(2) - 1, room };
    let text = Style::new().fg(t.text);
    let mut line: Line = vec![run("  ", muted), run(play, text), run(" ", muted)];
    let mut hot: Vec<Hot> = vec![(2..4, Some(Click::Play))];
    // The column of the moment that shows holds the mark, and the column of each message a
    // diamond, so the track reads as the history of the thread.
    let told: Vec<usize> = time
      .messages
      .keys()
      .map(|moment| axis.column(time.moments.partition_point(|one| one < moment)))
      .collect();
    let mark = axis.column(at);
    for one in 0..room {
      let past = one < mark;
      let (sign, style) = match (one.cmp(&mark), told.contains(&one)) {
        (Ordering::Equal, _) => ("●", text),
        (_, true) => (if past { "◆" } else { "◇" }, if past { hue } else { muted }),
        (Ordering::Less, false) => ("━", hue),
        (Ordering::Greater, false) => ("─", Style::new().fg(t.border)),
      };
      line.push(run(sign, style));
      if count > 1 {
        hot.push((BEFORE + one..BEFORE + one + 1, Some(Click::Time(axis.place(one)))));
      }
    }
    let label = BEFORE + room;
    hot.push((label..label + said, Some(Click::Key(Does::Chains))));
    line.push(run(format!("{says:>said$}"), muted));
    let flows = time.chains.as_ref().map(|chains| flowed(time, chains, &axis)).unwrap_or_default();
    let under = self.under(time, &axis, &flows);
    // The chains stand right under the bar, so a column reads as one moment down each row, and one
    // row under them says what is under the pointer, else what finds a chain.
    let mut rows = vec![Row { line, hot, ..Row::default() }];
    rows.extend(self.chain_rows(time, &axis, &flows));
    let wide = self.wide().saturating_sub(BEFORE);
    let said = under.map(|text| cut(&text, wide)).or_else(|| found(time));
    rows.push(Row::of(vec![run(
      format!("{}{}", " ".repeat(BEFORE), said.unwrap_or_default()),
      muted,
    )]));
    rows
  }

  /// What the pointer is on under the bar: on the track, the message in force at its moment, and on
  /// a flow between the chains, what it asks or answers.
  fn under(&self, time: &Time, axis: &Axis, flows: &Flows) -> Option<String> {
    let row = 1 + self.open_rows().len();
    let (x, y) = self.pointed()?;
    let column = usize::from(x).checked_sub(BEFORE).filter(|one| *one < axis.room)?;
    match usize::from(y).checked_sub(row)? {
      0 => time.message(axis.place(column)).map(str::to_owned),
      below => flows.at(below - 1, column).map(|(_, flow)| flow_says(time, flow)),
    }
  }

  /// The ink of a chain under the bar, grey while it rests. While it works, the chain that holds
  /// the thread takes the hue of the thread, as its bar, and each other chain takes in turn by its
  /// place among them the hues of the work less that one, so no other chain reads as the thread.
  fn ink(&self, chains: &Chains, at: usize) -> Style {
    let t = &self.tokens;
    let key = self.client.thread.as_ref().map(|one| &one.thread.key);
    let holder = key.and_then(|key| {
      chains.tracks.iter().position(|one| one.threads.iter().any(|began| began.prompt == key.2))
    });
    let hue = key.map(|key| self.hue(key));
    let ink = match hue {
      _ if resting(chains.tracks[at].state) => t.muted,
      Some(hue) if holder == Some(at) => hue,
      _ => {
        let others: Vec<Color> = t.hues.into_iter().filter(|one| Some(*one) != hue).collect();
        let place = at - usize::from(holder.is_some_and(|one| one < at));
        others[place % others.len()]
      }
    };
    Style::new().fg(ink)
  }

  /// A row for each chain that shows under the bar, on the columns of its track: a heavy cell where
  /// an act of the chain began or ended, a diamond where a thread of it began, the cells of the
  /// flows between the chains in the ink of the chain that sends each, the column of the moment
  /// that the thread shows across each row, and the label of the chain at its end. A chain that
  /// works takes its hue, and one that rests is grey. A chain that acted before the first moment
  /// of the thread says so at the start of its row. A click on a flow opens its prompt.
  fn chain_rows(&self, time: &Time, axis: &Axis, flows: &Flows) -> Vec<Row> {
    let mark = axis.column(time.place());
    let Some(chains) = time.chains.as_ref() else { return Vec::new() };
    let t = &self.tokens;
    let muted = t.style(Role::Muted);
    let shown = chains.shown();
    let column = |moment: &u64| axis.column(time.moments.partition_point(|one| one < moment));
    let first = time.moments.first().copied().unwrap_or_default();
    shown
      .iter()
      .enumerate()
      .map(|(row, (track, depth))| {
        let at = chains.tracks.iter().position(|one| ptr::eq(one, *track)).unwrap_or(row);
        let ink = self.ink(chains, at);
        let acted: Vec<usize> = track.moments.iter().map(column).collect();
        let began: Vec<usize> = track.threads.iter().map(|one| column(&one.at)).collect();
        let before = track.moments.iter().any(|moment| *moment < first);
        let mut line: Line = vec![run("  ", muted), run("▪", ink), run("  ", muted)];
        let mut hot: Vec<Hot> = Vec::new();
        for one in 0..axis.room {
          let flow = flows.at(row, one);
          let (sign, style) = match (began.contains(&one), acted.contains(&one), one == mark) {
            _ if one == 0 && before => ("‹", ink),
            _ if let Some((sign, flow)) = flow => (sign, self.ink(chains, flow.sender)),
            (true, ..) => ("◆", ink),
            (false, true, _) => ("━", ink),
            (false, false, true) => ("│", Style::new().fg(t.text)),
            (false, false, false) => ("─", Style::new().fg(t.border)),
          };
          line.push(run(sign, style));
          let click = match flow {
            Some((_, flow)) => Click::Agent(flow.prompt.clone()),
            None => Click::Chain { row, at: axis.place(one) },
          };
          hot.push((BEFORE + one..BEFORE + one + 1, Some(click)));
        }
        let label = if track.label.is_empty() { &track.chain } else { &track.label };
        let room = self.wide().saturating_sub(BEFORE + axis.room + 1);
        // A chain stands under the chain whose word made it.
        let label =
          format!(" {}{}", "  ".repeat(*depth), cut(label, room.saturating_sub(2 * depth)));
        let end = BEFORE + axis.room;
        hot.push((end..end + label.width(), Some(Click::Chain { row, at: time.place() })));
        line.push(run(label, ink));
        Row { line, hot, ..Row::default() }
      })
      .collect()
  }

  /// The keys of the bar of the time while it shows, for the row of hints.
  pub(crate) fn time_hints(&self) -> Option<Line> {
    let thread = &self.client.thread.as_ref()?.thread;
    let time = thread.time.as_ref()?;
    let keys = if time.chains.is_some() {
      "  type to find a chain · ⏎ open it at this moment · ⎋ close".to_owned()
    } else {
      let send = if thread.past() {
        format!(" · {} send from here", self.key(Does::Send))
      } else {
        String::new()
      };
      let chains = self.key(Does::Chains);
      format!("  ←→ a moment · Space play{send} · {chains} chains · {} now", self.key(Does::Close))
    };
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

  /// The thread of a chain under the bar opened at a place among the moments of the bar, as an
  /// agent opens, so an escape goes back: the last thread of that chain that began by that moment,
  /// at that moment. The thread that shows only moves there.
  pub(crate) fn open_chain(&mut self, row: usize, at: usize) {
    let Some((prompt, moment)) = self.client.chain_thread(row, at) else {
      self.client.notice = Some("This chain holds no thread.".to_owned());
      return;
    };
    let shows = self.client.thread.as_ref().is_some_and(|one| one.thread.key.2 == prompt);
    if shows {
      self.client.reach(at);
      self.client.chains();
      return;
    }
    self.open_agent(prompt);
    if let (Some(opened), Some(_)) = (self.client.thread.as_mut(), moment) {
      opened.thread.time = Some(Time { at: moment, stale: true, ..Time::default() });
    }
  }
}

/// The cells of the flows between the chains that show, on the column of the moment of each: a
/// tee on the row of the chain that sends it, an arrow on the row of the chain that takes it,
/// and a line on each row between them, solid for a prompt and dotted for its answer. A flow to
/// or from a chain that does not show runs to the edge under the last row, where an arrow says
/// that it leaves or comes in. A later flow on a cell draws over an earlier one.
fn flowed(time: &Time, chains: &Chains, axis: &Axis) -> Flows {
  let all = chains.flows();
  let last = chains.shown().len().saturating_sub(1);
  let mut cells = HashMap::new();
  for (at, flow) in all.iter().enumerate() {
    let column = axis.column(time.moments.partition_point(|one| *one < flow.at));
    let line = if flow.answer { "┆" } else { "│" };
    let (top, bottom, signs) = match (flow.from, flow.to) {
      (Some(from), Some(to)) if from < to => (from, to, ("┬", "▼")),
      (Some(from), Some(to)) => (to, from, ("▲", "┴")),
      (Some(from), None) if from == last => (from, from, ("↓", "↓")),
      (Some(from), None) => (from, last, ("┬", "↓")),
      (None, Some(to)) if to == last => (to, to, ("↑", "↑")),
      (None, Some(to)) => (to, last, ("▲", "↑")),
      (None, None) => continue,
    };
    cells.insert((top, column), (signs.0, at));
    for row in top + 1..bottom {
      cells.insert((row, column), (line, at));
    }
    cells.insert((bottom, column), (signs.1, at));
  }
  Flows { all, cells }
}

/// What a flow under the pointer says: the chain that sends it, whether it asks or answers, the
/// chain that takes it, and the first line of its message.
fn flow_says(time: &Time, flow: &Flow) -> String {
  let Some(chains) = time.chains.as_ref() else { return String::new() };
  let label = |at: usize| {
    let track = &chains.tracks[at];
    if track.label.is_empty() { track.chain.clone() } else { track.label.clone() }
  };
  let (sender, receiver, says) = (label(flow.sender), label(flow.receiver), &flow.says);
  if flow.answer {
    format!("{sender} answers {receiver} on “{says}” · click to open it")
  } else {
    format!("{sender} asks {receiver}: {says} · click to open it")
  }
}

/// Where the thread stands in its time: now, or a count of moments back from now.
fn back(moments: usize) -> String {
  match moments {
    0 => " now".to_owned(),
    1 => " 1 moment back".to_owned(),
    count => format!(" {count} moments back"),
  }
}

/// What finds a chain under the bar while the chains show: the text typed, or why no row shows.
fn found(time: &Time) -> Option<String> {
  let chains = time.chains.as_ref()?;
  match (chains.find.is_empty(), chains.shown().is_empty()) {
    (false, true) => Some(format!("no chain matches {}", chains.find)),
    (false, false) => Some(format!("find {}", chains.find)),
    (true, true) if chains.stale => Some("reading the chains".to_owned()),
    (true, true) => Some("no chain works now · type to find one".to_owned()),
    (true, false) => None,
  }
}
