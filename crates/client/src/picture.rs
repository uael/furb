//! The images that the open thread shows, which a client draws: the client asks the machine of
//! the thread once for each, by the uri by which a message names it, and keeps its bytes while the
//! thread shows it.

use std::{
  collections::{HashMap, hash_map::Entry},
  sync::Arc,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use furb_wire::protocol::{Answer, Frame, Verb};

use crate::{Asked, Client};

/// An image by its machine, its life and the uri by which a message names it.
pub type Shown = (usize, String, String);

/// Where an image stands: due to be asked, asked of its machine, held with its bytes and its size
/// in pixels, or refused with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picture {
  Due,
  Asked,
  Held(Held),
  Refused(String),
}

/// The bytes of an image, shared with each widget that draws it, and its size in pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
  pub bytes: Arc<[u8]>,
  pub width: u32,
  pub height: u32,
}

impl Client {
  /// The images that the open thread shows now: the client keeps what it holds of each and asks
  /// each new one, and lets go of the others, so it holds the bytes of one thread at most.
  pub fn show_pictures(&mut self, shown: impl IntoIterator<Item = Shown>) {
    let mut kept: HashMap<Shown, Picture> = HashMap::new();
    // Two messages can name one image, which the client asks once.
    for one in shown {
      if let Entry::Vacant(slot) = kept.entry(one) {
        let picture = self.pictures.remove(slot.key()).unwrap_or(Picture::Due);
        slot.insert(picture);
      }
    }
    self.pictures = kept;
  }

  /// Each image that is due, asked of its machine.
  pub(crate) fn pictures_due(&mut self) -> Vec<(usize, Verb, Asked)> {
    let due = self.pictures.iter_mut().filter(|(_, picture)| **picture == Picture::Due);
    due
      .map(|(shown, picture)| {
        *picture = Picture::Asked;
        let (at, life, source) = shown.clone();
        (at, Verb::Picture { life, source }, Asked::Picture(shown.clone()))
      })
      .collect()
  }

  /// What the machine answered for an image that the thread still shows.
  pub(crate) fn pictured(&mut self, shown: &Shown, heard: Option<Frame>) {
    if let Some(picture) = self.pictures.get_mut(shown) {
      *picture = match heard {
        Some(Frame::Reply { answer: Answer::Value(value), .. }) => held(&value),
        Some(Frame::Refused { why, .. }) => Picture::Refused(why),
        _ => Picture::Refused("its machine went away".to_owned()),
      };
    }
  }
}

/// The image of the answer of a machine, with its bytes in base64 as `data`.
fn held(value: &serde_json::Value) -> Picture {
  let data = value.get("data").and_then(serde_json::Value::as_str).unwrap_or_default();
  let held = STANDARD.decode(data).map_err(|no| no.to_string()).and_then(|bytes| {
    let size =
      imagesize::blob_size(&bytes).map_err(|no| format!("its bytes are no whole image ({no})"))?;
    let pixels = |count: usize| u32::try_from(count).unwrap_or(u32::MAX);
    Ok(Held { width: pixels(size.width), height: pixels(size.height), bytes: bytes.into() })
  });
  held.map_or_else(Picture::Refused, Picture::Held)
}
