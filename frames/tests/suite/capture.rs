//! The command `capture` on a program in place of the window, so the test proves the walk of a
//! scenario in tmux and its recording, which no other command runs.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, process::Command};

use serde_json::{Value, json};

use crate::{frames, put, read, said, yard};

/// The program in place of the window: it says the text that the first step of a walk waits for
/// after half a second, then shows each key that it reads, as the terminal echoes it, and says the
/// mark of a long paste at its first line, as the window folds it.
const WINDOW: &str = "#!/bin/sh\nstty -isig\nsleep 0.5\nprintf 'Ask for a change\\n'\nexec sed -u \
  's/Read the boot, then the seed[.]$/≡ paste-1 · 12 lines/'\n";
/// The program in place of furb: its daemon says that it listens, then ends.
const FURB: &str = "#!/bin/sh\necho listening\nsleep 1\n";

#[test]
fn a_capture_waits_for_the_text_of_each_step_and_its_recording_replays_into_its_key_frames() {
  if Command::new("tmux").arg("-V").output().is_err() {
    eprintln!("SKIPPED: this machine has no tmux, which each capture runs the window in");
    return;
  }
  let yard = yard("capture");
  for (name, text) in [("bin/furb-window", WINDOW), ("bin/furb", FURB)] {
    let path = yard.join(name);
    put(&path, text);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
      .unwrap_or_else(|no| panic!("{name} cannot run: {no}"));
  }
  said(&frames(&["capture", "05-prompt-box", "--bin", "bin", "--out", "out"], &yard));
  let out = yard.join("out/120x40");
  let cast = read(&out.join("05-prompt-box.cast"));
  let events: Vec<Value> = cast
    .lines()
    .map(|line| serde_json::from_str(line).unwrap_or_else(|no| panic!("{line}: {no}")))
    .collect();
  assert_eq!(events[0], json!({ "version": 3, "term": { "cols": 120, "rows": 40 } }));
  let marked: Vec<(&str, &str)> = events[1..]
    .iter()
    .filter(|one| one[1] != "o")
    .map(|one| (one[1].as_str().unwrap_or_default(), one[2].as_str().unwrap_or_default()))
    .map(|(code, data)| (code, data.lines().next().unwrap_or_default()))
    .collect();
  assert_eq!(
    marked,
    [
      ("m", "a"),
      ("i", "Where does the time of the open go?"),
      ("m", "b"),
      ("i", "<C-c>"),
      ("i", "Read the boot, then the seed."),
      ("m", "c"),
      ("i", "Read the boot, then the seed."),
      ("m", "d")
    ]
  );
  // The program says the text of the first step half a second after its start, so the frame of
  // the first marker shows it only when the capture waited for it.
  let frame = |step: &str| read(&out.join(format!("05-prompt-box.{step}.txt")));
  assert!(frame("a").starts_with("Ask for a change\n"), "{}", frame("a"));
  assert!(frame("b").contains("Where does the time of the open go?"), "{}", frame("b"));
  assert!(frame("c").contains("≡ paste-1 · 12 lines"), "{}", frame("c"));
  assert!(frame("d").contains("Then stop."), "{}", frame("d"));
}
