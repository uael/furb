//! The prose of a model, which is markdown, as the lines of a count of columns: a paragraph
//! wrapped between words, a list with its marks, a link in the accent that the terminal follows,
//! inline code on the raised ground, and a block of code as a block of code, with the colors of
//! python.

use std::mem;

use crate::cells::Cells;
use opentui::style::Color;
use opentui::style::{Span, Style};
use pulldown_cmark::{
  Alignment, BlockQuoteKind, CodeBlockKind, Event, Options, Parser, Tag, TagEnd,
};

use crate::{
  code::{Colors, Gutter},
  line::{self, Line, run},
  tokens::{APART, PROSE, Role, Tokens},
};

/// The lines of a text of markdown in a count of columns, each from the column of the prose.
pub(crate) fn lines(text: &str, room: usize, tokens: &Tokens, colors: &Colors) -> Vec<Line> {
  let mut prose = Prose {
    room,
    tokens,
    colors,
    out: Vec::new(),
    words: Vec::new(),
    styles: Vec::new(),
    link: None,
    lists: Vec::new(),
    code: None,
    table: None,
    quotes: Vec::new(),
  };
  // The markdown of GitHub: its tables, its struck words, its lists of tasks, its footnotes and the
  // alerts of its quotes.
  let gfm = Options::ENABLE_TABLES
    | Options::ENABLE_STRIKETHROUGH
    | Options::ENABLE_TASKLISTS
    | Options::ENABLE_FOOTNOTES
    | Options::ENABLE_GFM;
  for event in Parser::new_ext(text, gfm) {
    prose.event(event);
  }
  prose.flush();
  while prose.out.last().is_some_and(Vec::is_empty) {
    prose.out.pop();
  }
  prose.out
}

/// The lines made so far, the words of the line that comes, the styles that stand and the link that
/// stands, the count of each list that stands, none for a list with no numbers, the block of code
/// that comes, with its language, and the table that comes, with the side of each column and the
/// cells of each row, its head first.
struct Prose<'a> {
  room: usize,
  tokens: &'a Tokens,
  colors: &'a Colors,
  out: Vec<Line>,
  words: Vec<Span<'static>>,
  styles: Vec<Style>,
  link: Option<String>,
  lists: Vec<Option<usize>>,
  code: Option<(String, String)>,
  table: Option<(Vec<Alignment>, Vec<Vec<Line>>)>,
  /// The bar of each quote that stands, outer first, in its color.
  quotes: Vec<Color>,
}

impl Prose<'_> {
  fn event(&mut self, event: Event<'_>) {
    match event {
      Event::Start(tag) => self.start(tag),
      Event::End(tag) => self.end(tag),
      Event::Text(text) => match &mut self.code {
        Some((held, _)) => held.push_str(&text),
        None => self.text(&text, self.style()),
      },
      Event::Code(text) => self.text(&text, self.style().bg(self.tokens.raised)),
      // A task stands as its mark in the place of the mark of its item.
      Event::TaskListMarker(done) => {
        let (mark, color) =
          if done { ("✓ ", self.tokens.accent) } else { ("○ ", self.tokens.muted) };
        if self.words.last().is_some_and(|one| one.content == "• ") {
          self.words.pop();
        }
        self.words.push(run(mark, Style::new().fg(color)));
      }
      Event::FootnoteReference(label) => {
        self.words.push(run(format!("[{label}]"), Style::new().fg(self.tokens.accent)));
      }
      // A single newline breaks the line, as a comment of GitHub keeps it, so an answer that gives
      // the lines of a command keeps them.
      Event::SoftBreak | Event::HardBreak => self.flush(),
      Event::Rule => {
        self.flush();
        self.out.push(vec![run("─".repeat(self.room), self.tokens.style(Role::Muted))]);
        self.gap();
      }
      _ => {}
    }
  }

  fn start(&mut self, tag: Tag<'_>) {
    match tag {
      Tag::CodeBlock(kind) => {
        self.flush();
        let language = match kind {
          CodeBlockKind::Fenced(language) => language.into_string(),
          CodeBlockKind::Indented => "text".to_owned(),
        };
        self.code = Some((String::new(), language));
      }
      Tag::Item => {
        self.flush();
        // An item of a list inside a list stands under the text of the item that holds it.
        let depth = self.lists.len().saturating_sub(1);
        if depth > 0 {
          self.words.push(run(" ".repeat(2 * depth), Style::new()));
        }
        let mark = match self.lists.last_mut() {
          Some(Some(next)) => {
            *next += 1;
            format!("{}. ", *next - 1)
          }
          _ => "• ".to_owned(),
        };
        self.words.push(run(mark, self.tokens.style(Role::Muted)));
      }
      Tag::BlockQuote(kind) => {
        self.flush();
        let color = self.alerted(kind);
        self.quotes.push(color);
      }
      Tag::FootnoteDefinition(label) => {
        self.flush();
        self.words.push(run(format!("[{label}] "), self.tokens.style(Role::Muted)));
      }
      Tag::Strikethrough => self.styles.push(self.style().strikethrough()),
      Tag::List(first) => {
        self.flush();
        self.lists.push(first.map(|first| usize::try_from(first).unwrap_or_default()));
      }
      Tag::Table(sides) => {
        self.flush();
        self.table = Some((sides, Vec::new()));
      }
      Tag::TableHead | Tag::TableRow => {
        if let Some((_, rows)) = self.table.as_mut() {
          rows.push(Vec::new());
        }
      }
      // A title and a bold word stand in the bright text, over the softer body.
      Tag::Strong | Tag::Heading { .. } => {
        self.styles.push(self.style().fg(self.tokens.text).bold());
      }
      Tag::Emphasis => self.styles.push(self.style().italic()),
      Tag::Link { dest_url, .. } => {
        self.styles.push(self.style().fg(self.tokens.accent).underline());
        self.link = Some(dest_url.into_string());
      }
      _ => {}
    }
  }

  fn end(&mut self, tag: TagEnd) {
    match tag {
      TagEnd::CodeBlock => {
        if let Some((text, language)) = self.code.take() {
          self.block(text.trim_end_matches('\n'), &language);
        }
      }
      TagEnd::List(_) => {
        self.flush();
        self.lists.pop();
        if self.lists.is_empty() {
          self.gap();
        }
      }
      TagEnd::BlockQuote(_) => {
        self.flush();
        // A quote ends on its last line, and the prose goes on one blank row under it.
        while self.out.last().is_some_and(|line| self.bare(line)) {
          self.out.pop();
        }
        self.quotes.pop();
        self.gap();
      }
      TagEnd::FootnoteDefinition => {
        self.flush();
        self.gap();
      }
      TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
        self.styles.pop();
      }
      TagEnd::Link => {
        self.styles.pop();
        self.link = None;
      }
      TagEnd::Heading(_) => {
        self.styles.pop();
        self.flush();
        self.gap();
      }
      TagEnd::Paragraph => {
        self.flush();
        if self.lists.is_empty() {
          self.gap();
        }
      }
      TagEnd::Item => self.flush(),
      TagEnd::TableCell => {
        let words = mem::take(&mut self.words);
        if let Some(row) = self.table.as_mut().and_then(|(_, rows)| rows.last_mut()) {
          row.push(words);
        }
      }
      TagEnd::Table => {
        if let Some((sides, rows)) = self.table.take() {
          self.tabled(&sides, rows);
        }
      }
      _ => {}
    }
  }

  fn style(&self) -> Style {
    self.styles.last().copied().unwrap_or_else(|| Style::new().fg(self.tokens.prose))
  }

  /// A text added to the words, each word with its style and the link that stands. An address of
  /// the web in the text links to itself, as GitHub links it, less the stop that ends a sentence.
  fn text(&mut self, text: &str, style: Style) {
    for (at, word) in text.split(' ').enumerate() {
      let words = [(at > 0).then_some(" "), Some(word).filter(|word| !word.is_empty())];
      for word in words.into_iter().flatten() {
        let address = word.trim_end_matches(['.', ',', ';', ':', ')', '!', '?']);
        let bare = self.link.is_none()
          && (address.starts_with("https://") || address.starts_with("http://"));
        if bare && address.len() > "https://".len() {
          let linked = style.fg(self.tokens.accent).underline();
          self.words.push(run(address, linked).link(address.to_owned()));
          let rest = &word[address.len()..];
          if !rest.is_empty() {
            self.words.push(run(rest, style));
          }
          continue;
        }
        let span = run(word, style);
        self.words.push(match &self.link {
          Some(link) => span.link(link.clone()),
          None => span,
        });
      }
    }
  }

  /// The color of the bar of a quote, and the row of the title of an alert of GitHub with its mark,
  /// each alert in the color that tells what it says.
  fn alerted(&mut self, kind: Option<BlockQuoteKind>) -> Color {
    let t = self.tokens;
    let (mark, title, color) = match kind {
      None => return t.border,
      Some(BlockQuoteKind::Note) => ("ⓘ", "Note", t.attention),
      Some(BlockQuoteKind::Tip) => ("✦", "Tip", t.added),
      Some(BlockQuoteKind::Important) => ("❢", "Important", t.hues[0]),
      Some(BlockQuoteKind::Warning) => ("▲", "Warning", t.hues[2]),
      Some(BlockQuoteKind::Caution) => ("⊘", "Caution", t.danger),
    };
    let mut line = self.lead(color);
    line.push(run(format!("{mark} {title}"), Style::new().fg(color).bold()));
    self.out.push(line);
    color
  }

  /// The bars of the quotes that stand, and of one more in a color, before a line.
  fn lead(&self, color: Color) -> Line {
    let bars = self.quotes.iter().copied().chain([color]);
    bars.map(|one| run("│ ", Style::new().fg(one))).collect()
  }

  /// The bars of the quotes that stand, before each line inside them.
  fn bars(&self) -> Line {
    self.quotes.iter().map(|one| run("│ ", Style::new().fg(*one))).collect()
  }

  /// Whether a line holds only the bars of quotes.
  fn bare(&self, line: &Line) -> bool {
    !self.quotes.is_empty() && !line.is_empty() && line.iter().all(|one| one.content == "│ ")
  }

  /// The words wrapped into lines between words, under the mark of the item that they follow.
  fn flush(&mut self) {
    let indent = self.lists.len() * 2;
    let bars = self.bars();
    let room = self.room.min(PROSE).saturating_sub(2 * self.quotes.len());
    let mut line: Line = Vec::new();
    let mut used = 0;
    let words = mem::take(&mut self.words);
    for word in words {
      let space = word.content == " ";
      if space && used == 0 {
        continue;
      }
      if used + word.content.width() > room && used > 0 && !space {
        self.out.push([bars.clone(), line].concat());
        line = vec![run(" ".repeat(indent), Style::new())];
        used = indent;
      }
      used += word.content.width();
      line.push(word);
    }
    if used > 0 {
      self.out.push([bars, line].concat());
    }
  }

  fn gap(&mut self) {
    if self.out.last().is_some_and(|line| !line.is_empty() && !self.bare(line)) {
      let bars = self.bars();
      self.out.push(bars);
    }
  }

  /// A table: each column as wide as its widest cell, the widest ones cut while the table is wider
  /// than the prose, its columns apart as two things of one line, its head bold over a rule, and
  /// each cell on the side that its column names.
  fn tabled(&mut self, sides: &[Alignment], mut rows: Vec<Vec<Line>>) {
    let wide = |cell: &Line| cell.iter().map(|one| one.content.width()).sum::<usize>();
    let mut widths = vec![0; sides.len()];
    for row in &rows {
      for (width, cell) in widths.iter_mut().zip(row) {
        *width = (*width).max(wide(cell));
      }
    }
    let total =
      |widths: &[usize]| widths.iter().sum::<usize>() + APART * widths.len().saturating_sub(1);
    while total(&widths) > self.room
      && let Some(widest) = widths.iter_mut().filter(|one| **one > 1).max()
    {
      *widest -= 1;
    }
    let muted = self.tokens.style(Role::Muted);
    for (at, row) in rows.iter_mut().enumerate() {
      let mut line: Line = Vec::new();
      for (column, (width, side)) in widths.iter().zip(sides).enumerate() {
        let mut cell = row.get(column).cloned().unwrap_or_default();
        if wide(&cell) > *width {
          let text: String = cell.iter().map(|one| one.content.as_ref()).collect();
          let style = cell.first().map_or_else(|| self.style(), |one| one.style);
          cell = vec![run(line::cut(&text, *width), style)];
        }
        if at == 0 {
          cell = cell.into_iter().map(|one| run(one.content, one.style.bold())).collect();
        }
        let room = width - wide(&cell);
        let before = match side {
          Alignment::Right => room,
          Alignment::Center => room / 2,
          Alignment::Left | Alignment::None => 0,
        };
        let apart = if column > 0 { APART } else { 0 };
        line.push(run(" ".repeat(apart + before), Style::new()));
        line.extend(cell);
        line.push(run(" ".repeat(room - before), Style::new()));
      }
      self.out.push(line);
      if at == 0 {
        self.out.push(vec![run("─".repeat(total(&widths)), muted)]);
      }
    }
    self.gap();
  }

  /// A block of code as each block of code of the window: on the raised ground, as wide as the
  /// prose, its language as its head on the row above the box at its right, as the model of a block
  /// of the feed, and each line with its number close to the left of the box.
  fn block(&mut self, text: &str, language: &str) {
    let ground = self.tokens.raised;
    let (plain, muted) = (self.tokens.style(Role::Prose), self.tokens.style(Role::Muted));
    let lines = match language {
      "" | "py" => self.colors.lines(text, plain.bg(ground)).to_vec(),
      // A diff reads as each diff of the window: a line that it adds in cyan, one that it removes
      // in red.
      "diff" | "patch" => text
        .split('\n')
        .map(|one| {
          let color = match one.as_bytes().first() {
            Some(b'+') => self.tokens.added,
            Some(b'-') => self.tokens.danger,
            _ => self.tokens.prose,
          };
          vec![run(one, Style::new().fg(color))]
        })
        .collect(),
      language => self.colors.coded(text, language, plain.bg(ground)).to_vec(),
    };
    let numbers = Gutter::of(lines.len());
    let gutter = numbers.width;
    let head = vec![run(language, muted)];
    let head = (!language.is_empty())
      .then(|| line::spread(Vec::new(), head, self.room.saturating_sub(2), Style::new()));
    let room = Gutter::inside(self.room.saturating_sub(2)).saturating_sub(gutter + 2);
    let numbered = lines.into_iter().zip(1..).map(|(line, number)| {
      let mut line = line::clipped(line, room, muted);
      line.insert(0, run(format!("{number:>gutter$}  "), muted));
      line
    });
    self.out.extend(head);
    for line in numbered {
      self.out.push(line::boxed(line, self.room, ground));
    }
    self.gap();
  }
}
