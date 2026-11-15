//! The tests of the watcher of memory.

use super::counted;

#[test]
fn the_census_names_the_larger_test_process_of_the_user_and_counts_the_work() {
  let table = "501 10 3072000 bun test ./big.test.ts\n\
               501 11 1024000 bun test ./small.test.ts\n\
               501 12 9000000 /Applications/Ghostty.app/ghostty\n\
               0 13 9000000 bun test ./root.test.ts\n\
               501 14 512000 cargo build /work/tree\n";
  let census = counted("501", table, &["/work/tree".to_owned()]).unwrap();
  let largest = Some(("10".to_owned(), 3_072_000, "bun test ./big.test.ts".to_owned()));
  assert_eq!(census.largest, largest);
  assert_eq!(census.work, 3_072_000 + 1_024_000 + 512_000);
}
