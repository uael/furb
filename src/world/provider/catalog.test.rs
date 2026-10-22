//! The catalog of models, read from a snapshot of the tests and from the snapshot the crate carries, with the
//! credentials a test gives, so no test reads the environment or the network.

use std::{path::PathBuf, sync::Arc};

use rig_core::{completion::CompletionRequest, message::Message};
use serde_json::{Value, json};

use super::{
  super::{
    Asked, Model, Told,
    clients::Client,
    local::{
      Server,
      test::{dropping, fake},
    },
  },
  Catalog, SNAPSHOT, clamp, parsed,
};
use crate::ear::reactor;

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

/// The catalog of the snapshot of the tests and of the local server that the environment names, with an environment
/// that holds these names, each with the value after its `=`, or `key`.
fn catalog(environment: &[&str]) -> Catalog {
  let environment: Vec<(String, String)> = (environment.iter())
    .map(|one| one.split_once('=').unwrap_or((one, "key")))
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
  let env = move |name: &str| {
    environment.iter().find(|(held, _)| held == name).map(|(_, value)| value.clone())
  };
  let server = Server::named(&env).map(Arc::new);
  Catalog::of(&parsed(LISTED, None), None, server, env)
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
  let bedrock =
    catalog.find("amazon-bedrock:us.anthropic.claude-sonnet-5").expect("a model of Amazon");
  assert_eq!((bedrock.apart, bedrock.tokens), (true, Some(4000)));
  let radius = catalog.find("radius:balanced").expect("a model of Radius");
  assert_eq!((radius.apart, radius.conversation.as_deref()), (true, Some("sessionId")));
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
  assert_eq!(
    efforts(&catalog, "amazon-bedrock:us.anthropic.claude-sonnet-5")[1].1,
    json!({"output_config": {"effort": "high"}, "thinking": {"type": "adaptive"}}),
    "a model of Anthropic on Amazon reads an effort as Anthropic does"
  );
  assert_eq!(
    efforts(&catalog, "amazon-bedrock:qwen.qwen3"),
    [],
    "the other models of Amazon read none"
  );
  assert_eq!(
    efforts(&catalog, "radius:balanced")[2],
    ("max".to_owned(), json!({"reasoning": "max"}))
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
fn a_roster_holds_the_model_of_its_actor_first_and_the_models_a_host_names_or_every_model_the_catalog_offers()
 {
  let catalog = catalog(&["OPENAI_API_KEY"]);
  let names =
    |models: &[super::Model]| models.iter().map(|one| one.name.clone()).collect::<Vec<_>>();
  let (models, actor) = catalog.roster(None, None).expect("the roster of no name");
  assert_eq!(
    (names(&models), actor),
    (vec!["openai:gpt-5".to_owned(), "openai:gpt-4o".to_owned()], None),
    "a host that names nothing offers every model the catalog offers, and stands on the first"
  );
  let (models, actor) =
    catalog.roster(None, Some("openai:gpt-4o/high")).expect("the roster of an actor");
  assert_eq!(
    (names(&models), actor.as_deref()),
    (vec!["openai:gpt-4o".to_owned(), "openai:gpt-5".to_owned()], Some("openai:gpt-4o"))
  );
  let (models, _) =
    catalog.roster(Some(&[]), Some("openai:gpt-4o")).expect("a roster that names none");
  assert_eq!(names(&models), ["openai:gpt-4o"], "a roster that names none holds the actor alone");
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
  assert_eq!(
    actor.as_deref(),
    Some("anthropic:claude-opus-5/high"),
    "an actor that names no effort takes high"
  );
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
  let asked = Asked {
    actor: "google:gemini-flash".into(),
    chain: "chain1".into(),
    request,
    told,
    thought: None,
  };
  let Err(no) = reactor().block_on((model.answers)(asked)) else {
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
fn the_newer_of_the_cache_and_the_snapshot_holds_for_each_provider() {
  let cached = |day: &str| {
    let model = json!({"limit": {"context": 9}, "modalities": {"input": ["text"], "output": ["text"]},
      "last_updated": day});
    json!({"anthropic": {"models": {"claude-cached": model}}}).to_string()
  };
  let held = |day: &str| parsed(LISTED, Some(&cached(day)))["anthropic"].models.len();
  assert_eq!(held("2026-08-31"), 4, "a cache older than the snapshot hides nothing of it");
  assert_eq!(held("2026-09-01"), 1);
  assert_eq!(held("2026-10-01"), 1);
}

#[test]
fn the_snapshot_that_the_crate_carries_names_a_client_of_rig_for_each_provider() {
  let listed = parsed(SNAPSHOT, None);
  assert!(listed.len() >= 40 && listed.values().all(|one| !one.env.is_empty()));
  let addressed =
    |one: &&super::Listed| matches!(one.rig, Client::Chat | Client::Gateway | Client::Pi);
  assert!(listed.values().filter(addressed).all(|one| one.api.is_some()));
  let catalog = Catalog::of(&listed, None, None, |_| None);
  let opus = catalog.find("anthropic:claude-opus-5").expect("the snapshot holds opus");
  assert!(opus.window >= 200_000 && opus.images && opus.price.is_some());
  let radius =
    catalog.find("radius:balanced").expect("the data of pi-ai gives the models of Radius");
  assert!(radius.efforts().contains(&"high"));
}

#[test]
fn a_provider_is_offered_when_the_environment_holds_its_credential_and_each_place_of_its_address() {
  let offered = |environment: &[&str], name: &str| {
    catalog(environment).offered().iter().any(|model| model.name == name)
  };
  assert!(
    !offered(&["AZURE_OPENAI_API_KEY"], "azure:gpt-5-mini"),
    "the address needs its resource"
  );
  assert!(offered(&["AZURE_OPENAI_API_KEY", "AZURE_RESOURCE_NAME=r"], "azure:gpt-5-mini"));
  assert!(offered(
    &["AZURE_OPENAI_API_KEY", "AZURE_OPENAI_RESOURCE_NAME=r"],
    "azure:claude-in-azure"
  ));
  assert!(
    !offered(&["AZURE_OPENAI_API_KEY", "AZURE_RESOURCE_NAME=r"], "azure:model-router"),
    "no client of rig asks a model of a shape of its own"
  );
  assert!(offered(&["AWS_PROFILE"], "amazon-bedrock:qwen.qwen3"));
  assert!(!offered(&["GOOGLE_CLOUD_API_KEY"], "google-vertex:gemini-pro"), "Google asks a project");
  assert!(offered(&["GOOGLE_CLOUD_API_KEY", "GOOGLE_CLOUD_PROJECT=p"], "google-vertex:gemini-pro"));
  let home = std::env::temp_dir().join(format!("furb-adc-{}", std::process::id()));
  let (root, gcloud) =
    if cfg!(windows) { ("APPDATA", "gcloud") } else { ("HOME", ".config/gcloud") };
  std::fs::create_dir_all(home.join(gcloud)).expect("the folder of gcloud");
  std::fs::write(home.join(gcloud).join("application_default_credentials.json"), "{}")
    .expect("a file");
  let adc = format!("{root}={}", home.display());
  assert!(
    offered(&[&adc, "GOOGLE_CLOUD_PROJECT=p"], "google-vertex:gemini-pro"),
    "the credentials of an application stand where the client of Google finds them"
  );
  assert!(offered(&["RADIUS_API_KEY"], "radius:balanced"));
}

#[test]
fn each_place_of_an_address_is_filled_once_so_a_value_that_names_a_place_stands_as_it_is() {
  let offered =
    catalog(&["AZURE_OPENAI_API_KEY", "AZURE_RESOURCE_NAME=${AZURE_RESOURCE_NAME}"]).offered();
  assert!(offered.iter().any(|model| model.name == "azure:gpt-5-mini"));
}

/// The names of these models.
fn names_of(models: &[Model]) -> Vec<String> {
  models.iter().map(|one| one.name.clone()).collect()
}

/// A local server that lists one model, qwen, with its window; and its address as the environment names it, and each
/// request it reads.
fn qwen() -> (String, std::sync::mpsc::Receiver<String>) {
  let list = json!({"data": [{"id": "qwen", "max_model_len": 1000}]});
  let (address, told) =
    fake(move |request| request.starts_with("GET /v1/models ").then(|| list.clone()));
  (format!("FURB_LOCAL_URL={address}"), told)
}

#[test]
fn the_catalog_asks_the_local_server_only_when_a_name_needs_it_or_a_host_lists_what_it_offers() {
  let (url, told) = qwen();
  let catalog = catalog(&[&url, "OPENAI_API_KEY"]);
  let named = ["openai:gpt-4o".to_owned()];
  catalog.roster(Some(&named), Some("openai:gpt-5")).expect("a roster of the network");
  catalog
    .roster(Some(&[]), Some("openai:gpt-5/high"))
    .expect("an actor of the network at an effort");
  assert!(catalog.find("openai:gpt-6").is_none() && catalog.find("gpt-6").is_none());
  assert_eq!(
    told.try_iter().count(),
    0,
    "a roster that names no model of the local server asks it nothing"
  );
  let (models, actor) = catalog.roster(None, None).expect("the default");
  assert_eq!(
    (names_of(&models), actor),
    (vec!["openai:gpt-5".to_owned(), "openai:gpt-4o".to_owned(), "local:qwen".to_owned()], None),
    "a roster that the host does not name lists what the catalog offers, the local server after the network"
  );
  assert_eq!(told.try_iter().count(), 1);
  let offered: Vec<_> = catalog.offered().iter().map(|one| one.name.clone()).collect();
  assert_eq!(offered, names_of(&models));
  assert!(
    catalog.find("qwen").is_none(),
    "an id alone with no colon finds no model of the local server"
  );
  assert!(
    catalog.roster(None, Some("local:qwen/x")).is_err(),
    "a word after a slash that is no level names nothing"
  );
  let high = catalog.roster(None, Some("local:qwen/high")).map(|(_, actor)| actor);
  assert_eq!(high, Ok(Some("local:qwen".to_owned())));
  assert_eq!(catalog.find("local:qwen").map(|one| one.window), Some(1000));
  assert_eq!(told.try_iter().count(), 0, "a catalog asks the server once");
}

#[test]
fn the_default_actor_is_a_model_of_the_claude_command_line_then_of_the_network_then_of_the_local_server()
 {
  let (url, _) = qwen();
  let (models, _) = catalog(&[&url]).roster(None, None).expect("the default");
  assert_eq!(
    names_of(&models),
    ["local:qwen"],
    "the local server stands when nothing else is offered"
  );
  let claude = catalog(&[&url]).claude(Some(PathBuf::from("/bin/true")), None);
  let (models, _) = claude.roster(None, None).expect("the default");
  assert_eq!(names_of(&models)[0], "claude-cli:opus");
  assert_eq!(claude.find("local:qwen").map(|one| one.name.clone()).as_deref(), Some("local:qwen"));
}

#[test]
fn a_roster_that_the_host_does_not_name_offers_every_model_of_the_claude_command_line_that_it_finds()
 {
  let claude = catalog(&["OPENAI_API_KEY"]).claude(Some(PathBuf::from("/bin/true")), None);
  let (models, actor) = claude.roster(None, None).expect("the default");
  let all = [
    "claude-cli:opus",
    "claude-cli:sonnet",
    "claude-cli:haiku",
    "claude-cli:fable",
    "openai:gpt-5",
    "openai:gpt-4o",
  ];
  assert_eq!((names_of(&models), actor), (all.map(str::to_owned).to_vec(), None));
  let (models, actor) = claude.roster(None, Some("sonnet/medium")).expect("an actor of sonnet");
  assert_eq!(names_of(&models)[..2], ["claude-cli:sonnet", "claude-cli:opus"]);
  assert_eq!((models.len(), actor.as_deref()), (all.len(), Some("claude-cli:sonnet/medium")));
  let named = ["haiku".to_owned()];
  let (models, _) = claude.roster(Some(&named), Some("sonnet")).expect("a named roster");
  assert_eq!(
    names_of(&models),
    ["claude-cli:sonnet", "claude-cli:haiku"],
    "a named roster keeps its meaning"
  );
}

#[test]
fn a_roster_that_names_a_model_of_the_local_server_that_the_catalog_does_not_offer_says_why() {
  let port = dropping();
  let gone = catalog(&[&format!("FURB_LOCAL_URL=http://127.0.0.1:{port}")]);
  let no = gone.roster(Some(&["local:qwen".to_owned()]), None).err().unwrap_or_default();
  let why =
    format!("No model is local:qwen: the local server at http://127.0.0.1:{port} lists no model: ");
  assert!(no.starts_with(&why), "{no}");
  let bare = gone.roster(None, Some("qwen3:8b")).err().unwrap_or_default();
  assert!(bare.starts_with("No model is qwen3:8b: the local server at "), "{bare}");
  assert_eq!(
    catalog(&[]).roster(None, Some("local:qwen")).err().as_deref(),
    Some("No model is local:qwen: no local server is named: set FURB_LOCAL_URL.")
  );
  let (address, _) =
    fake(|request| request.starts_with("GET /v1/models ").then(|| json!({"data": [{"id": "z"}]})));
  let windowless = catalog(&[&format!("FURB_LOCAL_URL={address}")]);
  assert_eq!(
    windowless.roster(None, Some("local:z/high")).err().as_deref(),
    Some(
      "No model is local:z/high: the local server gives no window of it: set FURB_LOCAL_WINDOW."
    )
  );
}

#[test]
fn an_id_alone_of_the_local_server_whose_part_before_its_colon_is_a_provider_is_found_in_what_it_listed()
 {
  let list = json!({"data": [{"id": "google:gemma3", "max_model_len": 1000}]});
  let (address, told) =
    fake(move |request| request.starts_with("GET /v1/models ").then(|| list.clone()));
  let catalog = catalog(&[&format!("FURB_LOCAL_URL={address}")]);
  assert!(
    catalog.find("google:gemma3").is_none(),
    "a name of a provider of the network asks the server nothing"
  );
  assert_eq!(told.try_iter().count(), 0);
  assert_eq!(names_of(&catalog.offered()), ["local:google:gemma3"]);
  assert_eq!(told.try_iter().count(), 1);
  let found = catalog.find("google:gemma3").map(|one| one.name.clone());
  assert_eq!(
    (found.as_deref(), told.try_iter().count()),
    (Some("local:google:gemma3"), 0),
    "what it listed holds it"
  );
}
