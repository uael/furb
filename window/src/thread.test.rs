use std::{cell::Cell, collections::BTreeMap, ops::Range, path::PathBuf};

use furb::view::{self, Block, Exchange, Line as Code, Maker, Mark, Page, Standing, Thought, Work};
use furb_host::protocol::{Card, Verb};
use jiff::tz::TimeZone;

use super::{Anchor, Item, Items, Opened, Row};
use furb_client::Thread;

use crate::{
  Options, System,
  model::{Link, Model, Told},
  tokens::{Loader, Tokens},
};

/// One exchange of a thread: a message, and a block of two lines that ran.
fn exchange(at: usize) -> Exchange {
  let line = |number: usize| Code {
    number,
    text: format!("step = {number}"),
    mark: Mark::Ran,
    output: None,
  };
  let maker = Maker::Model { model: "claude-cli:opus".to_owned(), effort: None };
  let block =
    Block { maker, lines: vec![line(2 * at + 1), line(2 * at + 2)], more: false, thought: None };
  Exchange {
    at,
    message: format!("prompt{at}"),
    by: Maker::Operator,
    python: false,
    runs: false,
    text: format!("and the step {at}"),
    block: Some(block),
    said: Vec::new(),
  }
}

/// The page of the exchanges of a thread from one place to another.
fn page(places: Range<usize>) -> Page {
  Page { above: places.start, exchanges: places.map(exchange).collect(), ..Page::default() }
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
    refused: None,
    held: true,
    viewers: Vec::new(),
    pins: Vec::new(),
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
    still: false,
    system: System::MacOs,
    browse: |_| Ok(()),
  };
  let mut model = Model::new(vec![told], (100, 30), Tokens::on(None), &options);
  model.settle();
  let thread = Thread { key, asked: [(1, None)].into(), ..Thread::default() };
  model.client.thread = Some(Opened { thread, ..Opened::default() });
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

  model.client.thread.as_mut().expect("a thread is open").thread.asked.insert(2, Some(10));
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

#[test]
fn a_thread_that_ends_twice_in_one_second_is_said_seen_at_each_end_and_reads_at_once() {
  let mut model = opened(page(0..3));
  let standing = |model: &mut Model, standing: Standing| {
    model.client.machines[0].cards[0].thread.standing = standing;
  };
  for _ in 0..2 {
    standing(&mut model, Standing::Resolved);
    let said = model.client.seen_due();
    assert!(matches!(said, Some((0, Verb::Seen { .. }))), "{said:?}");
    // The card shows him as a reader before the daemon says so, and says nothing twice.
    assert!(!model.client.machines[0].cards[0].thread.unread("ada"));
    assert_eq!(model.client.seen_due(), None);
    // A reply makes the thread work again, and its next end falls in the same second.
    standing(&mut model, Standing::Thinks);
    model.client.machines[0].cards[0].thread.readers.clear();
    assert_eq!(model.client.seen_due(), None);
  }
}

#[test]
fn a_light_passes_over_the_word_of_a_model_that_writes_or_thinks_and_not_once_it_thought() {
  let writes = Code {
    number: 3,
    text: String::new(),
    mark: Mark::Runs { work: Work::Writes, began: None },
    output: None,
  };
  let maker = Maker::Model { model: "claude-cli:opus".to_owned(), effort: None };
  let thought = Thought { text: "the seed".to_owned(), began: 0, took: None };
  let block = Block { maker, lines: vec![writes], more: false, thought: Some(thought) };
  let mut end = page(0..1);
  end.exchanges[0].block = Some(block);
  let mut model = opened(end);
  let lights = |model: &Model| -> Vec<(String, &'static str)> {
    let left = model.left();
    let lit = left.loaders.iter().filter_map(|spot| match spot.loader {
      Loader::Light(word) => Some((texts(model)[usize::from(spot.y)].clone(), word)),
      _ => None,
    });
    lit
      .map(|(row, word)| (row.split_whitespace().next().unwrap_or_default().to_owned(), word))
      .collect()
  };
  assert_eq!(lights(&model), [("writes".to_owned(), "writes"), ("thinks".to_owned(), "thinks")]);
  let opened = model.client.thread.as_mut().expect("a thread is open");
  let exchange = opened.thread.exchanges.get_mut(&0).expect("the exchange");
  exchange.block.as_mut().and_then(|one| one.thought.as_mut()).expect("a thought").took = Some(3);
  assert_eq!(lights(&model), [("writes".to_owned(), "writes")]);
}
