use std::{cell::Cell, collections::BTreeMap};

use super::{Anchor, Item, Items, Row};

/// A speed law that no frame shows, so the test counts the work: the rows of a frame cost the
/// exchanges that show, whatever the length of the thread.
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
