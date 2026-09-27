//! The catalog of models, read from a snapshot of the tests and from the snapshot the crate carries, with the
//! credentials a test gives, so no test reads the environment or the network.

use std::sync::Arc;

use rig_core::{completion::CompletionRequest, message::Message};
use serde_json::{Value, json};

use super::{
  super::{Asked, Told, runtime},
  Catalog, SNAPSHOT, clamp, parsed,
};

/// A snapshot of the tests: a provider of each dialect, and models that think in each way.
const LISTED: &str = r#"{
"anthropic": {"rig": "anthropic", "env": ["ANTHROPIC_API_KEY"], "models": {
"claude-opus-5": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "medium", "high", "xhigh", "max"]}], "limit": {"context": 1000000, "output": 128000}, "cost": {"input": 5, "output": 25, "cache_read": 0.5, "cache_write": 6.25}, "modalities": {"input": ["text", "image"], "output": ["text"]}},
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
}}
}"#;

/// The catalog of the snapshot of the tests, with credentials of these names.
fn catalog(credentials: &[&str]) -> Catalog {
  let credentials: Vec<String> = credentials.iter().map(|one| (*one).to_owned()).collect();
  Catalog::of(&parsed(LISTED, None), None, move |name| {
    credentials.contains(&name.to_owned()).then(|| "key".to_owned())
  })
}

/// Each effort of a model, with its settings as JSON.
fn efforts(catalog: &Catalog, name: &str) -> Vec<(String, Value)> {
  let model = catalog.find(name).expect("the model is in the catalog");
  let each =
    model.efforts.iter().map(|(level, settings)| (level.clone(), Value::Object(settings.clone())));
  each.collect()
}

#[test]
fn the_catalog_offers_a_provider_whose_credential_stands_and_names_each_model_as_provider_and_id() {
  let offered = |credentials: &[&str]| {
    let catalog = catalog(credentials);
    catalog.offered().iter().map(|model| model.name.clone()).collect::<Vec<_>>()
  };
  assert_eq!(offered(&[]), Vec::<String>::new());
  assert_eq!(offered(&["GEMINI_API_KEY"]), ["google:gemini-flash"]);
  assert_eq!(
    offered(&["OPENAI_API_KEY", "ANTHROPIC_API_KEY"]),
    [
      "anthropic:claude-opus-5",
      "anthropic:claude-haiku-4-5",
      "anthropic:claude-sonnet-5",
      "openai:gpt-5",
      "openai:gpt-4o"
    ],
    "a deprecated model and a model that writes no text are not kept"
  );
}

#[test]
fn a_name_finds_a_model_by_its_provider_and_its_id_or_by_an_id_that_one_model_alone_holds() {
  let catalog = catalog(&[]);
  let found = |name: &str| catalog.find(name).map(|model| model.name.clone());
  assert_eq!(found("openai:gpt-4o").as_deref(), Some("openai:gpt-4o"));
  assert_eq!(
    found("opus").as_deref(),
    Some("claude-cli:opus"),
    "the catalog knows the claude command line it did not find"
  );
  assert_eq!(found("claude-opus-5").as_deref(), Some("anthropic:claude-opus-5"));
  assert_eq!(found("x/thinks").as_deref(), Some("openrouter:x/thinks"));
  assert_eq!(found("gpt-4o"), None, "two providers hold that id");
  assert_eq!(found("claude-old"), None, "a deprecated model is not kept");
}

#[test]
fn a_model_has_the_window_the_price_and_the_images_of_the_catalog() {
  let catalog = catalog(&[]);
  let opus = catalog.find("anthropic:claude-opus-5").expect("opus");
  assert_eq!(
    (opus.window, opus.images, opus.price),
    (1_000_000, true, Some([5.0, 25.0, 0.5, 6.25]))
  );
  assert_eq!(
    (opus.apart, opus.tokens),
    (true, Some(128_000)),
    "anthropic counts its cache apart, and asks a limit"
  );
  let gpt = catalog.find("openai:gpt-4o").expect("gpt-4o");
  assert_eq!(
    (gpt.window, gpt.images, gpt.price, gpt.apart, gpt.tokens),
    (128_000, false, None, false, None)
  );
}

#[test]
fn each_provider_reads_an_effort_in_its_own_words() {
  let catalog = catalog(&[]);
  let adaptive =
    |level: &str| json!({"output_config": {"effort": level}, "thinking": {"type": "adaptive"}});
  let names =
    |name: &str| efforts(&catalog, name).into_iter().map(|(level, _)| level).collect::<Vec<_>>();
  assert_eq!(names("anthropic:claude-opus-5"), ["low", "medium", "high", "xhigh", "max"]);
  assert_eq!(efforts(&catalog, "anthropic:claude-opus-5")[4], ("max".to_owned(), adaptive("max")));
  assert_eq!(
    efforts(&catalog, "anthropic:claude-haiku-4-5"),
    [
      ("off".to_owned(), json!({})),
      ("minimal".to_owned(), json!({"thinking": {"type": "enabled", "budget_tokens": 1024}})),
      ("low".to_owned(), json!({"thinking": {"type": "enabled", "budget_tokens": 2048}})),
      ("medium".to_owned(), json!({"thinking": {"type": "enabled", "budget_tokens": 8192}})),
      ("high".to_owned(), json!({"thinking": {"type": "enabled", "budget_tokens": 16384}})),
    ]
  );
  assert_eq!(
    efforts(&catalog, "anthropic:claude-sonnet-5"),
    [
      ("off".to_owned(), json!({"thinking": {"type": "disabled"}})),
      ("low".to_owned(), adaptive("low")),
      ("high".to_owned(), adaptive("high")),
    ]
  );
  assert_eq!(names("openai:gpt-5"), ["off", "minimal", "low", "high"]);
  assert_eq!(efforts(&catalog, "openai:gpt-5")[0].1, json!({"reasoning": {"effort": "none"}}));
  assert_eq!(
    names("openai:gpt-4o"),
    Vec::<String>::new(),
    "a model that does not reason takes no effort"
  );
  assert_eq!(
    efforts(&catalog, "google:gemini-flash")[1].1,
    json!({"generationConfig": {"thinkingConfig": {"thinkingBudget": 2048, "includeThoughts": true}}})
  );
  assert_eq!(names("openrouter:x/thinks"), Vec::<String>::new(), "a switch alone says no level");
  assert_eq!(
    efforts(&catalog, "openrouter:x/gpt-4o")[0].1,
    json!({"reasoning": {"enabled": false}})
  );
  assert_eq!(
    efforts(&catalog, "openrouter:x/gpt-4o")[4].1,
    json!({"reasoning": {"max_tokens": 16384}})
  );
  assert_eq!(
    efforts(&catalog, "fireworks-ai:gpt-4o"),
    [("low".to_owned(), json!({"reasoning_effort": "low"}))]
  );
}

#[test]
fn a_level_that_a_model_does_not_take_moves_to_the_nearest_one_it_takes() {
  let efforts = ["low", "high", "max"];
  assert_eq!(clamp(&efforts, "high"), Some("high"));
  assert_eq!(clamp(&efforts, "medium"), Some("high"), "the next one up first");
  assert_eq!(clamp(&efforts, "off"), Some("low"));
  assert_eq!(clamp(&["off", "low"], "xhigh"), Some("low"), "the next one down when none stands up");
  assert_eq!(clamp(&efforts, "loud"), Some("low"), "the first for a level of no name");
  assert_eq!(clamp(&[], "low"), None, "nothing for a model that takes no effort");
}

#[test]
fn a_roster_holds_the_models_a_host_names_and_the_model_of_its_actor_first() {
  let catalog = catalog(&["OPENAI_API_KEY"]);
  let names =
    |models: &[super::Model]| models.iter().map(|one| one.name.clone()).collect::<Vec<_>>();
  let (models, actor) = catalog.roster(None, None).expect("the offered roster");
  assert_eq!(
    (names(&models), actor),
    (vec!["openai:gpt-5".to_owned(), "openai:gpt-4o".to_owned()], None)
  );
  let named = ["claude-opus-5".to_owned(), "openai:gpt-5".to_owned()];
  let (models, actor) =
    catalog.roster(Some(&named), Some("x/thinks/high")).expect("a named roster");
  assert_eq!(names(&models), ["openrouter:x/thinks", "anthropic:claude-opus-5", "openai:gpt-5"]);
  assert_eq!(
    actor.as_deref(),
    Some("openrouter:x/thinks"),
    "a model that takes no effort is named alone"
  );
  let (models, actor) =
    catalog.roster(Some(&named), Some("openai:gpt-5/medium")).expect("the actor is named");
  assert_eq!((models.len(), actor.as_deref()), (2, Some("openai:gpt-5/high")));
  let (_, actor) = catalog.roster(Some(&named), Some("claude-opus-5")).expect("the model alone");
  assert_eq!(actor.as_deref(), Some("anthropic:claude-opus-5"));
  let no = catalog.roster(Some(&["nothing".to_owned()]), None).err();
  assert_eq!(no.as_deref(), Some("No model is nothing. Name one as provider:id."));
}

#[test]
fn a_model_whose_credential_stands_nowhere_is_refused_when_it_is_asked() {
  let catalog = catalog(&[]);
  let model = catalog.find("google:gemini-flash").expect("the model is known");
  let request = CompletionRequest {
    model: None,
    preamble: None,
    chat_history: vec![Message::user("hi")],
    documents: Vec::new(),
    tools: Vec::new(),
    temperature: None,
    max_tokens: None,
    tool_choice: None,
    additional_params: None,
    output_schema: None,
    record_telemetry_content: false,
  };
  let told: Told = Arc::new(|_, _| {});
  let asked = Asked { actor: "google:gemini-flash".into(), chain: "chain1".into(), request, told };
  let Err(no) = runtime().block_on((model.answers)(asked)) else {
    panic!("a model with no credential answers nothing")
  };
  assert!(no.to_string().contains("set GOOGLE_API_KEY or GEMINI_API_KEY"), "{no}");
}

#[test]
fn the_cache_holds_the_models_of_the_providers_of_the_snapshot_and_nothing_more() {
  let cached = r#"{
    "openai": {"id": "openai", "models": {"gpt-6": {"reasoning": false, "limit": {"context": 9}, "modalities": {"input": ["text"], "output": ["text"]}}}},
    "stranger": {"models": {"m": {"modalities": {"input": ["text"], "output": ["text"]}}}}
  }"#;
  let listed = parsed(LISTED, Some(cached));
  assert_eq!(listed["openai"].models.keys().collect::<Vec<_>>(), ["gpt-6"]);
  assert_eq!(
    listed["anthropic"].models.len(),
    4,
    "a provider the cache holds nothing of keeps the snapshot"
  );
  assert!(!listed.contains_key("stranger"));
  assert_eq!(parsed(LISTED, Some("not json"))["openai"].models.len(), 3);
}

#[test]
fn the_snapshot_that_the_crate_carries_names_a_client_of_rig_for_each_provider() {
  let listed = parsed(SNAPSHOT, None);
  let clients = [
    "anthropic",
    "openai",
    "gemini",
    "openrouter",
    "groq",
    "xai",
    "mistral",
    "deepseek",
    "together",
    "moonshot",
    "zai",
    "minimax",
    "huggingface",
    "xiaomi",
    "chat",
  ];
  assert!(listed.values().all(|one| clients.contains(&one.rig.as_str()) && !one.env.is_empty()));
  assert!(listed.values().filter(|one| one.rig == "chat").all(|one| one.api.is_some()));
  let catalog = Catalog::of(&listed, None, |_| None);
  let opus = catalog.find("anthropic:claude-opus-5").expect("the snapshot holds opus");
  assert!(opus.window >= 200_000 && opus.images && opus.price.is_some());
}
