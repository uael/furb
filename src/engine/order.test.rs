//! The list of the texts whose replay this build orders, against the texts that the records of the
//! corpus hold and the text of this build: no door gives the text that the sandbox of a segment
//! runs, so only a test of the module sees which text it rewrites.

use std::{fs, path::Path};

use sha2::{Digest, Sha256};

use super::{CARRIER, CARRIERS, FORMS, TEXTS, carrier, ordered};
use crate::CHECKPOINTER;

/// The text of the engine that each fact made of each record of the corpus holds.
fn recorded() -> Vec<String> {
  held(7)
}

/// The text at a place of the words of each fact made of each record of the corpus.
fn held(at: usize) -> Vec<String> {
  let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
  let mut texts = Vec::new();
  for folder in ["record-3", "next"] {
    for file in fs::read_dir(corpus.join(folder)).expect("the corpus holds the folder") {
      let path = file.expect("the folder lists its files").path();
      if path.extension().is_none_or(|one| one != "jsonl") {
        continue;
      }
      for line in fs::read_to_string(&path).expect("the record reads").lines() {
        let entry: serde_json::Value = serde_json::from_str(line).expect("each line is JSON");
        if entry[0][0] == "made"
          && let Some(text) = entry[0][at].as_str()
        {
          texts.push(text.to_owned());
        }
      }
    }
  }
  texts
}

fn digest(text: &str) -> String {
  format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[test]
fn each_listed_text_of_the_corpus_runs_the_dispatch_of_this_build_and_each_other_text_runs_as_it_is()
 {
  let texts = recorded();
  assert!(!texts.is_empty(), "the corpus holds texts of engines");
  for text in texts.into_iter().chain([crate::ENGINE.to_owned()]) {
    let listed = TEXTS.iter().find(|(one, _)| *one == digest(&text));
    let got = ordered(text.clone()).expect("a listed text holds its form");
    match listed {
      Some((_, form)) => {
        let (old, new) = FORMS[*form];
        assert!(got.contains(new) && !got.contains(old), "{} is ordered", digest(&text));
      }
      None => assert_eq!(got, text, "{} runs as it is", digest(&text)),
    }
  }
}

#[test]
fn a_text_that_holds_an_old_dispatch_and_is_not_listed_runs_as_it_is() {
  let (old, new) = FORMS[1];
  // The engine before this build, with a line more, as a build that no list holds made it.
  let planted = crate::ENGINE.replacen(new, old, 1) + "\n";
  assert_ne!(
    planted,
    crate::ENGINE,
    "the engine of this build holds the dispatch that it orders to"
  );
  assert_eq!(ordered(planted.clone()).expect("an unlisted text is no fault"), planted);
}

#[test]
fn each_listed_digest_is_listed_once_and_names_a_form() {
  for (at, (one, form)) in TEXTS.iter().enumerate() {
    assert!(*form < FORMS.len(), "{one} names a form");
    assert!(TEXTS.iter().skip(at + 1).all(|(other, _)| other != one), "{one} is listed once");
  }
}

#[test]
fn each_listed_checkpointer_of_the_corpus_runs_as_the_carrier_and_each_other_as_it_is() {
  let texts = held(9);
  assert!(
    texts.iter().any(|one| CARRIERS.contains(&digest(one).as_str())),
    "the corpus holds a listed checkpointer"
  );
  for text in texts.iter().map(String::as_str).chain([CHECKPOINTER]) {
    let listed = CARRIERS.contains(&digest(text).as_str());
    assert_eq!(carrier(text), if listed { CARRIER } else { text }, "{}", digest(text));
  }
}

#[test]
fn the_carrier_is_the_checkpointer_that_each_build_ran_in_place_of_a_listed_one() {
  // Each build from the list on held this checkpointer, so each record that a build seeded in
  // place of a listed one replays on it.
  assert_eq!(
    digest(CARRIER),
    "47d2f5f2357b74941078e28310ef9be4e7aa57a2f024ac526cce2d2411ba5b4c",
    "the carrier never changes"
  );
}
