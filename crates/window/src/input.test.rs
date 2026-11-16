use std::path::PathBuf;

use furb::view::{Exchange, Maker};
use furb_client::{Opened, Thread, Told};
use furb_host::{ProtocolError, protocol::Frame};
use jiff::tz::TimeZone;

use super::Word;
use crate::{Options, keys::System, model::Model, tokens::Tokens};

/// A window with one machine, which runs the word `x = 1` as `rung3` of the session b.
fn running() -> Model {
  let options = Options {
    folder: PathBuf::from("/furb"),
    home: None,
    kept: PathBuf::from("/furb/cards.json"),
    zone: TimeZone::UTC,
    clock: || 0,
    still: true,
    system: System::MacOs,
    browse: |_| Ok(()),
    program: None,
    reloaded: false,
    tailnet: None,
  };
  let mut model = Model::new(vec![Told::default()], (120, 40), Tokens::on(None), &options);
  model.word = Some(word());
  model
}

fn word() -> Word {
  let (life, text, rung) = ("/furb/b.jsonl", "x = 1", "rung3");
  Word {
    at: 0,
    life: life.to_owned(),
    text: text.to_owned(),
    rung: rung.to_owned(),
    asked: true,
    ran: None,
  }
}

/// Each life names its rungs alike, so the end of a word of one life never reaches the word of
/// another life that holds the same name.
#[test]
fn the_end_of_a_word_reaches_only_the_word_of_its_own_life() {
  let mut model = running();
  let refused = |life: &str| {
    let error = ProtocolError::Closed { life: life.to_owned() };
    Ok(Frame::Refused { id: Some(7), error, why: "The life ended.".to_owned() })
  };
  // The end of rung3 of the session a, which the window ran before, comes after it ran rung3 of b.
  model.ended(0, ("/furb/a.jsonl", "rung3"), refused("/furb/a.jsonl"));
  assert_eq!((model.word.clone(), model.fault.as_deref()), (Some(word()), None));
  model.ended(0, ("/furb/b.jsonl", "rung3"), refused("/furb/b.jsonl"));
  assert_eq!((model.word.clone(), model.fault.as_deref()), (None, Some("The life ended.")));
}

/// The hints of the word that the window ran stand under the input until the open thread shows
/// it, and a thread of another life whose message holds the same name does not.
#[test]
fn the_word_of_one_life_shows_its_hints_beside_a_thread_of_another_life_of_the_same_names() {
  let mut model = running();
  let exchange = |message: &str| Exchange {
    at: 0,
    message: message.to_owned(),
    said_at: 0,
    by: Maker::Operator,
    text: "x = 1".to_owned(),
    python: true,
    runs: false,
    block: None,
    said: Vec::new(),
  };
  let opened = |life: &str| {
    let key = (0, life.to_owned(), "prompt1".to_owned());
    let exchanges = [(0, exchange("rung3"))].into();
    Opened { thread: Thread { key, exchanges, ..Thread::default() }, ..Opened::default() }
  };
  let hints = |model: &Model| {
    model.word_hints(80).0.iter().map(|one| one.content.clone()).collect::<String>()
  };
  model.client.thread = Some(opened("/furb/a.jsonl"));
  assert_eq!(hints(&model), "    x = 1");
  model.client.thread = Some(opened("/furb/b.jsonl"));
  assert_eq!(hints(&model), "");
}
