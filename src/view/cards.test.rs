use serde_json::json;

use crate::view::{Card, Fold, Held, Part, Skill, Standing};

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

#[test]
fn the_skills_of_a_life_are_those_that_the_last_read_of_the_index_on_its_root_told() {
  let index = |names: &[&str]| {
    let lines = names.iter().map(|name| format!("{name} (code, on): Does {name}"));
    ["skill(name) loads a skill.".to_owned()]
      .into_iter()
      .chain(lines)
      .collect::<Vec<_>>()
      .join("\n")
  };
  let told = |id: &str, text: String| json!(["done", id, "skills", {"is": "Text", "path": "skills://", "content": text}]);
  let facts = [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["chain", "chain2", "chain1", "chain1", "boot", ""]),
    json!(["read", "read1", "rung1", "chain1", "skills://"]),
    told("read1", index(&["memory"])),
    json!(["read", "read2", "rung2", "chain1", "skills://"]),
    told("read2", index(&["memory", "web"])),
    // A read of another chain, which stands in another folder, and a read of one skill.
    json!(["read", "read3", "rung3", "chain2", "skills://"]),
    told("read3", index(&["boot"])),
    json!(["read", "read4", "rung4", "chain1", "skills://web"]),
    json!(["done", "read4", "skills", {"is": "Text", "path": "skills://web", "content": "def web(): ..."}]),
  ];
  let (mut fold, mut held) = (Fold::default(), Held::default());
  assert_eq!(held.skills(), []);
  for fact in facts {
    for act in fold.recorded(&fact) {
      held.heard(&act);
    }
  }
  let skill = |name: &str| Skill { name: name.to_owned(), description: format!("Does {name}") };
  assert_eq!(held.skills(), [skill("memory"), skill("web")]);
}
