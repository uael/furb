//! The watch over the word of a reply while its model writes it.
//!
//! A word is python, and a statement of it that cannot be python stays so whatever the model writes after it. So the
//! provider reads the statements that a model finished, each as it finishes, and ends the reply at the first one
//! that cannot be python, with the word as far as that statement. The gate refuses that word, as it refuses the
//! whole one, and the model is not made to write the rest of a word that the chain will refuse.
//!
//! A statement that is python can still hold a fault that the gate finds, as a name that nothing binds. A later
//! statement mends no such fault in a statement at the top of the word, which runs before it, unless the statement
//! holds a part that python runs later or never: a function, a class, a lambda, a comprehension or an annotation. So
//! the provider asks the gate about the statements that a model finished, and ends the reply after the first
//! statement with no such part that holds a fault.
//!
//! The statements that a model finished, and that the gate finds nothing against, run as the model writes the rest:
//! the provider says each new part of the word as a writing of its rung, a fact that the Kernel hears and runs. A
//! statement that raised ends the word where it raised, which the Kernel says as the fact ran, so the reply ends
//! there. Once a part ran, the turn of the reply is never a word other than the one that ran: it is the word as far
//! as the parts that ran, whatever the model wrote after them that the chain would refuse, since a later life runs
//! the word of the rung and a statement of it must run there as it ran.

use ruff_python_ast::{
  Expr, Stmt,
  visitor::{self, Visitor},
};
use ruff_python_parser::{ParseErrorType, parse_module};
use ruff_text_size::Ranged;

/// The mark that opens a quote in a word: `<s:name>` at the start of a line.
pub(super) const QUOTE: &str = "<s:";

/// The mark that opens the end of a quote in a word: `</s:name>` at the end of a line.
const UNQUOTE: &str = "</s:";

/// What a reply that is watched has of its word so far.
#[derive(Default)]
pub(super) struct Watch {
  text: String,
  /// Where the last statements to read ended: the start of a line at the left edge, or 0.
  seen: usize,
  /// The word, as python, that the parts given to the Kernel hold.
  sent: String,
  /// Where in the text the parts given to the Kernel end: the start of a line at the left edge, or 0.
  given: usize,
  /// Whether the word is given to the Kernel in no part, as when a pause held its first part.
  whole: bool,
  /// What the model wrote after the last end of a line, which the text takes once its line ends.
  tail: String,
  /// The marks of a fence or of a tool call that the text drops as each line ends.
  bare: Bare,
}

impl Watch {
  /// The word grown by a part the model wrote, and where the statements to read end if a new line at the left edge
  /// started: the start of the last such line. A statement that starts there may not be whole, but every statement
  /// before it is, unless a string or a bracket stays open, which the faults of [`broken`] leave to a later read. The
  /// text takes each line once it ends, less the marks that [`Bare`] drops, so a line that the text has not taken
  /// yet only says whether a line at the left edge starts.
  pub(super) fn wrote(&mut self, part: &str) -> Option<usize> {
    self.tail.push_str(part);
    while let Some(at) = self.tail.find('\n') {
      let line: String = self.tail.drain(..=at).collect();
      if let Some(kept) = self.bare.line(&line[..at], &self.text) {
        self.text.push_str(&kept);
        self.text.push('\n');
      }
    }
    let (text, tail) = (&self.text, &self.tail);
    let end =
      text[self.seen..].match_indices('\n').map(|(at, _)| self.seen + at + 1).rfind(|after| {
        text[*after..].chars().chain(tail.chars()).next().is_some_and(|one| !one.is_whitespace())
      })?;
    self.seen = end;
    Some(end)
  }

  /// The word up to where the statements to read end.
  pub(super) fn upto(&self, end: usize) -> &str {
    &self.text[..end]
  }

  /// The word as far as the end of a line of it, counted from 1.
  pub(super) fn through(&self, line: usize) -> &str {
    let end = self.text.match_indices('\n').nth(line - 1).map_or(self.text.len(), |(at, _)| at);
    &self.text[..end]
  }

  /// The word forgotten, as when the model is asked again and writes it from its start.
  pub(super) fn forget(&mut self) {
    *self = Watch::default();
  }

  /// The part of a word, as python, that is new since the parts given to the Kernel, and how many characters of the
  /// word stand before it, as python counts them; nothing when the word holds no new statement, or does not follow
  /// what was given.
  pub(super) fn part<'a>(&self, word: &'a str) -> Option<(&'a str, usize)> {
    let part = word.strip_prefix(self.sent.as_str()).filter(|_| !self.whole)?;
    (!part.trim().is_empty()).then(|| (part, self.sent.chars().count()))
  }

  /// The word, as python, given to the Kernel as far as where it ends in the text.
  pub(super) fn gave(&mut self, word: &str, end: usize) {
    self.sent = word.to_owned();
    self.given = end;
  }

  /// The word given to the Kernel in no part from now on, since a pause held its first part.
  pub(super) fn untaken(&mut self) {
    self.whole = true;
  }

  /// Whether the line that starts at this place in the text opens a clause of the statement before it, as `else` does
  /// of an `if`, so that statement is not whole yet though a line at the left edge starts after it.
  pub(super) fn clause(&self, end: usize) -> bool {
    let rest = if end < self.text.len() { &self.text[end..] } else { self.tail.as_str() };
    let line = rest.lines().next().unwrap_or_default();
    let word =
      line.split(|one: char| !(one.is_alphanumeric() || one == '_')).next().unwrap_or_default();
    matches!(word, "else" | "elif" | "except" | "finally")
  }

  /// Where a line of the text starts, counted from 1.
  pub(super) fn start(&self, line: usize) -> usize {
    if line <= 1 {
      return 0;
    }
    self.text.match_indices('\n').nth(line - 2).map_or(self.text.len(), |(at, _)| at + 1)
  }

  /// The text of the word as far as the parts given to the Kernel, when a part was given.
  pub(super) fn ran(&self) -> Option<&str> {
    (self.given > 0).then(|| &self.text[..self.given])
  }

  /// What the chain is told of the marks that the text dropped, or of the name that it gave a binding of no name.
  pub(super) fn note(&self) -> Option<String> {
    self.bare.note()
  }
}

/// The marks that a model writes around its word out of the habit of a fence or of a tool call, which the word loses
/// line by line: a fence, as ``` alone or with a language, a line that is a tag of a tool call alone, as
/// `<invoke name="python">`, `<function=write>` or their close, or a line that is a break of HTML alone, as
/// `<br>`, while no line of the word stands before it; and once one went, each such line and each fence, a tag
/// of a tool call at the start of a line and its close at the end of one, outside a quote. A line that is the close
/// of a tag of a tool call alone goes too after the word that it ends, when that word is python, so the line stands in
/// no string. A word that no such mark opens keeps each line. A first line of the word that starts with a lone `=`, as
/// the binding of a model that left out its name, binds `_`.
#[derive(Default)]
pub(super) struct Bare {
  /// Whether a line of the word that is no mark stood.
  started: bool,
  /// Whether a mark went.
  dropped: bool,
  /// Whether the first line of the word bound `_` in place of no name.
  named: bool,
}

/// The names of the tags of a tool call.
const TAGS: [&str; 5] = ["invoke", "parameter", "function_calls", "function", "tool_call"];

/// What the chain is told after the rung of a turn whose word lost the marks of a fence or of a tool call around it.
const WRAPPED: &str = "wrapped its word in a fence, in the markup of a tool call or in a break of HTML, and the \
  provider dropped that markup: a reply is python and quotes only, with no fence and no markup, and the text of a \
  file is a quote";

/// What the chain is told after the rung of a turn whose word started with a binding of no name.
const NAMED: &str = "started its word with =, a binding with no name, and the provider bound it to _: write the \
  name before =, as in got = 1";

impl Bare {
  /// A line of the text as the word holds it, or none when it is a mark that goes. `before` is the word before it.
  pub(super) fn line(&mut self, line: &str, before: &str) -> Option<String> {
    let mut line = line.to_owned();
    let trimmed = line.trim();
    if !open(before)
      && trimmed.starts_with("</")
      && tag(trimmed).is_some_and(str::is_empty)
      && self.started
      && parse_module(&unquoted(before)).is_ok()
    {
      self.dropped = true;
      return None;
    }
    if (!self.started || self.dropped) && !open(before) {
      if fence(trimmed) || (!self.started && broke(trimmed)) {
        self.dropped = true;
        return None;
      }
      if let Some(mut rest) = tag(trimmed) {
        while let Some(more) = tag(rest) {
          rest = more;
        }
        self.dropped = true;
        if rest.is_empty() {
          return None;
        }
        line = rest.to_owned();
      }
      if self.dropped {
        let end = line.trim_end();
        if let Some(kept) = ["</parameter>", "</invoke>", "</function>"]
          .into_iter()
          .find_map(|close| end.strip_suffix(close))
        {
          line = kept.to_owned();
        }
      }
    }
    if !self.started
      && let Some(rest) = line.trim_start().strip_prefix('=')
      && !rest.starts_with('=')
    {
      self.named = true;
      line = format!("_ ={rest}");
    }
    self.started |= !line.trim().is_empty();
    Some(line)
  }

  /// What the chain is told of what the word lost or gained, if it lost or gained anything.
  pub(super) fn note(&self) -> Option<String> {
    let said: Vec<&str> = [self.dropped.then_some(WRAPPED), self.named.then_some(NAMED)]
      .into_iter()
      .flatten()
      .collect();
    (!said.is_empty()).then(|| said.join(", and "))
  }
}

/// Whether a line, trimmed, is a break of HTML alone, as `<br>`, `<br/>` or `<br />`.
fn broke(line: &str) -> bool {
  matches!(line.to_ascii_lowercase().as_str(), "<br>" | "<br/>" | "<br />")
}

/// Whether a line, trimmed, is a fence: ``` alone or with the name of a language.
fn fence(line: &str) -> bool {
  line
    .strip_prefix("```")
    .is_some_and(|name| name.chars().all(|one| one.is_ascii_alphanumeric() || "_+-.".contains(one)))
}

/// What follows a tag of a tool call that opens a line, trimmed: `<invoke ...>`, `<parameter ...>`, `<function=...>`,
/// `<tool_call>`, `<function_calls>` or the close of one; none when the line opens with no such tag.
fn tag(line: &str) -> Option<&str> {
  let inside = line.strip_prefix("</").or_else(|| line.strip_prefix('<'))?;
  let name = TAGS.into_iter().find(|name| inside.starts_with(name))?;
  let after = &inside[name.len()..];
  if !after.starts_with([' ', '>', '=']) {
    return None;
  }
  Some(after[after.find('>')? + 1..].trim())
}

/// A whole text less the marks that [`Bare`] drops, with the name that it gives a binding of no name, and what the
/// chain is told of it; none when it changes nothing.
pub(super) fn bare(text: &str) -> Option<(String, String)> {
  let (mut bare, mut kept) = (Bare::default(), String::new());
  for line in text.split('\n') {
    if let Some(line) = bare.line(line, &kept) {
      kept.push_str(&line);
      kept.push('\n');
    }
  }
  bare.note().map(|note| (kept.trim_end().to_owned(), note))
}

/// The line, counted from 1, on which the last statement at the top of a word starts, if the word is python and holds
/// one.
pub(super) fn last(word: &str) -> Option<usize> {
  let parsed = parse_module(word).ok()?;
  let start = parsed.syntax().body.last()?.start().to_usize();
  Some(word[..start].matches('\n').count() + 1)
}

/// Each quote of a word, as the engine reads one: where it starts and, when it ends, where its close mark ends. A
/// quote opens at `<s:name>` at the start of a line and ends at the first `</s:name>` after it that ends a line, so a
/// close mark inside a line of its text ends nothing. The list ends with the first quote that does not end.
fn quotes(word: &str) -> Vec<(usize, &str, Option<usize>)> {
  let (mut from, mut found) = (0, Vec::new());
  while let Some(start) = word[from..].find(QUOTE).map(|at| from + at) {
    from = start + QUOTE.len();
    let Some((name, _)) = word[from..].split_once('>') else { continue };
    let named = !name.is_empty() && name.chars().all(|one| one.is_alphanumeric() || one == '_');
    if !named || !(start == 0 || word[..start].ends_with('\n')) {
      continue;
    }
    let text = from + name.len() + 1;
    let close = format!("{UNQUOTE}{name}>");
    let ends = word[text..]
      .match_indices(&close)
      .find(|(at, _)| matches!(word[text + at + close.len()..].chars().next(), None | Some('\n')));
    match ends {
      Some((at, _)) => {
        from = text + at + close.len();
        found.push((start, name, Some(from)));
      }
      None => {
        found.push((start, name, None));
        break;
      }
    }
  }
  found
}

/// Whether a quote stands open in a word, since the text of a quote is no python until its end is written.
pub(super) fn open(word: &str) -> bool {
  quotes(word).last().is_some_and(|(_, _, end)| end.is_none())
}

/// A word with each quote that ends as a binding of its name, on as many lines, so the word parses as python.
fn unquoted(word: &str) -> String {
  let (mut made, mut at) = (String::new(), 0);
  for (start, name, end) in quotes(word) {
    let Some(end) = end else { break };
    made.push_str(&word[at..start]);
    made.push_str(&format!("{name} = 0{}", "\n".repeat(word[start..end].matches('\n').count())));
    at = end;
  }
  made.push_str(&word[at..]);
  made
}

/// The line, counted from 1, of the first fault of a word that more text cannot mend, if it has one: a fault at the
/// end of the word may be a statement that is not whole yet, and so may a string that has no end yet.
pub(super) fn broken(word: &str) -> Option<usize> {
  let no = parse_module(word).err()?;
  let (start, end) = (usize::from(no.location.start()), usize::from(no.location.end()));
  let last = word.trim_end().len();
  let lexical = matches!(no.error, ParseErrorType::Lexical(_));
  (start < last && !(lexical && end >= last)).then(|| word[..start].matches('\n').count() + 1)
}

/// The first and the last line, counted from 1, of each statement at the top of a word whose faults no later
/// statement can mend, or none when the word is not python.
pub(super) fn settled(word: &str) -> Vec<(usize, usize)> {
  let Ok(parsed) = parse_module(word) else { return vec![] };
  let line = |at: usize| word[..at].matches('\n').count() + 1;
  let body = &parsed.syntax().body;
  body
    .iter()
    .filter(|stmt| {
      let mut later = Later(false);
      later.visit_stmt(stmt);
      !later.0
    })
    .map(|stmt| (line(stmt.start().to_usize()), line(stmt.end().to_usize())))
    .collect()
}

/// The last line of the first settled statement that holds a finding of the gate, if one does. Each finding starts
/// with the line of the word that it is found on, as `line 3: ...`.
pub(super) fn refused(settled: &[(usize, usize)], found: &[String]) -> Option<usize> {
  let lines: Vec<usize> = found
    .iter()
    .filter_map(|one| one.strip_prefix("line ")?.split(':').next()?.parse().ok())
    .collect();
  settled
    .iter()
    .find(|(first, last)| lines.iter().any(|line| (first..=last).contains(&line)))
    .map(|(_, last)| *last)
}

/// Whether a statement holds a part that python runs later or never, which a later statement can change the
/// meaning of.
struct Later(bool);

impl<'a> Visitor<'a> for Later {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(_) | Stmt::ClassDef(_) | Stmt::AnnAssign(_) => self.0 = true,
      _ => visitor::walk_stmt(self, stmt),
    }
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    match expr {
      Expr::Lambda(_)
      | Expr::ListComp(_)
      | Expr::SetComp(_)
      | Expr::DictComp(_)
      | Expr::Generator(_) => {
        self.0 = true;
      }
      _ => visitor::walk_expr(self, expr),
    }
  }
}

#[cfg(test)]
#[path = "watch.test.rs"]
mod test;
