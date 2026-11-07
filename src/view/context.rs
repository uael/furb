//! The breakdown of the window of a chain: the tokens that its last reply read, split by the size
//! of each part of the prefix that the chain sends, with the reserve that the compaction keeps
//! and the free space of the window. One rule splits the tokens, so each window draws the same
//! breakdown.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A part of the window of a chain, in the order that the breakdown lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Part {
  Messages,
  SystemTools,
  SystemPrompt,
  Skills,
  MemoryFiles,
  Reserve,
  Free,
}

/// A part with its tokens and its share of the window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Share {
  pub part: Part,
  pub tokens: u64,
  pub share: f64,
}

/// The breakdown of a window: the tokens of the prompt that the last reply read, split by the size
/// of each part of the prefix (in any unit, as bytes), each part taking its whole tokens and the
/// largest remainders the tokens left so that the parts sum to the prompt; then the reserve of the
/// compaction, and the free space that is left. A part of no size takes no token, and a window of
/// no size gives each part a share of zero.
pub fn breakdown(window: u64, prompt: u64, sizes: &[(Part, u64)], reserve: u64) -> Vec<Share> {
  let whole: u64 = sizes.iter().map(|one| one.1).sum();
  let mut split: Vec<(Part, u64, u64)> = sizes
    .iter()
    .map(|&(part, size)| {
      let exact = u128::from(prompt) * u128::from(size);
      let total = u128::from(whole.max(1));
      (
        part,
        u64::try_from(exact / total).unwrap_or(u64::MAX),
        u64::try_from(exact % total).unwrap_or(0),
      )
    })
    .collect();
  if whole > 0 {
    let given: u64 = split.iter().map(|one| one.1).sum();
    let mut order: Vec<usize> = (0..split.len()).collect();
    order.sort_by(|a, b| split[*b].2.cmp(&split[*a].2));
    for at in order.into_iter().take(usize::try_from(prompt.saturating_sub(given)).unwrap_or(0)) {
      split[at].1 += 1;
    }
  }
  let reserve = reserve.min(window.saturating_sub(prompt));
  let free = window.saturating_sub(prompt).saturating_sub(reserve);
  // A window holds far fewer tokens than a u32 counts, so the share reads them as u32.
  let wide = |tokens: u64| f64::from(u32::try_from(tokens).unwrap_or(u32::MAX));
  let share = |tokens: u64| if window == 0 { 0.0 } else { wide(tokens) / wide(window) };
  split
    .into_iter()
    .map(|(part, tokens, _)| (part, tokens))
    .chain([(Part::Reserve, reserve), (Part::Free, free)])
    .map(|(part, tokens)| Share { part, tokens, share: share(tokens) })
    .collect()
}

#[cfg(test)]
#[path = "context.test.rs"]
mod tests;
