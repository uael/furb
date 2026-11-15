//! The match of a text that the operator types against the rows of a list, as the pickers of a
//! terminal match it, by nucleo, the matcher of helix: each list of each client ranks its rows the
//! same way, and lights the same letters.

use std::cmp::Reverse;

use nucleo_matcher::{
  Config, Matcher, Utf32Str,
  pattern::{CaseMatching, Normalization, Pattern},
};

/// A row of a list that a text matches: its place in the list, and the places of the characters
/// of the row that the text matched, the first first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matched {
  pub at: usize,
  pub lit: Vec<usize>,
}

/// The rows of a list that a text matches, the best match first and rows of one score in their
/// order: each row in its order, with no letter lit, while the text is empty.
pub fn ranked<'a>(query: &str, rows: impl IntoIterator<Item = &'a str>) -> Vec<Matched> {
  let rows = rows.into_iter().enumerate();
  if query.is_empty() {
    return rows.map(|(at, _)| Matched { at, lit: Vec::new() }).collect();
  }
  let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
  let mut matcher = Matcher::new(Config::DEFAULT);
  let mut chars = Vec::new();
  let mut scored: Vec<(u32, Matched)> = rows
    .filter_map(|(at, row)| {
      let mut lit = Vec::new();
      let score = pattern.indices(Utf32Str::new(row, &mut chars), &mut matcher, &mut lit)?;
      lit.sort_unstable();
      lit.dedup();
      let lit = lit.into_iter().filter_map(|one| usize::try_from(one).ok()).collect();
      Some((score, Matched { at, lit }))
    })
    .collect();
  scored.sort_by_key(|one| Reverse(one.0));
  scored.into_iter().map(|one| one.1).collect()
}
