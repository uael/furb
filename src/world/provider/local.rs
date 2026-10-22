//! A local server of models: a server that serves the chat completions of OpenAI under `/v1` and lists its models at
//! `/v1/models`, as llama.cpp, LM Studio, Ollama, vLLM and Strata do.
//!
//! `FURB_LOCAL_URL` names its address, of http when it names no scheme, with `/v1` at its end or without it and with no
//! user and no password, and
//! `FURB_LOCAL_KEY` gives its key when it takes one. `FURB_LOCAL_WINDOW` gives the window of each of its models in
//! place of the one that the server gives, and `FURB_LOCAL_TOKENS` the most that a model of it writes in one turn,
//! which is a quarter of its window when it is unsaid; vLLM refuses a turn whose prompt and that most pass the
//! window. `FURB_LOCAL_THOUGHT` gives the most tokens of thought of a model of it in one turn, which is 8192 when it
//! is unsaid, and 0 gives no most: past it, the provider asks the model again with that thought and with its thought
//! turned off, as llama.cpp and vLLM read `enable_thinking` in `chat_template_kwargs`, so it writes its word at
//! once. Each of its models is named `local:` and the id that the server lists, takes no effort, and costs
//! nothing.
//!
//! A model reads the window that the server gives it: `meta.n_ctx` of llama.cpp or `max_model_len` of vLLM in the list,
//! or, from a server that gives none there, `default_generation_settings.n_ctx` at `/props` of llama.cpp, the window of
//! a loaded model at `/api/v0/models` of LM Studio, or `num_ctx` at `/api/show` or the window of a loaded model at
//! `/api/ps` of Ollama. A model whose window neither the server nor the environment gives is not offered, nor is a
//! model that the server says is not for chat. A server that does not answer in time offers no model, and it is asked
//! again once a delay passed, so a host that runs on sees a server that comes up later. A list that the server gave
//! holds for the process, but for a name of a model that it does not offer, or, for a host that lists the models, a
//! model for chat with no window, after which the server is asked again once a delay passed. A host that lists the
//! models waits for the server, five seconds at most and once in each delay while the server does not answer.

use std::{
  error::Error,
  sync::{Mutex, PoisonError},
  time::{Duration, Instant},
};

use rig_core::http_client::ReqwestClient;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
  Model,
  catalog::{LEVELS, reached},
  clients::{Client, Reach},
};
use crate::ear::reactor;

/// The provider of the models of a local server.
pub(super) const LOCAL: &str = "local";

/// The name of the environment that gives the address of the server.
pub(super) const ENV_URL: &str = "FURB_LOCAL_URL";

/// The name of the environment that gives the key of the server.
const ENV_KEY: &str = "FURB_LOCAL_KEY";

/// The name of the environment that gives the window of each model of the server.
const ENV_WINDOW: &str = "FURB_LOCAL_WINDOW";

/// The name of the environment that gives the most that a model of the server writes in one turn.
const ENV_TOKENS: &str = "FURB_LOCAL_TOKENS";

/// The name of the environment that gives the most tokens of thought of a model of the server in one turn.
const ENV_THOUGHT: &str = "FURB_LOCAL_THOUGHT";

/// The most tokens of thought of a model of the server in one turn, when the environment gives none: a model that
/// thinks more than this rarely writes a word in the turn.
const THOUGHT: u64 = 8192;

/// How long the catalog waits to connect to the server.
const CONNECT: Duration = Duration::from_secs(1);

/// How long the catalog waits for the server to list its models, with every request that the list needs, since a host
/// that lists the models waits for them. The tests wait one second here, and one second before they ask again.
#[cfg(not(test))]
const DEADLINE: Duration = Duration::from_secs(5);
#[cfg(test)]
const DEADLINE: Duration = Duration::from_secs(1);

/// How long the catalog waits before it asks again a server that listed nothing, or that listed no model of a name
/// that a roster names.
#[cfg(not(test))]
const AGAIN: Duration = Duration::from_secs(30);
#[cfg(test)]
const AGAIN: Duration = Duration::from_secs(1);

/// A local server as the environment names it: its address with no `/v1` at its end, its key, which is empty for a
/// server that takes none, the window, the most tokens of a turn and the most tokens of thought of a turn that the
/// environment gives each of its models, and what it listed and when, or why it listed nothing.
pub(super) struct Server {
  root: String,
  key: String,
  window: Option<u64>,
  tokens: Option<u64>,
  thought: Option<u64>,
  listed: Mutex<Option<Answered>>,
}

/// What a server answered when the catalog asked it for its models, and when.
type Answered = (Instant, Result<Vec<Offer>, String>);

/// What a local server listed, as the catalog offers it: its models, why it listed nothing, when it did, and why the
/// catalog offers no model of each other id that it listed.
#[derive(Default)]
pub(super) struct Heard {
  pub(super) models: Vec<Model>,
  failed: Option<String>,
  unlisted: Vec<(String, String)>,
}

/// One model that a local server lists: its id, its window when the server gives it, whether it takes images, and
/// whether it is for chat.
#[derive(Clone)]
struct Offer {
  id: String,
  window: Option<u64>,
  images: bool,
  chat: bool,
}

/// One entry of the list at `/v1/models`, which is read alone, so an entry of another shape hides no other: its id,
/// the meta of llama.cpp, the window of vLLM, and what it reads and writes as Strata says it.
#[derive(Deserialize)]
struct Entry {
  id: String,
  #[serde(default)]
  meta: Option<Meta>,
  #[serde(default)]
  max_model_len: Option<u64>,
  #[serde(default)]
  architecture: Option<Architecture>,
}

/// What llama.cpp says of a model: the window of its slot, which is zero for a model that it has not loaded.
#[derive(Deserialize)]
struct Meta {
  #[serde(default)]
  n_ctx: Option<u64>,
}

/// What a model reads and writes.
#[derive(Deserialize, Default)]
struct Architecture {
  #[serde(default)]
  input_modalities: Option<Vec<String>>,
  #[serde(default)]
  output_modalities: Option<Vec<String>>,
}

/// How the catalog asks a local server: its client, its address and its key.
struct Asking {
  client: ReqwestClient,
  root: String,
  key: String,
}

impl Server {
  /// The local server that the environment names, and none when the environment names no address.
  pub(super) fn named(env: impl Fn(&str) -> Option<String>) -> Option<Server> {
    let env = |name: &str| env(name).filter(|one| !one.trim().is_empty());
    let address = env(ENV_URL)?;
    let root = address.trim().trim_end_matches('/').trim_end_matches("/v1").trim_end_matches('/');
    // A user and a password in the address would go into each error that names it, so the key goes in its own name.
    // An address with no scheme is of http, as a local server serves it.
    let (scheme, rest) = root.split_once("://").unwrap_or(("http", root));
    let (authority, path) = rest.find('/').map_or((rest, ""), |at| rest.split_at(at));
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    let root = format!("{scheme}://{host}{path}");
    let count = |name: &str| env(name)?.trim().parse::<u64>().ok().filter(|one| *one > 0);
    let most = |given: String| {
      given.trim().parse::<u64>().map_or(Some(THOUGHT), |one| (one > 0).then_some(one))
    };
    Some(Server {
      root,
      key: env(ENV_KEY).unwrap_or_default(),
      window: count(ENV_WINDOW),
      tokens: count(ENV_TOKENS),
      thought: env(ENV_THOUGHT).map_or(Some(THOUGHT), most),
      listed: Mutex::default(),
    })
  }

  /// What the server lists, as the catalog offers it: the list it gave, or the list it gives now when it gave none
  /// yet, or a delay ago or more when it listed nothing, when it gave no model of the name that the catalog wants, or,
  /// when the catalog wants no name, when it gave no window of a model for chat. It waits for the answer of the server
  /// on the reactor, so it never runs on the thread of the reactor.
  pub(super) fn heard(&self, wanted: Option<&str>) -> Heard {
    let lists = |offers: &[Offer]| {
      let windowed = |one: &&Offer| self.window.or(one.window).is_some();
      let offered = || offers.iter().filter(|one| one.chat).all(|one| windowed(&one));
      let holds = |name: &str| {
        offers.iter().filter(|one| one.chat).filter(windowed).any(|one| names(name, &one.id))
      };
      wanted.map_or_else(offered, holds)
    };
    let held = |(at, got): &Answered| got.as_deref().is_ok_and(lists) || at.elapsed() < AGAIN;
    let lock = || self.listed.lock().unwrap_or_else(PoisonError::into_inner);
    if !lock().as_ref().is_some_and(held) {
      // The server is asked outside the lock, so what it listed already stays at hand meanwhile.
      let got = self.list();
      *lock() = Some((Instant::now(), got));
    }
    let listed = lock();
    match listed.as_ref().map(|(_, got)| got) {
      Some(Ok(offers)) => self.offered(offers),
      Some(Err(why)) => Heard { failed: Some(why.clone()), ..Heard::default() },
      None => Heard::default(),
    }
  }

  /// What the server listed last, as the catalog offers it, which asks the server nothing; and nothing when it listed
  /// nothing yet.
  pub(super) fn held(&self) -> Heard {
    let listed = self.listed.lock().unwrap_or_else(PoisonError::into_inner);
    match listed.as_ref().map(|(_, got)| got) {
      Some(Ok(offers)) => self.offered(offers),
      _ => Heard::default(),
    }
  }

  /// The models of these offers that the catalog offers, and why it offers none of the others.
  fn offered(&self, offers: &[Offer]) -> Heard {
    let mut heard = Heard::default();
    for offer in offers {
      let id = offer.id.clone();
      match self.window.or(offer.window) {
        _ if !offer.chat => {
          heard
            .unlisted
            .push((id, "the local server says that it is not a model for chat".to_owned()));
        }
        Some(window) => heard.models.push(self.model(format!("{LOCAL}:{id}"), offer, window)),
        None => {
          let why = format!("the local server gives no window of it: set {ENV_WINDOW}");
          heard.unlisted.push((id, why));
        }
      }
    }
    heard
  }

  /// A model of the server under its name, with its window: its chat completions are asked with the key of the
  /// server, it writes a quarter of its window at most in one turn, or what the environment gives, it costs nothing,
  /// it says why each turn ended, it may write its thought in its text, it may wrap its word in a fence, and it is
  /// asked again with its thought off once its thought passes the most of the server, or once its turn ended with a
  /// thought and no word.
  fn model(&self, name: String, offer: &Offer, window: u64) -> Model {
    let api = Some(format!("{}/v1", self.root));
    let reach = Reach { key: self.key.clone(), api, id: offer.id.clone(), ..Reach::default() };
    let mut model = reached(name, window, Client::Chat, Ok(reach));
    model.images = offer.images;
    model.price = Some([0.0; 4]);
    model.tokens = Some(self.tokens.unwrap_or(window / 4));
    model.ends = true;
    model.tagged = true;
    model.fenced = true;
    let off = json!({"reasoning_effort": null, "chat_template_kwargs": {"enable_thinking": false}});
    model.thought = self.thought.map(|most| (most, off.as_object().cloned().unwrap_or_default()));
    model
  }

  /// The models that the server lists now, or why it lists none.
  fn list(&self) -> Result<Vec<Offer>, String> {
    let client = ReqwestClient::builder().connect_timeout(CONNECT).build();
    let client = client.map_err(|no| no.to_string())?;
    let asking = Asking { client, root: self.root.clone(), key: self.key.clone() };
    // The windows that the server gives apart from its list take what is left of the deadline, and a list keeps its
    // models when they do not come in time.
    let listing = reactor().spawn(async move {
      let began = Instant::now();
      let listed = tokio::time::timeout(DEADLINE, asking.offers()).await;
      let mut offers = listed.map_err(|_| format!("it listed nothing in {DEADLINE:?}"))??;
      let _ = tokio::time::timeout(
        DEADLINE.saturating_sub(began.elapsed()),
        asking.windowed(&mut offers),
      )
      .await;
      Ok::<_, String>(offers)
    });
    let got = futures::executor::block_on(listing).map_err(|no| no.to_string()).and_then(|got| got);
    got.map_err(|why| format!("the local server at {} lists no model: {why}", self.root))
  }
}

impl Heard {
  /// Why the catalog offers no model of this name: why the server listed nothing, or why it does not offer that one.
  pub(super) fn why(&self, name: &str) -> Option<&str> {
    let unlisted =
      self.unlisted.iter().find(|(id, _)| names(name, id)).map(|(_, why)| why.as_str());
    self.failed.as_deref().or(unlisted)
  }
}

impl Entry {
  /// The model of this entry, with its window where the entry gives it: a model reads text where the entry says
  /// nothing of what it reads, and writes text where the entry says nothing of what it writes.
  fn offer(self) -> Offer {
    let n_ctx = self.meta.and_then(|meta| meta.n_ctx).filter(|one| *one > 0);
    let window = n_ctx.or(self.max_model_len.filter(|one| *one > 0));
    let Architecture { input_modalities, output_modalities } =
      self.architecture.unwrap_or_default();
    let holds = |sides: Option<Vec<String>>, side: &str| {
      sides.map(|sides| sides.iter().any(|one| one == side))
    };
    let images = holds(input_modalities, "image").unwrap_or(false);
    Offer { id: self.id, window, images, chat: holds(output_modalities, "text").unwrap_or(true) }
  }
}

impl Asking {
  /// What the server answers at a path, as JSON: a get, or a post of a body when one is given; or why it answered
  /// nothing.
  async fn asked(&self, path: &str, body: Option<&Value>) -> Result<Value, String> {
    let url = format!("{}{path}", self.root);
    let mut request = match body {
      Some(body) => {
        self.client.post(url).header("content-type", "application/json").body(body.to_string())
      }
      None => self.client.get(url),
    };
    if !self.key.is_empty() {
      request = request.bearer_auth(&self.key);
    }
    let text = async { request.send().await?.error_for_status()?.text().await };
    let text = text.await.map_err(|no| causes(&no.without_url()))?;
    serde_json::from_str(&text).map_err(|no| no.to_string())
  }

  /// The models that the server lists, each with the window that the list gives where it gives one.
  async fn offers(&self) -> Result<Vec<Offer>, String> {
    let listed = self.asked("/v1/models", None).await?;
    let entries = listed.get("data").and_then(Value::as_array).ok_or("its list holds no data")?;
    let entries = entries.iter().filter_map(|one| Entry::deserialize(one).ok());
    Ok(entries.map(Entry::offer).collect())
  }

  /// These models, with the windows that a server that lists none gives elsewhere, and what it says there of what
  /// each model is for: llama.cpp gives the window of the one model it serves at `/props`, LM Studio the type of each
  /// model and the window of each model that it loaded at `/api/v0/models`, since it loads the others in a window of
  /// its own, and Ollama the capabilities of a model at `/api/show`, and the
  /// window that it serves it in: the `num_ctx` of its parameters there, or the window of a model it loaded at
  /// `/api/ps`. Ollama cuts a longer prompt from its start with no error, so the length of the context that its model
  /// was trained on is no window.
  async fn windowed(&self, offers: &mut [Offer]) {
    if let [one] = offers
      && one.window.is_none()
      && let Ok(props) = self.asked("/props", None).await
    {
      one.window = positive(&props["default_generation_settings"]["n_ctx"]);
    }
    if offers.iter().any(|one| one.window.is_none())
      && let Ok(listed) = self.asked("/api/v0/models", None).await
    {
      for entry in listed["data"].as_array().into_iter().flatten() {
        let Some(one) = offers.iter_mut().find(|one| entry["id"] == one.id.as_str()) else {
          continue;
        };
        let kind = entry["type"].as_str().unwrap_or_default();
        one.window = one.window.or(positive(&entry["loaded_context_length"]));
        one.chat &= kind != "embeddings";
        one.images |= kind == "vlm";
      }
    }
    let loaded = match offers.iter().any(|one| one.window.is_none()) {
      true => self.asked("/api/ps", None).await.unwrap_or_default(),
      false => Value::Null,
    };
    for one in offers.iter_mut().filter(|one| one.window.is_none()) {
      let Ok(shown) = self.asked("/api/show", Some(&json!({"model": one.id}))).await else { break };
      let capabilities =
        shown["capabilities"].as_array().into_iter().flatten().filter_map(Value::as_str);
      let capabilities: Vec<&str> = capabilities.collect();
      if !capabilities.is_empty() {
        one.chat &= capabilities.contains(&"completion");
        one.images |= capabilities.contains(&"vision");
      }
      let parameters = shown["parameters"].as_str().unwrap_or_default();
      let set = parameters
        .lines()
        .find_map(|line| line.trim().strip_prefix("num_ctx")?.trim().parse().ok());
      let running = loaded["models"].as_array().into_iter().flatten();
      let mut running = running
        .filter(|model| model["name"] == one.id.as_str() || model["model"] == one.id.as_str());
      let running = running.find_map(|model| positive(&model["context_length"]));
      one.window = set.filter(|one| *one > 0).or(running);
    }
  }
}

/// Whether a name names the model of this id: as `local:` and the id, or as the id alone, with a level of effort after
/// a slash or with none. The catalog lets only a name with a colon reach it, so an id alone names a model only when
/// it holds one.
fn names(name: &str, id: &str) -> bool {
  let is = |one: &str| {
    one == id || one.strip_prefix(LOCAL).and_then(|rest| rest.strip_prefix(':')) == Some(id)
  };
  let leveled = |(model, level): (&str, &str)| LEVELS.contains(&level) && is(model);
  is(name) || name.rsplit_once('/').is_some_and(leveled)
}

/// A count of JSON that is more than zero.
fn positive(one: &Value) -> Option<u64> {
  one.as_u64().filter(|one| *one > 0)
}

/// An error, with each error under it.
fn causes(no: &dyn Error) -> String {
  let mut why = no.to_string();
  let mut under = no.source();
  while let Some(one) = under {
    why = format!("{why}: {one}");
    under = one.source();
  }
  why
}

#[cfg(test)]
#[path = "local.test.rs"]
pub(super) mod test;
