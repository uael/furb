//! The rows of an open thread, from the exchanges that its daemon gave: the message of the operator
//! on its band, the block of code of the answer, which rolls in seven rows, the thought of its
//! model on one row, what the answer said as markdown, and the messages of the operator that the
//! thread does not show yet, then the dock of the agents of the thread.

use furb::view::{
  Agent, Block, Exchange, Line as Code, Maker, Mark, Retrying, Said, Saying, Standing, Thought,
  Work,
};
use opentui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

use crate::{
  code::Gutter,
  input::Armed,
  keys::Does,
  line::{self, Line, clipped, run, spread, wrapped},
  list::project,
  model::Model,
  prose,
  question::closed,
  thread::{Click, Fold, Item, Opened, Row, rests, width},
  tokens::{APART, Loader, Role},
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
      rows.extend(self.opened.pending.iter().flat_map(|one| self.message(&one.text, true)));
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
        return card.map(|card| self.message(&card.thread.title, true)).unwrap_or_default();
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
  /// button that resolves it, once the daemon made the thread and gave its page. A thread that
  /// ended shows how, and its button turns the input to it, since a reply opens it again. A resolve
  /// of a thread that works waits for a confirm on the button, which says that it stops the work.
  /// The thread of an agent says that it is one, and its button goes back to the thread that it
  /// came from.
  pub(crate) fn head(&self) -> Row {
    let t = &self.model.tokens;
    let root = self.opened.root();
    let card = self.model.told_card(&root.key);
    let project = card.map(project).unwrap_or_default();
    let named = card.and_then(|card| card.session.clone()).filter(|name| !name.is_empty());
    let (prose, muted) = (self.style(Role::Prose), self.style(Role::Muted));
    let title = self.model.title().unwrap_or_default();
    // The selected card names the thread that an agent came from, so the head names the agent.
    let (mark, title) = match &self.opened.from {
      Some(_) => ("agent · ", title),
      None => ("", named.unwrap_or(title)),
    };
    let left =
      vec![run(format!(" {project} · {mark}"), muted), run(title, self.style(Role::Title))];
    let made = !self.opened.key.2.is_empty() && !self.model.leaves();
    let (mark, button) = match card.map(|card| &card.thread) {
      _ if self.opened.from.is_some() => (None, Some((run("[ Back ]", prose), Does::Close))),
      _ if !made => (None, None),
      Some(thread) if thread.ended() => {
        let mark = match thread.standing {
          Standing::Resolved => run("✓ Resolved", muted),
          Standing::Failed => run("✗ Failed", Style::new().fg(t.danger)),
          _ => run("Stopped", muted),
        };
        (Some(mark), Some((run("[ Reopen ]", prose), Does::Resolve)))
      }
      Some(thread)
        if thread.works() && self.model.armed == Some(Armed::Resolve(self.opened.key.clone())) =>
      {
        (None, Some((run("[ Stop and resolve ]", Style::new().fg(t.attention)), Does::Resolve)))
      }
      _ => (None, Some((run("[ Resolve ]", prose), Does::Resolve))),
    };
    let apart = |mark| [mark, run(" ".repeat(APART), Style::new())];
    let right: Line =
      mark.into_iter().flat_map(apart).chain(button.clone().map(|one| one.0)).collect();
    let line = spread(left, right, self.wide.saturating_sub(2), Style::new());
    let end = width(&line);
    let hot = button.map(|(one, does)| (end - one.content.width()..end, Some(Click::Key(does))));
    Row { hot: hot.into_iter().collect(), ..Row::of(line) }
  }

  /// The last row of the feed while the operator reads away from its end.
  pub(crate) fn jump(&self) -> Row {
    let jump = vec![run("  Jump to latest ↓", Style::new().fg(self.model.tokens.accent))];
    let end = vec![run(self.model.key(Does::Jump), self.style(Role::Muted))];
    let line = spread(jump, end, self.wide.saturating_sub(2), Style::new());
    Row::clicked(line, Click::Key(Does::Jump))
  }

  /// The rows of one exchange: the message of the operator on its band, the block, the thought,
  /// and what the answer said.
  fn exchange(&self, exchange: &Exchange) -> Vec<Row> {
    let his = matches!(exchange.by, Maker::Operator | Maker::Member { .. });
    let mut rows = self.message(&exchange.text, his);
    if let Some(block) = &exchange.block {
      rows.extend(self.block(&exchange.message, block));
    }
    for said in &exchange.said {
      rows.extend(self.said(said));
    }
    rows
  }

  /// A message on a raised band, with a bar in the color of the thread when an operator said it,
  /// and muted when a model gave it to its agent, and a blank row under it.
  fn message(&self, text: &str, his: bool) -> Vec<Row> {
    let t = &self.model.tokens;
    let band = Style::new().bg(t.raised);
    let hue = if his { self.model.hue(&self.opened.key) } else { t.muted };
    let bar = run("┃", Style::new().fg(hue).bg(t.raised));
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
    let numbers = Gutter::of(block.lines.iter().map(|one| one.number).max().unwrap_or_default());
    let gutter = numbers.width;
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
      rows.extend(self.code(line, colors.clone(), numbers));
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
  /// and its time after one second, and the line whose model is asked again says the attempt that
  /// comes and its wait. Why a line failed, or why its model is asked again, stands under it, or
  /// in its place when its model wrote none of it.
  fn code(&self, line: &Code, colors: Line, numbers: Gutter) -> Vec<Row> {
    let t = &self.model.tokens;
    let (gutter, inner) = (numbers.width, numbers.inside(self.inner()));
    let since = match line.mark {
      Mark::Runs { since: Some(since), .. } if self.model.now > since => {
        Some(self.model.now - since)
      }
      _ => None,
    };
    let (number, color, loader) = match &line.mark {
      Mark::Runs { work, .. } => (" ".to_owned(), t.accent, Some(loader(*work))),
      Mark::Retries(_) => (" ".to_owned(), t.accent, Some(Loader::Model)),
      Mark::Asks => ("◆".to_owned(), t.attention, None),
      Mark::Failed { .. } => ("✗".to_owned(), t.danger, None),
      Mark::Ran => (line.number.to_string(), t.muted, None),
    };
    let room = inner.saturating_sub(gutter + 2);
    let (mut under, look) = match &line.mark {
      Mark::Failed { why } => (wrapped(why, room), Style::new().fg(t.danger)),
      Mark::Retries(retry) => (wrapped(&retry.why, room), self.style(Role::Muted)),
      _ => (Vec::new(), Style::new()),
    };
    let colors = match &line.mark {
      Mark::Retries(retry) => self.retry(retry),
      Mark::Failed { .. } if line.text.is_empty() && !under.is_empty() => {
        vec![run(under.remove(0), look)]
      }
      _ => colors,
    };
    let ground = if loader.is_some() { t.lifted } else { t.raised };
    let timer =
      since.map(|since| vec![run(timer(since), self.style(Role::Muted))]).unwrap_or_default();
    let room = room.saturating_sub(width(&timer) + 2 * usize::from(!timer.is_empty()));
    let mut text = vec![run(format!("{number:>gutter$}  "), Style::new().fg(color))];
    text.extend(clipped(colors, room, self.style(Role::Muted)));
    let shown = self.boxed(spread(text, timer, inner, Style::new()), ground);
    let mut rows =
      vec![Row { loader: loader.map(|one| (PAD + gutter - 1, one)), ..Row::of(shown) }];
    let under = under
      .iter()
      .map(|one| self.boxed(vec![run(format!("{:gutter$}  {one}", ""), look)], t.raised));
    rows.extend(under.map(Row::of));
    rows
  }

  /// What a line whose model is asked again says: the attempt that comes of the most, and the time
  /// left before it while it waits. The time left is never more than the wait, so a clock of the
  /// window that stands behind the host shows the wait.
  fn retry(&self, retry: &Retrying) -> Line {
    let Retrying { attempt, most, wait, at, .. } = retry;
    let left = at.saturating_sub(self.model.now).min(*wait);
    let wait = if left > 0 { format!(" in {}", timer(left)) } else { String::new() };
    vec![run(
      format!("Attempt {attempt} of {most}{wait}"),
      Style::new().fg(self.model.tokens.accent),
    )]
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

  /// What an answer said, as markdown from the column of the text. A question is one line with its
  /// mark: its first line while it waits, since its form shows it whole, then what closed it.
  fn said(&self, said: &Said) -> Vec<Row> {
    let t = &self.model.tokens;
    if let Saying::Question { answered, .. } = &said.kind {
      let first = said.text.lines().find(|line| !line.trim().is_empty()).unwrap_or_default();
      let (mark, text) = match answered {
        None => (Style::new().fg(t.attention), self.style(Role::Prose)),
        Some(_) => (self.style(Role::Muted), self.style(Role::Muted)),
      };
      let left = vec![run("  ◆ ", mark), run(self.model.homed(first), text)];
      let login = &self.model.machines[self.opened.key.0].login;
      let right =
        answered.iter().map(|one| run(closed(one, login), self.style(Role::Muted))).collect();
      return vec![
        Row::of(spread(left, right, self.wide.saturating_sub(2), Style::new())),
        Row::default(),
      ];
    }
    let lines = prose::lines(
      &self.model.homed(&said.text),
      self.wide.saturating_sub(4),
      t,
      &self.model.colors,
    );
    let mut rows: Vec<Row> = lines
      .into_iter()
      .map(|mut line| {
        line.insert(0, run("  ", Style::new()));
        Row::of(line)
      })
      .collect();
    rows.push(Row::default());
    rows
  }

  /// The dock of the agents of the thread: the agents from the first that does not rest to the
  /// last, so an agent that ends among others that work keeps its row with its mark, and one row
  /// that counts the agents before them, with a blank row above and under them. Each agent takes a
  /// row with its title, its state, its time and its model, and one more with the line where it
  /// stands while it lives. A click on an agent opens its thread in place.
  pub(crate) fn dock(&self) -> Vec<Row> {
    let (done, shown) = self.opened.docked();
    let mut rows: Vec<Row> = shown.iter().flat_map(|agent| self.agent(agent)).collect();
    if done > 0 {
      let noun = if done == 1 { "agent" } else { "agents" };
      rows.insert(0, says(&format!("└ {done} {noun} done"), self.style(Role::Muted)));
    }
    if !rows.is_empty() {
      rows.insert(0, Row::default());
      rows.push(Row::default());
    }
    rows
  }

  /// The rows of one agent of the dock: its title with its state, its time and its model, then
  /// the line where it stands while it lives. Each takes a click that opens its thread.
  fn agent(&self, agent: &Agent) -> Vec<Row> {
    let t = &self.model.tokens;
    let card = &agent.card;
    let (muted, rest) = (self.style(Role::Muted), rests(agent));
    // A time shows from one second on, as the time of a line that runs.
    let since =
      agent.began.filter(|began| self.model.now > *began).map(|began| self.model.now - began);
    let time = agent.took.or(since).filter(|seconds| *seconds > 0).map(timer);
    let working = |loader| (Some(loader), "  Working", t.accent);
    let (mark, word, color) = match &card.standing {
      Standing::Asks { .. } => (None, "◆ Input", t.attention),
      Standing::Failed => (None, "✗ Failed", t.danger),
      Standing::Paused { .. } => (None, "‖ Paused", t.attention),
      Standing::Thinks => working(Loader::Model),
      Standing::Runs => working(Loader::Command),
      Standing::Delegates => working(Loader::Agent),
      Standing::Stopped => (None, "Stopped", t.muted),
      Standing::Rests | Standing::Resolved => (None, "✓ Resolved", t.muted),
    };
    let model = card.model.as_deref().and_then(|one| one.rsplit(':').next()).unwrap_or_default();
    let title = if rest { muted } else { self.style(Role::Prose) };
    let left = vec![run("  └ ", muted), run(card.title.clone(), title)];
    let mut right = vec![run(word, Style::new().fg(color))];
    right.extend(time.map(|time| run(format!(" {time}"), muted)));
    right.push(run(format!("  {model}"), muted));
    let line = spread(left, right.clone(), self.wide.saturating_sub(2), Style::new());
    let x = width(&line) - width(&right);
    let open = Click::Agent(card.prompt.clone());
    let mut rows =
      vec![Row { loader: mark.map(|one| (x, one)), ..Row::clicked(line, open.clone()) }];
    if let Some(stands) = agent.line.as_ref().filter(|_| !rest) {
      let text = vec![run(self.model.homed(&stands.text), muted)];
      let room = self.wide.saturating_sub(8);
      let mut line = vec![run("    ", Style::new())];
      line.extend(clipped(text, room, muted));
      rows.push(Row::clicked(line, open));
    }
    rows
  }
}

/// A row of prose from the column of the text.
fn says(text: &str, style: Style) -> Row {
  Row::of(vec![run(format!("  {text}"), style)])
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
pub(crate) fn maker(maker: &Maker) -> String {
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
