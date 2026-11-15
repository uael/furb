use super::*;

#[test]
fn a_status_gives_its_branch_and_each_file_with_its_state() {
  let status = "## main...origin/main [ahead 1]\0 M src/a.rs\0?? notes.md\0?? .furb/sessions/a.jsonl\0R  new.rs\0old.rs\0D  gone.rs\0";
  let file = |path: &str, state| Changed { path: path.to_owned(), added: 0, removed: 0, state };
  let files = vec![
    file("src/a.rs", Change::Modified),
    file("notes.md", Change::Untracked),
    file("new.rs", Change::Renamed),
    file("gone.rs", Change::Deleted),
  ];
  assert_eq!(statused(status), (Some("main".to_owned()), files));
}

#[test]
fn a_head_with_no_commit_names_its_branch_and_a_detached_head_names_none() {
  assert_eq!(branched("No commits yet on trunk"), Some("trunk".to_owned()));
  assert_eq!(branched("HEAD (no branch)"), None);
  assert_eq!(branched("w/feed"), Some("w/feed".to_owned()));
}

#[test]
fn the_counts_take_the_new_path_of_a_renamed_file_and_none_for_a_binary_one() {
  let counts = "3\t1\tsrc/a.rs\0-\t-\tlogo.png\x002\t0\t\0old.rs\0new.rs\0";
  let all =
    vec![("src/a.rs".to_owned(), 3, 1), ("logo.png".to_owned(), 0, 0), ("new.rs".to_owned(), 2, 0)];
  assert_eq!(counted(counts), all);
}
