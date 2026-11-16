//! The one write of a file, where no door reaches it: a write that fails or waits midway, which
//! only the body of [`filled`] holds open. The suite proves the rest through a life that writes.

use std::{fs, io::Write};

use super::{Access, filled, write};
use crate::testing::Yard;
use std::io;
use std::path;
use std::process;

/// An empty folder of the test, named with the id of the process, as every yard of the tests of
/// the crate is.
fn yard(name: &str) -> Yard {
  Yard::new(format!("furb-disk-{name}-{}", process::id()))
}

fn names(folder: &path::Path) -> Vec<String> {
  let mut found: Vec<_> = fs::read_dir(folder)
    .expect("the folder reads")
    .map(|one| one.expect("an entry").file_name().to_string_lossy().into_owned())
    .collect();
  found.sort();
  found
}

#[test]
fn a_write_that_fails_midway_as_on_a_full_disk_leaves_the_old_file_whole_and_no_part_of_a_new_one()
{
  for access in [Access::Shared, Access::Owner] {
    let at = yard("midway");
    fs::write(at.join("a.txt"), "old\n").unwrap();
    let torn = filled(&at.join("a.txt"), access, |file| {
      file.write_all(b"new ")?;
      Err(io::Error::other("the disk is full"))
    });
    assert_eq!(torn.unwrap_err().source.to_string(), "the disk is full");
    let full = |_: &mut fs::File| Err(io::Error::other("the disk is full"));
    assert!(filled(&at.join("b.png"), access, full).is_err(), "a new file fails too");
    assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "old\n");
    assert_eq!(names(&at), ["a.txt"], "no part file stays, and no part of a new file");
  }
}

#[test]
fn a_write_leaves_the_part_file_of_a_write_of_the_same_target_that_goes_on() {
  let at = yard("goes-on");
  let file = at.join("a.txt");
  filled(&file, Access::Shared, |part| {
    write(&file, b"second", Access::Shared).expect("a second write lands");
    let seen = names(&at);
    assert_eq!(seen.len(), 2, "the part of the first write stays beside the target: {seen:?}");
    part.write_all(b"first")
  })
  .expect("the first write lands");
  assert_eq!(fs::read_to_string(&file).unwrap(), "first");
  assert_eq!(names(&at), ["a.txt"]);
}
