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
  engine::Hosted,
  extension::{self, Places, memory::memory, skills::skills},
  world::{self, Catalog, Writes},
};

/// The ears of a life, each under the name the engine hears it by, in the order of boot.
pub type Ears = Vec<(String, Box<dyn Ear>)>;

/// What a life is opened on: the directory it stands on, the record it opens on and whether it keeps what it says
/// there, whether its ears do work, whether it enables the extensions that the configs turn on, the places of the
/// user, and what its provider offers.
pub struct Opening {
  directory: PathBuf,
  record: Option<PathBuf>,
  keeps: bool,
  works: bool,
  extends: bool,
  places: Places,
  actor: Option<String>,
  roster: Option<Vec<String>>,
  claude: Option<PathBuf>,
  stall: Option<Duration>,
  images: Option<PathBuf>,
  writes: Option<Writes>,
  answer: Option<world::Hosted>,
}

impl Opening {
  /// A life on a directory and on no record, whose ears do work, which enables at its tip the extensions that the
  /// configs of the user and of the directory turn on, on the places of this process, and whose provider offers the
  /// first model the catalog of this machine offers.
  pub fn new(directory: impl Into<PathBuf>) -> Opening {
    Opening {
      directory: directory.into(),
      record: None,
      keeps: false,
      works: true,
      extends: true,
      places: Places::here(),
      actor: None,
      roster: None,
      claude: None,
      stall: None,
      images: None,
      writes: None,
      answer: None,
    }
  }

  /// A life opened on the record at a path, which keeps what it says there under the lease of the store when it
  /// keeps, and reads it with no lease when it does not.
  pub fn record(self, path: impl Into<PathBuf>, keeps: bool) -> Opening {
    Opening { record: Some(path.into()), keeps, ..self }
  }

  /// A life that inspects its record: it keeps nothing, enables no new extension and asks no model, and the ears that
  /// do work, the files, the commands and time, are silent under their names, so it runs no command and touches no
  /// file.
  pub fn inspecting(self) -> Opening {
    Opening { keeps: false, works: false, extends: false, ..self }
  }

  /// A life that enables at its tip the extensions that the configs turn on, or none new when `extends` is false.
  pub fn extending(self, extends: bool) -> Opening {
    Opening { extends, ..self }
  }

  /// A life on the config directory of the user at a path, which the configs and the official extensions read, in
  /// place of the one of this process.
  pub fn config(self, config: Option<PathBuf>) -> Opening {
    let places = Places { config: config.unwrap_or(self.places.config), ..self.places };
    Opening { places, ..self }
  }

  /// The actor a prompt goes to when it names none, as the catalog names a model, and an effort after a slash that
  /// moves to the nearest one the model takes; the first model of the roster at its least effort when it is unsaid.
  pub fn actor(self, actor: Option<String>) -> Opening {
    Opening { actor, ..self }
  }

  /// The models the provider offers beside the model of the actor, as the catalog names them. When it is unsaid, the
  /// model of the actor stands alone, or the first model the catalog offers when the actor is unsaid too; and a
  /// roster that names none, with no actor, offers the operator alone.
  pub fn roster(self, roster: Option<Vec<String>>) -> Opening {
    Opening { roster, ..self }
  }

  /// The claude command line at a path, in place of the one of this machine, and how long its turn may go with no
  /// progress.
  pub fn claude(self, claude: Option<PathBuf>, stall: Option<Duration>) -> Opening {
    Opening { claude, stall, ..self }
  }

  /// The directory of the images that a turn names.
  pub fn images(self, images: Option<PathBuf>) -> Opening {
    Opening { images, ..self }
  }

  /// Whom the provider tells what a model writes, as it writes it.
  pub fn writes(self, writes: Option<Writes>) -> Opening {
    Opening { writes, ..self }
  }

  /// A function of the host that answers each request in place of the models it offers.
  pub fn answer(self, answer: Option<world::Hosted>) -> Opening {
    Opening { answer, ..self }
  }

  /// The record the life opens on, and the ears of the crate, in the order of boot: the provider, the extensions,
  /// each official extension, the files, the commands and time, and the store when the life keeps.
  pub fn parts(self) -> Result<(Vec<Object>, Ears), Fault> {
    let (record, store) = match (&self.record, self.keeps) {
      (Some(path), true) => world::store(path).map(|(held, store)| (held, Some(store)))?,
      (Some(path), false) => (world::kept(path)?, None),
      (None, _) => (Vec::new(), None),
    };
    let extensions = match self.extends {
      true => extension::configured(&self.places, &self.directory)?,
      false => Vec::new(),
    };
    let provider = self.provider()?;
    let working = |made: fn() -> Box<dyn Ear>| if self.works { made() } else { silent() };
    let config = self.places.config;
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
  fn provider(&self) -> Result<Box<dyn Ear>, Fault> {
    let catalog = Catalog::load().claude(self.claude.clone(), self.stall);
    let asks: world::Hosted =
      Arc::new(|_, _| Box::pin(async { Err("a life that inspects asks no model".to_owned()) }));
    let answer = if self.works { self.answer.clone() } else { Some(asks) };
    let directory = self.directory.display().to_string();
    let (roster, actor) = (self.roster.as_deref(), self.actor.as_deref());
    let mut provider =
      catalog.provider(directory, roster, actor, answer).map_err(Fault::refused)?;
    if let Some(images) = &self.images {
      provider = provider.images(images);
    }
    if let Some(writes) = &self.writes {
      provider = provider.writes(Arc::clone(writes));
    }
    Ok(provider.ear())
  }

  /// The life, booted on the ears of the host and then on the ears of the crate, and the record it opened on. A life
  /// whose record drifted is refused, since it would keep nothing more.
  pub fn boot<N: Into<String>>(
    self,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    self.boot_on(Hosted::default(), host)
  }

  /// The life, booted on the ears and the functions of a host that a door already added to.
  pub(crate) fn boot_on<N: Into<String>>(
    self,
    hosted: Hosted,
    host: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Engine, Vec<Object>), Fault> {
    let (record, ears) = self.parts()?;
    let host = host.into_iter().map(|(name, ear)| (name.into(), ear));
    let engine = Engine::open(hosted, record.clone(), host.chain(ears))?;
    match engine.raised() {
      Some(no) => Err(no.clone()),
      None => Ok((engine, record)),
    }
  }
}

/// An ear that takes nothing and says nothing, which stands in the place of an ear that does work.
fn silent() -> Box<dyn Ear> {
  ear(|co, _| async move {
    loop {
      hear(&co).await;
    }
  })
}

#[cfg(test)]
#[path = "life.test.rs"]
mod test;
