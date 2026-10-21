use serde_json::{Value, json};

use super::view;
use crate::model::Life;

fn facts() -> Vec<Value> {
  vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "Do it", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["ready", "rung1", "rung1", "x = read('a')\ny = read('b')\nz = bash('ls')"]),
    json!(["read", "read1", "rung1", "chain1", "a"]),
    json!(["read", "read2", "rung1", "chain1", "b"]),
    json!(["bash", "bash1", "rung1", "chain1", "ls", "", 600.0]),
  ]
}

#[test]
fn a_step_says_what_it_did_by_the_acts_that_it_made() {
  let life = Life::of(&facts(), 7);
  let got = view(&life, "demo", true);
  assert_eq!(got["threads"][0]["ladder"][0]["said"], "Read 2 files, ran 1 command");
}

#[test]
fn each_line_holds_the_acts_that_it_made() {
  let life = Life::of(&facts(), 7);
  let got = view(&life, "demo", true);
  let lines = &got["threads"][0]["ladder"][0]["lines"];
  assert_eq!(lines[0]["acts"][0]["id"], "read1");
  assert_eq!(lines[2]["acts"][0]["id"], "bash1");
  assert_eq!(lines[2]["acts"][0]["state"], "runs");
}
