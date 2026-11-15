//! The task `paths` as a process, on a tree of git of its own.

#[path = "../../tests/common/yard.rs"]
mod yard;

use std::{
  env, fs,
  path::Path,
  process::{self, Command, Output},
};

use yard::Yard;

/// The files that git holds in each tree of these tests.
const HELD: [(&str, &str); 5] = [
  ("bind/typescript/test/programs/claude.ts", ""),
  ("docs/models.md", ""),
  ("src/lib.rs", ""),
  ("README.md", ""),
  (".gitignore", "dist/\n"),
];

#[test]
fn a_path_that_git_holds_or_ignores_or_that_is_no_whole_path_stands() {
  let tree = tree(
    "stands",
    &[(
      "script/a.sh",
      "cp docs/models.md out\nls bind/typescript/test/programs/\nbun build bind/typescript\n\
     ls bind/dist\ncp bind/typescript/furb.$system.node x\nls docs/*.md\n\
     rm bind/typescript/furb.${{ x }}.node\ncd /usr/share/docs/x\ncurl https://example.com/src/y.sh\n\
     ls traces/overnight/a.json\nmy-docs/x\ncp demo.rs src/main.rs\n",
    )],
  );
  let done = checked(&tree);
  assert_eq!((done.status.code(), said(&done)), (Some(0), String::new()));
}

#[test]
fn a_path_that_a_move_left_in_a_tool_or_a_workflow_is_named_with_its_file_and_its_line() {
  let tree = tree(
    "named",
    &[
      (
        "script/prove-npm.sh",
        "set -e\nbun build --compile \"$root/bind/typescript/test/claude.ts\"\n",
      ),
      ("script/real.py", "x = ROOT / \"src/gone.py\"  # see docs/old.md.\n"),
      (".github/workflows/gates.yml", "      - run: python3 src/scope.py\n"),
      ("xtask/src/main.rs", "const HOOK: &str = \"docs/hooks/pre-commit\";\n"),
      ("docs/notes.md", "The old guide was docs/gone.md.\n"),
    ],
  );
  let done = checked(&tree);
  assert_eq!(
    (done.status.code(), said(&done)),
    (
      Some(1),
      ".github/workflows/gates.yml:1: src/scope.py is no file or folder of git\n\
       script/prove-npm.sh:2: bind/typescript/test/claude.ts is no file or folder of git\n\
       script/real.py:1: src/gone.py is no file or folder of git\n\
       script/real.py:1: docs/old.md is no file or folder of git\n\
       xtask/src/main.rs:1: docs/hooks/pre-commit is no file or folder of git\n"
        .to_owned()
    )
  );
}

/// A tree of git in a yard, which holds the files of `HELD` and `more`.
fn tree(name: &str, more: &[(&str, &str)]) -> Yard {
  let yard = Yard::new(format!("xtask-paths-{name}-{}", process::id()));
  for (file, text) in HELD.iter().chain(more) {
    let path = yard.join(file);
    let made =
      path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::write(&path, text));
    if let Err(no) = made {
      panic!("{} was not made: {no}", path.display());
    }
  }
  for args in [&["init", "-q"][..], &["add", "-A"]] {
    let done = run(Command::new("git").args(args), &yard);
    assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  }
  yard
}

fn checked(tree: &Path) -> Output {
  run(Command::new(env!("CARGO_BIN_EXE_xtask")).arg("paths"), tree)
}

/// The command in the tree, with no variable of git from a hook that runs the test, which would
/// point it at the repository of the hook.
fn run(command: &mut Command, tree: &Path) -> Output {
  for (name, _) in env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("GIT_")) {
    command.env_remove(name);
  }
  command.current_dir(tree).output().unwrap_or_else(|no| panic!("the command did not start: {no}"))
}

fn said(done: &Output) -> String {
  String::from_utf8_lossy(&done.stdout).into_owned()
}
