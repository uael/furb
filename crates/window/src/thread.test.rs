use std::{collections::BTreeMap, rc::Rc};

use super::{Item, Items, Place, Row};

/// A speed law that no frame shows, so the test counts the work: the rows of a frame cost the
/// exchanges that show, whatever the length of the thread.
#[test]
fn a_frame_makes_the_rows_of_the_exchanges_that_show_and_of_no_other() {
  // A thread of thousands of exchanges, which the window holds whole, each of four rows.
  let held: BTreeMap<usize, usize> = (0..5000).map(|at| (at, at)).collect();
  let items = Items { held: &held, head: false };
  // The rows that the window shows from a place, with how many exchanges it made the rows of.
  let shown = |place: Option<Place>| {
    let mut made = 0;
    let shown = items.shown(place, 20, |_: Item| {
      made += 1;
      (0..4).map(|_| Rc::new(Row::default())).collect()
    });
    (shown, made)
  };
  // At the end: the five last exchanges fill twenty rows.
  let (end, made) = shown(None);
  assert_eq!((end.rows.len(), end.end, made), (20, true, 5));
  // In the middle, from the third row of an exchange: that exchange and the five after it.
  let (middle, made) = shown(Some(Place { at: 2500, row: 2 }));
  assert_eq!((middle.rows.len(), middle.end, made), (20, false, 6));
  // Two rows above an exchange: the exchange before it, and the five from it.
  let (above, made) = shown(Some(Place { at: 2500, row: -2 }));
  let top = above.top.expect("the top stands on an exchange");
  assert_eq!((top, made), (Place { at: 2499, row: 2 }, 6));
}
