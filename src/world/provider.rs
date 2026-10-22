//! The provider of models: the ear of the World that answers what the chains stand on, and takes each reply, which a
//! model answers with its turn.
//!
//! A model is any completion model of rig, which the [`catalog`] makes of the models of the providers of the network,
//! of a local server and of the claude command line, or a function of the host that answers a request with a turn. A
//! call of a model is a future of the ear, which the life polls, and a call that its reply no longer needs ends. It
//! streams, and the host is told what the model writes as it writes it; what the reply came to the ear says of its
//! own accord.

pub mod catalog;
pub mod claude;
mod clients;
pub mod images;
mod local;
mod pi;
pub mod search;

use std::{
  collections::{HashMap, HashSet},
  path::PathBuf,
  sync::Arc,
};

use futures::{Stream, StreamExt, future::BoxFuture, stream};
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

use self::{images::Images, search::Searches};
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
  models: Vec<Model>,
  actor: Option<String>,
  thinking: Thinking,
  images: Option<PathBuf>,
  writes: Option<Writes>,
}

impl Provider {
  /// A provider of these models, on a directory, whose model thinks [`THINKING`] tokens at most in one turn.
  pub fn new(directory: impl Into<String>, models: Vec<Model>) -> Provider {
    let directory = directory.into();
    Provider {
      directory,
      models,
      actor: None,
      thinking: Thinking::default(),
      images: None,
      writes: None,
    }
  }

  /// The most tokens of thought of a model in one turn.
  pub fn thinking(mut self, thinking: Thinking) -> Provider {
    self.thinking = thinking;
    self
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

  /// The native search of the web of the model that an actor names, or none when its provider has none or no
  /// model of the provider is that actor.
  pub fn search(&self) -> Search {
    let models = self.models.clone();
    Arc::new(move |actor| {
      models
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
  /// chain first. A done of a reply that the ear did not say ends the call of its model.
  pub fn ear(self) -> Box<dyn Ear> {
    let Provider { directory, models, actor, thinking, images, writes } = self;
    let actor = actor.unwrap_or_else(|| {
      models.first().map_or_else(|| OPERATOR.to_owned(), |model| model.at(None))
    });
    let mut images = Images::new(images);
    hearing(&["stand", "reply"], move |mut co: Co<Replied>| async move {
      // The ids of chains repeat in every life, so a conversation of this life is keyed by the life too.
      let life = uuid();
      let mut replies: HashMap<String, Reply> = HashMap::new();
      // The actor whose last reply on each chain the ear refused.
      let mut mute: HashMap<String, String> = HashMap::new();
      loop {
        let a = match co.next().await {
          Next::Heard(a) => a,
          // What the model of a reply that still stands writes, told to the host under the rung it writes for.
          Next::Worked(Replied::Wrote { id, text, thinking }) => {
            if let (Some(writes), Some(reply)) = (&writes, replies.get(&id)) {
              writes(&reply.rung, &reply.chain, &text, &thinking);
            }
            continue;
          }
          // What the model of a reply that still stands came to.
          Next::Worked(Replied::Came { id, got }) => {
            let Some(reply) = replies.get(&id) else { continue };
            let got =
              got.map_err(|no| Fault::refused(format!("{} answered nothing: {no}", reply.actor)));
            if got.is_err() && reply.again {
              let _ = co.call("pause", vec![Object::string(&reply.chain)], vec![]).await;
            }
            let got = got.unwrap_or_else(|fault| fault.object());
            say(&mut co, Fact::says("done", &id, [got]));
            continue;
          }
        };
        let about = a.about().to_owned();
        match a.kind() {
          "stand" if a.question() => {
            say(&mut co, Fact::says("done", &about, [standing(&models, &directory, &actor)]));
          }
          "reply" if a.question() => {
            say(&mut co, Fact::says("started", &about, []));
            let chain = a.on().to_owned();
            let actor = a.word(1).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
            let turns = co.call("turns", vec![], vec![("on", Object::string(&chain))]).await?;
            let (tells, told) = mpsc::unbounded_channel();
            let wrote = written(writes.is_some(), &about, &tells);
            let key = format!("{life}/{chain}");
            let asked =
              requested(&models, &actor, &chain, turns.as_ref(), &key, &mut images, wrote)
                .map(|one| thinks(one, thinking));
            let again = mute.get(&chain) == Some(&actor);
            let (stop, stopped) = oneshot::channel();
            co.work(answered(about.clone(), asked, tells, told, stopped));
            let rung = a.by().to_owned();
            replies.insert(about, Reply { chain, actor, rung, again, _stop: stop });
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

/// A reply the ear took: its chain, its actor, the rung it answers, whether a refusal of it pauses its chain, and the
/// stop of its work, which ends the call of its model, and what the host is told of it, when the reply goes.
struct Reply {
  chain: String,
  actor: String,
  rung: String,
  again: bool,
  _stop: oneshot::Sender<()>,
}

/// What the work of a reply gives: a part that its model wrote, or what the reply came to.
enum Replied {
  Wrote { id: String, text: String, thinking: String },
  Came { id: String, got: Result<Object, CompletionError> },
}

/// What a turn tells as it streams, sent to the work of its reply when the host hears what a model writes.
fn written(heard: bool, id: &str, tells: &mpsc::UnboundedSender<Replied>) -> Told {
  if !heard {
    return Arc::new(|_, _| {});
  }
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
/// nothing: a user turn goes as its python and the images that the prompts it opens name, and one that holds
/// nothing goes not at all; an assistant turn goes as the blocks its provider gave, less its thought for a model that
/// forgets it, or as its python when a provider of another kind gave them. A model that takes no image refuses a turn
/// that names one.
fn messages(
  turns: ObjectRef<'_>,
  model: &Model,
  images: &mut Images,
) -> Result<Vec<Message>, CompletionError> {
  let mut messages = Vec::new();
  for turn in turns.items().unwrap_or_default() {
    let Some(role) = entry(&turn, 0).and_then(|one| one.as_str()) else { continue };
    let python = entry(&turn, 1).and_then(|one| one.as_str()).unwrap_or_default();
    if role == "assistant" {
      let blocks = entry(&turn, 3)
        .and_then(|one| serde_json::from_value::<Vec<AssistantContent>>(wire::record(one)).ok())
        .map(|mut blocks| {
          if model.forgets {
            blocks.retain(|one| !matches!(one, AssistantContent::Reasoning(_)));
          }
          blocks
        })
        .filter(|blocks| !blocks.is_empty())
        .unwrap_or_else(|| vec![AssistantContent::text(python)]);
      messages.push(Message::Assistant { id: None, content: blocks });
      continue;
    }
    if python.is_empty() {
      continue;
    }
    let seen = images.named(python).map_err(CompletionError::ProviderError)?;
    if !seen.is_empty() && !model.images {
      return Err(CompletionError::ProviderError(format!(
        "{} does not accept images.",
        model.name
      )));
    }
    let content = [UserContent::Text(Text::new(python))].into_iter().chain(seen);
    messages.push(Message::User { content: content.collect() });
  }
  Ok(messages)
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

/// The terminal record of a stream of a provider, from a response of it that came whole.
fn ended(provider: &str, response: CompletionResponse) -> StreamFinal {
  let mut record = StreamFinal::new(provider, response.usage);
  record.finish_reason = response.finish_reason();
  record.response_id = response.response_id;
  record.model = response.model;
  record.raw = response.raw;
  record
}

/// The work of a reply: each part its model writes, then the turn of the model or why it answered nothing, in the
/// order they came; and nothing more once the reply went, which ends the call of the model.
fn answered(
  id: String,
  asked: Result<(Model, Asked), CompletionError>,
  tells: mpsc::UnboundedSender<Replied>,
  mut told: mpsc::UnboundedReceiver<Replied>,
  stopped: oneshot::Receiver<()>,
) -> impl Stream<Item = Replied> {
  let came = async move {
    let got = async {
      let (model, asked) = asked?;
      match (model.answers)(asked).await? {
        Answer::Response(response) => turn(&model, &response),
        Answer::Turn(turn) => hosted(turn.as_ref()),
      }
    };
    let _ = tells.send(Replied::Came { id, got: got.await });
  };
  // The reply comes through the channel alone, so it comes after all that its model wrote before it.
  let told = stream::poll_fn(move |cx| told.poll_recv(cx));
  let came = stream::once(came).filter_map(|()| std::future::ready(None));
  stream::select(told, came).take_until(stopped)
}

/// The turn of a model: its text, which is the word of the rung, what it read and wrote, and the blocks it gave.
///
/// A model speaks python alone, so a fence or prose around the code stays in the word, for the gate to refuse; only
/// a turn of a model that may wrap its word in a fence, as a model of a local server, loses the fence that opens its
/// text, and the closing fence at its end when it has one. What it read is every token of its input, the cache
/// among them, and what it wrote is the rest of its total, its thought among it. A model that says why each turn
/// ended, and says nothing of the end of this one or says that the most it writes ended it, was cut, so its turn is
/// refused, and a part of a word never runs. The thought that a model writes in its text, before its word, is a block
/// of reasoning of its turn.
fn turn(model: &Model, response: &CompletionResponse) -> Result<Object, CompletionError> {
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
  if model.fenced
    && let Some(word) = unfenced(&text)
  {
    let mut worded = Vec::new();
    for one in choice {
      match one {
        AssistantContent::Text(_)
          if worded.iter().any(|one| matches!(one, AssistantContent::Text(_))) => {}
        AssistantContent::Text(_) => worded.push(AssistantContent::text(&word)),
        other => worded.push(other),
      }
    }
    (choice, text) = (worded, word);
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
  Ok(Object::tuple([Object::string("assistant"), Object::string(text.trim()), usage, blocks]))
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
