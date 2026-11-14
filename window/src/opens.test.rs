use std::{path::PathBuf, time::Duration};

use furb::view::{self, Standing};
use furb_host::protocol::Card;
use jiff::tz::TimeZone;

use super::{elapsed, grouped, size};
use crate::{
  Options, System,
  model::{Link, Model, Told},
  tokens::Tokens,
};

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

/// A window of 100 columns and 30 rows on one machine, on the system of a client, whose open
/// thread could not open its session, and the text of each row of its left side.
fn failed(system: System) -> Vec<String> {
  let thread = view::Card {
    prompt: "prompt0".to_owned(),
    title: "remove the gate".to_owned(),
    standing: Standing::Rests,
    model: None,
    agents: 0,
    readers: Vec::new(),
  };
  let life = "/home/ada/furb/.furb/sessions/gate.jsonl".to_owned();
  let card = Card {
    life: life.clone(),
    session: None,
    project: None,
    held: false,
    viewers: Vec::new(),
    since: 0,
    thread,
  };
  let told = Told {
    name: "mac".to_owned(),
    login: "ada".to_owned(),
    heard: 0,
    cards: vec![card],
    link: Link::Listed,
  };
  let options = Options {
    folder: PathBuf::from("/home/ada/furb"),
    home: Some(PathBuf::from("/home/ada")),
    kept: PathBuf::new(),
    zone: TimeZone::UTC,
    clock: || 0,
    system,
  };
  let mut model = Model::new(vec![told], (100, 30), Tokens::on(None), &options);
  model.settle();
  model.selected = Some((0, life, "prompt0".to_owned()));
  model.open_thread();
  let opened = model.thread.as_mut().expect("the thread opens");
  opened.fault = Some(format!("Refused: Another process owns {}.", "/home/ada/furb/x.jsonl"));
  let left = model.left();
  left.rows.iter().map(|row| row.iter().map(|one| one.content.as_ref()).collect()).collect()
}

#[test]
fn a_thread_that_failed_to_open_names_the_key_that_goes_back_as_the_system_of_the_client_writes_it()
{
  for (system, key) in [(System::MacOs, "⎋"), (System::Other, "Esc")] {
    let rows = failed(system);
    assert_eq!(rows[1].trim_end(), "  Refused: Another process owns ~/furb/x.jsonl.");
    assert_eq!(rows[3].trim_end(), format!("  The record stays as it was. {key} goes back."));
    assert!(rows[29].trim_end().ends_with(&format!("{key} back")), "{rows:#?}");
  }
}
