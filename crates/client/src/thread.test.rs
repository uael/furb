use std::time::Duration;

use furb_wire::{Block, Exchange, Line, Maker, Mark, Page, Work};

use super::Thread;

/// A page of one exchange whose block holds one line with a mark.
fn page(mark: Mark) -> Page {
  let maker = Maker::Model { model: "opus".to_owned(), effort: None };
  let line = Line::of(1, "out = await bash('sleep 1')".to_owned(), mark);
  let block = Block { maker, lines: vec![line], more: false, thought: None };
  let exchange = Exchange {
    at: 0,
    message: "prompt1".to_owned(),
    said_at: 0,
    by: Maker::Operator,
    text: "Run it.".to_owned(),
    python: false,
    runs: true,
    block: Some(block),
    said: Vec::new(),
  };
  Page { exchanges: vec![exchange], ..Page::default() }
}

#[test]
fn a_line_ends_when_its_work_ends_and_not_when_its_model_goes_on_to_the_next_line() {
  let (now, later) = (Duration::from_millis(100), Duration::from_millis(200));
  let mut thread = Thread::default();
  // The model wrote the line and goes on: the line waits to run, and nothing ended.
  thread.took(page(Mark::Runs { work: Work::Writes, began: None }), now);
  thread.took(page(Mark::Ran), now);
  assert!(thread.ended.is_empty(), "{:?}", thread.ended);
  // Its command ran and ended: the line ended then.
  thread.took(page(Mark::Runs { work: Work::Command, began: None }), now);
  thread.took(page(Mark::Ran), later);
  assert_eq!(thread.ended.get(&1), Some(&later));
}
