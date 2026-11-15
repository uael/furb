//! The catalog of models, from a snapshot of the tests. Each law here has no door: a door test of
//! the catalog stands in `tests/suite/world/offered.rs`.
//!
//! - The parse of the caches is a law of cost: a catalog reads a changed cache, which a door shows,
//!   and parses an unchanged one no more, which no door counts.
//! - The doctrine of the home: each build of a checkout runs in dev mode, so only a build outside
//!   git reads the official copy.
//! - Amazon counts the cache of a reply apart: a fake reply of Amazon needs the binary event stream
//!   of AWS, which is much code for one law.

use crate::testing;
use std::{fs, process, sync::Arc};

use super::{
  Catalog, DOCTRINE, doctrine_in, listed, parsed,
  update::{MODELS, PI},
};

/// A snapshot of the tests: a provider of each dialect, and models that think in each way.
const LISTED: &str = r#"{
"anthropic": {"rig": "anthropic", "env": ["ANTHROPIC_API_KEY"], "models": {
"claude-opus-5": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "medium", "high", "xhigh", "max"]}], "limit": {"context": 1000000, "output": 128000}, "cost": {"input": 5, "output": 25, "cache_read": 0.5, "cache_write": 6.25}, "modalities": {"input": ["text", "image"], "output": ["text"]}, "last_updated": "2026-09-01"},
"claude-haiku-4-5": {"reasoning": true, "reasoning_options": [{"type": "budget_tokens", "min": 1024}], "limit": {"context": 200000, "output": 64000}, "modalities": {"input": ["text"], "output": ["text"]}},
"claude-sonnet-5": {"reasoning": true, "reasoning_options": [{"type": "toggle"}, {"type": "effort", "values": ["low", "high"]}], "limit": {"context": 1000000, "output": 128000}, "modalities": {"input": ["text"], "output": ["text"]}},
"claude-old": {"status": "deprecated", "limit": {"context": 100000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"openai": {"rig": "openai", "env": ["OPENAI_API_KEY"], "models": {
"gpt-5": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["none", "minimal", "low", "high"]}], "limit": {"context": 400000, "output": 128000}, "modalities": {"input": ["text", "image"], "output": ["text"]}},
"gpt-4o": {"reasoning": false, "limit": {"context": 128000, "output": 16384}, "modalities": {"input": ["text"], "output": ["text"]}},
"gpt-image": {"limit": {"context": 0}, "modalities": {"input": ["text"], "output": ["image"]}}
}},
"google": {"rig": "gemini", "env": ["GOOGLE_API_KEY", "GEMINI_API_KEY"], "models": {
"gemini-flash": {"reasoning": true, "reasoning_options": [{"type": "budget_tokens"}], "limit": {"context": 1000000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"openrouter": {"rig": "openrouter", "env": ["OPENROUTER_API_KEY"], "models": {
"x/thinks": {"reasoning": true, "reasoning_options": [{"type": "toggle"}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}},
"x/gpt-4o": {"reasoning": true, "reasoning_options": [{"type": "budget_tokens"}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"fireworks-ai": {"rig": "chat", "env": ["FIREWORKS_API_KEY"], "api": "http://127.0.0.1:9/v1", "models": {
"gpt-4o": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "default"]}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"azure": {"rig": "azure", "env": ["AZURE_OPENAI_API_KEY"], "api": "https://${AZURE_RESOURCE_NAME}.openai.azure.com", "aliases": {"AZURE_RESOURCE_NAME": "AZURE_OPENAI_RESOURCE_NAME"}, "models": {
"gpt-5-mini": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}},
"claude-in-azure": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}, "provider": {"npm": "@ai-sdk/anthropic", "api": "https://${AZURE_RESOURCE_NAME}.services.ai.azure.com/anthropic/v1"}},
"model-router": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}, "provider": {"npm": "@ai-sdk/openai-compatible", "shape": "completions"}}
}},
"amazon-bedrock": {"rig": "bedrock", "env": ["AWS_PROFILE", "AWS_ACCESS_KEY_ID"], "models": {
"us.anthropic.claude-sonnet-5": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}], "limit": {"context": 8000, "output": 4000}, "modalities": {"input": ["text"], "output": ["text"]}},
"qwen.qwen3": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"google-vertex": {"rig": "vertex", "env": ["GOOGLE_CLOUD_API_KEY", "GOOGLE_APPLICATION_CREDENTIALS"], "models": {
"gemini-pro": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"radius": {"rig": "pi", "env": ["RADIUS_API_KEY"], "api": "https://radius.pi.dev/v1", "models": {
"balanced": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high", "max"]}], "limit": {"context": 8000, "output": 1000}, "modalities": {"input": ["text"], "output": ["text"]}}
}}
}"#;

#[test]
fn amazon_counts_the_cache_of_a_reply_apart() {
  let catalog = Catalog::of(&parsed(LISTED, None), None, None, |_| None);
  let model =
    catalog.find("amazon-bedrock:us.anthropic.claude-sonnet-5").expect("a model of Amazon");
  assert!(model.apart);
}

#[test]
fn the_parse_of_the_caches_is_made_again_only_when_a_file_changes() {
  let root = testing::Yard::unmade(format!("furb-listed-{}", process::id()));
  fs::create_dir_all(&root).unwrap();
  let at = root.join(MODELS);
  fs::write(&at, "{}").unwrap();
  let first = listed(Some(&at));
  assert!(Arc::ptr_eq(&first, &listed(Some(&at))));
  fs::write(root.join(PI), "{}").unwrap();
  let second = listed(Some(&at));
  assert!(!Arc::ptr_eq(&first, &second), "a change of the data of pi-ai is read");
  fs::write(&at, "not json").unwrap();
  assert!(!Arc::ptr_eq(&second, &listed(Some(&at))), "a change of the cache is read");
}

#[test]
fn the_doctrine_of_a_project_comes_first_then_the_checkout_of_dev_mode_then_the_official_copy_in_the_home()
 {
  let at = testing::Yard::unmade(format!("furb-doctrine-{}", process::id()));
  let (project, checkout, home) = (at.join("project"), at.join("checkout"), at.join("home"));
  for one in [&project, &checkout, &home] {
    fs::create_dir_all(one).unwrap();
  }
  let official = home.join(".furb").join("DOCTRINE.md");
  assert!(
    matches!(super::doctrine_place(&project, None, Some(&home)), Some(super::Place::Official(one)) if one == official),
    "the lookup names the official copy"
  );
  assert!(!official.exists(), "the lookup writes nothing");
  assert_eq!(doctrine_in(&project, None, Some(&home)), Some(official.clone()), "the official copy");
  assert_eq!(
    fs::read_to_string(&official).unwrap(),
    DOCTRINE,
    "furb writes it from the doctrine it carries"
  );
  fs::write(&official, "an older doctrine").unwrap();
  doctrine_in(&project, None, Some(&home));
  assert_eq!(
    fs::read_to_string(&official).unwrap(),
    DOCTRINE,
    "a copy that holds another text is written again"
  );
  assert_eq!(doctrine_in(&project, None, None), None, "no home holds no official copy");
  fs::write(checkout.join("DOCTRINE.md"), "the checkout").unwrap();
  assert_eq!(
    doctrine_in(&project, Some(&checkout), Some(&home)),
    Some(checkout.join("DOCTRINE.md")),
    "dev mode"
  );
  fs::create_dir_all(project.join(".furb")).unwrap();
  fs::write(project.join(".furb").join("DOCTRINE.md"), "the project").unwrap();
  let local = project.join(".furb").join("DOCTRINE.md");
  assert_eq!(
    doctrine_in(&project, Some(&checkout), Some(&home)),
    Some(local),
    "a doctrine of the project prevails"
  );
}
