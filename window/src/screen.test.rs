use std::{cell::RefCell, rc::Rc};

use super::Screen;

#[test]
fn a_part_draws_its_first_value_then_only_a_value_that_differs() {
  let drawn = Rc::new(RefCell::new(Vec::new()));
  let mut screen = Screen::new((0, 0));
  let seen = Rc::clone(&drawn);
  screen.part(|model: &(u8, u8)| model.0, move |shown| seen.borrow_mut().push(*shown));
  assert_eq!(*drawn.borrow(), [0]);
  // The second field is a change that the part does not show.
  screen.tell(|model| model.1 = 3);
  screen.tell(|model| model.0 = 1);
  assert_eq!(*drawn.borrow(), [0, 1]);
}
