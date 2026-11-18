//! The task `catalog`, on a yard where a stand-in of curl serves a catalog of models.dev and a
//! release of pi-ai from files, so it asks no network.
#![expect(clippy::panic, reason = "a panic is how a test fails")]

use std::{
  fs,
  path::Path,
  process,
  time::{SystemTime, UNIX_EPOCH},
};

use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};

use crate::{ended, git, shimmed, stand_in, xtask, yard::Yard};

/// The seconds of the epoch now.
fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// A release of pi-ai as its registry gives it: a gzipped tar that holds the folder `package`.
fn release(path: &Path) {
  let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
  let mut header = tar::Header::new_gnu();
  header.set_size(0);
  header.set_mode(0o644);
  header.set_cksum();
  let gzipped = archive
    .append_data(&mut header, "package/README.md", &[][..])
    .and_then(|()| archive.into_inner())
    .and_then(GzEncoder::finish)
    .and_then(|bytes| fs::write(path, bytes));
  gzipped.unwrap_or_else(|no| panic!("the release was not made: {no}"));
}

#[test]
fn the_snapshot_writes_each_value_as_python_writes_its_json_and_its_sources_say_when() {
  let yard = Yard::new(format!("xtask-catalog-{}", process::id()));
  let model = json!({
    "name": "dropped",
    "reasoning_options": [12_345_678.9],
    "limit": {"context": 0.000_01, "output": 1.5e16},
    "cost": {"input": 3, "output": 3.0, "cache_read": 0.25, "cache_write": 0.0001},
    "status": "Qwen \u{e9}\u{1f600}",
    "last_updated": "2026-11-15"
  });
  let catalog = json!({"anthropic": {"models": {"claude-haiku-9-9": model}}});
  let latest = json!({"version": "9.9.9", "dist": {"tarball": "https://example.com/pi.tgz"}});
  let made = fs::create_dir_all(yard.join("src/world/provider"))
    .and_then(|()| fs::write(yard.join("api.json"), catalog.to_string()))
    .and_then(|()| fs::write(yard.join("latest.json"), latest.to_string()));
  made.unwrap_or_else(|no| panic!("the sources were not made: {no}"));
  release(&yard.join("pi.tgz"));
  let served = [
    ("https://models.dev/api.json", yard.join("api.json")),
    ("https://registry.npmjs.org/@earendil-works/pi-ai/latest", yard.join("latest.json")),
    ("https://example.com/pi.tgz", yard.join("pi.tgz")),
  ];
  let served: Vec<(&str, &Path)> =
    served.iter().map(|(url, file)| (*url, file.as_path())).collect();
  stand_in(&yard, "curl", &served);
  git(&yard, &["init", "-q"]);
  let before = now();
  let (code, _, err) = ended(xtask(&yard, ["catalog"]).env("PATH", shimmed(&yard)));
  let after = now();
  assert_eq!(code, Some(0), "{err}");
  let read =
    |file: &str| fs::read_to_string(yard.join("src/world/provider").join(file)).unwrap_or_default();
  let snapshot = read("catalog.json");
  let line = r#""claude-haiku-9-9": {"reasoning_options": [12345678.9], "#.to_owned()
    + r#""limit": {"context": 1e-05, "output": 1.5e+16}, "#
    + r#""cost": {"input": 3, "output": 3.0, "cache_read": 0.25, "cache_write": 0.0001}, "#
    + r#""status": "Qwen \u00e9\ud83d\ude00", "last_updated": "2026-11-15", "class": "dear"}"#;
  assert!(snapshot.lines().any(|one| one == line), "{snapshot}");
  let parsed: Value = serde_json::from_str(&snapshot).unwrap_or_default();
  assert_eq!(parsed["claude-cli"]["models"]["haiku"], json!({"class": "dear"}), "{snapshot}");
  let sources: Value = serde_json::from_str(&read("catalog.sources.json")).unwrap_or_default();
  assert_eq!(sources["pi-ai"]["version"], "9.9.9");
  // git reads the time of the stamp, as a reader of the sources does, into seconds of the epoch.
  let stamp = sources["models.dev"]["fetched"].as_str().unwrap_or_default();
  let since = git(&yard, &["rev-parse", &format!("--since={stamp}")]);
  let at: u64 = since.trim_start_matches("--max-age=").parse().unwrap_or_default();
  assert!(before <= at && at <= after, "{before} {stamp} {after}");
}
