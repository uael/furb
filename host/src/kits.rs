//! The view of a kit of a session (D20). A kit that lives in the record alone, which
//! `extend(name, *rungs, session=True, view=value)` of ouroboros makes, names in its fact enable a
//! value of the module of the root that holds the tree of its view. The daemon serves that view as
//! the skill of that name serves one: a peer of its own, in the process of the daemon, attaches the
//! life through the door of the daemon with an outbox of its own, as a peer of a transport does,
//! and gives each view that a window mounts the tree that the value holds, then the tree again each
//! time that it changes. The tree is a read of the module, so the view says no fact, and a later
//! life serves it again from the fact enable.

use std::{collections::HashMap, pin::pin};

use futures::StreamExt;
use tokio::sync::oneshot;

use crate::{
  daemon::Inside,
  event::{Event, Seq},
  life::{Command, LifeHandle, Reply},
  protocol::{Frame, Kind, Node, Peer, Role, Slot, Tree, Verb},
  serve::Outbox,
};

/// A kit of a session whose view the daemon serves: the life, its handle, the name of the kit,
/// the value of the root that holds its tree, and the door of the daemon.
pub(crate) struct Kit {
  pub(crate) life: String,
  pub(crate) handle: LifeHandle,
  pub(crate) name: String,
  pub(crate) value: String,
  pub(crate) inside: Inside,
}

/// A view of the kit that a window mounted: its slot, and the version and the tree that the window
/// got last.
struct Shown {
  slot: Slot,
  version: u64,
  tree: Option<Node>,
}

impl Kit {
  /// The view of the kit served until the end, the end of the life or the end of the daemon. The
  /// kit attaches the life as the skill of its name and hears no event of the life there, since it
  /// reads the changes of the life from its handle.
  pub(crate) async fn served(self, end: oneshot::Receiver<()>) {
    let (outbox, mut frames) = Outbox::new();
    let peer = Peer {
      login: String::new(),
      role: Role::Write,
      kind: Kind::Skill,
      name: Some(self.name.clone()),
    };
    let attach = Verb::Attach { life: self.life.clone(), from: Seq::MAX };
    if self.inside.answer(&peer, attach, &outbox).await.is_err() {
      return;
    }
    let mut shown: HashMap<String, Shown> = HashMap::new();
    let mut events = pin!(self.handle.events(0));
    let mut end = pin!(end);
    loop {
      let mut told = Vec::new();
      tokio::select! {
        _ = &mut end => return,
        heard = frames.recv() => {
          if heard.is_some() {
            outbox.read();
          }
          match heard {
            Some(Frame::ViewMount(mount)) => {
              let viewer = mount.viewer.clone();
              shown.insert(viewer, Shown { slot: mount.slot, version: 0, tree: None });
              told.push(mount.viewer);
            }
            Some(Frame::ViewDispose(gone)) => {
              shown.remove(&gone.viewer);
            }
            _ => {}
          }
        },
        event = events.next() => match event.as_ref().map(|one| &*one.1) {
          Some(Event::Changed(_)) => told.extend(shown.keys().cloned()),
          Some(Event::Ended(_)) | None => return,
          Some(_) => {}
        },
      }
      if told.is_empty() {
        continue;
      }
      let Some(tree) = self.tree().await else { continue };
      for viewer in told {
        let Some(one) = shown.get_mut(&viewer).filter(|one| one.tree.as_ref() != Some(&tree))
        else {
          continue;
        };
        one.version += 1;
        one.tree = Some(tree.clone());
        let given =
          Tree { viewer, slot: one.slot.clone(), version: one.version, tree: tree.clone() };
        // A window that disposed its view meanwhile refuses the tree, and its dispose frame ends
        // the view here.
        self.inside.answer(&peer, Verb::ViewTree(given), &outbox).await.ok();
      }
    }
  }

  /// The tree that the value of the root holds now, when it holds a node of the protocol.
  async fn tree(&self) -> Option<Node> {
    let read = Command::Inspect { name: self.value.clone(), on: None };
    let Ok(Reply::Inspected { value, .. }) = self.handle.send(read).await else { return None };
    serde_json::from_value(value).ok()
  }
}
