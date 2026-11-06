//! The view of a kit of a session (D20). A kit that lives in the record alone, which
//! `extend(name, *rungs, session=True, view=value)` of ouroboros makes, names in its fact enable a
//! value of the module of the root that holds the tree of its view. The daemon serves that view as
//! the skill of that name serves one: a peer of its own dials the daemon, says its hello as that
//! skill and attaches the life, and gives each view that a window mounts the tree that the value
//! holds, then the tree again each time that it changes. The tree is a read of the module, so the
//! view says no fact, and a later life serves it again from the fact enable.

use std::{collections::HashMap, path::PathBuf, pin::pin};

use futures::{SinkExt, StreamExt};
use tokio::sync::oneshot;
use tokio_util::codec::{Framed, LinesCodec};

use crate::{
  event::{Event, Seq},
  life::{Command, LifeHandle, Reply},
  protocol::{BOUND, Frame, Hello, Kind, Node, PROTOCOL, Request, Slot, Tree, Verb},
  transport::{Dialer, Local},
};

/// A kit of a session whose view the daemon serves: the life, its handle, the name of the kit,
/// the value of the root that holds its tree, and the folder where the daemon listens.
pub(crate) struct Kit {
  pub(crate) life: String,
  pub(crate) handle: LifeHandle,
  pub(crate) name: String,
  pub(crate) value: String,
  pub(crate) host: PathBuf,
}

/// The stream of the peer of a kit.
type Lines = Framed<<Local as Dialer>::Stream, LinesCodec>;

/// A view of the kit that a window mounted: its slot, and the version and the tree that the window
/// got last.
struct Shown {
  slot: Slot,
  version: u64,
  tree: Option<Node>,
}

impl Kit {
  /// The view of the kit served until the end, the end of the life or the end of the peer.
  pub(crate) async fn served(self, end: oneshot::Receiver<()>) {
    let Some(mut lines) = self.attached().await else { return };
    let mut shown: HashMap<String, Shown> = HashMap::new();
    let mut events = pin!(self.handle.events(0));
    let mut end = pin!(end);
    let mut asked = 2;
    loop {
      let mut told = Vec::new();
      tokio::select! {
        _ = &mut end => return,
        heard = lines.next() => match heard.and_then(Result::ok).and_then(|one| serde_json::from_str::<Frame>(&one).ok()) {
          Some(Frame::ViewMount(mount)) => {
            shown.insert(mount.viewer.clone(), Shown { slot: mount.slot, version: 0, tree: None });
            told.push(mount.viewer);
          }
          Some(Frame::ViewDispose(gone)) => {
            shown.remove(&gone.viewer);
          }
          Some(_) => {}
          None => return,
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
        let Ok(line) = serde_json::to_string(&Request { id: asked, verb: Verb::ViewTree(given) })
        else {
          continue;
        };
        asked += 1;
        if lines.send(line).await.is_err() {
          return;
        }
      }
    }
  }

  /// The peer of the kit, which dialed the daemon, said its hello as the skill of the kit and
  /// attached the life. A skill hears the events of the life from its place in the log; the kit
  /// hears none there, since it reads the changes of the life from its handle.
  async fn attached(&self) -> Option<Lines> {
    let stream = Local::new(&self.host).dial().await.ok()?;
    let mut lines = Framed::new(stream, LinesCodec::new_with_max_length(BOUND));
    let name = Some(self.name.clone());
    let hello = Hello { protocol: PROTOCOL, kind: Kind::Skill, name, role: None };
    let attach = Verb::Attach { life: self.life.clone(), from: Seq::MAX };
    for (id, verb) in [(0, Verb::Hello(hello)), (1, attach)] {
      lines.send(serde_json::to_string(&Request { id, verb }).ok()?).await.ok()?;
    }
    Some(lines)
  }

  /// The tree that the value of the root holds now, when it holds a node of the protocol.
  async fn tree(&self) -> Option<Node> {
    let read = Command::Inspect { name: self.value.clone(), on: None };
    let Ok(Reply::Inspected { value, .. }) = self.handle.send(read).await else { return None };
    serde_json::from_value(value).ok()
  }
}
