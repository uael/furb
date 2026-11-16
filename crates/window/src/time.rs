//! The bar of the time of the open thread, under its head while the operator holds it: a play
//! button, the track of its moments with the one that the thread shows, and how many moments it
//! stands back from now. A click or a drag on the track takes the thread to the moment under the
//! pointer. Under it, while
//! the operator holds them open, the chains of the life stand on the same moments, one row each,
//! so a column is one moment in every row; a click on a chain opens its thread at that moment.
//! A prompt that a word of one chain asked of another flows down its column from the track of
//! the asker to the track of the chain that it asks, and its answer flows back, as the flow events
//! of a profiler join two tracks.

use std::{cmp::Ordering, ptr};

use crate::cells::Cells;
use furb_client::{Chains, Flow, Time, resting};
use opentui::style::{Color, Style};

use crate::{
  keys::Does,
  line::{Line, cut, run},
  model::Model,
  opens::QUIET,
  thread::{Click, Hot, Row, width},
  tokens::{Loader, Role},
};

/// The columns before the track: the margin, the play button, and a blank.
const BEFORE: usize = 5;

/// The columns of the toggle of the chains at the end of a bar of the time.
const TOGGLE: usize = 10;

/// The flows between the chains under the bar, and their cells, row after row of the columns of the
/// track: the sign, and the place of the flow among the flows.
#[derive(Default)]
struct Flows {
  all: Vec<Flow>,
  room: usize,
  cells: Vec<Option<(&'static str, usize)>>,
}

impl Flows {
  /// The sign and the flow at a cell of the rows of the chains.
  fn at(&self, row: usize, column: usize) -> Option<(&'static str, &Flow)> {
    let cell = (column < self.room).then(|| self.cells.get(row * self.room + column)).flatten();
    cell.copied().flatten().map(|(sign, at)| (sign, &self.all[at]))
  }
}

/// The columns of the track of a bar, each the moment nearest to its share of the track: the first
/// place among the moments that the track shows, how many places after it the track shows, the
/// columns from the first to the last, and the columns of the track.
struct Axis {
  first: usize,
  length: usize,
  span: usize,
  room: usize,
}

impl Axis {
  /// The place among the moments that a column of the track stands for.
  fn place(&self, column: usize) -> usize {
    self.first + (column * self.length + self.span / 2) / self.span
  }

  /// The column of the track that a place among the moments takes, past each column for a place
  /// that a zoom leaves out of the track.
  fn column(&self, at: usize) -> usize {
    match at.checked_sub(self.first).filter(|one| *one <= self.length) {
      Some(at) => (at * self.span + self.length / 2).checked_div(self.length).unwrap_or(self.room),
      None => usize::MAX,
    }
  }
}

/// The most acts of a chain in one column of its track that draw a thin line.
const BUSY: usize = 2;

/// The fewest columns between two marks of messages on a track, so a track that shows many
/// messages reads as a few marks and not as a wall of them.
const APART_MARKS: usize = 3;

impl Model {
  /// The rows of the bar of the time of the open thread, which always stands: the calm bar at now,
  /// or the bar that he holds, with its toggle of the chains at its end, and the chains while they
  /// show.
  pub(crate) fn time_rows(&self) -> Vec<Row> {
    let Some(time) = self.client.thread.as_ref().and_then(|one| one.thread.time.as_ref()) else {
      return self.calm_row().into_iter().collect();
    };
    // A bar that opens draws once its moments and its chains came, or from [`QUIET`] on, so the
    // input moves once, by the whole height of the bar; until then the calm bar stands.
    if time.reads() && self.client.moved.saturating_sub(time.since) < QUIET {
      return self.calm_row().into_iter().collect();
    }
    let t = &self.tokens;
    let key = &self.client.thread.as_ref().map(|one| one.thread.key.clone()).unwrap_or_default();
    // The track is the history of this thread, so what it passed takes the hue of the bar of its
    // messages, and the knob the bright text, as the hand on a scale.
    let (hue, muted) = (Style::new().fg(self.hue(key)), self.tokens.style(Role::Muted));
    let (says, said, axis) = self.axis(time);
    let at = time.place();
    let play = if time.played.is_some() { "❚❚" } else { "▶ " };
    let text = Style::new().fg(t.text);
    let mut line: Line = vec![run("  ", muted), run(play, text), run(" ", muted)];
    let mut hot: Vec<Hot> = vec![(2..4, Some(Click::Play))];
    // The column of the moment that shows holds the mark, and the column of each message a
    // diamond, so the track reads as the history of the thread.
    let mut told: Vec<usize> = Vec::new();
    for moment in time.messages.keys() {
      let column = axis.column(time.moments.partition_point(|one| one < moment));
      if column < axis.room && told.last().is_none_or(|last| column >= last + APART_MARKS) {
        told.push(column);
      }
    }
    let mark = axis.column(at);
    for one in 0..axis.room {
      let past = one < mark;
      let (sign, style) = match (one.cmp(&mark), told.contains(&one)) {
        (Ordering::Equal, _) => ("●", text),
        (_, true) => (if past { "◆" } else { "◇" }, if past { hue } else { muted }),
        (Ordering::Less, false) => ("━", hue),
        (Ordering::Greater, false) => ("─", Style::new().fg(t.border)),
      };
      line.push(run(sign, style));
      if time.moments.len() > 1 {
        hot.push((BEFORE + one..BEFORE + one + 1, Some(Click::Time(axis.place(one)))));
      }
    }
    // In the past the words say so in the attention color, and a click on them goes back to now.
    let past = time.at.is_some();
    let words = if past { Style::new().fg(t.attention) } else { muted };
    if past {
      let start = BEFORE + axis.room;
      hot.push((start..start + said, Some(Click::Key(Does::Rewind))));
    }
    line.push(run(format!("{says:>said$}"), words));
    let (toggle, start) = (self.chains_toggle(time.chains.is_some()), BEFORE + axis.room + said);
    hot.push((start..start + width(&toggle), Some(Click::Key(Does::Chains))));
    line.extend(toggle);
    let flows = time.chains.as_ref().map(|chains| flowed(time, chains, &axis)).unwrap_or_default();
    // The chains stand right under the bar, so a column reads as one moment down each row.
    let mut rows = vec![Row { line, hot, ..Row::default() }];
    rows.extend(self.chain_rows(time, &axis, &flows));
    rows
  }

  /// How many rows the bar of the time takes, as [`Model::time_rows`] lays them out, with no row
  /// of it made.
  pub(crate) fn time_count(&self) -> usize {
    let calm =
      || usize::from(self.client.thread.as_ref().is_some_and(|one| !one.thread.key.2.is_empty()));
    let Some(time) = self.client.thread.as_ref().and_then(|one| one.thread.time.as_ref()) else {
      return calm();
    };
    if time.reads() && self.client.moved.saturating_sub(time.since) < QUIET {
      return calm();
    }
    1 + time.chains.as_ref().map_or(0, |chains| chains.shown().len())
  }

  /// Whether a cell of the window stands on the bar of the time and its chains, the last rows of
  /// the left side.
  pub(crate) fn on_bar(&self, x: u16, y: u16) -> bool {
    let rows = self.time_count();
    rows > 0 && !self.on_list(x) && usize::from(y) + rows >= usize::from(self.size.1)
  }

  /// The words at the end of the bar of the time, the columns that they take, and the axis of its
  /// track. The words take the room of the farthest moment, so the track keeps its columns as the
  /// thread steps through its time.
  fn axis(&self, time: &Time) -> (String, usize, Axis) {
    let last = time.moments.len().saturating_sub(1);
    let (first, end) = time.shown();
    // The words stay short, so the track keeps its room: how far back it stands, and how close a
    // zoom shows it. The row of hints names the keys.
    let says = match time.moments.len() {
      0 if time.stale => " reading".to_owned(),
      0 | 1 => " nothing before now".to_owned(),
      _ if time.at.is_none() => " now".to_owned(),
      _ => format!(" {} back", last - time.place()),
    };
    let zoom = |length: usize| {
      let ratio = last.max(1) as f64 / length.max(1) as f64;
      if ratio < 10.0 { format!(" ×{ratio:.1}") } else { format!(" ×{ratio:.0}") }
    };
    let says = if time.zoom.is_some() { format!("{says}{}", zoom(end - first)) } else { says };
    let widest = format!(" {last} back{}", zoom(1)).width();
    let said = says.width().max(widest.min(" nothing before now".width()));
    // The toggle of the chains ends at the column of the text, as each label at the right of the
    // feed.
    let room = self.wide().saturating_sub(BEFORE + said + TOGGLE + 2);
    (says, said, Axis { first, length: end - first, span: room.max(2) - 1, room })
  }

  /// The toggle of the chains at the end of each bar of the time: it says that the tracks of the
  /// other chains are there, and whether they show.
  fn chains_toggle(&self, open: bool) -> Line {
    let sign = if open { "▴" } else { "▾" };
    vec![
      run("  ", Style::new()),
      run(format!("chains {sign}"), Style::new().fg(self.tokens.accent)),
    ]
  }

  /// What the status row says of the bar of the time, in the place of a notice: under the pointer
  /// on the track, the moment and the message then, and on a flow between two chains, what it asks
  /// or answers; else the text that finds a chain; else, while he walks the time, the moment that
  /// the thread shows.
  pub(crate) fn time_said(&self) -> Option<String> {
    let time = self.client.thread.as_ref()?.thread.time.as_ref()?;
    if time.reads() && self.client.moved.saturating_sub(time.since) < QUIET {
      return None;
    }
    let (_, _, axis) = self.axis(time);
    // The bar and its chains are the last rows of the left side, under the status row.
    let chained = time.chains.as_ref().map_or(0, |chains| chains.shown().len());
    let top = usize::from(self.size.1).saturating_sub(1 + chained);
    let walked = || {
      let at = time.place();
      time.at.and_then(|_| time.message(at)).map(|one| moment(time, at, one))
    };
    self.under(time, &axis, top).or_else(|| found(time)).or_else(walked)
  }

  /// The bar of the time while the operator does not hold it: one calm row under the input, from
  /// the margin of the boxes to their right margin, the track of the whole thread up to now, where
  /// a click or a drag goes back in its time as the rewind does. Its mark of now moves only while
  /// the thread works.
  fn calm_row(&self) -> Option<Row> {
    let key = &self.client.thread.as_ref()?.thread.key;
    if key.2.is_empty() {
      return None;
    }
    let t = &self.tokens;
    let span = self.wide().saturating_sub(5 + TOGGLE);
    let mut line: Line = vec![
      run("  ", Style::new()),
      run("─".repeat(span), Style::new().fg(t.border)),
      run("●", Style::new().fg(t.muted)),
    ];
    let mut hot: Vec<Hot> =
      (0..=span).map(|one| (2 + one..3 + one, Some(Click::Aim(one, span)))).collect();
    hot.push((3 + span..3 + span + TOGGLE, Some(Click::Key(Does::Chains))));
    line.extend(self.chains_toggle(false));
    let works = self.client.card(key).is_some_and(|card| card.thread.works());
    let loaders = if works { vec![(2 + span, Loader::Model)] } else { Vec::new() };
    Some(Row { line, hot, loaders, ..Row::default() })
  }

  /// What the pointer is on under the bar: on the track, the message in force at its moment, and on
  /// a flow between the chains, what it asks or answers.
  fn under(&self, time: &Time, axis: &Axis, row: usize) -> Option<String> {
    let (x, y) = self.pointed()?;
    let column = usize::from(x).checked_sub(BEFORE).filter(|one| *one < axis.room)?;
    match usize::from(y).checked_sub(row)? {
      0 => {
        let at = axis.place(column);
        time.message(at).map(|one| moment(time, at, one))
      }
      // The flows are made only while the pointer stands on the rows of the chains.
      below => {
        let flows = flowed(time, time.chains.as_ref()?, axis);
        flows.at(below - 1, column).map(|(_, flow)| flow_says(time, flow))
      }
    }
  }

  /// The place among the chains of the one that holds the open thread.
  fn holder(&self, chains: &Chains) -> Option<usize> {
    let key = &self.client.thread.as_ref()?.thread.key;
    chains.tracks.iter().position(|one| one.threads.iter().any(|began| began.prompt == key.2))
  }

  /// The ink of a chain under the bar, grey while it rests, with the place of the chain that holds
  /// the thread. While it works, that chain takes the hue of the thread, as its bar, and each other
  /// chain takes in turn by its place among them the hues of the work less that one, so no other
  /// chain reads as the thread.
  fn ink(&self, chains: &Chains, at: usize, holder: Option<usize>) -> Style {
    let t = &self.tokens;
    let key = self.client.thread.as_ref().map(|one| &one.thread.key);
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
    let holder = self.holder(chains);
    let column = |moment: &u64| axis.column(time.moments.partition_point(|one| one < moment));
    let first = time.moments.first().copied().unwrap_or_default();
    shown
      .iter()
      .enumerate()
      .map(|(row, (track, depth))| {
        let at = chains.tracks.iter().position(|one| ptr::eq(one, *track)).unwrap_or(row);
        let ink = self.ink(chains, at, holder);
        // Each column of the track once, so a row costs its moments and its columns, not their
        // product.
        let marked = |moments: &mut dyn Iterator<Item = &u64>| {
          let mut cells = vec![0usize; axis.room];
          for one in moments {
            if let Some(cell) = cells.get_mut(column(one)) {
              *cell += 1;
            }
          }
          cells
        };
        let acted = marked(&mut track.moments.iter());
        let began = marked(&mut track.threads.iter().map(|one| &one.at));
        let before = track.moments.iter().any(|moment| *moment < first);
        let mut line: Line = vec![run("  ", muted), run("▪", ink), run("  ", muted)];
        let mut hot: Vec<Hot> = Vec::new();
        for one in 0..axis.room {
          let flow = flows.at(row, one);
          // A column reads by how much the chain did there: a thin line for a few acts, a heavy
          // one where it did much, so a track zoomed out shows where its work gathers.
          let (sign, style) = match (began[one] > 0, acted[one], one == mark) {
            _ if one == 0 && before => ("‹", ink),
            _ if let Some((sign, flow)) = flow => (sign, self.ink(chains, flow.sender, holder)),
            (true, ..) => ("◆", ink),
            (false, 1..=BUSY, _) => ("─", ink),
            (false, 1.., _) => ("━", ink),
            (false, 0, true) => ("│", Style::new().fg(t.text)),
            (false, 0, false) => ("─", Style::new().fg(t.border)),
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
    let time = thread.time.as_ref().filter(|one| one.engaged())?;
    let keys = if time.chains.is_some() {
      "  type to find a chain · ⏎ open it at this moment · ⎋ close".to_owned()
    } else {
      let send = if thread.past() {
        format!(" · {} send from here", self.key(Does::Send))
      } else {
        String::new()
      };
      let chains = self.key(Does::Chains);
      let close = self.key(Does::Close);
      format!("  ←→ a moment · + − zoom · Space play{send} · {chains} chains · {close} now")
    };
    Some(vec![run(keys, self.tokens.style(Role::Muted))])
  }

  /// A drag on the left side: on the track of the bar of the time, the thread goes to the moment
  /// under the pointer.
  pub(crate) fn dragged_left(&mut self, x: u16, y: usize) {
    let x = usize::from(x);
    match self.left_rows().into_iter().nth(y).and_then(|row| row.click(x)) {
      Some(Click::Time(at)) => self.client.reach(at),
      Some(Click::Aim(column, span)) => self.client.aim(column, span),
      _ => {}
    }
  }

  /// The track zoomed in or out around the moment under the pointer, as the wheel with a modifier
  /// turns over the bar. Whether the pointer stood on the track.
  pub(crate) fn zoomed_left(&mut self, x: u16, y: usize, closer: bool) -> bool {
    match self.left_rows().into_iter().nth(y).and_then(|row| row.click(usize::from(x))) {
      Some(Click::Time(at)) => {
        self.client.zoom(closer, Some(at));
        true
      }
      _ => false,
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
      let since = self.client.moved;
      opened.thread.time = Some(Time { at: moment, stale: true, since, ..Time::default() });
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
  let rows = chains.shown().len();
  let (last, room) = (rows.saturating_sub(1), axis.room);
  let mut cells = vec![None; rows * room];
  let mut put = |row: usize, column: usize, cell| {
    if let Some(one) = (column < room).then(|| cells.get_mut(row * room + column)).flatten() {
      *one = Some(cell);
    }
  };
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
    put(top, column, (signs.0, at));
    for row in top + 1..bottom {
      put(row, column, (line, at));
    }
    put(bottom, column, (signs.1, at));
  }
  Flows { all, room, cells }
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

/// A moment of the bar as the status row says it: how far back it stands, and the message then.
fn moment(time: &Time, at: usize, message: &str) -> String {
  let last = time.moments.len().saturating_sub(1);
  format!("{} · {message}", back(last.saturating_sub(at)).trim())
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
