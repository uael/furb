//! The local server of the catalog, asked on servers of the tests on ports of this machine, so no test asks a real
//! one.

use std::{
  io::{BufRead, BufReader, Read, Write},
  net::TcpListener,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
  },
  thread,
  time::Instant,
};

use serde_json::{Value, json};

use super::{AGAIN, DEADLINE, Heard, Server, names};

/// A local server on a port of this machine, which answers each request with the JSON that a function of the request
/// gives, and with a 404 when it gives none; and its address, and each request it reads, as it reads it.
pub(in crate::world::provider) fn fake(
  answer: impl Fn(&str) -> Option<Value> + Send + 'static,
) -> (String, mpsc::Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let address = format!("http://{}", listener.local_addr().expect("the port"));
  (address, faked(listener, answer))
}

/// Each request that a local server on this listener reads, which it answers as [`fake`] does.
fn faked(
  listener: TcpListener,
  answer: impl Fn(&str) -> Option<Value> + Send + 'static,
) -> mpsc::Receiver<String> {
  let (tells, told) = mpsc::channel();
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(stream) = stream else { return };
      let mut reader = BufReader::new(stream);
      let mut head = String::new();
      while !head.ends_with("\r\n\r\n") {
        if reader.read_line(&mut head).unwrap_or_default() == 0 {
          break;
        }
      }
      let length = head.lines().find_map(|line| {
        line.to_ascii_lowercase().strip_prefix("content-length: ")?.trim().parse::<usize>().ok()
      });
      let mut body = vec![0; length.unwrap_or_default()];
      let _ = reader.read_exact(&mut body);
      let request = format!("{head}{}", String::from_utf8_lossy(&body));
      let out = match answer(&request) {
        Some(one) => {
          let one = one.to_string();
          let length = one.len();
          format!(
            "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-type: application/json\r\n\
             content-length: {length}\r\n\r\n{one}"
          )
        }
        None => {
          "HTTP/1.1 404 Not Found\r\nconnection: close\r\ncontent-length: 0\r\n\r\n".to_owned()
        }
      };
      let _ = tells.send(request);
      let _ = reader.into_inner().write_all(out.as_bytes());
    }
  });
  told
}

/// A port of this machine whose server closes each connection at once, which no other test can take.
pub(in crate::world::provider) fn dropping() -> u16 {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = listener.local_addr().expect("the port").port();
  thread::spawn(move || {
    for stream in listener.incoming() {
      drop(stream);
    }
  });
  port
}

/// The server at an address, with these more names of the environment.
fn on(address: &str, more: &[(&str, &str)]) -> Server {
  let mut pairs = vec![("FURB_LOCAL_URL".to_owned(), address.to_owned())];
  pairs.extend(more.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned())));
  let env =
    move |name: &str| pairs.iter().find(|(held, _)| held == name).map(|(_, value)| value.clone());
  Server::named(env).expect("the environment names the server")
}

/// The first line of each request that a server read since the last call.
fn heads(told: &mpsc::Receiver<String>) -> Vec<String> {
  told.try_iter().map(|one| one.lines().next().unwrap_or_default().to_owned()).collect()
}

/// How many times a server was asked for its list since the last call.
fn lists(told: &mpsc::Receiver<String>) -> usize {
  heads(told).iter().filter(|one| one.starts_with("GET /v1/models ")).count()
}

/// Each model that a server listed: its name, its window, whether it takes images, and the most it writes in a turn.
fn offered(heard: &Heard) -> Vec<(String, u64, bool, Option<u64>)> {
  heard.models.iter().map(|one| (one.name.clone(), one.window, one.images, one.tokens)).collect()
}

/// The answer of a server of the tests to the request of a path, by the first line of the request.
fn at(request: &str, path: &str) -> bool {
  request.lines().next().unwrap_or_default().split(' ').nth(1) == Some(path)
}

#[test]
fn the_environment_names_the_address_of_the_server_its_key_and_the_window_and_the_tokens_of_its_models()
 {
  let named = |pairs: &[(&str, &str)]| {
    let pairs: Vec<(String, String)> =
      pairs.iter().map(|(name, value)| ((*name).into(), (*value).into())).collect();
    Server::named(move |name| {
      pairs.iter().find(|(held, _)| held == name).map(|(_, value)| value.clone())
    })
  };
  assert!(named(&[]).is_none());
  assert!(named(&[("FURB_LOCAL_URL", " ")]).is_none(), "an empty address names no server");
  let server =
    named(&[("FURB_LOCAL_URL", "https://user:secret@host:8080/v1")]).expect("the server");
  assert_eq!(server.root, "https://host:8080", "the address loses its user and its password");
  let roots = [
    ("user:secret@host:8080", "http://host:8080"),
    ("http://host:1/path@at/v1", "http://host:1/path@at"),
  ];
  let named_as =
    [("local:qwen/high", true), ("qwen/high", true), ("local:qwen/x", false), ("local:qwen", true)];
  assert_eq!(
    named_as.map(|(name, _)| names(name, "qwen")),
    named_as.map(|(_, holds)| holds),
    "a slash names a level"
  );
  assert!(names("local:Qwen/Qwen3-8B", "Qwen/Qwen3-8B") && !names("local:Qwen/Qwen3-8B", "Qwen"));
  for (address, root) in roots {
    assert_eq!(named(&[("FURB_LOCAL_URL", address)]).expect("the server").root, root);
  }
  let server = named(&[
    ("FURB_LOCAL_URL", "http://host:8080/v1/"),
    ("FURB_LOCAL_KEY", "k"),
    ("FURB_LOCAL_WINDOW", "4096"),
    ("FURB_LOCAL_TOKENS", "0"),
  ])
  .expect("the server");
  assert_eq!(
    (server.root.as_str(), server.key.as_str(), server.window, server.tokens),
    ("http://host:8080", "k", Some(4096), None),
    "the address loses its /v1, and a count of zero says nothing"
  );
}

#[test]
fn each_model_for_chat_that_the_server_lists_is_offered_with_the_window_and_the_images_that_it_gives()
 {
  let list = json!({"object": "list", "data": [
    {"id": "qwen", "meta": {"n_ctx": 262144}, "architecture": {"input_modalities": ["text", "image"],
      "output_modalities": ["text"]}},
    {"id": "llama", "max_model_len": 32768},
    {"id": "odd", "meta": null, "architecture": null, "max_model_len": 8192},
    {"id": 5},
    {"id": "embed", "meta": {"n_ctx": 512}, "architecture": {"input_modalities": ["text"],
      "output_modalities": ["embeddings"]}},
  ]});
  let (address, told) = fake(move |request| at(request, "/v1/models").then(|| list.clone()));
  let heard = on(&address, &[("FURB_LOCAL_KEY", "k"), ("FURB_LOCAL_TOKENS", "1000")]).heard(None);
  assert_eq!(
    offered(&heard),
    [
      ("local:qwen".to_owned(), 262_144, true, Some(1000)),
      ("local:llama".to_owned(), 32_768, false, Some(1000)),
      ("local:odd".to_owned(), 8192, false, Some(1000)),
    ],
    "an entry of another shape hides no other, and a model that writes no text is not offered"
  );
  assert_eq!(
    heard.why("local:embed"),
    Some("the local server says that it is not a model for chat")
  );
  let each =
    heard.models.iter().map(|one| (one.efforts().len(), one.priced(), one.at(Some("high"))));
  assert_eq!(
    each.collect::<Vec<_>>(),
    ["local:qwen", "local:llama", "local:odd"].map(|name| (
      4,
      Some([0.0; 4]),
      format!("{name}/high")
    )),
    "a model of a local server takes the efforts off, low, medium and high, and costs nothing"
  );
  let requests: Vec<String> = told.try_iter().collect();
  assert_eq!(
    requests.len(),
    1,
    "a server that gives each window is asked nothing more: {requests:?}"
  );
  assert!(requests[0].starts_with("GET /v1/models "), "{requests:?}");
  assert!(requests[0].to_ascii_lowercase().contains("authorization: bearer k"), "{requests:?}");
}

#[test]
fn a_server_that_lists_no_window_gives_it_where_its_kind_of_server_gives_it() {
  let (address, _) = fake(|request| {
    if at(request, "/v1/models") {
      Some(json!({"data": [{"id": "one"}]}))
    } else {
      at(request, "/props").then(|| json!({"default_generation_settings": {"n_ctx": 16384}}))
    }
  });
  let quarter = ("local:one".to_owned(), 16_384, false, Some(4096));
  assert_eq!(offered(&on(&address, &[]).heard(None)), [quarter], "llama.cpp gives it at /props");
  let (address, _) = fake(|request| {
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
  let heard = on(&address, &[]).heard(None);
  let studio = [
    ("local:a".to_owned(), 8192, false, Some(2048)),
    ("local:c".to_owned(), 4096, true, Some(1024)),
  ];
  assert_eq!(offered(&heard), studio, "LM Studio gives it at /api/v0/models");
  assert_eq!(heard.why("local:b"), Some("the local server says that it is not a model for chat"));
  let reason = heard.why("local:d");
  assert_eq!(
    reason,
    Some("the local server gives no window of it: set FURB_LOCAL_WINDOW"),
    "LM Studio loads it in its own"
  );
  let (address, _) = fake(|request| {
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
        json!({"capabilities": ["completion", "vision"], "model_info": {"llama.context_length": 131072}}),
      )
    } else if shows("w") {
      Some(json!({"capabilities": ["completion"], "model_info": {"llama.context_length": 131072}}))
    } else {
      shows("e")
        .then(|| json!({"capabilities": ["embedding"], "model_info": {"bert.context_length": 512}}))
    }
  });
  let heard = on(&address, &[]).heard(None);
  let ollama = [
    ("local:x".to_owned(), 32_768, false, Some(8192)),
    ("local:y".to_owned(), 8192, true, Some(2048)),
  ];
  assert_eq!(
    offered(&heard),
    ollama,
    "Ollama serves a model in its num_ctx, or in the window of /api/ps"
  );
  assert_eq!(
    heard.why("local:w"),
    Some("the local server gives no window of it: set FURB_LOCAL_WINDOW"),
    "the context that a model of Ollama was trained on is no window that Ollama serves"
  );
  assert_eq!(heard.why("local:e"), Some("the local server says that it is not a model for chat"));
  let (address, _) =
    fake(|request| at(request, "/v1/models").then(|| json!({"data": [{"id": "z"}]})));
  let heard = on(&address, &[]).heard(None);
  assert_eq!(
    (heard.models.len(), heard.why("local:z")),
    (0, Some("the local server gives no window of it: set FURB_LOCAL_WINDOW"))
  );
  let heard = on(&address, &[("FURB_LOCAL_WINDOW", "2048")]).heard(None);
  assert_eq!(
    offered(&heard),
    [("local:z".to_owned(), 2048, false, Some(512))],
    "the environment gives it"
  );
}

#[test]
fn a_server_that_answers_nothing_in_time_offers_no_model_and_is_asked_again_once_a_delay_passed() {
  let port = dropping();
  let server = on(&format!("http://user:secret@127.0.0.1:{port}"), &[]);
  let heard = server.heard(None);
  let why = heard.why("local:any").unwrap_or_default().to_owned();
  let shown = format!("the local server at http://127.0.0.1:{port} lists no model: ");
  assert!(why.starts_with(&shown) && !why.contains("secret"), "{why}");
  assert!(heard.models.is_empty());
  let asked = Arc::new(AtomicUsize::new(0));
  let counted = Arc::clone(&asked);
  let (address, told) = fake(move |request| {
    if !at(request, "/v1/models") {
      return None;
    }
    let ids: &[&str] = match counted.fetch_add(1, Ordering::SeqCst) {
      0 => return None,
      1 => &["m", "Qwen/Qwen3-8B"],
      _ => &["m", "Qwen/Qwen3-8B", "new", "z"],
    };
    let entry = |id: &&str| match *id {
      "z" => json!({"id": id}),
      _ => json!({"id": id, "max_model_len": 1000}),
    };
    Some(json!({"data": ids.iter().map(entry).collect::<Vec<_>>()}))
  });
  let server = on(&address, &[]);
  assert!(
    server.heard(None).why("local:m").is_some(),
    "a server that answers with an error lists nothing"
  );
  assert!(
    server.heard(None).models.is_empty(),
    "a server that listed nothing is not asked again before the delay"
  );
  assert_eq!(lists(&told), 1);
  thread::sleep(AGAIN);
  let names = |heard: Heard| heard.models.iter().map(|one| one.name.clone()).collect::<Vec<_>>();
  let first = ["local:m", "local:Qwen/Qwen3-8B"];
  assert_eq!(names(server.heard(None)), first);
  assert_eq!(
    names(server.heard(Some("local:new"))),
    first,
    "a list is not asked again before the delay"
  );
  assert_eq!(lists(&told), 1, "a list that the server gave holds");
  thread::sleep(AGAIN);
  for wanted in [None, Some("local:Qwen/Qwen3-8B/high"), Some("Qwen/Qwen3-8B"), Some("local:m")] {
    assert_eq!(
      names(server.heard(wanted)),
      first,
      "a list that names what the catalog wants holds: {wanted:?}"
    );
  }
  assert_eq!(lists(&told), 0);
  let all = ["local:m", "local:Qwen/Qwen3-8B", "local:new"];
  assert_eq!(names(server.heard(Some("local:new/high"))), all);
  assert_eq!(
    lists(&told),
    1,
    "a list that names no model of a name is asked again after the delay"
  );
  let why = server.heard(Some("local:z/high")).why("local:z/high").map(str::to_owned);
  assert_eq!(why.as_deref(), Some("the local server gives no window of it: set FURB_LOCAL_WINDOW"));
  thread::sleep(AGAIN);
  server.heard(Some("local:z"));
  assert_eq!(
    lists(&told),
    1,
    "a model that the list holds and does not offer is asked for after the delay"
  );
  server.heard(None);
  assert_eq!(lists(&told), 0, "a list is not asked again before the delay");
  thread::sleep(AGAIN);
  server.heard(None);
  assert_eq!(
    lists(&told),
    1,
    "a list that gives no window of a model for chat is asked again after the delay"
  );
  let silent = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = silent.local_addr().expect("the port").port();
  let began = Instant::now();
  let address = format!("http://127.0.0.1:{port}");
  let (tells, told) = mpsc::channel();
  thread::spawn(move || {
    tells.send(on(&address, &[]).heard(None).why("local:any").map(str::to_owned))
  });
  let why =
    told.recv_timeout(DEADLINE * 3).expect("a server that answers nothing is given up on in time");
  let waited = began.elapsed();
  let why = why.unwrap_or_default();
  assert!(why.ends_with("it listed nothing in 1s"), "{why}");
  assert!(waited >= DEADLINE && waited < DEADLINE * 2, "{waited:?}");
  drop(silent);
}

#[test]
fn a_server_whose_windows_elsewhere_do_not_come_in_time_offers_the_models_whose_window_its_list_gives()
 {
  let (address, _) = fake(|request| {
    if at(request, "/v1/models") {
      Some(json!({"data": [{"id": "a", "max_model_len": 1000}, {"id": "b"}]}))
    } else {
      thread::sleep(DEADLINE * 2);
      None
    }
  });
  let began = Instant::now();
  let heard = on(&address, &[]).heard(None);
  assert!(began.elapsed() < DEADLINE * 2, "{:?}", began.elapsed());
  assert_eq!(offered(&heard), [("local:a".to_owned(), 1000, false, Some(250))]);
  assert_eq!(
    heard.why("local:b"),
    Some("the local server gives no window of it: set FURB_LOCAL_WINDOW")
  );
}
