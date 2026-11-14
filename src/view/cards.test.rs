use super::{Card, Part, Standing};

fn card(standing: Standing, readers: &[&str]) -> Card {
  Card {
    prompt: "prompt1".to_owned(),
    title: "a thread".to_owned(),
    standing,
    model: None,
    agents: 0,
    readers: readers.iter().map(|one| (*one).to_owned()).collect(),
  }
}

#[test]
fn a_card_stands_in_the_part_that_its_standing_and_its_reader_give() {
  let asks = |of: Option<&str>| Standing::Asks { of: of.map(str::to_owned) };
  let paused = |by: &str| Standing::Paused { by: by.to_owned() };
  let parts = [
    (card(asks(None), &[]), (Part::NeedsYou, 0)),
    (card(asks(Some("ada")), &[]), (Part::NeedsYou, 0)),
    // A question to another member waits for that member alone.
    (card(asks(Some("bob")), &[]), (Part::Resting, 0)),
    (card(Standing::Failed, &[]), (Part::NeedsYou, 1)),
    (card(Standing::Failed, &["operator/ada"]), (Part::Resting, 0)),
    (card(paused("provider"), &[]), (Part::NeedsYou, 1)),
    (card(paused("operator/ada"), &[]), (Part::Resting, 0)),
    (card(Standing::Thinks, &[]), (Part::Working, 0)),
    (card(Standing::Runs, &[]), (Part::Working, 0)),
    (card(Standing::Delegates, &[]), (Part::Working, 0)),
    (card(Standing::Stopped, &[]), (Part::Resting, 0)),
    (card(Standing::Rests, &[]), (Part::Resting, 0)),
    // An end that he did not read rests, with its mark.
    (card(Standing::Resolved, &[]), (Part::Resting, 0)),
    // What another member read is not read, and what the operator with no login read is.
    (card(Standing::Resolved, &["operator/bob"]), (Part::Resting, 0)),
    (card(Standing::Resolved, &["operator/ada"]), (Part::Resolved, 0)),
    (card(Standing::Resolved, &["operator"]), (Part::Resolved, 0)),
  ];
  for (card, part) in parts {
    assert_eq!(card.part("ada"), part, "{card:?}");
  }
}

#[test]
fn a_thread_that_worked_rests_once_no_life_holds_its_record() {
  let stood =
    [Standing::Thinks, Standing::Runs, Standing::Delegates, Standing::Failed, Standing::Resolved];
  let rests = stood.map(|standing| card(standing, &[]).at_rest().standing);
  let wanted =
    [Standing::Rests, Standing::Rests, Standing::Rests, Standing::Failed, Standing::Resolved];
  assert_eq!(rests, wanted);
  // No agent works there either.
  let delegates = Card { agents: 3, ..card(Standing::Delegates, &[]) };
  assert_eq!(delegates.at_rest().agents, 0);
}

#[test]
fn a_message_on_an_open_thread_is_a_notice_and_on_an_ended_thread_a_question() {
  let shapes = [Standing::Thinks, Standing::Rests, Standing::Resolved, Standing::Stopped]
    .map(|standing| card(standing, &[]).reply());
  assert_eq!(shapes, ["None", "None", "str", "str"]);
}
