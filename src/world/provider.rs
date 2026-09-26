//! The provider of models: the ear of the World that answers what the chains stand on, and takes each reply, which a
//! model of rig answers with its turn.
//!
//! A model is any completion model of rig, which the host names in the roster with its efforts and its window, and
//! [`claude`] makes the claude command line one of them. A call of a model runs on a runtime of its own, off the
//! thread that drives the engine, and says what it came to by the voice of the ear.

pub mod claude;

use std::{
  collections::HashMap,
  future::Future,
  pin::Pin,
  sync::{Arc, OnceLock},
  task::{Context, Poll},
};

use futures::future::BoxFuture;
use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
  message::{AssistantContent, Message, Text, UserContent},
};
use serde_json::{Map, Value};
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{
  SYSTEM,
  ear::{Ear, Voice, call, ear, hear, say},
  fact::Fact,
  value::{Fault, Object, ObjectRef, entry},
  wire,
};

/// The operator as the roster offers it: the actor that answers in person, with no effort and the window the engine
/// gives it.
const OPERATOR: (&str, i64) = ("operator", 200_000);

/// What asks a model of rig for one response, whatever model it is.
type Answers = Arc<
  dyn Fn(CompletionRequest) -> BoxFuture<'static, Result<CompletionResponse, CompletionError>>
    + Send
    + Sync,
>;

/// A model the provider offers: its name, the window it reads, the efforts it spends, and the model of rig that
/// answers it.
///
/// An actor names it as its name alone, or as its name and one of its efforts after a slash. The provider tells an
/// effort to the model of rig as the settings that effort adds to the request, since each provider of rig reads an
/// effort its own way. A model that says what a turn cost says it as the number `cost` of its raw record, and the
/// price of a model counts the cost of one that says none.
#[derive(Clone)]
pub struct Model {
  name: String,
  window: u64,
  efforts: Vec<(String, Map<String, Value>)>,
  conversation: Option<String>,
  price: Option<[f64; 4]>,
  answers: Answers,
}

impl Model {
  /// A model of rig under a name, with the window it reads, and no effort yet.
  pub fn new<M>(name: impl Into<String>, window: u64, model: M) -> Model
  where
    M: CompletionModel + 'static,
  {
    let model = Arc::new(model);
    let answers: Answers = Arc::new(move |request| {
      let model = Arc::clone(&model);
      Box::pin(async move { model.completion(request).await })
    });
    Model {
      name: name.into(),
      window,
      efforts: Vec::new(),
      conversation: None,
      price: None,
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

/// The ear of the provider: it answers a stand with the standing, the roster of its models and the operator, the
/// directory the life stands on, and the default actor; and it takes a reply, which the model of its actor answers.
///
/// The default actor is the one given, or the first model at its first effort, or the operator when there is no
/// model. A reply reads the turns of its chain: the system prompt is the engine, a user turn goes as its python, and
/// an assistant turn as the blocks its provider gave. It is done with the turn of the model, or with a refusal, and
/// a second refusal in a row of the same actor on a chain pauses the chain first. A done of a reply that the ear did
/// not say ends the call of its model.
pub fn provider(
  directory: impl Into<String>,
  models: Vec<Model>,
  actor: Option<String>,
) -> Box<dyn Ear> {
  let directory = directory.into();
  let actor = actor.unwrap_or_else(|| match models.first() {
    Some(model) => match model.efforts.first() {
      Some((effort, _)) => format!("{}/{effort}", model.name),
      None => model.name.clone(),
    },
    None => OPERATOR.0.to_owned(),
  });
  ear(move |co, voice| async move {
    // The ids of chains repeat in every life, so a conversation of this life is keyed by the life too.
    let life = uuid();
    let mut replies: HashMap<String, Reply> = HashMap::new();
    // The actor whose last reply on each chain the ear refused.
    let mut mute: HashMap<String, String> = HashMap::new();
    loop {
      let a = hear(&co).await;
      let about = a.about().to_owned();
      match a.kind() {
        "stand" if a.question() => {
          say(&co, Fact::says("done", &about, [standing(&models, &directory, &actor)])).await;
        }
        "reply" if a.question() => {
          say(&co, Fact::says("started", &about, [])).await;
          let chain = a.on().to_owned();
          let actor = a.word(1).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
          let turns = call(&co, "turns", vec![], vec![("on", Object::string(&chain))]).await?;
          let asked = requested(&models, &actor, turns.as_ref(), &format!("{life}/{chain}"));
          let again = mute.get(&chain) == Some(&actor);
          let said = Said { voice: voice.clone(), id: about.clone(), chain: chain.clone() };
          let call = spawned(answered(said, actor.clone(), again, asked));
          replies.insert(about, Reply { chain, actor, call });
        }
        "done" => {
          let Some(Reply { chain, actor, call }) = replies.remove(&about) else { continue };
          drop(call);
          voice.hush(&about);
          // A done that the ear said of its own reply ends the row of refusals on the chain with a turn, or adds to
          // it with a refusal, which is what the ear says when it says no turn; a done that a control said leaves
          // the row as it is.
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

/// A reply the ear took: its chain, its actor, and the call of its model, which ends when the reply goes.
struct Reply {
  chain: String,
  actor: String,
  call: Spawned<()>,
}

/// Who says what a reply came to: the voice of the ear, the reply, and the chain it is on.
struct Said {
  voice: Voice,
  id: String,
  chain: String,
}

/// What the chains stand on: the models and the operator, the directory, and the default actor.
fn standing(models: &[Model], directory: &str, actor: &str) -> Object {
  let offered = models.iter().map(|model| {
    let efforts = model.efforts.iter().map(|(effort, _)| Object::string(effort));
    let window = i64::try_from(model.window).unwrap_or(i64::MAX);
    Object::list([Object::string(&model.name), Object::list(efforts), Object::int(window)])
  });
  let operator =
    Object::list([Object::string(OPERATOR.0), Object::list([]), Object::int(OPERATOR.1)]);
  let roster = Object::list(offered.chain([operator]));
  Object::list([roster, Object::string(directory), Object::string(actor)])
}

/// The model an actor names, and the request of its reply: the engine as the system prompt, the turns of the chain,
/// the settings of the effort, and the conversation where the model takes one.
fn requested(
  models: &[Model],
  actor: &str,
  turns: ObjectRef<'_>,
  conversation: &str,
) -> Result<(Model, CompletionRequest), CompletionError> {
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
    chat_history: messages(turns),
    documents: Vec::new(),
    tools: Vec::new(),
    temperature: None,
    max_tokens: None,
    tool_choice: None,
    additional_params: (!settings.is_empty()).then_some(Value::Object(settings)),
    output_schema: None,
    record_telemetry_content: false,
  };
  Ok((model.clone(), request))
}

/// The turns of a chain as messages of rig. The engine phrases every turn as python and the provider renders
/// nothing: a user turn goes as its python, and one that holds nothing goes not at all, and an assistant turn goes as
/// the blocks its provider gave, or as its python when a provider of another kind gave them.
fn messages(turns: ObjectRef<'_>) -> Vec<Message> {
  let turns = turns.items().unwrap_or_default();
  let each = turns.iter().filter_map(|turn| {
    let role = entry(turn, 0)?.as_str()?;
    let python = entry(turn, 1).and_then(|one| one.as_str()).unwrap_or_default();
    if role == "assistant" {
      let blocks = entry(turn, 3)
        .and_then(|one| serde_json::from_value::<Vec<AssistantContent>>(wire::record(one)).ok())
        .filter(|blocks| !blocks.is_empty())
        .unwrap_or_else(|| vec![AssistantContent::text(python)]);
      return Some(Message::Assistant { id: None, content: blocks });
    }
    (!python.is_empty())
      .then(|| Message::User { content: vec![UserContent::Text(Text::new(python))] })
  });
  each.collect()
}

/// What a reply came to, said by the voice of the ear: the turn of the model, or the refusal of an actor that
/// answered nothing, after the pause of its chain when the refusal before it on that chain was of the same actor.
async fn answered(
  said: Said,
  actor: String,
  again: bool,
  asked: Result<(Model, CompletionRequest), CompletionError>,
) {
  let got = match asked {
    Ok((model, request)) => {
      (model.answers)(request).await.and_then(|response| turn(&model, &response))
    }
    Err(no) => Err(no),
  };
  let Said { voice, id, chain } = said;
  match got {
    Ok(turn) => voice.say("done", &id, [turn]),
    Err(no) => {
      if again {
        voice.call(&id, "pause", vec![Object::string(chain)], vec![]);
      }
      voice.say("done", &id, [Fault::refused(format!("{actor} answered nothing: {no}")).object()]);
    }
  }
}

/// The turn of a model: its text, which is the word of the rung, what it read and wrote, and the blocks it gave.
///
/// A model speaks python alone, so a fence or prose around the code stays in the word, for the gate to refuse. What
/// it read is every token of its input, the cache among them, which rig counts as its total less what it wrote.
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
  let (read, wrote, output) =
    (spent.cached_input_tokens, spent.cache_creation_input_tokens, spent.output_tokens);
  let input = match spent.total_tokens.checked_sub(output) {
    Some(input) if input > 0 => input,
    _ => spent.input_tokens + read + wrote,
  };
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

/// A new random id, a uuid of version 4, as the claude command line takes one for a conversation.
fn uuid() -> String {
  let mut bytes = [0u8; 16];
  let _ = getrandom::fill(&mut bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  let hex: String = bytes.iter().map(|one| format!("{one:02x}")).collect();
  format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

/// The runtime that every call of a model runs on, made once for the process, since a model of rig asks a runtime
/// of tokio and the engine is driven on a thread of its own.
fn runtime() -> &'static Runtime {
  static RUNTIME: OnceLock<Runtime> = OnceLock::new();
  RUNTIME.get_or_init(|| {
    tokio::runtime::Builder::new_multi_thread()
      .worker_threads(2)
      .thread_name("furb-models")
      .enable_all()
      .build()
      .expect("the machine gives the models a runtime")
  })
}

/// Work on the runtime of the models, which gives what it came to, and ends when it is dropped before it is done.
struct Spawned<T>(JoinHandle<T>);

impl<T> Drop for Spawned<T> {
  fn drop(&mut self) {
    self.0.abort();
  }
}

impl<T> Future for Spawned<T> {
  type Output = Result<T, CompletionError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    Pin::new(&mut self.0)
      .poll(cx)
      .map(|got| got.map_err(|no| CompletionError::ProviderError(no.to_string())))
  }
}

/// Work put on the runtime of the models.
fn spawned<F>(work: F) -> Spawned<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send + 'static,
{
  Spawned(runtime().spawn(work))
}

#[cfg(test)]
#[path = "provider.test.rs"]
mod test;
