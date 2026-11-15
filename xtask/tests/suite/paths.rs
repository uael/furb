//! The task `paths` as a process, on a tree of git of its own.

use std::{fs, path::Path, process};

use crate::{ended, git, xtask, yard::Yard};

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
  assert_eq!(checked(&tree), (Some(0), String::new()));
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
  assert_eq!(
    checked(&tree),
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
  git(&yard, &["init", "-q"]);
  git(&yard, &["add", "-A"]);
  yard
}

/// The code of the task and what it printed.
fn checked(tree: &Path) -> (Option<i32>, String) {
  let (code, out, _) = ended(&mut xtask(tree, ["paths"]));
  (code, out)
}
