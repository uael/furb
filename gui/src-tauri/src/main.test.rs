use std::path::Path;

use super::{asked, host};

#[test]
fn the_words_of_a_life_go_to_the_host() {
  let words: Vec<String> =
    ["--cwd", "/work", "--model", "claude-cli:opus", "--demo"].map(str::to_owned).to_vec();
  assert_eq!(asked(&words, Path::new("/tmp/x")), Ok((false, words.clone())));
}

#[test]
fn a_word_that_the_app_does_not_take_is_refused() {
  let words = vec!["--view".to_owned(), "out.json".to_owned()];
  assert_eq!(asked(&words, Path::new("/tmp/x")), Err("--view is no word of furb-gui.".to_owned()));
  assert_eq!(
    asked(&["--cwd".to_owned()], Path::new("/tmp/x")),
    Err("--cwd needs a value.".to_owned())
  );
}

#[test]
fn a_check_reads_a_record_on_a_list_of_its_own() {
  let words: Vec<String> = ["--check", "--record", "r.jsonl"].map(str::to_owned).to_vec();
  let list = Path::new("/tmp/x").join("workspaces.json").display().to_string();
  let host = ["--record", "r.jsonl", "--list", list.as_str()].map(str::to_owned).to_vec();
  assert_eq!(asked(&words, Path::new("/tmp/x")), Ok((true, host)));
  assert_eq!(asked(&words[..1], Path::new("/tmp/x")), Err("--check needs --record.".to_owned()));
}

#[test]
fn the_first_line_of_the_host_gives_its_address() {
  assert_eq!(
    host::address(r#"{"url":"ws://127.0.0.1:1/?token=t"}"#),
    Ok("ws://127.0.0.1:1/?token=t".to_owned())
  );
  assert!(host::address("bun: not found").is_err());
}

#[test]
fn the_program_of_the_host_is_the_one_of_the_checkout() {
  assert!(host::program().ends_with("host/main.ts"));
  assert!(host::program().is_file());
}
