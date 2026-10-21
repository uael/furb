use serde_json::json;

use super::{Lane, scripted, word};

#[test]
fn the_script_answers_a_worker_by_the_label_of_its_chain() {
  let request = json!({"messages": [{"role": "user", "content": "chain2_label = 'w shortcut'"}]});
  let (said, _) = scripted(&request);
  assert_eq!(said, word(Lane::Shortcut, 0));
}

#[test]
fn the_script_answers_the_root_by_default() {
  let request = json!({"messages": [{"role": "user", "content": "#chain1"}]});
  assert_eq!(scripted(&request).0, word(Lane::Root, 0));
}
