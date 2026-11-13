use super::engine_bound;
use crate::ENGINE;

#[test]
fn a_word_of_a_skill_that_binds_a_name_of_the_engine_in_a_block_a_def_a_walrus_a_capture_or_a_global_binds_it()
 {
  let bound = |word: &str| engine_bound(word, ENGINE);
  assert_eq!(bound("if True:\n  over = 1"), vec!["over"]);
  assert_eq!(bound("def under():\n  pass"), vec!["under"]);
  assert_eq!(bound("(question := 1)"), vec!["question"]);
  assert_eq!(bound("match 1:\n  case scope:\n    pass"), vec!["scope"]);
  assert_eq!(bound("def f():\n  global lineage\n  lineage = 1"), vec!["lineage"]);
  assert_eq!(bound("accepts = None\nactor = 'x'\nimport re"), Vec::<String>::new());
}
