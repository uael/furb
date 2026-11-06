#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

use std::{
  fs, io,
  path::{Path, PathBuf},
  sync::{Arc, Mutex, mpsc as held},
};

use serde_json::json;
use tokio::sync::mpsc;

use super::{Append, Journal, Line, before};
use crate::{error::JournalError, event::Event};

/// A writer that a test holds: each append waits for a release, then keeps its line.
fn writer() -> (Append, held::Sender<()>, Arc<Mutex<Vec<String>>>) {
  let (release, gate) = held::channel::<()>();
  let (gate, kept) = (Mutex::new(gate), Arc::new(Mutex::new(Vec::new())));
  let keeps = Arc::clone(&kept);
  let append: Append = Arc::new(move |_: &Path, line: &str| {
    gate.lock().unwrap().recv().map_err(io::Error::other)?;
    keeps.lock().unwrap().push(line.to_owned());
    Ok(())
  });
  (append, release, kept)
}

fn said(journal: &Journal, change: &str) {
  let line = Line::Change { chain: "chain1".to_owned(), change: change.to_owned() };
  journal.lines.send(line).unwrap();
}

#[tokio::test]
async fn the_chain_of_a_change_hears_that_it_changed_only_once_the_change_is_on_the_disk() {
  let (append, release, kept) = writer();
  let (told, mut heard) = mpsc::unbounded_channel();
  let journal = Journal::appending(PathBuf::new(), PathBuf::from("j"), told, append);
  said(&journal, "a");
  for _ in 0..16 {
    tokio::task::yield_now().await;
  }
  assert!(heard.try_recv().is_err());
  release.send(()).unwrap();
  journal.flushed().await.unwrap();
  assert_eq!(*kept.lock().unwrap(), ["a"]);
  assert!(matches!(heard.try_recv(), Ok(Event::Changed(chain)) if chain == "chain1"));
}

#[tokio::test]
async fn a_flush_waits_for_a_held_writer_and_finds_each_change_whole_and_in_order() {
  let (append, release, kept) = writer();
  let (told, _heard) = mpsc::unbounded_channel();
  let journal = Journal::appending(PathBuf::new(), PathBuf::from("j"), told, append);
  said(&journal, "a");
  said(&journal, "b");
  let flushed = journal.flushed();
  tokio::pin!(flushed);
  for _ in 0..16 {
    assert!(futures::poll!(&mut flushed).is_pending());
    tokio::task::yield_now().await;
  }
  assert!(kept.lock().unwrap().is_empty());
  release.send(()).unwrap();
  release.send(()).unwrap();
  flushed.await.unwrap();
  assert_eq!(*kept.lock().unwrap(), ["a", "b"]);
}

#[tokio::test]
async fn a_flush_says_once_what_the_disk_lost_and_the_life_hears_each_fault_at_once() {
  let append: Append = Arc::new(|_: &Path, line: &str| match line {
    "lost" => Err(io::Error::other("the disk is full")),
    _ => Ok(()),
  });
  let (told, mut heard) = mpsc::unbounded_channel();
  let journal = Journal::appending(PathBuf::new(), PathBuf::from("j"), told, append);
  for change in ["lost", "kept", "lost"] {
    said(&journal, change);
  }
  let lost = journal.flushed().await.unwrap_err();
  assert!(matches!(lost, JournalError::Lost { count: 2, .. }), "{lost}");
  assert!(lost.to_string().ends_with("the first for: the disk is full"), "{lost}");
  journal.flushed().await.unwrap();
  let notices = std::iter::from_fn(|| heard.try_recv().ok());
  assert_eq!(notices.filter(|one| matches!(one, Event::Notice(_))).count(), 3);
}

#[tokio::test]
async fn the_life_hears_one_notice_when_the_journal_begins_to_lose_changes_and_one_when_it_recovers()
 {
  let append: Append = Arc::new(|_: &Path, line: &str| match line {
    "kept" => Ok(()),
    _ => Err(io::Error::other("the disk is full")),
  });
  let (told, mut heard) = mpsc::unbounded_channel();
  let journal = Journal::appending(PathBuf::new(), PathBuf::from("j"), told, append);
  for change in ["a", "b", "c", "kept"] {
    said(&journal, change);
  }
  assert!(journal.flushed().await.is_err());
  let notices: Vec<String> = std::iter::from_fn(|| heard.try_recv().ok())
    .filter_map(|one| match one {
      Event::Notice(text) => Some(text),
      _ => None,
    })
    .collect();
  assert_eq!(notices.len(), 2, "{notices:?}");
  assert!(notices[0].contains("took no change"), "{notices:?}");
  assert!(notices[1].contains("takes changes again"), "{notices:?}");
}

#[test]
fn a_write_keeps_what_its_file_held_made_when_it_stood_not_and_why_it_could_not_read_it() {
  let at = std::env::temp_dir().join(format!("furb-host-changes-{}", std::process::id()));
  fs::create_dir_all(&at).unwrap();
  let (text, binary) = (at.join("a.txt"), at.join("b.bin"));
  fs::write(&text, "old\n").unwrap();
  fs::write(&binary, [0xff, 0xfe, 0x00]).unwrap();
  let path = |file: &Path| json!(file.display().to_string());
  assert_eq!(before(&text), json!({ "path": path(&text), "before": "old\n" }));
  let made = at.join("c.txt");
  assert_eq!(before(&made), json!({ "path": path(&made), "before": "", "made": true }));
  let unread = before(&binary);
  fs::remove_dir_all(&at).unwrap();
  assert_eq!((&unread["before"], unread.get("made")), (&json!(""), None));
  assert!(unread["unread"].as_str().is_some_and(|why| !why.is_empty()), "{unread}");
}
