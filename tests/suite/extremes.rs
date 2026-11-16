//! The readers of what comes from outside, each given the extreme values that a disk, a peer or an
//! older build may hold: nothing, zero, the largest and the most negative numbers, a part shorter
//! than its reader looks for, a character cut in two, bytes that are no utf-8, and nesting past
//! any bound. Each reads them with no panic: it gives a value or a fault that it names.

use std::{
  fs,
  panic::{AssertUnwindSafe, catch_unwind},
  process,
};

use furb::{
  fact::Fact,
  image, view, wire,
  world::{self, config::Layers},
};
use serde_json::Value;

use crate::yard::Yard;

/// The lines of the extremes, each a whole line of a record, a fact or a frame as a peer could
/// write it.
fn lines() -> Vec<Vec<u8>> {
  let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
  let long = format!("[[\"say\", \"chain1\", \"\", \"{}\"]]", "x".repeat(2 << 20));
  let texts = [
    "",
    "\n",
    "[]",
    "[[]]",
    "[[\"a\"]]",
    "[[\"a\", \"b\"]]",
    "[[\"done\", \"x\", \"y\"], null, 18446744073709551615]",
    "[[\"done\", \"x\", \"y\"], null, -9223372036854775808]",
    "[[\"done\", \"x\", \"y\", 1e309]]",
    "[[\"started\", \"\", \"\"], [], 0]",
    "[[1, 2, 3]]",
    "[[\"say\", \"é\", \"\\ud800\"]]",
    "{}",
    "{\"is\": 1}",
    "{\"is\": \"float\", \"args\": []}",
    "[\"\\u0000\", -0, 0.0, -1]",
    "\u{feff}[[\"a\", \"b\", \"c\"]]",
    "null",
    "-1",
    "1e309",
  ];
  let mut out: Vec<Vec<u8>> = texts.iter().map(|one| one.as_bytes().to_vec()).collect();
  out.push(deep.into_bytes());
  out.push(long.into_bytes());
  // A character cut after its first byte, and bytes that no utf-8 holds.
  out.push(b"[[\"say\", \"\xc3".to_vec());
  out.push(vec![0xff, 0xfe, 0x00, b'\n', 0xc3]);
  out
}

/// What a reader did with one extreme, named so that a failure says which.
fn read(what: &str, line: &[u8], reader: impl FnOnce()) {
  let shown = String::from_utf8_lossy(line.get(..60).unwrap_or(line)).into_owned();
  assert!(catch_unwind(AssertUnwindSafe(reader)).is_ok(), "{what} panicked on {shown:?}");
}

#[test]
fn a_record_its_log_its_image_and_its_carry_read_each_extreme_with_no_panic() {
  let yard = Yard::new(format!("furb-extremes-record-{}", process::id()));
  let record = yard.join("record.jsonl");
  for line in lines() {
    let mut bytes = line.clone();
    bytes.push(b'\n');
    for beside in ["", ".log", ".image", ".carry"] {
      fs::write(format!("{}{beside}", record.display()), &bytes).unwrap();
    }
    read("the record", &line, || drop(world::kept(&record)));
    read("the reader of a record", &line, || drop(world::retold(&record)));
    read("the log", &line, || drop(world::logged(&record)));
    read("the image", &line, || {
      image::agrees(&record);
    });
    read("the store", &line, || drop(world::store(&record)));
  }
}

#[test]
fn a_frame_a_fact_and_a_word_read_each_extreme_with_no_panic() {
  for line in lines() {
    let text = String::from_utf8_lossy(&line).into_owned();
    read("the wire", &line, || drop(wire::parsed(&text)));
    let value: Option<Value> = serde_json::from_slice(&line).ok();
    read("a fact", &line, || {
      if let Ok(object) = wire::parsed(&text) {
        drop(Fact::of(object.as_ref()));
      }
    });
    read("the fold", &line, || {
      let mut fold = view::Fold::default();
      if let Some(value) = &value {
        drop(fold.recorded(value));
        drop(fold.hear(value, &[String::new()], None));
      }
    });
    read("a word that a model writes", &line, || drop(view::writing(&text, &text)));
    read("the quotes of a word", &line, || drop(view::quotes(&text, true)));
  }
}

#[test]
fn a_config_reads_each_extreme_with_no_panic() {
  let yard = Yard::new(format!("furb-extremes-config-{}", process::id()));
  let (user, project) = (yard.join("user"), yard.join("project"));
  for folder in [&user, &project.join(".furb")] {
    fs::create_dir_all(folder).unwrap();
  }
  let mut texts = lines();
  texts.push(b"{\"skills\": {\"\": true, \"a\": 1, \"b\": \"../../..\"}}".to_vec());
  texts.push(b"{\"providers\": {\"\": {}}, \"model\": [], \"effort\": -1}".to_vec());
  for line in texts {
    for file in [user.join("config.json"), project.join(".furb").join("config.json")] {
      fs::write(&file, &line).unwrap();
    }
    let layers = Layers::new(&project, Some(user.clone()));
    read("the config", &line, || drop(layers.get()));
    read("the values of the config", &line, || drop(layers.values()));
  }
}
