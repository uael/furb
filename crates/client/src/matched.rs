//! The match of a text that the operator types against the rows of a list, as the pickers of a
//! terminal match it, by nucleo, the matcher of helix: each list of each client ranks its rows the
//! same way, and lights the same letters.

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
  matched(query, rows, Config::DEFAULT, |_| 0.0)
}

/// The paths of a list that a text matches, as `ranked` ranks rows, with the bonus of a match
/// that starts a part of a path, as a picker of files ranks them, and each score raised by a share
/// that the place of the path gives, as how often and how lately he named it: the paths that he
/// named come first while the text is empty.
pub fn pathed<'a>(
  query: &str,
  paths: impl IntoIterator<Item = &'a str>,
  liked: impl Fn(usize) -> f64,
) -> Vec<Matched> {
  matched(query, paths, Config::DEFAULT.match_paths(), liked)
}

fn matched<'a>(
  query: &str,
  rows: impl IntoIterator<Item = &'a str>,
  config: Config,
  liked: impl Fn(usize) -> f64,
) -> Vec<Matched> {
  let rows = rows.into_iter().enumerate();
  let mut scored: Vec<(f64, Matched)> = if query.is_empty() {
    rows.map(|(at, _)| (liked(at), Matched { at, lit: Vec::new() })).collect()
  } else {
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(config);
    let mut chars = Vec::new();
    rows
      .filter_map(|(at, row)| {
        let mut lit = Vec::new();
        let score = pattern.indices(Utf32Str::new(row, &mut chars), &mut matcher, &mut lit)?;
        lit.sort_unstable();
        lit.dedup();
        let lit = lit.into_iter().filter_map(|one| usize::try_from(one).ok()).collect();
        Some((f64::from(score) * (1.0 + liked(at)), Matched { at, lit }))
      })
      .collect()
  };
  scored.sort_by(|one, other| other.0.total_cmp(&one.0));
  scored.into_iter().map(|one| one.1).collect()
}
