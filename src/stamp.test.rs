//! The stamp of a dump, read back as it was written.

use super::{Stamp, Tally};
use crate::Object;

/// A tally of one entry: the fact of a done of this act.
fn done(about: &str) -> Tally {
  let mut tally = Tally::default();
  let fact =
    Object::tuple([Object::string("done"), Object::string(about), Object::string("world")]);
  tally.kept(Object::tuple([fact]).as_ref());
  tally
}

#[test]
fn a_stamp_reads_back_as_it_was_written_and_the_session_after_it_stands_whole() {
  let session = b"MONTY\0\n\nthe rest".to_vec();
  let bytes = Stamp::now(done("prompt1")).stamped(&session).unwrap();
  let (stamp, rest) = Stamp::read(&bytes).unwrap();
  assert_eq!(rest, session.as_slice());
  assert!(stamp.differs(&Stamp::now(done("prompt1"))).is_empty());
  let empty = Stamp::now(Tally::default()).stamped(b"").unwrap();
  assert!(Stamp::read(&empty).unwrap().0.differs(&Stamp::now(Tally::default())).is_empty());
}

#[test]
fn a_stamp_of_another_record_says_both_records() {
  let bytes = Stamp::now(done("prompt1")).stamped(b"").unwrap();
  let (stamp, _) = Stamp::read(&bytes).unwrap();
  let mut two = done("prompt1");
  two.kept(
    Object::tuple([Object::tuple([Object::string("done"), Object::string("prompt2")])]).as_ref(),
  );
  assert_eq!(
    stamp.differs(&Stamp::now(two)),
    [
      "it is of a record of 1 entry, done prompt1, and the record holds 2 entries that end with done prompt2"
    ]
  );
  assert_eq!(
    stamp.differs(&Stamp::now(Tally::default())),
    ["it is of a record of 1 entry, done prompt1, and the record holds no entry"]
  );
}

#[test]
fn what_is_no_stamp_is_no_dump_of_a_life() {
  let no = |bytes: &[u8]| Stamp::read(bytes).unwrap_err().message();
  assert_eq!(no(b"MONTY\0"), "no dump of a life: it holds no stamp");
  assert_eq!(no(b"a dump\n\n"), "no dump of a life: it does not begin with the mark of a dump");
  assert_eq!(no(b"furb dump\nengine 0\n\n"), "no dump of a life: its stamp says no build");
  let counted = b"furb dump\nengine 0\nbuild 0\nentries many\nkind \nabout \n\n";
  assert_eq!(no(counted), "no dump of a life: its stamp counts no entries");
  assert_eq!(no(b"furb dump\xff\n\n"), "no dump of a life: its stamp is no text");
}

#[test]
fn a_record_whose_last_entry_names_a_line_break_takes_no_stamp() {
  let no = Stamp::now(done("prompt\n1")).stamped(b"").unwrap_err();
  assert_eq!(no.message(), "the last entry of the record names a line break, which no stamp holds");
}
