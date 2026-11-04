//! The provider of models: the ear of the World that answers what the chains stand on, and takes each reply, which a
//! model answers with its turn.
//!
//! A model is any completion model of rig, which the [`catalog`] makes of the models of the providers of the network,
//! of a local server and of the claude command line, or a function of the host that answers a request with a turn. A
//! call of a model is a future of the ear, which the life polls, and a call that its reply no longer needs ends. It
//! streams, and the host is told what the model writes as it writes it; what the reply came to the ear says of its
//! own accord. The ear reads the statements of the word of a reply as the model finishes them, and when one cannot be
//! python it ends the reply there, with the word as far as that statement, which the gate refuses. The statements that
//! the gate finds nothing against run as the model writes the rest: the ear says each new part of them as a writing
//! of the rung, a fact that the Kernel hears and runs, and a part that raised, which the Kernel says it ran with,
//! ends the reply with the word that ran. A writing is no act that the ear awaits, since a part that prompts makes a
//! chain ask this ear for a reply, which it takes only at a yield.

#![expect(unreachable_pub, reason = "debt: pub where the crate alone uses it")]
#![expect(unused_qualifications, reason = "debt: a path where its name is in scope")]
#![expect(clippy::cast_precision_loss, reason = "debt: an as cast that loses precision")]
#![expect(clippy::doc_markdown, reason = "debt: a name in a doc comment with no backticks")]
#![expect(
  clippy::format_collect,
  reason = "debt: a format! for each item, collected into a String"
)]
#![expect(clippy::format_push_string, reason = "debt: push_str of a format!, where write! serves")]
#![expect(clippy::implicit_clone, reason = "debt: to_owned or to_string where clone serves")]
#![expect(clippy::map_unwrap_or, reason = "debt: map then unwrap_or, where map_or serves")]
#![expect(
  clippy::return_self_not_must_use,
  reason = "debt: a method that returns Self with no must_use"
)]
#![expect(
  clippy::semicolon_if_nothing_returned,
  reason = "debt: a unit expression with no semicolon"
)]
#![expect(clippy::similar_names, reason = "debt: names that differ by one letter")]
#![expect(
  clippy::single_match_else,
  reason = "debt: a match of one arm and else, where if let serves"
)]
#![expect(
  clippy::struct_excessive_bools,
  reason = "debt: a struct of many bools, where an enum serves"
)]
#![expect(clippy::too_many_lines, reason = "debt: a function longer than 60 lines")]
#![expect(clippy::used_underscore_binding, reason = "debt: a binding named with _ that is used")]

pub mod catalog;
pub mod claude;
mod clients;
pub mod codex;
pub mod images;
mod local;
mod pi;
pub mod search;
mod watch;

use std::{
  collections::{HashMap, HashSet},
  path::{Path, PathBuf},
  sync::{Arc, PoisonError, RwLock},
  time::Duration,
};

use futures::{FutureExt, Stream, StreamExt, future::BoxFuture, stream};
use jiff::{Timestamp, Zoned, civil, tz::TimeZone};
use rig_core::{
  completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse, FinishReason,
  },
  http_client,
  message::{AssistantContent, Message, Reasoning, Text, UserContent},
  streaming::{StreamFinal, StreamedAssistantContent, StreamingCompletionResponse},
};
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, watch as holding};
use unsync::oneshot;

use self::{
  images::Images,
  search::{Searches, free},
  watch::Watch,
};
use crate::{
  SYSTEM,
  ear::{Co, Ear, Next, hearing, runtime, say},
  engine::{OPERATOR, WINDOW},
  fact::Fact,
  value::{Fault, Object, ObjectRef, entry},
  wire,
};

/// The name of the environment that gives the most tokens of thought of a model in one turn, as Claude Code reads it.
pub(super) const ENV_THINKING: &str = "MAX_THINKING_TOKENS";

/// The key of a config of the project that gives the most tokens of thought of a model in one turn, and the setting under
/// which the provider tells it to the claude command line.
pub(super) const KEY_THINKING: &str = "max_thinking_tokens";

/// The most tokens of thought of a model in one turn when neither the environment nor the config gives one: a model
/// that thinks more than this rarely writes a word in the turn. A model whose thought the provider cuts takes a share
/// of it at a lower effort, as [`budgeted`] says.
pub const THINKING: u64 = 32_768;

/// The most tokens of thought of a model in one turn, as Claude Code means it, 0 turning the thought off, and whether
/// the environment of furb gave it, so the claude command line inherits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thinking {
  pub most: u64,
  pub inherited: bool,
}

impl Default for Thinking {
  fn default() -> Thinking {
    Thinking { most: THINKING, inherited: false }
  }
}

/// The most tokens of thought of a model in one turn, as the environment gives it, which wins, else as the key
/// `max_thinking_tokens` of the configs of the project in the directory of the life gives it, `.furb/config.local.json`
/// over `.furb/config.json`, else [`THINKING`]. A value that is no count of tokens is as unsaid.
pub(super) fn thinking(env: Option<&str>, directory: &std::path::Path) -> Thinking {
  let given = env.and_then(|one| one.trim().parse::<u64>().ok());
  let read = |file: std::path::PathBuf| -> Option<u64> {
    let text = std::fs::read_to_string(file).ok()?;
    serde_json::from_str::<Value>(&text).ok()?.get(KEY_THINKING)?.as_u64()
  };
  let config = || crate::extension::projected(directory).into_iter().rev().find_map(read);
  Thinking { most: given.or_else(config).unwrap_or(THINKING), inherited: env.is_some() }
}

/// How a model takes the most tokens of thought of one turn.
#[derive(Clone)]
pub(super) enum Thought {
  /// Its server streams its thought one token in each part, so the provider cuts its thought at the most and asks it
  /// again with these settings, which turn its thought off, or asks it with them from the first request when the most
  /// is 0. A setting of null takes that setting off the request.
  Off(Map<String, Value>),
  /// It takes the most as the setting `max_thinking_tokens`, which the provider gives it unless the environment of
  /// furb gave the most, which it then inherits.
  Told,
}

/// The settings with these settings put in, a setting of null taking that setting off.
fn merged(settings: &mut Map<String, Value>, with: &Map<String, Value>) {
  for (key, value) in with {
    match value {
      Value::Null => settings.remove(key),
      one => settings.insert(key.clone(), one.clone()),
    };
  }
}

/// What a turn of a model tells as it streams: what it added to its text, and what it added to its thought.
pub type Told = Arc<dyn Fn(&str, &str) + Send + Sync>;

/// What the host is told while a model writes: the rung it writes for, the chain of that rung, and what it added to
/// its text and to its thought. It is no fact, and the turn that the reply is done with holds all of it.
pub type Writes = Arc<dyn Fn(&str, &str, &str, &str) + Send + Sync>;

/// What the host is told when a call of a model failed for a cause that passes, and the provider asks the model
/// again after a wait: the rung the reply is for, the chain of that rung, and the retry. What the model wrote before
/// the failure is void, and the turn of the reply holds none of it.
pub type Retries = Arc<dyn Fn(&str, &str, &Retry) + Send + Sync>;

/// A retry of a call of a model: the attempt that comes next, the most attempts of the call, the time the provider
/// waits before that attempt, and why the attempt before it failed, in a few words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retry {
  pub attempt: u32,
  pub most: u32,
  pub wait: Duration,
  pub why: String,
}

/// How the provider asks a model again after a failure that passes: the most attempts of one call, and the wait
/// before the second attempt, which doubles before each later attempt, to a minute at most.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
  pub most: u32,
  pub first: Duration,
}

/// The backoff of a provider that is given none: ten attempts, and four seconds before the second.
pub const BACKOFF: Backoff = Backoff { most: 10, first: Duration::from_secs(4) };

impl Backoff {
  /// The wait before an attempt, from the second on.
  fn wait(self, attempt: u32) -> Duration {
    let doubled = self.first.saturating_mul(1 << attempt.saturating_sub(2).min(16));
    doubled.min(Duration::from_secs(60))
  }
}

/// A cause of a failed call that passes, so the provider asks the model again. Its text is the why
/// of the retry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
enum Transient {
  /// The program of the claude command line was missing, as while an update replaces it.
  #[error("the program is missing")]
  Missing,
  /// The account of the claude command line was parked, and the next call runs as another one.
  #[error("{0}")]
  Switched(String),
  #[error("connection reset")]
  Reset,
  #[error("timed out")]
  TimedOut,
  #[error("the provider is overloaded")]
  Overloaded,
  #[error("rate limited")]
  RateLimited,
  #[error("connection lost")]
  Lost,
  /// A status of the side of the provider: 500, 502, 503 or 504.
  #[error("server error {0}")]
  Server(u16),
  #[error("network error")]
  Network,
}

/// The HTTP status that a failure carries: the status of an HTTP error of rig, or the status that
/// heads the text of a provider, as `503 Service Unavailable`, or that follows `status code`, as
/// rig words it. A number elsewhere in the text is no status.
fn status(no: &CompletionError) -> Option<u16> {
  let said = match no {
    CompletionError::HttpError(
      http_client::Error::InvalidStatusCode(code)
      | http_client::Error::InvalidStatusCodeWithMessage(code, _)
      | http_client::Error::InvalidStatusCodeWithDetails { status: code, .. },
    ) => return Some(code.as_u16()),
    CompletionError::HttpError(no) => no.to_string(),
    CompletionError::ProviderError(why) => why.clone(),
    _ => return None,
  };
  let lower = said.to_lowercase();
  let after = lower.split_once("status code").map_or(lower.as_str(), |(_, rest)| rest.trim_start());
  let head: String = after.chars().take_while(char::is_ascii_digit).collect();
  let ends = after[head.len()..].chars().next().is_none_or(|one| !one.is_ascii_alphanumeric());
  if head.len() == 3 && ends { head.parse().ok() } else { None }
}

/// Why a call of a model failed, when the cause passes: the connection was lost or timed out, the
/// provider is overloaded, limits the rate, or failed on its side, or the program of the claude
/// command line was missing, as while an update replaces it. A failure of the request, of the turn,
/// or of a model that the provider cannot make does not pass, and the model is not asked again.
fn passing(no: &CompletionError) -> Option<Transient> {
  match claude::error(no) {
    Some(claude::Error::Missing(_)) => return Some(Transient::Missing),
    Some(why @ claude::Error::Switched(_)) => return Some(Transient::Switched(why.to_string())),
    Some(_) => return None,
    None => {}
  }
  let said = match no {
    CompletionError::HttpError(no) => no.to_string(),
    CompletionError::ProviderError(why) => why.clone(),
    _ => return None,
  }
  .to_lowercase();
  let has = |words: &[&str]| words.iter().any(|one| said.contains(one));
  let status = status(no);
  let why = if has(&["connection reset", "reset by peer"]) {
    Transient::Reset
  } else if has(&["timed out", "timeout"]) || status == Some(408) {
    Transient::TimedOut
  } else if has(&["overloaded"]) || status == Some(529) {
    Transient::Overloaded
  } else if has(&["rate limit", "too many requests"]) || status == Some(429) {
    Transient::RateLimited
  } else if has(&["connection closed", "connection error", "broken pipe", "error sending request"])
  {
    Transient::Lost
  } else if let Some(code @ (500 | 502 | 503 | 504)) = status {
    Transient::Server(code)
  } else if has(&[
    "connection refused",
    "could not resolve",
    "dns error",
    "tls handshake",
    "network is unreachable",
  ]) {
    Transient::Network
  } else {
    return None;
  };
  Some(why)
}

/// A failed call in words: a failure of the claude command line as a provider error, as it always
/// read.
fn failure(no: &CompletionError) -> String {
  claude::error(no).map_or_else(|| no.to_string(), |why| format!("ProviderError: {why}"))
}

/// Whether a failed call can be corrected by another turn of the model: a transient provider fault,
/// a cut turn, or a request that does not fit the model's context.
pub(super) fn recoverable(no: &CompletionError) -> bool {
  if passing(no).is_some() {
    return true;
  }
  let text = match no {
    CompletionError::ResponseError(text) | CompletionError::ProviderError(text) => {
      text.to_lowercase()
    }
    CompletionError::HttpError(error) => error.to_string().to_lowercase(),
    _ => return false,
  };
  [
    "reached the most it writes before its end",
    "stopped before its end",
    "context length",
    "context window",
    "maximum context",
    "prompt is too long",
    "too many tokens",
    "maximum tokens",
    "token limit",
  ]
  .iter()
  .any(|phrase| text.contains(phrase))
}

/// The wait before the claude command line is asked again when its message gives no moment for the end of a limit.
const LIMIT_POLL: Duration = Duration::from_secs(300);

/// The wait before the claude command line is asked again after a failed sign in, which a sign in outside furb ends.
const SIGN_POLL: Duration = Duration::from_secs(60);

/// The time after the end of a limit that the provider waits, so the call is not made before the limit lifts.
const LIMIT_SLACK: Duration = Duration::from_secs(1);

/// The moment a usage limit ends, from the message of the claude command line: `resets 5:10pm (Europe/Paris)`, with
/// a day before the time, as `resets Oct 28, 3pm`, when it ends in a later day, a time with no zone being in the zone
/// of the machine. A time with no day is the next one after now.
fn resets(said: &str, now: &Zoned) -> Option<Zoned> {
  let rest = said.split_once("resets")?.1.trim();
  let (head, zone) = match rest.split_once('(') {
    Some((head, zone)) => (head, TimeZone::get(zone.split_once(')')?.0.trim()).ok()?),
    None => (rest, now.time_zone().clone()),
  };
  let now = now.with_time_zone(zone.clone());
  let mut words: Vec<_> =
    head.split([' ', ',']).filter(|one| !one.is_empty() && *one != "at").collect();
  let time = words.pop()?.to_lowercase();
  let (clock, pm) = match time.strip_suffix("pm") {
    Some(clock) => (clock, true),
    None => (time.strip_suffix("am")?, false),
  };
  let (hour, minute) = clock.split_once(':').unwrap_or((clock, "0"));
  let (hour, minute): (i8, i8) = (hour.parse().ok()?, minute.parse().ok()?);
  if !(1..=12).contains(&hour) {
    return None;
  }
  let hour = hour % 12 + if pm { 12 } else { 0 };
  let at = |date: civil::Date| date.at(hour, minute, 0, 0).to_zoned(zone.clone()).ok();
  match words.as_slice() {
    [] => {
      let today = at(now.date())?;
      if today > now { Some(today) } else { at(now.date().tomorrow().ok()?) }
    }
    [month, day] => {
      const MONTHS: [&str; 12] =
        ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
      let month =
        i8::try_from(MONTHS.iter().position(|one| month.to_lowercase().starts_with(one))? + 1)
          .ok()?;
      let day = day.parse().ok()?;
      let this = at(civil::Date::new(now.year(), month, day).ok()?)?;
      if this > now { Some(this) } else { at(civil::Date::new(now.year() + 1, month, day).ok()?) }
    }
    _ => None,
  }
}

/// A function of the host that stands in for a model: it is given a request, as JSON, and what it tells as it
/// writes, and it answers with the turn of the model, or with why it gives none.
pub type Hosted =
  Arc<dyn Fn(Value, Told) -> BoxFuture<'static, Result<Object, String>> + Send + Sync>;

/// The native search of the web of the model that an actor names, if its provider has one.
pub type Search = Arc<dyn Fn(&str) -> Option<Searches> + Send + Sync>;

/// What streams one response of a model of rig, whatever model it is.
type Streams = Arc<
  dyn Fn(
      CompletionRequest,
    ) -> BoxFuture<'static, Result<StreamingCompletionResponse, CompletionError>>
    + Send
    + Sync,
>;

/// The streams of a model of rig.
fn streams<M: CompletionModel + 'static>(model: M) -> Streams {
  let model = Arc::new(model);
  Arc::new(move |request| {
    let model = Arc::clone(&model);
    Box::pin(async move { model.stream(request).await })
  })
}

/// One turn asked of a model: the actor and the chain it answers, the request, what it tells as it streams, and the
/// budget of its thought.
#[derive(Clone)]
struct Asked {
  actor: String,
  chain: String,
  request: CompletionRequest,
  told: Told,
  thought: Option<(u64, Map<String, Value>)>,
}

/// What a model answers a turn with: the response of a model of rig, or the turn that a function of the host gave.
enum Answer {
  Response(Box<CompletionResponse>),
  Turn(Object),
}

/// What asks a model for one turn.
type Answers =
  Arc<dyn Fn(Asked) -> BoxFuture<'static, Result<Answer, CompletionError>> + Send + Sync>;

/// The class of a model, which automatic work reads: a cheap model takes the work that no one waits on, and a dear one
/// the work of the operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Class {
  Cheap,
  Dear,
}

impl Class {
  /// The name of the class, as the roster, the catalog and `.furb/config.json` say it.
  pub fn name(self) -> &'static str {
    match self {
      Class::Cheap => "cheap",
      Class::Dear => "dear",
    }
  }

  /// The class of a name, and nothing for another word.
  pub fn named(name: &str) -> Option<Class> {
    match name {
      "cheap" => Some(Class::Cheap),
      "dear" => Some(Class::Dear),
      _ => None,
    }
  }
}

/// The most that a cheap model costs, in dollars for a million tokens that it reads.
pub const CHEAP: f64 = 1.0;

/// The key of a config whose map gives a model the class that it names, over the one of the
/// catalog.
pub(super) const KEY_CLASSES: &str = "classes";

/// The key of a config whose list names the models of automatic work, the first first.
pub(super) const KEY_AUTOMATIC: &str = "automatic";

/// What the configs say of the models: the class of each model that a config names, and the models
/// of automatic work.
#[derive(Default)]
pub(super) struct Configured {
  classes: HashMap<String, Class>,
  automatic: Vec<String>,
}

/// What the configs of the user, in the config directory, and of the project, in a directory, say
/// of the models, in the order of [`crate::extension::configs`]: a later file gives a model another
/// class over an earlier one, and its list of automatic work replaces the list of an earlier one. A
/// word that names no class, and a name of the list that is no string, give nothing.
pub(super) fn configured(user: &Path, directory: &Path) -> Configured {
  let mut given = Configured::default();
  for file in crate::extension::configs(user, directory) {
    let Ok(text) = std::fs::read_to_string(file) else { continue };
    let Ok(config) = serde_json::from_str::<Value>(&text) else { continue };
    for (name, class) in config.get(KEY_CLASSES).and_then(Value::as_object).into_iter().flatten() {
      if let Some(class) = class.as_str().and_then(Class::named) {
        given.classes.insert(name.clone(), class);
      }
    }
    if let Some(list) = config.get(KEY_AUTOMATIC).and_then(Value::as_array) {
      given.automatic = list.iter().filter_map(Value::as_str).map(str::to_owned).collect();
    }
  }
  given
}

/// A model the provider offers: its name, the window it reads, the efforts it spends, what it costs, whether it takes
/// an image, and what answers it.
///
/// An actor names it as its name alone, or as its name and one of its efforts after a slash. The provider tells an
/// effort to the model as the settings that effort adds to the request, since each provider reads an effort its own
/// way. A model that says what a turn cost says it as the number `cost` of its raw record, and the price of a model
/// counts the cost of one that says none.
#[derive(Clone)]
pub struct Model {
  name: String,
  window: u64,
  efforts: Vec<(String, Map<String, Value>)>,
  conversation: Option<String>,
  price: Option<[f64; 4]>,
  images: bool,
  /// Whether it counts what it read of the cache apart from the rest of what it read, as Anthropic does.
  apart: bool,
  /// The most it writes in one turn, which a provider that asks for it is told.
  tokens: Option<u64>,
  /// Whether it says why each of its turns ended, so a turn that says nothing of its end, or that the most it writes
  /// ended, was cut.
  ends: bool,
  /// Whether its server may write its thought in its text, between `<think>` and `</think>`, which the host is told as
  /// text while it streams.
  tagged: bool,
  /// Whether it may wrap its word in a fence or in a tag of markup, as a model of a local server does, so its turn
  /// loses those marks, as [`unfenced`] says.
  fenced: bool,
  /// Whether a function of the host answers in place of the model, which tells what it likes while it works, so what
  /// it tells is no part of the word that it gives.
  hosted: bool,
  /// How it takes the most tokens of thought of one turn, when it takes one.
  thought: Option<Thought>,
  /// Its class, which the catalog gives, else its price.
  class: Option<Class>,
  /// Whether it reads its earlier turns with no thought, as a model of a local server, whose own template drops the
  /// thought of each earlier turn, and whose thought of each turn would else fill its window in a few turns.
  forgets: bool,
  answers: Answers,
  /// The native search of the web of its provider, which the provider asks for an actor that
  /// names it.
  search: Option<Searches>,
}

impl Model {
  /// A model of rig under a name, with the window it reads, and no effort yet.
  pub fn new<M>(name: impl Into<String>, window: u64, model: M) -> Model
  where
    M: CompletionModel + 'static,
  {
    let streams = streams(model);
    Model::lazy(name, window, move || Ok(Arc::clone(&streams)))
  }

  /// A model of rig that is made when it is first asked, or the reason it cannot be.
  fn lazy(
    name: impl Into<String>,
    window: u64,
    made: impl Fn() -> Result<Streams, CompletionError> + Send + Sync + 'static,
  ) -> Model {
    let answers: Answers = Arc::new(move |asked| {
      let made = made();
      Box::pin(async move { Ok(Answer::Response(Box::new(streamed(made?, asked).await?))) })
    });
    Model {
      name: name.into(),
      window,
      efforts: Vec::new(),
      conversation: None,
      price: None,
      images: false,
      apart: false,
      tokens: None,
      ends: false,
      tagged: false,
      fenced: false,
      hosted: false,
      forgets: false,
      thought: None,
      class: None,
      answers,
      search: None,
    }
  }

  /// One effort the model spends, and the settings that it adds to the request, a map of rig's additional
  /// parameters: a budget of thinking for one provider, an effort of reasoning for another.
  pub fn effort(mut self, name: impl Into<String>, settings: Value) -> Model {
    let settings = match settings {
      Value::Object(settings) => settings,
      _ => Map::new(),
    };
    self.efforts.push((name.into(), settings));
    self
  }

  /// The setting that carries the conversation of a reply, the chain of this life, which a provider keeps a
  /// conversation or a cache by.
  pub fn conversation(mut self, key: impl Into<String>) -> Model {
    self.conversation = Some(key.into());
    self
  }

  /// What the model costs, in dollars for a million tokens: read, written, read from the cache, and written to it.
  pub fn price(mut self, input: f64, output: f64, read: f64, wrote: f64) -> Model {
    self.price = Some([input, output, read, wrote]);
    self
  }

  /// The model takes the images that a turn names.
  pub fn images(mut self) -> Model {
    self.images = true;
    self
  }

  /// The model, whose provider searches the web with this search.
  pub fn searches(mut self, search: Searches) -> Model {
    self.search = Some(search);
    self
  }

  /// The same model, whose turns a function of the host answers in place of the model.
  pub fn hosted(mut self, host: Hosted) -> Model {
    self.hosted = true;
    self.answers = Arc::new(move |asked| {
      let Asked { actor, chain, request, told, .. } = asked;
      let settings = request.additional_params.unwrap_or_else(|| json!({}));
      let request = json!({
        "actor": actor,
        "chain": chain,
        "messages": request.chat_history,
        "settings": settings,
      });
      let got = host(request, told);
      Box::pin(async move { got.await.map(Answer::Turn).map_err(CompletionError::ProviderError) })
    });
    self
  }

  /// Whether what the model streams as text is its word, so the statements of it that are finished can be read: the
  /// text of a model whose server wraps its word, or writes its thought in it, is cleaned after it ends, and a
  /// function of the host tells what it likes.
  fn streams_its_word(&self) -> bool {
    !(self.hosted || self.tagged || self.fenced)
  }

  /// The name an actor names the model by.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// The window the model reads, in tokens.
  pub fn window(&self) -> u64 {
    self.window
  }

  /// The efforts the model spends, from least to most.
  pub fn efforts(&self) -> Vec<&str> {
    self.efforts.iter().map(|(name, _)| name.as_str()).collect()
  }

  /// The actor of this model at a level, moved to the nearest one the model takes, and at [`catalog::LEVEL`] when the
  /// level is unsaid; the model alone when it takes no effort.
  pub fn at(&self, level: Option<&str>) -> String {
    match catalog::clamp(&self.efforts(), level.unwrap_or(catalog::LEVEL)) {
      Some(level) => format!("{}/{level}", self.name),
      None => self.name.clone(),
    }
  }

  /// Whether the model takes the images that a turn names.
  pub fn sees(&self) -> bool {
    self.images
  }

  /// What the model costs, in dollars for a million tokens: read, written, read from the cache, and written to it.
  pub fn priced(&self) -> Option<[f64; 4]> {
    self.price
  }

  /// The model, of a class that the catalog gives, which its price no longer gives.
  pub fn classed(mut self, class: Class) -> Model {
    self.class = Some(class);
    self
  }

  /// The class of the model: the one that the catalog gave, else cheap when a million tokens that
  /// it reads cost [`CHEAP`] dollars or less, else dear, as a model that says no price is.
  pub fn class(&self) -> Class {
    self.class.unwrap_or(match self.price {
      Some([input, ..]) if input <= CHEAP => Class::Cheap,
      _ => Class::Dear,
    })
  }

  /// The settings of an actor that names this model: none for its name alone, and those of its effort for its name
  /// and an effort it spends.
  fn settings(&self, actor: &str) -> Option<Map<String, Value>> {
    if actor == self.name {
      return Some(Map::new());
    }
    let effort = actor.strip_prefix(self.name.as_str())?.strip_prefix('/')?;
    self.efforts.iter().find(|(name, _)| name == effort).map(|(_, settings)| settings.clone())
  }
}

/// The provider: the directory the life stands on, the models it offers, the default actor, the most tokens of thought
/// of a model in one turn, the directory of the images that a turn names, whom it tells what a model writes, and the
/// extensions that it gives in the standing.
pub struct Provider {
  directory: String,
  config: Option<PathBuf>,
  extensions: Option<Vec<Advertised>>,
  models: Arc<RwLock<Vec<Model>>>,
  actor: Option<String>,
  thinking: Thinking,
  images: Option<PathBuf>,
  writes: Option<Writes>,
  backoff: Backoff,
  retries: Option<Retries>,
  refresh: Option<Refresh>,
  gone: Vec<String>,
  searching: bool,
}

/// An extension as the standing gives it: its name, the file of its word, its life word, and whether it plays on the
/// root alone.
pub type Advertised = (String, String, String, bool);

/// The models of a provider, its default actor and the most tokens of thought of a turn, as the
/// settings give them.
type Settings = (Vec<Model>, Option<String>, Thinking);

type Refresh = Arc<dyn Fn() -> Result<Settings, String> + Send + Sync>;

impl Provider {
  /// A provider of these models, on a directory, whose model thinks [`THINKING`] tokens at most in one turn.
  pub fn new(directory: impl Into<String>, models: Vec<Model>) -> Provider {
    let directory = directory.into();
    Provider {
      directory,
      config: None,
      extensions: None,
      models: Arc::new(RwLock::new(models)),
      actor: None,
      thinking: Thinking::default(),
      images: None,
      writes: None,
      backoff: BACKOFF,
      retries: None,
      refresh: None,
      gone: Vec::new(),
      searching: true,
    }
  }

  /// The config directory of the user whose `config.json` the standing reads for the classes
  /// and the list of automatic work, else the one that [`crate::extension::Places::here`]
  /// names: a life that opens on a config directory of its own reads no file of the machine.
  pub fn config(mut self, config: impl Into<PathBuf>) -> Provider {
    self.config = Some(config.into());
    self
  }

  /// The most tokens of thought of a model in one turn.
  pub fn thinking(mut self, thinking: Thinking) -> Provider {
    self.thinking = thinking;
    self
  }

  /// Whether the provider answers the question search. A life that inspects searches nothing.
  pub(crate) fn searching(mut self, on: bool) -> Provider {
    self.searching = on;
    self
  }

  pub(crate) fn refreshing(mut self, refresh: Refresh) -> Provider {
    self.refresh = Some(refresh);
    self
  }

  /// The names of the roster and of the actor that the catalog lacked when the life opened. The ear
  /// tells each chain once, at its first stand, and the standing leaves them out.
  pub fn gone(mut self, names: Vec<String>) -> Provider {
    self.gone = names;
    self
  }

  pub(crate) fn into_settings(self) -> (Vec<Model>, Option<String>, Thinking) {
    let models = self.models.read().unwrap_or_else(PoisonError::into_inner).clone();
    (models, self.actor, self.thinking)
  }

  /// The actor a prompt goes to when it names none, which is the first model at the level of an actor that names
  /// none when none is given, and the operator when there is no model.
  pub fn actor(mut self, actor: Option<String>) -> Provider {
    self.actor = actor;
    self
  }

  /// The extensions that the standing gives, from which the engine seeds the root of a new record.
  pub fn advertises(mut self, extensions: Vec<Advertised>) -> Provider {
    self.extensions = Some(extensions);
    self
  }

  /// The directory of the images that a turn names.
  pub fn images(mut self, directory: impl Into<PathBuf>) -> Provider {
    self.images = Some(directory.into());
    self
  }

  /// Whom the provider tells what a model writes, as it writes it.
  pub fn writes(mut self, writes: Writes) -> Provider {
    self.writes = Some(writes);
    self
  }

  /// How the provider asks a model again after a failure that passes.
  pub fn backoff(mut self, backoff: Backoff) -> Provider {
    self.backoff = backoff;
    self
  }

  /// Whom the provider tells each retry of a call of a model, before its wait.
  pub fn retries(mut self, retries: Retries) -> Provider {
    self.retries = Some(retries);
    self
  }

  /// The native search of the web of the model that an actor names, or none when its provider has none or no
  /// model of the provider is that actor.
  pub fn search(&self) -> Search {
    let models = Arc::clone(&self.models);
    Arc::new(move |actor| {
      models
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|model| model.settings(actor).is_some())
        .and_then(|model| model.search.clone())
    })
  }

  /// The ear of the provider: it answers a stand with the standing, the roster of its models and the operator, the
  /// directory the life stands on, and the default actor; and it takes a reply, which the model of its actor
  /// answers.
  ///
  /// A reply reads the turns of its chain: the system prompt is the engine, a user turn goes as its
  /// python and the images that the prompts it opens name, and an assistant turn as the blocks its
  /// provider gave. It is done with the turn of the model, or with a refusal; a recoverable refusal
  /// gives the model another turn with its cause, and never pauses the chain. A turn whose word
  /// lost the marks of a fence or of a tool call around it, or the line of three backticks at its
  /// end, tells its chain so once, before its done. A done of a reply that the ear did not say ends
  /// the call of its model.
  ///
  /// The ear says each part of a word that the model finished and that the gate finds nothing against as a writing of
  /// its rung, unless a pause holds the rung. A reply whose part ran is done with the word that ran when the Kernel
  /// says that the part ran with a raise, when a later statement holds a fault, and when its call fails, which is
  /// then made no more; and with its turn when the gate finds nothing against the whole word, which holds the word
  /// that ran.
  ///
  /// A stand from outside an act reads the settings again, after the first: it starts, and the ear
  /// reads them on the runtime of the ears, so every chain of the life goes on meanwhile, and it is
  /// done with the standing once they come, or refused with why they did not. A stand that a word
  /// makes, or the first, is done at once.
  ///
  /// The first stand on each chain tells it which names of the roster and of the actor the
  /// catalog lacked when the life opened, once, before its done.
  ///
  /// A stand ends the wait of each reply whose call waits for a retry, a usage limit among them, and asks its model
  /// again at once, on the models that stand then: so a change of the command line or of its account, or the operator
  /// who asks to retry now, ends a wait at once.
  ///
  /// A pause over the chain of a reply holds the next call of its model: the call that waits for a retry, or that a
  /// stand asks again, is made at the wake, and not at the end of its wait. A call that runs returns.
  pub fn ear(self) -> Box<dyn Ear> {
    let native = self.search();
    let Provider {
      directory,
      config,
      extensions,
      models,
      actor,
      mut thinking,
      images,
      writes,
      backoff,
      retries,
      refresh,
      gone,
      searching,
    } = self;
    let mut actor = actor.unwrap_or_else(|| {
      models
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .first()
        .map_or_else(|| OPERATOR.to_owned(), |model| model.at(None))
    });
    let mut images = Images::new(images);
    hearing(&["stand", "reply", "search", RAN], move |mut co: Co<Replied>| async move {
      // The ids of chains repeat in every life, so a conversation of this life is keyed by the life too.
      let life = uuid().map_err(|no| Fault::refused(no.to_string()))?;
      let mut replies: HashMap<String, Reply> = HashMap::new();
      // The stops of the searches that work, which end a search that a control ended first.
      let mut searches: HashMap<String, oneshot::Sender<()>> = HashMap::new();
      // The actor whose last reply on each chain the ear refused.
      let mut mute: HashMap<String, String> = HashMap::new();
      // The opening of the life loaded the settings just now, so the first stand, which the boot asks, reads them
      // again only after it: a refresh there asks each program of a model, as the claude command line, a second time.
      let mut stood = false;
      // The stand that the ear answers next with the standing, and whether a word made it.
      let mut stands: Option<(String, bool)> = None;
      // The chains that the ear told which names left the catalog.
      let mut lacked: std::collections::HashSet<String> = std::collections::HashSet::new();
      loop {
        if let Some((about, word)) = stands.take() {
          let given = standing(
            &models.read().unwrap_or_else(PoisonError::into_inner),
            &directory,
            &actor,
            extensions.as_deref(),
            &config.clone().unwrap_or_else(|| crate::extension::Places::here().config),
          );
          say(&mut co, Fact::says("done", &about, [given]));
          // A stand reads the settings again, as after a change of the command line or of its
          // account, or when the operator asks to retry now, so a reply whose call waits for a
          // retry, which can wait for days at a usage limit, ends its wait and asks again at once,
          // on the models that stand now.
          let waiting: Vec<String> = replies
            .iter()
            .filter(|(_, reply)| reply.waits && !word)
            .map(|(id, _)| id.clone())
            .collect();
          for id in waiting {
            let Some(reply) = replies.get(&id) else { continue };
            let (chain, actor) = (reply.chain.clone(), reply.actor.clone());
            let (watch, stop, hold) = asking(
              &mut co,
              &id,
              &chain,
              &actor,
              &life,
              &models,
              &mut images,
              thinking,
              backoff,
              true,
            )
            .await?;
            if let Some(reply) = replies.get_mut(&id) {
              reply.waits = false;
              reply.watch = watch.then(Watch::default);
              reply.hold = hold;
              reply._stop = stop;
            }
          }
        }
        let a = match co.next().await {
          Next::Heard(a) => a,
          // The settings that a refresh read, on which the provider stands from now on, or why it
          // read none.
          Next::Worked(Replied::Refreshed { about, got }) => {
            match got {
              Ok((fresh, default, budget)) => {
                if !fresh.iter().any(|model| model.settings(&actor).is_some()) {
                  actor = default
                    .or_else(|| fresh.first().map(|model| model.at(None)))
                    .unwrap_or_else(|| OPERATOR.to_owned());
                }
                *models.write().unwrap_or_else(PoisonError::into_inner) = fresh;
                thinking = budget;
                stands = Some((about, false));
              }
              Err(why) => say(&mut co, Fact::says("done", &about, [Fault::refused(why).object()])),
            }
            continue;
          }
          // What the model of a reply that still stands writes, told to the host under the rung it writes for.
          Next::Worked(Replied::Wrote { id, text, thinking }) => {
            if let Some(reply) = replies.get_mut(&id) {
              reply.waits = false;
            }
            if let (Some(writes), Some(reply)) = (&writes, replies.get(&id)) {
              writes(&reply.rung, &reply.chain, &text, &thinking);
            }
            let Some((chain, rung)) =
              replies.get(&id).map(|one| (one.chain.clone(), one.rung.clone()))
            else {
              continue;
            };
            let Some(watch) = replies.get_mut(&id).and_then(|reply| reply.watch.as_mut()) else {
              continue;
            };
            let Some(end) = watch.wrote(&text).filter(|_| !text.is_empty()) else { continue };
            let read = watch.upto(end).to_owned();
            // The statements before the last line at the left edge, as python: each quote in them bound as a string.
            let word = if !read.contains(watch::QUOTE) {
              read
            } else if watch::open(&read) {
              continue;
            } else {
              let got = co.call("unquoted", vec![Object::string(&read)], vec![]).await;
              match got.ok().and_then(|one| one.as_ref().as_str().map(str::to_owned)) {
                Some(word) => word,
                None => continue,
              }
            };
            let (line, why) = match watch::broken(&word) {
              Some(line) => (line, format!("line {line}: the statement there cannot be python")),
              None => {
                let found = findings(&mut co, &chain, &word).await;
                if found.is_empty() {
                  // The word holds no fault so far, so its new statements run now, as the turn holds them: from its
                  // first line that is not blank. A statement whose clause starts at the end, as an `else`, waits.
                  let (word, end) = match watch.clause(end).then(|| watch::last(&word)).flatten() {
                    Some(line) => {
                      let held: String = word.split_inclusive('\n').take(line - 1).collect();
                      (held, watch.start(line))
                    }
                    None if watch.clause(end) => continue,
                    None => (word, end),
                  };
                  let given = word.trim_start();
                  let Some((part, before)) = watch.part(given) else { continue };
                  if writing(&mut co, &rung, part, before).await {
                    watch.gave(given, end);
                  } else if watch.ran().is_none() {
                    // A word whose first part a pause held is given whole, so no later part of it is given.
                    watch.untaken();
                  }
                  continue;
                }
                let settled = watch::settled(&word);
                let Some(line) = watch::refused(&settled, &found) else { continue };
                (line, found.join("\n"))
              }
            };
            // The word as far as the statement that cannot be python, or that the gate refuses for good, is the turn
            // of the reply, so the gate refuses it and the chain tells why; and once a part of the word ran, the
            // turn is the word that ran, which the chain runs as it ran, and the chain is told why before it. The
            // model wrote no more of it that counts, and its call ends with the reply.
            let ran = watch.ran().map(str::to_owned);
            let wrapped = watch.note();
            let Ok(turn) = cut(ran.as_deref().unwrap_or_else(|| watch.through(line))) else {
              continue;
            };
            if let Some(Reply { chain, .. }) = replies.remove(&id) {
              mute.remove(&chain);
              if let Some(note) = wrapped {
                told(&mut co, &chain, &format!("#{rung} {note}"));
              }
              if ran.is_some() {
                let ran = ending(ran.as_deref().unwrap_or_default());
                noted(
                  &mut co,
                  &chain,
                  &rung,
                  AFTER,
                  &[("findings", &afterward(&why)), ("ran", &ran)],
                )
                .await;
              }
              say(&mut co, Fact::says("done", &id, [turn]));
            }
            continue;
          }
          // A retry of the call of the model of a reply that still stands, told to the host under its rung.
          Next::Worked(Replied::Retried { id, retry, limit }) => {
            // A model asked again writes another word, and a part of this one ran, so the reply ends with the word
            // that ran, and the model is not asked again.
            let ran =
              replies.get(&id).and_then(|reply| reply.watch.as_ref()?.ran().map(str::to_owned));
            if let Some(Ok(turn)) = ran.as_deref().map(cut) {
              if let Some(Reply { chain, rung, watch, .. }) = replies.remove(&id) {
                mute.remove(&chain);
                if let Some(note) = watch.as_ref().and_then(Watch::note) {
                  told(&mut co, &chain, &format!("#{rung} {note}"));
                }
                noted(
                  &mut co,
                  &chain,
                  &rung,
                  STOPPED,
                  &[("why", &retry.why), ("ran", &ending(ran.as_deref().unwrap_or_default()))],
                )
                .await;
                say(&mut co, Fact::says("done", &id, [turn]));
              }
              continue;
            }
            if let Some(reply) = replies.get_mut(&id) {
              reply.waits = true;
              if let Some(watch) = reply.watch.as_mut() {
                watch.forget();
              }
            }
            if let (Some(retries), Some(reply)) = (&retries, replies.get(&id)) {
              retries(&reply.rung, &reply.chain, &retry);
            }
            // A wait for the end of a limit lasts hours, so the chain is told once, before it. A stand asks the call
            // again, and a call that meets the same wait tells nothing more.
            if let (true, Some(reply)) = (limit, replies.get_mut(&id)) {
              let mut note = format!("#{} waits on {}: {}", reply.chain, reply.actor, retry.why);
              if retry.why.starts_with("usage limit") {
                let chain = &reply.chain;
                note += &format!(
                  ". To go on with another actor, say rung(\"actor = '<model>'\", on=\"{chain}\") and wake(\"{chain}\")."
                );
              }
              if reply.noted.as_deref() != Some(note.as_str()) {
                say(
                  &mut co,
                  Fact::says("tell", &reply.chain, [Object::list([Object::string(&note)])]),
                );
                reply.noted = Some(note);
              }
            }
            continue;
          }
          // What a search came to: its text, or why it gives none, unless a control ended it first.
          Next::Worked(Replied::Searched { id, got }) => {
            if searches.remove(&id).is_some() {
              let got = got.map_or_else(|no| Fault::refused(no).object(), |one| one.object());
              say(&mut co, Fact::says("done", &id, [got]));
            }
            continue;
          }
          // What the model of a reply that still stands came to.
          Next::Worked(Replied::Came { id, got, dropped }) => {
            let Some(reply) = replies.get(&id) else { continue };
            // Once a part of the word ran, the turn is a word that holds the word that ran and that the gate finds
            // nothing against, so the chain runs it whole; any other end of the reply is the word that ran.
            let ran = reply.watch.as_ref().and_then(Watch::ran).map(str::to_owned);
            let got = match (ran, got) {
              (Some(ran), got) => {
                let (chain, rung) = (reply.chain.clone(), reply.rung.clone());
                let word = got
                  .as_ref()
                  .ok()
                  .and_then(|turn| Some(turn.as_ref().items()?.get(1)?.as_str()?.to_owned()));
                let why = match &word {
                  Some(word) if word.starts_with(ran.trim()) => {
                    let python = if word.contains(watch::QUOTE) {
                      let got = co.call("unquoted", vec![Object::string(word)], vec![]).await;
                      got.ok().and_then(|one| one.as_ref().as_str().map(str::to_owned))
                    } else {
                      Some(word.clone())
                    };
                    match python {
                      Some(python) => findings(&mut co, &chain, &python).await.join("\n"),
                      None => "the word holds a quote that is not whole".to_owned(),
                    }
                  }
                  Some(_) => "the turn does not hold the word that ran".to_owned(),
                  None => match &got {
                    Err(no) => format!("{STOPPED}: {}", failure(no)),
                    Ok(_) => "the turn holds no word".to_owned(),
                  },
                };
                if why.is_empty() {
                  got
                } else {
                  let Ok(turn) = cut(&ran) else { continue };
                  match &got {
                    Err(no) => {
                      noted(
                        &mut co,
                        &chain,
                        &rung,
                        STOPPED,
                        &[("why", &failure(no)), ("ran", &ending(&ran))],
                      )
                      .await
                    }
                    Ok(_) => {
                      let fields = [("findings", afterward(&why)), ("ran", ending(&ran))];
                      let fields = fields.each_ref().map(|(word, text)| (*word, text.as_str()));
                      noted(&mut co, &chain, &rung, AFTER, &fields).await
                    }
                  }
                  Ok(turn)
                }
              }
              (None, got) => got,
            };
            let Some(reply) = replies.get(&id) else { continue };
            // The word that runs is not all the text that the model wrote, so the chain is told what went, before it.
            let wrapped = reply.watch.as_ref().and_then(Watch::note);
            if let Some(dropped) = dropped.or(wrapped) {
              let note = format!("#{} {dropped}", reply.rung);
              say(
                &mut co,
                Fact::says("tell", &reply.chain, [Object::list([Object::string(&note)])]),
              );
            }
            let recoverable = got.as_ref().is_err_and(recoverable);
            let got =
              got.map_err(|no| format!("{} answered nothing: {}", reply.actor, failure(&no)));
            if let (Err(why), true) = (&got, reply.again)
              && !recoverable
            {
              let paused = co
                .call("pause", vec![Object::string(&reply.chain), Object::string(why)], vec![])
                .await;
              if let Err(no) = paused {
                told(
                  &mut co,
                  &reply.chain,
                  &format!("#{} could not pause the chain: {}", reply.rung, no.message()),
                );
              }
            }
            let got = got.map_err(Fault::refused).unwrap_or_else(|fault| fault.object());
            say(&mut co, Fact::says("done", &id, [got]));
            continue;
          }
        };
        let about = a.about().to_owned();
        match a.kind() {
          "stand" if a.question() => {
            // A stand that a word makes, as the stand of a setup, reads the standing alone: only a stand from outside
            // an act reads the settings again, so a setup or a new standing on many chains asks no command line.
            let word = crate::fact::named(a.by(), "rung");
            let first = !std::mem::replace(&mut stood, true);
            if let Some(refresh) = refresh.as_ref().filter(|_| !first && !word) {
              // Reading the settings asks each program of a model, which can take seconds, so the
              // stand reads them on the runtime of the ears, and every chain of the life goes on
              // meanwhile.
              say(&mut co, Fact::says("started", &about, []));
              let refresh = Arc::clone(refresh);
              let read = runtime().spawn_blocking(move || refresh());
              co.work(stream::once(read.map(move |got| {
                let got = got.map_err(|no| no.to_string()).and_then(|got| got);
                Replied::Refreshed { about, got }
              })));
              continue;
            }
            if !gone.is_empty() && lacked.insert(a.on().to_owned()) {
              told(&mut co, a.on(), &departed(&about, &gone));
            }
            stands = Some((about, word));
          }
          "reply" if a.question() => {
            say(&mut co, Fact::says("started", &about, []));
            let chain = a.on().to_owned();
            let actor = a.word(1).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
            let (watch, stop, hold) = asking(
              &mut co,
              &about,
              &chain,
              &actor,
              &life,
              &models,
              &mut images,
              thinking,
              backoff,
              false,
            )
            .await?;
            let again = mute.get(&chain) == Some(&actor);
            let rung = a.by().to_owned();
            let watch = watch.then(Watch::default);
            let reply = Reply {
              chain,
              actor,
              rung,
              again,
              watch,
              waits: false,
              noted: None,
              hold,
              _stop: stop,
            };
            replies.insert(about, reply);
          }
          // A part of a word that raised ends the word where it raised, so its reply ends with the word that ran,
          // whose run is done with that raise.
          RAN => {
            if a.word(1).is_none_or(|one| one.type_name() == "NoneType") {
              continue;
            }
            let mine = replies.iter().find(|(_, reply)| reply.rung == about);
            let Some(id) = mine.map(|(id, _)| id.clone()) else { continue };
            let ran =
              replies.get(&id).and_then(|reply| reply.watch.as_ref()?.ran().map(str::to_owned));
            let Some(Ok(turn)) = ran.as_deref().map(cut) else { continue };
            if let Some(Reply { chain, rung, watch, .. }) = replies.remove(&id) {
              mute.remove(&chain);
              if let Some(note) = watch.as_ref().and_then(Watch::note) {
                told(&mut co, &chain, &format!("#{rung} {note}"));
              }
              say(&mut co, Fact::says("done", &id, [turn]));
            }
          }
          "done" => {
            searches.remove(&about);
            let Some(Reply { chain, actor, .. }) = replies.remove(&about) else { continue };
            // A done that the ear said of its own reply ends the row of refusals on the chain with a turn, or adds
            // to it with a refusal, which is what the ear says when it says no turn; a done that a control said
            // leaves the row as it is.
            let turned = a
              .word(0)
              .and_then(|one| one.items())
              .is_some_and(|turn| turn.len() == 4 && turn[0].as_str() == Some("assistant"));
            if a.by() == about {
              continue;
            }
            if turned {
              mute.remove(&chain);
            } else {
              mute.insert(chain, actor);
            }
          }
          // A pause or a wake changes what holds a reply, so each reply that stands reads whether a pause holds it
          // now, and a call that waits to be made again goes on at the wake.
          // A search of the web: the native search of the provider of its actor, else the free
          // search, as a work of the ear, so every chain of the life goes on meanwhile. A life
          // that inspects searches nothing.
          "search" if a.question() => {
            let word = |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
            let (query, actor) = (word(1).unwrap_or_default(), word(2).unwrap_or_default());
            if !searching {
              let no = "a life that inspects searches nothing";
              say(&mut co, Fact::says("done", &about, [Fault::refused(no).object()]));
              continue;
            }
            say(&mut co, Fact::says("started", &about, []));
            let search = native(&actor).unwrap_or_else(free);
            let (stop, stopped) = oneshot::channel();
            let id = about.clone();
            let work = async move {
              let got = search(query.clone()).await;
              Replied::Searched {
                id,
                got: got.map(|answer| crate::value::Text::new(query, answer)),
              }
            };
            co.work(stream::once(work).take_until(stopped));
            searches.insert(about, stop);
          }
          "pause" | "wake" => {
            let ids: Vec<String> = replies.keys().cloned().collect();
            for id in ids {
              let held = co.call("paused", vec![Object::string(&id)], vec![]).await;
              let held = held.ok().and_then(|one| one.as_ref().as_bool()).unwrap_or(false);
              if let Some(reply) = replies.get(&id) {
                reply.hold.send_replace(held);
              }
            }
          }
          _ => {}
        }
      }
    })
  }
}

/// A reply the ear took: its chain, its actor, the rung it answers, whether a prior refusal of it
/// pauses its chain, the watch over its word while its model writes it, when the model streams its
/// word, and the stop of its work, which ends the call of its model, and what the host is told of
/// it, when the reply goes.
struct Reply {
  chain: String,
  actor: String,
  rung: String,
  again: bool,
  watch: Option<Watch>,
  waits: bool,
  /// The last note that told the chain of a wait, so the same wait asked again after a stand is told once.
  noted: Option<String>,
  /// Whether a pause holds the reply, which holds the next call of its model: a call that runs returns, and a call
  /// that waits to be made again waits for the wake.
  hold: holding::Sender<bool>,
  _stop: oneshot::Sender<()>,
}

/// The call of the model of a reply, begun on the turns of its chain and the models that stand now, as a work of the
/// ear; whether its model streams its word, which the ear reads as it comes; and what ends the call once dropped.
#[expect(clippy::too_many_arguments, reason = "debt: more than 4 parameters is a struct")]
async fn asking(
  co: &mut Co<Replied>,
  about: &str,
  chain: &str,
  actor: &str,
  life: &str,
  models: &RwLock<Vec<Model>>,
  images: &mut Images,
  thinking: Thinking,
  backoff: Backoff,
  retry: bool,
) -> Result<(bool, oneshot::Sender<()>, holding::Sender<bool>), Fault> {
  let turns = co.call("turns", vec![], vec![("on", Object::string(chain))]).await?;
  let (tells, told) = mpsc::unbounded_channel();
  let wrote = written(about, &tells);
  let key = format!("{life}/{chain}");
  let asked = {
    let held = models.read().unwrap_or_else(PoisonError::into_inner);
    requested(&held, actor, chain, turns.as_ref(), &key, images, wrote)
      .map(|one| thinks(one, thinking))
  };
  let streams = asked.as_ref().is_ok_and(|(model, _)| model.streams_its_word());
  let paused = co.call("paused", vec![Object::string(about)], vec![]).await;
  let paused = paused.ok().and_then(|one| one.as_ref().as_bool()).unwrap_or(false);
  let (hold, holds) = holding::channel(paused);
  let (stop, stopped) = oneshot::channel();
  co.work(answered(about.to_owned(), asked, backoff, tells, told, stopped, holds, retry));
  Ok((streams, stop, hold))
}

/// The wait of a call of a model to be made again while a pause holds its reply, which the wake ends. A reply that
/// went has no pause to wait for, so its call ends.
async fn released(hold: &mut holding::Receiver<bool>) -> Result<(), CompletionError> {
  hold.wait_for(|paused| !*paused).await.map(drop).map_err(reply_gone)
}

/// The failure of a call of a model whose reply went: no one reads what it would give.
fn reply_gone(why: impl std::fmt::Display) -> CompletionError {
  CompletionError::ProviderError(format!(
    "the reply went before its call of the model ended: {why}"
  ))
}

/// What the work of a reply gives: a part that its model wrote, a retry of the call of its model, or what the reply
/// came to, and what the chain is told when its turn lost marks of its text: the marks of a fence, of a tool call or of
/// a break of HTML, and the name that a binding of no name lacked, which [`watch::bare`] drops and gives, or the line
/// of three backticks at the end of its word, which [`unclosed`] drops.
enum Replied {
  Wrote { id: String, text: String, thinking: String },
  Retried { id: String, retry: Retry, limit: bool },
  Came { id: String, got: Result<Object, CompletionError>, dropped: Option<String> },
  Refreshed { about: String, got: Result<Settings, String> },
  Searched { id: String, got: Result<crate::value::Text, String> },
}

/// What the chain is told after the rung of a turn whose model wrote a fault after the part of its word that ran.
const AFTER: &str = "ran a part of its word, whose last lines its ran shows, then wrote a statement that the chain \
  would refuse, so its turn is the word that ran: each act, close and name of that part stands, and no later statement \
  runs";

/// How many lines of the end of the word that ran a note shows.
const ENDING: usize = 3;

/// The last lines of the word that ran, each with its number, which a note shows so that a model knows where its run
/// stopped.
fn ending(ran: &str) -> String {
  let lines: Vec<(usize, &str)> =
    ran.trim_end().lines().enumerate().filter(|(_, line)| !line.trim().is_empty()).collect();
  let shown = lines[lines.len().saturating_sub(ENDING)..].iter();
  shown.map(|(at, line)| format!("line {}: {line}\n", at + 1)).collect()
}

/// The findings of the gate on a word whose part ran. The gate says of a word that is not python that nothing of it
/// ran, which holds only while no part of it ran.
fn afterward(findings: &str) -> String {
  findings.replace("nothing of this word ran", "no statement after the part that ran runs")
}

/// What the chain is told after the rung of a reply whose call ended after a part of its word ran.
const STOPPED: &str =
  "is the word that ran, since the call of its model ended after a part of it ran";

/// What the gate finds against a word, as python, on a chain, and nothing when the gate gives no answer.
async fn findings(co: &mut Co<Replied>, chain: &str, word: &str) -> Vec<String> {
  let words = vec![Object::string(chain), Object::string(word)];
  let found = co.call("against", words, vec![]).await.ok();
  let found = found.and_then(|one| {
    let items = one.as_ref().items()?;
    Some(items.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
  });
  found.unwrap_or_default()
}

/// The kind of the fact that the Kernel says, as a rung, once the part of a writing of that rung is over:
/// `("ran", rung, rung, before, raised)`, with how many characters of the word stand before the part.
const RAN: &str = "ran";

/// The part of the word of a rung that is new, said as a writing of that rung, `("writing", rung, by, part,
/// before)`, which the Kernel hears and runs as that rung; and false when a pause holds the rung, since no part of a
/// word runs while a pause holds its chain. The writing is a fact that the ear says, and no act that it awaits, so
/// the ear stands at its next yield when the Kernel runs the part, and takes each reply that the part makes a chain
/// ask, and the record keeps the writing, so a later life runs the part where this one ran it.
async fn writing(co: &mut Co<Replied>, rung: &str, part: &str, before: usize) -> bool {
  let paused = co.call("paused", vec![Object::string(rung)], vec![]).await;
  if paused.ok().and_then(|one| one.as_ref().as_bool()) != Some(false) {
    return false;
  }
  let before = Object::int(i64::try_from(before).unwrap_or(i64::MAX));
  say(co, Fact::says("writing", rung, [Object::string(part), before]));
  true
}

/// A note told to a chain.
fn told(co: &mut Co<Replied>, chain: &str, note: &str) {
  say(co, Fact::says("tell", chain, [Object::list([Object::string(note)])]));
}

/// The note that tells a chain which names of the roster and of the actor left the catalog, as a
/// stand names it.
fn departed(about: &str, names: &[String]) -> String {
  let quote = |name: &String| format!("'{}'", name.replace('\\', "\\\\").replace('\'', "\\'"));
  let list = names.iter().map(quote).collect::<Vec<_>>().join(", ");
  format!("#{about} names left the catalog\n{about}_left = [{list}]")
}

/// A note of the chain about the turn of a rung: its header, then each text that the gate or the call gave, which the
/// engine binds under the rung as it binds a value of an act, so the turn that holds the note stays python. A text after the
/// header as a line of its own is no python, and the run of the turn would raise a SyntaxError.
async fn noted(co: &mut Co<Replied>, chain: &str, rung: &str, what: &str, fields: &[(&str, &str)]) {
  let words = vec![Object::string(rung), Object::string(what)];
  let fields = fields.iter().map(|(word, text)| (*word, Object::string(*text))).collect();
  let got = co.call("headed", words, fields).await;
  let note = got.ok().and_then(|one| one.as_ref().as_str().map(str::to_owned));
  told(co, chain, &note.unwrap_or_else(|| format!("#{rung} {what}")));
}

/// What a turn tells as it streams, sent to the work of its reply, which tells the host and reads the word.
fn written(id: &str, tells: &mpsc::UnboundedSender<Replied>) -> Told {
  let (id, tells) = (id.to_owned(), tells.clone());
  Arc::new(move |text, thinking| {
    let (text, thinking) = (text.to_owned(), thinking.to_owned());
    // A reply that went has no reader for what its model still writes.
    drop(tells.send(Replied::Wrote { id: id.clone(), text, thinking }));
  })
}

/// What the chains stand on: the models and the operator, the directory, the default actor, and the
/// path of the doctrine that a chain reads, which [`catalog::doctrine`] gives, each path as
/// [`super::files::tilded`] writes it, then the extensions that the engine seeds the root with,
/// each as `[name, file, life word, root alone]`. A standing with no doctrine, or of a provider
/// that advertises no extensions, gives none, and the engine seeds no root then. Each model of the
/// roster gives its class, and a model of the list of automatic work its place in that list, which
/// the configs of the user, in the config directory, and of the project give, as [`configured`]
/// reads them.
fn standing(
  models: &[Model],
  directory: &str,
  actor: &str,
  extensions: Option<&[Advertised]>,
  user: &Path,
) -> Object {
  let given = configured(user, Path::new(directory));
  let offered = models.iter().map(|model| {
    let efforts = model.efforts.iter().map(|(effort, _)| Object::string(effort));
    let window = i64::try_from(model.window).unwrap_or(i64::MAX);
    let class = given.classes.get(&model.name).copied().unwrap_or_else(|| model.class());
    let place = given.automatic.iter().position(|one| *one == model.name);
    let place = place.and_then(|at| i64::try_from(at).ok()).map(Object::int);
    let fields = [Object::string(&model.name), Object::list(efforts), Object::int(window)];
    Object::list(fields.into_iter().chain([Object::string(class.name())]).chain(place))
  });
  // The operator answers in person, with no effort, reads the window of the engine, and costs no dollar.
  let operator = Object::list([
    Object::string(OPERATOR),
    Object::list([]),
    Object::int(WINDOW),
    Object::string(Class::Cheap.name()),
  ]);
  let roster = Object::list(offered.chain([operator]));
  let home = std::env::home_dir();
  let shown = |one: &Path| Object::string(super::files::tilded(one, home.as_deref()));
  let doctrine = catalog::doctrine(Path::new(directory)).map(|one| shown(&one));
  let given = |all: &[Advertised]| {
    Object::list(all.iter().map(|(name, file, life, root)| {
      let texts = [name, file, life].map(Object::string);
      Object::list(texts.into_iter().chain([Object::bool(*root)]))
    }))
  };
  let advertised = extensions.filter(|_| doctrine.is_some()).map(given);
  Object::list(
    [roster, shown(Path::new(directory)), Object::string(actor)]
      .into_iter()
      .chain(doctrine)
      .chain(advertised),
  )
}

/// The model an actor names, and what a reply asks it: the engine as the system prompt, the turns of the chain,
/// the settings of the effort, the conversation where the model takes one, and whom it tells what it writes.
fn requested(
  models: &[Model],
  actor: &str,
  chain: &str,
  turns: ObjectRef<'_>,
  conversation: &str,
  images: &mut Images,
  told: Told,
) -> Result<(Model, Asked), CompletionError> {
  let found =
    models.iter().find_map(|model| model.settings(actor).map(|settings| (model, settings)));
  let Some((model, mut settings)) = found else {
    return Err(CompletionError::ProviderError(format!("No model is {actor}.")));
  };
  if let Some(key) = &model.conversation {
    settings.insert(key.clone(), Value::String(conversation.to_owned()));
  }
  let request = CompletionRequest {
    model: None,
    preamble: Some(SYSTEM.to_owned()),
    chat_history: messages(turns, model, images)?,
    documents: Vec::new(),
    tools: Vec::new(),
    temperature: None,
    max_tokens: model.tokens,
    tool_choice: None,
    additional_params: (!settings.is_empty()).then_some(Value::Object(settings)),
    output_schema: None,
    record_telemetry_content: false,
  };
  let asked =
    Asked { actor: actor.to_owned(), chain: chain.to_owned(), request, told, thought: None };
  Ok((model.clone(), asked))
}

/// The most tokens of thought of one turn at an effort of its actor, for a model whose thought the provider cuts: an
/// eighth of the most at `minimal` or `low`, a quarter at `medium`, and all of it at `high`, above it or with no effort,
/// since a model that thinks to the most of `high` at each level spends minutes on a turn that a lower level means to
/// keep short. A share is one token at least.
pub(super) fn budgeted(most: u64, effort: Option<&str>) -> u64 {
  match effort {
    Some("minimal" | "low") => (most / 8).max(1),
    Some("medium") => (most / 4).max(1),
    _ => most,
  }
}

/// What a reply asks a model, with the most tokens of thought of the turn as the model takes it, at the effort of its
/// actor as [`budgeted`] says for a model whose thought the provider cuts.
fn thinks((model, mut asked): (Model, Asked), thinking: Thinking) -> (Model, Asked) {
  let mut settings = match asked.request.additional_params.take() {
    Some(Value::Object(settings)) => settings,
    _ => Map::new(),
  };
  let effort =
    asked.actor.strip_prefix(model.name.as_str()).and_then(|rest| rest.strip_prefix('/'));
  asked.thought = match &model.thought {
    Some(Thought::Off(off)) if thinking.most == 0 => {
      merged(&mut settings, off);
      None
    }
    Some(Thought::Off(off)) => Some((budgeted(thinking.most, effort), off.clone())),
    Some(Thought::Told) if !thinking.inherited => {
      settings.insert(KEY_THINKING.to_owned(), Value::from(thinking.most));
      None
    }
    _ => None,
  };
  asked.request.additional_params = (!settings.is_empty()).then_some(Value::Object(settings));
  (model, asked)
}

/// The id of a thought that the provider cut at the most of its turn, or that ended with no word, and gave back to its
/// model to write its word with no more thought.
const CUT: &str = "cut";

/// The most characters of a thought that the provider cut that a model that forgets reads again in its next turn.
pub(super) const TAIL: usize = 16_384;

/// The blocks of a turn of a model that forgets its thought, less that thought: but for the last turn, which keeps the
/// end of a thought that the provider cut, [`TAIL`] characters at most, since the model wrote a word from that thought
/// alone, and with no thought of it in the next turn, it thinks again to the most of that turn.
fn forgotten(blocks: Vec<AssistantContent>, latest: bool) -> Vec<AssistantContent> {
  blocks
    .into_iter()
    .filter_map(|one| match one {
      AssistantContent::Reasoning(thought) if latest && thought.id.as_deref() == Some(CUT) => {
        let text = thought.display_text();
        Some(AssistantContent::Reasoning(Reasoning::new(tail(&text, TAIL)).with_id(CUT.to_owned())))
      }
      AssistantContent::Reasoning(_) => None,
      other => Some(other),
    })
    .collect()
}

/// The last characters of a text, this many at most.
pub(super) fn tail(text: &str, most: usize) -> &str {
  let skip = text.chars().count().saturating_sub(most);
  text.char_indices().nth(skip).map_or("", |(at, _)| &text[at..])
}

/// The turns of a chain as messages of rig. The engine phrases every turn as python and the provider renders
/// nothing: a user turn goes as its python and the images that the prompts it opens name, each after the text that
/// names it and once in the request, and one that holds nothing goes not at all; an assistant turn goes as the blocks
/// its provider gave, less its thought for a model that forgets it, as [`forgotten`] says, or as its python when a provider of another kind
/// gave them. A turn of the model whose word the gate refused goes on the user side, as a quote under the finding of
/// its rung, so the model reads it as a word that the chain refused and never as one that it would write again. A
/// model that takes no image refuses a turn that names one.
fn messages(
  turns: ObjectRef<'_>,
  model: &Model,
  images: &mut Images,
) -> Result<Vec<Message>, CompletionError> {
  let mut messages = Vec::new();
  // Whether the last turn of the model held nothing but marks, so the next user turn joins the one before it.
  let mut joins = false;
  // The images that an earlier turn gave, which no later turn gives again.
  let mut given = HashSet::new();
  // The word of the turn of the model that the gate refused, which the next user turn holds under its finding.
  let mut quoted: Option<(String, usize)> = None;
  let all = turns.items().unwrap_or_default();
  // The last turn that a model wrote, whose thought that the provider cut a model that forgets still reads. A word that
  // the chain placed in the turns, as a stand, holds no blocks, so it is no turn of the model.
  let last = all.iter().rposition(|turn| {
    entry(turn, 0).and_then(|one| one.as_str()) == Some("assistant")
      && entry(turn, 3).and_then(|one| one.items()).is_some()
  });
  for (at, turn) in all.iter().enumerate() {
    let Some(role) = entry(turn, 0).and_then(|one| one.as_str()) else { continue };
    let python = entry(turn, 1).and_then(|one| one.as_str()).unwrap_or_default();
    if role == "assistant" {
      let next = all.get(at + 1).and_then(|one| entry(one, 1)?.as_str().map(str::to_owned));
      if let Some(place) = next.as_deref().and_then(refusal) {
        // The quote holds the word less the marks of a fence or of a tool call, which the model would write again.
        let word = watch::bare(python).map_or_else(|| python.to_owned(), |(word, _)| word);
        quoted = (!word.trim().is_empty()).then_some((word, place));
        joins = true;
        continue;
      }
      let latest = Some(at) == last;
      let blocks = entry(turn, 3)
        .and_then(|one| serde_json::from_value::<Vec<AssistantContent>>(wire::record(one)).ok())
        .map(|blocks| if model.forgets { forgotten(blocks, latest) } else { blocks })
        .filter(|blocks| !blocks.is_empty())
        .unwrap_or_else(|| vec![AssistantContent::text(python)]);
      let (blocks, bared) = unmarked(blocks);
      joins = bared && !blocks.iter().any(|one| matches!(one, AssistantContent::Text(_)));
      if !joins {
        messages.push(Message::Assistant { id: None, content: blocks });
      }
      continue;
    }
    let held = quoted.take().map(|(word, place)| refused(python, place, &word));
    let python = held.as_deref().unwrap_or(python);
    if python.is_empty() {
      continue;
    }
    let seen = images.named(python, &mut given).map_err(CompletionError::ProviderError)?;
    if !seen.is_empty() && !model.images {
      return Err(CompletionError::ProviderError(format!(
        "{} does not accept images.",
        model.name
      )));
    }
    if std::mem::take(&mut joins)
      && let Some(Message::User { content }) = messages.last_mut()
      && let Some(UserContent::Text(was)) =
        content.iter_mut().rev().find(|one| matches!(one, UserContent::Text(_)))
    {
      was.text = format!("{}\n\n{python}", was.text);
      content.extend(seen);
      continue;
    }
    // The open of a chain after the setup that it holds goes as a block of its own, so the setup that every chain
    // with no source shares ends a block, which a provider can keep in its cache.
    let texts = match banner(python) {
      Some(at) => vec![&python[..at - 2], &python[at..]],
      None => vec![python],
    };
    let content = texts.into_iter().map(|one| UserContent::Text(Text::new(one))).chain(seen);
    messages.push(Message::User { content: content.collect() });
  }
  Ok(messages)
}

/// Where the open of a chain starts in a user turn, as `#chain3` alone on the line that starts a paragraph after
/// another, which the setup of the chain stands before.
pub(crate) fn banner(python: &str) -> Option<usize> {
  python.match_indices("\n\n#chain").map(|(at, _)| at + 2).find(|&at| {
    let rest = &python[at + "#chain".len()..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && rest[digits..].starts_with('\n')
  })
}

/// Where the finding of a refused rung ends in a user turn that holds one, `#rungN refused` at the start of a
/// paragraph with its findings after it: the end of the line of its findings, or of the quote that holds them.
fn refusal(python: &str) -> Option<usize> {
  let mut from = 0;
  let head = loop {
    let at = from + python[from..].find(" refused\n")?;
    let start = python[..at].rfind('\n').map_or(0, |one| one + 1);
    let name = python[start..at].strip_prefix('#').unwrap_or_default();
    if crate::fact::named(name, "rung") && (start == 0 || python[..start].ends_with("\n\n")) {
      break (name, at + " refused\n".len());
    }
    from = at + 1;
  };
  let (name, body) = head;
  let rest = &python[body..];
  let line = |from: usize| rest[from..].find('\n').map_or(rest.len(), |one| from + one);
  let end = if rest.starts_with(&format!("{name}_findings = ")) {
    line(0)
  } else {
    let tag = rest.strip_prefix("<s:")?.split_once('>')?.0;
    line(rest.find(&format!("</s:{tag}>"))?)
  };
  Some(body + end)
}

/// A user turn with the word that the gate refused quoted under the finding that ends at this place, under the name
/// `rungN_word` that a rung tells its word by, so the model reads it as the word of that rung.
fn refused(python: &str, place: usize, word: &str) -> String {
  let start = python[..place].rfind("\n\n").map_or(0, |one| one + 2);
  let rung = python[start + 1..].split(' ').next().unwrap_or_default();
  let mut name = format!("{rung}_word");
  while format!("{word}\n").contains(&format!("</s:{name}>\n")) {
    name.push('_');
  }
  let word = word.trim_end_matches('\n');
  format!("{}\n<s:{name}>\n{word}\n</s:{name}>{}", &python[..place], &python[place..])
}

/// The blocks of a turn of a model less the marks of a fence or of a tool call in each text, which [`watch::bare`]
/// drops, with each text that held only marks gone, and whether a mark went. A turn of a record that an older
/// provider made can hold them, as a word that the gate refused for them, and the model reads the finding of the
/// gate in the next turn, and never the marks, which it would write again.
fn unmarked(blocks: Vec<AssistantContent>) -> (Vec<AssistantContent>, bool) {
  let mut bared = false;
  let blocks = blocks
    .into_iter()
    .filter_map(|one| match one {
      AssistantContent::Text(text) => match watch::bare(&text.text) {
        Some((word, _)) => {
          bared = true;
          (!word.trim().is_empty()).then(|| AssistantContent::text(word))
        }
        None => Some(AssistantContent::Text(text)),
      },
      other => Some(other),
    })
    .collect();
  (blocks, bared)
}

/// One response of a model of rig, streamed: what it writes is told as it comes, and the response holds it whole,
/// with the raw record of its end, where a model says what the turn cost. A model whose thought passes its budget is
/// asked again, as [`rethought`] says, and so is a model with a budget whose turn ended with a thought and no word, so
/// the chain never reads an empty word from it. A turn that the most it writes ended is not asked again.
async fn streamed(streams: Streams, asked: Asked) -> Result<CompletionResponse, CompletionError> {
  let Asked { request, told, thought: budget, .. } = asked;
  let mut stream = streams(request.clone()).await?;
  // A thought whose parts streamed is told already, and a thought that came whole is told as it comes.
  let mut thought = HashSet::new();
  let (mut thinking, mut parts) = (String::new(), 0);
  while let Some(part) = stream.next().await {
    match part? {
      StreamedAssistantContent::Text(text) => told(&text.text, ""),
      StreamedAssistantContent::ReasoningDelta { id, reasoning, .. } => {
        told("", &reasoning);
        thought.insert(id);
        thinking.push_str(&reasoning);
        parts += 1;
        if let Some((most, again)) = &budget
          && parts > *most
        {
          // The call of this stream ends here.
          drop(stream);
          return rethought(&streams, request, again, thinking, parts, &told).await;
        }
      }
      StreamedAssistantContent::Reasoning { reasoning, id } if !thought.contains(&id) => {
        told("", &reasoning.display_text());
      }
      _ => {}
    }
  }
  let response = whole(stream);
  let Some((_, again)) = &budget else { return Ok(response) };
  let blocks = untagged(response.choice.to_vec());
  if thinking.is_empty() {
    thinking = blocks
      .iter()
      .filter_map(|one| match one {
        AssistantContent::Reasoning(one) => Some(one.display_text()),
        _ => None,
      })
      .collect();
  }
  let ended = !matches!(response.finish_reason(), None | Some(FinishReason::Length));
  if !ended || thinking.trim().is_empty() || !texts(&blocks).trim().is_empty() {
    return Ok(response);
  }
  let parts = response.usage.output_tokens.max(parts);
  rethought(&streams, request, again, thinking, parts, &told).await
}

/// The response of a stream that ended, with the raw record of its end. A model that says each block whole once it
/// streamed it, as the claude command line does, gives those blocks in that record as `settled`, and they are the
/// choice of the response, since a part of the stream may be lost where the whole block holds it.
fn whole(stream: StreamingCompletionResponse) -> CompletionResponse {
  let raw = stream.response.as_ref().map(|end| end.raw.clone()).unwrap_or_default();
  let settled = raw.get("settled").and_then(|one| serde_json::from_value(one.clone()).ok());
  let mut response = CompletionResponse::from(stream).with_raw(raw);
  if let Some(settled) = settled {
    response.choice = settled;
  }
  response
}

/// The turn of a model whose thought passed its budget. The provider asks it again with the same turns, its thought
/// given back as a turn of its own that holds that thought and no word, and with the settings of its budget, which turn its
/// thought off, so it writes its word at once. The turn holds that thought, under the id [`CUT`], and the word, and
/// what the model wrote counts each part of that thought.
async fn rethought(
  streams: &Streams,
  mut request: CompletionRequest,
  again: &Map<String, Value>,
  thinking: String,
  parts: u64,
  told: &Told,
) -> Result<CompletionResponse, CompletionError> {
  let mut settings = match request.additional_params.take() {
    Some(Value::Object(settings)) => settings,
    _ => Map::new(),
  };
  merged(&mut settings, again);
  request.additional_params = (!settings.is_empty()).then_some(Value::Object(settings));
  let thought = AssistantContent::Reasoning(Reasoning::new(&thinking).with_id(CUT.to_owned()));
  // rig sends no turn that holds no text, so the turn of the thought holds an empty text too.
  let given = vec![thought.clone(), AssistantContent::text("")];
  request.chat_history.push(Message::Assistant { id: None, content: given });
  let mut stream = streams(request).await?;
  while let Some(part) = stream.next().await {
    match part? {
      StreamedAssistantContent::Text(text) => told(&text.text, ""),
      StreamedAssistantContent::ReasoningDelta { reasoning, .. } => told("", &reasoning),
      _ => {}
    }
  }
  let mut response = whole(stream);
  let word = texts(&response.choice);
  response.choice = vec![thought, AssistantContent::text(word)];
  response.usage.output_tokens += parts;
  response.usage.total_tokens += parts;
  Ok(response)
}

/// A text less the marks that wrap it: an opening line of three backticks, alone or with `python`, at its start, and
/// a closing line of three backticks at its end when it has one; or a tag of markup alone on its first line, as
/// `<answer>` or a bare `<`, and each line that is that tag alone, as it opens or closes, since a model of a local
/// server wraps each part of its word in it. A text that no such mark opens gets nothing, so a text before the mark
/// keeps the text as it is, and the mark of a quote is no tag.
fn unfenced(text: &str) -> Option<String> {
  let text = text.trim();
  let (opening, rest) = text.split_once('\n').unwrap_or((text, ""));
  let (opening, rest) = (opening.trim_end(), rest.trim_end());
  if matches!(opening, "```" | "```python") {
    let closed =
      rest.strip_suffix("```").filter(|inside| inside.is_empty() || inside.ends_with('\n'));
    return Some(closed.unwrap_or(rest).trim_end().to_owned());
  }
  let name = opening.strip_prefix('<')?;
  let name = name.strip_suffix('>').unwrap_or(name);
  if !name.chars().all(|one| one.is_ascii_alphanumeric() || one == '_') {
    return None;
  }
  let tag = |line: &str| {
    let line = line.trim();
    let bare = line.strip_prefix("</").or_else(|| line.strip_prefix('<'));
    bare.is_some_and(|inside| inside.strip_suffix('>').unwrap_or(inside) == name)
  };
  let kept: Vec<&str> = rest.lines().filter(|line| !tag(line)).collect();
  Some(kept.join("\n").trim_end().to_owned())
}

/// What the chain is told after the rung of a turn whose word lost the line of three backticks at its end.
const UNCLOSED: &str = "wrote a line of three backticks alone at the end of its word, and the provider dropped that \
  line: a reply is python and quotes only, with no fence";

/// A word less the line of three backticks alone at its end, which a model writes out of the habit of a fence that
/// no line opened. A word gets nothing when its last line is not that fence, when its first line opens a fence, which
/// the gate refuses whole, when a quote stands open before that line, whose text the line may end, or when nothing
/// stands before that line.
fn unclosed(text: &str) -> Option<String> {
  let (word, last) = text.trim_end().rsplit_once('\n')?;
  let opens = word.trim_start().starts_with("```");
  let fence = last.trim() == "```" && !opens && !watch::open(word) && !word.trim().is_empty();
  fence.then(|| word.trim_end().to_owned())
}

/// The terminal record of a stream of a provider, from a response of it that came whole.
fn ended(provider: &str, response: CompletionResponse) -> StreamFinal {
  let mut record = StreamFinal::new(provider, response.usage);
  record.finish_reason = response.finish_reason();
  record.response_id = response.response_id;
  record.model = response.model;
  record.raw = response.raw;
  record
}

/// The work of a reply: each part its model writes, each retry of the call of its model, then the turn of the model
/// or why it answered nothing, in the order they came; and nothing more once the reply went, which ends the call of
/// the model. A call that failed for a cause that passes is made again after the wait of its backoff, until its
/// attempts are spent. A call that a usage limit refused is made again at the reset that the limit names, however
/// long that takes, and each such wait is no attempt. A call that a failed sign in refused is made again every minute
/// until it succeeds or a stand asks it at once, since a sign in outside furb ends the failure.
#[expect(clippy::too_many_arguments, reason = "debt: more than 4 parameters is a struct")]
fn answered(
  id: String,
  asked: Result<(Model, Asked), CompletionError>,
  backoff: Backoff,
  tells: mpsc::UnboundedSender<Replied>,
  mut told: mpsc::UnboundedReceiver<Replied>,
  stopped: oneshot::Receiver<()>,
  mut hold: holding::Receiver<bool>,
  retry: bool,
) -> impl Stream<Item = Replied> {
  let came = async move {
    let got = async {
      let (model, asked) = asked?;
      let mut attempt = 1;
      let mut limited = false;
      let mut signed = false;
      if retry {
        released(&mut hold).await?;
      }
      loop {
        let no = match (model.answers)(asked.clone()).await {
          Ok(Answer::Response(response)) => return turn(&model, &response),
          Ok(Answer::Turn(turn)) => return hosted(turn.as_ref()).map(|one| (one, None)),
          Err(no) => no,
        };
        if let Some(claude::Error::Limit { until }) = claude::error(&no) {
          let now = Zoned::now();
          let reset = until
            .and_then(|at| Timestamp::try_from(at).ok())
            .map(|at| at.to_zoned(now.time_zone().clone()));
          let wait = reset.as_ref().map_or(LIMIT_POLL, |at| {
            let seconds = at.timestamp().as_second() - now.timestamp().as_second();
            Duration::from_secs(u64::try_from(seconds).unwrap_or(0)) + LIMIT_SLACK
          });
          // The call that follows the end of a limit and meets it again waits a short time, not the next day.
          let wait = if limited { wait.min(LIMIT_POLL) } else { wait };
          let until =
            reset.map_or(String::new(), |at| at.strftime(" until %Y-%m-%d %H:%M (%Q)").to_string());
          let why = format!("usage limit{until}");
          let retry = Retry { attempt: attempt + 1, most: backoff.most, wait, why };
          tells
            .send(Replied::Retried { id: id.clone(), retry, limit: !limited })
            .map_err(reply_gone)?;
          limited = true;
          tokio::time::sleep(wait).await;
          released(&mut hold).await?;
          continue;
        }
        if let Some(claude::Error::SignedOut(_)) = claude::error(&no) {
          let why = "signed out, sign in again".to_owned();
          let retry = Retry { attempt: attempt + 1, most: backoff.most, wait: SIGN_POLL, why };
          tells
            .send(Replied::Retried { id: id.clone(), retry, limit: !signed })
            .map_err(reply_gone)?;
          signed = true;
          tokio::time::sleep(SIGN_POLL).await;
          released(&mut hold).await?;
          continue;
        }
        let Some(why) = passing(&no).filter(|_| attempt < backoff.most) else { return Err(no) };
        let why = why.to_string();
        attempt += 1;
        let wait = backoff.wait(attempt);
        let retry = Retry { attempt, most: backoff.most, wait, why };
        tells.send(Replied::Retried { id: id.clone(), retry, limit: false }).map_err(reply_gone)?;
        tokio::time::sleep(wait).await;
        released(&mut hold).await?;
      }
    };
    let (got, dropped) = match got.await {
      Ok((turn, dropped)) => (Ok(turn), dropped),
      Err(no) => (Err(no), None),
    };
    // The stream holds the receiver, so a send that fails means the stream is going.
    // The stream then gives the reply itself.
    tells.send(Replied::Came { id, got, dropped }).err().map(|no| no.0)
  };
  // The reply comes through the channel alone, so it comes after all that its model wrote before it.
  let told = stream::poll_fn(move |cx| told.poll_recv(cx));
  let came = stream::once(came).filter_map(std::future::ready);
  stream::select(told, came).take_until(stopped)
}

/// The turn of a model: its text, which is the word of the rung, what it read and wrote, and the blocks it gave.
///
/// A model speaks python alone, so prose around the code stays in the word, for the gate to refuse, but for the marks
/// of a fence, of a tool call or of a break of HTML around it, which [`watch::bare`] drops with the name that it gives
/// a first binding of no name, and a line of three backticks alone at its end,
/// which [`unclosed`] drops, since they hold no code that runs and a model writes them out of habit; a turn of a model that may wrap its word in a fence, as a model of a local server, loses
/// instead the fence that opens its text, and the closing fence at its end when it has one. What it read is every token of its input, the cache
/// among them, and what it wrote is the rest of its total, its thought among it. A model that says why each turn
/// ended, and says nothing of the end of this one or says that the most it writes ended it, was cut, so its turn is
/// refused, and no more of its word runs than the parts that ran as it wrote them, whose word the ear gives in place
/// of the refusal. The thought that a model writes in its text, before its word, is a block
/// of reasoning of its turn.
fn turn(
  model: &Model,
  response: &CompletionResponse,
) -> Result<(Object, Option<String>), CompletionError> {
  let cut = match response.finish_reason() {
    _ if !model.ends => None,
    None => Some(format!("the stream of {} stopped before its end", model.name)),
    Some(FinishReason::Length) => {
      Some(format!("the turn of {} reached the most it writes before its end", model.name))
    }
    Some(_) => None,
  };
  if let Some(why) = cut {
    return Err(CompletionError::ResponseError(format!("{why}: {}", wrote(model, response))));
  }
  let mut choice: Vec<AssistantContent> = response.choice.to_vec();
  if model.tagged {
    choice = untagged(choice);
  }
  let mut text = texts(&choice);
  let mut dropped = None;
  if model.fenced
    && let Some(word) = unfenced(&text)
  {
    (choice, text) = (worded(choice, &word), word);
  } else if let Some((word, note)) = watch::bare(&text) {
    (choice, text, dropped) = (worded(choice, &word), word, Some(note));
  } else if let Some(word) = unclosed(&text) {
    (choice, text, dropped) = (worded(choice, &word), word, Some(UNCLOSED.to_owned()));
  }
  let spent = response.usage;
  let (read, wrote) = (spent.cached_input_tokens, spent.cache_creation_input_tokens);
  let input = spent.input_tokens + if model.apart { read + wrote } else { 0 };
  let output =
    spent.total_tokens.checked_sub(input).filter(|one| *one > 0).unwrap_or(spent.output_tokens);
  let priced = model.price.map(|[fresh, out, cached, cache]| {
    let fresh = input.saturating_sub(read + wrote) as f64 * fresh;
    (fresh + output as f64 * out + read as f64 * cached + wrote as f64 * cache) / 1e6
  });
  let dollars = response.raw.get("cost").and_then(Value::as_f64).or(priced).unwrap_or_default();
  let failed = |why: String| CompletionError::ResponseError(why);
  let blocks = serde_json::to_value(&choice).map_err(|no| failed(no.to_string()))?;
  let blocks = wire::inward(&blocks).map_err(|fault| failed(fault.message()))?;
  let count = |n: u64| Object::int(i64::try_from(n).unwrap_or(i64::MAX));
  let usage =
    Object::tuple([count(input), count(output), count(read), count(wrote), Object::float(dollars)]);
  let turn =
    Object::tuple([Object::string("assistant"), Object::string(text.trim()), usage, blocks]);
  Ok((turn, dropped))
}

/// These blocks with the word as their one text, at the place of their first text.
fn worded(choice: Vec<AssistantContent>, word: &str) -> Vec<AssistantContent> {
  let mut worded = Vec::new();
  for one in choice {
    match one {
      AssistantContent::Text(_)
        if worded.iter().any(|one| matches!(one, AssistantContent::Text(_))) => {}
      AssistantContent::Text(_) => worded.push(AssistantContent::text(word)),
      other => worded.push(other),
    }
  }
  worded
}

/// The turn of a reply that ended at the first statement of its word that cannot be python: the word as far as that
/// statement, as the turn of a model that wrote nothing more. It counts nothing, since the usage of a stream comes
/// with its end.
fn cut(word: &str) -> Result<Object, CompletionError> {
  let word = word.trim();
  let failed = |why: String| CompletionError::ResponseError(why);
  let blocks =
    serde_json::to_value([AssistantContent::text(word)]).map_err(|no| failed(no.to_string()))?;
  let blocks = wire::inward(&blocks).map_err(|fault| failed(fault.message()))?;
  let zero = || Object::int(0);
  let usage = Object::tuple([zero(), zero(), zero(), zero(), Object::float(0.0)]);
  Ok(Object::tuple([Object::string("assistant"), Object::string(word), usage, blocks]))
}

/// How many characters of the end of a cut turn its refusal shows.
const END: usize = 400;

/// What a cut turn wrote, which its refusal says, since the record keeps the refusal and no part of the turn: the
/// tokens it wrote, the characters of its thought and of its text, and the end of the last of the two that it wrote. A
/// thought in the text whose close tag never came is thought.
fn wrote(model: &Model, response: &CompletionResponse) -> String {
  let mut choice: Vec<AssistantContent> = response.choice.to_vec();
  if model.tagged {
    choice = untagged(choice);
  }
  let mut thought: String = choice
    .iter()
    .filter_map(|one| match one {
      AssistantContent::Reasoning(one) => Some(one.display_text()),
      _ => None,
    })
    .collect();
  let mut text = texts(&choice);
  if model.tagged && text.trim_start().starts_with("<think>") && !text.contains("</think>") {
    thought.push_str(&text);
    text.clear();
  }
  let last: Vec<char> =
    if text.trim().is_empty() { &thought } else { &text }.trim_end().chars().collect();
  let end: String = last[last.len().saturating_sub(END)..].iter().collect();
  let (thought, text) = (thought.chars().count(), text.chars().count());
  let tokens = response.usage.output_tokens;
  format!(
    "it wrote {tokens} tokens, {thought} characters of thought and {text} of text, which end with {end:?}"
  )
}

/// The text of these blocks, whole.
fn texts(choice: &[AssistantContent]) -> String {
  let text = choice.iter().filter_map(|one| match one {
    AssistantContent::Text(text) => Some(text.text.as_str()),
    _ => None,
  });
  text.collect()
}

/// The blocks of a turn whose text starts with a thought between `<think>` and `</think>`, as a server with no reader
/// of thought writes it: at the place of its first text, the thought as a block of reasoning when it holds any, and the
/// text after it.
fn untagged(choice: Vec<AssistantContent>) -> Vec<AssistantContent> {
  let text = texts(&choice);
  let Some((thought, word)) =
    text.trim_start().strip_prefix("<think>").and_then(|rest| rest.split_once("</think>"))
  else {
    return choice;
  };
  let (thought, word) = (thought.trim().to_owned(), word.trim().to_owned());
  let mut blocks = Vec::new();
  for one in choice {
    match one {
      AssistantContent::Text(_)
        if blocks.iter().any(|one| matches!(one, AssistantContent::Text(_))) => {}
      AssistantContent::Text(_) => {
        if !thought.is_empty() {
          blocks.push(AssistantContent::Reasoning(Reasoning::new(&thought)));
        }
        blocks.push(AssistantContent::text(&word));
      }
      other => blocks.push(other),
    }
  }
  blocks
}

/// The turn that a function of the host gave, as a turn of a model: its role, its python, its usage, whose dollars
/// are a float however the host wrote them, and its blocks.
fn hosted(turn: ObjectRef<'_>) -> Result<Object, CompletionError> {
  let part = |at: usize| entry(&turn, at).map(|one| one.to_owned()).unwrap_or_else(Object::none);
  let (role, python) = (entry(&turn, 0), entry(&turn, 1));
  let whole = turn.items().is_some_and(|items| items.len() == 4)
    && role.and_then(|one| one.as_str()) == Some("assistant")
    && python.is_some_and(|one| one.as_str().is_some());
  if !whole {
    let why = format!(
      "a host answers with a turn: ('assistant', python, usage, blocks), not {}",
      turn.py_repr()
    );
    return Err(CompletionError::ResponseError(why));
  }
  let usage = entry(&turn, 2).and_then(|one| one.items()).map(|usage| {
    let each = usage.into_iter().enumerate().map(|(at, one)| match (at, one.as_int()) {
      (4, Some(whole)) => Object::float(whole as f64),
      _ => one.to_owned(),
    });
    Object::tuple(each)
  });
  Ok(Object::tuple([part(0), part(1), usage.unwrap_or_else(Object::none), part(3)]))
}

/// A new random id, a uuid of version 4, as the claude command line takes one for a conversation.
/// A machine that gives no random bytes fails the call that needs the id.
fn uuid() -> Result<String, CompletionError> {
  uuid_from(getrandom::fill)
}

/// The uuid that the bytes of `fill` make.
fn uuid_from(
  fill: impl FnOnce(&mut [u8]) -> Result<(), getrandom::Error>,
) -> Result<String, CompletionError> {
  let mut bytes = [0u8; 16];
  fill(&mut bytes).map_err(|no| {
    CompletionError::ProviderError(format!("the machine gave no random bytes: {no}"))
  })?;
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  let digits = crate::value::hex(&bytes);
  Ok(format!(
    "{}-{}-{}-{}-{}",
    &digits[..8],
    &digits[8..12],
    &digits[12..16],
    &digits[16..20],
    &digits[20..]
  ))
}

#[cfg(test)]
#[path = "provider.test.rs"]
mod test;
