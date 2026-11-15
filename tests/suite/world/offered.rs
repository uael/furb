//! What the catalog of a process offers, from the snapshot that the crate carries and the copy of
//! models.dev in its cache, with the credentials of its environment: each test runs again as a
//! child of itself, whose cache holds the listing of the tests, and says what it found.

#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{
  fs,
  io::{BufRead, BufReader, Write},
  net::TcpListener,
  path::{Path, PathBuf},
  thread,
  time::Duration,
};

#[cfg(unix)]
use furb::Text;
use furb::{
  life::Opening,
  world::{Catalog, Model, catalog::clamp},
};
use serde_json::{Value, json};

#[cfg(unix)]
use super::provider::closed;
use super::{
  catalog::{answered, body, chunk, ended, last, opened, replied, served, six},
  provider::{asked, yard},
};
#[cfg(unix)]
use crate::claude::Yard as Fake;
use crate::{
  child::{child, here, ran},
  drive::park,
  yard::Yard,
};

/// The listing of the tests as the copy of models.dev holds it, newer than the snapshot: a provider
/// of each dialect, and models that think in each way.
const LISTING: &str = r#"{
"anthropic": {"models": {
"claude-opus-t5": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "medium", "high", "xhigh", "max"]}], "limit": {"context": 1000000, "output": 128000}, "cost": {"input": 5, "output": 25, "cache_read": 0.5, "cache_write": 6.25}, "modalities": {"input": ["text", "image"], "output": ["text"]}},
"claude-haiku-4-5": {"reasoning": true, "reasoning_options": [{"type": "budget_tokens", "min": 1024}], "limit": {"context": 200000, "output": 64000}, "modalities": {"input": ["text"], "output": ["text"]}},
"claude-sonnet-5": {"reasoning": true, "reasoning_options": [{"type": "toggle"}, {"type": "effort", "values": ["low", "high"]}], "limit": {"context": 1000000, "output": 128000}, "modalities": {"input": ["text"], "output": ["text"]}},
"claude-old": {"status": "deprecated", "limit": {"context": 100000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"openai": {"models": {
"gpt-5": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["none", "minimal", "low", "high"]}], "limit": {"context": 400000, "output": 128000}, "modalities": {"input": ["text", "image"], "output": ["text"]}},
"gpt-4o": {"reasoning": false, "limit": {"context": 128000, "output": 16384}, "modalities": {"input": ["text"], "output": ["text"]}},
"gpt-image": {"limit": {"context": 0}, "modalities": {"input": ["text"], "output": ["image"]}}
}},
"google": {"models": {
"gemini-flash": {"reasoning": true, "reasoning_options": [{"type": "budget_tokens"}], "limit": {"context": 1000000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"openrouter": {"models": {
"x/thinks": {"reasoning": true, "reasoning_options": [{"type": "toggle"}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}},
"x/gpt-4o": {"reasoning": true, "reasoning_options": [{"type": "budget_tokens"}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"fireworks-ai": {"models": {
"gpt-4o": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "default"]}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"azure": {"models": {
"gpt-5-mini": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}},
"claude-in-azure": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}, "provider": {"npm": "@ai-sdk/anthropic", "api": "https://${AZURE_RESOURCE_NAME}.services.ai.azure.com/anthropic/v1"}},
"model-router": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}, "provider": {"npm": "@ai-sdk/openai-compatible", "shape": "completions"}}
}},
"amazon-bedrock": {"models": {
"us.anthropic.claude-sonnet-5": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}], "limit": {"context": 8000, "output": 4000}, "modalities": {"input": ["text"], "output": ["text"]}},
"qwen.qwen3": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}], "limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"google-vertex": {"models": {
"gemini-pro": {"limit": {"context": 8000}, "modalities": {"input": ["text"], "output": ["text"]}}
}},
"radius": {"models": {
"balanced": {"reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high", "max"]}], "limit": {"context": 8000, "output": 1000}, "modalities": {"input": ["text"], "output": ["text"]}}
}}
}"#;

/// The listing of the tests as the copy of models.dev holds it, each model updated on a day that
/// no snapshot reaches, so it holds over the snapshot.
pub(super) fn listing() -> String {
  let mut listing: Value = serde_json::from_str(LISTING).expect("the listing is JSON");
  for provider in listing.as_object_mut().into_iter().flat_map(|all| all.values_mut()) {
    for model in provider["models"].as_object_mut().into_iter().flat_map(|all| all.values_mut()) {
      model["last_updated"] = json!("2999-01-01");
    }
  }
  listing.to_string()
}

/// A yard of a test whose cache holds this copy of models.dev.
pub(super) fn cached(name: &str, copy: &str) -> Yard {
  let at = yard(name);
  fs::create_dir_all(at.join("cache/furb")).expect("the cache of furb");
  fs::write(at.join("cache/furb/models.json"), copy).expect("the copy of models.dev");
  at
}

/// The yard of the child of the test at `test` that ran in it, whose cache holds this copy of
/// models.dev, with these variables, each `NAME` with a key or `NAME=value`, and no claude command
/// line.
fn run(test: &str, at: Yard, vars: &[&str]) -> Yard {
  let vars: Vec<(&str, &str)> = (vars.iter())
    .map(|one| one.split_once('=').unwrap_or((one, "key")))
    .chain([("FURB_CLAUDE_BIN", "")])
    .collect();
  ran(test, &at, &vars);
  at
}

/// What the child of the test at `test` says, run as [`run`] runs it in a yard whose cache holds
/// this copy of models.dev.
fn told(test: &str, name: &str, copy: &str, vars: &[&str]) -> Value {
  let at = run(test, cached(name, copy), vars);
  serde_json::from_str(&fs::read_to_string(at.join("said")).expect("the child said"))
    .expect("the child said JSON")
}

/// Says a value to the test that ran this child.
fn say(at: &Path, value: &Value) {
  fs::write(at.join("said"), value.to_string()).expect("the child says");
}

/// The names of these models.
fn names(models: &[Model]) -> Vec<String> {
  models.iter().map(|one| one.name().to_owned()).collect()
}

#[test]
fn the_catalog_offers_a_provider_whose_credential_stands_and_names_each_model_as_provider_and_id() {
  if let Some(at) = child() {
    say(&at, &json!(names(&Catalog::load(None).offered())));
    return;
  }
  let test = here!();
  let offered = |name: &str, vars: &[&str]| told(test, name, &listing(), vars);
  assert_eq!(offered("offered-none", &[]), json!([]));
  assert_eq!(offered("offered-google", &["GEMINI_API_KEY"]), json!(["google:gemini-flash"]));
  assert_eq!(
    offered("offered-two", &["OPENAI_API_KEY", "ANTHROPIC_API_KEY"]),
    json!([
      "anthropic:claude-opus-t5",
      "anthropic:claude-haiku-4-5",
      "anthropic:claude-sonnet-5",
      "openai:gpt-5",
      "openai:gpt-4o"
    ]),
    "a deprecated model and a model that writes no text are not kept"
  );
}

#[test]
fn a_name_finds_a_model_by_its_provider_and_its_id_or_by_an_id_that_one_model_alone_holds() {
  if let Some(_at) = child() {
    let catalog = Catalog::load(None);
    let found = |name: &str| catalog.find(name).map(|model| model.name().to_owned());
    assert_eq!(found("openai:gpt-4o").as_deref(), Some("openai:gpt-4o"));
    assert_eq!(
      found("opus").as_deref(),
      Some("claude-cli:opus"),
      "the catalog knows the claude command line it did not find"
    );
    assert_eq!(found("claude-opus-t5").as_deref(), Some("anthropic:claude-opus-t5"));
    assert_eq!(found("x/thinks").as_deref(), Some("openrouter:x/thinks"));
    assert_eq!(found("gpt-4o"), None, "two providers hold that id");
    assert_eq!(found("claude-old"), None, "a deprecated model is not kept");
    return;
  }
  run(here!(), cached("found", &listing()), &[]);
}

#[test]
fn a_model_has_the_window_the_price_and_the_images_of_the_catalog() {
  if let Some(_at) = child() {
    let catalog = Catalog::load(None);
    let opus = catalog.find("anthropic:claude-opus-t5").expect("opus");
    assert_eq!(
      (opus.window(), opus.sees(), opus.priced()),
      (1_000_000, true, Some([5.0, 25.0, 0.5, 6.25]))
    );
    let gpt = catalog.find("openai:gpt-4o").expect("gpt-4o");
    assert_eq!((gpt.window(), gpt.sees(), gpt.priced()), (128_000, false, None));
    return;
  }
  run(here!(), cached("window", &listing()), &[]);
}

#[test]
fn a_roster_holds_the_model_of_its_actor_first_and_the_models_a_host_names_or_every_model_the_catalog_offers()
 {
  if let Some(_at) = child() {
    let catalog = Catalog::load(None);
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
    let named = ["claude-opus-t5".to_owned(), "openai:gpt-5".to_owned()];
    let (models, actor) =
      catalog.roster(Some(&named), Some("x/thinks/high")).expect("a named roster");
    assert_eq!(names(&models), ["openrouter:x/thinks", "anthropic:claude-opus-t5", "openai:gpt-5"]);
    assert_eq!(
      actor.as_deref(),
      Some("openrouter:x/thinks"),
      "a model that takes no effort is named alone"
    );
    let (models, actor) =
      catalog.roster(Some(&named), Some("openai:gpt-5/medium")).expect("the actor is named");
    assert_eq!((models.len(), actor.as_deref()), (2, Some("openai:gpt-5/high")));
    let (_, actor) = catalog.roster(Some(&named), Some("claude-opus-t5")).expect("the model alone");
    assert_eq!(
      actor.as_deref(),
      Some("anthropic:claude-opus-t5/high"),
      "an actor that names no effort takes high"
    );
    let no = catalog.roster(Some(&["nothing".to_owned()]), None).err().map(|no| no.to_string());
    assert_eq!(no.as_deref(), Some("No model is nothing. Name one as provider:id."));
    return;
  }
  run(here!(), cached("roster", &listing()), &["OPENAI_API_KEY"]);
}

#[test]
fn a_catalog_keeps_the_names_it_finds_and_names_each_one_that_left_once() {
  if let Some(_at) = child() {
    let catalog = Catalog::load(None);
    let named = ["openai:gpt-5".to_owned(), "nothing".to_owned()];
    let kept = catalog.keep(Some(&named), Some("nothing"));
    assert_eq!(kept.names, Some(vec!["openai:gpt-5".to_owned()]));
    assert_eq!((kept.actor, kept.left), (None, vec!["nothing".to_owned()]));
    let kept = catalog.keep(None, Some("openai:gpt-5/medium"));
    assert_eq!((kept.names, kept.actor), (None, Some("openai:gpt-5/medium".to_owned())));
    assert!(kept.left.is_empty());
    return;
  }
  run(here!(), cached("keep", &listing()), &[]);
}

#[test]
fn the_cache_holds_the_models_of_the_providers_of_the_snapshot_and_nothing_more() {
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    let held = ["openai:gpt-6", "openai:gpt-4o", "anthropic:claude-haiku-4-5", "stranger:m"];
    let found: Vec<bool> = held.iter().map(|name| catalog.find(name).is_some()).collect();
    say(&at, &json!(found));
    return;
  }
  let test = here!();
  let cached = r#"{
    "openai": {"id": "openai", "models": {"gpt-6": {"reasoning": false, "limit": {"context": 9}, "modalities": {"input": ["text"], "output": ["text"]}, "last_updated": "2999-01-01"}}},
    "stranger": {"models": {"m": {"modalities": {"input": ["text"], "output": ["text"]}, "last_updated": "2999-01-01"}}}
  }"#;
  assert_eq!(
    told(test, "cache-new", cached, &[]),
    json!([true, false, true, false]),
    "the cache holds the models of openai, a provider it holds nothing of keeps the snapshot, and \
     a provider that the snapshot lacks is not kept"
  );
  assert_eq!(
    told(test, "cache-broken", "not json", &[]),
    json!([false, true, true, false]),
    "a cache that is not JSON leaves the snapshot"
  );
}

#[test]
fn the_newer_of_the_cache_and_the_snapshot_holds_for_each_provider() {
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    let held = ["anthropic:claude-cached", "anthropic:claude-haiku-4-5"];
    let found: Vec<bool> = held.iter().map(|name| catalog.find(name).is_some()).collect();
    say(&at, &json!(found));
    return;
  }
  let test = here!();
  let snapshot: Value =
    serde_json::from_str(include_str!("../../../src/world/provider/catalog.json"))
      .expect("the snapshot is JSON");
  let models = snapshot["anthropic"]["models"].as_object().expect("the models of anthropic");
  let newest = models.values().filter_map(|one| one["last_updated"].as_str()).max();
  let newest = newest.expect("a day of the snapshot");
  let held = |name: &str, day: &str| {
    let model = json!({"limit": {"context": 9}, "modalities": {"input": ["text"], "output": ["text"]},
      "last_updated": day});
    let copy = json!({"anthropic": {"models": {"claude-cached": model}}}).to_string();
    told(test, name, &copy, &[])
  };
  assert_eq!(
    held("older", "2000-01-01"),
    json!([false, true]),
    "a cache older than the snapshot hides nothing of it"
  );
  assert_eq!(held("same", newest), json!([true, false]));
  assert_eq!(held("newer", "2999-01-01"), json!([true, false]));
}

#[test]
fn a_provider_is_offered_when_the_environment_holds_its_credential_and_each_place_of_its_address() {
  if let Some(at) = child() {
    say(&at, &json!(names(&Catalog::load(None).offered())));
    return;
  }
  let test = here!();
  let offered = |name: &str, vars: &[&str], model: &str| {
    let said = told(test, name, &listing(), vars);
    said.as_array().is_some_and(|all| all.iter().any(|one| one == model))
  };
  assert!(
    !offered("azure-bare", &["AZURE_OPENAI_API_KEY"], "azure:gpt-5-mini"),
    "the address needs its resource"
  );
  let resource = ["AZURE_OPENAI_API_KEY", "AZURE_RESOURCE_NAME=r"];
  assert!(offered("azure", &resource, "azure:gpt-5-mini"));
  let alias = ["AZURE_OPENAI_API_KEY", "AZURE_OPENAI_RESOURCE_NAME=r"];
  assert!(offered("azure-alias", &alias, "azure:claude-in-azure"));
  assert!(
    !offered("azure-shape", &resource, "azure:model-router"),
    "no client of rig asks a model of a shape of its own"
  );
  assert!(offered("bedrock", &["AWS_PROFILE"], "amazon-bedrock:qwen.qwen3"));
  assert!(
    !offered("vertex-bare", &["GOOGLE_CLOUD_API_KEY"], "google-vertex:gemini-pro"),
    "Google asks a project"
  );
  let project = ["GOOGLE_CLOUD_API_KEY", "GOOGLE_CLOUD_PROJECT=p"];
  assert!(offered("vertex", &project, "google-vertex:gemini-pro"));
  let home = yard("adc-home");
  let (root, gcloud) =
    if cfg!(windows) { ("APPDATA", "gcloud") } else { ("HOME", ".config/gcloud") };
  fs::create_dir_all(home.join(gcloud)).expect("the folder of gcloud");
  fs::write(home.join(gcloud).join("application_default_credentials.json"), "{}").expect("a file");
  let adc = format!("{root}={}", home.display());
  assert!(
    offered("vertex-adc", &[&adc, "GOOGLE_CLOUD_PROJECT=p"], "google-vertex:gemini-pro"),
    "the credentials of an application stand where the client of Google finds them"
  );
  assert!(offered("radius", &["RADIUS_API_KEY"], "radius:balanced"));
  let place = ["AZURE_OPENAI_API_KEY", "AZURE_RESOURCE_NAME=${AZURE_RESOURCE_NAME}"];
  assert!(
    offered("azure-place", &place, "azure:gpt-5-mini"),
    "each place of an address is filled once, so a value that names a place stands as it is"
  );
}

#[test]
fn the_snapshot_that_the_crate_carries_names_a_client_of_rig_for_each_provider() {
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    let setup = catalog.setup();
    // The command line of Codex signs in with an account of its own, and names no credential.
    let named = setup.providers.iter().filter(|one| one.provider != "codex");
    assert!(setup.providers.len() >= 40 && named.clone().all(|one| !one.env.is_empty()));
    let opus = catalog.find("anthropic:claude-opus-5").expect("the snapshot holds opus");
    assert!(opus.window() >= 200_000 && opus.sees() && opus.priced().is_some());
    let radius =
      catalog.find("radius:balanced").expect("the data of pi-ai gives the models of Radius");
    assert!(radius.efforts().contains(&"high"));
    let offered = catalog.offered();
    let offering: Vec<&str> =
      offered.iter().map(|one| one.name().split(':').next().unwrap_or_default()).collect();
    let silent: Vec<&str> = (setup.providers.iter())
      .filter(|one| one.enabled && !offering.contains(&one.provider.as_str()))
      .map(|one| one.provider.as_str())
      .collect();
    say(&at, &json!(silent));
    return;
  }
  // Each credential and each place of an address that the snapshot names, and the project of
  // Google: each provider whose client of rig needs an address finds one.
  let snapshot: Value =
    serde_json::from_str(include_str!("../../../src/world/provider/catalog.json"))
      .expect("the snapshot is JSON");
  let text = snapshot.to_string();
  let mut vars: Vec<String> = text
    .split("${")
    .skip(1)
    .filter_map(|one| Some(format!("{}=p", one.split_once('}')?.0)))
    .chain(["GOOGLE_CLOUD_PROJECT=p".to_owned()])
    .collect();
  for provider in snapshot.as_object().into_iter().flat_map(|all| all.values()) {
    let names = provider["env"].as_array().into_iter().flatten().filter_map(Value::as_str);
    vars.extend(names.map(str::to_owned));
  }
  vars.sort();
  vars.dedup();
  let vars: Vec<&str> = vars.iter().map(String::as_str).collect();
  assert_eq!(
    told(here!(), "snapshot", "{}", &vars),
    json!([]),
    "each provider that stands offers a model once its credential and its places stand"
  );
}

/// A local server on a port of this machine, which lists these models and writes the first line of
/// each request that it reads in the file `heard` of the yard before it answers; and its address as
/// the environment names it. It serves until the test ends.
fn local(at: &Path, models: Value) -> String {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let kept = at.join("heard");
  thread::spawn(move || {
    for stream in listener.incoming().flatten() {
      let mut reader = BufReader::new(stream);
      let mut head = String::new();
      while !head.ends_with("\r\n\r\n") && reader.read_line(&mut head).is_ok_and(|read| read > 0) {}
      let first = head.lines().next().unwrap_or_default().to_owned();
      let mut file = fs::OpenOptions::new().create(true).append(true).open(&kept).expect("heard");
      writeln!(file, "{first}").expect("the request is heard");
      let body = if first.starts_with("GET /v1/models ") {
        json!({"data": models}).to_string()
      } else {
        String::new()
      };
      let status = if body.is_empty() { "404 Not Found" } else { "200 OK" };
      let answer = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
         connection: close\r\n\r\n{body}",
        body.len()
      );
      reader.get_mut().write_all(answer.as_bytes()).expect("the server answers");
    }
  });
  format!("FURB_LOCAL_URL=http://127.0.0.1:{port}")
}

/// How many requests the local server of the yard read.
fn heard(at: &Path) -> usize {
  fs::read_to_string(at.join("heard")).map_or(0, |all| all.lines().count())
}

/// The model qwen of a local server, with its window.
fn qwen() -> Value {
  json!([{"id": "qwen", "max_model_len": 1000}])
}

/// A program that ends at once, which stands for the claude command line.
fn found() -> PathBuf {
  PathBuf::from("/bin/true")
}

#[test]
fn the_catalog_asks_the_local_server_only_when_a_name_needs_it_or_a_host_lists_what_it_offers() {
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    let named = ["openai:gpt-4o".to_owned()];
    catalog.roster(Some(&named), Some("openai:gpt-5")).expect("a roster of the network");
    catalog
      .roster(Some(&[]), Some("openai:gpt-5/high"))
      .expect("an actor of the network at an effort");
    assert!(catalog.find("openai:gpt-6").is_none() && catalog.find("gpt-6").is_none());
    assert_eq!(heard(&at), 0, "a roster that names no model of the local server asks it nothing");
    let (models, actor) = catalog.roster(None, None).expect("the default");
    assert_eq!(
      (names(&models), actor),
      (vec!["openai:gpt-5".to_owned(), "openai:gpt-4o".to_owned(), "local:qwen".to_owned()], None),
      "a roster that the host does not name lists what the catalog offers, the local server after \
       the network"
    );
    assert_eq!(heard(&at), 1);
    assert_eq!(names(&catalog.offered()), names(&models));
    assert!(
      catalog.find("qwen").is_none(),
      "an id alone with no colon finds no model of the local server"
    );
    assert!(
      catalog.roster(None, Some("local:qwen/x")).is_err(),
      "a word after a slash that is no level names nothing"
    );
    let high = catalog.roster(None, Some("local:qwen/high")).map(|(_, actor)| actor);
    assert_eq!(
      high.ok(),
      Some(Some("local:qwen/high".to_owned())),
      "a level after a slash is an effort of the model"
    );
    assert_eq!(catalog.find("local:qwen").map(|one| one.window()), Some(1000));
    assert_eq!(heard(&at), 1, "a catalog asks the server once");
    return;
  }
  let at = cached("asks-local", &listing());
  let url = local(&at, qwen());
  run(here!(), at, &[&url, "OPENAI_API_KEY"]);
}

#[test]
fn the_default_actor_is_a_model_of_the_claude_command_line_then_of_the_network_then_of_the_local_server()
 {
  if let Some(_at) = child() {
    let (models, _) = Catalog::load(None).roster(None, None).expect("the default");
    assert_eq!(
      names(&models),
      ["local:qwen"],
      "the local server stands when nothing else is offered"
    );
    let claude = Catalog::load(None).claude(Some(found()), None);
    let (models, _) = claude.roster(None, None).expect("the default");
    assert_eq!(names(&models)[0], "claude-cli:opus");
    assert_eq!(
      claude.find("local:qwen").map(|one| one.name().to_owned()).as_deref(),
      Some("local:qwen")
    );
    return;
  }
  let at = cached("default-actor", &listing());
  let url = local(&at, qwen());
  run(here!(), at, &[&url]);
}

#[test]
fn a_roster_that_the_host_does_not_name_offers_every_model_of_the_claude_command_line_that_it_finds()
 {
  if let Some(_at) = child() {
    let claude = Catalog::load(None).claude(Some(found()), None);
    let (models, actor) = claude.roster(None, None).expect("the default");
    let all = [
      "claude-cli:opus",
      "claude-cli:sonnet",
      "claude-cli:haiku",
      "claude-cli:fable",
      "openai:gpt-5",
      "openai:gpt-4o",
    ];
    assert_eq!((names(&models), actor), (all.map(str::to_owned).to_vec(), None));
    let (models, actor) = claude.roster(None, Some("sonnet/medium")).expect("an actor of sonnet");
    assert_eq!(names(&models)[..2], ["claude-cli:sonnet", "claude-cli:opus"]);
    assert_eq!((models.len(), actor.as_deref()), (all.len(), Some("claude-cli:sonnet/medium")));
    let named = ["haiku".to_owned()];
    let (models, _) = claude.roster(Some(&named), Some("sonnet")).expect("a named roster");
    assert_eq!(
      names(&models),
      ["claude-cli:sonnet", "claude-cli:haiku"],
      "a named roster keeps its meaning"
    );
    return;
  }
  run(here!(), cached("claude-roster", &listing()), &["OPENAI_API_KEY"]);
}

#[test]
fn a_roster_that_names_a_model_of_the_local_server_that_the_catalog_does_not_offer_says_why() {
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    let said = |names: Option<&[String]>, actor: Option<&str>| {
      catalog.roster(names, actor).err().map(|no| no.to_string()).unwrap_or_default()
    };
    let named = ["local:qwen".to_owned()];
    say(
      &at,
      &json!([
        said(Some(&named), None),
        said(None, Some("qwen3:8b")),
        said(None, Some("local:z/high"))
      ]),
    );
    return;
  }
  let test = here!();
  let gone = {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
    listener.local_addr().expect("the port").port()
  };
  let said =
    told(test, "local-gone", &listing(), &[&format!("FURB_LOCAL_URL=http://127.0.0.1:{gone}")]);
  let why =
    format!("No model is local:qwen: the local server at http://127.0.0.1:{gone} lists no model: ");
  assert!(said[0].as_str().is_some_and(|one| one.starts_with(&why)), "{said}");
  let bare = "No model is qwen3:8b: the local server at ";
  assert!(said[1].as_str().is_some_and(|one| one.starts_with(bare)), "{said}");
  let said = told(test, "local-none", &listing(), &[]);
  assert_eq!(said[0], "No model is local:qwen: no local server is named: set FURB_LOCAL_URL.");
  let at = cached("local-windowless", &listing());
  let url = local(&at, json!([{"id": "z"}]));
  let at = run(test, at, &[&url]);
  let said: Value = serde_json::from_str(&fs::read_to_string(at.join("said")).expect("said"))
    .expect("the child said JSON");
  assert_eq!(
    said[2],
    "No model is local:z/high: the local server gives no window of it: set FURB_LOCAL_WINDOW."
  );
}

#[test]
fn an_id_alone_of_the_local_server_whose_part_before_its_colon_is_a_provider_is_found_in_what_it_listed()
 {
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    assert!(
      catalog.find("google:gemma3").is_none(),
      "a name of a provider of the network asks the server nothing"
    );
    assert_eq!(heard(&at), 0);
    assert_eq!(names(&catalog.offered()), ["local:google:gemma3"]);
    assert_eq!(heard(&at), 1);
    let found = catalog.find("google:gemma3").map(|one| one.name().to_owned());
    assert_eq!(
      (found.as_deref(), heard(&at)),
      (Some("local:google:gemma3"), 1),
      "what it listed holds it"
    );
    return;
  }
  let at = cached("local-prefixed", &listing());
  let url = local(&at, json!([{"id": "google:gemma3", "max_model_len": 1000}]));
  run(here!(), at, &[&url]);
}

#[test]
fn a_first_run_checks_the_claude_command_line_each_provider_and_the_local_server() {
  if let Some(at) = child() {
    let setup = Catalog::load(None).setup();
    let claude = Catalog::load(None).claude(Some(found()), None).setup();
    let ready: Vec<Value> = (setup.providers.iter().take(3))
      .map(|one| json!([one.provider, one.env.join(" "), one.offered]))
      .collect();
    let local = setup.local.map(|one| json!([one.address, one.offered, one.failed]));
    say(
      &at,
      &json!({"setup": [setup.claude, setup.signed], "claude": [claude.claude, claude.signed],
      "ready": ready, "local": local}),
    );
    return;
  }
  let test = here!();
  let none = told(test, "setup-none", &listing(), &["ANTHROPIC_API_KEY"]);
  assert_eq!(
    none["setup"],
    json!([false, false]),
    "no claude command line is found, so none is signed in"
  );
  assert_eq!(
    none["ready"][0],
    json!(["anthropic", "ANTHROPIC_AUTH_TOKEN ANTHROPIC_API_KEY", 3]),
    "a provider whose credential stands"
  );
  assert_eq!(
    none["ready"][1],
    json!(["openai", "OPENAI_API_KEY", 0]),
    "a provider whose credential is missing"
  );
  assert_eq!(
    none["ready"][2][1], "GOOGLE_API_KEY GOOGLE_GENERATIVE_AI_API_KEY GEMINI_API_KEY",
    "each name of a credential"
  );
  assert_eq!(none["local"], Value::Null, "the environment names no local server");
  assert_eq!(
    none["claude"],
    json!([true, true]),
    "the claude command line is found at its path, and one that says nothing of its sign in passes"
  );
  let at = cached("setup-local", &listing());
  let url = local(&at, qwen());
  let at = run(test, at, &[&url]);
  let said: Value = serde_json::from_str(&fs::read_to_string(at.join("said")).expect("said"))
    .expect("the child said JSON");
  assert_eq!(
    (&said["local"][1], &said["local"][2]),
    (&json!(1), &Value::Null),
    "the local server lists its models"
  );
  let gone = told(test, "setup-gone", &listing(), &["FURB_LOCAL_URL=http://127.0.0.1:9"]);
  assert_eq!((&gone["local"][0], &gone["local"][1]), (&json!("http://127.0.0.1:9"), &json!(0)));
  assert!(
    gone["local"][2].as_str().is_some_and(|why| why.contains("lists no model")),
    "a server that does not answer says why: {gone}"
  );
}

#[cfg(unix)]
#[test]
fn a_first_run_reads_whether_the_claude_command_line_is_signed_in() {
  use std::os::unix::fs::PermissionsExt;
  if let Some(at) = child() {
    let signed = |name: &str| Catalog::load(None).claude(Some(at.join(name)), None).setup().signed;
    assert!(!signed("words"), "it says that it is not logged in");
    assert!(!signed("json"), "its JSON says that it is not logged in");
    assert!(signed("in"), "its JSON says that it is logged in");
    assert!(signed("old"), "a version with no such command passes");
    return;
  }
  let at = cached("signed", &listing());
  for (name, script) in [
    ("words", "echo 'Not logged in'; exit 1"),
    ("json", r#"echo '{"loggedIn": false}'; exit 1"#),
    ("in", r#"echo '{"loggedIn": true}'"#),
    ("old", "echo 'unknown command: auth' >&2; exit 1"),
  ] {
    let bin = at.join(name);
    fs::write(&bin, format!("#!/bin/sh\n{script}\n")).expect("the program");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("its mode");
  }
  run(here!(), at, &[]);
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

/// What a child says of the configs of the project of its yard, each written in turn as the file
/// `.furb/config.json`: whether the catalog takes it, or the first line of why it refuses it.
fn configs(test: &str, name: &str, configs: &[Value]) -> Vec<String> {
  let at = cached(name, &listing());
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  fs::write(at.join("configs.json"), json!(configs).to_string()).expect("the configs");
  let at = run(test, at, &[]);
  let said = fs::read_to_string(at.join("said")).expect("the child said");
  serde_json::from_str(&said).expect("the child said JSON")
}

/// Says, for each config of the file `configs.json` of the yard, whether the catalog takes it as
/// the config of the project, or why it refuses it.
fn configured(at: &Path) {
  let all = fs::read_to_string(at.join("configs.json")).expect("the configs");
  let all: Vec<Value> = serde_json::from_str(&all).expect("the configs are JSON");
  let mut said = Vec::new();
  for config in all {
    fs::write(at.join(".furb/config.json"), config.to_string()).expect("the config");
    let taken = Catalog::load(None).configured(at, Some(at));
    said.push(taken.map_or_else(|no| no.to_string(), |_| "taken".to_owned()));
  }
  say(at, &json!(said));
}

#[test]
fn an_invalid_claude_command_is_refused_before_it_starts_a_process() {
  if let Some(at) = child() {
    configured(&at);
    return;
  }
  let commands =
    [json!([]), json!(""), json!([" "]), json!(12), json!(["claude", 3]), json!(["claude", "\0"])];
  let all: Vec<Value> =
    commands.iter().map(|one| json!({"providers": {"claude-cli": {"command": one}}})).collect();
  for (said, command) in configs(here!(), "claude-command", &all).iter().zip(&commands) {
    assert!(said.contains("claude-cli"), "{command} was not refused: {said}");
  }
}

#[test]
fn invalid_claude_accounts_are_refused_before_they_start_a_process() {
  if let Some(at) = child() {
    configured(&at);
    return;
  }
  let one = json!({"name": "a", "token_env": "X"});
  let settings = [
    json!({"accounts": []}),
    json!({"accounts": [{"name": "a"}]}),
    json!({"accounts": [{"name": "a", "token_env": "X", "token_file": "/x"}]}),
    json!({"accounts": [{"name": "a", "token_file": "relative"}]}),
    json!({"accounts": [{"name": "a", "token_env": ""}]}),
    json!({"accounts": [one, one]}),
    json!({"accounts": [{"name": "two words", "token_env": "X"}]}),
    json!({"accounts": [{"name": "a", "token_env": "X", "other": 1}]}),
    json!({"accounts": [one], "switch": "random"}),
    json!({"switch": "spread"}),
  ];
  let all: Vec<Value> =
    settings.iter().map(|one| json!({"providers": {"claude-cli": one}})).collect();
  for (said, settings) in configs(here!(), "claude-accounts", &all).iter().zip(&settings) {
    assert!(said.contains("claude-cli"), "{settings} was not refused: {said}");
  }
}

#[test]
fn the_local_config_of_a_project_wins_over_its_config_for_a_provider() {
  if let Some(at) = child() {
    let configured =
      || Catalog::load(None).configured(&at, Some(&at)).expect("the settings are valid");
    let local = at.join(".furb/config.local.json");
    assert!(
      configured().find("strata:qwen").is_some(),
      "the config of the project gives the provider"
    );
    fs::write(&local, json!({"providers": {"strata": false}}).to_string())
      .expect("the local config");
    assert!(configured().find("strata:qwen").is_none(), "the local config turns the provider off");
    let models = json!({"providers": {"strata": {"models": {"other": {"window": 8192}}}}});
    fs::write(&local, models.to_string()).expect("the local config");
    let got = configured();
    assert!(
      got.find("strata:other").is_some() && got.find("strata:qwen").is_none(),
      "the local config wins for each field that it gives"
    );
    return;
  }
  let at = cached("local-config", &listing());
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  let provider = json!({
    "protocol": "chat", "url": "http://127.0.0.1:9/v1", "key": "k", "models": {"qwen": {"window": 262_144}}
  });
  let config = json!({"providers": {"strata": provider}}).to_string();
  fs::write(at.join(".furb/config.json"), config).expect("the config of the project");
  run(here!(), at, &[]);
}

/// A refusal of a provider, which no failure that passes reads, so the turn ends at once.
const REFUSED: &str = "HTTP/1.1 400 Bad Request\r\ncontent-type: application/json\r\n\
  content-length: 2\r\nconnection: close\r\n\r\n{}";

/// The body of the first request of each life that the child of the test at `test` opens, one for
/// each of these actors in turn, at a provider on a port of this machine of its own, which the
/// local config of the project names as the address of the provider of the actor, with these
/// variables, in a yard whose cache holds the listing of the tests; each answered
/// with a refusal.
fn requested(test: &str, at: Yard, actors: &[&str], vars: &[&str]) -> Vec<Value> {
  let served: Vec<_> = actors.iter().map(|_| served(vec![REFUSED.to_owned()])).collect();
  let ports: Vec<u16> = served.iter().map(|(port, _)| *port).collect();
  fs::write(at.join("asks.json"), json!({"ports": ports, "actors": actors}).to_string())
    .expect("the ports");
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  run(test, at, vars);
  // The child ended once each reply was done, so a provider that was asked has answered, and one
  // that was not never will.
  for ((_, asked), actor) in served.iter().zip(actors) {
    let answered = (0..500).any(|_| {
      asked.is_finished() || {
        park(Duration::from_millis(10));
        false
      }
    });
    assert!(answered, "{actor} was not asked at the address of its config");
  }
  let asked = served.into_iter().map(|(_, asked)| asked.join().expect("the provider read"));
  asked.map(|one| body(&one[0])).collect()
}

/// One turn of each actor that the test gave, each in a life of its own on the yard of the child,
/// whose local config names the next port that the test gave as the address of the provider of
/// the actor; each life ends once its reply is done.
fn asks(at: &Path) {
  let given = fs::read_to_string(at.join("asks.json")).expect("the actors");
  let given: Value = serde_json::from_str(&given).expect("the actors are JSON");
  let actors = given["actors"].as_array().into_iter().flatten().filter_map(Value::as_str);
  for (actor, port) in actors.zip(given["ports"].as_array().into_iter().flatten()) {
    let provider = actor.split(':').next().unwrap_or_default();
    let url = json!({"url": format!("http://127.0.0.1:{port}/v1")});
    let config = json!({"providers": {provider: url}}).to_string();
    fs::write(at.join(".furb/config.local.json"), config).expect("the local config");
    let opening = Opening { actor: Some(actor.to_owned()), ..opened(at) };
    let mut engine = opening.boot::<String>([]).expect("the life opens").0;
    asked(&mut engine);
    replied(&mut engine);
  }
}

#[test]
fn each_provider_reads_an_effort_in_its_own_words() {
  let actors = [
    "anthropic:claude-opus-t5/max",
    "anthropic:claude-haiku-4-5/off",
    "anthropic:claude-haiku-4-5/minimal",
    "anthropic:claude-haiku-4-5/low",
    "anthropic:claude-haiku-4-5/medium",
    "anthropic:claude-haiku-4-5/high",
    "anthropic:claude-sonnet-5/off",
    "anthropic:claude-sonnet-5/low",
    "anthropic:claude-sonnet-5/high",
    "openai:gpt-5/off",
  ];
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    let efforts = |name: &str| {
      catalog
        .find(name)
        .expect("the model")
        .efforts()
        .iter()
        .map(|one| (*one).to_owned())
        .collect::<Vec<_>>()
    };
    assert_eq!(efforts("anthropic:claude-opus-t5"), ["low", "medium", "high", "xhigh", "max"]);
    assert_eq!(efforts("openai:gpt-5"), ["off", "minimal", "low", "high"]);
    assert_eq!(
      efforts("openai:gpt-4o"),
      Vec::<String>::new(),
      "a model that does not reason takes no effort"
    );
    asks(&at);
    return;
  }
  let vars = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"];
  let asked = requested(here!(), cached("efforts", &listing()), &actors, &vars);
  let adaptive =
    |level: &str| json!({"output_config": {"effort": level}, "thinking": {"type": "adaptive"}});
  let budget = |tokens: u64| json!({"thinking": {"type": "enabled", "budget_tokens": tokens}});
  let read: Vec<Value> = asked.iter().map(effort).collect();
  assert_eq!(
    read,
    [
      adaptive("max"),
      json!({}),
      budget(1024),
      budget(2048),
      budget(8192),
      budget(16384),
      json!({"thinking": {"type": "disabled"}}),
      adaptive("low"),
      adaptive("high"),
      json!({"reasoning": {"effort": "none"}}),
    ]
  );
}

/// What a request says of the effort of its model, in the words of its provider.
fn effort(body: &Value) -> Value {
  let mut kept = serde_json::Map::new();
  for key in ["thinking", "output_config", "reasoning", "reasoning_effort", "chat_template_kwargs"]
  {
    if !body[key].is_null() {
      kept.insert(key.to_owned(), body[key].clone());
    }
  }
  if let Some(thinking) = body.pointer("/generationConfig/thinkingConfig") {
    kept.insert("generationConfig".to_owned(), json!({"thinkingConfig": thinking}));
  }
  if let Some(level) = body.pointer("/options/reasoning") {
    kept.insert("reasoning".to_owned(), level.clone());
  }
  Value::Object(kept)
}

#[test]
fn each_other_provider_reads_an_effort_in_its_own_words() {
  let actors = [
    "google:gemini-flash/low",
    "openrouter:x/thinks",
    "openrouter:x/gpt-4o/off",
    "openrouter:x/gpt-4o/high",
    "fireworks-ai:gpt-4o/low",
    "radius:balanced/max",
  ];
  if let Some(at) = child() {
    let catalog = Catalog::load(None);
    let efforts = |name: &str| catalog.find(name).expect("the model").efforts().join(" ");
    assert_eq!(efforts("openrouter:x/thinks"), "", "a switch alone says no level");
    assert_eq!(efforts("fireworks-ai:gpt-4o"), "low");
    asks(&at);
    return;
  }
  let test = here!();
  let vars = ["GEMINI_API_KEY", "OPENROUTER_API_KEY", "FIREWORKS_API_KEY", "RADIUS_API_KEY"];
  let asked = requested(test, cached("other-efforts", &listing()), &actors, &vars);
  assert_eq!(
    asked.iter().map(effort).collect::<Vec<_>>(),
    [
      json!({"generationConfig": {"thinkingConfig": {"thinkingBudget": 2048, "includeThoughts": true}}}),
      json!({}),
      json!({"reasoning": {"enabled": false}}),
      json!({"reasoning": {"max_tokens": 16384}}),
      json!({"reasoning_effort": "low"}),
      json!({"reasoning": "max"}),
    ]
  );
  let mut read = Vec::new();
  // Amazon finds its address in the environment of the process.
  for actor in
    ["amazon-bedrock:us.anthropic.claude-sonnet-5/high", "amazon-bedrock:qwen.qwen3/high"]
  {
    let (port, asked_of) = served(vec![REFUSED.to_owned()]);
    let at = cached(&actor.replace([':', '/'], "-"), &listing());
    fs::write(at.join("asks.json"), json!({"ports": [port], "actors": [actor]}).to_string())
      .expect("the actor");
    fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
    let url = format!("AWS_ENDPOINT_URL=http://127.0.0.1:{port}");
    let aws =
      [url.as_str(), "AWS_ACCESS_KEY_ID=k", "AWS_SECRET_ACCESS_KEY=s", "AWS_REGION=us-east-1"];
    run(test, at, &aws);
    let asked = asked_of.join().expect("the provider read");
    let body = body(&asked[0]);
    read.push(json!([body["additionalModelRequestFields"], body["inferenceConfig"]["maxTokens"]]));
  }
  assert_eq!(
    read,
    [
      json!([{"output_config": {"effort": "high"}, "thinking": {"type": "adaptive"}}, 4000]),
      json!([null, null])
    ],
    "a model of Anthropic on Amazon reads an effort as Anthropic does and asks its most tokens, and \
     the other models of Amazon read none"
  );
}

#[test]
fn a_model_asks_the_most_tokens_that_the_catalog_gives_it() {
  let actors = ["anthropic:claude-opus-t5", "openai:gpt-4o"];
  if let Some(at) = child() {
    asks(&at);
    return;
  }
  let vars = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"];
  let asked = requested(here!(), cached("tokens", &listing()), &actors, &vars);
  let most = |body: &Value| json!([body["max_tokens"], body["max_output_tokens"]]);
  assert_eq!(
    asked.iter().map(most).collect::<Vec<_>>(),
    [json!([128_000, null]), json!([null, null])],
    "anthropic asks the most that its model writes, and a model that gives none asks no most"
  );
}

#[test]
fn why_a_model_cannot_be_reached_is_a_failure_of_its_own_for_each_cause() {
  if let Some(at) = child() {
    let mut said = Vec::new();
    for actor in ["google:gemini-flash", "azure:gpt-5-mini", "google-vertex:gemini-pro"] {
      let opening = Opening { actor: Some(actor.to_owned()), ..opened(&at) };
      let mut engine = opening.boot::<String>([]).expect("the life opens").0;
      asked(&mut engine);
      said.push(replied(&mut engine));
    }
    say(&at, &json!(said));
    return;
  }
  let vars = ["AZURE_OPENAI_API_KEY", "GOOGLE_CLOUD_API_KEY"];
  let said = told(here!(), "unreached", &listing(), &vars);
  let refused = |actor: &str, why: &str| {
    json!(format!("Refused(args=('{actor} answered nothing: ProviderError: {why}'))"))
  };
  assert_eq!(
    said,
    json!([
      refused(
        "google:gemini-flash/high",
        "no credential of it stands in the environment: set GOOGLE_API_KEY or \
         GOOGLE_GENERATIVE_AI_API_KEY or GEMINI_API_KEY"
      ),
      refused(
        "azure:gpt-5-mini",
        "its address needs AZURE_RESOURCE_NAME: set it in the environment"
      ),
      refused(
        "google-vertex:gemini-pro",
        "no project of Google stands in the environment: set GOOGLE_CLOUD_PROJECT"
      ),
    ]),
    "a model whose credential stands nowhere, whose address lacks a place, or whose project is \
     missing is refused when it is asked, each with its cause"
  );
}

#[test]
fn a_url_that_a_config_names_gets_the_requests_of_each_client_that_holds_an_address_of_its_own() {
  if let Some(at) = child() {
    asks(&at);
    return;
  }
  let snapshot: Value =
    serde_json::from_str(include_str!("../../../src/world/provider/catalog.json"))
      .expect("the snapshot is JSON");
  let all = snapshot.as_object().expect("the providers of the snapshot");
  let (mut actors, mut vars) = (Vec::new(), Vec::new());
  // The listing of the tests holds the models of google and openrouter.
  for (actor, var) in
    [("google:gemini-flash", "GEMINI_API_KEY"), ("openrouter:x/thinks", "OPENROUTER_API_KEY")]
  {
    actors.push(actor.to_owned());
    vars.push(var.to_owned());
  }
  let clients =
    ["groq", "xai", "mistral", "deepseek", "together", "moonshot", "zai", "huggingface", "xiaomi"];
  for client in clients {
    let (name, provider) =
      all.iter().find(|(_, one)| one["rig"] == client).expect("a provider of the client");
    let models = provider["models"].as_object().expect("its models");
    let text = |one: &Value| {
      one["status"] != "deprecated"
        && one["modalities"]["output"]
          .as_array()
          .is_some_and(|out| out.iter().any(|one| one == "text"))
        && one["provider"].is_null()
    };
    let (id, _) = models.iter().find(|(_, one)| text(one)).expect("a model of the provider");
    actors.push(format!("{name}:{id}"));
    vars.push(provider["env"][0].as_str().expect("a credential").to_owned());
  }
  let actors: Vec<&str> = actors.iter().map(String::as_str).collect();
  let vars: Vec<&str> = vars.iter().map(String::as_str).collect();
  let asked = requested(here!(), cached("addressed", &listing()), &actors, &vars);
  assert_eq!(asked.len(), actors.len(), "each provider was asked at the address of its config");
}

#[test]
fn a_model_of_a_chat_provider_of_the_settings_takes_the_efforts_that_it_lists_as_a_local_model() {
  if let Some(at) = child() {
    if at.join("asks.json").exists() {
      asks(&at);
      return;
    }
    if at.join("configs.json").exists() {
      configured(&at);
      return;
    }
    let actor = fs::read_to_string(at.join("actor")).expect("the actor");
    let opening = Opening { actor: Some(actor), ..opened(&at) };
    let mut engine = opening.boot::<String>([]).expect("the life opens").0;
    let id = asked(&mut engine);
    assert_eq!(answered(&mut engine, &id), Some(3));
    return;
  }
  let test = here!();
  let actors = [
    "strata:qwen/off",
    "strata:qwen/low",
    "strata:qwen/medium",
    "strata:qwen/high",
    "strata:thinks",
    "strata:plain",
  ];
  let asked = requested(test, strata("chat-efforts", "http://127.0.0.1:9/v1"), &actors, &[]);
  let level = |one: &str| json!({"reasoning_effort": one});
  assert_eq!(
    asked.iter().map(effort).collect::<Vec<_>>(),
    [
      json!({"chat_template_kwargs": {"enable_thinking": false}}),
      level("low"),
      level("medium"),
      level("high"),
      level("medium"),
      json!({}),
    ],
    "a model takes the efforts that its config lists, and one that lists none takes none"
  );
  thought_cut(test);
  refused_efforts(test);
}

/// A yard whose project names the chat provider strata at an address, with three models that
/// list their efforts, and a most thought of two tokens.
fn strata(name: &str, url: &str) -> Yard {
  let models = json!({
    "qwen": {"window": 262_144, "reasoning": true, "efforts": ["high", "off", "low", "medium"]},
    "thinks": {"window": 262_144, "reasoning": true, "efforts": ["medium"]},
    "plain": {"window": 262_144, "reasoning": true}
  });
  let at = cached(name, &listing());
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  let provider = json!({"protocol": "chat", "url": url, "key": "k", "models": models});
  let config = json!({"max_thinking_tokens": 2, "providers": {"strata": provider}});
  fs::write(at.join(".furb/config.json"), config.to_string()).expect("the config");
  at
}

/// A model of strata that takes off is asked again with its thought and no more thought once its
/// thought passes the most; one that takes no off is never cut.
fn thought_cut(test: &str) {
  let thought =
    ["a", "b", "c", "d"].map(|one| chunk(&json!({"reasoning_content": one}), &Value::Null));
  let again = [
    chunk(&json!({"role": "assistant", "content": "close(3)"}), &json!("stop")),
    ended(5, Some(0)),
  ];
  let (port, asked_of) = served(vec![thought.concat(), again.concat()]);
  let at = strata("chat-cut", &format!("http://127.0.0.1:{port}/v1"));
  fs::write(at.join("actor"), "strata:qwen/high").expect("the actor");
  run(test, at, &[]);
  let got = asked_of.join().expect("the provider read the requests");
  let (last, second) = (last(&got[1]), body(&got[1]));
  assert_eq!(
    (&last["reasoning_content"], &second["chat_template_kwargs"]),
    (&json!("abc"), &json!({"enable_thinking": false}))
  );
  let (port, asked_of) = served(vec![six(true)]);
  let at = strata("chat-uncut", &format!("http://127.0.0.1:{port}/v1"));
  fs::write(at.join("actor"), "strata:thinks/medium").expect("the actor");
  run(test, at, &[]);
  assert_eq!(asked_of.join().expect("the provider read the request").len(), 1);
}

/// A model of a provider of the settings refuses a level that is no effort, an empty list of
/// efforts, and efforts on a protocol other than chat.
fn refused_efforts(test: &str) {
  let refused = [
    (json!({"qwen": {"window": 8, "efforts": ["max"]}}), "chat"),
    (json!({"qwen": {"window": 8, "efforts": []}}), "chat"),
    (json!({"qwen": {"window": 8, "efforts": ["low"]}}), "openai"),
  ];
  let all: Vec<Value> = (refused.iter())
    .map(|(models, protocol)| {
      let provider =
        json!({"protocol": protocol, "url": "http://127.0.0.1:9/v1", "key": "k", "models": models});
      json!({"providers": {"strata": provider}})
    })
    .collect();
  for (said, (models, protocol)) in configs(test, "chat-refused", &all).iter().zip(&refused) {
    assert!(said.contains("strata"), "{models} of {protocol} was accepted: {said}");
  }
}

/// The yard of the child of a test of the claude command line: the settings of the command line in
/// the config of the user of the life and in the config of its project, and what the child does,
/// `turn`, `search` or `setup`.
#[cfg(unix)]
fn commanded(name: &str, user: &Value, project: &Value, does: &str) -> Yard {
  let at = cached(name, &listing());
  for (folder, command) in [("config", user), (".furb", project)] {
    fs::create_dir_all(at.join(folder)).expect("the folder of a config");
    let config = json!({"providers": {"claude-cli": command}}).to_string();
    fs::write(at.join(folder).join("config.json"), config).expect("a config");
  }
  fs::write(at.join("does"), does).expect("what the child does");
  at
}

/// What the child of a test of the claude command line does: a turn of sonnet, a search of sonnet,
/// whose text it says, or the setup of the catalog of its configs, whose claude it says.
#[cfg(unix)]
fn commands(at: &Path) {
  let does = fs::read_to_string(at.join("does")).expect("what the child does");
  if does == "setup" {
    let catalog = Catalog::load(None).configured(at, Some(&at.join("config")));
    say(at, &json!(catalog.expect("the configs are valid").setup().claude));
    return;
  }
  let opening = Opening { actor: Some("claude-cli:sonnet".to_owned()), ..opened(at) };
  let mut engine = opening.boot::<String>([]).expect("the life opens").0;
  if does == "turn" {
    asked(&mut engine);
    replied(&mut engine);
    return;
  }
  let word = "close(await act('search', '', None, 'what is rust', 'claude-cli:sonnet'))";
  let got = closed(&mut engine, word).expect("the search ends");
  say(at, &json!(Text::of(got.as_ref()).expect("a text").content));
}

/// A program that stands for the claude command line in a search, which writes each word that it
/// was started with, then `---`, then what it read.
#[cfg(unix)]
fn searcher(at: &Path, says: &str) -> PathBuf {
  use std::os::unix::fs::PermissionsExt;
  let bin = at.join("search");
  // A process that asks for the models ends at once, as one of a command line that lists none:
  // else the discovery waits for its answer until its deadline.
  let script =
    format!("#!/bin/sh\n[ -n \"$FURB_CLAUDE_DISCOVER\" ] && exit 0\n{says}\necho ---\ncat\n");
  fs::write(&bin, script).expect("the search");
  fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("the search runs");
  bin
}

#[cfg(unix)]
#[test]
fn a_configured_claude_prefix_reaches_completions_and_search_and_project_settings_win() {
  if let Some(at) = child() {
    commands(&at);
    return;
  }
  let test = here!();
  let fake = Fake::new("configured-prefix");
  let bin = fake.at.join("claude");
  let words = ["run", "3", "two words", "$HOME;literal"];
  let project = |bin: &Path| {
    let command: Vec<String> =
      [bin.display().to_string()].into_iter().chain(words.map(str::to_owned)).collect();
    json!({"command": command})
  };
  let user = json!({"command": [bin, "user-prefix"]});
  run(test, commanded("prefix-turn", &user, &project(&bin), "turn"), &["OPENAI_API_KEY"]);
  let pids = fake.pids();
  assert_eq!(pids.len(), 1);
  let args = fake.args(&pids[0]);
  assert_eq!(
    (&args[..4], args[4].as_str()),
    (&words.map(str::to_owned)[..], "-p"),
    "the words of the project come before each flag"
  );
  let search = searcher(&fake.at, "printf '[%s]\\n' \"$@\"");
  let at = run(test, commanded("prefix-search", &user, &project(&search), "search"), &[]);
  let said: Value =
    serde_json::from_str(&fs::read_to_string(at.join("said")).expect("said")).expect("JSON");
  let said = said.as_str().expect("the text of the search");
  let (args, query) = said.split_once("---\n").expect("the words and the query");
  assert!(args.starts_with("[run]\n[3]\n[two words]\n[$HOME;literal]\n[-p]\n"), "{args}");
  assert_eq!(query, "what is rust");
  let setup = |name: &str, command: Value| {
    let at = run(test, commanded(name, &user, &command, "setup"), &[]);
    fs::read_to_string(at.join("said")).expect("said") == "true"
  };
  assert!(!setup("prefix-off", json!(false)));
  assert!(setup("prefix-on", json!(true)), "true keeps the user command");
  assert!(setup("prefix-one", json!({"command": bin})), "a single executable string is valid");
}

#[cfg(unix)]
#[test]
fn configured_claude_accounts_give_their_token_to_a_turn_and_to_a_search() {
  if let Some(at) = child() {
    commands(&at);
    return;
  }
  let test = here!();
  let fake = Fake::new("configured-accounts");
  let token = fake.at.join("a.token");
  fs::write(&token, "tok-a\n").expect("the token");
  let accounts = |bin: &Path| json!({"command": [bin], "accounts": [{"name": "a", "token_file": token}], "switch": "spread"});
  let none = json!(true);
  run(test, commanded("accounts-turn", &none, &accounts(&fake.at.join("claude")), "turn"), &[]);
  assert_eq!(fake.tokens(), ["tok-a"]);
  let search = searcher(&fake.at, "printf 'token=%s\\n' \"$CLAUDE_CODE_OAUTH_TOKEN\"");
  let at = run(test, commanded("accounts-search", &none, &accounts(&search), "search"), &[]);
  let said = fs::read_to_string(at.join("said")).expect("said");
  assert_eq!(said, json!("token=tok-a\n---\nwhat is rust").to_string());
}
