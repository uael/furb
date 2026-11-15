//! The tests of the snapshot of the catalog.

use serde_json::json;

use super::{python, served, utc};

#[test]
fn the_snapshot_writes_each_value_as_python_writes_its_json() {
  let value = json!({"name": "Qwen \u{e9}\u{1f600}", "cost": [3, 3.0, 0.25, 0.0001, 0.00001, 1.5e16, 12_345_678.9], "on": null, "ok": true});
  assert_eq!(
    python(&value),
    r#"{"name": "Qwen \u00e9\ud83d\ude00", "cost": [3, 3.0, 0.25, 0.0001, 1e-05, 1.5e+16, 12345678.9], "on": null, "ok": true}"#
  );
  assert_eq!(utc(1_763_208_000), "2025-11-15T12:00:00Z");
  assert_eq!(
    served().as_object().map(serde_json::Map::len),
    Some(39),
    "the providers that the crate serves read as JSON"
  );
}
