use super::{GRANTED, RESOLVED, WAITS, WITHDRAWN};
use crate::{
  Answered, Block, Exchange, Line, Maker, Mark, Page, Reply, Said, Saying,
  protocol::{Answer, Frame, PROTOCOL, Role},
};

/// A page of one exchange: a line that waits, a question that a cancel withdrew, and a message
/// that a person resolved.
fn page() -> Page {
  let line = Line::of(1, "x = 1".to_owned(), Mark::Waits);
  let block = Block { maker: Maker::Operator, lines: vec![line], more: false, thought: None };
  let answered = Answered { by: Maker::Operator, reply: Reply::Withdrawn(None) };
  let question = Saying::Question { shape: "str".to_owned(), answered: Some(answered) };
  let said = |kind| Said { act: "prompt2".into(), kind, text: "resolved".to_owned() };
  let exchange = Exchange {
    at: 0,
    message: "prompt1".into(),
    said_at: 0,
    by: Maker::Operator,
    text: "hi".to_owned(),
    python: false,
    runs: false,
    block: Some(block),
    said: vec![said(question), said(Saying::Resolved { by: Maker::Operator })],
  };
  Page { exchanges: vec![exchange], ..Page::default() }
}

/// The page of a reply as a peer of a version reads it.
fn read(version: u32) -> Page {
  let frame = Frame::Reply { id: 1, answer: Answer::Page(page()) };
  match frame.told_to(version) {
    Some(Frame::Reply { answer: Answer::Page(page), .. }) => page,
    other => panic!("a reply of a page stays a reply of a page: {other:?}"),
  }
}

#[test]
fn a_peer_of_now_reads_each_value_as_it_stands() {
  assert_eq!(read(PROTOCOL), page());
}

/// The mark of the line, the reply of the question and the saying of the resolve that a peer of a
/// version reads.
fn seen(version: u32) -> (Mark, Reply, Saying) {
  let mut exchange = read(version).exchanges.remove(0);
  let mark = exchange.block.expect("the exchange holds its block").lines.remove(0).mark;
  let (question, resolve) = (exchange.said.remove(0).kind, exchange.said.remove(0).kind);
  let Saying::Question { answered: Some(answered), .. } = question else {
    panic!("the question stays a question: {question:?}")
  };
  (mark, answered.reply, resolve)
}

#[test]
fn a_peer_older_than_a_value_reads_it_as_its_version_said_it() {
  let refused = Reply::Refused("CancelledError".to_owned());
  let resolved = Saying::Resolved { by: Maker::Operator };
  assert_eq!(seen(WITHDRAWN - 1), (Mark::Ran, refused, Saying::Answer));
  assert_eq!(seen(WAITS - 1), (Mark::Ran, Reply::Withdrawn(None), Saying::Answer));
  assert_eq!(seen(RESOLVED - 1), (Mark::Waits, Reply::Withdrawn(None), Saying::Answer));
  assert_eq!(seen(RESOLVED), (Mark::Waits, Reply::Withdrawn(None), resolved));
}

#[test]
fn a_frame_that_a_version_did_not_have_reaches_no_peer_of_that_version() {
  let granted = || Frame::Granted { role: Role::Read };
  assert_eq!(granted().told_to(GRANTED - 1), None);
  assert_eq!(granted().told_to(GRANTED), Some(granted()));
}
