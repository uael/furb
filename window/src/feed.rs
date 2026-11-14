//! The rows of an open thread, from the exchanges that its daemon gave: the message of the operator
//! on its band, the block of code of the answer, which rolls in seven rows, the thought of its
//! model on one row, what the answer said as markdown, and the messages of the operator that the
//! thread does not show yet, then the dock of the agents of the thread.

use furb::view::{
  Block, Exchange, Line as Code, Maker, Mark, Said, Saying, Standing, Thought, Work,
};
use opentui::style::{Color, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
  keys::Does,
  line::{self, Line, run, spread, wrapped},
  list::project,
  model::Model,
  prose,
  thread::{Click, Fold, Item, Opened, Row, width},
  tokens::{Loader, Role},
};

/// How many rows a block shows while it is folded.
const ROLLS: usize = 7;

/// The columns of the left padding of a block, and of its ground before its text.
const PAD: usize = 3;

/// An open thread, and the columns of the left side.
pub(crate) struct Feed<'a> {
  pub(crate) model: &'a Model,
  pub(crate) opened: &'a Opened,
  wide: usize,
}

impl Model {
  /// The rows of the open thread.
  pub(crate) fn feed(&self) -> Option<Feed<'_>> {
    let opened = self.thread.as_ref()?;
    Some(Feed { model: self, opened, wide: self.wide() })
  }
}

impl Feed<'_> {
  /// The rows of one part of the feed. The last part holds each message of the operator that the
  /// thread does not show yet.
  pub(crate) fn rows(&self, item: Item) -> Vec<Row> {
    let mut rows = match item {
      Item::Head => self.before(),
      Item::At(at) => {
        self.opened.exchanges.get(&at).map(|one| self.exchange(one)).unwrap_or_default()
      }
    };
    if self.opened.items().last() == Some(item) {
      rows.extend(self.opened.pending.iter().flat_map(|one| self.message(&one.text)));
    }
    rows
  }

  fn style(&self, role: Role) -> Style {
    self.model.tokens.style(role)
  }

  /// What stands above the exchanges that the window holds: why the daemon gave no page, the wait
  /// for the first page, or how many messages come before.
  fn before(&self) -> Vec<Row> {
    let muted = self.style(Role::Muted);
    let mut rows: Vec<Row> = match (&self.opened.fault, self.opened.above) {
      (Some(why), _) => {
        let lines = wrapped(&self.model.homed(why), self.wide.saturating_sub(4));
        let danger = Style::new().fg(self.model.tokens.danger);
        let mut rows: Vec<Row> = lines.iter().map(|line| says(line, danger)).collect();
        let back =
          format!("The record stays as it was. {} goes back.", self.model.key(Does::Close));
        rows.extend([Row::default(), says(&back, muted)]);
        rows
      }
      // Until its page comes, the thread shows what its card holds: the first line of its message.
      (None, None) => {
        let card = self.model.told_card(&self.opened.key);
        return card.map(|card| self.message(&card.thread.title)).unwrap_or_default();
      }
      (None, Some(0)) => return Vec::new(),
      (None, Some(above)) => {
        let noun = if above == 1 { "message" } else { "messages" };
        vec![says(&format!("… {above} earlier {noun}"), muted)]
      }
    };
    rows.push(Row::default());
    rows
  }

  /// The head of the thread, which never scrolls: its project, its session or its title, and the
  /// button that resolves it, once the daemon made the thread and gave its page.
  pub(crate) fn head(&self) -> Row {
    let card = self.model.told_card(&self.opened.key);
    let project = card.map(project).unwrap_or_default();
    let named = card.and_then(|card| card.session.clone()).filter(|name| !name.is_empty());
    let title = named.or_else(|| self.model.title()).unwrap_or_default();
    let left = vec![
      run(format!(" {project} · "), self.style(Role::Muted)),
      run(title, self.style(Role::Title)),
    ];
    let made = !self.opened.key.2.is_empty() && !self.model.leaves();
    let button = if made { vec![run("[ Resolve ]", self.style(Role::Prose))] } else { Vec::new() };
    let line = spread(left, button.clone(), self.wide.saturating_sub(2), Style::new());
    let end = width(&line);
    Row { hot: vec![(end - width(&button)..end, None)], ..Row::of(line) }
  }

  /// The last row of the feed while the operator reads away from its end.
  pub(crate) fn jump(&self) -> Row {
    let jump = vec![run("  Jump to latest ↓", Style::new().fg(self.model.tokens.accent))];
    let end = vec![run("end", self.style(Role::Muted))];
    let line = spread(jump, end, self.wide.saturating_sub(2), Style::new());
    Row::clicked(line, Click::Key(Does::Jump))
  }

  /// The rows of one exchange: the message of the operator on its band, the block, the thought,
  /// and what the answer said.
  fn exchange(&self, exchange: &Exchange) -> Vec<Row> {
    let mut rows = self.message(&exchange.text);
    if let Some(block) = &exchange.block {
      rows.extend(self.block(&exchange.message, block));
    }
    for said in &exchange.said {
      rows.extend(self.said(said));
    }
    rows
  }

  /// A message of the operator on a raised band, with a bar in the color of the thread, and a
  /// blank row under it.
  fn message(&self, text: &str) -> Vec<Row> {
    let t = &self.model.tokens;
    let band = Style::new().bg(t.raised);
    let bar = run("┃", Style::new().fg(self.model.hue(&self.opened.key)).bg(t.raised));
    let banded = |text: Line| {
      let mut line = vec![bar.clone(), run(" ", band)];
      line.extend(spread(text, Vec::new(), self.wide.saturating_sub(4), band));
      Row::of(line)
    };
    let lines = self
      .model
      .homed(text)
      .lines()
      .flat_map(|line| wrapped(line, self.wide.saturating_sub(6)))
      .collect::<Vec<_>>();
    let text =
      lines.into_iter().map(|line| banded(vec![run(line, self.style(Role::Prose).bg(t.raised))]));
    [banded(Vec::new())]
      .into_iter()
      .chain(text)
      .chain([banded(Vec::new()), Row::default()])
      .collect()
  }

  /// A line inside the box of a block, on a ground, from the column of the text to its right
  /// padding.
  fn boxed(&self, line: Line, ground: Color) -> Line {
    let mut boxed = vec![run("  ", Style::new())];
    boxed.extend(line::boxed(line, self.wide.saturating_sub(4), ground));
    boxed
  }

  /// The columns inside the box of a block.
  fn inner(&self) -> usize {
    self.wide.saturating_sub(2 * PAD)
  }

  /// The block of an answer, as a block of code of the markdown: the model as its head, then the
  /// window of seven rows that rolls, or each line once the operator opened it, each with its
  /// number in the thread, close to the left of the box. The thought stands under it.
  fn block(&self, message: &str, block: &Block) -> Vec<Row> {
    let t = &self.model.tokens;
    let muted = self.style(Role::Muted);
    let numbers = block.lines.iter().map(|one| one.number).max().unwrap_or_default();
    // Two columns at least, so the code moves no cell as the block grows past its ninth line.
    let gutter = numbers.to_string().len().max(2);
    let head = vec![run(format!("{:gutter$}  {}", "", maker(&block.maker)), muted)];
    let mut rows = vec![Row::of(self.boxed(head, t.raised))];
    let code = self.style(Role::Prose).bg(t.raised);
    let text = self
      .model
      .homed(&block.lines.iter().map(|one| one.text.as_str()).collect::<Vec<_>>().join("\n"));
    let colored = self.model.colors.lines(&text, code);
    let all = block.lines.len();
    let fold = Fold::Block(message.to_owned());
    let open = self.opened.unfolded.contains(&fold);
    let rolls = all + usize::from(block.more) > ROLLS;
    let first = if open || !rolls { 0 } else { all + 1 + usize::from(block.more) - ROLLS };
    if rolls {
      let (mark, count) = if open { ("▴", all) } else { ("…", first) };
      let noun = if count == 1 { "line" } else { "lines" };
      let line = vec![run(format!("{mark:>gutter$}  {count} {noun}"), muted)];
      rows.push(Row::clicked(self.boxed(line, t.raised), Click::Fold(fold)));
    }
    for (line, colors) in block.lines.iter().zip(colored.iter()).skip(first) {
      rows.extend(self.code(line, colors.clone(), gutter));
    }
    if block.more {
      rows.push(Row::of(self.boxed(vec![run(format!("{:>gutter$}", "…"), muted)], t.raised)));
    }
    if let Some(thought) = &block.thought {
      rows.extend(self.thought(message, thought));
    }
    rows.push(Row::default());
    rows
  }

  /// One line of a block: its number in a gutter of a count of columns, or the mark of how it
  /// stands in its place, and its code, cut to the box. The line that runs is lit, with its loader
  /// and its time after one second. The error of a line that failed stands under it.
  fn code(&self, line: &Code, colors: Line, gutter: usize) -> Vec<Row> {
    let t = &self.model.tokens;
    let since = match line.mark {
      Mark::Runs { since: Some(since), .. } if self.model.now > since => {
        Some(self.model.now - since)
      }
      _ => None,
    };
    let (number, color, loader) = match &line.mark {
      Mark::Runs { work, .. } => (" ".to_owned(), t.accent, Some(loader(*work))),
      Mark::Asks => ("◆".to_owned(), t.attention, None),
      Mark::Failed { .. } => ("✗".to_owned(), t.danger, None),
      Mark::Ran => (line.number.to_string(), t.muted, None),
    };
    let ground = if loader.is_some() { t.lifted } else { t.raised };
    let timer =
      since.map(|since| vec![run(timer(since), self.style(Role::Muted))]).unwrap_or_default();
    let room =
      self.inner().saturating_sub(gutter + 2 + width(&timer) + 2 * usize::from(!timer.is_empty()));
    let mut text = vec![run(format!("{number:>gutter$}  "), Style::new().fg(color))];
    text.extend(clipped(colors, room, self.style(Role::Muted)));
    let shown = self.boxed(spread(text, timer, self.inner(), Style::new()), ground);
    let mut rows =
      vec![Row { loader: loader.map(|one| (PAD + gutter - 1, one)), ..Row::of(shown) }];
    if let Mark::Failed { why } = &line.mark {
      let error = Style::new().fg(t.danger);
      let lines = wrapped(why, self.inner().saturating_sub(gutter + 2));
      let errors = lines
        .iter()
        .map(|one| self.boxed(vec![run(format!("{:gutter$}  {one}", ""), error)], t.raised));
      rows.extend(errors.map(Row::of));
    }
    rows
  }

  /// The thought of a model as one muted row through its whole life: its last words while it
  /// streams, then its first sentence and its time. A click opens it whole, or folds it.
  fn thought(&self, message: &str, thought: &Thought) -> Vec<Row> {
    let style = self.style(Role::Muted).italic();
    let fold = Fold::Thought(message.to_owned());
    let room = self.wide.saturating_sub(4);
    if self.opened.unfolded.contains(&fold) {
      let lines = wrapped(&thought.text.split_whitespace().collect::<Vec<_>>().join(" "), room);
      let row = |line: &String| {
        Row::clicked(vec![run(format!("  {line}"), style)], Click::Fold(fold.clone()))
      };
      return lines.iter().map(row).collect();
    }
    let line = match thought.took {
      None => vec![run(format!("  {}", tail(&thought.text, room)), style)],
      Some(took) => {
        let first = vec![run("  ", style), run(thought.first(), style)];
        // A time shows from one second on, as the time of a line that runs.
        let took = (took > 0).then(|| run(timer(took), self.style(Role::Muted)));
        spread(first, took.into_iter().collect(), room + 2, Style::new())
      }
    };
    vec![Row::clicked(line, Click::Fold(fold))]
  }

  /// What an answer said, as markdown from the column of the text: a question opens with its mark.
  fn said(&self, said: &Said) -> Vec<Row> {
    let t = &self.model.tokens;
    let lines = prose::lines(
      &self.model.homed(&said.text),
      self.wide.saturating_sub(4),
      t,
      &self.model.colors,
    );
    let mut rows: Vec<Row> = lines
      .into_iter()
      .enumerate()
      .map(|(at, mut line)| {
        if matches!(said.kind, Saying::Question { .. }) && at == 0 {
          line.insert(0, run("◆ ", Style::new().fg(t.attention)));
        }
        line.insert(0, run("  ", Style::new()));
        Row::of(line)
      })
      .collect();
    rows.push(Row::default());
    rows
  }

  /// The dock of the agents of the thread: one row for each agent that works, asks or failed, and
  /// one row that counts the others.
  pub(crate) fn dock(&self) -> Vec<Row> {
    let t = &self.model.tokens;
    let rests = |standing: &Standing| {
      matches!(standing, Standing::Resolved | Standing::Rests | Standing::Stopped)
    };
    let mut rows = Vec::new();
    for agent in self.opened.agents.iter().filter(|agent| !rests(&agent.standing)) {
      let (mark, word, color) = match &agent.standing {
        Standing::Asks { .. } => (None, "◆ Input", t.attention),
        Standing::Failed => (None, "✗ Failed", t.danger),
        Standing::Paused { .. } => (None, "‖ Paused", t.attention),
        Standing::Thinks => (Some(Loader::Model), "  Working", t.accent),
        _ => (Some(Loader::Command), "  Working", t.accent),
      };
      let model = agent.model.as_deref().and_then(|one| one.rsplit(':').next()).unwrap_or_default();
      let left = vec![
        run("  ↳ ", self.style(Role::Muted)),
        run(agent.title.clone(), self.style(Role::Prose)),
      ];
      let right =
        vec![run(word, Style::new().fg(color)), run(format!("  {model}"), self.style(Role::Muted))];
      let line = spread(left, right.clone(), self.wide.saturating_sub(2), Style::new());
      let x = width(&line) - width(&right);
      rows.push(Row { loader: mark.map(|one| (x, one)), ..Row::of(line) });
    }
    let done = self.opened.agents.len() - rows.len();
    if done > 0 {
      let noun = if done == 1 { "agent" } else { "agents" };
      rows.push(says(&format!("↳ {done} {noun} done"), self.style(Role::Muted)));
    }
    if !rows.is_empty() {
      rows.insert(0, Row::default());
    }
    rows
  }
}

/// A row of prose from the column of the text.
fn says(text: &str, style: Style) -> Row {
  Row::of(vec![run(format!("  {text}"), style)])
}

/// The runs of a line of code cut to a count of columns: a cut line ends with `…+` and the count of
/// the characters that it hides, in a muted look.
fn clipped(line: Line, room: usize, muted: Style) -> Line {
  if width(&line) <= room {
    return line;
  }
  let chars: usize = line.iter().map(|one| one.content.chars().count()).sum();
  // The mark is as wide as the count of all the characters at most, so the cut leaves it room.
  let mut left = room.saturating_sub(format!("…+{chars}").width());
  let (mut out, mut kept) = (Vec::new(), 0);
  for one in line {
    let fits = |char: &char| {
      let wide = char.width().unwrap_or_default();
      let fits = wide <= left;
      left = if fits { left - wide } else { 0 };
      fits
    };
    let part: String = one.content.chars().take_while(fits).collect();
    let whole = part.len() == one.content.len();
    kept += part.chars().count();
    out.push(run(part, one.style));
    if !whole {
      break;
    }
  }
  out.push(run(format!("…+{}", chars - kept), muted));
  out
}

/// The last words of a text that fit a count of columns.
fn tail(text: &str, room: usize) -> String {
  let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
  let skip = flat.chars().count().saturating_sub(room);
  if skip == 0 {
    return flat;
  }
  format!("…{}", flat.chars().skip(skip + 1).collect::<String>())
}

/// A count of seconds as a person reads it.
pub(crate) fn timer(seconds: u64) -> String {
  match seconds {
    ..60 => format!("{seconds} s"),
    60..3600 => format!("{}m {:02}s", seconds / 60, seconds % 60),
    3600.. => format!("{}h {:02}m", seconds / 3600, seconds / 60 % 60),
  }
}

/// The model of a block as its head shows it: its name with no provider, and its effort.
fn maker(maker: &Maker) -> String {
  match maker {
    Maker::Model { model, effort } => {
      let name = model.rsplit(':').next().unwrap_or(model);
      effort.as_ref().map_or_else(|| name.to_owned(), |effort| format!("{name}/{effort}"))
    }
    Maker::Operator | Maker::Member { .. } => "operator".to_owned(),
    Maker::Skill => "skill".to_owned(),
    Maker::Engine => "furb".to_owned(),
  }
}

/// The loader of what a line that runs waits for: a model that writes, a command, or an agent.
fn loader(work: Work) -> Loader {
  match work {
    Work::Writes => Loader::Model,
    Work::Command => Loader::Command,
    Work::Agent => Loader::Agent,
  }
}
