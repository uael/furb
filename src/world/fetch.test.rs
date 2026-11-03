//! The ear of the fetch: a page comes to its text, and in a life of the real engine a fetch reads a
//! page of a server of this machine.

#![expect(
  clippy::let_underscore_must_use,
  reason = "debt: a let _ drops a value that must be used"
)]
#![expect(clippy::let_underscore_untyped, reason = "debt: a let _ hides the type of what it drops")]

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

use super::{fetch, markdown, page};
use crate::{
  Ear, Engine, Fact, Object, Text,
  ear::{ear, hear, say},
  verbs,
};

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

/// A server of this machine that answers each request for a page by its path: a page of HTML, a
/// page that is not there, an image, and a page of text that tells no length and never ends. Its
/// address.
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

/// What a word that a rung runs on the root of a life on the fetch ear closes with, or why it
/// raised.
fn closed(engine: &mut Engine, word: &str) -> Result<Object, String> {
  let root = engine.root().to_owned();
  let rung = verbs::Rung { word: Some(word.to_owned()), on: Some(root), ..Default::default() };
  block_on(engine.rung(rung).unwrap()).map_err(|no| format!("{no:?}"))
}

#[test]
fn a_life_fetches_a_page_of_a_server_as_a_text_at_its_address_or_is_refused_with_why() {
  let at = served();
  let ears = [("stand", standing()), ("fetch", fetch())];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  let got = closed(&mut engine, &format!("close(await act('fetch', '', None, '{at}/page'))"));
  let text = Text::of(got.unwrap().as_ref()).unwrap();
  assert_eq!(text, Text::new(format!("{at}/page"), "# Rust\n\nIs fast."));
  let no =
    closed(&mut engine, &format!("close(await act('fetch', '', None, '{at}/gone'))")).unwrap_err();
  assert!(no.contains(&format!("{at}/gone answered 404 Not Found")), "{no}");
  let no =
    closed(&mut engine, &format!("close(await act('fetch', '', None, '{at}/image'))")).unwrap_err();
  assert!(no.contains("is no text: image/png"), "{no}");
  let no = closed(&mut engine, &format!("close(await act('fetch', '', None, '{at}/endless'))"))
    .unwrap_err();
  assert!(no.contains(&format!("{at}/endless is larger than {} bytes", 10 << 20)), "{no}");
}
