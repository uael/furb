//! The routing of the views (D15 of #983): a window mounts the view of a skill, the skill serves
//! its tree, and the daemon carries each frame between the two without reading it, but for the tree
//! that a window which only reads gets, with each node that acts inert.
//!
//! A skill attaches to a life as a window does, under its name, and hears the events of the life.
//! It holds no life: only a window and a pin do. A viewer is one view of the daemon, mounted by
//! one window and served by one skill, and no other peer says a frame of it.

use std::mem;

use tokio::{sync::mpsc::error::SendError, task::AbortHandle};

use crate::{
  error::ProtocolError,
  protocol::{Dispose, Frame, Mount, Peer, Role, Tree, ViewEffect, ViewEvent},
  serve::Outbox,
};

/// A skill attached to a life: its name, the outbox of its peer, and the task that pushes
/// the events of the life to it.
pub(crate) struct Skill {
  pub(crate) life: String,
  pub(crate) name: String,
  pub(crate) outbox: Outbox,
  pub(crate) told: AbortHandle,
}

/// A view: its viewer, the life, the window that mounted it with its role, and the skill that
/// serves it.
struct Mounted {
  viewer: String,
  life: String,
  window: Outbox,
  role: Role,
  skill: Outbox,
}

/// The skills attached to the lives of the daemon, and each view that a window mounted of one.
#[derive(Default)]
pub(crate) struct Views {
  skills: Vec<Skill>,
  mounted: Vec<Mounted>,
}

impl Views {
  /// A skill attached to a life, in the place of an earlier skill of the same name, whose views
  /// end.
  pub(crate) fn attach(&mut self, skill: Skill) {
    let held = self.skills.iter().position(|one| one.life == skill.life && one.name == skill.name);
    if let Some(at) = held {
      let old = self.skills.remove(at);
      old.told.abort();
      self.end(|viewed| viewed.skill.same_channel(&old.outbox));
    }
    self.skills.push(skill);
  }

  /// A view of a skill mounted by a window, which the skill hears.
  pub(crate) fn mount(
    &mut self,
    mount: Mount,
    peer: &Peer,
    window: &Outbox,
  ) -> Result<(), ProtocolError> {
    let Mount { viewer, life, skill, .. } = &mount;
    let none = || ProtocolError::NoSkill { life: life.clone(), skill: skill.clone() };
    let served = self.skills.iter().find(|one| one.life == *life && one.name == *skill);
    let served = served.ok_or_else(none)?;
    if self.mounted.iter().any(|one| one.viewer == *viewer) {
      return Err(ProtocolError::Mounted { viewer: viewer.clone() });
    }
    let viewed = Mounted {
      viewer: viewer.clone(),
      life: life.clone(),
      window: window.clone(),
      role: peer.role,
      skill: served.outbox.clone(),
    };
    let gone = none();
    served.outbox.send(Frame::ViewMount(mount)).map_err(|SendError(_)| gone)?;
    self.mounted.push(viewed);
    Ok(())
  }

  /// What a person did to a node, from the window of the view to its skill.
  pub(crate) fn event(&self, event: ViewEvent, window: &Outbox) -> Result<(), ProtocolError> {
    let viewed = self.viewed(&event.viewer, |viewed| &viewed.window, window)?;
    pushed(&viewed.skill, viewed, Frame::ViewEvent(event))
  }

  /// The end of a view, from its window to its skill.
  pub(crate) fn dispose(
    &mut self,
    dispose: &Dispose,
    window: &Outbox,
  ) -> Result<(), ProtocolError> {
    self.viewed(&dispose.viewer, |viewed| &viewed.window, window)?;
    self.end(|viewed| viewed.viewer == dispose.viewer);
    Ok(())
  }

  /// The tree of a view, from its skill to its window alone, inert for a window that only reads.
  pub(crate) fn tree(&self, mut tree: Tree, skill: &Outbox) -> Result<(), ProtocolError> {
    let viewed = self.viewed(&tree.viewer, |viewed| &viewed.skill, skill)?;
    if viewed.role == Role::Read {
      tree.tree = tree.tree.inert();
    }
    pushed(&viewed.window, viewed, Frame::ViewTree(tree))
  }

  /// What a skill asks of the one window of a view.
  pub(crate) fn effect(&self, effect: ViewEffect, skill: &Outbox) -> Result<(), ProtocolError> {
    let viewed = self.viewed(&effect.viewer, |viewed| &viewed.skill, skill)?;
    pushed(&viewed.window, viewed, Frame::ViewEffect(effect))
  }

  /// A peer that left a life, by its detach or by its end: its skill, and each view that it mounted
  /// or served, whose other side hears it end. Whether a skill of that peer was attached.
  pub(crate) fn left(&mut self, life: &str, peer: &Outbox) -> bool {
    let attached = self.skills.len();
    self.skills.retain(|skill| {
      let gone = skill.life == life && skill.outbox.same_channel(peer);
      if gone {
        skill.told.abort();
      }
      !gone
    });
    self.end(|viewed| {
      viewed.life == life && (viewed.window.same_channel(peer) || viewed.skill.same_channel(peer))
    });
    self.skills.len() < attached
  }

  /// A life that ended: its skills and its views go with it, and its events told them.
  pub(crate) fn ended(&mut self, life: &str) {
    self.skills.retain(|skill| skill.life != life);
    self.mounted.retain(|viewed| viewed.life != life);
  }

  /// The view of a viewer, when the peer that says the frame is the one that `side` names.
  fn viewed(
    &self,
    viewer: &str,
    side: impl Fn(&Mounted) -> &Outbox,
    peer: &Outbox,
  ) -> Result<&Mounted, ProtocolError> {
    let found =
      self.mounted.iter().find(|one| one.viewer == viewer && side(one).same_channel(peer));
    found.ok_or_else(|| ProtocolError::Unmounted { viewer: viewer.to_owned() })
  }

  /// Each view for which `ends` holds, removed, and told to its window and to its skill.
  fn end(&mut self, ends: impl Fn(&Mounted) -> bool) {
    let (ended, kept) = mem::take(&mut self.mounted).into_iter().partition(|one| ends(one));
    self.mounted = kept;
    for viewed in ended {
      // A peer that is gone hears nothing, and its own end drops what it holds.
      for peer in [&viewed.window, &viewed.skill] {
        peer.send(Frame::ViewDispose(Dispose { viewer: viewed.viewer.clone() })).ok();
      }
    }
  }
}

/// A frame of a view said to a peer of it, or the refusal that the peer is gone.
fn pushed(to: &Outbox, viewed: &Mounted, frame: Frame) -> Result<(), ProtocolError> {
  to.send(frame).map_err(|SendError(_)| ProtocolError::Unmounted { viewer: viewed.viewer.clone() })
}
