//! The update of the catalog, read from data of the tests, so no test reads the cache of the machine or the network.

use serde_json::json;

use super::{Source, Status, applied, numbers, pi, stamp};
use crate::world::provider::catalog::parsed;

const SNAPSHOT: &str =
  r#"{"radius": {"rig": "pi", "env": ["RADIUS_API_KEY"], "models": {"old": {}}}}"#;

fn ids(text: Option<&str>) -> Vec<String> {
  let mut listed = parsed(SNAPSHOT, None);
  applied(&mut listed, text);
  listed["radius"].models.keys().cloned().collect()
}

#[test]
fn a_count_of_seconds_reads_as_a_time() {
  assert_eq!(stamp(0), "1970-01-01T00:00:00Z");
  assert_eq!(stamp(951_782_400), "2000-02-29T00:00:00Z");
  assert_eq!(stamp(1_000_000_000), "2001-09-09T01:46:40Z");
}

#[test]
fn a_version_is_compared_by_its_numbers() {
  assert!(numbers("1.10.0") > numbers("1.4.0"));
  assert_eq!(numbers("1.4.0"), vec![1, 4, 0]);
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

#[test]
fn a_copy_of_pi_ai_as_new_as_the_snapshot_replaces_the_models_of_its_provider() {
  let text =
    json!({"version": "99.0.0", "providers": {"radius": {"models": {"new": {}}}}}).to_string();
  assert_eq!(ids(Some(&text)), ["new"]);
}

#[test]
fn a_copy_of_an_older_release_or_one_that_reads_wrong_leaves_the_snapshot() {
  let old =
    json!({"version": "0.1.0", "providers": {"radius": {"models": {"new": {}}}}}).to_string();
  assert_eq!(ids(Some(&old)), ["old"]);
  assert_eq!(ids(Some("{not json")), ["old"]);
  let empty = json!({"version": "99.0.0", "providers": {"radius": {"models": {}}}}).to_string();
  assert_eq!(ids(Some(&empty)), ["old"]);
  assert_eq!(ids(None), ["old"]);
}

#[test]
fn a_status_keeps_the_name_of_each_source_in_its_file() {
  let status = Status {
    pi_ai: Source { version: Some("1.4.0".to_owned()), ..Source::default() },
    ..Status::default()
  };
  let text = serde_json::to_string(&status).unwrap();
  assert!(text.contains("\"pi-ai\"") && text.contains("\"models.dev\""));
  assert_eq!(serde_json::from_str::<Status>(&text).unwrap(), status);
}
