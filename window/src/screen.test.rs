use std::{cell::RefCell, rc::Rc};

use furb_host::{
  ProtocolError,
  protocol::{Answer, Frame, Kind, Peer, Role},
};

use super::{Screen, Told};
use crate::Error;

#[test]
fn the_told_holds_the_welcome_then_the_count_of_the_records_and_keeps_the_first_loss() {
  let peer = Peer { login: "ada".to_owned(), role: Role::Write, kind: Kind::Window, name: None };
  let mut told = Told::default();
  told.hear(Ok(Frame::Reply { id: 0, answer: Answer::Welcome { protocol: 7, peer } }));
  assert_eq!(told, Told::Welcomed { protocol: 7, records: None });
  told.hear(Ok(Frame::Reply { id: 1, answer: Answer::Lives(Vec::new()) }));
  assert_eq!(told, Told::Welcomed { protocol: 7, records: Some(0) });
  let other = ProtocolError::Protocol { ours: 7, theirs: 8 };
  let refused = Told::Lost(other.to_string());
  told.hear(Ok(Frame::refused(Some(2), other)));
  assert_eq!(told, refused);
  told.hear(Err(Error::Closed));
  assert_eq!(told, refused);
}

#[test]
fn a_part_draws_its_first_value_then_only_a_value_that_differs() {
  let drawn = Rc::new(RefCell::new(Vec::new()));
  let mut screen = Screen::default();
  let seen = Rc::clone(&drawn);
  let welcomed = |told: &Told| matches!(told, Told::Welcomed { .. });
  screen.part(welcomed, move |shown| seen.borrow_mut().push(*shown));
  assert_eq!(*drawn.borrow(), [false]);
  // The count of the records is a change that the part does not show.
  screen.tell(|told| *told = Told::Welcomed { protocol: 1, records: None });
  screen.tell(|told| *told = Told::Welcomed { protocol: 1, records: Some(3) });
  assert_eq!(*drawn.borrow(), [false, true]);
  screen.tell(|told| *told = Told::Lost("gone".to_owned()));
  assert_eq!(*drawn.borrow(), [false, true, false]);
}
