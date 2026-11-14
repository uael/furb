//! The prose of a model, which is markdown, as the lines of a count of columns: a paragraph
//! wrapped between words, a list with its marks, a link in the accent that the terminal follows,
//! inline code on the raised ground, and a block of code as a block of code, with the colors of
//! python.

use opentui::style::{Span, Style};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use unicode_width::UnicodeWidthStr;

use crate::{
  code::Colors,
  line::{Line, run},
  tokens::{Role, Tokens},
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
  };
  for event in Parser::new(text) {
    prose.event(event);
  }
  prose.flush();
  while prose.out.last().is_some_and(Vec::is_empty) {
    prose.out.pop();
  }
  prose.out
}

/// The lines made so far, the words of the line that comes, the styles that stand and the link that
/// stands, the count of each list that stands, none for a list with no numbers, and the block of
/// code that comes, with whether it is python.
struct Prose<'a> {
  room: usize,
  tokens: &'a Tokens,
  colors: &'a Colors,
  out: Vec<Line>,
  words: Vec<Span<'static>>,
  styles: Vec<Style>,
  link: Option<String>,
  lists: Vec<Option<usize>>,
  code: Option<(String, bool)>,
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
      Event::SoftBreak => self.text(" ", self.style()),
      Event::HardBreak => self.flush(),
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
        let python = match &kind {
          CodeBlockKind::Fenced(lang) => ["", "python", "py"].contains(&&**lang),
          CodeBlockKind::Indented => false,
        };
        self.code = Some((String::new(), python));
      }
      Tag::Item => {
        self.flush();
        let mark = match self.lists.last_mut() {
          Some(Some(next)) => {
            *next += 1;
            format!("{}. ", *next - 1)
          }
          _ => "• ".to_owned(),
        };
        self.words.push(run(mark, self.tokens.style(Role::Muted)));
      }
      Tag::List(first) => {
        self.flush();
        self.lists.push(first.map(|first| usize::try_from(first).unwrap_or_default()));
      }
      Tag::Strong | Tag::Heading { .. } => self.styles.push(self.style().bold()),
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
        if let Some((text, python)) = self.code.take() {
          self.block(text.trim_end_matches('\n'), python);
        }
      }
      TagEnd::List(_) => {
        self.flush();
        self.lists.pop();
        self.gap();
      }
      TagEnd::Strong | TagEnd::Emphasis => {
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
      _ => {}
    }
  }

  fn style(&self) -> Style {
    self.styles.last().copied().unwrap_or_else(|| self.tokens.style(Role::Prose))
  }

  /// A text added to the words, each word with its style and the link that stands.
  fn text(&mut self, text: &str, style: Style) {
    for (at, word) in text.split(' ').enumerate() {
      let words = [(at > 0).then_some(" "), Some(word).filter(|word| !word.is_empty())];
      for word in words.into_iter().flatten() {
        let span = run(word, style);
        self.words.push(match &self.link {
          Some(link) => span.link(link.clone()),
          None => span,
        });
      }
    }
  }

  /// The words wrapped into lines between words, under the mark of the item that they follow.
  fn flush(&mut self) {
    let indent = self.lists.len() * 2;
    let mut line: Line = Vec::new();
    let mut used = 0;
    for word in self.words.drain(..) {
      let space = word.content == " ";
      if space && used == 0 {
        continue;
      }
      if used + word.content.width() > self.room && used > 0 && !space {
        self.out.push(line);
        line = vec![run(" ".repeat(indent), Style::new())];
        used = indent;
      }
      used += word.content.width();
      line.push(word);
    }
    if used > 0 {
      self.out.push(line);
    }
  }

  fn gap(&mut self) {
    if self.out.last().is_some_and(|line| !line.is_empty()) {
      self.out.push(Vec::new());
    }
  }

  /// A block of code on the raised ground, as wide as the prose.
  fn block(&mut self, text: &str, python: bool) {
    let ground = Style::new().bg(self.tokens.raised);
    let plain = self.tokens.style(Role::Prose).bg(self.tokens.raised);
    let lines = if python {
      self.colors.lines(text, plain).to_vec()
    } else {
      text.split('\n').map(|one| vec![run(one, plain)]).collect()
    };
    for mut line in lines {
      line.insert(0, run(" ", ground));
      let used: usize = line.iter().map(|one| one.content.width()).sum();
      line.push(run(" ".repeat(self.room.saturating_sub(used)), ground));
      self.out.push(line);
    }
    self.gap();
  }
}
