//! The words in which a person reads a time, a size, a count and a path, as each client of
//! furb-wire shows them.

use std::time::Duration;

use furb_wire::words::{age, elapsed, grouped, homed, since, size, timer, tokens};

#[test]
fn a_count_groups_its_digits_by_three_so_the_counts_of_a_column_align() {
  let counts = [0, 999, 1000, 141_208, 1_191_908].map(grouped);
  assert_eq!(counts, ["0", "999", "1 000", "141 208", "1 191 908"].map(str::to_owned));
}

#[test]
fn a_size_is_rounded_in_the_unit_that_keeps_it_under_a_thousand() {
  let sizes = [512, 12_288, 83_996, 999_600, 753_000_000, 1_200_000_000].map(size);
  assert_eq!(sizes, ["512 B", "12 KB", "84 KB", "1 MB", "753 MB", "1 GB"].map(str::to_owned));
}

#[test]
fn a_time_of_an_open_counts_tenths_under_ten_seconds() {
  let times = [400, 9_960, 12_000, 124_000].map(|ms| elapsed(Duration::from_millis(ms)));
  assert_eq!(times, ["0.4 s", "9.9 s", "12 s", "2m 04s"].map(str::to_owned));
}

#[test]
fn a_count_of_tokens_is_cut_and_never_shows_more_than_it_is() {
  let counts = [840, 8_499, 8_000, 124_999, 1_250_000, 1_000_000].map(tokens);
  assert_eq!(counts, ["840", "8.4k", "8k", "124k", "1.2M", "1M"]);
}

#[test]
fn a_timer_shows_seconds_then_minutes_then_hours_with_two_digits_after_the_first() {
  let times = [4, 65, 7_380].map(timer);
  assert_eq!(times, ["4 s", "1m 05s", "2h 03m"].map(str::to_owned));
}

#[test]
fn a_time_of_a_work_shows_from_one_whole_second_and_an_age_from_one_minute() {
  assert_eq!([since(1_000, 1_999), since(1_000, 2_000)], [None, Some(1)]);
  let ages = [59, 300, 10_800, 172_800].map(|ago| age(1_000_000 - ago, 1_000_000));
  assert_eq!(ages, [None, Some("5m".to_owned()), Some("3h".to_owned()), Some("2d".to_owned())]);
}

#[test]
fn a_path_of_the_home_folder_reads_with_a_tilde() {
  let home = "/Users/a";
  assert_eq!([homed("/Users/a", home), homed("in /Users/a/src", home)], ["~", "in ~/src"]);
  assert_eq!(homed(r"in C:\Users\a\src", r"C:\Users\a"), r"in ~\src");
}
