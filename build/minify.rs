//! The engine, minified: the system prompt of every model.

use ruff_python_ast::{
  comparable::ComparableModModule,
  token::{Token, TokenKind},
};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

/// One logical line of the engine: how deep it stands, and its parts, each the text of one token as
/// the engine writes it and the kind of that token. A string that holds expressions is one part,
/// whole.
#[derive(Clone)]
struct Line<'a> {
  depth: usize,
  parts: Vec<(TokenKind, &'a str)>,
}

impl Line<'_> {
  /// Whether the line is a simple statement, which a `;` may join to another: no compound statement
  /// starts with its first token.
  fn simple(&self) -> bool {
    !matches!(
      self.parts.first().map(|(kind, _)| kind),
      Some(
        TokenKind::If
          | TokenKind::Elif
          | TokenKind::Else
          | TokenKind::For
          | TokenKind::While
          | TokenKind::Try
          | TokenKind::Except
          | TokenKind::Finally
          | TokenKind::With
          | TokenKind::Def
          | TokenKind::Class
          | TokenKind::Async
          | TokenKind::At
          | TokenKind::Match
          | TokenKind::Case
      )
    )
  }

  /// The line as the prompt writes it: the parts with a space only where two would run together,
  /// and a pattern of a case without the parentheses around it, which change nothing when a colon
  /// or a guard follows them.
  fn text(&self) -> String {
    let mut parts: Vec<_> = self.parts.iter().collect();
    if let [(TokenKind::Case, _), (TokenKind::Lpar, _), ..] = parts[..] {
      let mut open = 0;
      let closing = parts.iter().skip(1).position(|(kind, _)| {
        open += match kind {
          TokenKind::Lpar => 1,
          TokenKind::Rpar => -1,
          _ => 0,
        };
        open == 0
      });
      if let Some(at) = closing.map(|at| at + 1)
        && at > 2
        && matches!(parts.get(at + 1), Some((TokenKind::Colon | TokenKind::If, _)))
      {
        parts.remove(at);
        parts.remove(1);
      }
    }
    let mut out = String::new();
    let mut before: Option<(TokenKind, char)> = None;
    for (kind, text) in parts {
      let first = text.chars().next().unwrap_or(' ');
      let word = |one: char| one.is_alphanumeric() || one == '_';
      if let Some((was, last)) = before {
        let number = matches!(was, TokenKind::Int | TokenKind::Float | TokenKind::Complex);
        if (word(last) && word(first)) || (number && (word(first) || first == '.')) {
          out.push(' ');
        }
      }
      out.push_str(text);
      before = Some((*kind, text.chars().last().unwrap_or(' ')));
    }
    out
  }
}

/// The logical lines of a program, from its tokens: no comment, no line break inside brackets, and
/// every string that holds expressions as the one part it is in the source, so what stands between
/// its quotes stays as it is. A comma that the formatter adds to break a long line goes with the
/// line break: one that a line break and a closing bracket follow.
fn logical<'a>(source: &'a str, tokens: &[Token]) -> Vec<Line<'a>> {
  let (mut lines, mut depth, mut parts) = (Vec::new(), 0, Vec::new());
  let mut at = 0;
  while at < tokens.len() {
    let token = tokens[at];
    at += 1;
    let broken = tokens.get(at).map(Token::kind) == Some(TokenKind::NonLogicalNewline);
    match token.kind() {
      TokenKind::Newline if !parts.is_empty() => {
        lines.push(Line { depth, parts: std::mem::take(&mut parts) });
      }
      TokenKind::Comma
        if broken
          && tokens[at..]
            .iter()
            .map(Token::kind)
            .find(|kind| !matches!(kind, TokenKind::NonLogicalNewline | TokenKind::Comment))
            .is_some_and(|kind| {
              matches!(kind, TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace)
            }) => {}
      TokenKind::Comment
      | TokenKind::NonLogicalNewline
      | TokenKind::Newline
      | TokenKind::EndOfFile => {}
      TokenKind::Indent => depth += 1,
      TokenKind::Dedent => depth -= 1,
      TokenKind::FStringStart | TokenKind::TStringStart => {
        let mut open = 1;
        let mut end = token.end();
        while open > 0 {
          let inner = tokens[at];
          at += 1;
          open += match inner.kind() {
            TokenKind::FStringStart | TokenKind::TStringStart => 1,
            TokenKind::FStringEnd | TokenKind::TStringEnd => -1,
            _ => 0,
          };
          end = inner.end();
        }
        parts.push((TokenKind::String, &source[usize::from(token.start())..usize::from(end)]));
      }
      kind => parts.push((kind, &source[token.range()])),
    }
  }
  lines
}

/// The engine minified with its program kept as it is, which is the system prompt of every model.
/// The comments go, one space is one level of indentation, and each pair of parentheses goes that
/// the program does not need, as the formatter adds them to break a long line. A suite of simple
/// statements stands on the line of its header, the simple statements that follow one another in a
/// block share one line, and the outer parentheses of a case pattern go. The build stops unless the
/// text it makes is the program on disk.
pub(crate) fn minified(source: &str) -> String {
  let parsed = parse_module(source).expect("the engine parses");
  let program = ComparableModModule::from(parsed.syntax());
  let mut lines = logical(source, parsed.tokens());
  // Each pair of parentheses goes, one after the other, when the program without it is the same
  // program.
  for at in 0..lines.len() {
    let mut open = 0;
    while open < lines[at].parts.len() {
      if lines[at].parts[open].0 == TokenKind::Lpar {
        let mut depth = 0;
        let close = open
          + lines[at].parts[open..]
            .iter()
            .position(|(kind, _)| {
              depth += match kind {
                TokenKind::Lpar => 1,
                TokenKind::Rpar => -1,
                _ => 0,
              };
              depth == 0
            })
            .expect("a line closes each parenthesis that it opens");
        let mut fewer = lines.clone();
        fewer[at].parts.remove(close);
        fewer[at].parts.remove(open);
        if parse_module(&laid(&fewer).join("\n"))
          .is_ok_and(|one| ComparableModModule::from(one.syntax()) == program)
        {
          lines = fewer;
          continue;
        }
      }
      open += 1;
    }
  }
  let prompt = laid(&lines).join("\n");
  let again = parse_module(&prompt).expect("the minified engine parses");
  assert!(
    ComparableModModule::from(again.syntax()) == program,
    "the minified engine is the program on disk"
  );
  prompt
}

/// The logical lines as the prompt writes them: each line with its indentation, a suite of simple
/// statements on the line of its header, and the simple statements that follow one another in a
/// block on one line.
fn laid(lines: &[Line]) -> Vec<String> {
  // Each line as the prompt writes it, with its depth, and whether a simple statement after it may
  // join it.
  let mut out: Vec<(String, usize, bool)> = Vec::new();
  let mut at = 0;
  while at < lines.len() {
    let line = &lines[at];
    let suite = lines[at + 1..].iter().take_while(|one| one.depth > line.depth).count();
    let body = &lines[at + 1..at + 1 + suite];
    let indent = " ".repeat(line.depth);
    if suite > 0 && body.iter().all(|one| one.depth == line.depth + 1 && one.simple()) {
      let joined = body.iter().map(Line::text).collect::<Vec<_>>().join(";");
      out.push((format!("{indent}{}{joined}", line.text()), line.depth, false));
      at += 1 + suite;
      continue;
    }
    let simple = suite == 0 && line.simple();
    match out.last_mut() {
      Some((text, depth, true)) if simple && line.depth > 0 && *depth == line.depth => {
        text.push(';');
        text.push_str(&line.text());
      }
      _ => out.push((format!("{indent}{}", line.text()), line.depth, simple)),
    }
    at += 1;
  }
  out.into_iter().map(|(text, ..)| text).collect()
}
