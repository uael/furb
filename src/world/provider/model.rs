//! The models of the provider, their class in the configs, and what a chain stands on and asks of a
//! model.

use std::{collections::HashMap, path::Path, sync::Arc};

use futures::future::BoxFuture;
use rig_core::{
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
  streaming::StreamingCompletionResponse,
};
use serde_json::{Map, Value, json};

use super::{
  Advertised, Hosted, Told, answer::streamed, catalog, images::Images, messages::messages,
  search::Searches, thinking::Thought,
};
use crate::{
  engine::{OPERATOR, WINDOW},
  value::{Object, ObjectRef},
};

/// What streams one response of a model of rig, whatever model it is.
pub(super) type Streams = Arc<
  dyn Fn(
      CompletionRequest,
    ) -> BoxFuture<'static, Result<StreamingCompletionResponse, CompletionError>>
    + Send
    + Sync,
>;

/// The streams of a model of rig.
pub(super) fn streams<M: CompletionModel + 'static>(model: M) -> Streams {
  let model = Arc::new(model);
  Arc::new(move |request| {
    let model = Arc::clone(&model);
    Box::pin(async move { model.stream(request).await })
  })
}

/// One turn asked of a model: the actor and the chain it answers, the request, what it tells as it
/// streams, and the budget of its thought.
#[derive(Clone)]
pub(super) struct Asked {
  pub(super) actor: String,
  pub(super) chain: String,
  pub(super) request: CompletionRequest,
  pub(super) told: Told,
  pub(super) thought: Option<(u64, Map<String, Value>)>,
}

/// What a model answers a turn with: the response of a model of rig, or the turn that a function of
/// the host gave.
pub(super) enum Answer {
  Response(Box<CompletionResponse>),
  Turn(Object),
}

/// What asks a model for one turn.
type Answers =
  Arc<dyn Fn(Asked) -> BoxFuture<'static, Result<Answer, CompletionError>> + Send + Sync>;

/// The class of a model, which automatic work reads: a cheap model takes the work that no one waits
/// on, and a dear one the work of the operator.
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
      Self::Cheap => "cheap",
      Self::Dear => "dear",
    }
  }

  /// The class of a name, and nothing for another word.
  pub fn named(name: &str) -> Option<Self> {
    match name {
      "cheap" => Some(Self::Cheap),
      "dear" => Some(Self::Dear),
      _ => None,
    }
  }
}

/// The most that a cheap model costs, in dollars for a million tokens that it reads.
pub(crate) const CHEAP: f64 = 1.0;

/// The key of a config whose map gives a model the class that it names, over the one of the
/// catalog.
pub(crate) const KEY_CLASSES: &str = "classes";

/// The key of a config whose list names the models of automatic work, the first first.
pub(crate) const KEY_AUTOMATIC: &str = "automatic";

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

/// A model the provider offers: its name, the window it reads, the efforts it spends, what it
/// costs, whether it takes an image, and what answers it.
///
/// An actor names it as its name alone, or as its name and one of its efforts after a slash. The
/// provider tells an effort to the model as the settings that effort adds to the request, since
/// each provider reads an effort its own way. A model that says what a turn cost says it as the
/// number `cost` of its raw record, and the price of a model counts the cost of one that says none.
#[derive(Clone)]
#[expect(
  clippy::struct_excessive_bools,
  reason = "each bool is a trait of the server of a model apart from the others, no state of one enum"
)]
pub struct Model {
  pub(super) name: String,
  pub(super) window: u64,
  pub(super) efforts: Vec<(String, Map<String, Value>)>,
  pub(super) conversation: Option<String>,
  pub(super) price: Option<[f64; 4]>,
  pub(super) images: bool,
  /// Whether it counts what it read of the cache apart from the rest of what it read, as Anthropic
  /// does.
  pub(super) apart: bool,
  /// The most it writes in one turn, which a provider that asks for it is told.
  pub(super) tokens: Option<u64>,
  /// Whether it says why each of its turns ended, so a turn that says nothing of its end, or that
  /// the most it writes ended, was cut.
  pub(super) ends: bool,
  /// Whether its server may write its thought in its text, between `<think>` and `</think>`, which
  /// the host is told as text while it streams.
  pub(super) tagged: bool,
  /// Whether it may wrap its word in a fence or in a tag of markup, as a model of a local server
  /// does, so its turn loses those marks, as [`unfenced`](super::answer::unfenced) says.
  pub(super) fenced: bool,
  /// Whether a function of the host answers in place of the model, which tells what it likes while
  /// it works, so what it tells is no part of the word that it gives.
  hosted: bool,
  /// How it takes the most tokens of thought of one turn, when it takes one.
  pub(super) thought: Option<Thought>,
  /// Its class, which the catalog gives, else its price.
  class: Option<Class>,
  /// Whether it reads its earlier turns with no thought, as a model of a local server, whose own
  /// template drops the thought of each earlier turn, and whose thought of each turn would else
  /// fill its window in a few turns.
  pub(super) forgets: bool,
  pub(super) answers: Answers,
  /// The native search of the web of its provider, which the provider asks for an actor that
  /// names it.
  pub(super) search: Option<Searches>,
}

impl Model {
  /// A model of rig under a name, with the window it reads, and no effort yet.
  pub fn new<M>(name: impl Into<String>, window: u64, model: M) -> Self
  where
    M: CompletionModel + 'static,
  {
    let streams = streams(model);
    Self::lazy(name, window, move || Ok(Arc::clone(&streams)))
  }

  /// A model of rig that is made when it is first asked, or the reason it cannot be.
  pub(super) fn lazy(
    name: impl Into<String>,
    window: u64,
    made: impl Fn() -> Result<Streams, CompletionError> + Send + Sync + 'static,
  ) -> Self {
    let answers: Answers = Arc::new(move |asked| {
      let made = made();
      Box::pin(async move { Ok(Answer::Response(Box::new(streamed(made?, asked).await?))) })
    });
    Self {
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

  /// One effort the model spends, and the settings that it adds to the request, a map of rig's
  /// additional parameters: a budget of thinking for one provider, an effort of reasoning for
  /// another.
  #[must_use]
  pub fn effort(mut self, name: impl Into<String>, settings: Value) -> Self {
    let settings = match settings {
      Value::Object(settings) => settings,
      _ => Map::new(),
    };
    self.efforts.push((name.into(), settings));
    self
  }

  /// The setting that carries the conversation of a reply, the chain of this life, which a provider
  /// keeps a conversation or a cache by.
  #[must_use]
  pub fn conversation(mut self, key: impl Into<String>) -> Self {
    self.conversation = Some(key.into());
    self
  }

  /// What the model costs, in dollars for a million tokens: read, written, read from the cache, and
  /// written to it.
  #[must_use]
  pub fn price(mut self, input: f64, output: f64, read: f64, wrote: f64) -> Self {
    self.price = Some([input, output, read, wrote]);
    self
  }

  /// The model takes the images that a turn names.
  #[must_use]
  pub fn images(mut self) -> Self {
    self.images = true;
    self
  }

  /// The model, whose provider searches the web with this search.
  #[must_use]
  pub fn searches(mut self, search: Searches) -> Self {
    self.search = Some(search);
    self
  }

  /// The same model, whose turns a function of the host answers in place of the model.
  #[must_use]
  pub fn hosted(mut self, host: Hosted) -> Self {
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

  /// Whether what the model streams as text is its word, so the statements of it that are finished
  /// can be read: the text of a model whose server wraps its word, or writes its thought in it, is
  /// cleaned after it ends, and a function of the host tells what it likes.
  pub(super) fn streams_its_word(&self) -> bool {
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

  /// The actor of this model at a level, moved to the nearest one the model takes, and at
  /// [`catalog::LEVEL`] when the level is unsaid; the model alone when it takes no effort.
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

  /// What the model costs, in dollars for a million tokens: read, written, read from the cache, and
  /// written to it.
  pub fn priced(&self) -> Option<[f64; 4]> {
    self.price
  }

  /// The model, of a class that the catalog gives, which its price no longer gives.
  #[must_use]
  pub fn classed(mut self, class: Class) -> Self {
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

  /// The settings of an actor that names this model: none for its name alone, and those of its
  /// effort for its name and an effort it spends.
  pub(super) fn settings(&self, actor: &str) -> Option<Map<String, Value>> {
    if actor == self.name {
      return Some(Map::new());
    }
    let effort = actor.strip_prefix(self.name.as_str())?.strip_prefix('/')?;
    self.efforts.iter().find(|(name, _)| name == effort).map(|(_, settings)| settings.clone())
  }
}

/// What the chains stand on: the models and the operator, the directory, the default actor, and the
/// path of the doctrine that a chain reads, which [`catalog::doctrine`] gives, each path as
/// [`crate::world::files::tilded`] writes it, then the extensions that the engine seeds the root
/// with, each as `[name, file, life word, root alone]`. A standing with no doctrine, or of a
/// provider that advertises no extensions, gives none, and the engine seeds no root then. Each
/// model of the roster gives its class, and a model of the list of automatic work its place in that
/// list, which the configs of the user, in the config directory, and of the project give, as
/// [`configured`] reads them.
pub(super) fn standing(
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
  // The operator answers in person, with no effort, reads the window of the engine, and costs no
  // dollar.
  let operator = Object::list([
    Object::string(OPERATOR),
    Object::list([]),
    Object::int(WINDOW),
    Object::string(Class::Cheap.name()),
  ]);
  let roster = Object::list(offered.chain([operator]));
  let home = std::env::home_dir();
  let shown = |one: &Path| Object::string(crate::world::files::tilded(one, home.as_deref()));
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

/// What a reply asks: the system prompt, which is the engine of the life minified, the actor, the
/// chain and its turns, and the conversation.
#[derive(Clone, Copy)]
pub(super) struct Ask<'a> {
  pub(super) system: &'a str,
  pub(super) actor: &'a str,
  pub(super) chain: &'a str,
  pub(super) turns: ObjectRef<'a>,
  pub(super) conversation: &'a str,
}

/// The model an actor names, and what a reply asks it: the engine as the system prompt, the turns
/// of the chain, the settings of the effort, the conversation where the model takes one, and whom
/// it tells what it writes.
pub(super) fn requested(
  models: &[Model],
  ask: Ask<'_>,
  images: &mut Images,
  told: Told,
) -> Result<(Model, Asked), CompletionError> {
  let Ask { system, actor, chain, turns, conversation } = ask;
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
    preamble: Some(system.to_owned()),
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
