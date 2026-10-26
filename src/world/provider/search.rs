//! The native search of the web of each provider that has one, which the extension web asks for the actor of a
//! chain: the claude command line with its tool WebSearch alone, the web search of the Messages API of Anthropic and
//! of the Responses API of OpenAI and of xAI, the grounding of Gemini in Google Search, and the search API of
//! DeepSeek. A search answers with text in Markdown, which ends with its sources.

use std::{future::Future, process::Stdio, sync::Arc, time::Duration};

use futures::future::BoxFuture;
use rig_core::http_client::ReqwestClient;
use serde_json::{Value, json};
use tokio::{io::AsyncWriteExt, task::AbortHandle};

use super::{
  claude::Program,
  clients::{Client, Reach},
};
use crate::ear::reactor;

/// A search of the web: given a query, the answer in Markdown with its sources, or why it gives none.
pub type Searches = Arc<dyn Fn(String) -> BoxFuture<'static, Result<String, String>> + Send + Sync>;

/// How long a search or a fetch may take.
const TIMEOUT: Duration = Duration::from_secs(300);

/// What a model that searches for a query is told.
const BRIEF: &str = "Search the web to answer the query of the user. Answer in Markdown, briefly, and end with a \
                     list of the URLs of your sources.";

/// The most a model writes in the answer of one search.
const TOKENS: u64 = 4096;

/// The work of a search or a fetch, on the reactor, so that a client of HTTP and a process have the runtime they
/// need, which ends after [`TIMEOUT`], and when what awaits it goes.
pub(crate) fn spawned<T: Send + 'static>(
  work: impl Future<Output = Result<T, String>> + Send + 'static,
) -> BoxFuture<'static, Result<T, String>> {
  /// The task, which ends when its handle goes.
  struct Aborted(AbortHandle);
  impl Drop for Aborted {
    fn drop(&mut self) {
      self.0.abort();
    }
  }
  let task = reactor().spawn(async move { tokio::time::timeout(TIMEOUT, work).await });
  let aborted = Aborted(task.abort_handle());
  Box::pin(async move {
    let got = task.await;
    drop(aborted);
    match got {
      Ok(Ok(got)) => got,
      Ok(Err(_)) => Err(format!("it took more than {} seconds", TIMEOUT.as_secs())),
      Err(no) => Err(no.to_string()),
    }
  })
}

/// What an address answers a POST of a body of JSON with, as text, with these headers; or why it answers nothing
/// that succeeds.
pub(crate) async fn posted(
  url: &str,
  headers: &[(&str, &str)],
  body: &Value,
) -> Result<String, String> {
  let mut request = ReqwestClient::new().post(url).header("content-type", "application/json");
  for (name, value) in headers {
    request = request.header(*name, *value);
  }
  let response = request.body(body.to_string()).send().await.map_err(|no| no.to_string())?;
  let status = response.status();
  let text = response.text().await.map_err(|no| no.to_string())?;
  if !status.is_success() {
    return Err(format!("{url} answered {status}: {}", text.chars().take(500).collect::<String>()));
  }
  Ok(text)
}

/// The answer of a search: its text, then the list of its sources, each once, under its title.
pub(crate) fn sourced(text: &str, sources: &[(String, String)]) -> String {
  let mut lines: Vec<String> = Vec::new();
  for (title, url) in sources {
    let line = format!("- [{}]({url})", if title.is_empty() { url } else { title });
    if !url.is_empty() && !lines.iter().any(|one| one.ends_with(&format!("({url})"))) {
      lines.push(line);
    }
  }
  let text = text.trim();
  if lines.is_empty() {
    text.to_owned()
  } else {
    format!("{text}\n\nSources:\n{}", lines.join("\n"))
  }
}

/// The search of a model of the claude command line: the command line that this program starts, on that model,
/// with its tool WebSearch alone, which is told the query on its stdin and answers on its stdout.
pub(super) fn claude(program: Program, model: String) -> Searches {
  Arc::new(move |query| {
    let (program, model) = (program.clone(), model.clone());
    spawned(async move {
      let mut command = program.command();
      command.args(["-p", "--model", &model, "--output-format", "text", "--system-prompt", BRIEF]);
      command.args(["--setting-sources", "", "--strict-mcp-config", "--disable-slash-commands"]);
      command.args([
        "--no-session-persistence",
        "--allowedTools",
        "WebSearch",
        "--tools",
        "WebSearch",
      ]);
      command.env("CLAUDE_CODE_DISABLE_BUNDLED_SKILLS", "1");
      command.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
      command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
      let mut child =
        command.spawn().map_err(|no| format!("{} did not start: {no}", program.name()))?;
      // A command line that ends before it reads the query closes its stdin, and its exit says why.
      if let Some(mut stdin) = child.stdin.take()
        && let Err(no) = stdin.write_all(query.as_bytes()).await
        && no.kind() != std::io::ErrorKind::BrokenPipe
      {
        return Err(no.to_string());
      }
      let out = child.wait_with_output().await.map_err(|no| no.to_string())?;
      let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
      if out.status.success() && !text.is_empty() {
        return Ok(text);
      }
      let err = String::from_utf8_lossy(&out.stderr);
      let why = last(&err).or_else(|| last(&text)).unwrap_or_default();
      Err(format!("claude {}: {why}", out.status))
    })
  })
}

/// The last line of a text that is not empty.
fn last(text: &str) -> Option<&str> {
  text.lines().rfind(|one| !one.trim().is_empty())
}

/// The native search of a model of a client of the network that has one, on the reach of that model.
pub(super) fn native(client: Client, reach: &Reach) -> Option<Searches> {
  let native = match client {
    Client::Anthropic => Native::Anthropic,
    Client::Openai => Native::Responses("https://api.openai.com/v1"),
    Client::Xai => Native::Responses("https://api.x.ai/v1"),
    Client::Gemini => Native::Gemini,
    Client::Deepseek => Native::Deepseek,
    _ => return None,
  };
  let reach = reach.clone();
  Some(Arc::new(move |query| spawned(native.asked(reach.clone(), query))))
}

/// The API of a native search: the Messages API of Anthropic, the Responses API at its default address, the
/// generation of Gemini grounded in Google Search, and the search API of DeepSeek.
#[derive(Clone, Copy)]
enum Native {
  Anthropic,
  Responses(&'static str),
  Gemini,
  Deepseek,
}

impl Native {
  /// The answer of the API to a query, on the reach of a model.
  async fn asked(self, reach: Reach, query: String) -> Result<String, String> {
    let api = |default: &str| reach.api.clone().unwrap_or_else(|| default.to_owned());
    let bearer = format!("Bearer {}", reach.key);
    let got = match self {
      Native::Anthropic => {
        let api = api("https://api.anthropic.com");
        let url = format!("{}/v1/messages", api.trim_end_matches('/').trim_end_matches("/v1"));
        let key: (&str, &str) =
          if reach.bearer { ("authorization", &bearer) } else { ("x-api-key", &reach.key) };
        let body = json!({
          "model": reach.id,
          "max_tokens": TOKENS,
          "system": BRIEF,
          "messages": [{"role": "user", "content": query}],
          "tools": [{"type": "web_search_20250305", "name": "web_search", "max_uses": 5}],
        });
        posted(&url, &[key, ("anthropic-version", "2023-06-01")], &body).await?
      }
      Native::Responses(default) => {
        let url = format!("{}/responses", api(default).trim_end_matches('/'));
        let body = json!({
          "model": reach.id,
          "instructions": BRIEF,
          "input": [{"role": "user", "content": query}],
          "tools": [{"type": "web_search"}],
        });
        posted(&url, &[("authorization", &bearer)], &body).await?
      }
      Native::Gemini => {
        let url = format!(
          "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
          reach.id
        );
        let body = json!({
          "systemInstruction": {"parts": [{"text": BRIEF}]},
          "contents": [{"role": "user", "parts": [{"text": query}]}],
          "tools": [{"google_search": {}}],
        });
        posted(&url, &[("x-goog-api-key", &reach.key)], &body).await?
      }
      Native::Deepseek => {
        let api = api("https://api.deepseek.com");
        let root = api.trim_end_matches('/').trim_end_matches("/v1").trim_end_matches("/anthropic");
        let body = json!({"query": query, "count": 10, "need_content": false});
        posted(&format!("{root}/v1/tools/search"), &[("authorization", &bearer)], &body).await?
      }
    };
    let got: Value =
      serde_json::from_str(&got).map_err(|no| format!("the search answered no JSON: {no}"))?;
    let answer = match self {
      Native::Anthropic => anthropic(&got),
      Native::Responses(_) => responses(&got),
      Native::Gemini => gemini(&got),
      Native::Deepseek => deepseek(&got),
    };
    answer.ok_or_else(|| format!("the search answered with no text: {got}"))
  }
}

/// The text of a value of JSON at a key, or an empty text.
fn text(value: &Value, key: &str) -> String {
  value.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// The items of a value of JSON at a key, or none.
fn items<'a>(value: &'a Value, key: &str) -> &'a [Value] {
  value.get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// The answer of a response of the Messages API: its texts, then the sources they cite, or else the results of its
/// searches.
fn anthropic(got: &Value) -> Option<String> {
  let (mut texts, mut cited, mut found) = (String::new(), Vec::new(), Vec::new());
  for block in items(got, "content") {
    match block.get("type").and_then(Value::as_str) {
      Some("text") => {
        texts.push_str(&text(block, "text"));
        let citations = items(block, "citations").iter();
        cited.extend(citations.map(|one| (text(one, "title"), text(one, "url"))));
      }
      Some("web_search_tool_result") => {
        let results = items(block, "content").iter();
        found.extend(results.map(|one| (text(one, "title"), text(one, "url"))));
      }
      _ => {}
    }
  }
  (!texts.trim().is_empty())
    .then(|| sourced(&texts, if cited.is_empty() { &found } else { &cited }))
}

/// The answer of a response of the Responses API: the text of its messages, then the sources it cites.
fn responses(got: &Value) -> Option<String> {
  let (mut texts, mut cited) = (Vec::new(), Vec::new());
  for item in items(got, "output").iter().filter(|one| text(one, "type") == "message") {
    for part in items(item, "content").iter().filter(|one| text(one, "type") == "output_text") {
      texts.push(text(part, "text"));
      let citations =
        items(part, "annotations").iter().filter(|one| text(one, "type") == "url_citation");
      cited.extend(citations.map(|one| (text(one, "title"), text(one, "url"))));
    }
  }
  let texts = texts.join("\n\n");
  (!texts.trim().is_empty()).then(|| sourced(&texts, &cited))
}

/// The answer of a generation of Gemini: the text of its first candidate, then the pages that grounded it.
fn gemini(got: &Value) -> Option<String> {
  let first = items(got, "candidates").first()?;
  let parts = first.get("content").map(|one| items(one, "parts")).unwrap_or_default();
  let texts: String = parts.iter().map(|one| text(one, "text")).collect();
  let grounding = first.get("groundingMetadata");
  let chunks = grounding.map(|one| items(one, "groundingChunks")).unwrap_or_default();
  let webs = chunks.iter().filter_map(|one| one.get("web"));
  let sources: Vec<_> = webs.map(|web| (text(web, "title"), text(web, "uri"))).collect();
  (!texts.trim().is_empty()).then(|| sourced(&texts, &sources))
}

/// The answer of the search API of DeepSeek: each result, with its title, its address and its summary, or a line
/// that says it found none.
fn deepseek(got: &Value) -> Option<String> {
  let results = items(got, "results").iter().filter(|one| !text(one, "url").is_empty());
  let lines: Vec<String> = results
    .enumerate()
    .map(|(at, one)| {
      let (title, url) = (text(one, "title"), text(one, "url"));
      let head = format!("{}. [{}]({url})", at + 1, if title.is_empty() { &url } else { &title });
      let about = [text(one, "summary"), text(one, "snippet")];
      match about.into_iter().find(|one| !one.trim().is_empty()) {
        Some(about) => format!("{head}\n   {}", about.trim()),
        None => head,
      }
    })
    .collect();
  Some(if lines.is_empty() { "No result.".to_owned() } else { lines.join("\n") })
}

#[cfg(test)]
#[path = "search.test.rs"]
mod test;
