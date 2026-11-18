//! The update of the catalog where no door reaches it with no network: the time that a fetch
//! stamps, the backoff of an update in the background, and a model of pi-ai that a fetch reads,
//! since each door of them fetches models.dev or pi-ai. The suite of the world proves the copies
//! in the cache and the status through the catalog of a child.

use serde_json::json;

use std::{
  fs,
  sync::atomic::{AtomicUsize, Ordering},
};

use jiff::Timestamp;

use super::{Updater, pi, stamp};
use crate::testing;
use std::process;

#[test]
fn a_count_of_seconds_reads_as_a_time() {
  let at = |secs| stamp(Timestamp::from_second(secs).unwrap());
  assert_eq!(at(0), "1970-01-01T00:00:00Z");
  assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
  assert_eq!(at(1_000_000_000), "2001-09-09T01:46:40Z");
  assert_eq!(stamp("2026-10-26T18:09:17.987Z".parse().unwrap()), "2026-10-26T18:09:17Z");
}

#[test]
fn a_stale_or_absent_copy_updates_once_for_each_backoff() {
  static RAN: AtomicUsize = AtomicUsize::new(0);
  let updater = Updater::new(|| {
    RAN.fetch_add(1, Ordering::SeqCst);
  });
  let dir = testing::Yard::unmade(format!("furb-update-{}", process::id()));
  fs::create_dir_all(&dir).unwrap();
  let fresh = dir.join("models.json");
  fs::write(&fresh, "{}").unwrap();
  updater.background(&fresh);
  assert_eq!(RAN.load(Ordering::SeqCst), 0);
  let absent = dir.join("absent.json");
  updater.background(&absent);
  updater.background(&absent);
  assert_eq!(RAN.load(Ordering::SeqCst), 1);
}

#[test]
fn a_model_of_pi_ai_reads_in_the_words_of_models_dev() {
  let got = pi(&json!({
    "reasoning": true, "contextWindow": 1000, "maxTokens": 100, "input": ["text", "image"],
    "thinkingLevelMap": {"off": null, "low": "low", "high": "high"},
    "cost": {"input": 1.5, "cacheRead": 0.5}
  }))
  .unwrap();
  assert_eq!(got["limit"], json!({"context": 1000, "output": 100}));
  assert_eq!(got["reasoning_options"], json!([{"type": "effort", "values": ["low", "high"]}]));
  assert_eq!(got["cost"]["cache_read"], json!(0.5));
  assert_eq!(got["modalities"]["input"], json!(["text", "image"]));
  assert!(pi(&json!({"contextWindow": 1000})).is_none());
}
