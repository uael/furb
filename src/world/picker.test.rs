//! The files of the picker: the walk that reads `.gitignore` and lists dot files, the rank of
//! known queries, a query that walks from outside the directory, and one walk for each open of a
//! picker, with none for a key.

use std::{
  fs,
  path::Path,
  process,
  time::{Duration, Instant},
};

use super::{Picker, placed, walked};
use crate::testing::{Yard, pause};

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

/// The rows of a query once the walk and the match ended, within ten seconds.
fn settled(picker: &mut Picker, query: &str) -> Vec<String> {
  picker.find(query);
  let began = Instant::now();
  loop {
    let rows = picker.rows(50);
    if !rows.running {
      return rows.paths;
    }
    assert!(began.elapsed() < Duration::from_secs(10), "the picker settles");
    pause(Duration::from_millis(5));
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
fn a_walk_leaves_out_the_state_that_furb_writes_under_its_folder_and_keeps_its_config_and_skills() {
  let at = Yard::new(format!("furb-picker-state-{}", process::id()));
  for path in [
    "src/main.rs",
    ".furb/config.json",
    ".furb/skills/review/SKILL.md",
    ".furb/history.json",
    ".furb/sessions/r1.jsonl",
    ".furb/sessions/r1.changes.jsonl",
    ".furb/sessions/r1.lock",
    ".furb/sessions/r1.log",
    ".furb/sessions/r1.session.json",
    ".furb/sessions/r1.told",
    ".furb/trash/r2/r2.jsonl",
    ".furb/shares/1.html",
    "docs/sessions/plan.md",
    "docs/history.json",
  ] {
    let file = at.join(path);
    fs::create_dir_all(file.parent().expect("a folder")).expect("the folder of a file");
    fs::write(file, "").expect("a file of the project");
  }
  let kept = [
    ".furb/config.json",
    ".furb/skills/review/SKILL.md",
    "docs/history.json",
    "docs/sessions/plan.md",
    "src/main.rs",
  ];
  assert_eq!(walked(&at), kept);
  let mut rows = settled(&mut Picker::new(&at), "");
  rows.sort();
  assert_eq!(rows, kept, "a query of no text lists no file of the state of furb");
  assert!(walked(&at.join(".furb/sessions")).is_empty(), "a walk from the state lists nothing");
}

#[test]
fn a_known_query_ranks_the_file_that_it_means_first() {
  let at = Yard::new(format!("furb-picker-rank-{}", process::id()));
  project(&at);
  let mut picker = Picker::new(&at);
  for (query, meant) in [
    ("fold", "src/view/fold.rs"),
    ("world prov test", "src/world/provider.test.rs"),
    ("provwatch", "src/world/provider/watch.rs"),
    ("readme", "README.md"),
    ("env", ".env"),
  ] {
    let rows = settled(&mut picker, query);
    assert_eq!(rows.first().map(String::as_str), Some(meant), "{query}");
  }
  assert_eq!(settled(&mut picker, "").len(), 8, "a query of no text picks each file");
  assert!(settled(&mut picker, "zzz").is_empty());
}

#[test]
fn a_query_that_leaves_the_directory_walks_from_the_folder_that_it_names() {
  let at = Yard::new(format!("furb-picker-outside-{}", process::id()));
  project(&at);
  let mut picker = Picker::new(&at.join("src/view"));
  assert_eq!(settled(&mut picker, "fold"), ["fold.rs"]);
  let rows = settled(&mut picker, "../world/wat");
  assert_eq!(rows.first().map(String::as_str), Some("../world/watching.rs"));
  let absolute = format!("{}/", at.display());
  assert_eq!(settled(&mut picker, &format!("{absolute}readme")), [format!("{absolute}README.md")]);
  assert!(settled(&mut picker, "../../.git/").iter().all(|one| !one.contains(".git/")));
}

#[test]
fn a_picker_walks_its_folder_once_when_it_opens_and_never_at_a_key() {
  let at = Yard::new(format!("furb-picker-once-{}", process::id()));
  project(&at);
  let mut picker = Picker::new(&at.join("src/view"));
  assert!(settled(&mut picker, "zebra").is_empty());
  assert!(settled(&mut picker, "../world/zeb").is_empty());
  // A file made after each folder was walked: a key that a walk read would find it.
  fs::write(at.join("src/view/zebra.rs"), "").unwrap();
  fs::write(at.join("src/world/zebra.rs"), "").unwrap();
  assert!(settled(&mut picker, "zebr").is_empty(), "a key in the directory walked it again");
  assert!(
    settled(&mut picker, "../world/zebr").is_empty(),
    "a key outside walked its folder again"
  );
  let mut opened = Picker::new(&at.join("src/view"));
  assert_eq!(settled(&mut opened, "zebr"), ["zebra.rs"], "an open walks its folder");
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
