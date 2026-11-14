use std::{path::PathBuf, time::Duration};

use furb::view::{self, Block, Exchange, Maker, Page, Said, Saying, Standing, declined};
use furb_host::protocol::{Card, Verb};
use jiff::tz::TimeZone;

use super::{HOLD, Keyed};
use crate::{
  Options, System,
  keys::Does,
  model::{Link, Model, Told},
  thread::Opened,
  tokens::Tokens,
};

const LIFE: &str = "/home/ada/furb/.furb/sessions/side.jsonl";

/// A window on a system whose open thread holds one question of a shape, which waits.
fn asked(shape: &str, system: System) -> Model {
  let thread = view::Card {
    prompt: "prompt0".to_owned(),
    title: "where does the list stand".to_owned(),
    standing: Standing::Asks { of: None },
    model: None,
    agents: 0,
    readers: Vec::new(),
  };
  let card = Card {
    life: LIFE.to_owned(),
    session: None,
    project: None,
    held: true,
    viewers: Vec::new(),
    since: 0,
    thread,
  };
  let told =
    Told { name: "mac".to_owned(), login: "ada".to_owned(), cards: vec![card], ..Told::default() };
  let told = Told { link: Link::Listed, ..told };
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
  let question = Saying::Question { shape: shape.to_owned(), answered: None };
  let said = Said { act: "prompt2".to_owned(), kind: question, text: "Where?".to_owned() };
  let maker = Maker::Model { model: "claude-cli:opus".to_owned(), effort: None };
  let block = Block { maker, lines: Vec::new(), more: true, thought: None };
  let exchange = Exchange {
    at: 0,
    message: "prompt0".to_owned(),
    by: Maker::Operator,
    text: "where does the list stand".to_owned(),
    block: Some(block),
    said: vec![said],
  };
  let key = (0, LIFE.to_owned(), "prompt0".to_owned());
  model.thread = Some(Opened { key, asked: [(1, None)].into(), ..Opened::default() });
  let page = Page { exchanges: vec![exchange], above: 0, agents: Vec::new() };
  assert!(model.answered(0, 1, Ok(page)));
  model.questioned();
  model
}

/// The window once the form takes keys.
fn held(mut model: Model) -> Model {
  model.moved += HOLD;
  model
}

fn answer(text: &str) -> Keyed {
  let (life, act) = (LIFE.to_owned(), "prompt2".to_owned());
  Keyed::Says(0, Verb::Answer { life, act, line: text.to_owned() })
}

#[test]
fn a_form_takes_no_key_in_its_first_moments() {
  let mut model = asked("Literal['right', 'left']", System::MacOs);
  model.moved += HOLD.saturating_sub(Duration::from_millis(1));
  assert_eq!(model.keyed(Some(Does::Down), false), Keyed::Taken);
  assert_eq!(model.keyed(Some(Does::Send), false), Keyed::Taken);
  assert_eq!(model.choose(1), None);
  assert_eq!(model.form().map(|(form, _)| form.at), Some(0));
  // A key of the window that is no key of the form does its thing.
  assert_eq!(model.keyed(Some(Does::NewThread), false), Keyed::Passed);
}

#[test]
fn a_choice_answers_the_value_of_its_row_and_his_own_words_close_it_as_a_refusal() {
  let mut model = held(asked("Literal['right', 'left']", System::MacOs));
  assert_eq!(model.keyed(Some(Does::Down), false), Keyed::Taken);
  assert_eq!(model.keyed(Some(Does::Send), false), answer("left"));
  // A letter on the row of a value writes nothing; on the last row, it writes his words.
  assert_eq!(model.keyed(None, true), Keyed::Taken);
  assert_eq!(model.keyed(Some(Does::Down), false), Keyed::Taken);
  assert_eq!(model.keyed(None, true), Keyed::Passed);
  assert_eq!(model.keyed(Some(Does::Send), false), Keyed::Taken);
  model.draft.text = "at the top".to_owned();
  let close =
    Verb::Close { life: LIFE.to_owned(), act: "prompt2".to_owned(), value: declined("at the top") };
  assert_eq!(model.keyed(Some(Does::Send), false), Keyed::Says(0, close));
}

#[test]
fn values_to_tick_answer_the_list_of_those_ticked() {
  let mut model = held(asked("list[Literal['up', 'down', 'tab']]", System::MacOs));
  model.keyed(Some(Does::Tick), false);
  model.keyed(Some(Does::Down), false);
  model.keyed(Some(Does::Down), false);
  model.keyed(Some(Does::Tick), false);
  assert_eq!(model.keyed(Some(Does::Send), false), answer(r#"["up", "tab"]"#));
}

#[test]
fn ctrl_c_empties_the_words_of_a_form_then_refuses_its_question() {
  let mut model = held(asked("int", System::MacOs));
  model.draft.text = "12".to_owned();
  assert_eq!(model.keyed(Some(Does::Empty), false), Keyed::Empties);
  model.draft.text.clear();
  let value = declined(view::REFUSED);
  let close = Verb::Close { life: LIFE.to_owned(), act: "prompt2".to_owned(), value };
  assert_eq!(model.keyed(Some(Does::Empty), false), Keyed::Says(0, close));
}

#[test]
fn a_question_that_comes_while_he_writes_shows_no_form_until_the_input_is_empty() {
  let mut model = asked("bool", System::MacOs);
  model.thread.as_mut().unwrap().form = None;
  model.draft.text = "a draft".to_owned();
  model.questioned();
  assert!(model.form().is_none() && model.waiting_row().is_some());
  model.draft.text.clear();
  model.questioned();
  assert!(model.form().is_some() && model.waiting_row().is_none());
}

/// The text of the hints of a form.
fn hints(shape: &str, system: System) -> String {
  let model = asked(shape, system);
  model.form_hints().unwrap().iter().map(|one| one.content.as_ref()).collect()
}

#[test]
fn on_macos_the_hints_of_a_form_show_the_signs_of_its_keys() {
  assert_eq!(hints("bool", System::MacOs), "  ↑↓ choose · ⏎ answer");
  assert_eq!(hints("list[Literal['a']]", System::MacOs), "  ↑↓ choose · Space tick · ⏎ answer");
  assert_eq!(hints("int", System::MacOs), "  ⏎ answer");
}

#[test]
fn on_another_system_the_hints_of_a_form_show_its_keys_as_words() {
  assert_eq!(hints("bool", System::Other), "  ↑↓ choose · Enter answer");
  assert_eq!(hints("list[Literal['a']]", System::Other), "  ↑↓ choose · Space tick · Enter answer");
  assert_eq!(hints("int", System::Other), "  Enter answer");
}
