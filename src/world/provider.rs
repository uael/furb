//! The provider of models: the ear of the World that answers what the chains stand on, and takes each reply, which a
//! model answers with its turn.
//!
//! A model is any completion model of rig, which the [`catalog`] makes of the models of the providers of the network
//! and of the claude command line, or a function of the host that answers a request with a turn. A call of a model
//! is a future of the ear, which the life polls, and a call that its reply no longer needs ends. It streams, and the host is told what the
//! model writes as it writes it; what the reply came to the ear says of its own accord.

pub mod catalog;
pub mod claude;
mod clients;
pub mod images;
mod pi;

use std::{
  collections::{HashMap, HashSet},
  path::PathBuf,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use futures::{Stream, StreamExt, future::BoxFuture, stream};
use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
  message::{AssistantContent, Message, Text, UserContent},
  streaming::{StreamFinal, StreamedAssistantContent, StreamingCompletionResponse},
};
use serde_json::{Map, Value, json};
use unsync::oneshot;

use self::images::Images;
use crate::{
  SYSTEM,
  ear::{Co, Ear, Next, call, ear, say, tell},
  engine::{OPERATOR, WINDOW},
  fact::Fact,
  value::{Fault, Object, ObjectRef, entry},
  wire,
};

/// What a turn of a model tells as it streams: what it added to its text, and what it added to its thought.
pub type Told = Arc<dyn Fn(&str, &str) + Send + Sync>;

/// What the host is told while a model writes: the rung it writes for, the chain of that rung, and what it added to
/// its text and to its thought. It is no fact, and the turn that the reply is done with holds all of it.
pub type Writes = Arc<dyn Fn(&str, &str, &str, &str) + Send + Sync>;

/// A function of the host that stands in for a model: it is given a request, as JSON, and what it tells as it
/// writes, and it answers with the turn of the model, or with why it gives none.
pub type Hosted =
  Arc<dyn Fn(Value, Told) -> BoxFuture<'static, Result<Object, String>> + Send + Sync>;

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

/// One turn asked of a model: the actor and the chain it answers, the request, and what it tells as it streams.
struct Asked {
  actor: String,
  chain: String,
  request: CompletionRequest,
  told: Told,
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
  answers: Answers,
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
      answers,
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

  /// The same model, whose turns a function of the host answers in place of the model.
  pub fn hosted(mut self, host: Hosted) -> Model {
    self.answers = Arc::new(move |asked| {
      let Asked { actor, chain, request, told } = asked;
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

/// The provider: the directory the life stands on, the models it offers, the default actor, the directory of the
/// images that a turn names, and whom it tells what a model writes.
pub struct Provider {
  directory: String,
  models: Vec<Model>,
  actor: Option<String>,
  images: Option<PathBuf>,
  writes: Option<Writes>,
}

impl Provider {
  /// A provider of these models, on a directory.
  pub fn new(directory: impl Into<String>, models: Vec<Model>) -> Provider {
    Provider { directory: directory.into(), models, actor: None, images: None, writes: None }
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

  /// The ear of the provider: it answers a stand with the standing, the roster of its models and the operator, the
  /// directory the life stands on, and the default actor; and it takes a reply, which the model of its actor
  /// answers.
  ///
  /// A reply reads the turns of its chain: the system prompt is the engine, a user turn goes as its python and the
  /// images that the prompts it opens name, and an assistant turn as the blocks its provider gave. It is done with
  /// the turn of the model, or with a refusal, and a second refusal in a row of the same actor on a chain pauses the
  /// chain first. A done of a reply that the ear did not say ends the call of its model.
  pub fn ear(self) -> Box<dyn Ear> {
    let Provider { directory, models, actor, images, writes } = self;
    let actor = actor.unwrap_or_else(|| {
      models.first().map_or_else(|| OPERATOR.to_owned(), |model| model.at(None))
    });
    let mut images = Images::new(images);
    ear(move |co: Co<Called>| async move {
      // The ids of chains repeat in every life, so a conversation of this life is keyed by the life too.
      let life = uuid();
      let mut replies: HashMap<String, Reply> = HashMap::new();
      // The actor whose last reply on each chain the ear refused.
      let mut mute: HashMap<String, String> = HashMap::new();
      loop {
        let a = match co.next().await {
          Next::Heard(a) => a,
          // What the model of a reply that still stands came to.
          Next::Worked((id, got)) => {
            let Some(reply) = replies.get(&id) else { continue };
            let got =
              got.map_err(|no| Fault::refused(format!("{} answered nothing: {no}", reply.actor)));
            if got.is_err() && reply.again {
              tell(&co, "pause", vec![Object::string(&reply.chain)], vec![]).await;
            }
            let got = got.unwrap_or_else(|fault| fault.object());
            say(&co, Fact::says("done", &id, [got])).await;
            continue;
          }
        };
        let about = a.about().to_owned();
        match a.kind() {
          "stand" if a.question() => {
            say(&co, Fact::says("done", &about, [standing(&models, &directory, &actor)])).await;
          }
          "reply" if a.question() => {
            say(&co, Fact::says("started", &about, [])).await;
            let chain = a.on().to_owned();
            let actor = a.word(1).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
            let turns = call("turns", vec![], vec![("on", Object::string(&chain))])?;
            let over = Arc::new(AtomicBool::new(false));
            let told = told(writes.as_ref(), a.by(), &chain, &over);
            let key = format!("{life}/{chain}");
            let asked = requested(&models, &actor, &chain, turns.as_ref(), &key, &mut images, told);
            let again = mute.get(&chain) == Some(&actor);
            let (stop, stopped) = oneshot::channel();
            co.work(answered(about.clone(), asked, stopped));
            replies.insert(about, Reply { chain, actor, again, over, _stop: stop });
          }
          "done" => {
            let Some(Reply { chain, actor, over, .. }) = replies.remove(&about) else { continue };
            over.store(true, Ordering::SeqCst);
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

/// A reply the ear took: its chain, its actor, whether a refusal of it pauses its chain, whether it went, after which
/// the host is told nothing more of what a model writes for it, and the stop of the call of its model, which ends
/// that call when the reply goes.
struct Reply {
  chain: String,
  actor: String,
  again: bool,
  over: Arc<AtomicBool>,
  _stop: oneshot::Sender<()>,
}

/// What a turn tells as it streams, told to the host under the rung it writes for and the chain of that rung, until
/// its reply is over.
fn told(writes: Option<&Writes>, rung: &str, chain: &str, over: &Arc<AtomicBool>) -> Told {
  let Some(writes) = writes.cloned() else { return Arc::new(|_, _| {}) };
  let (rung, chain, over) = (rung.to_owned(), chain.to_owned(), Arc::clone(over));
  Arc::new(move |text, thinking| {
    if !over.load(Ordering::SeqCst) {
      writes(&rung, &chain, text, thinking);
    }
  })
}

/// What the chains stand on: the models and the operator, the directory, and the default actor.
fn standing(models: &[Model], directory: &str, actor: &str) -> Object {
  let offered = models.iter().map(|model| {
    let efforts = model.efforts.iter().map(|(effort, _)| Object::string(effort));
    let window = i64::try_from(model.window).unwrap_or(i64::MAX);
    Object::list([Object::string(&model.name), Object::list(efforts), Object::int(window)])
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
  let asked = Asked { actor: actor.to_owned(), chain: chain.to_owned(), request, told };
  Ok((model.clone(), asked))
}

/// The turns of a chain as messages of rig. The engine phrases every turn as python and the provider renders
/// nothing: a user turn goes as its python and the images that the prompts it opens name, and one that holds
/// nothing goes not at all; an assistant turn goes as the blocks its provider gave, or as its python when a
/// provider of another kind gave them. A model that takes no image refuses a turn that names one.
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
/// with the raw record of its end, where a model says what the turn cost.
async fn streamed(streams: Streams, asked: Asked) -> Result<CompletionResponse, CompletionError> {
  let Asked { request, told, .. } = asked;
  let mut stream = streams(request).await?;
  // A thought whose parts streamed is told already, and a thought that came whole is told as it comes.
  let mut thought = HashSet::new();
  while let Some(part) = stream.next().await {
    match part? {
      StreamedAssistantContent::Text(text) => told(&text.text, ""),
      StreamedAssistantContent::ReasoningDelta { id, reasoning, .. } => {
        told("", &reasoning);
        thought.insert(id);
      }
      StreamedAssistantContent::Reasoning { reasoning, id } if !thought.contains(&id) => {
        told("", &reasoning.display_text());
      }
      _ => {}
    }
  }
  let raw = stream.response.as_ref().map(|end| end.raw.clone()).unwrap_or_default();
  Ok(CompletionResponse::from(stream).with_raw(raw))
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

/// A reply and what its call came to.
type Called = (String, Result<Object, CompletionError>);

/// What a reply came to: the turn of the model, or why it answered nothing; or nothing, when the reply went first,
/// which ends the call.
fn answered(
  id: String,
  asked: Result<(Model, Asked), CompletionError>,
  stopped: oneshot::Receiver<()>,
) -> impl Stream<Item = Called> {
  let got = async {
    let (model, asked) = asked?;
    match (model.answers)(asked).await? {
      Answer::Response(response) => turn(&model, &response),
      Answer::Turn(turn) => hosted(turn.as_ref()),
    }
  };
  let ended = async move {
    tokio::select! {
      got = got => Some((id, got)),
      _ = stopped => None,
    }
  };
  stream::once(ended).filter_map(std::future::ready)
}

/// The turn of a model: its text, which is the word of the rung, what it read and wrote, and the blocks it gave.
///
/// A model speaks python alone, so a fence or prose around the code stays in the word, for the gate to refuse. What
/// it read is every token of its input, the cache among them, and what it wrote is the rest of its total, its
/// thought among it.
fn turn(model: &Model, response: &CompletionResponse) -> Result<Object, CompletionError> {
  let text: String = response
    .choice
    .iter()
    .filter_map(|one| match one {
      AssistantContent::Text(text) => Some(text.text.as_str()),
      _ => None,
    })
    .collect();
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
  let blocks = serde_json::to_value(&response.choice).map_err(|no| failed(no.to_string()))?;
  let blocks = wire::inward(&blocks).map_err(|fault| failed(fault.message()))?;
  let count = |n: u64| Object::int(i64::try_from(n).unwrap_or(i64::MAX));
  let usage =
    Object::tuple([count(input), count(output), count(read), count(wrote), Object::float(dollars)]);
  Ok(Object::tuple([Object::string("assistant"), Object::string(text.trim()), usage, blocks]))
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
