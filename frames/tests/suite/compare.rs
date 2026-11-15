//! The command `compare`: the frame of the window in a scenario beside the two winner frames of
//! the other programs, with the scores of the judgment.

use std::{
  path::MAIN_SEPARATOR_STR,
  process::{Command, Output},
};

use crate::{put, yard};

const BEST: &str = "| # | Scenario | 1st | 2nd | furb |
| --- | --- | --- | --- | --- |
| 05 | prompt-box | OpenCode `b` | Claude Code `c` | near |

## 05 prompt-box

| Aspect | Claude Code | Codex | OpenCode | Pi | furb |
| --- | --- | --- | --- | --- | --- |
| 1 hierarchy | 7: rules | 6: bare | 8: a bar | 6: rules | 8: the hue bar |

## 06 prompt-history
";

/// The compare of the scenario 05-prompt-box, on a capture of the window and on the frames of
/// the other programs in the folder that `FURB_RIVALS` names, each a text.
fn compared(name: &str, window: &str, opencode: &str, claude: &str) -> Output {
  let yard = yard(name);
  let frame = |text: &str| {
    let lines: Vec<String> =
      text.lines().map(|line| format!("[[{line:?},\"#d4d4d4\",\"#1e1e1e\",0]]")).collect();
    format!("{{\"cols\":3,\"rows\":{},\"lines\":[{}]}}", lines.len(), lines.join(","))
  };
  let files = [
    ("rivals/BEST.md", BEST.to_owned()),
    ("rivals/opencode/05-prompt-box.b.txt", opencode.to_owned()),
    ("rivals/opencode/05-prompt-box.b.json", frame(opencode)),
    ("rivals/claude/05-prompt-box.c.txt", claude.to_owned()),
    ("rivals/claude/05-prompt-box.c.json", frame(claude)),
    ("out/120x40/05-prompt-box.b.txt", window.to_owned()),
  ];
  for (path, text) in files {
    put(&yard.join(path), &text);
  }
  let mut command = Command::new(env!("CARGO_BIN_EXE_furb-frames"));
  command
    .args(["compare", "05-prompt-box", "--out", "out"])
    .env("FURB_RIVALS", yard.join("rivals"));
  command
    .current_dir(&*yard)
    .output()
    .unwrap_or_else(|no| panic!("furb-frames did not start: {no}"))
}

/// What the compare said, which ends with no fault.
fn said(done: &Output) -> String {
  assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  String::from_utf8_lossy(&done.stdout).into_owned()
}

#[test]
fn the_winners_and_their_scores_come_from_the_judgment() {
  let said = said(&compared("winners", "w\n", "o\n", "c\n"));
  let head = said.lines().nth(2).unwrap_or_default();
  assert!(head.ends_with(" │ 1 OpenCode b │ 2 Claude Code c"), "{said}");
  // The command names each image by its path, with the separator of the system.
  let path = |parts: &[&str]| parts.join(MAIN_SEPARATOR_STR);
  let images = [
    path(&["out", "120x40", "05-prompt-box.b.png"]),
    path(&["out", "rivals", "05-prompt-box.1-opencode.b.png"]),
    path(&["out", "rivals", "05-prompt-box.2-claude.c.png"]),
  ];
  let end = format!(
    "\nimages: {}\n        {}\n        {}\n\nscores:\n- 1 hierarchy: OpenCode 8: a bar | Claude Code 7: \
     rules | furb 8: the hue bar\n\nprobes:\nnone\n",
    images[0], images[1], images[2]
  );
  assert!(said.ends_with(&end), "{said}");
}

#[test]
fn frames_stand_side_by_side_each_in_its_width() {
  let said = said(&compared("beside", "w\n", "日x\n", "y\nz\n"));
  let rows: Vec<&str> = said.lines().skip(3).take(2).collect();
  let pad = " ".repeat(119);
  assert_eq!(rows, [format!("w{pad} │ 日x │ y"), format!(" {pad} │     │ z")], "{said}");
}
