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
  path::PathBuf,
  sync::{Arc, PoisonError, RwLock},
  time::Duration,
};

use futures::{Stream, StreamExt, future::BoxFuture, stream};
use jiff::{Timestamp, Zoned, civil, tz::TimeZone};
use rig_core::{
  completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse, FinishReason,
  },
  message::{AssistantContent, Message, Reasoning, Text, UserContent},
  streaming::{StreamFinal, StreamedAssistantContent, StreamingCompletionResponse},
};
use serde_json::{Map, Value, json};
use tokio::sync::mpsc;
use unsync::oneshot;

use self::{images::Images, search::Searches, watch::Watch};
use crate::{
  SYSTEM,
  ear::{Co, Ear, Next, hearing, say},
  engine::{OPERATOR, WINDOW},
  fact::Fact,
  value::{Fault, Object, ObjectRef, entry},
  wire,
};

/// The name of the environment that gives the most tokens of thought of a model in one turn, as Claude Code reads it.
pub(super) const ENV_THINKING: &str = "MAX_THINKING_TOKENS";

/// The key of `.furb/config.json` that gives the most tokens of thought of a model in one turn, and the setting under
/// which the provider tells it to the claude command line.
pub(super) const KEY_THINKING: &str = "max_thinking_tokens";

/// The most tokens of thought of a model in one turn when neither the environment nor the config gives one: a model
/// that thinks more than this rarely writes a word in the turn.
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
/// `max_thinking_tokens` of `.furb/config.json` in the directory of the life gives it, else [`THINKING`]. A value that
/// is no count of tokens is as unsaid.
pub(super) fn thinking(env: Option<&str>, directory: &std::path::Path) -> Thinking {
  let given = env.and_then(|one| one.trim().parse::<u64>().ok());
  let config = || {
    let text = std::fs::read_to_string(directory.join(".furb").join("config.json")).ok()?;
    serde_json::from_str::<Value>(&text).ok()?.get(KEY_THINKING)?.as_u64()
  };
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

/// Why a call of a model failed, in a few words, when the cause passes: the connection was lost or timed out, the
/// provider is overloaded, limits the rate, or failed on its side, or the program of the claude command line was
/// missing, as while an update replaces it. A failure of the request, of the turn, or of a model that the provider
/// cannot make does not pass, and the model is not asked again.
fn passing(no: &CompletionError) -> Option<String> {
  let told = match no {
    CompletionError::HttpError(no) => no.to_string(),
    CompletionError::ProviderError(why) => why.clone(),
    _ => return None,
  };
  let said = told.to_lowercase();
  let has = |words: &[&str]| words.iter().any(|one| said.contains(one));
  let status = said
    .split(|one: char| !one.is_ascii_digit())
    .find(|one| matches!(*one, "408" | "429" | "500" | "502" | "503" | "504" | "529"));
  let why = if said.starts_with(&claude::MISSING.to_lowercase()) {
    "the program is missing".to_owned()
  } else if said.starts_with(&claude::SWITCHED.to_lowercase()) {
    told
  } else if has(&["connection reset", "reset by peer"]) {
    "connection reset".to_owned()
  } else if has(&["timed out", "timeout"]) || status == Some("408") {
    "timed out".to_owned()
  } else if has(&["overloaded"]) || status == Some("529") {
    "the provider is overloaded".to_owned()
  } else if has(&["rate limit", "too many requests"]) || status == Some("429") {
    "rate limited".to_owned()
  } else if has(&["connection closed", "connection error", "broken pipe", "error sending request"])
  {
    "connection lost".to_owned()
  } else if let Some(code) = status {
    format!("server error {code}")
  } else if matches!(no, CompletionError::HttpError(_)) {
    "network error".to_owned()
  } else {
    return None;
  };
  Some(why)
}

/// The wait before the claude command line is asked again when its message gives no moment for the end of a limit.
const LIMIT_POLL: Duration = Duration::from_secs(300);

/// The time after the end of a limit that the provider waits, so the call is not made before the limit lifts.
const LIMIT_SLACK: Duration = Duration::from_secs(1);

/// Whether a failure is the usage limit of the claude command line, and then the moment it ends, if its message names
/// one. A limit passes at its reset, so the provider asks again then, however long that takes.
fn limit(no: &CompletionError, now: &Zoned) -> Option<Option<Zoned>> {
  let CompletionError::ProviderError(said) = no else { return None };
  let lower = said.to_lowercase();
  (lower.contains("usage limit reached") || (lower.contains("limit") && lower.contains("resets")))
    .then(|| resets(said, now))
}

/// The moment a usage limit ends, from the message of the claude command line: `resets 5:10pm (Europe/Paris)`, with
/// a day before the time, as `resets Oct 28, 3pm`, when it ends in a later day, a time with no zone being in the zone
/// of the machine, or the older `usage limit reached|1760000000`. A time with no day is the next one after now.
fn resets(said: &str, now: &Zoned) -> Option<Zoned> {
  if let Some((_, epoch)) = said.rsplit_once('|')
    && let Ok(second) = epoch.trim().parse::<i64>()
  {
    return Timestamp::from_second(second).ok().map(|at| at.to_zoned(now.time_zone().clone()));
  }
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
  /// Whether it reads its earlier turns with no thought, as a model of a local server, whose own template drops the
  /// thought of each earlier turn, and whose thought of each turn would else fill its window in a few turns.
  forgets: bool,
  answers: Answers,
  /// The native search of the web of its provider, which the extension web asks for an actor that names it.
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
/// of a model in one turn, the directory of the images that a turn names, and whom it tells what a model writes.
pub struct Provider {
  directory: String,
  models: Arc<RwLock<Vec<Model>>>,
  actor: Option<String>,
  thinking: Thinking,
  images: Option<PathBuf>,
  writes: Option<Writes>,
  backoff: Backoff,
  retries: Option<Retries>,
  refresh: Option<Refresh>,
}

type Refresh =
  Arc<dyn Fn() -> Result<(Vec<Model>, Option<String>, Thinking), String> + Send + Sync>;

impl Provider {
  /// A provider of these models, on a directory, whose model thinks [`THINKING`] tokens at most in one turn.
  pub fn new(directory: impl Into<String>, models: Vec<Model>) -> Provider {
    let directory = directory.into();
    Provider {
      directory,
      models: Arc::new(RwLock::new(models)),
      actor: None,
      thinking: Thinking::default(),
      images: None,
      writes: None,
      backoff: BACKOFF,
      retries: None,
      refresh: None,
    }
  }

  /// The most tokens of thought of a model in one turn.
  pub fn thinking(mut self, thinking: Thinking) -> Provider {
    self.thinking = thinking;
    self
  }

  pub(crate) fn refreshing(mut self, refresh: Refresh) -> Provider {
    self.refresh = Some(refresh);
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
  /// A reply reads the turns of its chain: the system prompt is the engine, a user turn goes as its python and the
  /// images that the prompts it opens name, and an assistant turn as the blocks its provider gave. It is done with
  /// the turn of the model, or with a refusal, and a second refusal in a row of the same actor on a chain pauses the
  /// chain first. A turn whose word lost the marks of a fence or of a tool call around it, or the line of three
  /// backticks at its end, tells its chain so once, before its done. A
  /// done of a reply that the ear did not say ends the call of its model.
  ///
  /// The ear says each part of a word that the model finished and that the gate finds nothing against as a writing of
  /// its rung, unless a pause holds the rung. A reply whose part ran is done with the word that ran when the Kernel
  /// says that the part ran with a raise, when a later statement holds a fault, and when its call fails, which is
  /// then made no more; and with its turn when the gate finds nothing against the whole word, which holds the word
  /// that ran.
  ///
  /// A stand ends the wait of each reply whose call waits for a retry, a usage limit among them, and asks its model
  /// again at once, on the models that stand then: so a change of the command line or of its account, or the operator
  /// who asks to retry now, ends a wait at once.
  pub fn ear(self) -> Box<dyn Ear> {
    let Provider {
      directory,
      models,
      actor,
      mut thinking,
      images,
      writes,
      backoff,
      retries,
      refresh,
    } = self;
    let mut actor = actor.unwrap_or_else(|| {
      models
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .first()
        .map_or_else(|| OPERATOR.to_owned(), |model| model.at(None))
    });
    let mut images = Images::new(images);
    hearing(&["stand", "reply", RAN], move |mut co: Co<Replied>| async move {
      // The ids of chains repeat in every life, so a conversation of this life is keyed by the life too.
      let life = uuid();
      let mut replies: HashMap<String, Reply> = HashMap::new();
      // The actor whose last reply on each chain the ear refused.
      let mut mute: HashMap<String, String> = HashMap::new();
      // The opening of the life loaded the settings just now, so the first stand, which the boot asks, reads them
      // again only after it: a refresh there asks each program of a model, as the claude command line, a second time.
      let mut stood = false;
      loop {
        let a = match co.next().await {
          Next::Heard(a) => a,
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
            let wrapped = watch.dropped();
            let Ok(turn) = cut(ran.as_deref().unwrap_or_else(|| watch.through(line))) else {
              continue;
            };
            if let Some(Reply { chain, .. }) = replies.remove(&id) {
              mute.remove(&chain);
              if wrapped {
                told(&mut co, &chain, &format!("#{rung} {WRAPPED}"));
              }
              if ran.is_some() {
                told(&mut co, &chain, &format!("#{rung} {AFTER}\n{why}"));
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
                if watch.as_ref().is_some_and(Watch::dropped) {
                  told(&mut co, &chain, &format!("#{rung} {WRAPPED}"));
                }
                told(&mut co, &chain, &format!("#{rung} {STOPPED}: {}", retry.why));
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
            // A wait for the end of a limit lasts hours, so the chain is told once, before it.
            if let (true, Some(reply)) = (limit, replies.get(&id)) {
              let note = format!("#{} waits on {}: {}", reply.chain, reply.actor, retry.why);
              say(
                &mut co,
                Fact::says("tell", &reply.chain, [Object::list([Object::string(&note)])]),
              );
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
                    Err(no) => format!("{STOPPED}: {no}"),
                    Ok(_) => "the turn holds no word".to_owned(),
                  },
                };
                if why.is_empty() {
                  got
                } else {
                  let Ok(turn) = cut(&ran) else { continue };
                  let note = if got.is_err() {
                    format!("#{rung} {why}")
                  } else {
                    format!("#{rung} {AFTER}\n{why}")
                  };
                  told(&mut co, &chain, &note);
                  Ok(turn)
                }
              }
              (None, got) => got,
            };
            let Some(reply) = replies.get(&id) else { continue };
            // The word that runs is not all the text that the model wrote, so the chain is told what went, before it.
            let wrapped = reply.watch.as_ref().is_some_and(Watch::dropped).then_some(WRAPPED);
            if let Some(dropped) = dropped.or(wrapped) {
              let note = format!("#{} {dropped}", reply.rung);
              say(
                &mut co,
                Fact::says("tell", &reply.chain, [Object::list([Object::string(&note)])]),
              );
            }
            let got = got.map_err(|no| format!("{} answered nothing: {no}", reply.actor));
            if let (Err(why), true) = (&got, reply.again) {
              let _ = co
                .call("pause", vec![Object::string(&reply.chain), Object::string(why)], vec![])
                .await;
            }
            let got = got.map_err(Fault::refused).unwrap_or_else(|fault| fault.object());
            say(&mut co, Fact::says("done", &id, [got]));
            continue;
          }
        };
        let about = a.about().to_owned();
        match a.kind() {
          "stand" if a.question() => {
            let first = !std::mem::replace(&mut stood, true);
            if let Some(refresh) = refresh.as_ref().filter(|_| !first) {
              match refresh() {
                Ok((fresh, default, budget)) => {
                  if !fresh.iter().any(|model| model.settings(&actor).is_some()) {
                    actor = default
                      .or_else(|| fresh.first().map(|model| model.at(None)))
                      .unwrap_or_else(|| OPERATOR.to_owned());
                  }
                  *models.write().unwrap_or_else(PoisonError::into_inner) = fresh;
                  thinking = budget;
                }
                Err(why) => {
                  say(&mut co, Fact::says("done", &about, [Fault::refused(why).object()]));
                  continue;
                }
              }
            }
            let stood =
              standing(&models.read().unwrap_or_else(PoisonError::into_inner), &directory, &actor);
            say(&mut co, Fact::says("done", &about, [stood]));
            // A stand reads the settings again, as after a change of the command line or of its account, or when the
            // operator asks to retry now, so a reply whose call waits for a retry, which can wait for days at a usage
            // limit, ends its wait and asks again at once, on the models that stand now.
            let waiting: Vec<String> =
              replies.iter().filter(|(_, reply)| reply.waits).map(|(id, _)| id.clone()).collect();
            for id in waiting {
              let Some(reply) = replies.get(&id) else { continue };
              let (chain, actor) = (reply.chain.clone(), reply.actor.clone());
              let (watch, stop) = asking(
                &mut co,
                &id,
                &chain,
                &actor,
                &life,
                &models,
                &mut images,
                thinking,
                backoff,
              )
              .await?;
              if let Some(reply) = replies.get_mut(&id) {
                reply.waits = false;
                reply.watch = watch.then(Watch::default);
                reply._stop = stop;
              }
            }
          }
          "reply" if a.question() => {
            say(&mut co, Fact::says("started", &about, []));
            let chain = a.on().to_owned();
            let actor = a.word(1).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
            let (watch, stop) = asking(
              &mut co,
              &about,
              &chain,
              &actor,
              &life,
              &models,
              &mut images,
              thinking,
              backoff,
            )
            .await?;
            let again = mute.get(&chain) == Some(&actor);
            let rung = a.by().to_owned();
            let watch = watch.then(Watch::default);
            let reply = Reply { chain, actor, rung, again, watch, waits: false, _stop: stop };
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
              if watch.as_ref().is_some_and(Watch::dropped) {
                told(&mut co, &chain, &format!("#{rung} {WRAPPED}"));
              }
              say(&mut co, Fact::says("done", &id, [turn]));
            }
          }
          "done" => {
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
          _ => {}
        }
      }
    })
  }
}

/// A reply the ear took: its chain, its actor, the rung it answers, whether a refusal of it pauses its chain, the
/// watch over its word while its model writes it, when the model streams its word, and the stop of its work, which
/// ends the call of its model, and what the host is told of it, when the reply goes.
struct Reply {
  chain: String,
  actor: String,
  rung: String,
  again: bool,
  watch: Option<Watch>,
  waits: bool,
  _stop: oneshot::Sender<()>,
}

/// The call of the model of a reply, begun on the turns of its chain and the models that stand now, as a work of the
/// ear; whether its model streams its word, which the ear reads as it comes; and what ends the call once dropped.
#[allow(clippy::too_many_arguments)]
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
) -> Result<(bool, oneshot::Sender<()>), Fault> {
  let turns = co.call("turns", vec![], vec![("on", Object::string(chain))]).await?;
  let (tells, told) = mpsc::unbounded_channel();
  let wrote = written(about, &tells);
  let key = format!("{life}/{chain}");
  let held = models.read().unwrap_or_else(PoisonError::into_inner);
  let asked = requested(&held, actor, chain, turns.as_ref(), &key, images, wrote)
    .map(|one| thinks(one, thinking));
  drop(held);
  let streams = asked.as_ref().is_ok_and(|(model, _)| model.streams_its_word());
  let (stop, stopped) = oneshot::channel();
  co.work(answered(about.to_owned(), asked, backoff, tells, told, stopped));
  Ok((streams, stop))
}

/// What the work of a reply gives: a part that its model wrote, a retry of the call of its model, or what the reply
/// came to, and what the chain is told when its turn lost marks of its text: the marks of a fence or of a tool call,
/// which [`watch::bare`] drops, or the line of three backticks at the end of its word, which [`unclosed`] drops.
enum Replied {
  Wrote { id: String, text: String, thinking: String },
  Retried { id: String, retry: Retry, limit: bool },
  Came { id: String, got: Result<Object, CompletionError>, dropped: Option<&'static str> },
}

/// What the chain is told after the rung of a turn whose model wrote a fault after the part of its word that ran.
const AFTER: &str = "wrote a statement after the part of its word that ran, which the chain would refuse, so its turn \
  is the word that ran, and the gate found:";

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

/// What a turn tells as it streams, sent to the work of its reply, which tells the host and reads the word.
fn written(id: &str, tells: &mpsc::UnboundedSender<Replied>) -> Told {
  let (id, tells) = (id.to_owned(), tells.clone());
  Arc::new(move |text, thinking| {
    let (text, thinking) = (text.to_owned(), thinking.to_owned());
    let _ = tells.send(Replied::Wrote { id: id.clone(), text, thinking });
  })
}

/// What the chains stand on: the models and the operator, the directory, and the default actor. The entry of each
/// model holds its doctrine after its window.
fn standing(models: &[Model], directory: &str, actor: &str) -> Object {
  let offered = models.iter().map(|model| {
    let efforts = model.efforts.iter().map(|(effort, _)| Object::string(effort));
    let window = i64::try_from(model.window).unwrap_or(i64::MAX);
    let entry = [Object::string(&model.name), Object::list(efforts), Object::int(window)];
    Object::list(entry.into_iter().chain([Object::string(catalog::DOCTRINE)]))
  });
  // The operator answers in person, with no effort, and reads the window of the engine.
  let operator = Object::list([Object::string(OPERATOR), Object::list([]), Object::int(WINDOW)]);
  let roster = Object::list(offered.chain([operator]));
  Object::list([roster, Object::string(directory), Object::string(actor)])
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

/// What a reply asks a model, with the most tokens of thought of the turn as the model takes it.
fn thinks((model, mut asked): (Model, Asked), thinking: Thinking) -> (Model, Asked) {
  let mut settings = match asked.request.additional_params.take() {
    Some(Value::Object(settings)) => settings,
    _ => Map::new(),
  };
  asked.thought = match &model.thought {
    Some(Thought::Off(off)) if thinking.most == 0 => {
      merged(&mut settings, off);
      None
    }
    Some(Thought::Off(off)) => Some((thinking.most, off.clone())),
    Some(Thought::Told) if !thinking.inherited => {
      settings.insert(KEY_THINKING.to_owned(), Value::from(thinking.most));
      None
    }
    _ => None,
  };
  asked.request.additional_params = (!settings.is_empty()).then_some(Value::Object(settings));
  (model, asked)
}

/// The turns of a chain as messages of rig. The engine phrases every turn as python and the provider renders
/// nothing: a user turn goes as its python and the images that the prompts it opens name, each after the text that
/// names it and once in the request, and one that holds nothing goes not at all; an assistant turn goes as the blocks
/// its provider gave, less its thought for a model that forgets it, or as its python when a provider of another kind
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
  for (at, turn) in all.iter().enumerate() {
    let Some(role) = entry(turn, 0).and_then(|one| one.as_str()) else { continue };
    let python = entry(turn, 1).and_then(|one| one.as_str()).unwrap_or_default();
    if role == "assistant" {
      let next = all.get(at + 1).and_then(|one| entry(one, 1)?.as_str().map(str::to_owned));
      if let Some(place) = next.as_deref().and_then(refusal) {
        // The quote holds the word less the marks of a fence or of a tool call, which the model would write again.
        let word = watch::bare(python).unwrap_or_else(|| python.to_owned());
        quoted = (!word.trim().is_empty()).then_some((word, place));
        joins = true;
        continue;
      }
      let blocks = entry(turn, 3)
        .and_then(|one| serde_json::from_value::<Vec<AssistantContent>>(wire::record(one)).ok())
        .map(|mut blocks| {
          if model.forgets {
            blocks.retain(|one| !matches!(one, AssistantContent::Reasoning(_)));
          }
          blocks
        })
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
      && let Some(UserContent::Text(was)) = content.first_mut()
    {
      was.text = format!("{}\n\n{python}", was.text);
      content.extend(seen);
      continue;
    }
    let content = [UserContent::Text(Text::new(python))].into_iter().chain(seen);
    messages.push(Message::User { content: content.collect() });
  }
  Ok(messages)
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
        Some(word) => {
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
/// thought off, so it writes its word at once. The turn holds that thought and the word, and what the model wrote
/// counts each part of that thought.
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
  let thought = AssistantContent::Reasoning(Reasoning::new(&thinking));
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

/// What the chain is told after the rung of a turn whose word lost the marks of a fence or of a tool call around it.
const WRAPPED: &str = "wrapped its word in a fence or in the markup of a tool call, and the provider dropped that \
  markup: a reply is python and quotes only, with no fence and no markup";

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
/// long that takes, and each such wait is no attempt.
fn answered(
  id: String,
  asked: Result<(Model, Asked), CompletionError>,
  backoff: Backoff,
  tells: mpsc::UnboundedSender<Replied>,
  mut told: mpsc::UnboundedReceiver<Replied>,
  stopped: oneshot::Receiver<()>,
) -> impl Stream<Item = Replied> {
  let came = async move {
    let got = async {
      let (model, asked) = asked?;
      let mut attempt = 1;
      let mut limited = false;
      loop {
        let no = match (model.answers)(asked.clone()).await {
          Ok(Answer::Response(response)) => return turn(&model, &response),
          Ok(Answer::Turn(turn)) => return hosted(turn.as_ref()).map(|one| (one, None)),
          Err(no) => no,
        };
        if let Some(reset) = limit(&no, &Zoned::now()) {
          let now = Zoned::now();
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
          let _ = tells.send(Replied::Retried { id: id.clone(), retry, limit: !limited });
          limited = true;
          tokio::time::sleep(wait).await;
          continue;
        }
        let Some(why) = passing(&no).filter(|_| attempt < backoff.most) else { return Err(no) };
        attempt += 1;
        let wait = backoff.wait(attempt);
        let retry = Retry { attempt, most: backoff.most, wait, why };
        let _ = tells.send(Replied::Retried { id: id.clone(), retry, limit: false });
        tokio::time::sleep(wait).await;
      }
    };
    let (got, dropped) = match got.await {
      Ok((turn, dropped)) => (Ok(turn), dropped),
      Err(no) => (Err(no), None),
    };
    let _ = tells.send(Replied::Came { id, got, dropped });
  };
  // The reply comes through the channel alone, so it comes after all that its model wrote before it.
  let told = stream::poll_fn(move |cx| told.poll_recv(cx));
  let came = stream::once(came).filter_map(|()| std::future::ready(None));
  stream::select(told, came).take_until(stopped)
}

/// The turn of a model: its text, which is the word of the rung, what it read and wrote, and the blocks it gave.
///
/// A model speaks python alone, so prose around the code stays in the word, for the gate to refuse, but for the marks
/// of a fence or of a tool call around it, which [`watch::bare`] drops, and a line of three backticks alone at its end,
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
) -> Result<(Object, Option<&'static str>), CompletionError> {
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
  } else if let Some(word) = watch::bare(&text) {
    (choice, text, dropped) = (worded(choice, &word), word, Some(WRAPPED));
  } else if let Some(word) = unclosed(&text) {
    (choice, text, dropped) = (worded(choice, &word), word, Some(UNCLOSED));
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
fn uuid() -> String {
  let mut bytes = [0u8; 16];
  let _ = getrandom::fill(&mut bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  let hex: String = bytes.iter().map(|one| format!("{one:02x}")).collect();
  format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

#[cfg(test)]
#[path = "provider.test.rs"]
mod test;
