//! The ear of the web extension, which answers a search of the web and a fetch of a page of the web.

#![expect(clippy::doc_markdown, reason = "debt: a name in a doc comment with no backticks")]
#![expect(clippy::match_bool, reason = "debt: a match on a bool")]
#![expect(clippy::similar_names, reason = "debt: names that differ by one letter")]

use std::{collections::HashMap, sync::Arc};

use futures::{FutureExt, StreamExt, future::BoxFuture, stream};
use serde_json::{Value, json};
use unsync::oneshot;

use crate::{
  ear::{Co, Ear, Next, hearing, say},
  fact::Fact,
  value::{Fault, Text},
  world::{Search, Searches, fetched, posted, spawned},
};

/// The free search, which needs no key: the server of Parallel for agents, which answers the protocol of MCP.
const FREE: &str = "https://search.parallel.ai/mcp";

/// The most of the excerpts of one result of the free search that an answer holds, in characters.
const EXCERPT: usize = 1500;

/// What a work of the ear came to: the act, and its text or why it has none.
type Worked = (String, Result<Text, String>);

/// The ear of the web: it takes a web_search and a web_fetch, says each started, and says it done later with its
/// text or with the refusal. A web_search, of a query and an actor, is answered by the native search of the provider
/// of that actor, which `native` gives, or by the free search when the provider has none; its text stands at the
/// path of the query. A web_fetch of an address is answered with the page at that address, as Markdown when it is
/// HTML and as it is when it is other text, at the path of that address, by the same code as the question fetch of
/// the World. A control that ends an act first ends its work, and the ear says nothing more of it.
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

#[cfg(test)]
#[path = "web.test.rs"]
mod test;
