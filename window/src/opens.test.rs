use std::{path::PathBuf, time::Duration};

use furb::view::{self, Standing};
use furb_host::protocol::Card;
use jiff::tz::TimeZone;

use crate::{
  Options, System,
  model::{Link, Model, Told},
  tokens::Tokens,
};

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
    pins: Vec::new(),
    since: 0,
    thread,
  };
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

#[test]
fn a_page_refused_while_its_life_opens_ends_the_open_on_its_card_too() {
  let life = "/home/ada/furb/.furb/sessions/gate.jsonl".to_owned();
  let options = Options {
    folder: PathBuf::from("/home/ada/furb"),
    home: Some(PathBuf::from("/home/ada")),
    kept: PathBuf::new(),
    zone: TimeZone::UTC,
    clock: || 0,
    still: false,
    system: System::MacOs,
  };
  let told = Told { name: "mac".to_owned(), link: Link::Listed, ..Told::default() };
  let mut model = Model::new(vec![told], (100, 30), Tokens::on(None), &options);
  model.selected = Some((0, life.clone(), "prompt0".to_owned()));
  model.open_thread();
  model.moved = Duration::from_secs(1);
  assert!(model.shown_open(0, &life).is_some(), "the card says that the life opens");
  model.thread.as_mut().expect("the thread opens").asked.insert(7, None);
  let why = "Refused: Another process owns the record.".to_owned();
  assert!(model.answered(0, 7, Err(why)));
  assert!(model.shown_open(0, &life).is_none(), "the card says no open once the open failed");
}
