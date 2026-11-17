use std::time::Duration;

use furb_wire::{Block, Exchange, Line, Maker, Mark, Page, Said, Saying, Work};

use super::Thread;

/// A page of one exchange whose block holds one line with a mark.
fn page(mark: Mark) -> Page {
  let maker = Maker::Model { model: "opus".to_owned(), effort: None };
  let line = Line::of(1, "out = await bash('sleep 1')".to_owned(), mark);
  let block = Block { maker, lines: vec![line], more: false, thought: None };
  let exchange = Exchange {
    at: 0,
    message: "prompt1".into(),
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
fn a_line_ends_when_its_work_ends_and_not_when_it_waits_to_run() {
  let (now, later) = (Duration::from_millis(100), Duration::from_millis(200));
  let mut thread = Thread::default();
  // The model wrote the line and goes on: the line waits to run, and nothing ended.
  thread.took(page(Mark::Runs { work: Work::Writes, began: None }), now);
  thread.took(page(Mark::Waits), now);
  assert!(thread.ended.is_empty(), "{:?}", thread.ended);
  // Its command ran and ended: the line ended then.
  thread.took(page(Mark::Runs { work: Work::Command, began: None }), now);
  thread.took(page(Mark::Ran), later);
  assert_eq!(thread.ended.get(&1), Some(&later));
}

#[test]
fn what_a_thread_said_last_is_its_last_answer_or_notice_and_never_a_question() {
  let said =
    |act: &str, kind: Saying, text: &str| Said { act: act.into(), kind, text: text.to_owned() };
  let mut first = page(Mark::Ran);
  first.exchanges[0].said = vec![said("close1", Saying::Answer, "The first answer.")];
  let mut second = first.exchanges[0].clone();
  second.at = 1;
  second.said = vec![
    said("prompt2", Saying::Notice, "A notice."),
    said("prompt3", Saying::Question { shape: "str".to_owned(), answered: None }, "Which one?"),
  ];
  first.exchanges.push(second);
  let mut thread = Thread::default();
  thread.took(first, Duration::ZERO);
  assert_eq!(thread.last_said(), Some("A notice."));
}
