//! The ear of the web: what a server of MCP answers comes to the answer of the free search, a page comes to its
//! text, and in a life of the real engine a search goes to the native search of its actor and a fetch reads a page
//! of a server of this machine.

use std::{
  future::Future,
  io::{Read, Write},
  net::TcpListener,
  pin::pin,
  sync::Arc,
  task::{Context, Poll, Wake, Waker},
  thread,
  time::Duration,
};

use futures::future::BoxFuture;

use super::{found, markdown, page, web};
use crate::{
  Ear, Engine, Fact, Object, Text,
  ear::{ear, hear, say},
  verbs,
  world::{Search, Searches},
};

#[test]
fn the_free_search_answers_each_result_under_its_title_and_its_address_with_its_excerpts_cut() {
  let long = "x".repeat(2000);
  let listed = serde_json::json!({"results": [
    {"url": "https://one", "title": "One", "excerpts": ["First.", "Second."]},
    {"url": "https://two", "title": "", "excerpts": [long]},
  ]});
  let reply = serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": {"content": [{"type": "text", "text": listed.to_string()}]}});
  let got = found(&reply.to_string()).unwrap();
  let cut = format!("## [https://two](https://two)\n\n{}…", "x".repeat(1500));
  assert_eq!(got, format!("## [One](https://one)\n\nFirst.\n\nSecond.\n\n{cut}"));
  let streamed = format!("event: message\ndata: {reply}\n\n");
  assert_eq!(
    found(&streamed).unwrap(),
    got,
    "a stream of events carries the same answer in its data"
  );
  let none =
    serde_json::json!({"result": {"content": [{"type": "text", "text": "{\"results\": []}"}]}});
  assert_eq!(found(&none.to_string()).unwrap(), "No result.");
}

#[test]
fn the_free_search_refuses_with_the_error_of_its_server_and_gives_a_text_that_is_no_list_as_it_is()
{
  let error = serde_json::json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32000, "message": "Too many requests"}});
  assert_eq!(found(&error.to_string()).unwrap_err(), "Too many requests");
  let failed = serde_json::json!({"result": {"isError": true, "content": [{"type": "text", "text": "No query."}]}});
  assert_eq!(found(&failed.to_string()).unwrap_err(), "No query.");
  let plain = serde_json::json!({"result": {"content": [{"type": "text", "text": "Plain."}]}});
  assert_eq!(found(&plain.to_string()).unwrap(), "Plain.");
  assert!(found("<html>").unwrap_err().starts_with("the free search answered no JSON"));
}

#[test]
fn a_page_of_html_is_its_markdown_other_text_is_itself_and_a_page_of_no_text_is_refused() {
  let html = "<html><head><title>T</title><style>p{}</style></head><body><h1>Rust</h1>\n\n\n<p>Is <b>fast</b>.</p><script>x()</script></body></html>";
  assert_eq!(page("text/html; charset=utf-8", html).unwrap(), "# Rust\n\nIs **fast**.");
  assert_eq!(
    page("", html).unwrap(),
    "# Rust\n\nIs **fast**.",
    "a page of no kind that opens with a tag is HTML"
  );
  assert_eq!(page("application/json", "{\"a\": 1}").unwrap(), "{\"a\": 1}");
  assert_eq!(page("text/plain", "plain  \n").unwrap(), "plain  \n");
  assert_eq!(page("image/png", "\u{89}PNG").unwrap_err(), "is no text: image/png");
}

#[test]
fn the_markdown_of_a_page_has_no_space_at_the_end_of_a_line_and_no_run_of_empty_lines() {
  assert_eq!(markdown("<p>one</p><div><br><br><br></div><p>two</p>"), "one\n\ntwo");
}

/// A waker that unparks the thread of the test.
struct Parked(thread::Thread);

impl Wake for Parked {
  fn wake(self: Arc<Self>) {
    self.0.unpark();
  }
}

/// One future driven to its end on this thread.
fn block_on<T>(fut: impl Future<Output = T>) -> T {
  let waker = Waker::from(Arc::new(Parked(thread::current())));
  let mut cx = Context::from_waker(&waker);
  let mut fut = pin!(fut);
  loop {
    if let Poll::Ready(got) = fut.as_mut().poll(&mut cx) {
      return got;
    }
    thread::park_timeout(Duration::from_millis(20));
  }
}

/// The ear that answers a stand with the operator alone, in the directory of the tests.
fn standing() -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() == "stand" && a.question() {
        let operator = Object::list([Object::string("operator"), Object::list([]), Object::int(1)]);
        let here = Object::string(std::env::temp_dir().display().to_string());
        let standing = Object::list([Object::list([operator]), here, Object::string("operator")]);
        say(&mut co, Fact::says("done", a.about(), [standing]));
      }
    }
  })
}

/// A server of this machine that answers each request for a page by its path: a page of HTML, a page that is not
/// there, an image, and a page of text that tells no length and never ends. Its address.
fn served() -> String {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let at = format!("http://{}", listener.local_addr().unwrap());
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else { continue };
      let mut asked = [0; 4096];
      let size = stream.read(&mut asked).unwrap_or_default();
      let asked = String::from_utf8_lossy(&asked[..size]);
      let path = asked.split_whitespace().nth(1).unwrap_or_default().to_owned();
      if path == "/endless" {
        let head =
          "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ntransfer-encoding: chunked\r\n\r\n";
        let chunk = format!("100000\r\n{}\r\n", "a".repeat(1 << 20));
        let _ = stream.write_all(head.as_bytes());
        while stream.write_all(chunk.as_bytes()).is_ok() {}
        continue;
      }
      let (status, kind, body) = match path.as_str() {
        "/page" => ("200 OK", "text/html", "<h1>Rust</h1><p>Is fast.</p>"),
        "/image" => ("200 OK", "image/png", "PNG"),
        _ => ("404 Not Found", "text/plain", "none"),
      };
      let head = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
      );
      let _ = stream.write_all(format!("{head}{body}").as_bytes());
    }
  });
  at
}

/// The native search of the test, which only the actor m has, and which answers with the query.
fn native() -> Search {
  let searches: Searches = Arc::new(|query| -> BoxFuture<'static, Result<String, String>> {
    Box::pin(async move { Ok(format!("Found {query}.")) })
  });
  Arc::new(move |actor| (actor == "m").then(|| Arc::clone(&searches)))
}

/// What a word that a rung runs on the root of a life on the web ear closes with, or why it raised.
fn closed(engine: &mut Engine, word: &str) -> Result<Object, String> {
  let root = engine.root().to_owned();
  let rung = verbs::Rung { word: Some(word.to_owned()), on: Some(root), ..Default::default() };
  block_on(engine.rung(rung).unwrap()).map_err(|no| format!("{no:?}"))
}

#[test]
fn a_life_searches_with_the_native_search_of_the_actor_and_fetches_a_page_of_a_server() {
  let at = served();
  let ears = [("stand", standing()), ("web", web(native()))];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  let text = |got: Object| Text::of(got.as_ref()).unwrap();
  let got = closed(&mut engine, "close(await act('web_search', '', None, 'rust', 'm'))").unwrap();
  assert_eq!(text(got), Text::new("rust", "Found rust."));
  let got =
    closed(&mut engine, &format!("close(await act('web_fetch', '', None, '{at}/page'))")).unwrap();
  assert_eq!(text(got), Text::new(format!("{at}/page"), "# Rust\n\nIs fast."));
  let no = closed(&mut engine, &format!("close(await act('web_fetch', '', None, '{at}/gone'))"))
    .unwrap_err();
  assert!(no.contains(&format!("{at}/gone answered 404 Not Found")), "{no}");
  let no = closed(&mut engine, &format!("close(await act('web_fetch', '', None, '{at}/image'))"))
    .unwrap_err();
  assert!(no.contains("is no text: image/png"), "{no}");
  let no = closed(&mut engine, &format!("close(await act('web_fetch', '', None, '{at}/endless'))"))
    .unwrap_err();
  assert!(no.contains(&format!("{at}/endless is larger than {} bytes", 10 << 20)), "{no}");
}
