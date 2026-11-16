//! The times of the local server of the catalog, on servers of the tests on ports of this machine:
//! the delay before it is asked again and the deadline of its list, which a door would wait whole,
//! 30 s and 5 s each time, so these tests shorten them. The suite of the world proves the rest
//! through the catalog of a child.

#![expect(
  clippy::let_underscore_must_use,
  reason = "debt: a let _ drops a value that must be used"
)]
#![expect(clippy::let_underscore_untyped, reason = "debt: a let _ hides the type of what it drops")]

use crate::testing::{on_reactor, pause};
use std::{
  io::{BufRead, BufReader, Read, Write},
  net::TcpListener,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
  },
  thread,
  time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::{Heard, Server};

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

/// How long the servers of the tests wait for a list, and before they ask again.
const DEADLINE: Duration = Duration::from_secs(1);
const AGAIN: Duration = Duration::from_secs(1);

/// The server at an address, with these more names of the environment.
fn on(address: &str, more: &[(&str, &str)]) -> Server {
  let mut pairs = vec![("FURB_LOCAL_URL".to_owned(), address.to_owned())];
  pairs.extend(more.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned())));
  let env =
    move |name: &str| pairs.iter().find(|(held, _)| held == name).map(|(_, value)| value.clone());
  Server::named(env).expect("the environment names the server").timed(DEADLINE, AGAIN)
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

/// A server whose first list is an error, then two models, then four.
fn counting_server() -> (String, mpsc::Receiver<String>) {
  let asked = Arc::new(AtomicUsize::new(0));
  let counted = Arc::clone(&asked);
  fake(move |request| {
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
  })
}

#[test]
fn a_server_that_answers_nothing_in_time_offers_no_model_and_is_asked_again_once_a_delay_passed() {
  let (address, told) = counting_server();
  let server = on(&address, &[]);
  assert!(
    server.heard_now(None).why("local:m").is_some(),
    "a server that answers with an error lists nothing"
  );
  assert!(
    server.heard_now(None).models.is_empty(),
    "a server that listed nothing is not asked again before the delay"
  );
  assert_eq!(lists(&told), 1);
  pause(AGAIN);
  let names = |heard: Heard| heard.models.iter().map(|one| one.name.clone()).collect::<Vec<_>>();
  let first = ["local:m", "local:Qwen/Qwen3-8B"];
  assert_eq!(names(server.heard_now(None)), first);
  assert_eq!(
    names(server.heard_now(Some("local:new"))),
    first,
    "a list is not asked again before the delay"
  );
  assert_eq!(lists(&told), 1, "a list that the server gave holds");
  pause(AGAIN);
  for wanted in [None, Some("local:Qwen/Qwen3-8B/high"), Some("Qwen/Qwen3-8B"), Some("local:m")] {
    assert_eq!(
      names(server.heard_now(wanted)),
      first,
      "a list that names what the catalog wants holds: {wanted:?}"
    );
  }
  assert_eq!(lists(&told), 0);
  let all = ["local:m", "local:Qwen/Qwen3-8B", "local:new"];
  assert_eq!(names(server.heard_now(Some("local:new/high"))), all);
  assert_eq!(
    lists(&told),
    1,
    "a list that names no model of a name is asked again after the delay"
  );
  let why = server.heard_now(Some("local:z/high")).why("local:z/high").map(str::to_owned);
  assert_eq!(why.as_deref(), Some("the local server gives no window of it: set FURB_LOCAL_WINDOW"));
  pause(AGAIN);
  server.heard_now(Some("local:z"));
  assert_eq!(
    lists(&told),
    1,
    "a model that the list holds and does not offer is asked for after the delay"
  );
  server.heard_now(None);
  assert_eq!(lists(&told), 0, "a list is not asked again before the delay");
  pause(AGAIN);
  server.heard_now(None);
  assert_eq!(
    lists(&told),
    1,
    "a list that gives no window of a model for chat is asked again after the delay"
  );
}

#[test]
fn a_server_that_answers_nothing_at_all_is_given_up_on_at_the_deadline() {
  let port = dropping();
  let server = on(&format!("http://user:secret@127.0.0.1:{port}"), &[]);
  let heard = server.heard_now(None);
  let why = heard.why("local:any").unwrap_or_default().to_owned();
  let shown = format!("the local server at http://127.0.0.1:{port} lists no model: ");
  assert!(why.starts_with(&shown) && !why.contains("secret"), "{why}");
  assert!(heard.models.is_empty());
  let silent = TcpListener::bind("127.0.0.1:0").expect("a port of this machine");
  let port = silent.local_addr().expect("the port").port();
  let began = Instant::now();
  let address = format!("http://127.0.0.1:{port}");
  let (tells, told) = mpsc::channel();
  thread::spawn(move || {
    tells.send(on(&address, &[]).heard_now(None).why("local:any").map(str::to_owned))
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
  // The first list of a process pays to start its client of HTTP, which a loaded machine stretches
  // past the deadline: a server that lists at once measures that start, and the list of the
  // server whose windows come late takes no more than that and the deadline.
  let (quick, _) = fake(|request| at(request, "/v1/models").then(|| json!({"data": []})));
  let began = Instant::now();
  on(&quick, &[]).heard_now(None);
  let start = began.elapsed();
  let (address, _) = fake(|request| {
    if at(request, "/v1/models") {
      Some(json!({"data": [{"id": "a", "max_model_len": 1000}, {"id": "b"}]}))
    } else {
      pause(DEADLINE * 4);
      None
    }
  });
  let began = Instant::now();
  let heard = on(&address, &[]).heard_now(None);
  assert!(began.elapsed() < start + DEADLINE * 2, "{:?} past {start:?}", began.elapsed());
  assert_eq!(offered(&heard), [("local:a".to_owned(), 1000, false, Some(250))]);
  assert_eq!(
    heard.why("local:b"),
    Some("the local server gives no window of it: set FURB_LOCAL_WINDOW")
  );
}

/// What a server lists, waited for on the reactor, as the catalog awaits it.
trait Hears {
  fn heard_now(&self, wanted: Option<&str>) -> Heard;
}

impl Hears for Server {
  fn heard_now(&self, wanted: Option<&str>) -> Heard {
    on_reactor(self.heard(wanted))
  }
}
