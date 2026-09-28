//! One life of the engine, as the operator holds it: the World of this machine it stands on, the record it keeps,
//! and what the operator hears of it.

use std::{
  cell::RefCell,
  collections::HashSet,
  future::Future,
  path::{Path, PathBuf},
  pin::Pin,
  rc::Rc,
  sync::Arc,
  task::{Context, Poll, Wake, Waker},
  thread,
};

use furb::{
  Act, Ear, Engine, Fact, Fault, Object,
  ear::{ear, hear},
  engine::OPERATOR,
  extension::{self, Extension},
  fact,
  life::Opening,
  verbs,
  world::Catalog,
};

use crate::console;

/// One life, as the operator holds it: its engine, its root, the facts of the record it was made again from, each act
/// that record showed started and not done, with its kind, and what the operator hears of its root.
pub struct Life {
  pub engine: Engine,
  pub root: String,
  held: Vec<Fact>,
  left: Vec<(String, String)>,
  quiet: Rc<RefCell<Quiet>>,
}

/// Whether a pause stands over the acts the operator watches, the root and the act it awaits, and the last refusal
/// of a reply, which says why the World paused a chain. A control is over the act it names and over every act on the
/// chain it names, and an act of the operator has no maker above it, so the controls about these acts are all the
/// controls over them.
#[derive(Default)]
struct Quiet {
  acts: Vec<String>,
  paused: bool,
  refused: String,
}

impl Quiet {
  /// The acts it watches, and what the facts said so far say of them, from the first.
  fn watch(&mut self, acts: Vec<String>, facts: &[Fact]) {
    (self.acts, self.paused) = (acts, false);
    for one in facts {
      self.heard(one);
    }
  }

  /// What one more fact says.
  fn heard(&mut self, a: &Fact) {
    let watched = self.acts.iter().any(|one| one == a.about());
    let reply = fact::named(a.about(), "reply");
    match a.kind() {
      "pause" | "wake" if watched => self.paused = a.kind() == "pause",
      "done" if reply => {
        if let Some(no) = a.word(0).and_then(Fault::of) {
          self.refused = no.message();
        }
      }
      _ => {}
    }
  }
}

impl Life {
  /// A life as it opens, on a console of the operator, which comes before the ears of the crate. Each ear is heard
  /// under the name every host hears it by, so the record of one opens in another.
  ///
  /// The journal says the whole record again before boot returns, so the life stands whole on its record here, and a
  /// life whose record drifted is refused, since it would keep nothing more. `heard` is given every fact said after
  /// that, as an ear of the engine hears it.
  pub fn open(
    opening: Opening,
    console: Box<dyn Ear>,
    mut heard: impl FnMut(&Fact) + 'static,
  ) -> Result<Life, String> {
    let failed = |no: Fault| no.to_string();
    let (mut engine, held) = opening.boot([("console", console)]).map_err(failed)?;
    let root = engine.root().to_owned();
    let mut quiet = Quiet::default();
    let facts = engine.transcript(verbs::Transcript { on: Some(root.clone()) }).map_err(failed)?;
    quiet.watch(vec![root.clone()], &facts);
    let quiet = Rc::new(RefCell::new(quiet));
    let quieted = Rc::clone(&quiet);
    let observer = ear(move |co, _| async move {
      loop {
        let a = hear(&co).await;
        quieted.borrow_mut().heard(&a);
        heard(&a);
      }
    });
    engine.drive(observer, "observer").map_err(failed)?;
    let held: Vec<Fact> =
      held.iter().filter_map(|entry| Fact::of(*entry.as_ref().items()?.first()?)).collect();
    let left = left(&held);
    Ok(Life { engine, root, held, left, quiet })
  }

  /// A life for one command of the operator, on the terminal.
  pub fn lived(opening: Opening) -> Result<Life, String> {
    Life::open(opening, console::terminal(), |_| {})
  }

  /// The extensions that the life runs, in the order it enabled them.
  pub fn extensions(&mut self) -> Result<Vec<Extension>, String> {
    let root = self.engine.transcript(verbs::Transcript { on: Some(self.root.clone()) });
    Ok(extension::enabled(&root.map_err(|no| no.to_string())?))
  }

  /// Whether a pause stands over the root.
  pub fn paused(&self) -> bool {
    self.quiet.borrow().paused
  }

  /// Each act that the record showed started and not done when the life opened and that is still not done, with its
  /// kind: work that an earlier life began, which waits for a wake.
  pub fn pending(&mut self) -> Vec<(String, String)> {
    let left = self.left.iter().filter(|(id, _)| matches!(self.engine.outcome(id), Ok(None)));
    left.cloned().collect()
  }

  /// The name of the prompt the record already holds for this message: of the operator, on the root, of this shape
  /// and to this actor; and nothing when it holds none.
  ///
  /// The engine matches nothing the operator says again, so a life stood up on its own record would open a second
  /// prompt beside the one that record stands on, and ask a model for what it was answered once.
  pub fn again(&self, shape: &str, message: &str, to: &str) -> Option<String> {
    let wanted = [OPERATOR, self.root.as_str(), shape, message, to];
    self.held.iter().find_map(|a| {
      let words =
        a.0.as_ref().items()?.iter().map(|one| one.as_str()).collect::<Option<Vec<_>>>()?;
      (words.len() == 7 && a.kind() == "prompt" && words[2..] == wanted)
        .then(|| a.about().to_owned())
    })
  }

  /// What an act of the operator on the root came to, awaited until it is done, as the operator waits on its own
  /// loop; or why it never will be: a pause over the act or over the root, which no one wakes while one command
  /// runs.
  pub fn settled(&mut self, id: &str) -> Result<Object, String> {
    let facts = self.engine.transcript(verbs::Transcript { on: Some(self.root.clone()) });
    let facts = facts.map_err(|no| no.to_string())?;
    self.quiet.borrow_mut().watch(vec![self.root.clone(), id.to_owned()], &facts);
    let waker = Waker::from(Arc::new(Parked(thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
      if let Poll::Ready(got) = Pin::new(&mut Act::<Object>::of(&mut self.engine, id)).poll(&mut cx)
      {
        return got.map_err(|no| no.to_string());
      }
      let quiet = self.quiet.borrow();
      if quiet.paused {
        let why =
          if quiet.refused.is_empty() { String::new() } else { format!(": {}", quiet.refused) };
        return Err(format!(
          "{id} is paused{why}. A wake from the TUI or from `furb --mode rpc` makes it go on."
        ));
      }
      drop(quiet);
      thread::park();
    }
  }
}

/// Each act that the facts of a record show started and not done, as its name and its kind: the journal keeps the
/// started of an act that the outside took, and says the entries again as the life opens.
fn left(facts: &[Fact]) -> Vec<(String, String)> {
  let (mut started, mut done) = (HashSet::new(), HashSet::new());
  for a in facts {
    match a.kind() {
      "started" => started.insert(a.about()),
      "done" => done.insert(a.about()),
      _ => false,
    };
  }
  let left = facts
    .iter()
    .filter(|a| a.question() && started.contains(a.about()) && !done.contains(a.about()));
  left.map(|a| (a.about().to_owned(), a.kind().to_owned())).collect()
}

/// The shape of a prompt of the operator that names none: a str, so the model works until it closes the prompt with
/// its report.
pub const SHAPE: &str = "str";

/// A shape as a prompt is given it, by its name: None itself, or the name of a type.
pub fn shape(name: &str) -> Object {
  if name == "None" { Object::none() } else { Object::string(name) }
}

/// An actor as the roster names it: the model the catalog finds by the name, with the effort moved to the nearest
/// one that model takes, and the name as it is when the catalog knows no model by it, the operator among them.
pub fn actor(to: &str) -> String {
  let named = Catalog::load().roster(Some(&[]), Some(to)).ok().and_then(|(_, actor)| actor);
  named.unwrap_or_else(|| to.to_owned())
}

/// The model of an actor, as the catalog names it, and nothing for an actor that the catalog knows no model by, the
/// operator among them.
pub fn model(to: &str) -> Option<String> {
  let named = Catalog::load().roster(Some(&[]), Some(to)).ok();
  named.and_then(|(models, _)| models.first().map(|model| model.name().to_owned()))
}

/// The directory a life stands on: the one given, or the current one, as an absolute path.
pub fn directory(cwd: Option<&Path>) -> PathBuf {
  let cwd = cwd.unwrap_or(Path::new("."));
  std::path::absolute(cwd).unwrap_or_else(|_| cwd.to_path_buf())
}

/// A waker that unparks the thread that awaits, so a voice that speaks from another thread wakes it.
struct Parked(thread::Thread);

impl Wake for Parked {
  fn wake(self: Arc<Self>) {
    self.0.unpark();
  }
}
