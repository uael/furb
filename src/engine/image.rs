//! The image of a life: its session, as monty dumps it between two runs, and the half of the host,
//! which is what the host holds of that session and what each ear gives a fresh ear of its make.
//! A life that opens from its image runs no boot and replays no entry: its session is the one that
//! monty loads, and each of its ears is a fresh one that stands where the ear before it stood.

use std::{collections::HashMap, num::ParseIntError};

use monty_types::MontyUuid;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
  Engine, Outside, crossing,
  hosted::{At, Handed, Hosted},
  opening::Opened,
};
use crate::{
  ear::Ear,
  sand::Sand,
  value::{Fault, Object},
};

/// The half of the host in the image of a life: what [`Outside`] and [`Engine`] hold of the
/// session, the part of each ear by its name, and the standing of the life, which the provider
/// of a later life stands on before the session loads, with the text of the engine that the life
/// runs, which that provider gives its models. The build digests this file into the key of an
/// image, as `src/image.rs` says, so a build reads no half of another shape.
#[derive(Serialize, Deserialize)]
pub(crate) struct Half {
  pub(crate) source: String,
  opened: Opened,
  /// How many ids the host gave, as text, since JSON holds no number that wide.
  given: String,
  standing: Vec<(MontyUuid, String, At)>,
  waits: Vec<(MontyUuid, Object)>,
  kept: Vec<MontyUuid>,
  numbered: Vec<(i64, Object)>,
  fed: usize,
  root: String,
  holding: Vec<String>,
  parked: Vec<(String, String)>,
  taken: Object,
  ears: Vec<(String, Value)>,
  pub(crate) stood: Object,
}

impl Engine {
  /// The image of the life between two entries: the half of the host and the session. Nothing
  /// when the life cannot stand again from an image: an ear cannot say what a fresh ear needs, as
  /// a generator of a host, the life holds a function of a host, which no later process holds, or
  /// the life stopped or drifted.
  pub(crate) fn image(&mut self) -> Result<Option<(Half, Vec<u8>)>, Fault> {
    let handed = &self.outside.hosted.handed;
    if self.stopped.is_some() || self.raised.is_some() || !handed.calls.is_empty() {
      return Ok(None);
    }
    let parts = handed.ears.iter().map(|(name, ear)| Some((name.clone(), ear.image()?)));
    let Some(ears) = parts.collect::<Option<Vec<_>>>() else { return Ok(None) };
    let stood = self.verb("standing", vec![], vec![])?;
    let (hosted, crossing) = (&self.outside.hosted, &self.outside.crossing);
    let half = Half {
      source: self.source.clone(),
      opened: self.outside.opened.clone(),
      given: self.outside.given.to_string(),
      standing: hosted.standing.iter().map(|(id, (name, at))| (*id, name.clone(), *at)).collect(),
      waits: hosted.waits.iter().map(|(id, wait)| (*id, wait.clone())).collect(),
      kept: crossing.kept.iter().copied().collect(),
      numbered: crossing.numbered.iter().map(|(n, one)| (*n, one.clone())).collect(),
      fed: self.sand.fed(),
      root: self.root.clone(),
      holding: self.holding.clone(),
      parked: self.parked.clone(),
      taken: self.taken.clone(),
      ears,
      stood,
    };
    Ok(Some((half, self.sand.image()?)))
  }

  /// The life that [`Engine::image`] gave, on a sandbox that loaded its session and on fresh ears
  /// of the same names, each given what the ear before it gave. An ear that stood at a yield
  /// stands there: its fresh ear starts at the first thing that it hears, and takes each act that
  /// the ear before it held in flight as an act that an earlier life took. The ears are those that
  /// [`Half::names`] gives, since the session steps each of them and no other.
  pub(crate) fn imaged<N: Into<String>>(
    half: Half,
    sand: Sand,
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<Self, Fault> {
    let (mut handed, _) = Handed::of(ears)?;
    for (name, part) in half.ears {
      if let Some(ear) = handed.ears.get_mut(&name) {
        ear.imaged(part);
      }
    }
    let mut outside = Outside {
      hosted: Hosted { handed, ..Hosted::default() },
      opened: half.opened,
      given: half.given.parse().map_err(|no: ParseIntError| Fault::refused(no.to_string()))?,
      ..Outside::default()
    };
    outside.hosted.standing =
      half.standing.into_iter().map(|(id, name, at)| (id, (name, at))).collect();
    outside.hosted.waits = half.waits.into_iter().collect();
    outside.crossing.kept = half.kept.into_iter().collect();
    for (n, one) in half.numbered {
      crossing::numbered_past(n);
      if let Some(handle) = one.as_ref().handle() {
        outside.crossing.numbers.insert(handle, n);
      }
      outside.crossing.numbered.insert(n, one);
    }
    let mut engine = Self {
      sand,
      outside,
      watchers: HashMap::new(),
      root: half.root,
      raised: None,
      stopped: None,
      holding: half.holding,
      parked: half.parked,
      taken: half.taken,
      source: half.source,
    };
    engine.run(|sand, outside| outside.bound(sand))?;
    Ok(engine)
  }
}

impl Half {
  /// The name of each ear of the host that the session of the image steps.
  pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
    self.standing.iter().map(|one| one.1.as_str())
  }

  /// The sandbox on the session of the image that this half stands beside.
  pub(crate) fn sand(&self, session: &[u8]) -> Result<Sand, Fault> {
    Sand::of_image(session, self.fed)
  }
}
