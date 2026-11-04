//! The cut of each literal that the checker could not read in time.

use ruff_python_ast::token::TokenKind;
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_text_size::{Ranged, TextRange, TextSize};

/// The most brackets, one in another, that the checker reads in a literal. The checker takes time
/// of about the fourth power of the depth of a literal, and more again when a part has no type that
/// it knows, as an empty list: a list of 32 levels with an empty list at the bottom takes 6 s.
/// monty writes a value of about 990 levels as a literal, which a tell puts in the program. Each
/// shape of lists, tuples and dicts that was measured reads in less than 0.05 s at 12 levels.
pub(super) const DEEP: usize = 12;

/// The sheet with each literal of a list, a tuple, a set or a dict that holds more than `DEEP`
/// brackets, one in another, cut to an empty one of its kind, which keeps its lines. A value that a
/// tell wrote as a literal stays in the program, so each later word reads it again. The whole
/// literal is cut, and not its deep parts, so no part of a type that the checker does not know
/// stands deep in a union. The cut reads the tokens and not the tree, since the parser gives no
/// tree past about 200 brackets.
pub(super) fn shallowed(sheet: &str) -> String {
  let parsed = parse_unchecked(sheet, ParseOptions::from(Mode::Module));
  // Each open bracket: where it starts, its kind, whether it opens a literal, whether a colon
  // stands in it as an item of its own, and the most brackets that stand one in another in it,
  // itself among them.
  let mut open: Vec<(TextSize, TokenKind, bool, bool, usize)> = vec![];
  let mut cut: Vec<(TextRange, (&str, &str))> = vec![];
  let (mut before, mut strings) = (TokenKind::Newline, 0usize);
  for token in parsed.tokens() {
    let kind = token.kind();
    match kind {
      TokenKind::FStringStart | TokenKind::TStringStart => strings += 1,
      TokenKind::FStringEnd | TokenKind::TStringEnd => strings = strings.saturating_sub(1),
      TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => {
        // A bracket after a value opens a call or a subscript, and a bracket in an f-string opens a
        // part of it.
        let literal = strings == 0 && (kind == TokenKind::Lbrace || !valued(before));
        open.push((token.start(), kind, literal, false, 1));
      }
      TokenKind::Colon => {
        if let Some(top) = open.last_mut() {
          top.3 = true;
        }
      }
      TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace => {
        if let Some((start, opened, literal, colon, deepest)) = open.pop() {
          if let Some(around) = open.last_mut() {
            around.4 = around.4.max(deepest + 1);
          }
          if literal && deepest > DEEP {
            let marks = match opened {
              TokenKind::Lsqb => ("[", "]"),
              TokenKind::Lpar => ("tuple([", "])"),
              _ if colon => ("{", "}"),
              _ => ("set(", ")"),
            };
            cut.push((TextRange::new(start, token.end()), marks));
          }
        }
      }
      _ => {}
    }
    if !matches!(kind, TokenKind::Comment | TokenKind::NonLogicalNewline) {
      before = kind;
    }
  }
  cut.sort_by_key(|(range, _)| range.start());
  let (mut text, mut at) = (String::with_capacity(sheet.len()), 0);
  for (range, (first, last)) in cut {
    let (from, to) = (range.start().to_usize(), range.end().to_usize());
    // A literal in a literal that is cut goes with it.
    if from < at {
      continue;
    }
    text.push_str(&sheet[at..from]);
    text.push_str(first);
    text.extend(std::iter::repeat_n('\n', sheet[from..to].matches('\n').count()));
    text.push_str(last);
    at = to;
  }
  text.push_str(&sheet[at..]);
  text
}

/// Whether a token ends a value, so that a bracket after it opens a call or a subscript. A soft
/// keyword counts as a name, so a bracket after one is never cut.
fn valued(token: TokenKind) -> bool {
  matches!(
    token,
    TokenKind::Name
      | TokenKind::Int
      | TokenKind::Float
      | TokenKind::Complex
      | TokenKind::String
      | TokenKind::FStringEnd
      | TokenKind::TStringEnd
      | TokenKind::Rpar
      | TokenKind::Rsqb
      | TokenKind::Rbrace
      | TokenKind::None
      | TokenKind::True
      | TokenKind::False
      | TokenKind::Ellipsis
      | TokenKind::Case
      | TokenKind::Match
      | TokenKind::Type
      | TokenKind::Lazy
  )
}
