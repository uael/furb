//! The form of a question that the open thread asks the operator, which takes the place of the
//! input in that thread alone.

use std::{collections::BTreeSet, time::Duration};

/// How long a form takes no key after it shows, so a key that the operator pressed for another
/// thing answers nothing.
pub const HOLD: Duration = Duration::from_millis(400);

/// The form of the open thread: the act of the question that it shows, the row that the operator
/// chose, the values that he ticked, when it showed, and why the daemon refused his last answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
  pub act: String,
  pub at: usize,
  pub ticked: BTreeSet<usize>,
  pub since: Duration,
  pub why: Option<String>,
}
