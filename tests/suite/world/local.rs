//! The local server that the environment of a process names, as its catalog offers its models:
//! each test runs again as a child of itself, as a user starts a host with that environment, and a
//! server of the test on a port of this machine answers it and keeps each request that it read.

use std::{
  fs,
  io::{BufRead, BufReader, Read, Write},
  net::TcpListener,
  sync::mpsc,
  thread,
};

use furb::world::Catalog;
use serde_json::{Value, json};

use super::offered::{cached, run};
use crate::child::{child, here};

/// A local server on a port of this machine, which answers each request with the JSON that a
/// function of the request gives, and with a 404 when it gives none; and its port, and each
/// request it reads, as it reads it.
fn fake(answer: impl Fn(&str) -> Option<Value> + Send + 'static) -> (u16, mpsc::Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  let (tells, told) = mpsc::channel();
  thread::spawn(move || {
    for stream in listener.incoming().flatten() {
      let mut reader = BufReader::new(stream);
      let mut head = String::new();
      while !head.ends_with("\r\n\r\n") && reader.read_line(&mut head).is_ok_and(|read| read > 0) {}
      let length = head.lines().find_map(|line| {
        line.to_ascii_lowercase().strip_prefix("content-length: ")?.trim().parse::<usize>().ok()
      });
      let mut body = vec![0; length.unwrap_or_default()];
      reader.read_exact(&mut body).expect("the body of the request");
      let request = format!("{head}{}", String::from_utf8_lossy(&body));
      let (status, out) = answer(&request)
        .map_or(("404 Not Found", String::new()), |one| ("200 OK", one.to_string()));
      let out = format!(
        "HTTP/1.1 {status}\r\nconnection: close\r\ncontent-type: application/json\r\n\
         content-length: {}\r\n\r\n{out}",
        out.len()
      );
      // The test reads the requests after its child ended, so a channel that no test reads is no
      // fault of the server.
      drop(tells.send(request));
      reader.into_inner().write_all(out.as_bytes()).expect("the server answers");
    }
  });
  (port, told)
}

/// Whether a request asks this path, by its first line.
fn at(request: &str, path: &str) -> bool {
  request.lines().next().unwrap_or_default().split(' ').nth(1) == Some(path)
}

/// Each model of the local server that the catalog of the child offers: its name, its window and
/// whether it sees images.
fn offered() -> Vec<(String, u64, bool)> {
  let models = Catalog::load(None).unwrap().offered().unwrap().into_iter();
  let local = models.filter(|one| one.name().starts_with("local:"));
  local.map(|one| (one.name().to_owned(), one.window(), one.sees())).collect()
}

/// Why the catalog of the child offers no model of this name.
fn why(name: &str) -> String {
  Catalog::load(None)
    .unwrap()
    .roster(None, Some(name))
    .err()
    .map(|no| no.to_string())
    .unwrap_or_default()
}

/// The environment that names the local server on this port.
fn named(port: u16) -> String {
  format!("FURB_LOCAL_URL=http://127.0.0.1:{port}")
}

#[test]
fn the_environment_names_the_address_of_the_server_with_no_user_no_password_and_no_v1_and_its_key_and_window()
 {
  if let Some(at) = child() {
    if at.join("blank").exists() {
      assert_eq!(
        why("local:qwen"),
        "No model is local:qwen: no local server is named: set FURB_LOCAL_URL."
      );
      return;
    }
    assert_eq!(
      offered(),
      [("local:Qwen/Qwen3-8B".to_owned(), 4096, false)],
      "the environment gives the window"
    );
    let actor =
      Catalog::load(None).unwrap().roster(None, Some("local:Qwen/Qwen3-8B/high")).map(|one| one.1);
    assert_eq!(
      actor.ok(),
      Some(Some("local:Qwen/Qwen3-8B/high".to_owned())),
      "a slash after the id names a level"
    );
    return;
  }
  let listed = json!({"data": [{"id": "Qwen/Qwen3-8B", "max_model_len": 1000}]});
  let (port, told) = fake(move |request| at(request, "/path@at/v1/models").then(|| listed.clone()));
  // An address with no scheme, a user and a password, an @ in its path and a /v1 at its end.
  let address = format!("FURB_LOCAL_URL=user:secret@127.0.0.1:{port}/path@at/v1/");
  run(
    here!(),
    cached("local-named", "{}"),
    &[&address, "FURB_LOCAL_KEY=k", "FURB_LOCAL_WINDOW=4096"],
  );
  let requests: Vec<String> = told.try_iter().collect();
  assert!(
    !requests.is_empty() && requests.iter().all(|one| one.starts_with("GET /path@at/v1/models ")),
    "{requests:?}"
  );
  let lower = requests.iter().map(|one| one.to_ascii_lowercase());
  assert!(lower.clone().all(|one| one.contains("authorization: bearer k\r\n")), "{requests:?}");
  assert!(
    lower.clone().all(|one| !one.contains("secret") && !one.contains("authorization: basic")),
    "{requests:?}"
  );
  let blank = cached("local-blank", "{}");
  fs::write(blank.join("blank"), "").expect("the mark");
  run(here!(), blank, &["FURB_LOCAL_URL= "]);
}

#[test]
fn each_model_for_chat_that_the_server_lists_is_offered_with_the_window_and_the_images_that_it_gives()
 {
  if let Some(_at) = child() {
    assert_eq!(
      offered(),
      [
        ("local:qwen".to_owned(), 262_144, true),
        ("local:llama".to_owned(), 32_768, false),
        ("local:odd".to_owned(), 8192, false),
      ],
      "an entry of another shape hides no other, and a model that writes no text is not offered"
    );
    let catalog = Catalog::load(None).unwrap();
    for name in ["local:qwen", "local:llama", "local:odd"] {
      let model = catalog.find(name).unwrap().expect("a model of the server");
      assert_eq!(
        (model.efforts(), model.priced(), model.at(Some("high"))),
        (vec!["off", "low", "medium", "high"], Some([0.0; 4]), format!("{name}/high")),
        "a model of a local server takes the efforts off, low, medium and high, and costs nothing"
      );
    }
    assert_eq!(
      why("local:embed"),
      "No model is local:embed: the local server says that it is not a model for chat."
    );
    return;
  }
  let list = json!({"object": "list", "data": [
    {"id": "qwen", "meta": {"n_ctx": 262_144}, "architecture": {"input_modalities": ["text", "image"],
      "output_modalities": ["text"]}},
    {"id": "llama", "max_model_len": 32768},
    {"id": "odd", "meta": null, "architecture": null, "max_model_len": 8192},
    {"id": 5},
    {"id": "embed", "meta": {"n_ctx": 512}, "architecture": {"input_modalities": ["text"],
      "output_modalities": ["embeddings"]}},
  ]});
  let (port, told) = fake(move |request| at(request, "/v1/models").then(|| list.clone()));
  run(here!(), cached("local-shapes", "{}"), &[&named(port)]);
  let heads: Vec<String> =
    told.try_iter().map(|one| one.lines().next().unwrap_or_default().to_owned()).collect();
  assert!(
    heads.iter().all(|one| one.starts_with("GET /v1/models ")),
    "a server that gives each window is asked nothing more: {heads:?}"
  );
}

#[test]
fn a_server_of_llama_cpp_gives_the_window_of_a_model_that_its_list_does_not_at_props() {
  if let Some(_at) = child() {
    assert_eq!(offered(), [("local:one".to_owned(), 16_384, false)]);
    return;
  }
  let (port, _) = fake(|request| {
    if at(request, "/v1/models") {
      Some(json!({"data": [{"id": "one"}]}))
    } else {
      at(request, "/props").then(|| json!({"default_generation_settings": {"n_ctx": 16384}}))
    }
  });
  run(here!(), cached("local-llama", "{}"), &[&named(port)]);
}

#[test]
fn a_server_of_lm_studio_gives_the_window_that_it_loaded_and_one_that_it_did_not_load_waits_for_the_environment()
 {
  if let Some(_at) = child() {
    let studio = [("local:a".to_owned(), 8192, false), ("local:c".to_owned(), 4096, true)];
    assert_eq!(offered(), studio);
    assert_eq!(
      why("local:b"),
      "No model is local:b: the local server says that it is not a model for chat."
    );
    assert_eq!(
      why("local:d"),
      "No model is local:d: the local server gives no window of it: set FURB_LOCAL_WINDOW."
    );
    return;
  }
  let (port, _) = fake(|request| {
    if at(request, "/v1/models") {
      Some(json!({"data": [{"id": "a"}, {"id": "b"}, {"id": "c"}, {"id": "d"}]}))
    } else {
      at(request, "/api/v0/models").then(|| {
        json!({"data": [{"id": "a", "type": "llm", "loaded_context_length": 8192, "max_context_length": 32768},
          {"id": "b", "type": "embeddings", "max_context_length": 2048},
          {"id": "c", "type": "vlm", "loaded_context_length": 4096, "max_context_length": 8192},
          {"id": "d", "type": "llm", "max_context_length": 65536}]})
      })
    }
  });
  run(here!(), cached("local-studio", "{}"), &[&named(port)]);
}

#[test]
fn a_server_of_ollama_gives_the_window_that_it_serves_and_not_the_one_that_a_model_was_trained_on()
{
  if let Some(_at) = child() {
    let ollama = [("local:x".to_owned(), 32_768, false), ("local:y".to_owned(), 8192, true)];
    assert_eq!(
      offered(),
      ollama,
      "Ollama serves a model in its num_ctx, or in the window of /api/ps"
    );
    assert_eq!(
      why("local:w"),
      "No model is local:w: the local server gives no window of it: set FURB_LOCAL_WINDOW."
    );
    assert_eq!(
      why("local:e"),
      "No model is local:e: the local server says that it is not a model for chat."
    );
    return;
  }
  let (port, _) = fake(|request| {
    let shows =
      |id: &str| at(request, "/api/show") && request.contains(&format!("{{\"model\":\"{id}\"}}"));
    if at(request, "/v1/models") {
      Some(json!({"data": [{"id": "x"}, {"id": "y"}, {"id": "w"}, {"id": "e"}]}))
    } else if at(request, "/api/ps") {
      Some(json!({"models": [{"name": "y", "model": "y", "context_length": 8192}]}))
    } else if shows("x") {
      let parameters = "num_ctx 32768\ntemperature 1";
      Some(json!({"capabilities": ["completion"], "parameters": parameters,
        "model_info": {"qwen3.context_length": 40960}}))
    } else if shows("y") {
      Some(
        json!({"capabilities": ["completion", "vision"], "model_info": {"llama.context_length": 131_072}}),
      )
    } else if shows("w") {
      Some(json!({"capabilities": ["completion"], "model_info": {"llama.context_length": 131_072}}))
    } else {
      shows("e")
        .then(|| json!({"capabilities": ["embedding"], "model_info": {"bert.context_length": 512}}))
    }
  });
  run(here!(), cached("local-ollama", "{}"), &[&named(port)]);
}
