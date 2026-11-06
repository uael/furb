//! The provider of models: the ear of the World that answers what the chains stand on, and takes
//! each reply, which a model answers with its turn.
//!
//! A model is any completion model of rig, which the [`catalog`] makes of the models of the
//! providers of the network, of a local server and of the claude command line, or a function of the
//! host that answers a request with a turn. A call of a model is a future of the ear, which the
//! life polls, and a call that its reply no longer needs ends. It streams, and the host is told
//! what the model writes as it writes it; what the reply came to the ear says of its own accord.
//! The ear reads the statements of the word of a reply as the model finishes them, and when one
//! cannot be python it ends the reply there, with the word as far as that statement, which the gate
//! refuses. The statements that the gate finds nothing against run as the model writes the rest:
//! the ear says each new part of them as a writing of the rung, a fact that the Kernel hears and
//! runs, and a part that raised, which the Kernel says it ran with, ends the reply with the word
//! that ran. A writing is no act that the ear awaits, since a part that prompts makes a chain ask
//! this ear for a reply, which it takes only at a yield.

mod answer;
pub mod catalog;
pub mod claude;
mod clients;
pub mod codex;
mod hearer;
pub mod images;
mod local;
mod messages;
mod model;
mod pi;
mod retry;
pub(crate) mod search;
mod thinking;
mod watch;
mod word;

use std::{
  path::PathBuf,
  sync::{Arc, PoisonError, RwLock},
};

use futures::future::BoxFuture;
use serde_json::Value;

use self::search::Searches;
use super::interrupted;
use crate::{
  ear::{Co, Ear, Next, hearing},
  value::{Fault, Object},
};

use self::answer::{ended, numbers, uuid};
use self::hearer::{Hearer, Replied, searched};
use self::messages::banner;
use self::model::{Class, Streams, streams};
use self::retry::{LIMIT_POLL, resets};
use self::thinking::{ENV_THINKING, KEY_THINKING, Thinking, Thought, thinking};
use self::word::RAN;
pub use self::{
  model::Model,
  retry::{BACKOFF, Backoff, Retry},
};

/// What a turn of a model tells as it streams: what it added to its text, and what it added to its
/// thought.
pub type Told = Arc<dyn Fn(&str, &str) + Send + Sync>;

/// What the host is told while a model writes: the rung it writes for, the chain of that rung, and
/// what it added to its text and to its thought. It is no fact, and the turn that the reply is done
/// with holds all of it.
pub type Writes = Arc<dyn Fn(&str, &str, &str, &str) + Send + Sync>;

/// What the host is told when a call of a model failed for a cause that passes, and the provider
/// asks the model again after a wait: the rung the reply is for, the chain of that rung, and the
/// retry. What the model wrote before the failure is void, and the turn of the reply holds none of
/// it.
pub type Retries = Arc<dyn Fn(&str, &str, &Retry) + Send + Sync>;

/// A function of the host that stands in for a model: it is given a request, as JSON, and what it
/// tells as it writes, and it answers with the turn of the model, or with why it gives none.
pub type Hosted =
  Arc<dyn Fn(Value, Told) -> BoxFuture<'static, Result<Object, String>> + Send + Sync>;

/// The native search of the web of the model that an actor names, if its provider has one.
pub type Search = Arc<dyn Fn(&str) -> Option<Searches> + Send + Sync>;

/// The provider: the directory the life stands on, the models it offers, the default actor, the
/// most tokens of thought of a model in one turn, the directory of the images that a turn names,
/// whom it tells what a model writes, and the extensions that it gives in the standing.
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
  mode: Mode,
  /// The system prompt of each request: the engine of the life, minified.
  system: Arc<str>,
}

/// What a life does with its provider. A life that only inspects its record asks no model and
/// refuses the question search. When the record holds the standing of its root already, the
/// provider also answers no stand: the standing of the life stays the one that its record holds,
/// and no change of it reaches a chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
  Live,
  Inspecting,
  Held,
}

impl Mode {
  /// The mode of a life that does work or not, whose record holds the standing of its root or not.
  pub(crate) fn of(works: bool, stood: bool) -> Self {
    match (works, stood) {
      (true, _) => Self::Live,
      (false, true) => Self::Held,
      (false, false) => Self::Inspecting,
    }
  }
}

/// An extension as the standing gives it: its name, the file of its word, its life word, and
/// whether it plays on the root alone.
pub type Advertised = (String, String, String, bool);

/// The models of a provider, its default actor and the most tokens of thought of a turn, as the
/// settings give them.
type Settings = (Vec<Model>, Option<String>, Thinking);

type Refresh = Arc<dyn Fn() -> BoxFuture<'static, Result<Settings, String>> + Send + Sync>;

impl Provider {
  /// A provider of these models, on a directory, whose model thinks
  /// [`THINKING`](thinking::THINKING) tokens at most in one turn.
  pub fn new(directory: impl Into<String>, models: Vec<Model>) -> Self {
    let directory = directory.into();
    Self {
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
      mode: Mode::Live,
      system: Arc::from(crate::SYSTEM),
    }
  }

  /// The config directory of the user whose `config.json` the standing reads for the classes
  /// and the list of automatic work, else the one that [`crate::extension::Places::here`]
  /// names: a life that opens on a config directory of its own reads no file of the machine.
  #[must_use]
  pub fn config(mut self, config: impl Into<PathBuf>) -> Self {
    self.config = Some(config.into());
    self
  }

  /// The most tokens of thought of a model in one turn.
  #[must_use]
  pub fn thinking(mut self, thinking: Thinking) -> Self {
    self.thinking = thinking;
    self
  }

  /// What the life does with the provider, as [`Mode`] says.
  #[must_use]
  pub(crate) fn mode(mut self, mode: Mode) -> Self {
    self.mode = mode;
    self
  }

  #[must_use]
  pub(crate) fn refreshing(mut self, refresh: Refresh) -> Self {
    self.refresh = Some(refresh);
    self
  }

  /// The system prompt of each request: the engine that the life runs, minified, which is this
  /// build's own, [`crate::SYSTEM`], unless the record of the life holds another.
  #[must_use]
  pub fn system(mut self, system: Arc<str>) -> Self {
    self.system = system;
    self
  }

  /// The names of the roster and of the actor that the catalog lacked when the life opened. The ear
  /// tells each chain once, at its first stand, and the standing leaves them out.
  #[must_use]
  pub fn gone(mut self, names: Vec<String>) -> Self {
    self.gone = names;
    self
  }

  pub(crate) fn into_settings(self) -> (Vec<Model>, Option<String>, Thinking) {
    let models = self.models.read().unwrap_or_else(PoisonError::into_inner).clone();
    (models, self.actor, self.thinking)
  }

  /// The actor a prompt goes to when it names none, which is the first model at the level of an
  /// actor that names none when none is given, and the operator when there is no model.
  #[must_use]
  pub fn actor(mut self, actor: Option<String>) -> Self {
    self.actor = actor;
    self
  }

  /// The extensions that the standing gives, from which the engine seeds the root of a new record.
  #[must_use]
  pub fn advertises(mut self, extensions: Vec<Advertised>) -> Self {
    self.extensions = Some(extensions);
    self
  }

  /// The directory of the images that a turn names.
  #[must_use]
  pub fn images(mut self, directory: impl Into<PathBuf>) -> Self {
    self.images = Some(directory.into());
    self
  }

  /// Whom the provider tells what a model writes, as it writes it.
  #[must_use]
  pub fn writes(mut self, writes: Writes) -> Self {
    self.writes = Some(writes);
    self
  }

  /// How the provider asks a model again after a failure that passes.
  #[must_use]
  pub fn backoff(mut self, backoff: Backoff) -> Self {
    self.backoff = backoff;
    self
  }

  /// Whom the provider tells each retry of a call of a model, before its wait.
  #[must_use]
  pub fn retries(mut self, retries: Retries) -> Self {
    self.retries = Some(retries);
    self
  }

  /// The native search of the web of the model that an actor names, or none when its provider has
  /// none or no model of the provider is that actor.
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

  /// The ear of the provider: it answers a stand with the standing, the roster of its models and
  /// the operator, the directory the life stands on, and the default actor; and it takes a reply,
  /// which the model of its actor answers.
  ///
  /// A reply reads the turns of its chain: the system prompt is the engine, a user turn goes as its
  /// python and the images that the prompts it opens name, and an assistant turn as the blocks its
  /// provider gave. It is done with the turn of the model, or with a refusal; a recoverable refusal
  /// gives the model another turn with its cause, and never pauses the chain. A prompt too long for
  /// the context of the model is no recoverable refusal, so a second one in a row pauses the chain.
  /// Its refusal says first that the prompt passes the window of the model, so the extension
  /// compaction cuts the chain before its next reply.
  /// A turn whose word lost the marks of a fence or of a tool call around it, or the line of three
  /// backticks at its end, tells its chain so once, before its done. A done of a reply that the ear
  /// did not say ends the call of its model.
  ///
  /// The ear says each part of a word that the model finished and that the gate finds nothing
  /// against as a writing of its rung, unless a pause holds the rung. A reply whose part ran is
  /// done with the word that ran when the Kernel says that the part ran with a raise, when a later
  /// statement holds a fault, and when its call fails, which is then made no more; and with its
  /// turn when the gate finds nothing against the whole word, which holds the word that ran.
  ///
  /// A stand from outside an act reads the settings again, after the first: it starts, and the ear
  /// reads them on the runtime of the ears, so every chain of the life goes on meanwhile, and it is
  /// done with the standing once they come, or refused with why they did not. A stand that a word
  /// makes, or the first, is done at once.
  ///
  /// The first stand on each chain tells it which names of the roster and of the actor the
  /// catalog lacked when the life opened, once, before its done.
  ///
  /// A stand ends the wait of each reply whose call waits for a retry, a usage limit among them,
  /// and asks its model again at once, on the models that stand then: so a change of the command
  /// line or of its account, or the operator who asks to retry now, ends a wait at once.
  ///
  /// A pause over the chain of a reply holds the next call of its model: the call that waits for a
  /// retry, or that a stand asks again, is made at the wake, and not at the end of its wait. A call
  /// that runs returns.
  ///
  /// A search that an earlier life took and did not end the ear closes as interrupted at the wake,
  /// and searches no more; a stand and a reply of an earlier life it takes again.
  pub fn ear(self) -> Box<dyn Ear> {
    hearing(&["stand", "reply", "search", RAN], move |mut co: Co<Replied>| async move {
      // The ids of chains repeat in every life, so a conversation of this life is keyed by the
      // life too.
      let life = uuid().map_err(|no| Fault::refused(no.to_string()))?;
      let mut hearer = Hearer::new(self, life);
      loop {
        hearer.answer(&mut co).await?;
        match co.next().await {
          Next::Resumed(a) if a.kind() == "search" => interrupted(&mut co, &a).await?,
          Next::Heard(a) | Next::Resumed(a) => hearer.heard(&mut co, &a).await?,
          Next::Worked(Replied::Refreshed { about, got }) => hearer.refreshed(&mut co, about, got),
          Next::Worked(Replied::Wrote(wrote)) => hearer.wrote(&mut co, wrote).await,
          Next::Worked(Replied::Retried(retried)) => hearer.retried(&mut co, retried).await,
          Next::Worked(Replied::Came(came)) => hearer.came(&mut co, came).await,
          Next::Worked(Replied::Searched { id, got }) => searched(&mut co, &id, got),
        }
      }
    })
  }
}

#[cfg(test)]
#[path = "provider.test.rs"]
mod test;
