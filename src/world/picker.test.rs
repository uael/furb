//! The files of the picker: the walk that reads `.gitignore` and lists dot files, the rank of
//! known queries, and a query that walks from outside the directory.

use std::{fs, path::Path, process};

use super::{picked, placed, ranked, walked};
use crate::testing::Yard;

/// A project with a `.gitignore`, a dot file, a folder `.git`, and files at two depths.
fn project(at: &Path) {
  for (path, text) in [
    (".gitignore", "target/\n*.log\n"),
    (".env", "A=1\n"),
    (".git/HEAD", "ref: refs/heads/main\n"),
    ("target/built.rs", ""),
    ("run.log", ""),
    ("src/world/provider/watch.rs", ""),
    ("src/world/watching.rs", ""),
    ("src/world/provider.test.rs", ""),
    ("tui/test/providers.test.ts", ""),
    ("src/view/fold.rs", ""),
    ("README.md", ""),
  ] {
    let file = at.join(path);
    fs::create_dir_all(file.parent().expect("a folder")).expect("the folder of a file");
    fs::write(file, text).expect("a file of the project");
  }
}

#[test]
fn a_walk_lists_each_dot_file_but_git_itself_and_each_file_that_gitignore_hides() {
  let at = Yard::new(format!("furb-picker-walk-{}", process::id()));
  project(&at);
  assert_eq!(
    walked(&at),
    [
      ".env",
      ".gitignore",
      "README.md",
      "src/view/fold.rs",
      "src/world/provider.test.rs",
      "src/world/provider/watch.rs",
      "src/world/watching.rs",
      "tui/test/providers.test.ts",
    ]
  );
}

#[test]
fn a_known_query_ranks_the_file_that_it_means_first() {
  let at = Yard::new(format!("furb-picker-rank-{}", process::id()));
  project(&at);
  let files = walked(&at);
  for (query, meant) in [
    ("fold", "src/view/fold.rs"),
    ("world prov test", "src/world/provider.test.rs"),
    ("provwatch", "src/world/provider/watch.rs"),
    ("readme", "README.md"),
    ("env", ".env"),
  ] {
    assert_eq!(ranked(query, &files).first().map(String::as_str), Some(meant), "{query}");
  }
  assert_eq!(ranked("", &files), files, "a query of no text keeps the order of the walk");
  assert!(ranked("zzz", &files).is_empty());
}

#[test]
fn a_query_that_leaves_the_directory_walks_from_the_folder_that_it_names() {
  let at = Yard::new(format!("furb-picker-outside-{}", process::id()));
  project(&at);
  let session = at.join("src/view");
  let files = walked(&session);
  assert_eq!(picked(&session, "fold", &files), ["fold.rs"]);
  assert_eq!(
    picked(&session, "../world/wat", &files).first().map(String::as_str),
    Some("../world/watching.rs")
  );
  let absolute = format!("{}/", at.display());
  let found = picked(&session, &format!("{absolute}readme"), &files);
  assert_eq!(found, [format!("{absolute}README.md")]);
  assert!(picked(&session, "../../.git/", &files).iter().all(|one| !one.contains(".git/")));
}

#[test]
fn a_query_from_the_home_folder_names_a_folder_beneath_it_with_each_separator_of_the_platform() {
  let home = Path::new("home");
  let from = |folder| placed(Path::new("directory"), folder, Some(home.to_path_buf()));
  assert_eq!(from("~/docs/"), Some(home.join("docs")));
  #[cfg(windows)]
  assert_eq!(from("~\\docs\\"), Some(home.join("docs")));
  assert_eq!(placed(Path::new("directory"), "~/", None), None);
}
