//! The frames of the protocol as a peer of an older version reads them. A kit works from the moment
//! that it is written, so the daemon takes each kit from [`KITS`] on, and says each value that a
//! later version added in the form of the version of the kit, as Zed keeps the interface of each
//! version of its extensions. Each value that a version added has its line here, with the version
//! that added it, and `host/tests/kits` holds the schema of the first version, against which a test
//! finds each value of the protocol of now that no line here maps.

use crate::{
  Agent, Answered, Exchange, Line, Mark, Page, Reply, Saying,
  protocol::{Answer, Frame},
};

#[cfg(doc)]
use crate::protocol::KITS;

/// The version that added the frame `granted`, which tells a peer that its role moved.
const GRANTED: u32 = 60;
/// The version that added the reply `withdrawn`, which said `refused` with the class of the cancel.
const WITHDRAWN: u32 = 54;
/// The version that added the mark `waits`, which said `ran`.
const WAITS: u32 = 66;
/// The version that added the saying `resolved`, which said `answer`.
const RESOLVED: u32 = 68;

impl Frame {
  /// The frame as a peer of a version reads it, or none for a frame that the version did not have.
  #[must_use]
  pub fn told_to(mut self, version: u32) -> Option<Self> {
    if matches!(self, Self::Granted { .. }) && version < GRANTED {
      return None;
    }
    if let Self::Reply { answer: Answer::Page(page), .. } | Self::Thread { page, .. } = &mut self {
      page.told_to(version);
    }
    Some(self)
  }
}

impl Page {
  fn told_to(&mut self, version: u32) {
    for exchange in &mut self.exchanges {
      exchange.told_to(version);
    }
    for agent in self.agents.iter_mut().chain(self.sub.as_deref_mut()) {
      agent.told_to(version);
    }
  }
}

impl Exchange {
  fn told_to(&mut self, version: u32) {
    for line in self.block.iter_mut().flat_map(|block| &mut block.lines) {
      line.told_to(version);
    }
    for said in &mut self.said {
      match &mut said.kind {
        Saying::Resolved { .. } if version < RESOLVED => said.kind = Saying::Answer,
        Saying::Question { answered: Some(answered), .. } => answered.told_to(version),
        _ => {}
      }
    }
  }
}

impl Agent {
  fn told_to(&mut self, version: u32) {
    if let Some(line) = &mut self.line {
      line.told_to(version);
    }
  }
}

impl Line {
  fn told_to(&mut self, version: u32) {
    if self.mark == Mark::Waits && version < WAITS {
      self.mark = Mark::Ran;
    }
  }
}

impl Answered {
  fn told_to(&mut self, version: u32) {
    if let Reply::Withdrawn(why) = &mut self.reply
      && version < WITHDRAWN
    {
      self.reply = Reply::Refused(why.take().unwrap_or_else(|| "CancelledError".to_owned()));
    }
  }
}

#[cfg(test)]
#[path = "older.test.rs"]
mod test;
