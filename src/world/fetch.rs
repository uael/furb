//! The fetch of the World: a page over HTTP, which an ear answers as a text.

use std::{collections::HashMap, time::Duration};

use futures::{FutureExt, StreamExt, future::BoxFuture, stream};
use rig_core::http_client::ReqwestClient;
use unsync::oneshot;

use super::{provider::search::Cut, spawned};
use crate::{
  ear::{Co, Ear, Next, hearing, say},
  fact::Fact,
  value::{Fault, Text},
};

/// How a fetch names itself to a server.
const AGENT: &str = "Mozilla/5.0 (compatible; furb; +https://github.com/uael/furb)";

/// The kinds of a page that a fetch prefers.
const ACCEPT: &str = "text/html, text/markdown;q=0.9, text/plain;q=0.8, */*;q=0.5";

/// The largest page that a fetch reads, in bytes.
const LARGEST: u64 = 10 << 20;

/// The tags of a page that hold no text to read, which its Markdown leaves out.
const SKIPPED: [&str; 8] =
  ["head", "script", "style", "noscript", "template", "svg", "iframe", "canvas"];

/// What a work of the ear came to: the act, and its text or why it has none.
type Worked = (String, Result<Text, Error>);

/// The ear of the fetch: it takes a fetch of an address, says it started, and says it done later
/// with the page at that address, as a text at the path of that address: as Markdown when the page
/// is HTML, and as it is when it is other text. It says done with the refusal when the page is
/// none, is no text, or is larger than the bound. A control that ends an act first ends its work,
/// and the ear says nothing more of it.
pub fn fetch() -> Box<dyn Ear> {
  hearing(&["fetch"], move |mut co: Co<Worked>| async move {
    let mut stops: HashMap<String, oneshot::Sender<()>> = HashMap::new();
    loop {
      let a = match co.next().await {
        Next::Heard(a) => a,
        Next::Worked((id, got)) => {
          if stops.remove(&id).is_some() {
            let got =
              got.map_or_else(|no| Fault::refused(no.to_string()).object(), |one| one.object());
            say(&mut co, Fact::says("done", &id, [got]));
          }
          continue;
        }
      };
      let about = a.about().to_owned();
      let work = match a.kind() {
        "fetch" if a.question() => {
          fetched(a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default())
        }
        "done" => {
          stops.remove(&about);
          continue;
        }
        _ => continue,
      };
      say(&mut co, Fact::says("started", &about, []));
      let (stop, stopped) = oneshot::channel();
      let id = about.clone();
      co.work(stream::once(work.map(move |got| (id, got))).take_until(stopped));
      stops.insert(about, stop);
    }
  })
}

/// Why a page is none.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
  #[error("{0}")]
  Client(reqwest::Error),
  #[error("{url}: {source}")]
  Request { url: String, source: reqwest::Error },
  #[error("{url} answered {status}")]
  Status { url: String, status: reqwest::StatusCode },
  #[error("{url} is larger than {LARGEST} bytes")]
  TooLarge { url: String },
  #[error("{url} is no text: {kind}")]
  NotText { url: String, kind: String },
  #[error("{0}")]
  Cut(#[from] Cut),
}

/// The page at an address, as a text at the path of that address: as Markdown when it is HTML, and
/// as it is when it is other text; or why it is none.
pub(crate) fn fetched(url: String) -> BoxFuture<'static, Result<Text, Error>> {
  spawned(async move {
    let client = ReqwestClient::builder()
      .user_agent(AGENT)
      .timeout(Duration::from_secs(60))
      .build()
      .map_err(Error::Client)?;
    let request = |source| Error::Request { url: url.clone(), source };
    let mut response = client.get(&url).header("accept", ACCEPT).send().await.map_err(request)?;
    let status = response.status();
    if !status.is_success() {
      return Err(Error::Status { url, status });
    }
    let kind = response.headers().get("content-type").and_then(|one| one.to_str().ok());
    let kind = kind.unwrap_or_default().to_ascii_lowercase();
    if response.content_length().is_some_and(|size| size > LARGEST) {
      return Err(Error::TooLarge { url });
    }
    // A page that tells no length, or tells one that is wrong, is read on only while it stays under
    // the bound.
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(request)? {
      if (bytes.len() + chunk.len()) as u64 > LARGEST {
        return Err(Error::TooLarge { url });
      }
      bytes.extend_from_slice(&chunk);
    }
    let content = page(&url, &kind, &String::from_utf8_lossy(&bytes))?;
    Ok(Text::new(url, content))
  })
}

/// The text of a page of a kind: its Markdown when it is HTML, the page itself when it is other
/// text, and a page of no kind is HTML when it opens with a tag; or why it is no text.
fn page(url: &str, kind: &str, body: &str) -> Result<String, Error> {
  let html = kind.contains("html") || (kind.is_empty() && body.trim_start().starts_with('<'));
  let texts = ["text/", "json", "xml", "javascript", "yaml", "toml"];
  if html {
    Ok(markdown(body))
  } else if kind.is_empty() || texts.iter().any(|one| kind.contains(one)) {
    Ok(body.to_owned())
  } else {
    Err(Error::NotText { url: url.to_owned(), kind: kind.to_owned() })
  }
}

/// The Markdown of a page of HTML, less the tags that hold no text to read, with no space at the
/// end of a line and no run of more than one empty line.
fn markdown(html: &str) -> String {
  let converter = htmd::HtmlToMarkdown::builder().skip_tags(SKIPPED.to_vec()).build();
  let text = converter.convert(html).unwrap_or_else(|_| html.to_owned());
  let mut out: Vec<&str> = Vec::new();
  for line in text.lines().map(str::trim_end) {
    if !(line.is_empty() && out.last().is_none_or(|one| one.is_empty())) {
      out.push(line);
    }
  }
  while out.last().is_some_and(|one| one.is_empty()) {
    out.pop();
  }
  out.join("\n")
}

#[cfg(test)]
#[path = "fetch.test.rs"]
mod test;
