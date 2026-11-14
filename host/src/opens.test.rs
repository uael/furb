#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

use std::time::Duration;

use furb::life::Stage;
use tokio::time::{Instant, timeout};

use super::{Heard, Opens, PACE, Step, Told};
use crate::protocol::Frame;

fn frame(stage: Stage, done: u64, of: u64) -> Frame {
  Frame::Opening { life: "a".to_owned(), stage, done, of }
}

#[test]
fn an_open_only_goes_forward() {
  let mut opens = Opens::default();
  assert!(opens.at(Stage::Find, 0, 10));
  assert!(opens.at(Stage::Find, 7, 10));
  assert!(!opens.at(Stage::Find, 3, 10), "a number of a stage never goes back");
  assert!(opens.at(Stage::Log, 0, 4));
  assert!(!opens.at(Stage::Find, 10, 10), "a stage never comes again");
  assert!(!opens.whole(), "the log did not take each event yet");
  assert!(opens.at(Stage::Log, 6, 4), "the work done never passes the work known");
  let step = |stage, done, of| Step { stage, done, of };
  assert_eq!(opens.steps, [step(Stage::Find, 7, 10), step(Stage::Log, 6, 6)]);
  assert!(opens.whole());
}

#[tokio::test]
async fn a_window_hears_each_stage_that_moved_at_once_and_a_stage_that_goes_on_at_the_pace() {
  let (told, opens) = Told::new();
  let progress = told.progress();
  told.born();
  let mut early = Heard::new(opens.clone());
  (progress.0)(Stage::Find, 0, 10);
  early.due().await;
  assert_eq!(early.frames("a"), [frame(Stage::Find, 0, 10)]);
  (progress.0)(Stage::Find, 10, 10);
  let at_once = Duration::ZERO;
  assert!(timeout(at_once, early.due()).await.is_err(), "a stage that goes on waits");
  (progress.0)(Stage::Seed, 0, 0);
  assert!(timeout(at_once, early.due()).await.is_ok(), "a new stage waits for no pace");
  let reached = [frame(Stage::Find, 10, 10), frame(Stage::Seed, 0, 0)];
  assert_eq!(early.frames("a"), reached, "a stage that passed has its last numbers");
  let mut late = Heard::new(opens.clone());
  assert_eq!(late.frames("a"), reached, "a window that attaches hears each stage reached");
  (progress.0)(Stage::Log, 0, 4);
  let sent = Instant::now();
  assert_eq!(early.frames("a"), [frame(Stage::Log, 0, 4)]);
  (progress.0)(Stage::Log, 4, 4);
  early.due().await;
  assert!(sent.elapsed() >= PACE, "a stage that goes on waits for the pace");
  assert_eq!(early.frames("a"), [frame(Stage::Log, 4, 4)]);
  assert_eq!(early.frames("a"), [], "a window hears each frame once");
  let mut after = Heard::new(opens);
  assert_eq!(after.frames("a"), [], "a life that stands tells no stage of its open");
  told.born();
  (progress.0)(Stage::Find, 1, 3);
  assert_eq!(after.frames("a"), [frame(Stage::Find, 1, 3)], "a rebirth tells its stages again");
}
