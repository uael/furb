//! The rows of an open thread, from the exchanges that its daemon gave: the message of the operator
//! on its band, the block of code of the answer, which rolls in seven rows, the thought of its
//! model on one row, what the answer said as markdown, and the messages of the operator that the
//! thread does not show yet, each agent under the line of the block that made it, then the dock of
//! the agents that no line made.

use std::{cell::OnceCell, collections::HashMap, mem, path::Path};

use furb::view::{
  Agent, Block, Exchange, Line as Code, Made, Maker, Mark, Output, Retrying, Said, Saying,
  Standing, Thought, Work, homed, lines, operator, timer,
};
use furb::world::images;
use furb_client::{Anchor, Fold, Opened, PICTURE, Picture, Shown, Target, checkout, rests};
use furb_host::{
  FollowUp,
  protocol::{Card, Present},
};
use opentui::style::{Color, Span, Style};
use unicode_width::UnicodeWidthStr;

use crate::{
  code::Gutter,
  input::{Armed, named},
  keys::Does,
  line::{self, Line, capital, clipped, marked, run, sliced, spread, styled, wrapped},
  list::project,
  model::Model,
  names::{self, Named},
  picture, prose,
  question::{closed, person},
  review::shown_at,
  thread::{Boxed, Click, Item, Items, Row, width},
  tokens::{APART, Loader, PROMPT, PROSE, Role, WORKS, mixed},
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
  /// The numbers of the lines of the thread that bind each name, in their order, once a row asks.
  bindings: OnceCell<HashMap<String, Vec<usize>>>,
}

impl Model {
  /// The rows of the open thread.
  pub(crate) fn feed(&self) -> Option<Feed<'_>> {
    let opened = self.client.thread.as_ref()?;
    Some(Feed { model: self, opened, wide: self.wide(), bindings: OnceCell::new() })
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
        self.opened.thread.exchanges.get(&at).map(|one| self.exchange(at, one)).unwrap_or_default()
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
  /// rests: his message where it will stand, its bar and its text muted, with a chip in the hue of
  /// the thread that says that it waits for it, and the cross that takes it back to the input at
  /// its right. Once it goes, it stands there in its colors.
  fn queued(&self) -> Vec<Row> {
    let t = &self.model.tokens;
    let muted = self.style(Role::Muted).bg(t.raised);
    let chip = self.style(Role::Button).fg(self.model.hue(&self.opened.thread.key));
    // The cross ends where the time of a line of a block ends, two columns in from the band.
    let right = vec![run(" queued ", chip), run(" ✕", muted), run("  ", muted)];
    let chip = |one: &FollowUp| {
      let mut rows = self.band(&one.message, (t.border, muted), right.clone());
      if let Some(row) = rows.get_mut(1) {
        let end = width(&row.line) - 2;
        row.hot = vec![(end - 1..end, Some(Click::Unqueue(one.entry.clone())))];
      }
      rows
    };
    self.model.client.follow_ups().into_iter().flat_map(chip).collect()
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
      // which a model gave to the thread of an agent. A thread that his message started shows that
      // message already, as it waits for the daemon to show it.
      (None, None) if self.opened.thread.started => return Vec::new(),
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

  /// The head of the thread, which never scrolls: its project and its title, since the window is
  /// thread first and the card and Go to name the session, and the button that resolves it, once
  /// the daemon made the thread and gave its page. A thread that ended shows how, and while the
  /// input sends elsewhere its button turns the input to it, since a reply opens it again. A
  /// resolve of a thread that works waits for a confirm on the button, which says that it stops the
  /// work. The thread of an agent says that it is one, and the button of a thread that the operator
  /// opened from another, an agent or a thread of a chain, goes back to that one.
  pub(crate) fn head(&self) -> Row {
    let t = &self.model.tokens;
    let root = self.opened.root();
    let untold = self.model.untold();
    let card = self.model.client.card(&root.thread.key).or(untold.as_ref().map(|one| &one.1));
    let project = card.map(project).unwrap_or_default();
    let muted = self.style(Role::Muted);
    let title = self.model.title().unwrap_or_default();
    let (branch, branches) = self.branch();
    let sub = self.opened.sub().and_then(|one| one.anchor.clone()).or_else(|| self.made_sub());
    let mark = match (&self.opened.from, self.opened.agent(), branch) {
      (Some(_), _, Some(at)) => format!("⑂ {} · ", at + 2),
      (Some(_), _, None) if let Some(one) = &sub => format!("◆ {} · ", shown_at(one)),
      (Some(_), Some(_), None) => "agent · ".to_owned(),
      (None, ..) if branches > 0 => format!("⑂ 1 of {} · ", branches + 1),
      _ => String::new(),
    };
    // The head marks a session whose life runs as its card does, in the column of the bars of the
    // messages, so its project stands over their text.
    let runs = card.is_some_and(|card| self.model.live(&root.thread.key, card));
    let mut left = vec![
      run(if runs { "●" } else { " " }, Style::new().fg(t.live)),
      run(format!(" {project} · "), muted),
    ];
    let forks = (width(&left)..width(&left) + mark.width() - 3, branches > 0);
    left.push(run(mark, muted));
    let accent = self.style(Role::Title).fg(self.model.tokens.accent);
    left.extend(picture::marked(&title, self.style(Role::Title), accent));
    let (mark, button) = self.closer(card, branch);
    // The thread of an agent that the operator went back from stands ahead, beside the closer.
    let ahead = self.opened.ahead.as_ref().map(|_| run(" Forward ", self.style(Role::Button)));
    let parts = self.present().into_iter().chain(mark).chain(ahead.clone());
    let parts = parts.chain(button.clone().map(|one| one.0));
    let apart = |(at, one)| [(at > 0).then(|| run(" ".repeat(APART), Style::new())), Some(one)];
    let right: Line = parts.enumerate().flat_map(apart).flatten().collect();
    // A button ends at the edge of the boxes, and a mark with no button after it at the column of
    // the text, as each label at the right of the feed.
    let reach = self.wide.saturating_sub(if button.is_some() { 2 } else { 4 });
    let line = spread(left, right, reach, Style::new());
    let end = width(&line);
    let hot = button.map(|(one, does)| (end - one.content.width()..end, Some(Click::Key(does))));
    let before = hot.as_ref().map_or(end, |one| one.0.start - APART);
    let forward =
      ahead.map(|one| (before - one.content.width()..before, Some(Click::Key(Does::Forward))));
    // The mark of the branches of a thread opens its last branch.
    let last = self.opened.thread.branches().last().map(|one| one.card.prompt.clone());
    let fork = last.filter(|_| forks.1).map(|prompt| (forks.0, Some(Click::Agent(prompt))));
    Row { hot: hot.into_iter().chain(forward).chain(fork).collect(), ..Row::of(line) }
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
    // A sub thread resolves as a thread of the list does, and escape goes back from it.
    let sub = self.opened.sub().map(|one| &one.card);
    // A life that a fault ended takes no resolve: it only reads.
    let leaves = self.model.leaves() && sub.is_none();
    let made = !self.opened.thread.key.2.is_empty() && !leaves && !self.ended();
    let (mark, button) = match sub.or(card.map(|card| &card.thread)) {
      _ if branch.is_some() => (None, Some((run(" ⑂ 1 ", button), Does::Close))),
      // A sub thread that his message just made works, as its page will say.
      None if self.made_sub().is_some() => (None, Some((run(" Resolve ", button), Does::Resolve))),
      _ if self.opened.from.is_some() && sub.is_none() => {
        (None, Some((run(" Back ", button), Does::Close)))
      }
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

  /// The place of the line that the message that made this sub thread is about, while the page of
  /// the sub thread does not name it yet: the window knows it from the message that it sent.
  fn made_sub(&self) -> Option<Anchor> {
    let thread = &self.opened.thread;
    let first = thread.pending.first().filter(|_| thread.started && self.opened.sub().is_none())?;
    Anchor::of(&first.text).map(|(anchor, _)| anchor)
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
  fn exchange(&self, at: usize, exchange: &Exchange) -> Vec<Row> {
    let his = matches!(exchange.by, Maker::Operator | Maker::Member { .. });
    let mut rows = Vec::new();
    match &exchange.block {
      Some(block) if exchange.python => rows.extend(self.word(&exchange.message, block)),
      block => {
        rows.extend(self.message(&exchange.text, his));
        if let Some(block) = block {
          // The head of the block takes the blank row under the message.
          rows.pop();
          // The model heads a block where it differs from the model of the block before, as a
          // chat names a sender once for his messages in a row.
          let before = self.opened.thread.exchanges.range(..at).rev();
          let before = before.filter(|one| !one.1.python).find_map(|one| one.1.block.as_ref());
          let named = before.is_none_or(|one| one.maker != block.maker);
          rows.extend(self.block(&exchange.message, block, named));
        }
      }
    }
    // The last answer that wrote a file that furb knew before offers to revert it, a blank row
    // under its block or the band of his word, before the blank row that ends the block, to a
    // window that may write.
    let reads = self.model.client.only_reads(self.opened.thread.key.0).is_some();
    if let Some((message, writes)) = self.model.reverts()
      && message == exchange.message
      && !reads
    {
      let mut files: Vec<&str> =
        writes.iter().filter(|one| one.known).map(|one| &*one.path).collect();
      files.sort_unstable();
      files.dedup();
      let said = match files.as_slice() {
        [one] => format!("↶ Revert {}", named(one)),
        many => format!("↶ Revert {} files", many.len()),
      };
      let muted = self.style(Role::Muted);
      let line = vec![run("  ", Style::new()), run(said, muted), run("  ", muted)];
      let key = run(self.model.key(Does::Revert), muted);
      let row = Row::clicked([line, vec![key]].concat(), Click::Key(Does::Revert));
      let end = rows.len().saturating_sub(1);
      rows.splice(end..end, [Row::default(), row]);
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
    let hue = if his { self.model.hue(&self.opened.thread.key) } else { t.muted };
    // A message about a line of a file says what it says of it, and names the line at its right.
    let Some((anchor, said)) = Anchor::of(text) else {
      return self.band(text, (hue, self.style(Role::Prose)), Vec::new());
    };
    let line = shown_at(&anchor);
    // The line ends two columns in from the band, as the cross of a queued message does.
    let muted = self.style(Role::Muted).bg(t.raised);
    let right =
      vec![run("◆ ", Style::new().fg(t.accent).bg(t.raised)), run(line, muted), run("  ", muted)];
    self.band(said, (hue, self.style(Role::Prose)), right)
  }

  /// A text on the raised band of a message, after the bar in a hue, in a style, with a line at
  /// the right of its first row, then the box of each image that it names, and a blank row under
  /// it.
  fn band(&self, text: &str, (hue, style): (Color, Style), right: Line) -> Vec<Row> {
    let t = &self.model.tokens;
    let pictures = images::references(text);
    let room = self.wide.saturating_sub(6 + width(&right) + 2 * usize::from(!right.is_empty()));
    let room = room.min(PROSE);
    let lines = picture::shown(&self.model.homed(text))
      .lines()
      .flat_map(|line| wrapped(line, room))
      .collect::<Vec<_>>();
    // An image that the message names, and a file that it mentions, stand as marks, as the input
    // shows them.
    let (plain, mark) = (style.bg(t.raised), Style::new().fg(t.accent).bg(t.lifted));
    let mut right = Some(right);
    let text = lines.into_iter().map(|line| {
      self.banded(hue, picture::marked(&line, plain, mark), right.take().unwrap_or_default())
    });
    [self.banded(hue, Vec::new(), Vec::new())]
      .into_iter()
      .chain(text)
      .chain(pictures.iter().flat_map(|one| {
        let row = |line: Line| self.banded(hue, line, Vec::new());
        [row(Vec::new())].into_iter().chain(self.picture(
          one,
          (2, self.wide.saturating_sub(6)),
          row,
        ))
      }))
      .chain([self.banded(hue, Vec::new(), Vec::new()), Row::default()])
      .collect()
  }

  /// A row of the band of a message: the bar in a hue, then a text, and a line at its right.
  fn banded(&self, hue: Color, text: Line, right: Line) -> Row {
    let band = Style::new().bg(self.model.tokens.raised);
    let mut line =
      vec![run("┃", Style::new().fg(hue).bg(self.model.tokens.raised)), run(" ", band)];
    line.extend(spread(text, right, self.wide.saturating_sub(4), band));
    Row::of(line)
  }

  /// The box of an image that a text names, whose rows a row of a line makes, from a column and in
  /// a room of columns, which the image fills once its machine gave it. Until then its first row
  /// names it, and an image that does not draw takes one row, which says why.
  fn picture(
    &self,
    named: &images::Named,
    (x, room): (u16, usize),
    row: impl Fn(Line) -> Row,
  ) -> Vec<Row> {
    let (at, life, _) = &self.opened.thread.key;
    let held = self.model.client.pictures.get(&(*at, life.clone(), named.uri.clone()));
    let (wide, high) = picture::cells(held, room);
    let mut rows: Vec<Row> = (0..high).map(|_| row(Vec::new())).collect();
    // A message and a block stand on the same raised ground.
    let muted = self.style(Role::Muted).bg(self.model.tokens.raised);
    let said =
      |text: String| wrapped(&text, room).into_iter().map(|line| row(vec![run(line, muted)]));
    match held {
      Some(Picture::Held(_)) => {
        let source = named.uri.clone();
        rows[0].picture = Some(Boxed { source, x, y: 0, wide, high });
      }
      Some(Picture::Refused(why)) => {
        rows.extend(said(format!("{PICTURE}{} does not show: {why}", named.name)));
      }
      _ => {
        rows.splice(0..1, said(format!("{PICTURE}{}", named.name)).take(1));
      }
    }
    rows
  }

  /// Each image that a line of a block gave, under it at the column of its code: its box in a
  /// block that shows each of its lines, and else a row that names it, as a file that the line
  /// wrote stands.
  fn pictures_of(&self, line: &Code, numbers: Gutter, open: bool) -> Vec<Row> {
    let gutter = numbers.width;
    let room = Gutter::inside(self.inner()).saturating_sub(gutter + 2);
    let muted = self.style(Role::Muted);
    let row = |part: Line| {
      let lead = run(format!("{:gutter$}  ", ""), muted);
      Row::of(self.boxed([vec![lead], part].concat(), self.model.tokens.raised))
    };
    let named = line.pictures.iter().flat_map(|text| images::references(text));
    let x = u16::try_from(PAD + gutter + 2).unwrap_or(u16::MAX);
    named
      .flat_map(|one| {
        if open {
          self.picture(&one, (x, room), row)
        } else {
          vec![row(vec![run(format!("{PICTURE}{}", one.name), muted)])]
        }
      })
      .collect()
  }

  /// The images that the open thread shows, by its machine and its life: those that its messages
  /// name, and those that the lines of each block that shows each of its lines gave.
  pub(crate) fn pictures(&self) -> Vec<Shown> {
    let (at, life, _) = &self.opened.thread.key;
    let thread = &self.opened.thread;
    let pending = thread.pending.iter().map(|one| one.text.as_str());
    let open = |one: &Exchange| self.opened.unfolded.contains(&Fold::Block(one.message.clone()));
    let lines = thread
      .exchanges
      .values()
      .filter_map(|one| one.block.as_ref().filter(|block| open(one) || !block.rolls()));
    let gave = lines.flat_map(|block| block.lines.iter().flat_map(|line| &line.pictures));
    let texts = thread.exchanges.values().map(|one| one.text.as_str()).chain(pending);
    let texts = texts.chain(gave.map(String::as_str));
    texts.flat_map(images::references).map(|one| (*at, life.clone(), one.uri)).collect()
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

  /// The block of an answer, as a block of code of the markdown: the model as its head where it is
  /// named, on the row above the box at its right, where the times of its lines end, then the
  /// window of seven rows that rolls, or each line once the operator opened it, each with its
  /// number in the thread, close to the left of the box. The thought stands under it.
  fn block(&self, message: &str, block: &Block, named: bool) -> Vec<Row> {
    let maker =
      if named { vec![run(block.maker.shown(), self.style(Role::Muted))] } else { Vec::new() };
    let mut head = spread(Vec::new(), maker.clone(), Gutter::inside(self.inner()), Style::new());
    head.insert(0, run(" ".repeat(PAD), Style::new()));
    let numbers = Gutter::of(block.lines.iter().map(|one| one.number).max().unwrap_or_default());
    let code = PAD + numbers.width + 2;
    let mut rows = vec![Row { inspector: Some((code, maker)), ..Row::of(head) }];
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
    let naming = self.model.names.of(&text);
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
    // A stop ends the answer, so each line under the line that it ended never ran: its code reads
    // muted, as a debugger shows the code that it did not reach.
    let stopped = block.lines.iter().rposition(|one| matches!(one.mark, Mark::Stopped { .. }));
    let drawn = block.lines.iter().zip(colored.iter()).enumerate();
    for (at, (line, colors)) in drawn.take(end).skip(shown.start) {
      let colors = match stopped {
        Some(stop) if at > stop => {
          colors.iter().map(|one| run(one.content.clone(), muted.bg(t.raised))).collect()
        }
        _ => colors.clone(),
      };
      let names = naming.get(at).map_or(&[][..], Vec::as_slice);
      rows.extend(self.code(line, (colors, names), numbers));
      rows.extend(self.writes(line, numbers, (&fold, open)));
      rows.extend(self.pictures_of(line, numbers, open || !block.rolls()));
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
  fn code(&self, line: &Code, (colors, names): (Line, &[Named]), numbers: Gutter) -> Vec<Row> {
    let t = &self.model.tokens;
    let (gutter, inner) = (numbers.width, Gutter::inside(self.inner()));
    let mark = self.mark(line);
    let (number, color, loader) = self.number(line, mark);
    let room = inner.saturating_sub(gutter + 2);
    let ground = if loader.is_some() { t.lifted } else { t.raised };
    // What the command of a line at work writes stands in the band of the line, as one work.
    let output = line.output.as_ref().map(|one| self.output(one, (gutter, room), ground));
    let (mut under, look) = match mark {
      Mark::Failed { why } => (wrapped(why, room), Style::new().fg(t.danger)),
      Mark::Retries(retry) => (wrapped(&retry.why, room), self.style(Role::Muted)),
      _ => (Vec::new(), Style::new()),
    };
    let colors = match mark {
      Mark::Failed { .. } if line.text.is_empty() && !under.is_empty() => {
        vec![run(under.remove(0), look)]
      }
      _ => self.written(line, mark).unwrap_or(colors),
    };
    let muted = self.style(Role::Muted);
    let timer = self.right(line, mark).map(|right| vec![run(right, muted)]).unwrap_or_default();
    let room = room.saturating_sub(width(&timer) + 2 * usize::from(!timer.is_empty()));
    let head = run(format!("{number:>gutter$}  "), Style::new().fg(color));
    let row = |lead: Span<'static>, part: Line, right: Line| {
      self.boxed(spread([vec![lead], part].concat(), right, inner, Style::new()), ground)
    };
    let pointed = self.gave(line, matches!(mark, Mark::Ran)).map(|gave| {
      row(head.clone(), clipped([colors.clone(), gave].concat(), room, muted), timer.clone())
    });
    // A line cut to its row says how many characters it cut, and a click there shows it whole: its
    // code wraps on the rows under its first, and a click on any of them cuts it again, while its
    // code keeps the colors of its grammar under the pointer and its number lights.
    let fold = Fold::Line(line.number);
    let cut = width(&colors) > room;
    let whole = cut && self.opened.unfolded.contains(&fold);
    let parts = if whole { sliced(&colors, room) } else { vec![clipped(colors, room, muted)] };
    let loaders = self.loaders(line, parts.first(), gutter);
    let pad = run(" ".repeat(gutter + 2), Style::new());
    let mut rows: Vec<Row> = (parts.into_iter().enumerate())
      .map(|(at, part)| {
        let (lead, right) = if at == 0 { (&head, timer.clone()) } else { (&pad, Vec::new()) };
        let shown = row(lead.clone(), part, right);
        let click = Click::Fold(fold.clone());
        if whole {
          Row { keeps: Some(PAD + gutter + 2..usize::MAX), ..Row::clicked(shown, click) }
        } else {
          let hot = cut.then(|| marked(&shown)).flatten().map(|at| (at, Some(click)));
          Row { hot: hot.into_iter().collect(), ..Row::of(shown) }
        }
      })
      .collect();
    if let Some(first) = rows.first_mut() {
      (first.loaders, first.pointed) = (loaders, pointed.filter(|_| !whole));
      (first.awaits, first.code) = (line.awaits.clone(), Some(line.number));
      if !whole {
        self.name(first, (line, names), gutter);
      }
    }
    rows.extend(output.into_iter().flatten());
    let under = under
      .iter()
      .map(|one| self.boxed(vec![run(format!("{:gutter$}  {one}", ""), look)], t.raised));
    rows.extend(under.map(Row::of));
    rows
  }

  /// The names of the code of the first row of a line, up to where the row cuts it, each with what
  /// a click on it does: a call of `prompt` that made an agent opens the thread of that agent, as a
  /// change to the chain that runs it, and a name that an earlier line of the thread binds goes to
  /// that line. The terminal keeps the command key for itself, so the click needs none.
  fn name(&self, row: &mut Row, (line, names): (&Code, &[Named]), gutter: usize) {
    let code = PAD + gutter + 2;
    let end = marked(&row.line).map_or(width(&row.line), |one| one.start);
    let agents: Vec<&Agent> =
      self.opened.thread.agents.iter().filter(|one| one.at == Some(line.number)).collect();
    let mut prompts = 0;
    for named in names.iter().filter(|one| code + one.at.end <= end) {
      let at = code + named.at.start..code + named.at.end;
      row.names.push((at.clone(), named.name.clone()));
      let click = if named.name == PROMPT {
        prompts += 1;
        let agent = agents.get(prompts - 1).or(agents.last());
        agent.map(|one| Click::Agent(one.card.prompt.clone()))
      } else if named.binds {
        None
      } else {
        self.bound(&named.name, line.number).map(|one| Click::Line(one, named.name.clone()))
      };
      // The name that the operator went to stands in the accent where its line binds it, and the
      // runs of a name that takes a click split at its edges, so the pointer lights it alone.
      let went = self
        .opened
        .went
        .as_ref()
        .is_some_and(|(at, name)| named.binds && *at == line.number && *name == named.name);
      if went {
        let accent = self.model.tokens.accent;
        row.line = styled(mem::take(&mut row.line), &at, |one| one.fg(accent).bold());
      }
      if let Some(click) = click {
        row.line = styled(mem::take(&mut row.line), &at, |one| one);
        row.hot.insert(0, (at, Some(click)));
      }
    }
  }

  /// The last line of the thread before a line that binds a name.
  fn bound(&self, name: &str, before: usize) -> Option<usize> {
    let bindings = self.bindings.get_or_init(|| {
      let mut bindings: HashMap<String, Vec<usize>> = HashMap::new();
      let blocks = self.opened.thread.exchanges.values().filter_map(|one| one.block.as_ref());
      for block in blocks {
        let text = block.lines.iter().map(|one| one.text.as_str()).collect::<Vec<_>>().join("\n");
        let named = self.model.names.of(&self.model.homed(&text));
        for (line, names) in block.lines.iter().zip(named.iter()) {
          for one in names.iter().filter(|one| one.binds) {
            bindings.entry(one.name.clone()).or_default().push(line.number);
          }
        }
      }
      bindings
    });
    bindings.get(name)?.iter().copied().filter(|one| *one < before).max()
  }

  /// The head of a block while the pointer is on a name of its code, from the column of the code:
  /// what the contract says of a verb of the engine, or the type and the value of the name in the
  /// module of the chain of the thread, as its machine reads it, beside what the head says at its
  /// right.
  pub(crate) fn inspector(&self, name: &str, (code, right): (usize, Line)) -> Line {
    let (prose, muted) = (self.style(Role::Prose), self.style(Role::Muted));
    let inspected = self.model.client.inspected.as_ref();
    let told = inspected.filter(|one| one.name == name && one.key == self.opened.thread.key);
    let said = match (names::verb(name), told.and_then(|one| one.told.as_ref())) {
      (Some((call, said)), _) => vec![run(call, prose), run(format!("  {said}"), muted)],
      (None, Some(Ok((kind, repr)))) => {
        let repr = self.model.homed(repr).replace('\n', "\\n");
        vec![run(name.to_owned(), prose), run(format!(": {kind} = "), muted), run(repr, prose)]
      }
      (None, Some(Err(why))) => vec![run(name.to_owned(), prose), run(format!("  {why}"), muted)],
      (None, None) => vec![run(name.to_owned(), prose), run(" …", muted)],
    };
    let room = Gutter::inside(self.inner()).saturating_sub(code - PAD);
    let fits = room.saturating_sub(width(&right) + 2);
    let mut line = spread(clipped(said, fits, muted), right, room, Style::new());
    line.insert(0, run(" ".repeat(code), Style::new()));
    line
  }

  /// What a line shows in place of its code: the attempt that comes while its model is asked
  /// again, and a word while its model has not begun it, so the lit row is never empty, or once a
  /// stop came before it began one.
  fn written(&self, line: &Code, mark: &Mark) -> Option<Line> {
    let said = |word| vec![run(word, self.style(Role::Muted).italic())];
    match mark {
      Mark::Retries(retry) => Some(self.retry(retry)),
      _ if !line.text.is_empty() => None,
      Mark::Runs { work: Work::Writes, .. } => Some(said(WRITES)),
      Mark::Stopped { .. } => Some(said("wrote nothing")),
      _ => None,
    }
  }

  /// The loaders of a line at work, at the columns of the row that shows it first. The light
  /// passes over the word that says that the model writes, as over each word of work, and over the
  /// call of `prompt` that the line awaits while its agent works: the one call that makes code and
  /// value in furb.
  fn loaders(&self, line: &Code, shown: Option<&Line>, gutter: usize) -> Vec<(usize, Loader)> {
    let mark = self.mark(line);
    let Some(loader) = self.number(line, mark).2 else { return Vec::new() };
    let code = PAD + gutter + 2;
    let shown: String = shown.into_iter().flatten().map(|one| one.content.as_ref()).collect();
    let light = match mark {
      Mark::Runs { work: Work::Writes, .. } if line.text.is_empty() => {
        Some((code, Loader::Light(WRITES)))
      }
      Mark::Runs { work: Work::Agent, .. } => shown
        .find(&format!("{PROMPT}("))
        .map(|at| (code + shown[..at].width(), Loader::Light(PROMPT))),
      _ => None,
    };
    let wide = if let Loader::Number(number) = loader { number.to_string().len() } else { 1 };
    [(PAD + gutter - wide, loader)].into_iter().chain(light).collect()
  }

  /// How a line stands: a line that the record left at work in a life that a fault ended works no
  /// more.
  fn mark<'a>(&self, line: &'a Code) -> &'a Mark {
    match &line.mark {
      Mark::Runs { .. } | Mark::Retries(_) if self.ended() => &Mark::Ran,
      mark => mark,
    }
  }

  /// What stands in the gutter of a line, in its color, and the loader of the line at work.
  fn number(&self, line: &Code, mark: &Mark) -> (String, Color, Option<Loader>) {
    let t = &self.model.tokens;
    match mark {
      // The line that its model writes keeps its number, which the light passes over in the beat
      // of the word that says the model thinks or writes.
      Mark::Runs { work: Work::Writes, .. } => {
        (line.number.to_string(), t.accent, Some(Loader::Number(line.number)))
      }
      Mark::Runs { work, .. } => (" ".to_owned(), t.accent, Some(loader(*work))),
      Mark::Retries(_) => (" ".to_owned(), t.accent, Some(Loader::Model)),
      Mark::Asks => ("◆".to_owned(), t.attention, None),
      Mark::Failed { .. } => ("✗".to_owned(), t.danger, None),
      Mark::Stopped { .. } => ("■".to_owned(), t.muted, None),
      Mark::Waits => (line.number.to_string(), t.muted, None),
      // A line that ended while he read the thread lights its number in the accent of its work,
      // which settles into the grey of a line that ran, so the eye sees where the work moved on.
      Mark::Ran => {
        let (ended, settles) = (self.opened.thread.ended.get(&line.number), self.model.settles);
        let age = ended.map(|at| self.model.client.moved.saturating_sub(*at));
        let share =
          age.filter(|age| *age < settles).map(|age| age.as_millis() * 100 / settles.as_millis());
        let color = share
          .map_or(t.muted, |share| mixed(t.accent, t.muted, u16::try_from(share).unwrap_or(100)));
        (line.number.to_string(), color, None)
      }
    }
  }

  /// What stands at the right of a line: the time of its work, after how many of its acts ended
  /// while it awaits more than one, or how long ago a stop ended it, from one minute on, as the age
  /// of a card, or the time that its command took.
  fn right(&self, line: &Code, mark: &Mark) -> Option<String> {
    match *mark {
      Mark::Runs { began, .. } => {
        let ended = line.made.iter().filter(|one| !matches!(one, Made::Works)).count();
        let count = (line.made.len() > 1).then(|| format!("{ended} of {} ended", line.made.len()));
        let time = began.and_then(|began| self.model.since(began)).map(timer);
        count.into_iter().chain(time).reduce(|count, time| format!("{count} · {time}"))
      }
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
  /// with the home folder written `~`, as the code above them writes it, on the ground of its line.
  fn output(&self, output: &Output, (gutter, room): (usize, usize), ground: Color) -> Vec<Row> {
    let muted = self.style(Role::Muted);
    let more = (output.more > 0).then(|| format!("… {}", lines(output.more)));
    let lines = more.into_iter().chain(output.tail.iter().map(|line| self.model.homed(line)));
    let row = |text: String| {
      let mut shown = vec![run(format!("{:gutter$}  ", ""), muted)];
      shown.extend(clipped(vec![run(text, muted)], room, muted));
      Row::of(self.boxed(shown, ground))
    };
    lines.map(row).collect()
  }

  /// What each write of a file of a line changed, under it at the column of the code: a row for
  /// each file, with its path from the folder of the thread and the lines that it added and
  /// removed, which a click opens or folds with the block; and while the block stands open, the
  /// diff of the last file, each line in the color of what it does on the ground of its change,
  /// and how many lines follow. A write of a file that the life did not know says how many lines it
  /// wrote. A block that rolls keeps a row for each file, so a diff never takes its rows.
  fn writes(&self, line: &Code, numbers: Gutter, (fold, open): (&Fold, bool)) -> Vec<Row> {
    let (gutter, inner) = (numbers.width, Gutter::inside(self.inner()));
    // A file stands under the line as each other row under a line, after its lead, and its diff
    // under the file, in the column of its path.
    let room = inner.saturating_sub(gutter + 4);
    let t = &self.model.tokens;
    let muted = self.style(Role::Muted);
    let pad = |lead: &str| run(format!("{:gutter$}  {lead}", ""), muted);
    let key = &self.opened.thread.key;
    // The World writes a path under the home of the user of its machine from `~`, so the folder
    // takes that home too, which may not be the home of the window.
    let home = self.model.client.machines.get(key.0).and_then(|one| one.home.as_deref());
    let checkout = checkout(&key.1).map(|one| one.display().to_string());
    let folder = checkout.map(|one| home.map_or_else(|| one.clone(), |home| homed(&one, home)));
    let mut rows = Vec::new();
    for one in &line.writes {
      let path = folder
        .as_deref()
        .and_then(|folder| one.path.strip_prefix(folder)?.strip_prefix(['/', '\\']))
        .map_or_else(|| self.model.homed(&one.path), str::to_owned);
      let counts = if one.known {
        self.model.counts(one.added, one.removed)
      } else {
        vec![run(format!("wrote {}", lines(one.added as usize)), muted)]
      };
      let mut shown = vec![run(format!("{path}  "), muted)];
      shown.extend(counts);
      let shown = [vec![pad("└ ")], clipped(shown, room, muted)].concat();
      rows.push(Row::clicked(self.boxed(shown, t.raised), Click::Fold(fold.clone())));
    }
    let Some(last) = line.writes.last().filter(|_| open) else { return rows };
    // The code of a file of python takes the colors of its grammar, read on the whole of what
    // the diff shows, as the code of a block.
    let python =
      Path::new(&last.path).extension().is_some_and(|one| one.eq_ignore_ascii_case("py"));
    let code: Vec<&str> = last
      .diff
      .iter()
      .filter(|one| !one.starts_with("@@"))
      .map(|one| one.get(1..).unwrap_or_default())
      .collect();
    let prose = self.style(Role::Prose);
    let colored = python.then(|| self.model.colors.lines(&code.join("\n"), prose));
    let mut at = 0;
    for text in &last.diff {
      let (sign, ground) = match text.as_bytes().first() {
        Some(b'+') => (Style::new().fg(t.added), t.added_ground),
        Some(b'-') => (Style::new().fg(t.danger), t.removed_ground),
        _ => (muted, t.raised),
      };
      // The header of a hunk says that lines stand between it and the one before.
      let shown = if text.starts_with("@@") {
        vec![pad("  "), run("…", muted)]
      } else {
        let (mark, plain) = text.split_at(text.chars().next().map_or(0, char::len_utf8));
        let runs = colored.as_ref().and_then(|all| all.get(at)).cloned();
        let runs = runs.unwrap_or_else(|| vec![run(plain.to_owned(), prose)]);
        at += 1;
        // A blank parts the mark from the code, as the review of the file shows it.
        [vec![pad("  "), run(format!("{mark} "), sign)], clipped(runs, room - 2, muted)].concat()
      };
      rows.push(Row::of(self.boxed(shown, ground)));
    }
    if last.more > 0 {
      let more = vec![pad("  "), run(format!("… {} in the review", lines(last.more)), muted)];
      rows.push(Row::clicked(self.boxed(more, t.raised), Click::Key(Does::Changes)));
    }
    rows
  }

  /// What a line whose model is asked again says, in the accent: furb works.
  fn retry(&self, retry: &Retrying) -> Line {
    vec![run(retry.said(self.model.second()), Style::new().fg(self.model.tokens.accent))]
  }

  /// The thought of a model as one muted row through its whole life: its last words while it
  /// streams, then its first sentence and its time. A click opens it whole, or folds it.
  fn thought(&self, message: &str, thought: &Thought) -> Vec<Row> {
    let (muted, style) = (self.style(Role::Muted), self.style(Role::Muted).italic());
    let fold = Fold::Thought(message.to_owned());
    let room = self.wide.saturating_sub(4);
    // The word says what the row is, and the time stands at the right, in the column of the time of
    // a line that runs: it counts while the model thinks, and stays as the time that it took.
    let (word, seconds) = match thought.took {
      None => (THINKS, self.model.since(thought.began)),
      Some(took) => ("thought", Some(took).filter(|one| *one > 0)),
    };
    let head = format!("  {word} · ");
    let time = seconds.map(timer);
    // The text stands in one column after the word, and its first row leaves room for the time.
    let (room, gap) = (room + 2 - head.width(), " ".repeat(head.width()));
    let first = room.saturating_sub(time.as_ref().map_or(0, |time| time.width() + 2));
    let mut lines = if self.opened.unfolded.contains(&fold) {
      // Open, the thought goes on from the row that said its start, so the click only adds rows.
      let whole = thought.text.split_whitespace().collect::<Vec<_>>().join(" ");
      let start = wrapped(&whole, first).swap_remove(0);
      let rest = whole[start.len()..].trim_start();
      let rest = if rest.is_empty() { Vec::new() } else { wrapped(rest, room) };
      [vec![start], rest].concat()
    } else if thought.took.is_none() {
      // The time of a thought that runs grows, so its row keeps the room of a time of eight
      // columns.
      vec![tail(&thought.text, room.saturating_sub(10))]
    } else {
      vec![thought.first().to_owned()]
    };
    let first = lines.remove(0);
    let said = if thought.took.is_none() {
      // While the model thinks, its last words stand brighter toward the end, where it writes, as
      // the tail of the light that passes over the word before them.
      let t = &self.model.tokens;
      let chars: Vec<char> = first.chars().collect();
      let from = chars.len().saturating_sub(PEN);
      let ink = chars[from..].iter().enumerate().map(|(at, one)| {
        let share = u16::try_from((at + 1) * 100 / (PEN + 1)).unwrap_or(100);
        run(one.to_string(), style.fg(mixed(t.muted, t.text, share)))
      });
      let said = [run(head.clone(), muted), run(chars[..from].iter().collect::<String>(), style)];
      said.into_iter().chain(ink).collect()
    } else {
      vec![run(head.clone(), muted), run(first, style)]
    };
    let time = time.map(|time| run(time, muted));
    let line = spread(said, time.into_iter().collect(), head.width() + room, Style::new());
    // While the model thinks, the light passes over the word that says so.
    let light = thought.took.is_none().then_some((2, Loader::Light(THINKS)));
    let head =
      Row { loaders: light.into_iter().collect(), ..Row::clicked(line, Click::Fold(fold.clone())) };
    let rest = lines.iter().map(|line| {
      Row::clicked(vec![run(format!("{gap}{line}"), style)], Click::Fold(fold.clone()))
    });
    [head].into_iter().chain(rest).collect()
  }

  /// What an answer said, as markdown from the column of the text. A question is one line with its
  /// mark: its first line while it waits, then what closed it. While its form shows it whole, the
  /// form is its one place, and the feed shows none of it. A resolve by a person is one quiet row
  /// that says who resolved it, as a question says who closed it.
  fn said(&self, said: &Said) -> Vec<Row> {
    let t = &self.model.tokens;
    let login = &self.model.client.machines[self.opened.thread.key.0].login;
    if let Saying::Resolved { by } = &said.kind {
      let muted = self.style(Role::Muted);
      let left = vec![run("  ✓ Resolved", muted)];
      let right = vec![run(person(by, login), muted)];
      return vec![
        Row::of(spread(left, right, self.wide.saturating_sub(4), Style::new())),
        Row::default(),
      ];
    }
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
      let right =
        answered.iter().map(|one| run(closed(one, login), self.style(Role::Muted))).collect();
      return vec![
        Row::of(spread(left, right, self.wide.saturating_sub(4), Style::new())),
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
      // Sub threads that rest were resolved, as a thread of the list is.
      let mut rested = self.opened.thread.agents.iter().filter(|one| one.at.is_none()).take(done);
      let subs = under.is_none() && rested.all(|one| one.anchor.is_some());
      let noun = match (subs, done) {
        (true, 1) => "sub thread",
        (true, _) => "sub threads",
        (false, 1) => "agent",
        (false, _) => "agents",
      };
      let ended = if subs { "resolved" } else { "done" };
      let (muted, ground) = (self.style(Role::Muted), self.model.tokens.raised);
      rows.insert(
        0,
        match under {
          Some((_, numbers)) => {
            let said = format!("{:>width$}  └ {done} {noun} {ended}", "", width = numbers.width);
            Row::of(self.boxed(vec![run(said, muted)], ground))
          }
          None => says(&format!("└ {done} {noun} {ended}"), muted),
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
    let mut title = vec![run(agent.card.title.clone(), title)];
    // A sub thread names the place that it is about before its title.
    if let Some(one) = &agent.anchor {
      let line = format!("{}  ", shown_at(one));
      title.splice(0..0, [run("◆ ", Style::new().fg(self.model.tokens.accent)), run(line, muted)]);
    }
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
    let room = Gutter::inside(self.inner());
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

/// The row of an agent whose state stands at a column: its mark moves there, and the light passes
/// over the word after it while it works, as over each word of work; under the pointer its text
/// goes bright while its state keeps the color that says it.
fn titled(row: Row, x: usize, right: &Line, mark: Option<Loader>) -> Row {
  let keeps = x..x + width(&right[..1]);
  let loaders = mark.map(|one| [(x, one), (x + 2, Loader::Light(WORKS))]).into_iter().flatten();
  Row { keeps: Some(keeps), loaders: loaders.collect(), ..row }
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

/// How many letters at the end of a thought that runs stand brighter, where its model writes.
const PEN: usize = 6;

/// The loader of what a line that runs waits for: a model that writes, a command, or an agent.
fn loader(work: Work) -> Loader {
  match work {
    Work::Writes => Loader::Model,
    Work::Command => Loader::Command,
    Work::Agent => Loader::Agent,
  }
}
