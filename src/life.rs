//! The opening of a life, which every host shares: the record it opens on and keeps, the ears of the World that the
//! crate writes, and the extensions it runs. A host adds its own ears, the provider of its models and the console of
//! its operator among them, which come first in the order of boot, so each takes a question in the place of an ear
//! of the crate, or wraps it.
//!
//! Every life hears the ear of the extensions and the ear of each official extension, whatever it enables, since a
//! record is made again by running its words: a life whose host turns the extensions off enables nothing new, and
//! runs what its record enables all the same. The ears are named as every host names them, so the record of one
//! host opens in another.

use std::path::{Path, PathBuf};

use crate::{
  Ear, Engine, Fault, Object,
  ear::{ear, hear},
  engine::Hosted,
  extension::{self, Extension, Places, memory::memory, skills::skills},
  world,
};

/// The ears of a life, each under the name the engine hears it by, in the order of boot.
pub type Ears = Vec<(String, Box<dyn Ear>)>;

/// What a life is opened on: the record it opens on and whether it keeps what it says there, whether its ears do
/// work, the extensions it enables at its tip, and the places of the user.
pub struct Opening {
  record: Option<PathBuf>,
  keeps: bool,
  works: bool,
  extensions: Vec<Extension>,
  places: Places,
}

impl Default for Opening {
  fn default() -> Opening {
    Opening::new()
  }
}

impl Opening {
  /// A life on no record, whose ears do work, which enables no extension, on the places of this process.
  pub fn new() -> Opening {
    Opening {
      record: None,
      keeps: false,
      works: true,
      extensions: Vec::new(),
      places: Places::here(),
    }
  }

  /// A life opened on the record at a path, which keeps what it says there under the lease of the store when it
  /// keeps, and reads it with no lease when it does not.
  pub fn record(self, path: impl Into<PathBuf>, keeps: bool) -> Opening {
    Opening { record: Some(path.into()), keeps, ..self }
  }

  /// A life that inspects its record: it keeps nothing, and the ears that do work, the files, the commands and time,
  /// are silent under their names, so it runs no command and touches no file.
  pub fn inspecting(self) -> Opening {
    Opening { keeps: false, works: false, ..self }
  }

  /// A life on the config directory of these places, which the configs and the official extensions read.
  pub fn places(self, places: Places) -> Opening {
    Opening { places, ..self }
  }

  /// A life that enables these extensions at its tip, when its record does not enable them already.
  pub fn extensions(self, extensions: Vec<Extension>) -> Opening {
    Opening { extensions, ..self }
  }

  /// A life that enables the extensions that the configs of the user and of this project turn on.
  pub fn configured(self, project: &Path) -> Result<Opening, Fault> {
    let extensions = extension::configured(&self.places, project)?;
    Ok(self.extensions(extensions))
  }

  /// The record the life opens on, and the ears of the crate, in the order of boot: the extensions, each official
  /// extension, the files, the commands and time, and the store when the life keeps.
  pub fn parts(self) -> Result<(Vec<Object>, Ears), Fault> {
    let (record, store) = match (&self.record, self.keeps) {
      (Some(path), true) => world::store(path).map(|(held, store)| (held, Some(store)))?,
      (Some(path), false) => (world::kept(path)?, None),
      (None, _) => (Vec::new(), None),
    };
    let working = |made: fn() -> Box<dyn Ear>| if self.works { made() } else { silent() };
    let config = self.places.config;
    let mut ears = vec![
      ("extensions", extension::extensions(self.extensions)),
      ("memory", memory(config.clone())),
      ("skills", skills(config)),
      ("files", working(world::files)),
      ("bash", working(world::bash)),
      ("time", working(world::time)),
    ];
    ears.extend(store.map(|store| ("store", store)));
    Ok((record, ears.into_iter().map(|(name, ear)| (name.to_owned(), ear)).collect()))
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
