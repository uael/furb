//! The opening of a life, which every host shares: the directory it stands on, the record it opens on and keeps, the
//! ears of the World that the crate writes, the provider of its models among them, and the extensions it runs. A
//! host adds its own ears, such as the console of its operator, which come first in the order of boot, so each takes
//! a question in the place of an ear of the crate, or wraps it.
//!
//! Every life hears the ear of the extensions and the ear of each official extension, whatever it enables, since a
//! record is made again by running its words: a life whose host turns the extensions off enables nothing new, and
//! runs what its record enables all the same. The ears are named as every host names them, so the record of one
//! host opens in another.

use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
  Ear, Engine, Fault, Object,
  ear::{ear, hear},
  extension::{self, Places, memory::memory, skills::skills},
  world::{self, Catalog, Writes},
};

/// The ears of a life, each under the name the engine hears it by, in the order of boot.
pub type Ears = Vec<(String, Box<dyn Ear>)>;

/// A function of the host that answers each request in place of the models: it is given the request and a function
/// that tells what it writes as it writes it, and it gives the turn.
pub struct Answer(pub world::Hosted);

/// A function of the host that is told what a model writes, as it writes it: the rung it writes for, the chain of
/// that rung, and what it added to its text and to its thought.
pub struct Stream(pub Writes);

/// What a life is opened on, which every host gives the same: the directory it stands on, the record it opens on and
/// whether it keeps what it says there, whether it does work or only inspects its record, whether it enables the
/// extensions that the configs turn on, the config directory of the user, and what its provider offers. Each part
/// that is unsaid takes the default it names.
#[derive(Default)]
#[cfg_attr(feature = "typescript", napi_derive::napi(object, object_to_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::FromPyObject), pyo3(from_item_all))]
pub struct Opening {
  /// The directory the life stands on, which each chain stands in until it goes elsewhere, and whose config turns
  /// extensions on.
  pub directory: String,
  /// The record the life opens on; none when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub record: Option<String>,
  /// Whether the life keeps what it says to its record, under the lease of the store; true when unsaid. A life that
  /// keeps nothing reads its record with no lease.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub keeps: Option<bool>,
  /// Whether the life only inspects its record; false when unsaid. Such a life keeps nothing, enables no new
  /// extension and asks no model, and the ears that do work, the files, the commands and time, are silent under
  /// their names, so it runs no command and touches no file.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub inspecting: Option<bool>,
  /// Whether the life enables at its tip the extensions that the configs turn on; true when unsaid. A life runs what
  /// its record enables either way.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub extensions: Option<bool>,
  /// The config directory of the user, which the configs and the official extensions read; the one of this process
  /// when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub config: Option<String>,
  /// The actor a prompt goes to when it names none, as the catalog names a model, and an effort after a slash, at its
  /// level as [`world::Model::at`] moves it; the first model of the roster when unsaid.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub actor: Option<String>,
  /// The models the provider offers beside the model of the actor, as the catalog names them. When it is unsaid, the
  /// model of the actor stands alone, or the first model the catalog offers when the actor is unsaid too; a roster
  /// that names none, with no actor, offers the operator alone.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub roster: Option<Vec<String>>,
  /// The claude command line to run, in place of the one that `FURB_CLAUDE_BIN` names or this machine holds.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub claude: Option<String>,
  /// How many seconds a turn of the claude command line may go with no progress.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub stall: Option<f64>,
  /// The directory of the images that a turn names.
  #[cfg_attr(feature = "python", pyo3(default))]
  pub images: Option<String>,
  /// A function of the host that answers each request in place of the models.
  #[cfg_attr(
    feature = "typescript",
    napi(
      ts_type = "(request: { actor: string; chain: string; messages: unknown[]; settings: Record<string, unknown> }, write: (delta: { text?: string; thinking?: string }) => void) => Promise<unknown>"
    )
  )]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub answer: Option<Answer>,
  /// A function of the host that is told what a model writes, as it writes it.
  #[cfg_attr(
    feature = "typescript",
    napi(ts_type = "(rung: string, chain: string, text: string, thinking: string) => void")
  )]
  #[cfg_attr(feature = "python", pyo3(default))]
  pub stream: Option<Stream>,
}

impl Opening {
  /// Whether the life does work, which a life that inspects does not.
  fn works(&self) -> bool {
    self.inspecting != Some(true)
  }

  /// The record the life opens on, and the ears of the crate, in the order of boot: the provider, the extensions,
  /// each official extension, the files, the commands and time, and the store when the life keeps.
  pub fn parts(self) -> Result<(Vec<Object>, Ears), Fault> {
    let works = self.works();
    let (record, store) = match (&self.record, self.keeps != Some(false) && works) {
      (Some(path), true) => world::store(path).map(|(held, store)| (held, Some(store)))?,
      (Some(path), false) => (world::kept(path)?, None),
      (None, _) => (Vec::new(), None),
    };
    let here = Places::here();
    let places = Places { config: self.config.clone().map_or(here.config, PathBuf::from), ..here };
    let extensions = match self.extensions != Some(false) && works {
      true => extension::configured(&places, self.directory.as_ref())?,
      false => Vec::new(),
    };
    let provider = self.provider()?;
    let working = |made: fn() -> Box<dyn Ear>| if works { made() } else { silent() };
    let config = places.config;
    let mut ears = vec![
      ("provider", provider),
      ("extensions", extension::extensions(extensions)),
      ("memory", memory(config.clone())),
      ("skills", skills(config)),
      ("files", working(world::files)),
      ("bash", working(world::bash)),
      ("time", working(world::time)),
    ];
    ears.extend(store.map(|store| ("store", store)));
    Ok((record, ears.into_iter().map(|(name, ear)| (name.to_owned(), ear)).collect()))
  }

  /// The ear of the provider: the models of the roster that the catalog makes, which the function of the host
  /// answers when it gives one, and which a life that inspects asks nothing.
  fn provider(self) -> Result<Box<dyn Ear>, Fault> {
    let works = self.works();
    let stall = self.stall.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
    let catalog = Catalog::load().claude(self.claude.map(PathBuf::from), stall);
    let asks: world::Hosted =
      Arc::new(|_, _| Box::pin(async { Err("a life that inspects asks no model".to_owned()) }));
    let answer = if works { self.answer.map(|one| one.0) } else { Some(asks) };
    let (roster, actor) = (self.roster.as_deref(), self.actor.as_deref());
    let mut provider =
      catalog.provider(self.directory, roster, actor, answer).map_err(Fault::refused)?;
    if let Some(images) = self.images {
      provider = provider.images(images);
    }
    if let Some(stream) = self.stream {
      provider = provider.writes(stream.0);
    }
    Ok(provider.ear())
  }

  /// The life, booted on the ears of the host and then on the ears of the crate, and the record it opened on. A life
  /// whose record drifted is refused, since it would keep nothing more.
  pub fn boot<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let (record, ears) = self.parts()?;
    let host = host.into_iter().map(|(name, ear)| (name.into(), ear));
    let engine = Engine::boot(record.clone(), host.chain(ears))?;
    match engine.raised() {
      Some(no) => Err(no.clone()),
      None => Ok((engine, record)),
    }
  }
}

/// An ear that takes nothing and says nothing, which stands in the place of an ear that does work.
fn silent() -> Box<dyn Ear> {
  ear(|mut co| async move {
    loop {
      hear(&mut co).await;
    }
  })
}

#[cfg(test)]
#[path = "life.test.rs"]
mod test;
