use std::{cell::Cell, collections::BTreeMap, ops::Range, path::PathBuf};

use furb::view::{self, Block, Exchange, Line as Code, Maker, Mark, Page, Standing};
use furb_host::protocol::{Card, Verb};
use jiff::tz::TimeZone;

use super::{Anchor, Item, Items, Opened, Row};
use crate::{
  Options, System,
  model::{Link, Model, Told},
  tokens::Tokens,
};

/// One exchange of a thread: a message, and a block of two lines that ran.
fn exchange(at: usize) -> Exchange {
  let line = |number: usize| Code { number, text: format!("step = {number}"), mark: Mark::Ran };
  let maker = Maker::Model { model: "claude-cli:opus".to_owned(), effort: None };
  let block =
    Block { maker, lines: vec![line(2 * at + 1), line(2 * at + 2)], more: false, thought: None };
  Exchange {
    at,
    message: format!("prompt{at}"),
    by: Maker::Operator,
    text: format!("and the step {at}"),
    block: Some(block),
    said: Vec::new(),
  }
}

/// The page of the exchanges of a thread from one place to another.
fn page(places: Range<usize>) -> Page {
  Page { above: places.start, exchanges: places.map(exchange).collect(), agents: Vec::new() }
}

/// A window of 100 columns and 30 rows on one machine, whose thread is open on the page of its
/// end.
fn opened(end: Page) -> Model {
  let thread = view::Card {
    prompt: "prompt0".to_owned(),
    title: "ground the branch".to_owned(),
    standing: Standing::Rests,
    model: None,
    agents: 0,
    readers: Vec::new(),
  };
  let card = Card {
    life: "/home/ada/furb/.furb/sessions/long.jsonl".to_owned(),
    session: None,
    project: None,
    held: true,
    viewers: Vec::new(),
    since: 0,
    thread,
  };
  let key = (0, card.life.clone(), card.thread.prompt.clone());
  let told = Told {
    name: "mac".to_owned(),
    login: "ada".to_owned(),
    heard: 0,
    cards: vec![card],
    link: Link::Listed,
    ..Told::default()
  };
  let options = Options {
    folder: PathBuf::from("/home/ada/furb"),
    home: Some(PathBuf::from("/home/ada")),
    kept: PathBuf::new(),
    zone: TimeZone::UTC,
    clock: || 0,
    system: System::MacOs,
  };
  let mut model = Model::new(vec![told], (100, 30), Tokens::on(None), &options);
  model.settle();
  model.thread = Some(Opened { key, asked: [(1, None)].into(), ..Opened::default() });
  assert!(model.answered(0, 1, Ok(end)));
  model
}

/// The text of each row of the left side.
fn texts(model: &Model) -> Vec<String> {
  let left = model.left();
  left.rows.iter().map(|row| row.iter().map(|one| one.content.as_ref()).collect()).collect()
}

#[test]
fn a_page_of_older_exchanges_keeps_each_row_that_the_operator_reads_where_it_stands() {
  let mut model = opened(page(10..30));
  // He reads away from the end, until less than a screen of the feed stands above him: the window
  // asks the page before the first exchange that it holds.
  let mut asked = None;
  while asked.is_none() {
    asked = model.scroll_feed(-3);
  }
  let Some(Verb::Thread { before, limit, .. }) = asked else { panic!("a page of the thread") };
  assert_eq!((before, limit), (Some(10), 20));
  let read = texts(&model);
  assert!(!read.iter().any(|row| row.contains("earlier messages")), "{read:#?}");

  model.thread.as_mut().expect("a thread is open").asked.insert(2, Some(10));
  assert!(model.answered(0, 2, Ok(page(0..10))));
  assert_eq!(texts(&model), read);
}

#[test]
fn a_frame_makes_the_rows_of_the_exchanges_that_show_and_of_no_other() {
  // A thread of thousands of exchanges, which the window holds whole, each of four rows.
  let held: BTreeMap<usize, usize> = (0..5000).map(|at| (at, at)).collect();
  let items = Items { held: &held, head: false };
  let made = Cell::new(0);
  let rows_of = |_: Item| {
    made.set(made.get() + 1);
    vec![Row::default(); 4]
  };
  // At the end: the five last exchanges fill twenty rows.
  let end = items.shown(None, 20, rows_of);
  assert_eq!((end.rows.len(), end.end, made.replace(0)), (20, true, 5));
  // In the middle, from the third row of an exchange: that exchange and the five after it.
  let middle = items.shown(Some(Anchor { at: 2500, row: 2 }), 20, rows_of);
  assert_eq!((middle.rows.len(), middle.end, made.replace(0)), (20, false, 6));
  // Two rows above an exchange: the exchange before it, and the five from it.
  let above = items.shown(Some(Anchor { at: 2500, row: -2 }), 20, rows_of);
  let top = above.top.expect("the top stands on an exchange");
  assert_eq!((top, made.replace(0)), (Anchor { at: 2499, row: 2 }, 6));
}
