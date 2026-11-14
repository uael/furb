use std::path::PathBuf;

use furb::view::{self, Part, Standing};
use furb_host::protocol::{Answer, Card, Frame, Kind, PROTOCOL, Peer, Role};
use jiff::tz::TimeZone;

use super::{Link, Model, Told};
use crate::{Options, System, input::Asked, list::Row, opens::Ask, tokens::Tokens};

fn card(life: &str, title: &str, standing: Standing, since: u64) -> Card {
  let thread = view::Card {
    prompt: "prompt1".to_owned(),
    title: title.to_owned(),
    standing,
    model: Some("claude-cli:opus".to_owned()),
    agents: 0,
    readers: vec!["operator".to_owned()],
  };
  Card {
    life: life.to_owned(),
    session: None,
    project: None,
    held: true,
    viewers: Vec::new(),
    since,
    thread,
  }
}

/// A model of a window of 100 columns on one machine that told its cards.
fn model(cards: Vec<Card>, rows: u16) -> Model {
  let told = Told {
    name: "mac".to_owned(),
    login: "ada".to_owned(),
    cards,
    link: Link::Listed,
    ..Told::default()
  };
  let folder = PathBuf::from("/home/ada/furb");
  let options = Options {
    folder,
    home: Some(PathBuf::from("/home/ada")),
    kept: PathBuf::new(),
    zone: TimeZone::UTC,
    clock: || 0,
    system: System::MacOs,
  };
  let mut model = Model::new(vec![told], (100, rows), Tokens::on(None), &options);
  model.settle();
  model
}

fn titles(model: &Model) -> Vec<(Part, &str)> {
  model.settled.iter().map(|one| (one.0, one.2.thread.title.as_str())).collect()
}

#[test]
fn a_card_shows_its_new_state_in_its_place_until_the_list_settles() {
  let cards = vec![card("a", "one", Standing::Thinks, 5), card("b", "two", Standing::Rests, 9)];
  let mut model = model(cards, 30);
  assert_eq!(titles(&model), [(Part::Working, "one"), (Part::Resting, "two")]);
  let asks = card("b", "two", Standing::Asks { of: None }, 12);
  let frame = Frame::Cards { life: "b".to_owned(), changed: vec![asks.clone()], gone: Vec::new() };
  assert!(model.hear(0, Ok(frame)));
  assert_eq!(titles(&model), [(Part::Working, "one"), (Part::Resting, "two")]);
  assert_eq!(model.settled[1].2, asks);
  model.settle();
  assert_eq!(titles(&model), [(Part::NeedsYou, "two"), (Part::Working, "one")]);
}

#[test]
fn a_machine_that_is_lost_keeps_its_cards_and_hears_no_more() {
  let mut model = model(vec![card("a", "one", Standing::Rests, 5)], 30);
  assert!(model.hear(0, Err(crate::Error::Closed)));
  assert_eq!(model.machines[0].link, Link::Lost("the daemon closed the connection".to_owned()));
  assert!(!model.hear(0, Ok(Frame::Reply { id: 1, answer: Answer::Cards(Vec::new()) })));
  assert_eq!(model.machines[0].cards.len(), 1);
}

#[test]
fn a_loss_gives_each_message_on_its_way_back_to_the_input_and_a_new_hello_hears_the_machine_again()
{
  let mut model = model(vec![card("a", "one", Standing::Rests, 5)], 30);
  model.selected = model.settled.first().map(|one| one.1.clone());
  model.open_thread();
  model.holds.values_mut().for_each(|hold| hold.ask = Ask::Answered);
  assert!(model.send("and the seed?".to_owned()));
  let (at, _) = model.due().unwrap();
  model.asked.insert((at, 7), Asked::Say);
  assert!(model.hear(0, Err(crate::Error::Closed)));
  assert_eq!(model.restore.as_deref(), Some("and the seed?"));
  let why = "mac went away before it took your message. If the thread shows it, it went.";
  assert_eq!((model.fault.as_deref(), model.asked.len()), (Some(why), 0));
  assert!(model.holds.values().all(|hold| hold.ask == Ask::Due));
  // Nothing goes while the machine is away: the text stays in the input.
  model.draft.text = "and the seed?".to_owned();
  assert!(!model.send("and the seed?".to_owned()));
  assert!(model.outgoing.is_empty());
  // A new connection says its hello, then its cards.
  let peer = Peer { login: "ada".to_owned(), role: Role::Write, kind: Kind::Window, name: None };
  let (machine, build) = ("mac".to_owned(), String::new());
  let welcome = Answer::Welcome { protocol: PROTOCOL, peer, machine, build, view: 0 };
  assert!(model.hear(0, Ok(Frame::Reply { id: 8, answer: welcome })));
  assert_eq!(model.machines[0].link, Link::Dialing);
  assert!(model.hear(0, Ok(Frame::Reply { id: 9, answer: Answer::Cards(Vec::new()) })));
  assert_eq!(model.machines[0].link, Link::Listed);
}

#[test]
fn the_keys_step_through_the_cards_and_to_each_thread_that_needs_the_operator() {
  let cards = vec![
    card("a", "asks", Standing::Asks { of: None }, 1),
    card("b", "failed", Standing::Failed, 2),
    card("c", "works", Standing::Thinks, 3),
    card("d", "done", Standing::Resolved, 4),
  ];
  let mut model = model(cards, 30);
  // The operator read each thread, so the one that failed rests.
  assert_eq!(
    titles(&model),
    [
      (Part::NeedsYou, "asks"),
      (Part::Working, "works"),
      (Part::Resting, "failed"),
      (Part::Resolved, "done")
    ]
  );
  let selected = |model: &Model| model.selected.clone().map(|key| key.1);
  model.step(1);
  model.step(1);
  assert_eq!(selected(&model).as_deref(), Some("c"));
  // The resolved threads are folded, so the last card of the keys is the one before them.
  model.step(5);
  assert_eq!(selected(&model).as_deref(), Some("b"));
  model.needed();
  assert_eq!(selected(&model).as_deref(), Some("a"));
  // A click on the head of the resolved threads shows them, and a click on a card selects it.
  let head = model.rows().iter().position(|row| *row == Row::Head(Part::Resolved)).unwrap();
  model.clicked(head);
  assert!(model.open);
  model.clicked(head + 2);
  assert_eq!(selected(&model).as_deref(), Some("d"));
  // A step past the last card above the folded resolved threads shows them, and goes to the first,
  // as a click on their head does.
  model.open = false;
  model.step(-1);
  assert_eq!(selected(&model).as_deref(), Some("b"));
  model.step(1);
  assert!(model.open);
  assert_eq!(selected(&model).as_deref(), Some("d"));
}

#[test]
fn the_window_writes_the_rows_that_show_whatever_the_count_of_the_cards() {
  let cards = (0..600)
    .map(|n| card(&format!("life{n:03}"), &format!("thread {n:03}"), Standing::Rests, 600 - n));
  let mut model = model(cards.collect(), 30);
  assert_eq!(model.rows().len(), 2 + 1 + 600 * 3 + 1);
  assert_eq!(model.list().len(), 30);
  // The selected card stays in the rows that show, with the row above it and the row under it.
  for _ in 0..40 {
    model.step(1);
  }
  let shown: Vec<Row> = model.shown().collect();
  assert_eq!(shown.len(), 30);
  assert_eq!(
    shown[26..],
    [
      Row::Gap(Some(38), Some(39)),
      Row::Card(39, false),
      Row::Card(39, true),
      Row::Gap(Some(39), Some(40))
    ]
  );
  model.scrolled(-10_000);
  assert_eq!(model.shown().nth(2), Some(Row::Head(Part::Resting)));
}

/// The folder that a window in a folder shows, for a home.
fn shown(folder: &str, home: &str) -> String {
  let options = Options {
    folder: PathBuf::from(folder),
    home: Some(PathBuf::from(home)),
    kept: PathBuf::new(),
    zone: TimeZone::UTC,
    clock: || 0,
    system: System::MacOs,
  };
  Model::new(Vec::new(), (100, 30), Tokens::on(None), &options).folder_shown()
}

#[test]
fn a_folder_of_the_home_shows_as_a_path_of_the_home_with_a_slash_on_each_system() {
  assert_eq!(shown("/home/ada/src/furb", "/home/ada"), "~/src/furb");
  assert_eq!(shown(r"C:\Users\ada\src\furb", r"C:\Users\ada"), "~/src/furb");
  assert_eq!(shown(r"C:\Users\ada", r"C:\Users\ada"), "~");
  assert_eq!(shown(r"C:\Users\adam\furb", r"C:\Users\ada"), r"C:\Users\adam\furb");
  assert_eq!(shown("/srv/furb", "/home/ada"), "/srv/furb");
}
