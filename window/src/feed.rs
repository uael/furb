//! The rows of an open thread, from the exchanges that its daemon gave: the message of the operator
//! on its band, the block of code of the answer, which rolls in seven rows, the thought of its
//! model on one row, what the answer said as markdown, and the messages of the operator that the
//! thread does not show yet, each agent under the line of the block that made it, then the dock of
//! the agents that no line made.

use furb::view::{
  Agent, Block, Exchange, Line as Code, Made, Maker, Mark, Output, Retrying, Said, Saying,
  Standing, Thought, Work, lines, operator, timer,
};
use furb_client::{Fold, Opened, Target, rests};
use furb_host::protocol::{Card, Present};
use opentui::style::{Color, Span, Style};
use unicode_width::UnicodeWidthStr;

use crate::{
  code::Gutter,
  input::Armed,
  keys::Does,
  line::{self, Line, capital, clipped, run, spread, wrapped},
  list::project,
  model::Model,
  picture, prose,
  question::closed,
  thread::{Click, Item, Items, Row, width},
  tokens::{APART, Loader, PROMPT, Role, WORKS},
};

/// The word on the line of a block whose model has not begun to write it.
const WRITES: &str = "writes";

/// The word of the thought of a model while it thinks.
const THINKS: &str = "thinks";

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
    let opened = self.client.thread.as_ref()?;
    Some(Feed { model: self, opened, wide: self.wide() })
  }
}

impl Feed<'_> {
  /// The rows of one part of the feed. The last part holds the way out of a pause that holds the
  /// thread, then each message of the operator that the thread does not show yet, then each
  /// follow-up that waits on the thread.
  pub(crate) fn rows(&self, item: Item) -> Vec<Row> {
    let mut rows = match item {
      Item::Head => self.before(),
      Item::At(at) => {
        self.opened.thread.exchanges.get(&at).map(|one| self.exchange(one)).unwrap_or_default()
      }
    };
    if self.items().last() == Some(item) {
      rows.extend(self.paused());
      rows.extend(self.opened.thread.pending.iter().flat_map(|one| self.message(&one.text, true)));
      rows.extend(self.queued());
    }
    rows
  }

  /// Each follow-up that waits on the thread, which its life sends once the chain of the thread
  /// rests, as a chip and its message, and the cross that takes it back to the input. The part
  /// above it ends with a blank row.
  fn queued(&self) -> Vec<Row> {
    let waiting = self.model.client.follow_ups();
    waiting
      .into_iter()
      .map(|one| {
        let chip = run(" queued ", self.style(Role::Button));
        let back = " ✕";
        let room = self.wide.saturating_sub(2 + 8 + 2 + back.width());
        let text = one.message.lines().next().unwrap_or_default();
        // The chip starts at the column of the text of a message and of what a model said.
        let mut line = vec![run("  ", Style::new()), chip, run("  ", Style::new())];
        line.push(run(line::cut(text, room), self.style(Role::Muted)));
        let end = width(&line);
        line.push(run(back, self.style(Role::Muted)));
        let hot = vec![(end..end + back.width(), Some(Click::Unqueue(one.entry.clone())))];
        Row { hot, ..Row::of(line) }
      })
      .collect()
  }

  /// What a pause that holds the thread leaves him: that it holds, and the way out as buttons. His
  /// resume wakes it: the work that a pause of his held goes on, and a pause that furb said after a
  /// failure, which the line above shows, asks the model again. A message resumes it too, and
  /// nothing is lost. A pause of furb offers another model as well.
  fn paused(&self) -> Vec<Row> {
    let card = self.model.client.card(&self.opened.thread.key);
    let Some(Standing::Paused { by, .. }) = card.map(|one| &one.thread.standing) else {
      return Vec::new();
    };
    let (t, his) = (&self.model.tokens, operator(by));
    let resume = (" Resume ", Does::Pause);
    let (mark, said, chips) = if his {
      (t.muted, "You paused this thread.", vec![resume])
    } else {
      let said = "This thread is paused. Nothing is lost.";
      (t.attention, said, vec![resume, (" Change the model ", Does::Model)])
    };
    let mut rows: Vec<Row> = wrapped(said, self.wide.saturating_sub(6))
      .into_iter()
      .enumerate()
      .map(|(at, text)| {
        let lead = run(if at == 0 { "  ‖ " } else { "    " }, Style::new().fg(mark));
        Row::of(vec![lead, run(text, self.style(Role::Prose))])
      })
      .collect();
    let (mut line, mut hot) = (vec![run("    ", Style::new())], Vec::new());
    for (label, does) in chips {
      let at = width(&line);
      line.extend([run(label, self.style(Role::Button)), run("  ", Style::new())]);
      hot.push((at..at + label.width(), Some(Click::Key(does))));
    }
    rows.extend([Row::default(), Row { hot, ..Row::of(line) }, Row::default()]);
    rows
  }

  fn style(&self, role: Role) -> Style {
    self.model.tokens.style(role)
  }

  /// Why a fault ended the life of the thread, which then only reads.
  fn fault(&self) -> Option<&str> {
    self.model.client.fault_of(self.opened.thread.key.0, &self.opened.thread.key.1)
  }

  /// Whether a fault ended the life of the thread.
  fn ended(&self) -> bool {
    self.fault().is_some()
  }

  /// The parts of the feed. The head shows until the first exchange of the thread is held, and
  /// holds the messages of a thread that the daemon did not make yet, and why a fault ended the
  /// life of the thread.
  pub(crate) fn items(&self) -> Items<'_, Exchange> {
    let opened = self.opened;
    let head = self.ended() || opened.thread.fault.is_some() || opened.thread.above != Some(0);
    Items { held: &opened.thread.exchanges, head: head || opened.thread.exchanges.is_empty() }
  }

  /// What stands above the exchanges that the window holds: why a fault ended the life, whose
  /// threads the window still reads, then why the daemon gave no page, the wait for the first page,
  /// or how many messages come before.
  fn before(&self) -> Vec<Row> {
    let Some(why) = self.fault() else { return self.above() };
    let room = self.wide.saturating_sub(4);
    let said = |text: &str, style: Style| -> Vec<Row> {
      wrapped(text, room).iter().map(|line| says(line, style)).collect()
    };
    let mut rows =
      said(&capital(&self.model.homed(why)), Style::new().fg(self.model.tokens.danger));
    rows.push(Row::default());
    let key = self.model.key(Does::Close);
    let read = format!("The window reads this session and sends it nothing. {key} goes back.");
    rows.extend(said(&read, self.style(Role::Muted)));
    rows.push(Row::default());
    rows.extend(self.above());
    rows
  }

  /// What stands above the exchanges that the window holds in a life that reads: why the daemon
  /// gave no page, the wait for the first page, or how many messages come before.
  fn above(&self) -> Vec<Row> {
    let muted = self.style(Role::Muted);
    let mut rows: Vec<Row> = match (&self.opened.thread.fault, self.opened.thread.above) {
      (Some(why), _) => {
        let lines = wrapped(&self.model.homed(why), self.wide.saturating_sub(4));
        let danger = Style::new().fg(self.model.tokens.danger);
        let mut rows: Vec<Row> = lines.iter().map(|line| says(line, danger)).collect();
        let back =
          format!("The record stays as it was. {} goes back.", self.model.key(Does::Close));
        rows.extend([Row::default(), says(&back, muted)]);
        rows
      }
      // Until its page comes, the thread shows what its card holds: the first line of its message,
      // which a model gave to the thread of an agent.
      (None, None) => {
        let his = self.opened.from.is_none();
        return self.model.title().map(|title| self.message(&title, his)).unwrap_or_default();
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
  /// ended shows how, and while the input sends elsewhere its button turns the input to it, since a
  /// reply opens it again. A resolve
  /// of a thread that works waits for a confirm on the button, which says that it stops the work.
  /// The thread of an agent says that it is one, and the button of a thread that the operator
  /// opened from another, an agent or a thread of a chain, goes back to that one.
  pub(crate) fn head(&self) -> Row {
    let t = &self.model.tokens;
    let root = self.opened.root();
    let untold = self.model.untold();
    let card = self.model.client.card(&root.thread.key).or(untold.as_ref().map(|one| &one.1));
    let project = card.map(project).unwrap_or_default();
    let named = card.and_then(|card| card.session.clone()).filter(|name| !name.is_empty());
    let muted = self.style(Role::Muted);
    let title = self.model.title().unwrap_or_default();
    let (branch, branches) = self.branch();
    let (mark, title) = match (&self.opened.from, self.opened.agent(), branch) {
      (Some(_), _, Some(at)) => (format!("⑂ {} · ", at + 2), title),
      (Some(_), Some(_), None) => ("agent · ".to_owned(), title),
      (Some(_), None, None) => (String::new(), title),
      (None, ..) if branches > 0 => (format!("⑂ 1 of {} · ", branches + 1), named.unwrap_or(title)),
      (None, ..) => (String::new(), named.unwrap_or(title)),
    };
    // The head marks a session whose life runs as its card does, in the column of the bars of the
    // messages, so its project stands over their text.
    let runs = card.is_some_and(|card| self.model.live(&root.thread.key, card));
    let mut left = vec![
      run(if runs { "●" } else { " " }, Style::new().fg(t.live)),
      run(format!(" {project} · "), muted),
    ];
    let forks = (width(&left)..width(&left) + mark.width() - 3, branches > 0);
    left.extend([run(mark, muted), run(title, self.style(Role::Title))]);
    let (mark, button) = self.closer(card, branch);
    let apart = |mark| [mark, run(" ".repeat(APART), Style::new())];
    let right: Line = self
      .present()
      .into_iter()
      .chain(mark)
      .flat_map(apart)
      .chain(button.clone().map(|one| one.0))
      .collect();
    let line = spread(left, right, self.wide.saturating_sub(2), Style::new());
    let end = width(&line);
    let hot = button.map(|(one, does)| (end - one.content.width()..end, Some(Click::Key(does))));
    // The mark of the branches of a thread opens its last branch.
    let last = self.opened.thread.branches().last().map(|one| one.card.prompt.clone());
    let fork = last.filter(|_| forks.1).map(|prompt| (forks.0, Some(Click::Agent(prompt))));
    Row { hot: hot.into_iter().chain(fork).collect(), ..Row::of(line) }
  }

  /// The mark of how the thread of the head ended, and its button: back to a branch or to the
  /// thread that the operator came from, a reopen, a resolve, or a stop and resolve that confirms.
  fn closer(
    &self,
    card: Option<&Card>,
    branch: Option<usize>,
  ) -> (Option<Span<'static>>, Option<(Span<'static>, Does)>) {
    let t = &self.model.tokens;
    let (button, muted) = (self.style(Role::Button), self.style(Role::Muted));
    // A life that a fault ended takes no resolve: it only reads.
    let made = !self.opened.thread.key.2.is_empty() && !self.model.leaves() && !self.ended();
    let (mark, button) = match card.map(|card| &card.thread) {
      _ if branch.is_some() => (None, Some((run(" ⑂ 1 ", button), Does::Close))),
      _ if self.opened.from.is_some() => (None, Some((run(" Back ", button), Does::Close))),
      _ if !made => (None, None),
      Some(thread) if thread.ended() => {
        let mark = match thread.standing {
          Standing::Resolved => run("✓ Resolved", muted),
          Standing::Failed => run("✗ Failed", Style::new().fg(t.danger)),
          _ => run("Stopped", muted),
        };
        // The button turns the input to the thread, so it stands only while the input sends
        // elsewhere or runs a word: a button that would change nothing does not show.
        let replies = self.model.client.target() == Target::Thread(self.opened.thread.key.clone())
          && !self.model.client.draft.python;
        (Some(mark), (!replies).then(|| (run(" Reopen ", button), Does::Resolve)))
      }
      Some(thread)
        if thread.works()
          && self.model.armed == Some(Armed::Resolve(self.opened.thread.key.clone())) =>
      {
        (None, Some((run(" Stop and resolve ", button.fg(t.attention)), Does::Resolve)))
      }
      _ => (None, Some((run(" Resolve ", button), Does::Resolve))),
    };
    // A window that only reads shows no button that writes: a resolve, a reopen or a stop.
    let reads = self.model.client.only_reads(self.opened.thread.key.0).is_some();
    let button = button.filter(|(_, does)| !reads || *does != Does::Resolve);
    (mark, button)
  }

  /// The place of the thread among the branches of the thread that it came from, when a rewind
  /// made it, and how many branches the thread itself made. The selected card names the thread
  /// that an agent came from, so the head names the agent; a rewind is a branch of that thread, the
  /// first branch being the thread itself, whose head names how many branches it has.
  fn branch(&self) -> (Option<usize>, usize) {
    let branch = self.opened.from.as_deref().and_then(|from| {
      let mine = |one: &&Agent| one.card.prompt == self.opened.thread.key.2;
      from.thread.branches().iter().position(mine)
    });
    (branch, self.opened.thread.branches().len())
  }

  /// Who else is on the session of the thread: the login of each other operator whose window holds
  /// its life, and the one who writes in this thread first, in the accent.
  fn present(&self) -> Option<Span<'static>> {
    let (at, life, thread) = &self.opened.thread.key;
    let others = self.model.client.others(*at, life);
    let here = |one: &Present| one.writes.as_ref().is_some_and(|it| it.on.as_ref() == Some(thread));
    if let Some(one) = others.iter().find(|one| here(one)) {
      let accent = Style::new().fg(self.model.tokens.accent);
      return Some(run(format!("◉ {} writes…", one.operator), accent));
    }
    let names: Vec<&str> = others.iter().map(|one| one.operator.as_str()).collect();
    (!names.is_empty()).then(|| run(format!("◉ {}", names.join(", ")), self.style(Role::Muted)))
  }

  /// The last row of the feed while the operator reads away from its end.
  pub(crate) fn jump(&self) -> Row {
    let jump = run("  ↓ Jump to latest", Style::new().fg(self.model.tokens.accent));
    let key = run(format!("  {}", self.model.key(Does::Jump)), self.style(Role::Muted));
    Row::clicked(vec![jump, key], Click::Key(Does::Jump))
  }

  /// The rows of one exchange: the message of the operator on its band, the block, the thought,
  /// and what the answer said. A word of Python that he ran is his message as code: its block on
  /// the band of his messages, with no head.
  fn exchange(&self, exchange: &Exchange) -> Vec<Row> {
    let his = matches!(exchange.by, Maker::Operator | Maker::Member { .. });
    let mut rows = Vec::new();
    match &exchange.block {
      Some(block) if exchange.python => rows.extend(self.word(&exchange.message, block)),
      block => {
        rows.extend(self.message(&exchange.text, his));
        if let Some(block) = block {
          // The head of the block takes the blank row under the message.
          rows.pop();
          rows.extend(self.block(&exchange.message, block));
        }
      }
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
    let hue = if his { self.model.hue(&self.opened.thread.key) } else { t.muted };
    let bar = run("┃", Style::new().fg(hue).bg(t.raised));
    let banded = |text: Line| {
      let mut line = vec![bar.clone(), run(" ", band)];
      line.extend(spread(text, Vec::new(), self.wide.saturating_sub(4), band));
      Row::of(line)
    };
    let lines = picture::shown(&self.model.homed(text))
      .lines()
      .flat_map(|line| wrapped(line, self.wide.saturating_sub(6)))
      .collect::<Vec<_>>();
    // An image that the message names stands as its mark, as the input shows it.
    let (plain, mark) =
      (self.style(Role::Prose).bg(t.raised), Style::new().fg(t.accent).bg(t.lifted));
    let text = lines.into_iter().map(|line| banded(picture::marked(&line, plain, mark)));
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

  /// A word of Python that the operator ran: the lines of its block on the band of his messages,
  /// with the bar in the color of the thread and a blank row of the band above and under them, so
  /// his code reads as a message of his, and its numbers stand in the column of those of a model.
  fn word(&self, message: &str, block: &Block) -> Vec<Row> {
    let t = &self.model.tokens;
    let bar = run("┃", Style::new().fg(self.model.hue(&self.opened.thread.key)).bg(t.raised));
    let blank = Row::of(self.boxed(Vec::new(), t.raised));
    let mut rows = vec![blank.clone()];
    rows.extend(self.lines(message, block));
    rows.push(blank);
    for row in &mut rows {
      row.line.splice(..1, [bar.clone(), run(" ", Style::new().bg(t.raised))]);
    }
    rows.push(Row::default());
    rows
  }

  /// The block of an answer, as a block of code of the markdown: the model as its head, on the
  /// row above the box at its right, where the times of its lines end, then the window of seven
  /// rows that rolls, or each line once the operator opened it, each with its number in the
  /// thread, close to the left of the box. The thought stands under it.
  fn block(&self, message: &str, block: &Block) -> Vec<Row> {
    let numbers = Gutter::of(block.lines.iter().map(|one| one.number).max().unwrap_or_default());
    let head = vec![run(block.maker.shown(), self.style(Role::Muted))];
    let mut head = spread(Vec::new(), head, numbers.inside(self.inner()), Style::new());
    head.insert(0, run(" ".repeat(PAD), Style::new()));
    let mut rows = vec![Row::of(head)];
    rows.extend(self.lines(message, block));
    if let Some(thought) = &block.thought {
      rows.push(Row::default());
      rows.extend(self.thought(message, thought));
    }
    rows.push(Row::default());
    rows
  }

  /// The lines of a block in its box: the window of seven rows that rolls around the line where
  /// its word stands, or each line once the operator opened it. A row above says how many lines
  /// stand above, and a row below how many follow, each a click that opens the block, or that
  /// more comes.
  fn lines(&self, message: &str, block: &Block) -> Vec<Row> {
    let t = &self.model.tokens;
    let muted = self.style(Role::Muted);
    let numbers = Gutter::of(block.lines.iter().map(|one| one.number).max().unwrap_or_default());
    let gutter = numbers.width;
    let mut rows = Vec::new();
    let code = self.style(Role::Prose).bg(t.raised);
    let text = self
      .model
      .homed(&block.lines.iter().map(|one| one.text.as_str()).collect::<Vec<_>>().join("\n"));
    let colored = self.model.colors.lines(&text, code);
    let all = block.lines.len();
    let fold = Fold::Block(message.to_owned());
    let open = self.opened.unfolded.contains(&fold);
    let shown = block.shown(open);
    let counted = |mark: &str, count: usize| {
      let line = vec![run(format!("{mark:>gutter$}  {}", lines(count)), muted)];
      Row::clicked(self.boxed(line, t.raised), Click::Fold(fold.clone()))
    };
    if open && block.rolls() {
      rows.push(counted("▴", all));
    } else if shown.start > 0 {
      rows.push(counted("…", shown.start));
    }
    let (follow, end) = (all - shown.end, shown.end);
    for (line, colors) in block.lines.iter().zip(colored.iter()).take(end).skip(shown.start) {
      rows.extend(self.code(line, colors.clone(), numbers));
      rows.extend(self.agents(Some((line, numbers))));
    }
    if follow > 0 {
      rows.push(counted("…", follow));
    } else if block.more {
      rows.push(Row::of(self.boxed(vec![run(format!("{:>gutter$}", "…"), muted)], t.raised)));
    }
    rows
  }

  /// One line of a block: its number in a gutter of a count of columns, or the mark of how it
  /// stands in its place, and its code, cut to the box. The line that runs is lit, with its loader
  /// and its time after one second, a line that a stop ended says when, and the line whose model
  /// is asked again says the attempt that comes and its wait. What the command of a line wrote
  /// stands under it, and the time that the command took at its right, from one second on. Why a
  /// line failed, or why its model is asked again, stands under it, or in its place when its model
  /// wrote none of it.
  fn code(&self, line: &Code, colors: Line, numbers: Gutter) -> Vec<Row> {
    let t = &self.model.tokens;
    let (gutter, inner) = (numbers.width, numbers.inside(self.inner()));
    // A line that the record left at work in a life that a fault ended works no more.
    let mark = match &line.mark {
      Mark::Runs { .. } | Mark::Retries(_) if self.ended() => &Mark::Ran,
      mark => mark,
    };
    let (number, color, loader) = match mark {
      Mark::Runs { work, .. } => (" ".to_owned(), t.accent, Some(loader(*work))),
      Mark::Retries(_) => (" ".to_owned(), t.accent, Some(Loader::Model)),
      Mark::Asks => ("◆".to_owned(), t.attention, None),
      Mark::Failed { .. } => ("✗".to_owned(), t.danger, None),
      Mark::Stopped { .. } => ("■".to_owned(), t.muted, None),
      Mark::Ran => (line.number.to_string(), t.muted, None),
    };
    let room = inner.saturating_sub(gutter + 2);
    let output = line.output.as_ref().map(|one| self.output(one, gutter, room));
    let (mut under, look) = match mark {
      Mark::Failed { why } => (wrapped(why, room), Style::new().fg(t.danger)),
      Mark::Retries(retry) => (wrapped(&retry.why, room), self.style(Role::Muted)),
      _ => (Vec::new(), Style::new()),
    };
    let writes = matches!(mark, Mark::Runs { work: Work::Writes, .. }) && line.text.is_empty();
    let colors = match mark {
      Mark::Retries(retry) => self.retry(retry),
      Mark::Failed { .. } if line.text.is_empty() && !under.is_empty() => {
        vec![run(under.remove(0), look)]
      }
      // A model asked for a line that it has not begun says so, so the lit row is never empty.
      _ if writes => vec![run(WRITES, self.style(Role::Muted).italic())],
      _ => colors,
    };
    let ground = if loader.is_some() { t.lifted } else { t.raised };
    let muted = self.style(Role::Muted);
    let timer = self.right(line, mark).map(|right| vec![run(right, muted)]).unwrap_or_default();
    let room = room.saturating_sub(width(&timer) + 2 * usize::from(!timer.is_empty()));
    let head = run(format!("{number:>gutter$}  "), Style::new().fg(color));
    let lined = |colors: Line| {
      let text = [head.clone()].into_iter().chain(clipped(colors, room, muted)).collect();
      self.boxed(spread(text, timer.clone(), inner, Style::new()), ground)
    };
    let pointed =
      self.gave(line, matches!(mark, Mark::Ran)).map(|gave| lined([colors.clone(), gave].concat()));
    // The light passes over the word that says that the model writes, as over each word of work,
    // and over the call of `prompt` that the line awaits while its agent works: the one call that
    // makes code and value in furb.
    let code = PAD + gutter + 2;
    let writes = (writes && loader.is_some()).then_some((code, Loader::Light(WRITES)));
    let shines = matches!(mark, Mark::Runs { work: Work::Agent, .. }).then(|| {
      let shown: String =
        clipped(colors.clone(), room, muted).iter().map(|one| one.content.as_ref()).collect();
      let at = shown.find(&format!("{PROMPT}("))?;
      Some((code + shown[..at].width(), Loader::Light(PROMPT)))
    });
    let loaders = loader.map(|one| (PAD + gutter - 1, one));
    let loaders = loaders.into_iter().chain(writes).chain(shines.flatten()).collect();
    let mut rows = vec![Row { loaders, pointed, ..Row::of(lined(colors)) }];
    rows.extend(output.into_iter().flatten());
    let under = under
      .iter()
      .map(|one| self.boxed(vec![run(format!("{:gutter$}  {one}", ""), look)], t.raised));
    rows.extend(under.map(Row::of));
    rows
  }

  /// What stands at the right of a line: the time of its work, or how long ago a stop ended it,
  /// from one minute on, as the age of a card, or the time that its command took.
  fn right(&self, line: &Code, mark: &Mark) -> Option<String> {
    match *mark {
      Mark::Runs { began: Some(began), .. } => self.model.since(began).map(timer),
      Mark::Stopped { at } => {
        let age = at.and_then(|at| self.model.age(at / 1000));
        Some(age.map_or_else(|| "stopped".to_owned(), |age| format!("stopped {age} ago")))
      }
      _ => line.output.as_ref().and_then(|one| one.took).filter(|took| *took > 0).map(timer),
    }
  }

  /// The value that the last act of a line that ran came to, as a debugger shows it beside the
  /// code while the pointer is on the line, when no row under the line says it already, as what a
  /// command wrote or an agent.
  fn gave(&self, line: &Code, ran: bool) -> Option<Line> {
    let (done, agents) = self.opened.made(Some(line.number));
    let said = done + agents.len() > 0 || line.output.is_some();
    let value = match line.made.last() {
      Some(Made::Done(value)) if ran && !said && value != "None" => value,
      _ => return None,
    };
    let muted = self.style(Role::Muted);
    Some(vec![run("  → ", muted), run(self.model.homed(value), muted)])
  }

  /// What the command of a line wrote, under it at the column of the code: how many lines came
  /// before its last ones, as the block says the lines above it, then those, muted, cut to the box,
  /// with the home folder written `~`, as the code above them writes it.
  fn output(&self, output: &Output, gutter: usize, room: usize) -> Vec<Row> {
    let muted = self.style(Role::Muted);
    let more = (output.more > 0).then(|| format!("… {}", lines(output.more)));
    let lines = more.into_iter().chain(output.tail.iter().map(|line| self.model.homed(line)));
    let row = |text: String| {
      let mut shown = vec![run(format!("{:gutter$}  ", ""), muted)];
      shown.extend(clipped(vec![run(text, muted)], room, muted));
      Row::of(self.boxed(shown, self.model.tokens.raised))
    };
    lines.map(row).collect()
  }

  /// What a line whose model is asked again says, in the accent: furb works.
  fn retry(&self, retry: &Retrying) -> Line {
    vec![run(retry.said(self.model.second()), Style::new().fg(self.model.tokens.accent))]
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
    // The word says what the row is, and the time stands at the right, in the column of the time of
    // a line that runs: it counts while the model thinks, and stays as the time that it took.
    let muted = self.style(Role::Muted);
    let (word, text, seconds) = match thought.took {
      // The last words keep room for the word before them and a time of eight columns after.
      None => {
        let words = tail(&thought.text, room.saturating_sub(" thinks · ".len() + 2 + 8));
        (THINKS, words, self.model.since(thought.began))
      }
      Some(took) => ("thought", thought.first().to_owned(), Some(took).filter(|one| *one > 0)),
    };
    let time = seconds.map(|seconds| run(timer(seconds), muted));
    let said = vec![run(format!("  {word} · "), muted), run(text, style)];
    let line = spread(said, time.into_iter().collect(), room + 2, Style::new());
    // While the model thinks, the light passes over the word that says so.
    let light = thought.took.is_none().then_some((2, Loader::Light(THINKS)));
    vec![Row { loaders: light.into_iter().collect(), ..Row::clicked(line, Click::Fold(fold)) }]
  }

  /// What an answer said, as markdown from the column of the text. A question is one line with its
  /// mark: its first line while it waits, then what closed it. While its form shows it whole, the
  /// form is its one place, and the feed shows none of it.
  fn said(&self, said: &Said) -> Vec<Row> {
    let t = &self.model.tokens;
    if let Saying::Question { answered, .. } = &said.kind {
      let formed = self.opened.form.as_ref().is_some_and(|form| form.act == said.act);
      if answered.is_none() && formed {
        return Vec::new();
      }
      let first = said.text.lines().find(|line| !line.trim().is_empty()).unwrap_or_default();
      let (mark, text) = match answered {
        None => (Style::new().fg(t.attention), self.style(Role::Prose)),
        Some(_) => (self.style(Role::Muted), self.style(Role::Muted)),
      };
      let left = vec![run("  ◆ ", mark), run(self.model.homed(first), text)];
      let login = &self.model.client.machines[self.opened.thread.key.0].login;
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

  /// The dock of the agents of the thread that no line of a block made, with a blank row above
  /// and under them.
  pub(crate) fn dock(&self) -> Vec<Row> {
    let mut rows = self.agents(None);
    if !rows.is_empty() {
      rows.insert(0, Row::default());
      rows.push(Row::default());
    }
    rows
  }

  /// The agents that a line made, in the box of its block at the column of its code, or those of
  /// the dock across the thread: from the first that does not rest to the last, so an agent that
  /// ends among others that work keeps its row with its mark, and one row that counts the agents
  /// before them. A click on an agent opens its thread in place.
  fn agents(&self, under: Option<(&Code, Gutter)>) -> Vec<Row> {
    let (done, shown) = self.opened.made(under.map(|(line, _)| line.number));
    // A line whose acts are its agents alone says what each came to in their order.
    let gave = under
      .map(|(line, _)| line.made.as_slice())
      .filter(|made| made.len() == done + shown.len())
      .unwrap_or_default();
    let mut rows: Vec<Row> = shown
      .into_iter()
      .enumerate()
      .map(|(at, agent)| {
        let value = match gave.get(done + at) {
          Some(Made::Done(value)) => Some(value.as_str()),
          _ => None,
        };
        self.agent(agent, under.map(|(_, gutter)| (gutter, value)))
      })
      .collect();
    if done > 0 {
      let noun = if done == 1 { "agent" } else { "agents" };
      let (muted, ground) = (self.style(Role::Muted), self.model.tokens.raised);
      rows.insert(
        0,
        match under {
          Some((_, numbers)) => {
            let said = format!("{:>width$}  └ {done} {noun} done", "", width = numbers.width);
            Row::of(self.boxed(vec![run(said, muted)], ground))
          }
          None => says(&format!("└ {done} {noun} done"), muted),
        },
      );
    }
    rows
  }

  /// The row of one agent, a click that opens its thread, with its state, its time and its model
  /// at the right. In the dock its title leads, then the line where it stands while it lives. Under
  /// the line that made it, which says its prompt already, the row stands in the box at the column
  /// of the code, and says the line where the agent stands while it has one, the last line of its
  /// thought while its model only thinks, the value that it gave once it rests, and else its title.
  fn agent(&self, agent: &Agent, under: Option<(Gutter, Option<&str>)>) -> Row {
    let (muted, rest) = (self.style(Role::Muted), rests(agent));
    let (mark, right) = self.status(agent);
    // The line that its model has not begun to write says nothing that its state does not say.
    let stands = agent.line.as_ref().filter(|one| !rest && !one.text.is_empty());
    let stands = stands.map(|one| vec![run(self.model.homed(&one.text), muted)]);
    // While its model only thinks, the row peeks at the last line of the thought, in the form of
    // the row of a thought under a block.
    let thinks = agent
      .thought
      .as_ref()
      .filter(|_| !rest)
      .map(|one| vec![run(format!("{THINKS} · "), muted), run(one.clone(), muted.italic())]);
    let stands = stands.or(thinks);
    let title = if rest { muted } else { self.style(Role::Prose) };
    let title = vec![run(agent.card.title.clone(), title)];
    let open = Click::Agent(agent.card.prompt.clone());
    let Some((numbers, value)) = under else {
      let room = self.wide.saturating_sub(2);
      let said = title
        .into_iter()
        .chain(stands.into_iter().flat_map(|text| [run("  ", muted)].into_iter().chain(text)));
      let fits = room.saturating_sub(width(&right) + 6);
      let left = [run("  └ ", muted)].into_iter().chain(clipped(said.collect(), fits, muted));
      let line = spread(left.collect(), right.clone(), room, Style::new());
      let x = width(&line) - width(&right);
      return titled(Row::clicked(line, open), x, &right, mark);
    };
    let lead = vec![run(format!("{:>width$}  └ ", "", width = numbers.width), muted)];
    let room = numbers.inside(self.inner());
    let gave = value
      .filter(|_| rest)
      .and_then(|value| self.model.colors.lines(value, muted).first().cloned());
    let text = stands.or(gave).unwrap_or(title);
    let left = lead.into_iter().chain(clipped(text, room, muted)).collect();
    let line = spread(left, right.clone(), room, Style::new());
    let x = PAD + width(&line) - width(&right);
    titled(Row::clicked(self.boxed(line, self.model.tokens.raised), open), x, &right, mark)
  }

  /// The state of an agent with its time and its model, and the mark that moves before its word
  /// while it works.
  fn status(&self, agent: &Agent) -> (Option<Loader>, Line) {
    let t = &self.model.tokens;
    let (card, muted) = (&agent.card, self.style(Role::Muted));
    // An agent that ended says how long it took, from one second on, as one that works does.
    let since = agent.began.and_then(|began| self.model.since(began));
    let time = agent.took.map_or(since, |took| Some(took).filter(|one| *one > 0)).map(timer);
    let working = |loader| (Some(loader), WORKS, t.accent);
    let (mark, word, color) = match &card.standing {
      _ if self.ended() && card.works() => (None, "Ended", t.muted),
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
    // A mark that moves stands in the two cells before its word.
    let word = if mark.is_some() { format!("  {word}") } else { word.to_owned() };
    let mut right = vec![run(word, Style::new().fg(color))];
    right.extend(time.map(|time| run(format!(" {time}"), muted)));
    right.push(run(format!("  {model}"), muted));
    (mark, right)
  }
}

/// The row of an agent whose state stands at a column: its mark moves there, and under the pointer
/// its text goes bright while its state keeps the color that says it.
fn titled(row: Row, x: usize, right: &Line, mark: Option<Loader>) -> Row {
  let keeps = x..x + width(&right[..1]);
  Row { keeps: Some(keeps), loaders: mark.map(|one| (x, one)).into_iter().collect(), ..row }
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

/// The loader of what a line that runs waits for: a model that writes, a command, or an agent.
fn loader(work: Work) -> Loader {
  match work {
    Work::Writes => Loader::Model,
    Work::Command => Loader::Command,
    Work::Agent => Loader::Agent,
  }
}
