use super::{mentioned, mentions};
use crate::pathed;

#[test]
fn a_mention_names_its_path_with_no_mark_that_ends_a_sentence_and_a_quoted_path_keeps_its_blank() {
  let text = "Read @src/lib.rs, then @\"docs/a b.md\" and mail me@ once.";
  assert_eq!(mentions(text), ["src/lib.rs", "docs/a b.md"]);
}

#[test]
fn a_file_that_he_named_comes_first_among_the_files_that_match_as_well() {
  let paths = ["src/a.rs", "src/b.rs"];
  let liked = |at: usize| if at == 1 { 1.0 } else { 0.0 };
  let order = |query: &str| -> Vec<usize> {
    pathed(query, paths, liked).into_iter().map(|one| one.at).collect()
  };
  assert_eq!((order(""), order("rs")), (vec![1, 0], vec![1, 0]));
}

#[test]
fn a_mention_takes_the_bytes_from_its_mark_to_the_end_of_its_path_and_its_quotes() {
  let text = "Read @src/lib.rs, and @\"a b.md\" now.";
  let spans: Vec<&str> = mentioned(text).into_iter().map(|(span, _)| &text[span]).collect();
  assert_eq!(spans, ["@src/lib.rs", "@\"a b.md\""]);
}
