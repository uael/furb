//! The ear of the web extension, which answers a search of the web and a fetch of a page of the web.

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::{FutureExt, StreamExt, future::BoxFuture, stream};
use rig_core::http_client::ReqwestClient;
use serde_json::{Value, json};
use unsync::oneshot;

use crate::{
  ear::{Co, Ear, Next, hearing, say},
  fact::Fact,
  value::{Fault, Text},
  world::{Search, Searches, posted, spawned},
};

/// The free search, which needs no key: the server of Parallel for agents, which answers the protocol of MCP.
const FREE: &str = "https://search.parallel.ai/mcp";

/// How a fetch names itself to a server.
const AGENT: &str = "Mozilla/5.0 (compatible; furb; +https://github.com/uael/furb)";

/// The kinds of a page that a fetch prefers.
const ACCEPT: &str = "text/html, text/markdown;q=0.9, text/plain;q=0.8, */*;q=0.5";

/// The largest page that a fetch reads, in bytes.
const LARGEST: u64 = 10 << 20;

/// The most of the excerpts of one result of the free search that an answer holds, in characters.
const EXCERPT: usize = 1500;

/// The tags of a page that hold no text to read, which its Markdown leaves out.
const SKIPPED: [&str; 8] =
  ["head", "script", "style", "noscript", "template", "svg", "iframe", "canvas"];

/// What a work of the ear came to: the act, and its text or why it has none.
type Worked = (String, Result<Text, String>);

/// The ear of the web: it takes a web_search and a web_fetch, says each started, and says it done later with its
/// text or with the refusal. A web_search, of a query and an actor, is answered by the native search of the provider
/// of that actor, which `native` gives, or by the free search when the provider has none; its text stands at the
/// path of the query. A web_fetch of an address is answered with the page at that address, as Markdown when it is
/// HTML and as it is when it is other text, at the path of that address. A control that ends an act first ends its
/// work, and the ear says nothing more of it.
pub fn web(native: Search) -> Box<dyn Ear> {
  hearing(&["web_search", "web_fetch"], move |mut co: Co<Worked>| async move {
    let mut stops: HashMap<String, oneshot::Sender<()>> = HashMap::new();
    loop {
      let a = match co.next().await {
        Next::Heard(a) => a,
        Next::Worked((id, got)) => {
          if stops.remove(&id).is_some() {
            let got = got.map_or_else(|no| Fault::refused(no).object(), |one| one.object());
            say(&mut co, Fact::says("done", &id, [got]));
          }
          continue;
        }
      };
      let about = a.about().to_owned();
      let word =
        |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      let work: BoxFuture<'static, Result<Text, String>> = match a.kind() {
        "web_search" if a.question() => {
          let (query, search) = (word(1), native(&word(2)).unwrap_or_else(free));
          Box::pin(
            async move { search(query.clone()).await.map(|answer| Text::new(query, answer)) },
          )
        }
        "web_fetch" if a.question() => fetched(word(1)),
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

/// The free search, which asks the tool web_search of [`FREE`] for the query.
fn free() -> Searches {
  Arc::new(|query| {
    spawned(async move {
      let arguments = json!({"objective": query, "search_queries": [query]});
      let call = json!({"name": "web_search", "arguments": arguments});
      let body = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": call});
      let got = posted(FREE, &[("accept", "application/json, text/event-stream")], &body).await?;
      found(&got)
    })
  })
}

/// The answer of the free search, from what its server answered: each result under its title and its address, with
/// its excerpts cut at [`EXCERPT`] characters; or why it gives none. A server of MCP answers in JSON, or in a stream
/// of events whose data is that JSON.
fn found(got: &str) -> Result<String, String> {
  let data = match got.trim_start().starts_with('{') {
    true => got.to_owned(),
    false => got.lines().filter_map(|line| line.strip_prefix("data:")).map(str::trim).collect(),
  };
  let reply: Value =
    serde_json::from_str(&data).map_err(|no| format!("the free search answered no JSON: {no}"))?;
  if let Some(error) = reply.get("error") {
    return Err(
      error.get("message").and_then(Value::as_str).map_or_else(|| error.to_string(), str::to_owned),
    );
  }
  let result = &reply["result"];
  let parts = result["content"].as_array().map_or(&[][..], Vec::as_slice);
  let said: String = parts.iter().filter_map(|one| one["text"].as_str()).collect();
  if result["isError"] == true {
    return Err(said);
  }
  let Ok(listed) = serde_json::from_str::<Value>(&said) else { return Ok(said) };
  let results = listed["results"].as_array().map_or(&[][..], Vec::as_slice);
  let answers: Vec<String> = results
    .iter()
    .map(|one| {
      let (url, title) =
        (one["url"].as_str().unwrap_or_default(), one["title"].as_str().unwrap_or_default());
      let excerpts = one["excerpts"].as_array().map_or(&[][..], Vec::as_slice);
      let excerpts: Vec<&str> = excerpts.iter().filter_map(Value::as_str).collect();
      let excerpts = excerpts.join("\n\n");
      let cut: String = excerpts.chars().take(EXCERPT).collect();
      let more = if cut.len() < excerpts.len() { "…" } else { "" };
      format!("## [{}]({url})\n\n{}{more}", if title.is_empty() { url } else { title }, cut.trim())
    })
    .collect();
  Ok(if answers.is_empty() { "No result.".to_owned() } else { answers.join("\n\n") })
}

/// The page at an address, as a text at the path of that address: as Markdown when it is HTML, and as it is when it
/// is other text; or why it is none.
fn fetched(url: String) -> BoxFuture<'static, Result<Text, String>> {
  spawned(async move {
    let client =
      ReqwestClient::builder().user_agent(AGENT).timeout(Duration::from_secs(60)).build();
    let client = client.map_err(|no| no.to_string())?;
    let response = client
      .get(&url)
      .header("accept", ACCEPT)
      .send()
      .await
      .map_err(|no| format!("{url}: {no}"))?;
    let status = response.status();
    if !status.is_success() {
      return Err(format!("{url} answered {status}"));
    }
    let kind = response.headers().get("content-type").and_then(|one| one.to_str().ok());
    let kind = kind.unwrap_or_default().to_ascii_lowercase();
    if response.content_length().is_some_and(|size| size > LARGEST) {
      return Err(format!("{url} is larger than {LARGEST} bytes"));
    }
    let bytes = response.bytes().await.map_err(|no| format!("{url}: {no}"))?;
    if bytes.len() as u64 > LARGEST {
      return Err(format!("{url} is larger than {LARGEST} bytes"));
    }
    let content =
      page(&kind, &String::from_utf8_lossy(&bytes)).map_err(|no| format!("{url} {no}"))?;
    Ok(Text::new(url, content))
  })
}

/// The text of a page of a kind: its Markdown when it is HTML, the page itself when it is other text, and a page of
/// no kind is HTML when it opens with a tag; or why it is no text.
fn page(kind: &str, body: &str) -> Result<String, String> {
  let html = kind.contains("html") || (kind.is_empty() && body.trim_start().starts_with('<'));
  let texts = ["text/", "json", "xml", "javascript", "yaml", "toml"];
  match () {
    () if html => Ok(markdown(body)),
    () if kind.is_empty() || texts.iter().any(|one| kind.contains(one)) => Ok(body.to_owned()),
    () => Err(format!("is no text: {kind}")),
  }
}

/// The Markdown of a page of HTML, less the tags that hold no text to read, with no space at the end of a line and
/// no run of more than one empty line.
fn markdown(html: &str) -> String {
  let converter = htmd::HtmlToMarkdown::builder().skip_tags(SKIPPED.to_vec()).build();
  let converted = converter.convert(html).unwrap_or_else(|_| html.to_owned());
  let mut out: Vec<&str> = Vec::new();
  for line in converted.lines().map(str::trim_end) {
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
#[path = "web.test.rs"]
mod test;
