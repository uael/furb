//! The task `catalog`, on a yard whose curl serves a catalog of models.dev and a release of pi-ai
//! from files, so it asks no network.

#![cfg(unix)]

use std::{fs, path::Path, process};

use serde_json::{Value, json};

use crate::{ended, git, program, shim, shimmed, xtask, yard::Yard};

/// The time of this moment as the sources write it, from the clock of the machine.
fn now(yard: &Path) -> String {
  let (code, out, _) = ended(program("date", yard).args(["-u", "+%Y-%m-%dT%H:%M:%SZ"]));
  assert_eq!(code, Some(0));
  out.trim().to_owned()
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
  let made = fs::create_dir_all(yard.join("pi/package"))
    .and_then(|()| fs::create_dir_all(yard.join("src/world/provider")))
    .and_then(|()| fs::write(yard.join("api.json"), catalog.to_string()))
    .and_then(|()| fs::write(yard.join("latest.json"), latest.to_string()))
    .and_then(|()| fs::write(yard.join("pi/package/README.md"), ""));
  made.unwrap_or_else(|no| panic!("the sources were not made: {no}"));
  let (code, _, err) =
    ended(program("tar", &yard.join("pi")).args(["czf", "../pi.tgz", "package"]));
  assert_eq!(code, Some(0), "{err}");
  let at = yard.display();
  shim(
    &yard,
    "curl",
    &format!(
      "for url; do :; done\ncase \"$url\" in\n\
       https://models.dev/api.json) cat {at}/api.json ;;\n\
       https://registry.npmjs.org/@earendil-works/pi-ai/latest) cat {at}/latest.json ;;\n\
       https://example.com/pi.tgz) cat {at}/pi.tgz ;;\n\
       *) echo \"no file for $url\" >&2; exit 22 ;;\nesac\n"
    ),
  );
  git(&yard, &["init", "-q"]);
  let before = now(&yard);
  let (code, _, err) = ended(xtask(&yard, ["catalog"]).env("PATH", shimmed(&yard)));
  let after = now(&yard);
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
  let stamp = sources["models.dev"]["fetched"].as_str().unwrap_or_default();
  assert!(before.as_str() <= stamp && stamp <= after.as_str(), "{before} {stamp} {after}");
}
