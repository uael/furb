//! The views of the kits of a life: the skills that each machine says are attached to each life
//! that the client holds, the view of each that the client mounts for the life of the target of
//! its input, with the tree that its skill gave last, and what the operator does to a node, which
//! goes to that skill. The view of each other life ends, so a skill serves the views that show.

use std::{mem, process};

use furb_wire::protocol::{
  Dispose, Effect, Frame, Gesture, Mount, Node, Slot, Tree, Verb, ViewEffect, ViewEvent,
};

use crate::{Asked, Client, Target};

/// A view that the client mounted: its machine, its life, the skill that serves it, the viewer
/// that names it in the daemon, and the tree that the skill gave last with its version, none
/// until the skill gives one.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
  pub at: usize,
  pub life: String,
  pub skill: String,
  pub viewer: String,
  pub tree: Option<Node>,
  pub version: u64,
}

/// The node of a view whose input the input of the client fills: its viewer and its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filled {
  pub viewer: String,
  pub node: String,
}

impl Client {
  /// The views of the life of the target of the input, in the order that their skills attached,
  /// each once its skill gave a tree.
  pub fn shown_views(&self) -> Vec<&View> {
    let Some((at, life)) = self.target_life() else { return Vec::new() };
    let mine = |one: &&View| one.at == at && one.life == life && one.tree.is_some();
    self.views.iter().filter(mine).collect()
  }

  /// The life of the target of the input, on its machine: none for a new session.
  fn target_life(&self) -> Option<(usize, String)> {
    match self.target() {
      Target::Thread(key) => Some((key.0, key.1)),
      Target::Session(at, life) => Some((at, life)),
      Target::New(..) => None,
    }
  }

  /// The verbs of the views that are due, each with its machine and what it asks: the mount of
  /// each skill of the life of the target that has no view, the end of each view of another life
  /// or of a skill that left, and what the operator did to a node.
  pub fn views_due(&mut self) -> Vec<(usize, Verb, Asked)> {
    let target = self.target_life();
    let views = mem::take(&mut self.views);
    let (kept, gone): (Vec<View>, Vec<View>) =
      views.into_iter().partition(|one| self.served_now(one, target.as_ref()));
    self.views = kept;
    let ended = |one: View| {
      let verb = Verb::ViewDispose(Dispose { viewer: one.viewer });
      (one.at, verb, Asked::View)
    };
    let mut due: Vec<_> = gone.into_iter().map(ended).collect();
    if let Some((at, life)) = target {
      let served = self.machines.get(at).and_then(|told| told.served.get(&life)).cloned();
      for skill in served.unwrap_or_default() {
        if self.views.iter().any(|one| one.at == at && one.life == life && one.skill == skill) {
          continue;
        }
        // A viewer names one view of the daemon, among the views of each window of each machine.
        self.next += 1;
        let viewer = format!("{}.{}.{}", process::id(), self.now, self.next);
        let mount = Mount { viewer: viewer.clone(), life: life.clone(), slot: Slot::Footer, skill };
        let view = View {
          at,
          life: life.clone(),
          skill: mount.skill.clone(),
          viewer,
          tree: None,
          version: 0,
        };
        self.views.push(view);
        due.push((at, Verb::ViewMount(mount), Asked::View));
      }
    }
    let said = self.gestures.drain(..).map(|(at, event)| (at, Verb::ViewEvent(event), Asked::View));
    due.extend(said);
    due
  }

  /// Whether a view stays: its life is the one of the target and its skill is still attached.
  fn served_now(&self, view: &View, target: Option<&(usize, String)>) -> bool {
    let attached = self.machines.get(view.at).and_then(|told| told.served.get(&view.life));
    target.is_some_and(|(at, life)| view.at == *at && view.life == *life)
      && attached.is_some_and(|skills| skills.contains(&view.skill))
  }

  /// What the operator did to a node of a view, which goes to its skill.
  pub fn gesture(&mut self, event: ViewEvent) {
    let Some(view) = self.views.iter().find(|one| one.viewer == event.viewer) else { return };
    self.gestures.push((view.at, event));
  }

  /// The input of the client turned to fill an input node of a view, from the value that it holds
  /// now; or back to the target, with none.
  pub fn fill(&mut self, filled: Option<Filled>, value: String) {
    if filled.is_some() {
      self.restore = Some(value);
    }
    self.filled = filled;
  }

  /// The text of the input given to the node that it fills, as a submit: whether it filled one.
  pub fn submit_fill(&mut self, text: &str) -> bool {
    let Some(Filled { viewer, node }) = self.filled.take() else { return false };
    self.gesture(ViewEvent { viewer, node, kind: Gesture::Submit, value: Some(text.to_owned()) });
    true
  }

  /// A frame of a machine about the views: the skills attached to a life, the tree of a view, the
  /// end of a view, or what its skill asks of the client. Whether it was one.
  pub(crate) fn viewed(&mut self, at: usize, frame: &Frame) -> bool {
    match frame {
      Frame::Served { life, skills } => {
        if let Some(told) = self.machines.get_mut(at) {
          told.served.insert(life.clone(), skills.clone());
        }
      }
      Frame::ViewTree(Tree { viewer, version, tree, .. }) => {
        let newer = |one: &&mut View| one.viewer == *viewer && one.version < *version;
        if let Some(view) = self.views.iter_mut().find(newer) {
          (view.version, view.tree) = (*version, Some(tree.clone()));
        }
      }
      Frame::ViewDispose(Dispose { viewer }) => {
        self.views.retain(|one| one.viewer != *viewer);
        if self.filled.as_ref().is_some_and(|one| one.viewer == *viewer) {
          self.filled = None;
        }
      }
      Frame::ViewEffect(ViewEffect { effect, .. }) => match effect {
        Effect::Draft { text } => self.restore = Some(text.clone()),
        Effect::Notice { text } => self.notice = Some(text.clone()),
        Effect::Open { url } => self.notice = Some(format!("The view asks to open {url}.")),
      },
      _ => return false,
    }
    true
  }
}
