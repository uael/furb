//! The image of a life as it goes and at its end, from which a later life goes on.
//!
//! What bounds an open after a crash is the image that a life writes as it goes: once [`TAIL`]
//! facts of the life stand in no image of it, the life writes its image between two runs, and goes
//! on. Those facts are the ones that it said since its last image, or each fact
//! of its log when it stands on no image, as after an open by its record, since its next open would
//! read its whole record. An image keeps each act in flight, so no act holds it. A later open goes
//! on from that image and feeds only the entries that the record took after it.
//!
//! The thread of the life only takes the image, as [`image::take`] says, and a thread of the
//! blocking pool writes it, as [`Imaging`] says, so no window waits for the write.
//!
//! A fault that stops the sandbox cuts the life, and ends it: the record keeps each fact up to the
//! fault, and a later open goes on from the last image, or replays them. A life that ends with no
//! fault writes its image, as [`ended`] says.

use std::{future, path::Path};

use furb::{
  Engine, Fault,
  image::{self, Taken},
};
use serde::{Deserialize, Serialize};
use tokio::task::{JoinHandle, spawn_blocking};

use crate::event::Event;

/// How many facts of a life stand in no image of it before the life writes its image. A later
/// open feeds each fact after the image: 0.3 to 0.5 ms each on this build, against 2 to 13 s to
/// write the image of a large life, so a tail of this length keeps the open after a crash a few
/// seconds longer than an open with no tail.
pub(crate) const TAIL: u64 = 10_000;

/// What the actor watches of a life since it opened: the facts that it said, those that it said
/// up to its last image, and whether it stands on an image.
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Watch {
  facts: u64,
  imaged: u64,
  /// Whether the life wrote an image or opened from one, since it last opened by its record.
  on_image: bool,
}

impl Watch {
  /// One event that the life told.
  pub(crate) fn heard(&mut self, event: &Event) {
    if let Event::Fact(_) = event {
      self.facts += 1;
    }
  }

  /// Whether the life writes its image now: [`TAIL`] facts of it stand in no image, those that it
  /// said since its last image, or, on no image, the facts that its log `said`.
  pub(crate) fn images(&self, said: u64) -> bool {
    (if self.on_image { self.facts - self.imaged } else { said }) >= TAIL
  }

  /// The life wrote its image, or tried to, after the facts that it said so far.
  pub(crate) fn imaged(&mut self) {
    self.imaged = self.facts;
    self.on_image = true;
  }

  /// The life told the past of its record, which no image holds: it stands on no image, with no
  /// fact said.
  pub(crate) fn reborn(&mut self) {
    *self = Self::default();
  }
}

/// The image that a life writes as it goes, on a thread of the blocking pool, while the life goes
/// on. One write stands at a time, so an image never lands over a later one, and the life awaits
/// it before it writes the image of its end.
#[derive(Default)]
pub(crate) struct Imaging(Option<JoinHandle<Result<(), Fault>>>);

impl Imaging {
  /// Whether a write stands.
  pub(crate) fn writes(&self) -> bool {
    self.0.is_some()
  }

  /// The write of an image that the life took, with what its host kept when it took it.
  pub(crate) fn start(&mut self, taken: Taken, kept: impl Serialize + Send + 'static) {
    self.0 = Some(spawn_blocking(move || taken.write(&kept)));
  }

  /// The outcome of the write that stands, once it ends. With no write, it never ends.
  pub(crate) async fn written(&mut self) -> Result<(), Fault> {
    let Some(write) = &mut self.0 else { return future::pending().await };
    let written = write.await;
    self.0 = None;
    written.unwrap_or_else(|no| Err(Fault::refused(format!("the image was not written: {no}"))))
  }
}

/// The engine of a life dropped at the end of the life. A life that keeps its record ends with its
/// image, which holds what the host keeps of the life, so the next life of the record opens from
/// it and reads no fact of its past, and a build that reads no image of this one opens it by the
/// record, on the texts that the record names. A life that writes no image, as one that holds a
/// function of a host, ends with nothing more, and the next life opens by the record. It gives the
/// fault of the end, when one came.
pub(crate) fn ended(
  mut engine: Engine,
  record: Option<&Path>,
  kept: &impl Serialize,
) -> Option<Fault> {
  image::keep(&mut engine, record?, kept).err()
}

#[cfg(test)]
#[path = "rebirth.test.rs"]
mod tests;
